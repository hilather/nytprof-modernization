/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-006 — Real v5 wire sink (oracle FileHandle.xs protocol).
 *
 * Wire tags and packed integers match baseline 6.15 FileHandle.xs /
 * nytprof-format-v5. Logical seq (COL-003) is not written on the wire.
 */
#include "nytp_sink_v5.h"

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include <zlib.h>

/* Wire tag bytes (FileHandle.h). */
#define NYTP_TAG_NO_TAG ((unsigned char)'\0')
#define NYTP_TAG_ATTRIBUTE ((unsigned char)':')
#define NYTP_TAG_OPTION ((unsigned char)'!')
#define NYTP_TAG_COMMENT ((unsigned char)'#')
#define NYTP_TAG_TIME_BLOCK ((unsigned char)'*')
#define NYTP_TAG_TIME_LINE ((unsigned char)'+')
#define NYTP_TAG_DISCOUNT ((unsigned char)'-')
#define NYTP_TAG_NEW_FID ((unsigned char)'@')
#define NYTP_TAG_SRC_LINE ((unsigned char)'S')
#define NYTP_TAG_SUB_INFO ((unsigned char)'s')
#define NYTP_TAG_SUB_CALLERS ((unsigned char)'c')
#define NYTP_TAG_PID_START ((unsigned char)'P')
#define NYTP_TAG_PID_END ((unsigned char)'p')
#define NYTP_TAG_STRING ((unsigned char)'\'')
#define NYTP_TAG_STRING_UTF8 ((unsigned char)'"')
#define NYTP_TAG_START_DEFLATE ((unsigned char)'z')
#define NYTP_TAG_SUB_ENTRY ((unsigned char)'>')
#define NYTP_TAG_SUB_RETURN ((unsigned char)'<')

#define NYTP_V5_MAJOR 5u
#define NYTP_V5_MINOR 0u
#define NYTP_V5_DEFAULT_COMPRESS 6
#define NYTP_V5_INIT_CAP 4096u

typedef struct v5_impl {
    nytp_counting_stats stats;
    char *path; /* sink-owned copy; may be NULL */
    uint8_t *buf;
    size_t len;
    size_t cap;
    int compress_level; /* 1..9, or 0 => use default 6 at deflate start */
    int deflating;
    int deflate_finished;
    int file_written;
    int header_ok;
    int durable; /* 1 => flush/close are seal_publish; live RAM uncompressed */
    size_t header_end;
    size_t len_at_last_seal;
    int last_seal_ok;
    z_stream zs;
} v5_impl;

/* ---- buffer helpers ---- */

static nytp_status buf_reserve(v5_impl *vi, size_t need_extra)
{
    size_t need;
    size_t ncap;
    uint8_t *nbuf;

    if (need_extra > SIZE_MAX - vi->len) {
        return NYTP_ERR_OVERFLOW;
    }
    need = vi->len + need_extra;
    if (need <= vi->cap) {
        return NYTP_OK;
    }
    ncap = vi->cap ? vi->cap : NYTP_V5_INIT_CAP;
    while (ncap < need) {
        if (ncap > SIZE_MAX / 2u) {
            ncap = need;
            break;
        }
        ncap *= 2u;
    }
    nbuf = (uint8_t *)realloc(vi->buf, ncap);
    if (!nbuf) {
        return NYTP_ERR_IO;
    }
    vi->buf = nbuf;
    vi->cap = ncap;
    return NYTP_OK;
}

/* Append raw bytes (no deflate). Used for header and pre-deflate body. */
static nytp_status buf_append_raw(v5_impl *vi, const void *p, size_t n)
{
    nytp_status st;
    if (n == 0) {
        return NYTP_OK;
    }
    st = buf_reserve(vi, n);
    if (st != NYTP_OK) {
        return st;
    }
    memcpy(vi->buf + vi->len, p, n);
    vi->len += n;
    return NYTP_OK;
}

/* Grow output room and bind zs next_out/avail_out; returns room size. */
static nytp_status zs_bind_out(v5_impl *vi, size_t *room_out)
{
    nytp_status st;
    size_t room;
    /* Prefer at least 1 KiB free when compressing. */
    st = buf_reserve(vi, 1024);
    if (st != NYTP_OK) {
        return st;
    }
    room = vi->cap - vi->len;
    if (room == 0) {
        return NYTP_ERR_IO;
    }
    if (room > 0xFFFFu) {
        room = 0xFFFFu;
    }
    vi->zs.next_out = vi->buf + vi->len;
    vi->zs.avail_out = (uInt)room;
    if (room_out) {
        *room_out = room;
    }
    return NYTP_OK;
}

static void zs_commit_out(v5_impl *vi, size_t room)
{
    size_t produced = room - (size_t)vi->zs.avail_out;
    vi->len += produced;
}

/*
 * Append logical payload bytes. When deflating, run through zlib into buf.
 * Oracle: writes after START_DEFLATE are compressed with windowBits=15.
 */
static nytp_status buf_write(v5_impl *vi, const void *p, size_t n)
{
    const uint8_t *in;
    size_t remaining;
    if (!vi->deflating || vi->deflate_finished) {
        return buf_append_raw(vi, p, n);
    }
    if (n == 0) {
        return NYTP_OK;
    }
    in = (const uint8_t *)p;
    remaining = n;
    while (remaining > 0) {
        size_t chunk = remaining > 0xFFFFu ? 0xFFFFu : remaining;
        size_t room = 0;
        int zst;
        nytp_status st;

        /* zlib mutates next_in; cast away const (input not written). */
        vi->zs.next_in = (Bytef *)(uintptr_t)in;
        vi->zs.avail_in = (uInt)chunk;

        while (vi->zs.avail_in > 0) {
            st = zs_bind_out(vi, &room);
            if (st != NYTP_OK) {
                return st;
            }
            zst = deflate(&vi->zs, Z_NO_FLUSH);
            zs_commit_out(vi, room);
            if (zst != Z_OK && zst != Z_BUF_ERROR) {
                return NYTP_ERR_IO;
            }
            /* If no progress with full out buffer, grow and retry. */
            if (vi->zs.avail_in > 0 && vi->zs.avail_out == 0) {
                st = buf_reserve(vi, vi->cap ? vi->cap : 1024);
                if (st != NYTP_OK) {
                    return st;
                }
                continue;
            }
            if (vi->zs.avail_in > 0 && room == (size_t)vi->zs.avail_out) {
                /* No input consumed and no output — force grow. */
                st = buf_reserve(vi, (vi->cap ? vi->cap : 1024) + 1024);
                if (st != NYTP_OK) {
                    return st;
                }
            }
        }
        in += chunk;
        remaining -= chunk;
    }
    return NYTP_OK;
}

static nytp_status deflate_finish(v5_impl *vi)
{
    int zst;
    if (!vi->deflating || vi->deflate_finished) {
        return NYTP_OK;
    }
    vi->zs.next_in = Z_NULL;
    vi->zs.avail_in = 0;
    for (;;) {
        size_t room = 0;
        nytp_status st = zs_bind_out(vi, &room);
        if (st != NYTP_OK) {
            return st;
        }
        zst = deflate(&vi->zs, Z_FINISH);
        zs_commit_out(vi, room);
        if (zst == Z_STREAM_END) {
            break;
        }
        if (zst != Z_OK && zst != Z_BUF_ERROR) {
            return NYTP_ERR_IO;
        }
        /* Need more output space. */
        st = buf_reserve(vi, (vi->cap ? vi->cap : 1024) + 1024);
        if (st != NYTP_OK) {
            return st;
        }
    }
    (void)deflateEnd(&vi->zs);
    memset(&vi->zs, 0, sizeof(vi->zs));
    vi->deflate_finished = 1;
    vi->deflating = 0;
    return NYTP_OK;
}

/* ---- packed integers (FileHandle.xs output_tag_u32) ---- */

static nytp_status write_tag_u32(v5_impl *vi, unsigned char tag, uint32_t i)
{
    uint8_t buffer[6];
    size_t n = 0;
    if (tag != NYTP_TAG_NO_TAG) {
        buffer[n++] = tag;
    }
    if (i < 0x80u) {
        buffer[n++] = (uint8_t)i;
    } else if (i < 0x4000u) {
        buffer[n++] = (uint8_t)((i >> 8) | 0x80u);
        buffer[n++] = (uint8_t)i;
    } else if (i < 0x200000u) {
        buffer[n++] = (uint8_t)((i >> 16) | 0xC0u);
        buffer[n++] = (uint8_t)(i >> 8);
        buffer[n++] = (uint8_t)i;
    } else if (i < 0x10000000u) {
        buffer[n++] = (uint8_t)((i >> 24) | 0xE0u);
        buffer[n++] = (uint8_t)(i >> 16);
        buffer[n++] = (uint8_t)(i >> 8);
        buffer[n++] = (uint8_t)i;
    } else {
        buffer[n++] = 0xFFu;
        buffer[n++] = (uint8_t)(i >> 24);
        buffer[n++] = (uint8_t)(i >> 16);
        buffer[n++] = (uint8_t)(i >> 8);
        buffer[n++] = (uint8_t)i;
    }
    return buf_write(vi, buffer, n);
}

static nytp_status write_u32(v5_impl *vi, uint32_t i)
{
    return write_tag_u32(vi, NYTP_TAG_NO_TAG, i);
}

static nytp_status write_tag_i32(v5_impl *vi, unsigned char tag, int32_t i)
{
    uint32_t u;
    memcpy(&u, &i, sizeof(u));
    return write_tag_u32(vi, tag, u);
}

static nytp_status write_nv(v5_impl *vi, double nv)
{
    /* Oracle: native double bytes (LE on this platform / fixtures). */
    return buf_write(vi, &nv, sizeof(nv));
}

/*
 * Fail-closed string-view check *before* any wire bytes are written.
 * Contract: ptr may be NULL only when len == 0 (see nytp_string_view).
 */
static nytp_status check_str_view(nytp_string_view sv)
{
    if (sv.len > 0 && !sv.ptr) {
        return NYTP_ERR_NULL;
    }
    if (sv.len > 0xFFFFFFFFu) {
        return NYTP_ERR_OVERFLOW;
    }
    return NYTP_OK;
}

static nytp_status write_str(v5_impl *vi, nytp_string_view sv)
{
    unsigned char tag = sv.is_utf8 ? NYTP_TAG_STRING_UTF8 : NYTP_TAG_STRING;
    uint32_t len;
    nytp_status st = check_str_view(sv);
    if (st != NYTP_OK) {
        return st;
    }
    len = (uint32_t)sv.len;
    st = write_tag_u32(vi, tag, len);
    if (st != NYTP_OK) {
        return st;
    }
    if (len == 0) {
        return NYTP_OK;
    }
    return buf_write(vi, sv.ptr, (size_t)len);
}

static nytp_status write_plain_kv(v5_impl *vi, unsigned char prefix,
                                  nytp_string_view key, nytp_string_view value)
{
    nytp_status st;
    const char eq = '=';
    const char nl = '\n';
    st = check_str_view(key);
    if (st != NYTP_OK) {
        return st;
    }
    st = check_str_view(value);
    if (st != NYTP_OK) {
        return st;
    }
    st = buf_write(vi, &prefix, 1);
    if (st != NYTP_OK) {
        return st;
    }
    if (key.len) {
        st = buf_write(vi, key.ptr, key.len);
        if (st != NYTP_OK) {
            return st;
        }
    }
    st = buf_write(vi, &eq, 1);
    if (st != NYTP_OK) {
        return st;
    }
    if (value.len) {
        st = buf_write(vi, value.ptr, value.len);
        if (st != NYTP_OK) {
            return st;
        }
    }
    return buf_write(vi, &nl, 1);
}

/* Project nytp_ticks onto v5 I32; fail closed outside range (OI-003-01 residual). */
static nytp_status ticks_to_i32(nytp_ticks t, int32_t *out)
{
    if (t < (nytp_ticks)INT32_MIN || t > (nytp_ticks)INT32_MAX) {
        return NYTP_ERR_OVERFLOW;
    }
    *out = (int32_t)t;
    return NYTP_OK;
}

/* ---- stats helpers (shared with counting-layout tests) ---- */

static void note_kind(v5_impl *vi, nytp_event_kind kind)
{
    if ((unsigned)kind < (unsigned)NYTP_EVT_KIND_COUNT) {
        vi->stats.by_kind[kind]++;
    }
    vi->stats.total_emits++;
    vi->stats.last_kind = kind;
}

static void v5_on_logical_committed(nytp_sink *sink, nytp_seq seq,
                                    nytp_event_kind kind)
{
    v5_impl *vi;
    if (!sink || !sink->impl) {
        return;
    }
    vi = (v5_impl *)sink->impl;
    vi->stats.logical_emits++;
    vi->stats.last_seq = seq;
    vi->stats.has_last_seq = 1;
    if (vi->stats.seq_ring_len < NYTP_COUNTING_SEQ_RING) {
        vi->stats.seq_ring[vi->stats.seq_ring_len] = seq;
        vi->stats.kind_ring[vi->stats.seq_ring_len] = kind;
        vi->stats.seq_ring_len++;
    } else {
        memmove(vi->stats.seq_ring, vi->stats.seq_ring + 1,
                (NYTP_COUNTING_SEQ_RING - 1) * sizeof(nytp_seq));
        memmove(vi->stats.kind_ring, vi->stats.kind_ring + 1,
                (NYTP_COUNTING_SEQ_RING - 1) * sizeof(nytp_event_kind));
        vi->stats.seq_ring[NYTP_COUNTING_SEQ_RING - 1] = seq;
        vi->stats.kind_ring[NYTP_COUNTING_SEQ_RING - 1] = kind;
    }
}

static void copy_subname(v5_impl *vi, nytp_string_view name)
{
    size_t n = name.len;
    if (n >= sizeof(vi->stats.last_subname)) {
        n = sizeof(vi->stats.last_subname) - 1;
    }
    if (n > 0 && name.ptr) {
        memcpy(vi->stats.last_subname, name.ptr, n);
    }
    vi->stats.last_subname[n] = '\0';
    vi->stats.last_subname_len = n;
}

static void copy_src_text(v5_impl *vi, nytp_string_view text)
{
    size_t n = text.len;
    if (n >= sizeof(vi->stats.last_src_text)) {
        n = sizeof(vi->stats.last_src_text) - 1;
    }
    if (n > 0 && text.ptr) {
        memcpy(vi->stats.last_src_text, text.ptr, n);
    }
    vi->stats.last_src_text[n] = '\0';
    vi->stats.last_src_text_len = n;
}

static v5_impl *vi_of(nytp_sink *sink)
{
    return (v5_impl *)sink->impl;
}

static const char *v5_name(const nytp_sink *sink)
{
    (void)sink;
    return "v5";
}

static nytp_status v5_activate(nytp_sink *sink)
{
    (void)sink;
    return NYTP_OK;
}

static nytp_status write_to_path(v5_impl *vi)
{
    FILE *fp;
    size_t nw;
    if (!vi->path) {
        return NYTP_OK;
    }
    fp = fopen(vi->path, "wb");
    if (!fp) {
        return NYTP_ERR_IO;
    }
    if (vi->len > 0) {
        nw = fwrite(vi->buf, 1, vi->len, fp);
        if (nw != vi->len) {
            fclose(fp);
            return NYTP_ERR_IO;
        }
    }
    if (fclose(fp) != 0) {
        return NYTP_ERR_IO;
    }
    vi->file_written = 1;
    return NYTP_OK;
}

static nytp_status atomic_replace_path(const char *path, const uint8_t *data,
                                       size_t n)
{
    char tmp[4096];
    int fd;
    size_t off = 0;
    int ntmp;

    if (path == NULL) {
        return NYTP_OK;
    }
    ntmp = snprintf(tmp, sizeof(tmp), "%s.tmp", path);
    if (ntmp < 0 || (size_t)ntmp >= sizeof(tmp)) {
        return NYTP_ERR_OVERFLOW;
    }
    fd = open(tmp, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) {
        return NYTP_ERR_IO;
    }
    while (off < n) {
        ssize_t w = write(fd, data + off, n - off);
        if (w < 0) {
            if (errno == EINTR) {
                continue;
            }
            (void)close(fd);
            (void)unlink(tmp);
            return NYTP_ERR_IO;
        }
        off += (size_t)w;
    }
    if (fsync(fd) != 0) {
        (void)close(fd);
        (void)unlink(tmp);
        return NYTP_ERR_IO;
    }
    if (close(fd) != 0) {
        (void)unlink(tmp);
        return NYTP_ERR_IO;
    }
    if (rename(tmp, path) != 0) {
        (void)unlink(tmp);
        return NYTP_ERR_IO;
    }
    return NYTP_OK;
}

static nytp_status sealed_zlib_copy(v5_impl *vi, uint8_t **outp, size_t *outn)
{
    size_t prefix = vi->header_end;
    const uint8_t *body;
    size_t blen;
    int level;
    z_stream zs;
    uint8_t *out;
    size_t cap;
    size_t used;
    int zst;

    if (prefix > vi->len) {
        return NYTP_ERR_OVERFLOW;
    }
    body = vi->buf + prefix;
    blen = vi->len - prefix;
    level = vi->compress_level > 0 ? vi->compress_level
                                   : NYTP_V5_DEFAULT_COMPRESS;
    cap = prefix + 1u + blen + 128u;
    if (cap < prefix + 256u) {
        cap = prefix + 256u;
    }
    out = (uint8_t *)malloc(cap);
    if (!out) {
        return NYTP_ERR_IO;
    }
    if (prefix > 0) {
        memcpy(out, vi->buf, prefix);
    }
    out[prefix] = NYTP_TAG_START_DEFLATE;
    used = prefix + 1u;
    memset(&zs, 0, sizeof(zs));
    zst = deflateInit2(&zs, level, Z_DEFLATED, 15, 8, Z_DEFAULT_STRATEGY);
    if (zst != Z_OK) {
        free(out);
        return NYTP_ERR_IO;
    }
    zs.next_in = (Bytef *)(uintptr_t)body;
    zs.avail_in = (uInt)blen;
    for (;;) {
        size_t room;
        if (used + 256u > cap) {
            size_t ncap = cap * 2u;
            uint8_t *nbuf;
            if (ncap < used + 256u) {
                ncap = used + 256u;
            }
            nbuf = (uint8_t *)realloc(out, ncap);
            if (!nbuf) {
                (void)deflateEnd(&zs);
                free(out);
                return NYTP_ERR_IO;
            }
            out = nbuf;
            cap = ncap;
        }
        room = cap - used;
        zs.next_out = out + used;
        zs.avail_out = (uInt)(room > 0xFFFFu ? 0xFFFFu : room);
        zst = deflate(&zs, Z_FINISH);
        used = (size_t)(zs.next_out - out);
        if (zst == Z_STREAM_END) {
            break;
        }
        if (zst != Z_OK && zst != Z_BUF_ERROR) {
            (void)deflateEnd(&zs);
            free(out);
            return NYTP_ERR_IO;
        }
    }
    (void)deflateEnd(&zs);
    *outp = out;
    *outn = used;
    return NYTP_OK;
}

nytp_status nytp_v5_sink_set_durable(nytp_sink *sink, int durable)
{
    v5_impl *vi;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return NYTP_ERR_NULL;
    }
    vi = (v5_impl *)sink->impl;
    if (vi->deflating) {
        return NYTP_ERR_STATE;
    }
    vi->durable = durable ? 1 : 0;
    return NYTP_OK;
}

int nytp_v5_sink_is_durable(const nytp_sink *sink)
{
    v5_impl *vi;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return 0;
    }
    vi = (v5_impl *)sink->impl;
    return vi->durable;
}

nytp_status nytp_v5_sink_mark_header_end(nytp_sink *sink)
{
    v5_impl *vi;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return NYTP_ERR_NULL;
    }
    vi = (v5_impl *)sink->impl;
    vi->header_end = vi->len;
    return NYTP_OK;
}

size_t nytp_v5_sink_header_end(const nytp_sink *sink)
{
    v5_impl *vi;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return 0;
    }
    vi = (v5_impl *)sink->impl;
    return vi->header_end;
}

size_t nytp_v5_sink_len_at_last_seal(const nytp_sink *sink)
{
    v5_impl *vi;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return 0;
    }
    vi = (v5_impl *)sink->impl;
    return vi->len_at_last_seal;
}

nytp_status nytp_v5_seal_publish(nytp_sink *sink)
{
    v5_impl *vi;
    nytp_status st;
    uint8_t *copy = NULL;
    size_t copy_len = 0;
    int need_free = 0;

    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return NYTP_ERR_NULL;
    }
    vi = (v5_impl *)sink->impl;
    if (vi->last_seal_ok && vi->len == vi->len_at_last_seal) {
        return NYTP_OK;
    }
    if (vi->header_end > vi->len) {
        return NYTP_ERR_OVERFLOW;
    }
    if (vi->compress_level > 0) {
        st = sealed_zlib_copy(vi, &copy, &copy_len);
        if (st != NYTP_OK) {
            return st;
        }
        need_free = 1;
    } else {
        copy = vi->buf;
        copy_len = vi->len;
    }
    if (vi->path) {
        st = atomic_replace_path(vi->path, copy, copy_len);
        if (st != NYTP_OK) {
            if (need_free) {
                free(copy);
            }
            return st;
        }
        vi->file_written = 1;
    }
    if (need_free) {
        free(copy);
    }
    vi->len_at_last_seal = vi->len;
    vi->last_seal_ok = 1;
    return NYTP_OK;
}

static nytp_status v5_flush(nytp_sink *sink)
{
    v5_impl *vi = vi_of(sink);
    nytp_status st;
    if (vi->durable) {
        return nytp_v5_seal_publish(sink);
    }
    /*
     * Mid-stream flush does **not** call Z_FINISH. Path/buffer after flush
     * while deflating is an unfinished zlib snapshot — **not** decoder-ready.
     * Only post-close (deflate_finish) bytes are a complete profile stream.
     * Non-deflating streams already hold complete records.
     */
    if (vi->path) {
        st = write_to_path(vi);
        if (st != NYTP_OK) {
            return st;
        }
    }
    return NYTP_OK;
}

static nytp_status v5_close(nytp_sink *sink)
{
    v5_impl *vi = vi_of(sink);
    nytp_status st;
    if (vi->durable) {
        return nytp_v5_seal_publish(sink);
    }
    st = deflate_finish(vi);
    if (st != NYTP_OK) {
        return st;
    }
    if (vi->path) {
        st = write_to_path(vi);
        if (st != NYTP_OK) {
            return st;
        }
    }
    return NYTP_OK;
}

static void v5_destroy(nytp_sink *sink)
{
    v5_impl *vi;
    if (!sink) {
        return;
    }
    vi = (v5_impl *)sink->impl;
    if (vi) {
        if (vi->deflating && !vi->deflate_finished) {
            (void)deflateEnd(&vi->zs);
        }
        free(vi->path);
        free(vi->buf);
        free(vi);
    }
    free(sink);
}

/* ---- emit ops ---- */

static nytp_status v5_emit_attribute(nytp_sink *sink, nytp_string_view key,
                                     nytp_string_view value)
{
    v5_impl *vi = vi_of(sink);
    nytp_status st = write_plain_kv(vi, NYTP_TAG_ATTRIBUTE, key, value);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_ATTRIBUTE);
    return NYTP_OK;
}

static nytp_status v5_emit_option(nytp_sink *sink, nytp_string_view key,
                                  nytp_string_view value)
{
    v5_impl *vi = vi_of(sink);
    nytp_status st = write_plain_kv(vi, NYTP_TAG_OPTION, key, value);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_OPTION);
    return NYTP_OK;
}

static nytp_status v5_emit_comment(nytp_sink *sink, nytp_string_view text)
{
    v5_impl *vi = vi_of(sink);
    unsigned char tag = NYTP_TAG_COMMENT;
    const char nl = '\n';
    nytp_status st = check_str_view(text);
    if (st != NYTP_OK) {
        return st;
    }
    st = buf_write(vi, &tag, 1);
    if (st != NYTP_OK) {
        return st;
    }
    if (text.len) {
        st = buf_write(vi, text.ptr, text.len);
        if (st != NYTP_OK) {
            return st;
        }
    }
    st = buf_write(vi, &nl, 1);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_COMMENT);
    return NYTP_OK;
}

static nytp_status v5_emit_time_line(nytp_sink *sink, nytp_ticks ticks,
                                     nytp_fid fid, nytp_line line)
{
    v5_impl *vi = vi_of(sink);
    int32_t elapsed;
    nytp_status st = ticks_to_i32(ticks, &elapsed);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_tag_i32(vi, NYTP_TAG_TIME_LINE, elapsed);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, fid);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, line);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_TIME_LINE);
    vi->stats.last_ticks = ticks;
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    vi->stats.last_block_line = 0;
    vi->stats.last_sub_line = 0;
    return NYTP_OK;
}

static nytp_status v5_emit_time_block(nytp_sink *sink, nytp_ticks ticks,
                                      nytp_fid fid, nytp_line line,
                                      nytp_line block_line, nytp_line sub_line)
{
    v5_impl *vi = vi_of(sink);
    int32_t elapsed;
    nytp_status st = ticks_to_i32(ticks, &elapsed);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_tag_i32(vi, NYTP_TAG_TIME_BLOCK, elapsed);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, fid);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, line);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, block_line);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, sub_line);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_TIME_BLOCK);
    vi->stats.last_ticks = ticks;
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    vi->stats.last_block_line = block_line;
    vi->stats.last_sub_line = sub_line;
    return NYTP_OK;
}

static nytp_status v5_emit_discount(nytp_sink *sink)
{
    v5_impl *vi = vi_of(sink);
    unsigned char tag = NYTP_TAG_DISCOUNT;
    nytp_status st = buf_write(vi, &tag, 1);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_DISCOUNT);
    return NYTP_OK;
}

static nytp_status v5_emit_new_fid(nytp_sink *sink, nytp_fid fid,
                                   nytp_fid eval_fid, nytp_line eval_line,
                                   uint32_t flags, uint32_t size,
                                   uint32_t mtime, nytp_string_view name)
{
    v5_impl *vi = vi_of(sink);
    nytp_status st = check_str_view(name);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_tag_u32(vi, NYTP_TAG_NEW_FID, fid);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, eval_fid);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, eval_line);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, flags);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, size);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, mtime);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_str(vi, name);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_NEW_FID);
    vi->stats.last_fid = fid;
    return NYTP_OK;
}

static nytp_status v5_emit_src_line(nytp_sink *sink, nytp_fid fid,
                                    nytp_line line, nytp_string_view text)
{
    v5_impl *vi = vi_of(sink);
    nytp_status st = check_str_view(text);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_tag_u32(vi, NYTP_TAG_SRC_LINE, fid);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, line);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_str(vi, text);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_SRC_LINE);
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    vi->stats.last_src_fid = fid;
    vi->stats.last_src_line = line;
    copy_src_text(vi, text);
    return NYTP_OK;
}

static nytp_status v5_emit_sub_info(nytp_sink *sink, nytp_fid fid,
                                    nytp_line first_line, nytp_line last_line,
                                    nytp_string_view name)
{
    v5_impl *vi = vi_of(sink);
    /* Wire: fid, name, first, last (callback order differs). */
    nytp_status st = check_str_view(name);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_tag_u32(vi, NYTP_TAG_SUB_INFO, fid);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_str(vi, name);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, first_line);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, last_line);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_SUB_INFO);
    vi->stats.last_fid = fid;
    vi->stats.last_line = first_line;
    vi->stats.last_block_line = last_line;
    copy_subname(vi, name);
    return NYTP_OK;
}

static nytp_status v5_emit_sub_callers(nytp_sink *sink, nytp_fid fid,
                                       nytp_line line, uint32_t count,
                                       double incl, double excl, double reci,
                                       uint32_t rec_depth,
                                       nytp_string_view called,
                                       nytp_string_view caller)
{
    v5_impl *vi = vi_of(sink);
    /* Wire: fid, line, caller, count, incl, excl, reci, rec_depth, called */
    nytp_status st = check_str_view(caller);
    if (st != NYTP_OK) {
        return st;
    }
    st = check_str_view(called);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_tag_u32(vi, NYTP_TAG_SUB_CALLERS, fid);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, line);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_str(vi, caller);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, count);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_nv(vi, incl);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_nv(vi, excl);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_nv(vi, reci);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, rec_depth);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_str(vi, called);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_SUB_CALLERS);
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    copy_subname(vi, called);
    return NYTP_OK;
}

static nytp_status v5_emit_pid_start(nytp_sink *sink, nytp_pid pid,
                                     nytp_pid ppid, double start_time)
{
    v5_impl *vi = vi_of(sink);
    nytp_status st = write_tag_u32(vi, NYTP_TAG_PID_START, pid);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, ppid);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_nv(vi, start_time);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_PID_START);
    vi->stats.last_fid = (nytp_fid)pid;
    return NYTP_OK;
}

static nytp_status v5_emit_pid_end(nytp_sink *sink, nytp_pid pid,
                                   double end_time)
{
    v5_impl *vi = vi_of(sink);
    nytp_status st = write_tag_u32(vi, NYTP_TAG_PID_END, pid);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_nv(vi, end_time);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_PID_END);
    vi->stats.last_fid = (nytp_fid)pid;
    return NYTP_OK;
}

static nytp_status v5_emit_sub_entry(nytp_sink *sink, nytp_fid caller_fid,
                                     nytp_line caller_line)
{
    v5_impl *vi = vi_of(sink);
    nytp_status st = write_tag_u32(vi, NYTP_TAG_SUB_ENTRY, caller_fid);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_u32(vi, caller_line);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_SUB_ENTRY);
    vi->stats.last_fid = caller_fid;
    vi->stats.last_line = caller_line;
    return NYTP_OK;
}

static nytp_status v5_emit_sub_return(nytp_sink *sink, nytp_depth depth,
                                      double incl_time, double excl_time,
                                      nytp_string_view subname)
{
    v5_impl *vi = vi_of(sink);
    nytp_status st = check_str_view(subname);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_tag_u32(vi, NYTP_TAG_SUB_RETURN, depth);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_nv(vi, incl_time);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_nv(vi, excl_time);
    if (st != NYTP_OK) {
        return st;
    }
    st = write_str(vi, subname);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(vi, NYTP_EVT_SUB_RETURN);
    vi->stats.last_depth = depth;
    copy_subname(vi, subname);
    return NYTP_OK;
}

/*
 * START_DEFLATE: write tag 'z', then switch subsequent writes to zlib.
 * Matches FileHandle.xs NYTP_start_deflate (windowBits=15, default strategy).
 * Optional preceding comment is left to the caller (oracle writes one).
 */
static nytp_status v5_emit_start_deflate(nytp_sink *sink)
{
    v5_impl *vi = vi_of(sink);
    unsigned char tag = NYTP_TAG_START_DEFLATE;
    int level;
    int zst;
    nytp_status st;

    if (vi->durable) {
        return NYTP_ERR_STATE;
    }
    if (vi->deflating || vi->deflate_finished) {
        return NYTP_ERR_STATE;
    }
    st = buf_write(vi, &tag, 1);
    if (st != NYTP_OK) {
        return st;
    }
    level = vi->compress_level > 0 ? vi->compress_level
                                   : NYTP_V5_DEFAULT_COMPRESS;
    memset(&vi->zs, 0, sizeof(vi->zs));
    zst = deflateInit2(&vi->zs, level, Z_DEFLATED, 15 /* windowBits */,
                       8 /* memLevel */, Z_DEFAULT_STRATEGY);
    if (zst != Z_OK) {
        return NYTP_ERR_IO;
    }
    vi->deflating = 1;
    note_kind(vi, NYTP_EVT_START_DEFLATE);
    return NYTP_OK;
}

static const nytp_sink_ops v5_ops = {
    .name = v5_name,
    .activate = v5_activate,
    .flush = v5_flush,
    .close = v5_close,
    .destroy = v5_destroy,
    .emit_attribute = v5_emit_attribute,
    .emit_option = v5_emit_option,
    .emit_comment = v5_emit_comment,
    .emit_time_line = v5_emit_time_line,
    .emit_time_block = v5_emit_time_block,
    .emit_discount = v5_emit_discount,
    .emit_new_fid = v5_emit_new_fid,
    .emit_src_line = v5_emit_src_line,
    .emit_sub_info = v5_emit_sub_info,
    .emit_sub_callers = v5_emit_sub_callers,
    .emit_pid_start = v5_emit_pid_start,
    .emit_pid_end = v5_emit_pid_end,
    .emit_sub_entry = v5_emit_sub_entry,
    .emit_sub_return = v5_emit_sub_return,
    .emit_start_deflate = v5_emit_start_deflate,
    .on_logical_committed = v5_on_logical_committed,
};

static nytp_sink *v5_create_common(const char *path, int compress_level)
{
    nytp_sink *sink = (nytp_sink *)calloc(1, sizeof(*sink));
    v5_impl *vi = (v5_impl *)calloc(1, sizeof(*vi));
    static const char header[] = "NYTProf 5 0\n";
    if (!sink || !vi) {
        free(sink);
        free(vi);
        return NULL;
    }
    if (path) {
        size_t n = strlen(path);
        vi->path = (char *)malloc(n + 1);
        if (!vi->path) {
            free(vi);
            free(sink);
            return NULL;
        }
        memcpy(vi->path, path, n + 1);
    }
    vi->compress_level = compress_level;
    if (buf_append_raw(vi, header, sizeof(header) - 1) != NYTP_OK) {
        free(vi->path);
        free(vi->buf);
        free(vi);
        free(sink);
        return NULL;
    }
    vi->header_ok = 1;
    vi->durable = 0;
    vi->header_end = vi->len;
    vi->len_at_last_seal = 0;
    vi->last_seal_ok = 0;
    sink->ops = &v5_ops;
    sink->state = NYTP_SINK_OPEN;
    sink->impl = vi;
    sink->next_seq = 0;
    sink->last_seq = 0;
    sink->has_last_seq = 0;
    sink->fail_reason = NYTP_OK;
    return sink;
}

nytp_sink *nytp_v5_sink_create(const char *path)
{
    return v5_create_common(path, 0);
}

nytp_sink *nytp_v5_sink_create_ex(const char *path, int compress_level)
{
    if (compress_level < 0 || compress_level > 9) {
        return NULL;
    }
    return v5_create_common(path, compress_level);
}

int nytp_v5_sink_is_v5(const nytp_sink *sink)
{
    return sink != NULL && sink->ops == &v5_ops;
}

const nytp_counting_stats *nytp_v5_sink_stats(const nytp_sink *sink)
{
    v5_impl *vi;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return NULL;
    }
    vi = (v5_impl *)sink->impl;
    return &vi->stats;
}

const char *nytp_v5_sink_path(const nytp_sink *sink)
{
    v5_impl *vi;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return NULL;
    }
    vi = (v5_impl *)sink->impl;
    return vi->path;
}

const uint8_t *nytp_v5_sink_wire(const nytp_sink *sink, size_t *out_len)
{
    v5_impl *vi;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        if (out_len) {
            *out_len = 0;
        }
        return NULL;
    }
    vi = (v5_impl *)sink->impl;
    if (out_len) {
        *out_len = vi->len;
    }
    return vi->buf;
}

size_t nytp_v5_sink_wire_len(const nytp_sink *sink)
{
    size_t n = 0;
    (void)nytp_v5_sink_wire(sink, &n);
    return n;
}

int nytp_v5_sink_file_written(const nytp_sink *sink)
{
    v5_impl *vi;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return 0;
    }
    vi = (v5_impl *)sink->impl;
    return vi->file_written ? 1 : 0;
}

int nytp_v5_sink_is_deflating(const nytp_sink *sink)
{
    v5_impl *vi;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return 0;
    }
    vi = (v5_impl *)sink->impl;
    return (vi->deflating || vi->deflate_finished) ? 1 : 0;
}

void nytp_v5_sink_version(const nytp_sink *sink, uint32_t *major,
                          uint32_t *minor)
{
    (void)sink;
    if (major) {
        *major = NYTP_V5_MAJOR;
    }
    if (minor) {
        *minor = NYTP_V5_MINOR;
    }
}

nytp_status nytp_v5_sink_detach_path(nytp_sink *sink)
{
    return nytp_v5_sink_rebind_path(sink, NULL);
}

nytp_status nytp_v5_sink_rebind_path(nytp_sink *sink, const char *path)
{
    v5_impl *vi;
    char *np = NULL;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return NYTP_ERR_NULL;
    }
    if (sink->state == NYTP_SINK_CLOSED || sink->state == NYTP_SINK_FAILED) {
        return NYTP_ERR_STATE;
    }
    vi = (v5_impl *)sink->impl;
    if (path) {
        size_t n = strlen(path);
        np = (char *)malloc(n + 1);
        if (!np) {
            return NYTP_ERR_IO;
        }
        memcpy(np, path, n + 1);
    }
    free(vi->path);
    vi->path = np;
    vi->file_written = 0;
    return NYTP_OK;
}

nytp_status nytp_v5_sink_fork_child_reinit(nytp_sink *sink, const char *new_path)
{
    v5_impl *vi;
    static const char header[] = "NYTProf 5 0\n";
    nytp_status st;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return NYTP_ERR_NULL;
    }
    if (sink->state == NYTP_SINK_CLOSED || sink->state == NYTP_SINK_FAILED) {
        return NYTP_ERR_STATE;
    }
    vi = (v5_impl *)sink->impl;

    /* Abort inherited compressor — child starts a clean stream (COL-015). */
    if (vi->deflating && !vi->deflate_finished) {
        (void)deflateEnd(&vi->zs);
    }
    vi->deflating = 0;
    vi->deflate_finished = 0;
    memset(&vi->zs, 0, sizeof(vi->zs));

    st = nytp_v5_sink_rebind_path(sink, new_path);
    if (st != NYTP_OK) {
        return st;
    }

    vi->len = 0;
    vi->file_written = 0;
    vi->header_ok = 0;
    memset(&vi->stats, 0, sizeof(vi->stats));
    st = buf_append_raw(vi, header, sizeof(header) - 1);
    if (st != NYTP_OK) {
        return st;
    }
    vi->header_ok = 1;
    vi->header_end = vi->len;
    vi->len_at_last_seal = 0;
    vi->last_seal_ok = 0;
    return NYTP_OK;
}

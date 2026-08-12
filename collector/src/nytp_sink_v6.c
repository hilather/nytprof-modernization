/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-007 (PR-B06) — Absolute provisional v6 wire sink (codec NONE EVENT).
 *
 * Layout matches crates/nytprof-format-v6 encode_file_prefix + encode_chunk_frame
 * + absolute event_body (no packing flags). IDs from nytprof_v6_ids.h.
 */
#include "nytp_sink_v6.h"
#include "nytprof_v6_ids.h"

#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define NYTP_V6_INIT_CAP 4096u

typedef struct v6_impl {
    nytp_counting_stats stats;
    char *path;
    /* Sealed file bytes: prefix [+ EVENT chunk]. Grows on create/seal. */
    uint8_t *wire;
    size_t wire_len;
    size_t wire_cap;
    /* Open event-body (absolute records) until sealed. */
    uint8_t *body;
    size_t body_len;
    size_t body_cap;
    uint16_t minor;
    uint64_t required_features;
    uint64_t optional_features;
    uint32_t header_crc;
    uint32_t event_count; /* logical EVENT records (not START_DEFLATE alone? all body recs) */
    int sealed;
    int file_written;
    int header_ok;
} v6_impl;

/* ---- buffer helpers ---- */

static nytp_status buf_reserve(uint8_t **buf, size_t *len, size_t *cap,
                               size_t need_extra)
{
    size_t need;
    size_t ncap;
    uint8_t *nbuf;

    if (need_extra > SIZE_MAX - *len) {
        return NYTP_ERR_OVERFLOW;
    }
    need = *len + need_extra;
    if (need <= *cap) {
        return NYTP_OK;
    }
    ncap = *cap ? *cap : NYTP_V6_INIT_CAP;
    while (ncap < need) {
        if (ncap > SIZE_MAX / 2u) {
            ncap = need;
            break;
        }
        ncap *= 2u;
    }
    nbuf = (uint8_t *)realloc(*buf, ncap);
    if (!nbuf) {
        return NYTP_ERR_IO;
    }
    *buf = nbuf;
    *cap = ncap;
    return NYTP_OK;
}

static nytp_status buf_append(uint8_t **buf, size_t *len, size_t *cap,
                              const void *p, size_t n)
{
    nytp_status st;
    if (n == 0) {
        return NYTP_OK;
    }
    st = buf_reserve(buf, len, cap, n);
    if (st != NYTP_OK) {
        return st;
    }
    memcpy(*buf + *len, p, n);
    *len += n;
    return NYTP_OK;
}

static nytp_status wire_append(v6_impl *vi, const void *p, size_t n)
{
    return buf_append(&vi->wire, &vi->wire_len, &vi->wire_cap, p, n);
}

static nytp_status body_append(v6_impl *vi, const void *p, size_t n)
{
    /* Fail-closed before exceeding MAX_EVENT_BODY_BYTES. */
    if (n > NYTPROF_V6_MAX_EVENT_BODY_BYTES ||
        vi->body_len > NYTPROF_V6_MAX_EVENT_BODY_BYTES - n) {
        return NYTP_ERR_OVERFLOW;
    }
    return buf_append(&vi->body, &vi->body_len, &vi->body_cap, p, n);
}

/* Checkpoint / rollback so a failed multi-field emit never leaves a truncated record. */
static void body_rollback(v6_impl *vi, size_t mark)
{
    if (mark <= vi->body_len) {
        vi->body_len = mark;
    }
}

/*
 * On any emit failure after body bytes may have been written: restore mark.
 * Call only for non-OK statuses from mid-record paths.
 */
static nytp_status emit_fail(v6_impl *vi, size_t mark, nytp_status st)
{
    body_rollback(vi, mark);
    return st;
}

/* ---- ULEB128 (canonical, matches Rust encode_u64) ---- */

static size_t uleb_encode(uint64_t value, uint8_t out[10])
{
    size_t n = 0;
    for (;;) {
        uint8_t byte = (uint8_t)(value & 0x7fu);
        value >>= 7;
        if (value != 0) {
            byte |= 0x80u;
            out[n++] = byte;
        } else {
            out[n++] = byte;
            break;
        }
    }
    return n;
}

static nytp_status body_uleb(v6_impl *vi, uint64_t value)
{
    uint8_t tmp[10];
    size_t n = uleb_encode(value, tmp);
    return body_append(vi, tmp, n);
}

static nytp_status body_u8(v6_impl *vi, uint8_t b)
{
    return body_append(vi, &b, 1);
}

/* string-blob: ULEB id || ULEB len || u8 flags || bytes */
static nytp_status check_str_view(nytp_string_view sv)
{
    if (sv.len > 0 && !sv.ptr) {
        return NYTP_ERR_NULL;
    }
    if ((uint64_t)sv.len > NYTPROF_V6_MAX_STRING_BYTES) {
        return NYTP_ERR_OVERFLOW;
    }
    return NYTP_OK;
}

static nytp_status body_string_blob(v6_impl *vi, uint64_t string_id,
                                    uint8_t flags, nytp_string_view sv)
{
    nytp_status st = check_str_view(sv);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_uleb(vi, string_id);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_uleb(vi, (uint64_t)sv.len);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_u8(vi, flags);
    if (st != NYTP_OK) {
        return st;
    }
    if (sv.len == 0) {
        return NYTP_OK;
    }
    return body_append(vi, sv.ptr, sv.len);
}

static uint8_t utf8_flag(nytp_string_view sv)
{
    return sv.is_utf8 ? (uint8_t)NYTPROF_V6_FLAG_UTF8 : 0u;
}

/* opcode ULEB + flags=0 (absolute; no packing bits). */
static nytp_status body_op(v6_impl *vi, uint64_t opcode)
{
    nytp_status st = body_uleb(vi, opcode);
    if (st != NYTP_OK) {
        return st;
    }
    return body_u8(vi, 0); /* absolute flags */
}

/* Project signed ticks to u64 absolute; fail closed on negative. */
static nytp_status ticks_to_u64(nytp_ticks t, uint64_t *out)
{
    if (t < 0) {
        return NYTP_ERR_OVERFLOW;
    }
    *out = (uint64_t)t;
    return NYTP_OK;
}

/*
 * Project NV double to non-negative integer ULEB domain.
 * Finite, >= 0, and <= UINT64_MAX; truncated toward zero.
 * NaN / Inf / negative / out-of-range → OVERFLOW.
 */
static nytp_status nv_to_u64(double d, uint64_t *out)
{
    /* NaN (d != d), negative, +Inf / out-of-u64-range → fail closed. */
    if (d != d || d < 0.0 || d >= 18446744073709551616.0) { /* 2^64 */
        return NYTP_ERR_OVERFLOW;
    }
    *out = (uint64_t)d;
    return NYTP_OK;
}

/* ---- stats helpers ---- */

static void note_kind(v6_impl *vi, nytp_event_kind kind)
{
    if ((unsigned)kind < (unsigned)NYTP_EVT_KIND_COUNT) {
        vi->stats.by_kind[kind]++;
    }
    vi->stats.total_emits++;
    vi->stats.last_kind = kind;
}

static void v6_on_logical_committed(nytp_sink *sink, nytp_seq seq,
                                    nytp_event_kind kind)
{
    v6_impl *vi;
    if (!sink || !sink->impl) {
        return;
    }
    vi = (v6_impl *)sink->impl;
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

static void copy_subname(v6_impl *vi, nytp_string_view name)
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

static void copy_src_text(v6_impl *vi, nytp_string_view text)
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

static v6_impl *vi_of(nytp_sink *sink)
{
    return (v6_impl *)sink->impl;
}

static const char *v6_name(const nytp_sink *sink)
{
    (void)sink;
    return "v6-abs";
}

static nytp_status v6_activate(nytp_sink *sink)
{
    (void)sink;
    return NYTP_OK;
}

/* ---- file prefix + chunk seal ---- */

static nytp_status write_file_prefix(v6_impl *vi)
{
    uint8_t hdr[NYTPROF_V6_HEADER_LEN_FULL];
    uint8_t end_tlv[16];
    size_t end_n;
    static const uint8_t magic[8] = {
        NYTPROF_V6_MAGIC_0, NYTPROF_V6_MAGIC_1, NYTPROF_V6_MAGIC_2,
        NYTPROF_V6_MAGIC_3, NYTPROF_V6_MAGIC_4, NYTPROF_V6_MAGIC_5,
        NYTPROF_V6_MAGIC_6, NYTPROF_V6_MAGIC_7};
    uint16_t major = (uint16_t)NYTPROF_V6_SUPPORTED_MAJOR;
    uint32_t header_len = NYTPROF_V6_HEADER_LEN_FULL;
    nytp_status st;

    memset(hdr, 0, sizeof(hdr));
    memcpy(hdr + 0, magic, 8);
    hdr[8] = (uint8_t)(major & 0xffu);
    hdr[9] = (uint8_t)((major >> 8) & 0xffu);
    hdr[10] = (uint8_t)(vi->minor & 0xffu);
    hdr[11] = (uint8_t)((vi->minor >> 8) & 0xffu);
    hdr[12] = (uint8_t)(header_len & 0xffu);
    hdr[13] = (uint8_t)((header_len >> 8) & 0xffu);
    hdr[14] = (uint8_t)((header_len >> 16) & 0xffu);
    hdr[15] = (uint8_t)((header_len >> 24) & 0xffu);
    {
        uint64_t rf = vi->required_features;
        uint64_t of = vi->optional_features;
        uint32_t crc = vi->header_crc;
        size_t i;
        for (i = 0; i < 8; i++) {
            hdr[16 + i] = (uint8_t)((rf >> (8 * i)) & 0xffu);
            hdr[24 + i] = (uint8_t)((of >> (8 * i)) & 0xffu);
        }
        hdr[32] = (uint8_t)(crc & 0xffu);
        hdr[33] = (uint8_t)((crc >> 8) & 0xffu);
        hdr[34] = (uint8_t)((crc >> 16) & 0xffu);
        hdr[35] = (uint8_t)((crc >> 24) & 0xffu);
    }
    st = wire_append(vi, hdr, sizeof(hdr));
    if (st != NYTP_OK) {
        return st;
    }
    /* Empty multi-TLV region: single END terminator (type_id=0x7e, len=0, flags=0). */
    end_n = 0;
    end_n += uleb_encode(NYTPROF_V6_TLV_END, end_tlv + end_n);
    end_n += uleb_encode(0, end_tlv + end_n);
    end_tlv[end_n++] = 0; /* flags */
    return wire_append(vi, end_tlv, end_n);
}

static nytp_status encode_chunk_frame(v6_impl *vi, uint8_t kind, uint8_t codec,
                                      uint16_t flags, uint64_t sequence,
                                      uint64_t first_logical,
                                      uint32_t logical_count,
                                      uint32_t uncompressed_len,
                                      const uint8_t *payload, size_t payload_len,
                                      uint32_t checksum)
{
    uint8_t hdr[NYTPROF_V6_CHUNK_HEADER_LEN];
    uint32_t sync = NYTPROF_V6_CHUNK_SYNC;
    uint32_t compressed_len;
    size_t i;
    nytp_status st;

    if (payload_len > NYTPROF_V6_MAX_CHUNK_PAYLOAD) {
        return NYTP_ERR_OVERFLOW;
    }
    if (uncompressed_len > NYTPROF_V6_MAX_CHUNK_PAYLOAD) {
        return NYTP_ERR_OVERFLOW;
    }
    compressed_len = (uint32_t)payload_len;

    memset(hdr, 0, sizeof(hdr));
    for (i = 0; i < 4; i++) {
        hdr[i] = (uint8_t)((sync >> (8 * i)) & 0xffu);
    }
    hdr[4] = kind;
    hdr[5] = codec;
    hdr[6] = (uint8_t)(flags & 0xffu);
    hdr[7] = (uint8_t)((flags >> 8) & 0xffu);
    for (i = 0; i < 8; i++) {
        hdr[8 + i] = (uint8_t)((sequence >> (8 * i)) & 0xffu);
        hdr[16 + i] = (uint8_t)((first_logical >> (8 * i)) & 0xffu);
    }
    for (i = 0; i < 4; i++) {
        hdr[24 + i] = (uint8_t)((logical_count >> (8 * i)) & 0xffu);
        hdr[28 + i] = (uint8_t)((uncompressed_len >> (8 * i)) & 0xffu);
        hdr[32 + i] = (uint8_t)((compressed_len >> (8 * i)) & 0xffu);
        hdr[36 + i] = (uint8_t)((checksum >> (8 * i)) & 0xffu);
    }
    st = wire_append(vi, hdr, sizeof(hdr));
    if (st != NYTP_OK) {
        return st;
    }
    if (payload_len) {
        st = wire_append(vi, payload, payload_len);
        if (st != NYTP_OK) {
            return st;
        }
    }
    return NYTP_OK;
}

static nytp_status seal_event_chunk(v6_impl *vi, const nytp_sink *sink)
{
    nytp_status st;
    uint32_t unc;
    if (vi->sealed) {
        return NYTP_OK;
    }
    /*
     * Sticky FAILED (OVERFLOW/IO after emit): do **not** seal a product
     * mini-profile. Emit paths checkpoint/rollback so open body has no
     * truncated record; discard remaining complete-but-failed-stream body
     * so close cannot report OK with a sealed EVENT after sticky fail.
     * Lifecycle close-from-FAILED still returns OK (prefix-only wire).
     */
    if (sink && sink->state == NYTP_SINK_FAILED) {
        vi->body_len = 0;
        vi->event_count = 0;
        vi->sealed = 1; /* finished; no EVENT chunk */
        return NYTP_OK;
    }
    if (vi->body_len > NYTPROF_V6_MAX_CHUNK_PAYLOAD ||
        vi->body_len > NYTPROF_V6_MAX_EVENT_BODY_BYTES) {
        return NYTP_ERR_OVERFLOW;
    }
    unc = (uint32_t)vi->body_len;
    /*
     * Empty body → no EVENT chunk (Rust encode_mini_profile empty parity).
     * Non-empty / event_count > 0 → one codec-NONE EVENT chunk.
     */
    if (vi->body_len > 0 || vi->event_count > 0) {
        st = encode_chunk_frame(vi, (uint8_t)NYTPROF_V6_KIND_EVENT,
                                (uint8_t)NYTPROF_V6_CODEC_NONE, 0 /* flags */,
                                0 /* sequence */, 0 /* first_logical */,
                                vi->event_count, unc, vi->body, vi->body_len,
                                0 /* checksum placeholder */);
        if (st != NYTP_OK) {
            return st;
        }
    }
    vi->sealed = 1;
    return NYTP_OK;
}

static nytp_status write_to_path(v6_impl *vi)
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
    if (vi->wire_len > 0) {
        nw = fwrite(vi->wire, 1, vi->wire_len, fp);
        if (nw != vi->wire_len) {
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

static nytp_status v6_flush(nytp_sink *sink)
{
    v6_impl *vi = vi_of(sink);
    /*
     * Mid-stream flush does **not** seal the EVENT chunk. Path/buffer after
     * flush while unsealed is prefix-only (+ open body not framed) — **not**
     * a complete mini-profile. Only post-close sealed bytes are decoder-ready.
     */
    if (vi->path && vi->sealed) {
        return write_to_path(vi);
    }
    if (vi->path && !vi->sealed) {
        /* Write prefix-only snapshot (honest unfinished residual). */
        return write_to_path(vi);
    }
    return NYTP_OK;
}

static nytp_status v6_close(nytp_sink *sink)
{
    v6_impl *vi = vi_of(sink);
    nytp_status st = seal_event_chunk(vi, sink);
    if (st != NYTP_OK) {
        /* Do not write a path file after refuse-seal / sticky fail. */
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

static void v6_destroy(nytp_sink *sink)
{
    v6_impl *vi;
    if (!sink) {
        return;
    }
    vi = (v6_impl *)sink->impl;
    if (vi) {
        free(vi->path);
        free(vi->wire);
        free(vi->body);
        free(vi);
    }
    free(sink);
}

/* ---- emit ops (absolute bodies) ---- */

static nytp_status after_record(v6_impl *vi, nytp_event_kind kind)
{
    if (vi->event_count == UINT32_MAX) {
        return NYTP_ERR_OVERFLOW;
    }
    vi->event_count++;
    note_kind(vi, kind);
    return NYTP_OK;
}

static nytp_status v6_emit_attribute(nytp_sink *sink, nytp_string_view key,
                                     nytp_string_view value)
{
    v6_impl *vi = vi_of(sink);
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = check_str_view(key);
    if (st != NYTP_OK) {
        return st;
    }
    st = check_str_view(value);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_op(vi, NYTPROF_V6_OP_ATTRIBUTE);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob(vi, 0, utf8_flag(key), key);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob(vi, 0, utf8_flag(value), value);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = after_record(vi, NYTP_EVT_ATTRIBUTE);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_option(nytp_sink *sink, nytp_string_view key,
                                  nytp_string_view value)
{
    v6_impl *vi = vi_of(sink);
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = check_str_view(key);
    if (st != NYTP_OK) {
        return st;
    }
    st = check_str_view(value);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_op(vi, NYTPROF_V6_OP_OPTION);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob(vi, 0, utf8_flag(key), key);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob(vi, 0, utf8_flag(value), value);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = after_record(vi, NYTP_EVT_OPTION);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_comment(nytp_sink *sink, nytp_string_view text)
{
    v6_impl *vi = vi_of(sink);
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = check_str_view(text);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_op(vi, NYTPROF_V6_OP_COMMENT);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob(vi, 0, utf8_flag(text), text);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = after_record(vi, NYTP_EVT_COMMENT);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_time_line(nytp_sink *sink, nytp_ticks ticks,
                                     nytp_fid fid, nytp_line line)
{
    v6_impl *vi = vi_of(sink);
    uint64_t ut;
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = ticks_to_u64(ticks, &ut);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_op(vi, NYTPROF_V6_OP_TIME_LINE);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)fid);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)line);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, ut);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_ticks = ticks;
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    vi->stats.last_block_line = 0;
    vi->stats.last_sub_line = 0;
    st = after_record(vi, NYTP_EVT_TIME_LINE);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_time_block(nytp_sink *sink, nytp_ticks ticks,
                                      nytp_fid fid, nytp_line line,
                                      nytp_line block_line, nytp_line sub_line)
{
    v6_impl *vi = vi_of(sink);
    uint64_t ut;
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    (void)sub_line; /* provisional absolute TIME_BLOCK has no sub_line field */
    mark = vi->body_len;
    st = ticks_to_u64(ticks, &ut);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_op(vi, NYTPROF_V6_OP_TIME_BLOCK);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)fid);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)line);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)block_line);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, ut);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_ticks = ticks;
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    vi->stats.last_block_line = block_line;
    vi->stats.last_sub_line = sub_line;
    st = after_record(vi, NYTP_EVT_TIME_BLOCK);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_discount(nytp_sink *sink)
{
    v6_impl *vi = vi_of(sink);
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = body_op(vi, NYTPROF_V6_OP_DISCOUNT);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = after_record(vi, NYTP_EVT_DISCOUNT);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_new_fid(nytp_sink *sink, nytp_fid fid,
                                   nytp_fid eval_fid, nytp_line eval_line,
                                   uint32_t flags, uint32_t size,
                                   uint32_t mtime, nytp_string_view name)
{
    v6_impl *vi = vi_of(sink);
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    /* Provisional absolute NEW_FID: fid + filename only. */
    (void)eval_fid;
    (void)eval_line;
    (void)flags;
    (void)size;
    (void)mtime;
    mark = vi->body_len;
    st = check_str_view(name);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_op(vi, NYTPROF_V6_OP_NEW_FID);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)fid);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob(vi, 0, utf8_flag(name), name);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_fid = fid;
    st = after_record(vi, NYTP_EVT_NEW_FID);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_src_line(nytp_sink *sink, nytp_fid fid,
                                    nytp_line line, nytp_string_view text)
{
    v6_impl *vi = vi_of(sink);
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = check_str_view(text);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_op(vi, NYTPROF_V6_OP_SRC_LINE);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)fid);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)line);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob(vi, 0, utf8_flag(text), text);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    vi->stats.last_src_fid = fid;
    vi->stats.last_src_line = line;
    copy_src_text(vi, text);
    st = after_record(vi, NYTP_EVT_SRC_LINE);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_sub_info(nytp_sink *sink, nytp_fid fid,
                                    nytp_line first_line, nytp_line last_line,
                                    nytp_string_view name)
{
    v6_impl *vi = vi_of(sink);
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = check_str_view(name);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_op(vi, NYTPROF_V6_OP_SUB_INFO);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)fid);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)first_line);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)last_line);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob(vi, 0, utf8_flag(name), name);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_fid = fid;
    vi->stats.last_line = first_line;
    vi->stats.last_block_line = last_line;
    copy_subname(vi, name);
    st = after_record(vi, NYTP_EVT_SUB_INFO);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_sub_callers(nytp_sink *sink, nytp_fid fid,
                                       nytp_line line, uint32_t count,
                                       double incl, double excl, double reci,
                                       uint32_t rec_depth,
                                       nytp_string_view called,
                                       nytp_string_view caller)
{
    v6_impl *vi = vi_of(sink);
    uint64_t ui, ue, ur;
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = check_str_view(called);
    if (st != NYTP_OK) {
        return st;
    }
    st = check_str_view(caller);
    if (st != NYTP_OK) {
        return st;
    }
    st = nv_to_u64(incl, &ui);
    if (st != NYTP_OK) {
        return st;
    }
    st = nv_to_u64(excl, &ue);
    if (st != NYTP_OK) {
        return st;
    }
    st = nv_to_u64(reci, &ur);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_op(vi, NYTPROF_V6_OP_SUB_CALLERS);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)fid);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)line);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)count);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, ui);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, ue);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, ur);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)rec_depth);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob(vi, 0, utf8_flag(called), called);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob(vi, 0, utf8_flag(caller), caller);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    copy_subname(vi, called);
    st = after_record(vi, NYTP_EVT_SUB_CALLERS);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_pid_start(nytp_sink *sink, nytp_pid pid,
                                     nytp_pid ppid, double start_time)
{
    v6_impl *vi = vi_of(sink);
    uint64_t ut;
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = nv_to_u64(start_time, &ut);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_op(vi, NYTPROF_V6_OP_PID_START);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)pid);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)ppid);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, ut);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_fid = (nytp_fid)pid;
    st = after_record(vi, NYTP_EVT_PID_START);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_pid_end(nytp_sink *sink, nytp_pid pid,
                                   double end_time)
{
    v6_impl *vi = vi_of(sink);
    uint64_t ut;
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = nv_to_u64(end_time, &ut);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_op(vi, NYTPROF_V6_OP_PID_END);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)pid);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, ut);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_fid = (nytp_fid)pid;
    st = after_record(vi, NYTP_EVT_PID_END);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_sub_entry(nytp_sink *sink, nytp_fid caller_fid,
                                     nytp_line caller_line)
{
    v6_impl *vi = vi_of(sink);
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = body_op(vi, NYTPROF_V6_OP_SUB_ENTRY);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)caller_fid);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)caller_line);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_fid = caller_fid;
    vi->stats.last_line = caller_line;
    st = after_record(vi, NYTP_EVT_SUB_ENTRY);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static nytp_status v6_emit_sub_return(nytp_sink *sink, nytp_depth depth,
                                      double incl_time, double excl_time,
                                      nytp_string_view subname)
{
    v6_impl *vi = vi_of(sink);
    uint64_t ui, ue;
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = check_str_view(subname);
    if (st != NYTP_OK) {
        return st;
    }
    st = nv_to_u64(incl_time, &ui);
    if (st != NYTP_OK) {
        return st;
    }
    st = nv_to_u64(excl_time, &ue);
    if (st != NYTP_OK) {
        return st;
    }
    st = body_op(vi, NYTPROF_V6_OP_SUB_RETURN);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, (uint64_t)depth);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, ui);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_uleb(vi, ue);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob(vi, 0, utf8_flag(subname), subname);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_depth = depth;
    copy_subname(vi, subname);
    st = after_record(vi, NYTP_EVT_SUB_RETURN);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

/*
 * START_DEFLATE: absolute empty marker opcode only (PR-B06).
 * Does **not** switch payload codecs (PR-B07) and is still a control event
 * for COL-003 (no logical seq via public wrappers).
 */
static nytp_status v6_emit_start_deflate(nytp_sink *sink)
{
    v6_impl *vi = vi_of(sink);
    size_t mark;
    nytp_status st;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    mark = vi->body_len;
    st = body_op(vi, NYTPROF_V6_OP_START_DEFLATE);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    /* Counts as a body record for chunk logical_event_count. */
    st = after_record(vi, NYTP_EVT_START_DEFLATE);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

static const nytp_sink_ops v6_ops = {
    .name = v6_name,
    .activate = v6_activate,
    .flush = v6_flush,
    .close = v6_close,
    .destroy = v6_destroy,
    .emit_attribute = v6_emit_attribute,
    .emit_option = v6_emit_option,
    .emit_comment = v6_emit_comment,
    .emit_time_line = v6_emit_time_line,
    .emit_time_block = v6_emit_time_block,
    .emit_discount = v6_emit_discount,
    .emit_new_fid = v6_emit_new_fid,
    .emit_src_line = v6_emit_src_line,
    .emit_sub_info = v6_emit_sub_info,
    .emit_sub_callers = v6_emit_sub_callers,
    .emit_pid_start = v6_emit_pid_start,
    .emit_pid_end = v6_emit_pid_end,
    .emit_sub_entry = v6_emit_sub_entry,
    .emit_sub_return = v6_emit_sub_return,
    .emit_start_deflate = v6_emit_start_deflate,
    .on_logical_committed = v6_on_logical_committed,
};

static nytp_sink *v6_create_common(const char *path, uint16_t minor,
                                   uint64_t required_features,
                                   uint64_t optional_features,
                                   uint32_t header_crc)
{
    nytp_sink *sink = (nytp_sink *)calloc(1, sizeof(*sink));
    v6_impl *vi = (v6_impl *)calloc(1, sizeof(*vi));
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
    vi->minor = minor;
    vi->required_features = required_features;
    vi->optional_features = optional_features;
    vi->header_crc = header_crc;
    if (write_file_prefix(vi) != NYTP_OK) {
        free(vi->path);
        free(vi->wire);
        free(vi);
        free(sink);
        return NULL;
    }
    vi->header_ok = 1;
    sink->ops = &v6_ops;
    sink->state = NYTP_SINK_OPEN;
    sink->impl = vi;
    sink->next_seq = 0;
    sink->last_seq = 0;
    sink->has_last_seq = 0;
    sink->fail_reason = NYTP_OK;
    return sink;
}

nytp_sink *nytp_v6_sink_create(const char *path)
{
    return v6_create_common(path, 0, 0, 0, 0);
}

nytp_sink *nytp_v6_sink_create_ex(const char *path, uint16_t minor,
                                  uint64_t required_features,
                                  uint64_t optional_features,
                                  uint32_t header_crc)
{
    return v6_create_common(path, minor, required_features, optional_features,
                            header_crc);
}

int nytp_v6_sink_is_v6(const nytp_sink *sink)
{
    return sink && sink->ops == &v6_ops;
}

const nytp_counting_stats *nytp_v6_sink_stats(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return NULL;
    }
    return &((const v6_impl *)sink->impl)->stats;
}

const char *nytp_v6_sink_path(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return NULL;
    }
    return ((const v6_impl *)sink->impl)->path;
}

const uint8_t *nytp_v6_sink_wire(const nytp_sink *sink, size_t *out_len)
{
    const v6_impl *vi;
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        if (out_len) {
            *out_len = 0;
        }
        return NULL;
    }
    vi = (const v6_impl *)sink->impl;
    if (out_len) {
        *out_len = vi->wire_len;
    }
    return vi->wire;
}

size_t nytp_v6_sink_wire_len(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return 0;
    }
    return ((const v6_impl *)sink->impl)->wire_len;
}

int nytp_v6_sink_file_written(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return 0;
    }
    return ((const v6_impl *)sink->impl)->file_written;
}

int nytp_v6_sink_is_sealed(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return 0;
    }
    return ((const v6_impl *)sink->impl)->sealed;
}

void nytp_v6_sink_version(const nytp_sink *sink, uint16_t *major,
                          uint16_t *minor)
{
    const v6_impl *vi;
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        if (major) {
            *major = 0;
        }
        if (minor) {
            *minor = 0;
        }
        return;
    }
    vi = (const v6_impl *)sink->impl;
    if (major) {
        *major = (uint16_t)NYTPROF_V6_SUPPORTED_MAJOR;
    }
    if (minor) {
        *minor = vi->minor;
    }
}

nytp_status nytp_v6_sink_test_force_body_len(nytp_sink *sink, size_t len)
{
    v6_impl *vi;
    nytp_status st;
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return NYTP_ERR_NULL;
    }
    vi = (v6_impl *)sink->impl;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    if (len > NYTPROF_V6_MAX_EVENT_BODY_BYTES) {
        return NYTP_ERR_OVERFLOW;
    }
    if (len > vi->body_cap) {
        /* Grow capacity without treating the extra as payload yet. */
        size_t need_extra = len - vi->body_len;
        st = buf_reserve(&vi->body, &vi->body_len, &vi->body_cap, need_extra);
        if (st != NYTP_OK) {
            return st;
        }
    }
    if (len > vi->body_len) {
        memset(vi->body + vi->body_len, 0, len - vi->body_len);
    }
    vi->body_len = len;
    return NYTP_OK;
}

uint32_t nytp_v6_sink_event_count(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return 0;
    }
    return ((const v6_impl *)sink->impl)->event_count;
}

const uint8_t *nytp_v6_sink_event_body(const nytp_sink *sink, size_t *out_len)
{
    const v6_impl *vi;
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        if (out_len) {
            *out_len = 0;
        }
        return NULL;
    }
    vi = (const v6_impl *)sink->impl;
    if (vi->sealed) {
        if (out_len) {
            *out_len = 0;
        }
        return NULL;
    }
    if (out_len) {
        *out_len = vi->body_len;
    }
    return vi->body;
}

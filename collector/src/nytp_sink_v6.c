/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-007 (PR-B06..B08) — Provisional v6 wire sink.
 *
 * Layout matches crates/nytprof-format-v6 encode_file_prefix + encode_chunk_frame
 * + event_body (absolute or ADR-0001 packing) + payload codecs + multi-chunk + CRC
 * + mid-stream codec regions + ADR-0002 FOOTER string dictionary.
 * IDs from nytprof_v6_ids.h.
 */
#include "nytp_sink_v6.h"
#include "nytprof_v6_ids.h"

#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <lz4.h>
#include <zlib.h>
#include <zstd.h>

#define NYTP_V6_INIT_CAP 4096u
#define NYTP_V6_ZSTD_LEVEL 3
#define NYTP_V6_ZLIB_LEVEL 6

/* ADR-0001 packing cursor + next logical packing sequence. */
typedef struct v6_packing {
    uint64_t fid;
    uint64_t line;
    uint64_t block_line;
    uint64_t caller_fid;
    uint64_t caller_line;
    uint64_t next_seq;
} v6_packing;

/* ADR-0002 FOOTER-local dictionary entry (owned bytes). */
typedef struct v6_dict_entry {
    uint64_t id;
    uint8_t flags;
    uint8_t *data;
    size_t len;
} v6_dict_entry;

typedef struct v6_impl {
    nytp_counting_stats stats;
    char *path;
    /* Sealed file bytes: prefix [+ EVENT region chunk(s)] [+ FOOTER]. */
    uint8_t *wire;
    size_t wire_len;
    size_t wire_cap;
    /* Open event-body for the current codec region. */
    uint8_t *body;
    size_t body_len;
    size_t body_cap;
    /* Body offset of each committed record start (for multi-chunk partition). */
    size_t *rec_off;
    uint32_t rec_off_len;
    uint32_t rec_off_cap;
    uint16_t minor;
    uint64_t required_features;
    uint64_t optional_features;
    uint32_t header_crc; /* sealed value after write_file_prefix */
    uint32_t event_count; /* records in open body region */
    uint32_t total_event_records; /* cumulative sealed + open records */
    uint32_t event_chunk_count; /* EVENT frames sealed so far (all regions) */
    uint64_t next_chunk_seq; /* next EVENT chunk sequence number */
    uint8_t event_codec; /* NYTPROF_V6_CODEC_* for current region */
    size_t max_records_per_chunk; /* 0 = unlimited */
    int enable_packing;
    int enable_string_dict;
    v6_packing pack;
    v6_dict_entry *dict;
    uint32_t dict_len;
    uint32_t dict_cap;
    uint64_t dict_next_id; /* next non-zero id (starts at 1) */
    size_t dict_total_bytes;
    int has_footer_dict; /* set after final FOOTER seal */
    int sealed; /* final profile seal (close) */
    int file_written;
    int header_ok;
    /* In-flight emit checkpoints for fail-closed rollback (dict + packing). */
    int emit_snap_active;
    uint32_t emit_dict_len0;
    uint64_t emit_dict_next_id0;
    size_t emit_dict_total0;
    uint64_t emit_pack_seq0;
    /* Test hook: fail seal after successfully framing this many chunks (0=off). */
    uint32_t test_fail_seal_after_chunks;
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

static nytp_status rec_off_push(v6_impl *vi, size_t off)
{
    if (vi->rec_off_len == vi->rec_off_cap) {
        uint32_t ncap = vi->rec_off_cap ? vi->rec_off_cap * 2u : 32u;
        size_t *nbuf;
        if (ncap < vi->rec_off_len + 1u) {
            ncap = vi->rec_off_len + 1u;
        }
        nbuf = (size_t *)realloc(vi->rec_off, (size_t)ncap * sizeof(size_t));
        if (!nbuf) {
            return NYTP_ERR_IO;
        }
        vi->rec_off = nbuf;
        vi->rec_off_cap = ncap;
    }
    vi->rec_off[vi->rec_off_len++] = off;
    return NYTP_OK;
}

/* Snapshot packing + dict state at the start of an emit (once per record). */
static void emit_snap_begin(v6_impl *vi)
{
    if (vi->emit_snap_active) {
        return;
    }
    vi->emit_snap_active = 1;
    vi->emit_dict_len0 = vi->dict_len;
    vi->emit_dict_next_id0 = vi->dict_next_id;
    vi->emit_dict_total0 = vi->dict_total_bytes;
    vi->emit_pack_seq0 = vi->pack.next_seq;
}

static void emit_snap_clear(v6_impl *vi)
{
    vi->emit_snap_active = 0;
}

/* Drop dictionary entries interned during a failed emit. */
static void dict_rollback_emit(v6_impl *vi)
{
    if (!vi->emit_snap_active || !vi->enable_string_dict) {
        return;
    }
    while (vi->dict_len > vi->emit_dict_len0) {
        uint32_t i = vi->dict_len - 1u;
        free(vi->dict[i].data);
        vi->dict[i].data = NULL;
        vi->dict[i].len = 0;
        vi->dict_len = i;
    }
    vi->dict_next_id = vi->emit_dict_next_id0;
    vi->dict_total_bytes = vi->emit_dict_total0;
}

/*
 * On any emit failure after body bytes may have been written: restore mark.
 * Call only for non-OK statuses from mid-record paths.
 * Also drops any record offsets at/after mark (defensive; after_record is last),
 * rolls packing seq and FOOTER dict entries created mid-record.
 */
static nytp_status emit_fail(v6_impl *vi, size_t mark, nytp_status st)
{
    body_rollback(vi, mark);
    while (vi->rec_off_len > 0 &&
           vi->rec_off[vi->rec_off_len - 1u] >= mark) {
        vi->rec_off_len--;
    }
    if (vi->event_count > vi->rec_off_len) {
        uint32_t dropped = vi->event_count - vi->rec_off_len;
        vi->event_count = vi->rec_off_len;
        if (vi->total_event_records >= dropped) {
            vi->total_event_records -= dropped;
        } else {
            vi->total_event_records = 0;
        }
    }
    if (vi->emit_snap_active) {
        if (vi->enable_packing) {
            vi->pack.next_seq = vi->emit_pack_seq0;
        }
        dict_rollback_emit(vi);
        emit_snap_clear(vi);
    }
    return st;
}

/* CRC-32/IEEE (ISO-HDLC) via zlib — matches format-v6 crc32_ieee. */
static uint32_t v6_crc32_ieee(const uint8_t *data, size_t len)
{
    uLong c = crc32(0L, Z_NULL, 0);
    if (len > 0 && data) {
        /* zlib crc32 takes uInt; chunk large buffers. */
        size_t off = 0;
        while (off < len) {
            uInt n = (uInt)((len - off) > 0xffffffffu ? 0xffffffffu
                                                     : (len - off));
            if (n == 0) {
                break;
            }
            c = crc32(c, data + off, n);
            off += (size_t)n;
        }
    }
    return (uint32_t)c;
}

static int codec_supported(uint8_t c)
{
    return c == NYTPROF_V6_CODEC_NONE || c == NYTPROF_V6_CODEC_ZLIB ||
           c == NYTPROF_V6_CODEC_ZSTD || c == NYTPROF_V6_CODEC_LZ4;
}

/*
 * Compress plain partition into *out / *out_len (malloc'd; caller frees).
 * NONE: copy. Fail-closed on oversize / codec errors.
 */
static nytp_status compress_payload(uint8_t codec, const uint8_t *plain,
                                    size_t plain_len, uint8_t **out,
                                    size_t *out_len)
{
    *out = NULL;
    *out_len = 0;
    if (plain_len > NYTPROF_V6_MAX_CHUNK_PAYLOAD) {
        return NYTP_ERR_OVERFLOW;
    }
    if (codec == NYTPROF_V6_CODEC_NONE) {
        if (plain_len == 0) {
            *out = NULL;
            *out_len = 0;
            return NYTP_OK;
        }
        *out = (uint8_t *)malloc(plain_len);
        if (!*out) {
            return NYTP_ERR_IO;
        }
        memcpy(*out, plain, plain_len);
        *out_len = plain_len;
        return NYTP_OK;
    }
    if (codec == NYTPROF_V6_CODEC_ZLIB) {
        uLongf bound = compressBound((uLong)plain_len);
        uint8_t *buf;
        uLongf clen;
        int zst;
        if (bound == 0) {
            bound = 64; /* empty / tiny */
        }
        buf = (uint8_t *)malloc((size_t)bound);
        if (!buf) {
            return NYTP_ERR_IO;
        }
        clen = bound;
        zst = compress2(buf, &clen, plain, (uLong)plain_len, NYTP_V6_ZLIB_LEVEL);
        if (zst != Z_OK) {
            free(buf);
            return NYTP_ERR_IO;
        }
        if (clen > NYTPROF_V6_MAX_CHUNK_PAYLOAD) {
            free(buf);
            return NYTP_ERR_OVERFLOW;
        }
        *out = buf;
        *out_len = (size_t)clen;
        return NYTP_OK;
    }
    if (codec == NYTPROF_V6_CODEC_ZSTD) {
        size_t bound = ZSTD_compressBound(plain_len);
        uint8_t *buf;
        size_t clen;
        if (ZSTD_isError(bound)) {
            return NYTP_ERR_IO;
        }
        if (bound == 0) {
            bound = 64;
        }
        buf = (uint8_t *)malloc(bound);
        if (!buf) {
            return NYTP_ERR_IO;
        }
        clen = ZSTD_compress(buf, bound, plain, plain_len, NYTP_V6_ZSTD_LEVEL);
        if (ZSTD_isError(clen)) {
            free(buf);
            return NYTP_ERR_IO;
        }
        if (clen > NYTPROF_V6_MAX_CHUNK_PAYLOAD) {
            free(buf);
            return NYTP_ERR_OVERFLOW;
        }
        *out = buf;
        *out_len = clen;
        return NYTP_OK;
    }
    if (codec == NYTPROF_V6_CODEC_LZ4) {
        int bound = LZ4_compressBound((int)plain_len);
        uint8_t *buf;
        int clen;
        if (plain_len > (size_t)LZ4_MAX_INPUT_SIZE) {
            return NYTP_ERR_OVERFLOW;
        }
        if (bound <= 0) {
            bound = 64;
        }
        buf = (uint8_t *)malloc((size_t)bound);
        if (!buf) {
            return NYTP_ERR_IO;
        }
        if (plain_len == 0) {
            /* Empty raw block: zero-length payload is valid (uncompressed_len=0). */
            free(buf);
            *out = NULL;
            *out_len = 0;
            return NYTP_OK;
        }
        clen = LZ4_compress_default((const char *)plain, (char *)buf,
                                    (int)plain_len, bound);
        if (clen <= 0) {
            free(buf);
            return NYTP_ERR_IO;
        }
        if ((size_t)clen > NYTPROF_V6_MAX_CHUNK_PAYLOAD) {
            free(buf);
            return NYTP_ERR_OVERFLOW;
        }
        *out = buf;
        *out_len = (size_t)clen;
        return NYTP_OK;
    }
    return NYTP_ERR_UNSUPPORTED;
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

/* ZigZag encode (matches format-v6 zigzag_encode_i64). */
static uint64_t zigzag_encode_i64(int64_t n)
{
    return ((uint64_t)n << 1) ^ (uint64_t)((int64_t)n >> 63);
}

static nytp_status body_zigzag(v6_impl *vi, int64_t v)
{
    return body_uleb(vi, zigzag_encode_i64(v));
}

/* Signed delta from→to as i64; fail-closed on range overflow. */
static nytp_status i64_delta_u64(uint64_t from, uint64_t to, int64_t *out)
{
    /* Portable range check without __int128. */
    if (to >= from) {
        uint64_t d = to - from;
        if (d > (uint64_t)INT64_MAX) {
            return NYTP_ERR_OVERFLOW;
        }
        *out = (int64_t)d;
    } else {
        uint64_t d = from - to;
        /* magnitude must fit in |INT64_MIN| = 2^63 */
        if (d > (uint64_t)INT64_MAX + 1ull) {
            return NYTP_ERR_OVERFLOW;
        }
        if (d == (uint64_t)INT64_MAX + 1ull) {
            *out = INT64_MIN;
        } else {
            *out = -(int64_t)d;
        }
    }
    return NYTP_OK;
}

/*
 * Begin a wire record: opcode ULEB + flags [+ packing seq].
 * use_site_delta: when packing, also set FLAG_SITE_DELTA (TIME_LINE/BLOCK/SUB_ENTRY).
 * When packing, always writes FLAG_HAS_SEQ + next_seq (does not advance yet).
 */
static nytp_status body_begin_op(v6_impl *vi, uint64_t opcode, int use_site_delta)
{
    uint8_t flags = 0;
    nytp_status st;
    emit_snap_begin(vi);
    st = body_uleb(vi, opcode);
    if (st != NYTP_OK) {
        return st;
    }
    if (vi->enable_packing) {
        flags = (uint8_t)NYTPROF_V6_FLAG_HAS_SEQ;
        if (use_site_delta) {
            flags = (uint8_t)(flags | NYTPROF_V6_FLAG_SITE_DELTA);
        }
    }
    st = body_u8(vi, flags);
    if (st != NYTP_OK) {
        return st;
    }
    if (vi->enable_packing) {
        st = body_uleb(vi, vi->pack.next_seq);
        if (st != NYTP_OK) {
            return st;
        }
    }
    return NYTP_OK;
}

/* Absolute-only convenience (flags=0). Prefer body_begin_op for packing paths. */
static nytp_status body_op(v6_impl *vi, uint64_t opcode)
{
    return body_begin_op(vi, opcode, 0);
}

static nytp_status packing_advance(v6_impl *vi, uint64_t n)
{
    if (!vi->enable_packing || n == 0) {
        return NYTP_OK;
    }
    if (vi->pack.next_seq > UINT64_MAX - n) {
        return NYTP_ERR_OVERFLOW;
    }
    vi->pack.next_seq += n;
    return NYTP_OK;
}

/* Intern string for FOOTER dict; returns string_id (0 if dict off / empty). */
static nytp_status dict_intern(v6_impl *vi, nytp_string_view sv, uint64_t *out_id,
                               uint8_t *out_flags)
{
    uint32_t i;
    uint8_t flags;
    uint8_t *copy;
    v6_dict_entry *nbuf;
    *out_id = 0;
    *out_flags = utf8_flag(sv);
    if (!vi->enable_string_dict) {
        return NYTP_OK;
    }
    if (sv.len == 0) {
        /* Empty still uses inline id 0 (no dict key). */
        return NYTP_OK;
    }
    flags = *out_flags;
    for (i = 0; i < vi->dict_len; i++) {
        if (vi->dict[i].len == sv.len && vi->dict[i].flags == flags &&
            (sv.len == 0 || memcmp(vi->dict[i].data, sv.ptr, sv.len) == 0)) {
            *out_id = vi->dict[i].id;
            return NYTP_OK;
        }
    }
    if (vi->dict_len >= NYTPROF_V6_MAX_DICT_ENTRIES) {
        return NYTP_ERR_OVERFLOW;
    }
    if (vi->dict_total_bytes > NYTPROF_V6_MAX_DICT_TOTAL_BYTES - sv.len) {
        return NYTP_ERR_OVERFLOW;
    }
    if (vi->dict_next_id == 0 || vi->dict_next_id > UINT64_MAX - 1) {
        return NYTP_ERR_OVERFLOW;
    }
    if (vi->dict_len == vi->dict_cap) {
        uint32_t ncap = vi->dict_cap ? vi->dict_cap * 2u : 8u;
        if (ncap < vi->dict_len + 1u) {
            ncap = vi->dict_len + 1u;
        }
        nbuf = (v6_dict_entry *)realloc(vi->dict, (size_t)ncap * sizeof(*nbuf));
        if (!nbuf) {
            return NYTP_ERR_IO;
        }
        vi->dict = nbuf;
        vi->dict_cap = ncap;
    }
    copy = (uint8_t *)malloc(sv.len ? sv.len : 1);
    if (!copy) {
        return NYTP_ERR_IO;
    }
    if (sv.len) {
        memcpy(copy, sv.ptr, sv.len);
    }
    vi->dict[vi->dict_len].id = vi->dict_next_id++;
    vi->dict[vi->dict_len].flags = flags;
    vi->dict[vi->dict_len].data = copy;
    vi->dict[vi->dict_len].len = sv.len;
    vi->dict_len++;
    vi->dict_total_bytes += sv.len;
    *out_id = vi->dict[vi->dict_len - 1u].id;
    return NYTP_OK;
}

/*
 * Emit string-blob: when dict enabled and interned id != 0, write empty inline
 * payload (dict carries bytes). Otherwise id 0 + full inline.
 */
static nytp_status body_string_blob_maybe_dict(v6_impl *vi, nytp_string_view sv)
{
    uint64_t id = 0;
    uint8_t flags = 0;
    nytp_status st;
    emit_snap_begin(vi);
    st = dict_intern(vi, sv, &id, &flags);
    if (st != NYTP_OK) {
        return st;
    }
    if (id != 0) {
        nytp_string_view empty;
        empty.ptr = NULL;
        empty.len = 0;
        empty.is_utf8 = 0;
        return body_string_blob(vi, id, flags, empty);
    }
    return body_string_blob(vi, 0, flags ? flags : utf8_flag(sv), sv);
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
    return "v6";
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
    /* Seal header CRC over first 32 bytes (excludes CRC field). */
    {
        uint32_t crc = v6_crc32_ieee(hdr, 32);
        hdr[32] = (uint8_t)(crc & 0xffu);
        hdr[33] = (uint8_t)((crc >> 8) & 0xffu);
        hdr[34] = (uint8_t)((crc >> 16) & 0xffu);
        hdr[35] = (uint8_t)((crc >> 24) & 0xffu);
        vi->header_crc = crc;
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

/*
 * Seal the open body into EVENT chunk(s) under the current codec.
 * Appends to wire; clears open body for the next region. Does **not** set
 * final sealed flag (mid-stream region seal may continue packing).
 * On failure: rewinds wire to entry mark; leaves body intact for retry.
 */
static nytp_status seal_open_event_region(v6_impl *vi, const nytp_sink *sink)
{
    nytp_status st;
    uint32_t nrec;
    size_t start;
    uint64_t seq0;
    size_t wire_mark;
    uint32_t chunks_this = 0;

    if (vi->sealed) {
        return NYTP_OK;
    }
    if (sink && sink->state == NYTP_SINK_FAILED) {
        vi->body_len = 0;
        vi->event_count = 0;
        vi->rec_off_len = 0;
        return NYTP_OK;
    }
    if (vi->body_len > NYTPROF_V6_MAX_EVENT_BODY_BYTES) {
        return NYTP_ERR_OVERFLOW;
    }
    nrec = vi->event_count;
    if (nrec != vi->rec_off_len) {
        return NYTP_ERR_IO;
    }
    if (vi->body_len == 0 || nrec == 0) {
        /* Nothing to frame in this region. */
        vi->body_len = 0;
        vi->event_count = 0;
        vi->rec_off_len = 0;
        return NYTP_OK;
    }

    wire_mark = vi->wire_len;
    seq0 = vi->next_chunk_seq;
    start = 0;
    while (start < (size_t)nrec) {
        size_t count;
        size_t off0;
        size_t off1;
        size_t plain_len;
        const uint8_t *plain;
        uint8_t *payload = NULL;
        size_t payload_len = 0;
        uint32_t checksum;
        uint64_t seq;

        if (vi->max_records_per_chunk == 0) {
            count = (size_t)nrec - start;
        } else {
            size_t rem = (size_t)nrec - start;
            count = rem < vi->max_records_per_chunk ? rem
                                                   : vi->max_records_per_chunk;
        }
        if (count == 0 || count > (size_t)UINT32_MAX) {
            vi->wire_len = wire_mark;
            return NYTP_ERR_OVERFLOW;
        }
        off0 = vi->rec_off[start];
        off1 = (start + count < (size_t)nrec) ? vi->rec_off[start + count]
                                              : vi->body_len;
        if (off1 < off0 || off1 > vi->body_len) {
            vi->wire_len = wire_mark;
            return NYTP_ERR_IO;
        }
        plain = vi->body + off0;
        plain_len = off1 - off0;
        if (plain_len > NYTPROF_V6_MAX_CHUNK_PAYLOAD) {
            vi->wire_len = wire_mark;
            return NYTP_ERR_OVERFLOW;
        }
        st = compress_payload(vi->event_codec, plain, plain_len, &payload,
                              &payload_len);
        if (st != NYTP_OK) {
            free(payload);
            vi->wire_len = wire_mark;
            return st;
        }
        checksum = v6_crc32_ieee(payload, payload_len);
        seq = seq0 + (uint64_t)chunks_this;
        st = encode_chunk_frame(vi, (uint8_t)NYTPROF_V6_KIND_EVENT,
                                vi->event_codec, 0 /* flags */, seq,
                                0 /* first_logical */, (uint32_t)count,
                                (uint32_t)plain_len, payload, payload_len,
                                checksum);
        free(payload);
        if (st != NYTP_OK) {
            vi->wire_len = wire_mark;
            return st;
        }
        chunks_this++;
        start += count;
        if (vi->test_fail_seal_after_chunks > 0 &&
            chunks_this == vi->test_fail_seal_after_chunks) {
            vi->wire_len = wire_mark;
            return NYTP_ERR_IO;
        }
    }
    vi->next_chunk_seq = seq0 + (uint64_t)chunks_this;
    vi->event_chunk_count += chunks_this;
    /* Clear open body for next region; packing state continues. */
    vi->body_len = 0;
    vi->event_count = 0;
    vi->rec_off_len = 0;
    return NYTP_OK;
}

static nytp_status encode_footer_string_dict(v6_impl *vi)
{
    uint8_t *payload = NULL;
    size_t payload_len = 0;
    size_t cap = 64;
    size_t len = 0;
    uint8_t tmp[10];
    size_t n;
    uint32_t i;
    uint32_t checksum;
    nytp_status st;

    if (!vi->enable_string_dict) {
        return NYTP_OK;
    }
    /* Always emit FOOTER table when dict enabled (entry_count may be 0). */
    payload = (uint8_t *)malloc(cap);
    if (!payload) {
        return NYTP_ERR_IO;
    }
    n = uleb_encode((uint64_t)vi->dict_len, tmp);
    if (n > cap) {
        free(payload);
        return NYTP_ERR_OVERFLOW;
    }
    memcpy(payload, tmp, n);
    len = n;
    for (i = 0; i < vi->dict_len; i++) {
        size_t need = 10 + 1 + 10 + vi->dict[i].len;
        if (len > SIZE_MAX - need) {
            free(payload);
            return NYTP_ERR_OVERFLOW;
        }
        if (len + need > cap) {
            size_t ncap = cap;
            uint8_t *np;
            while (ncap < len + need) {
                if (ncap > SIZE_MAX / 2u) {
                    ncap = len + need;
                    break;
                }
                ncap *= 2u;
            }
            np = (uint8_t *)realloc(payload, ncap);
            if (!np) {
                free(payload);
                return NYTP_ERR_IO;
            }
            payload = np;
            cap = ncap;
        }
        n = uleb_encode(vi->dict[i].id, tmp);
        memcpy(payload + len, tmp, n);
        len += n;
        payload[len++] = vi->dict[i].flags;
        n = uleb_encode((uint64_t)vi->dict[i].len, tmp);
        memcpy(payload + len, tmp, n);
        len += n;
        if (vi->dict[i].len) {
            memcpy(payload + len, vi->dict[i].data, vi->dict[i].len);
            len += vi->dict[i].len;
        }
    }
    if (len > NYTPROF_V6_MAX_CHUNK_PAYLOAD) {
        free(payload);
        return NYTP_ERR_OVERFLOW;
    }
    payload_len = len;
    checksum = v6_crc32_ieee(payload, payload_len);
    st = encode_chunk_frame(vi, (uint8_t)NYTPROF_V6_KIND_FOOTER,
                            (uint8_t)NYTPROF_V6_CODEC_NONE, 0,
                            vi->next_chunk_seq /* sequence after EVENT */,
                            0, 0, (uint32_t)payload_len, payload, payload_len,
                            checksum);
    free(payload);
    if (st != NYTP_OK) {
        return st;
    }
    vi->has_footer_dict = 1;
    return NYTP_OK;
}

/*
 * Final seal: open EVENT region + optional FOOTER dict; mark sealed.
 * Sticky FAILED: discard open body, no FOOTER product claim.
 */
static nytp_status seal_event_chunk(v6_impl *vi, const nytp_sink *sink)
{
    nytp_status st;
    size_t wire_mark;
    if (vi->sealed) {
        return NYTP_OK;
    }
    if (sink && sink->state == NYTP_SINK_FAILED) {
        vi->body_len = 0;
        vi->event_count = 0;
        vi->rec_off_len = 0;
        vi->total_event_records = 0;
        vi->sealed = 1;
        return NYTP_OK;
    }
    st = seal_open_event_region(vi, sink);
    if (st != NYTP_OK) {
        return st;
    }
    wire_mark = vi->wire_len; /* after EVENT region(s) */
    if (vi->enable_string_dict) {
        st = encode_footer_string_dict(vi);
        if (st != NYTP_OK) {
            if (vi->wire_len > wire_mark) {
                vi->wire_len = wire_mark; /* drop partial FOOTER */
            }
            vi->has_footer_dict = 0;
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
    uint32_t i;
    if (!sink) {
        return;
    }
    vi = (v6_impl *)sink->impl;
    if (vi) {
        free(vi->path);
        free(vi->wire);
        free(vi->body);
        free(vi->rec_off);
        if (vi->dict) {
            for (i = 0; i < vi->dict_len; i++) {
                free(vi->dict[i].data);
            }
            free(vi->dict);
        }
        free(vi);
    }
    free(sink);
}

/* ---- emit ops (absolute bodies) ---- */

static nytp_status after_record(v6_impl *vi, nytp_event_kind kind,
                                size_t rec_start)
{
    nytp_status st;
    if (vi->event_count == UINT32_MAX) {
        return NYTP_ERR_OVERFLOW;
    }
    /* Fail-closed packing-seq overflow before committing the record. */
    if (vi->enable_packing && vi->pack.next_seq > UINT64_MAX - 1ull) {
        return NYTP_ERR_OVERFLOW;
    }
    st = rec_off_push(vi, rec_start);
    if (st != NYTP_OK) {
        return st;
    }
    vi->event_count++;
    vi->total_event_records++;
    note_kind(vi, kind);
    /* Packing seq advanced only after successful commit (matches body_begin_op peek). */
    st = packing_advance(vi, 1);
    if (st != NYTP_OK) {
        return st;
    }
    emit_snap_clear(vi);
    return NYTP_OK;
}

/* Like after_record but packing seq advances by n (TIME_*_RUN base..base+N-1). */
static nytp_status after_record_n(v6_impl *vi, nytp_event_kind kind,
                                  size_t rec_start, uint64_t pack_n)
{
    nytp_status st;
    if (vi->event_count == UINT32_MAX) {
        return NYTP_ERR_OVERFLOW;
    }
    if (vi->enable_packing && pack_n > 0 &&
        vi->pack.next_seq > UINT64_MAX - pack_n) {
        return NYTP_ERR_OVERFLOW;
    }
    st = rec_off_push(vi, rec_start);
    if (st != NYTP_OK) {
        return st;
    }
    vi->event_count++;
    vi->total_event_records++;
    note_kind(vi, kind);
    st = packing_advance(vi, pack_n);
    if (st != NYTP_OK) {
        return st;
    }
    emit_snap_clear(vi);
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
    st = body_string_blob_maybe_dict(vi, key);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob_maybe_dict(vi, value);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = after_record(vi, NYTP_EVT_ATTRIBUTE, mark);
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
    st = body_string_blob_maybe_dict(vi, key);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob_maybe_dict(vi, value);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = after_record(vi, NYTP_EVT_OPTION, mark);
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
    st = body_string_blob_maybe_dict(vi, text);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = after_record(vi, NYTP_EVT_COMMENT, mark);
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
    st = body_begin_op(vi, NYTPROF_V6_OP_TIME_LINE, vi->enable_packing ? 1 : 0);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    if (vi->enable_packing) {
        int64_t df, dl;
        st = i64_delta_u64(vi->pack.fid, (uint64_t)fid, &df);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = i64_delta_u64(vi->pack.line, (uint64_t)line, &dl);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = body_zigzag(vi, df);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = body_zigzag(vi, dl);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = body_uleb(vi, ut);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
    } else {
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
    }
    vi->stats.last_ticks = ticks;
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    vi->stats.last_block_line = 0;
    vi->stats.last_sub_line = 0;
    st = after_record(vi, NYTP_EVT_TIME_LINE, mark);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    if (vi->enable_packing) {
        vi->pack.fid = (uint64_t)fid;
        vi->pack.line = (uint64_t)line;
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
    st = body_begin_op(vi, NYTPROF_V6_OP_TIME_BLOCK, vi->enable_packing ? 1 : 0);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    if (vi->enable_packing) {
        int64_t df, dl, db;
        st = i64_delta_u64(vi->pack.fid, (uint64_t)fid, &df);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = i64_delta_u64(vi->pack.line, (uint64_t)line, &dl);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = i64_delta_u64(vi->pack.block_line, (uint64_t)block_line, &db);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = body_zigzag(vi, df);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = body_zigzag(vi, dl);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = body_zigzag(vi, db);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = body_uleb(vi, ut);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
    } else {
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
    }
    vi->stats.last_ticks = ticks;
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    vi->stats.last_block_line = block_line;
    vi->stats.last_sub_line = sub_line;
    st = after_record(vi, NYTP_EVT_TIME_BLOCK, mark);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    if (vi->enable_packing) {
        vi->pack.fid = (uint64_t)fid;
        vi->pack.line = (uint64_t)line;
        vi->pack.block_line = (uint64_t)block_line;
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
    st = after_record(vi, NYTP_EVT_DISCOUNT, mark);
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
    st = body_string_blob_maybe_dict(vi, name);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_fid = fid;
    st = after_record(vi, NYTP_EVT_NEW_FID, mark);
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
    st = body_string_blob_maybe_dict(vi, text);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    vi->stats.last_src_fid = fid;
    vi->stats.last_src_line = line;
    copy_src_text(vi, text);
    st = after_record(vi, NYTP_EVT_SRC_LINE, mark);
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
    st = body_string_blob_maybe_dict(vi, name);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_fid = fid;
    vi->stats.last_line = first_line;
    vi->stats.last_block_line = last_line;
    copy_subname(vi, name);
    st = after_record(vi, NYTP_EVT_SUB_INFO, mark);
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
    st = body_string_blob_maybe_dict(vi, called);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    st = body_string_blob_maybe_dict(vi, caller);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    copy_subname(vi, called);
    st = after_record(vi, NYTP_EVT_SUB_CALLERS, mark);
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
    st = after_record(vi, NYTP_EVT_PID_START, mark);
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
    st = after_record(vi, NYTP_EVT_PID_END, mark);
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
    st = body_begin_op(vi, NYTPROF_V6_OP_SUB_ENTRY, vi->enable_packing ? 1 : 0);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    if (vi->enable_packing) {
        int64_t df, dl;
        st = i64_delta_u64(vi->pack.caller_fid, (uint64_t)caller_fid, &df);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = i64_delta_u64(vi->pack.caller_line, (uint64_t)caller_line, &dl);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = body_zigzag(vi, df);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = body_zigzag(vi, dl);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
    } else {
        st = body_uleb(vi, (uint64_t)caller_fid);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
        st = body_uleb(vi, (uint64_t)caller_line);
        if (st != NYTP_OK) {
            return emit_fail(vi, mark, st);
        }
    }
    vi->stats.last_fid = caller_fid;
    vi->stats.last_line = caller_line;
    st = after_record(vi, NYTP_EVT_SUB_ENTRY, mark);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    if (vi->enable_packing) {
        vi->pack.caller_fid = (uint64_t)caller_fid;
        vi->pack.caller_line = (uint64_t)caller_line;
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
    st = body_string_blob_maybe_dict(vi, subname);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    vi->stats.last_depth = depth;
    copy_subname(vi, subname);
    st = after_record(vi, NYTP_EVT_SUB_RETURN, mark);
    if (st != NYTP_OK) {
        return emit_fail(vi, mark, st);
    }
    return NYTP_OK;
}

/*
 * START_DEFLATE: empty marker opcode only (typed body empty).
 * Does **not** by itself switch payload codecs — use
 * nytp_v6_sink_begin_codec_region for mid-stream codec switch (PR-B08).
 * Control event for COL-003 (no logical sink seq). Packing may still write
 * FLAG_HAS_SEQ on the wire (ADR-0001 packing stream).
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
    st = after_record(vi, NYTP_EVT_START_DEFLATE, mark);
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
                                   uint8_t event_codec,
                                   size_t max_records_per_chunk,
                                   int enable_packing,
                                   int enable_string_dict)
{
    nytp_sink *sink;
    v6_impl *vi;
    if (!codec_supported(event_codec)) {
        return NULL;
    }
    sink = (nytp_sink *)calloc(1, sizeof(*sink));
    vi = (v6_impl *)calloc(1, sizeof(*vi));
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
    vi->event_codec = event_codec;
    vi->max_records_per_chunk = max_records_per_chunk;
    vi->enable_packing = enable_packing ? 1 : 0;
    vi->enable_string_dict = enable_string_dict ? 1 : 0;
    vi->dict_next_id = 1;
    vi->next_chunk_seq = 0;
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
    return v6_create_common(path, 0, 0, 0, (uint8_t)NYTPROF_V6_CODEC_NONE, 0, 0, 0);
}

nytp_sink *nytp_v6_sink_create_ex(const char *path, uint16_t minor,
                                  uint64_t required_features,
                                  uint64_t optional_features,
                                  uint32_t header_crc)
{
    (void)header_crc; /* PR-B07: always sealed */
    return v6_create_common(path, minor, required_features, optional_features,
                            (uint8_t)NYTPROF_V6_CODEC_NONE, 0, 0, 0);
}

nytp_sink *nytp_v6_sink_create_codec(const char *path, uint8_t event_codec,
                                     size_t max_records_per_chunk)
{
    return v6_create_common(path, 0, 0, 0, event_codec, max_records_per_chunk, 0, 0);
}

nytp_sink *nytp_v6_sink_create_codec_ex(const char *path, uint16_t minor,
                                        uint64_t required_features,
                                        uint64_t optional_features,
                                        uint8_t event_codec,
                                        size_t max_records_per_chunk)
{
    return v6_create_common(path, minor, required_features, optional_features,
                            event_codec, max_records_per_chunk, 0, 0);
}

nytp_sink *nytp_v6_sink_create_opts(const char *path,
                                    const nytp_v6_sink_options *opt)
{
    if (!opt) {
        return v6_create_common(path, 0, 0, 0, (uint8_t)NYTPROF_V6_CODEC_NONE, 0,
                                0, 0);
    }
    return v6_create_common(path, opt->minor, opt->required_features,
                            opt->optional_features, opt->event_codec,
                            opt->max_records_per_chunk, opt->enable_packing,
                            opt->enable_string_dict);
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

/*
 * Test hook: after successfully framing N EVENT chunks during seal, abort and
 * rewind wire to pre-seal prefix (simulates mid-multi-chunk OOM/IO). 0 disables.
 * No-op if not a v6 sink.
 */
void nytp_v6_sink_test_fail_seal_after_chunks(nytp_sink *sink, uint32_t n)
{
    v6_impl *vi;
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return;
    }
    vi = (v6_impl *)sink->impl;
    if (vi->sealed) {
        return;
    }
    vi->test_fail_seal_after_chunks = n;
}

nytp_status nytp_v6_sink_test_try_seal(nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return NYTP_ERR_NULL;
    }
    return seal_event_chunk((v6_impl *)sink->impl, sink);
}


uint8_t nytp_v6_sink_event_codec(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return 0;
    }
    return ((const v6_impl *)sink->impl)->event_codec;
}

size_t nytp_v6_sink_max_records_per_chunk(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return 0;
    }
    return ((const v6_impl *)sink->impl)->max_records_per_chunk;
}

uint32_t nytp_v6_sink_event_chunk_count(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return 0;
    }
    return ((const v6_impl *)sink->impl)->event_chunk_count;
}

uint32_t nytp_v6_sink_event_count(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return 0;
    }
    /* Cumulative logical records (open + sealed regions). */
    return ((const v6_impl *)sink->impl)->total_event_records;
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

int nytp_v6_sink_packing_enabled(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return 0;
    }
    return ((const v6_impl *)sink->impl)->enable_packing;
}

int nytp_v6_sink_string_dict_enabled(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return 0;
    }
    return ((const v6_impl *)sink->impl)->enable_string_dict;
}

int nytp_v6_sink_has_footer_dict(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return 0;
    }
    return ((const v6_impl *)sink->impl)->has_footer_dict;
}

uint32_t nytp_v6_sink_dict_entry_count(const nytp_sink *sink)
{
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return 0;
    }
    return ((const v6_impl *)sink->impl)->dict_len;
}

/*
 * Sticky-fail helper for public v6 packing APIs (mirrors nytp emit_commit).
 */
static nytp_status v6_public_fail(nytp_sink *sink, nytp_status st)
{
    if (st == NYTP_ERR_IO || st == NYTP_ERR_FAILED || st == NYTP_ERR_OVERFLOW) {
        sink->state = NYTP_SINK_FAILED;
        sink->fail_reason = st;
    }
    return st;
}

/* Advance COL-003 logical sink seq by n after a successful multi-logical emit. */
static void v6_commit_logical_n(nytp_sink *sink, nytp_event_kind kind, uint64_t n)
{
    uint64_t i;
    for (i = 0; i < n; i++) {
        nytp_seq seq = sink->next_seq;
        sink->last_seq = seq;
        sink->next_seq = seq + 1;
        sink->has_last_seq = 1;
        if (sink->ops && sink->ops->on_logical_committed) {
            sink->ops->on_logical_committed(sink, seq, kind);
        }
    }
}

nytp_status nytp_v6_sink_begin_codec_region(nytp_sink *sink, uint8_t next_codec)
{
    v6_impl *vi;
    nytp_status st;
    size_t mark;
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return NYTP_ERR_NULL;
    }
    vi = (v6_impl *)sink->impl;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    if (sink->state == NYTP_SINK_FAILED) {
        return NYTP_ERR_STATE;
    }
    /* Lifecycle: START_DEFLATE only legal in OPEN/ACTIVE (same as nytp_emit_*). */
    if (!nytp_sink_can_emit(sink, NYTP_EVT_START_DEFLATE)) {
        return NYTP_ERR_STATE;
    }
    if (!codec_supported(next_codec)) {
        return NYTP_ERR_UNSUPPORTED;
    }
    if (next_codec == vi->event_codec) {
        return NYTP_ERR_UNSUPPORTED; /* must differ (mid-stream preflight) */
    }
    /* Fail-closed: region must contain at least one prior record. */
    if (vi->body_len == 0 || vi->event_count == 0) {
        return NYTP_ERR_STATE;
    }
    /* 1. Emit empty START_DEFLATE into current region. */
    mark = vi->body_len;
    st = body_begin_op(vi, NYTPROF_V6_OP_START_DEFLATE, 0);
    if (st != NYTP_OK) {
        return v6_public_fail(sink, emit_fail(vi, mark, st));
    }
    st = after_record(vi, NYTP_EVT_START_DEFLATE, mark);
    if (st != NYTP_OK) {
        return v6_public_fail(sink, emit_fail(vi, mark, st));
    }
    /* 2. Seal current region under current codec (packing continues). */
    st = seal_open_event_region(vi, sink);
    if (st != NYTP_OK) {
        /*
         * START_DEFLATE was committed (after_record cleared emit snap).
         * Reverse packing seq for that one record, then roll body so a later
         * begin_codec_region retry does not double-emit the marker.
         */
        if (vi->enable_packing && vi->pack.next_seq > 0) {
            vi->pack.next_seq--;
        }
        return v6_public_fail(sink, emit_fail(vi, mark, st));
    }
    /* 3. Switch codec for subsequent emits. */
    vi->event_codec = next_codec;
    return NYTP_OK;
}

nytp_status nytp_v6_sink_emit_time_line_run(nytp_sink *sink, nytp_fid fid,
                                            nytp_line line,
                                            const uint64_t *ticks,
                                            size_t n_ticks)
{
    v6_impl *vi;
    size_t mark;
    size_t i;
    nytp_status st;
    if (!nytp_v6_sink_is_v6(sink) || !sink->impl) {
        return NYTP_ERR_NULL;
    }
    vi = (v6_impl *)sink->impl;
    if (vi->sealed) {
        return NYTP_ERR_STATE;
    }
    if (sink->state == NYTP_SINK_FAILED) {
        return NYTP_ERR_STATE;
    }
    /* Lifecycle: TIME_LINE_RUN expands to logical TIME_LINE events. */
    if (!nytp_sink_can_emit(sink, NYTP_EVT_TIME_LINE)) {
        return NYTP_ERR_STATE;
    }
    if (!vi->enable_packing) {
        return NYTP_ERR_UNSUPPORTED;
    }
    if (!ticks || n_ticks == 0) {
        return NYTP_ERR_OVERFLOW; /* empty run fail-closed */
    }
    if (n_ticks > NYTPROF_V6_MAX_TIME_RUN_LEN) {
        return v6_public_fail(sink, NYTP_ERR_OVERFLOW);
    }
    mark = vi->body_len;
    /* TIME_LINE_RUN: absolute site + FLAG_HAS_SEQ base only (no SITE_DELTA). */
    st = body_begin_op(vi, NYTPROF_V6_OP_TIME_LINE_RUN, 0);
    if (st != NYTP_OK) {
        return v6_public_fail(sink, emit_fail(vi, mark, st));
    }
    st = body_uleb(vi, (uint64_t)fid);
    if (st != NYTP_OK) {
        return v6_public_fail(sink, emit_fail(vi, mark, st));
    }
    st = body_uleb(vi, (uint64_t)line);
    if (st != NYTP_OK) {
        return v6_public_fail(sink, emit_fail(vi, mark, st));
    }
    st = body_uleb(vi, (uint64_t)n_ticks);
    if (st != NYTP_OK) {
        return v6_public_fail(sink, emit_fail(vi, mark, st));
    }
    for (i = 0; i < n_ticks; i++) {
        st = body_uleb(vi, ticks[i]);
        if (st != NYTP_OK) {
            return v6_public_fail(sink, emit_fail(vi, mark, st));
        }
    }
    /* after_record_n advances packing by n_ticks; after_record would only +1. */
    st = after_record_n(vi, NYTP_EVT_TIME_LINE, mark, (uint64_t)n_ticks);
    if (st != NYTP_OK) {
        return v6_public_fail(sink, emit_fail(vi, mark, st));
    }
    vi->pack.fid = (uint64_t)fid;
    vi->pack.line = (uint64_t)line;
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    if (n_ticks) {
        vi->stats.last_ticks = (nytp_ticks)ticks[n_ticks - 1u];
    }
    /* COL-003: expand run to N logical TIME_LINE seq commits. */
    v6_commit_logical_n(sink, NYTP_EVT_TIME_LINE, (uint64_t)n_ticks);
    return NYTP_OK;
}


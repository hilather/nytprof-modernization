/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-007 (PR-B08) — packing + FOOTER string-dict + mid-stream codec region.
 *
 * Validates ADR-0001/0002 intent in the C v6 sink:
 *   - site-delta + FLAG_HAS_SEQ packing on TIME_LINE / TIME_BLOCK / SUB_ENTRY
 *   - multi-chunk packing continuity (shared packing state across partitions)
 *   - TIME_LINE_RUN packed form
 *   - mid-stream begin_codec_region: START_DEFLATE + codec switch
 *   - FOOTER-local string dictionary intern + resolve shape
 *
 * Build/run: make -C collector test
 */
#include "nytp_sink.h"
#include "nytp_sink_v6.h"
#include "nytprof_v6_ids.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <zlib.h>

static int failures = 0;

#define EXPECT(cond, msg)                                                      \
    do {                                                                       \
        if (!(cond)) {                                                         \
            fprintf(stderr, "FAIL: %s (%s:%d)\n", (msg), __FILE__, __LINE__);  \
            failures++;                                                        \
        }                                                                      \
    } while (0)

static uint32_t ref_crc32(const uint8_t *data, size_t len)
{
    uLong c = crc32(0L, Z_NULL, 0);
    if (len && data) {
        c = crc32(c, data, (uInt)len);
    }
    return (uint32_t)c;
}

static size_t read_uleb(const uint8_t *data, size_t len, size_t *pos,
                        uint64_t *out)
{
    uint64_t result = 0;
    unsigned shift = 0;
    size_t i;
    for (i = 0; i < 10; i++) {
        uint8_t byte;
        if (*pos >= len) {
            return 0;
        }
        byte = data[(*pos)++];
        result |= ((uint64_t)(byte & 0x7f)) << shift;
        if ((byte & 0x80) == 0) {
            *out = result;
            return 1;
        }
        shift += 7;
        if (shift >= 64) {
            return 0;
        }
    }
    return 0;
}

static int64_t zigzag_decode(uint64_t u)
{
    return (int64_t)(u >> 1) ^ (-(int64_t)(u & 1));
}

static size_t read_zigzag(const uint8_t *data, size_t len, size_t *pos,
                          int64_t *out)
{
    uint64_t u;
    if (!read_uleb(data, len, pos, &u)) {
        return 0;
    }
    *out = zigzag_decode(u);
    return 1;
}

static int skip_prefix(const uint8_t *wire, size_t wire_len, size_t *out_pos)
{
    size_t pos;
    uint32_t header_len;
    if (wire_len < NYTPROF_V6_HEADER_LEN_FULL) {
        return 0;
    }
    if (memcmp(wire, "NYTPROF6", 8) != 0) {
        return 0;
    }
    header_len = (uint32_t)wire[12] | ((uint32_t)wire[13] << 8) |
                 ((uint32_t)wire[14] << 16) | ((uint32_t)wire[15] << 24);
    if (header_len != NYTPROF_V6_HEADER_LEN_FULL) {
        return 0;
    }
    pos = header_len;
    while (pos < wire_len) {
        uint64_t tid, vlen;
        size_t p = pos;
        if (!read_uleb(wire, wire_len, &p, &tid) ||
            !read_uleb(wire, wire_len, &p, &vlen) || p >= wire_len) {
            return 0;
        }
        p++; /* flags */
        if (p + (size_t)vlen > wire_len) {
            return 0;
        }
        p += (size_t)vlen;
        pos = p;
        if (tid == NYTPROF_V6_TLV_END) {
            break;
        }
    }
    *out_pos = pos;
    return 1;
}

typedef struct chunk_view {
    uint8_t kind;
    uint8_t codec;
    uint64_t sequence;
    uint32_t logical_count;
    uint32_t uncompressed_len;
    uint32_t compressed_len;
    uint32_t checksum;
    const uint8_t *payload;
} chunk_view;

static int parse_chunk_at(const uint8_t *wire, size_t wire_len, size_t *pos,
                          chunk_view *out)
{
    size_t p = *pos;
    size_t i;
    if (p + NYTPROF_V6_CHUNK_HEADER_LEN > wire_len) {
        return 0;
    }
    {
        uint32_t sync = (uint32_t)wire[p] | ((uint32_t)wire[p + 1] << 8) |
                        ((uint32_t)wire[p + 2] << 16) |
                        ((uint32_t)wire[p + 3] << 24);
        if (sync != NYTPROF_V6_CHUNK_SYNC) {
            return 0;
        }
    }
    out->kind = wire[p + 4];
    out->codec = wire[p + 5];
    out->sequence = 0;
    for (i = 0; i < 8; i++) {
        out->sequence |= ((uint64_t)wire[p + 8 + i]) << (8 * i);
    }
    out->logical_count = (uint32_t)wire[p + 24] |
                         ((uint32_t)wire[p + 25] << 8) |
                         ((uint32_t)wire[p + 26] << 16) |
                         ((uint32_t)wire[p + 27] << 24);
    out->uncompressed_len = (uint32_t)wire[p + 28] |
                            ((uint32_t)wire[p + 29] << 8) |
                            ((uint32_t)wire[p + 30] << 16) |
                            ((uint32_t)wire[p + 31] << 24);
    out->compressed_len = (uint32_t)wire[p + 32] |
                          ((uint32_t)wire[p + 33] << 8) |
                          ((uint32_t)wire[p + 34] << 16) |
                          ((uint32_t)wire[p + 35] << 24);
    out->checksum = (uint32_t)wire[p + 36] | ((uint32_t)wire[p + 37] << 8) |
                    ((uint32_t)wire[p + 38] << 16) |
                    ((uint32_t)wire[p + 39] << 24);
    p += NYTPROF_V6_CHUNK_HEADER_LEN;
    if (p + out->compressed_len > wire_len) {
        return 0;
    }
    out->payload = wire + p;
    *pos = p + out->compressed_len;
    return 1;
}

static int inflate_none(const chunk_view *ch, uint8_t **out, size_t *out_len)
{
    *out = NULL;
    *out_len = 0;
    if (ch->codec != NYTPROF_V6_CODEC_NONE) {
        return 0;
    }
    if (ch->uncompressed_len != ch->compressed_len) {
        return 0;
    }
    if (ch->compressed_len == 0) {
        return 1;
    }
    *out = (uint8_t *)malloc(ch->compressed_len);
    if (!*out) {
        return 0;
    }
    memcpy(*out, ch->payload, ch->compressed_len);
    *out_len = ch->compressed_len;
    return 1;
}

/* Join all EVENT plains from wire (NONE codec only). */
static int join_event_plains(const uint8_t *wire, size_t wlen, uint8_t **out,
                             size_t *out_len, uint32_t *n_event_chunks,
                             int *saw_footer, const uint8_t **footer,
                             size_t *footer_len)
{
    size_t pos = 0;
    uint8_t *acc = NULL;
    size_t acc_len = 0;
    *out = NULL;
    *out_len = 0;
    *n_event_chunks = 0;
    *saw_footer = 0;
    *footer = NULL;
    *footer_len = 0;
    if (!skip_prefix(wire, wlen, &pos)) {
        return 0;
    }
    while (pos < wlen) {
        chunk_view ch;
        uint8_t *plain = NULL;
        size_t plain_len = 0;
        if (!parse_chunk_at(wire, wlen, &pos, &ch)) {
            free(acc);
            return 0;
        }
        EXPECT(ch.checksum == ref_crc32(ch.payload, ch.compressed_len), "crc");
        if (ch.kind == NYTPROF_V6_KIND_EVENT) {
            if (!inflate_none(&ch, &plain, &plain_len)) {
                free(acc);
                free(plain);
                return 0;
            }
            if (plain_len) {
                uint8_t *n = (uint8_t *)realloc(acc, acc_len + plain_len);
                if (!n) {
                    free(acc);
                    free(plain);
                    return 0;
                }
                acc = n;
                memcpy(acc + acc_len, plain, plain_len);
                acc_len += plain_len;
            }
            free(plain);
            (*n_event_chunks)++;
        } else if (ch.kind == NYTPROF_V6_KIND_FOOTER) {
            EXPECT(ch.codec == NYTPROF_V6_CODEC_NONE, "footer NONE");
            *saw_footer = 1;
            *footer = ch.payload;
            *footer_len = ch.compressed_len;
        } else {
            free(acc);
            return 0;
        }
    }
    *out = acc;
    *out_len = acc_len;
    return 1;
}

/* Decode packed TIME_LINE sequence with continuous site cursor + seq. */
static int decode_packed_time_lines(const uint8_t *body, size_t blen,
                                    uint64_t *fids, uint64_t *lines,
                                    uint64_t *ticks, uint64_t *seqs,
                                    size_t max, size_t *n_out)
{
    size_t pos = 0;
    uint64_t base_fid = 0, base_line = 0;
    size_t n = 0;
    *n_out = 0;
    while (pos < blen && n < max) {
        uint64_t op, seq;
        uint8_t flags;
        int64_t df, dl;
        uint64_t t;
        if (!read_uleb(body, blen, &pos, &op)) {
            return 0;
        }
        if (pos >= blen) {
            return 0;
        }
        flags = body[pos++];
        if (op == NYTPROF_V6_OP_TIME_LINE) {
            EXPECT((flags & NYTPROF_V6_FLAG_SITE_DELTA) != 0, "site delta");
            EXPECT((flags & NYTPROF_V6_FLAG_HAS_SEQ) != 0, "has seq");
            if (!read_uleb(body, blen, &pos, &seq)) {
                return 0;
            }
            if (!read_zigzag(body, blen, &pos, &df) ||
                !read_zigzag(body, blen, &pos, &dl) ||
                !read_uleb(body, blen, &pos, &t)) {
                return 0;
            }
            base_fid = (uint64_t)((int64_t)base_fid + df);
            base_line = (uint64_t)((int64_t)base_line + dl);
            fids[n] = base_fid;
            lines[n] = base_line;
            ticks[n] = t;
            seqs[n] = seq;
            n++;
        } else if (op == NYTPROF_V6_OP_TIME_LINE_RUN) {
            uint64_t fid, line, count, i;
            EXPECT((flags & NYTPROF_V6_FLAG_HAS_SEQ) != 0, "run has seq");
            EXPECT((flags & NYTPROF_V6_FLAG_SITE_DELTA) == 0, "run no site delta");
            if (!read_uleb(body, blen, &pos, &seq) ||
                !read_uleb(body, blen, &pos, &fid) ||
                !read_uleb(body, blen, &pos, &line) ||
                !read_uleb(body, blen, &pos, &count)) {
                return 0;
            }
            base_fid = fid;
            base_line = line;
            for (i = 0; i < count && n < max; i++) {
                if (!read_uleb(body, blen, &pos, &t)) {
                    return 0;
                }
                fids[n] = fid;
                lines[n] = line;
                ticks[n] = t;
                seqs[n] = seq + i;
                n++;
            }
        } else if (op == NYTPROF_V6_OP_START_DEFLATE) {
            if (flags & NYTPROF_V6_FLAG_HAS_SEQ) {
                if (!read_uleb(body, blen, &pos, &seq)) {
                    return 0;
                }
            }
            /* empty body */
        } else {
            /* skip unknown for this test by failing closed */
            return 0;
        }
    }
    *n_out = n;
    return pos == blen || n == max;
}

static void test_packing_site_delta_seq_single(void)
{
    nytp_v6_sink_options opt;
    nytp_sink *s;
    const uint8_t *wire;
    size_t wlen = 0;
    uint8_t *plain = NULL;
    size_t plen = 0;
    uint32_t nchunks = 0;
    int saw_footer = 0;
    const uint8_t *footer = NULL;
    size_t flen = 0;
    uint64_t fids[8], lines[8], ticks[8], seqs[8];
    size_t n = 0;

    memset(&opt, 0, sizeof(opt));
    opt.event_codec = (uint8_t)NYTPROF_V6_CODEC_NONE;
    opt.enable_packing = 1;
    s = nytp_v6_sink_create_opts(NULL, &opt);
    EXPECT(s != NULL, "create packing");
    EXPECT(nytp_v6_sink_packing_enabled(s), "packing on");
    EXPECT(nytp_emit_time_line(s, 10, 1, 1) == NYTP_OK, "tl1");
    EXPECT(nytp_emit_time_line(s, 20, 1, 2) == NYTP_OK, "tl2");
    EXPECT(nytp_emit_time_line(s, 30, 2, 5) == NYTP_OK, "tl3");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire && join_event_plains(wire, wlen, &plain, &plen, &nchunks,
                                     &saw_footer, &footer, &flen),
           "join");
    EXPECT(nchunks == 1, "1 chunk");
    EXPECT(!saw_footer, "no footer");
    EXPECT(decode_packed_time_lines(plain, plen, fids, lines, ticks, seqs, 8,
                                    &n),
           "decode packed");
    EXPECT(n == 3, "3 lines");
    EXPECT(fids[0] == 1 && lines[0] == 1 && ticks[0] == 10 && seqs[0] == 0,
           "abs0");
    EXPECT(fids[1] == 1 && lines[1] == 2 && ticks[1] == 20 && seqs[1] == 1,
           "abs1");
    EXPECT(fids[2] == 2 && lines[2] == 5 && ticks[2] == 30 && seqs[2] == 2,
           "abs2");
    free(plain);
    nytp_sink_destroy(s);
}

static void test_packing_multi_chunk_continuity(void)
{
    nytp_v6_sink_options opt;
    nytp_sink *s;
    const uint8_t *wire;
    size_t wlen = 0;
    uint8_t *plain = NULL;
    size_t plen = 0;
    uint32_t nchunks = 0;
    int saw_footer = 0;
    const uint8_t *footer = NULL;
    size_t flen = 0;
    uint64_t fids[8], lines[8], ticks[8], seqs[8];
    size_t n = 0;

    memset(&opt, 0, sizeof(opt));
    opt.event_codec = (uint8_t)NYTPROF_V6_CODEC_NONE;
    opt.max_records_per_chunk = 1;
    opt.enable_packing = 1;
    s = nytp_v6_sink_create_opts("build/v6_pack_multi.nytprof", &opt);
    EXPECT(s != NULL, "create multi packing");
    EXPECT(nytp_emit_time_line(s, 10, 1, 1) == NYTP_OK, "tl1");
    EXPECT(nytp_emit_time_line(s, 20, 1, 2) == NYTP_OK, "tl2");
    EXPECT(nytp_emit_time_line(s, 30, 1, 3) == NYTP_OK, "tl3");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    EXPECT(nytp_v6_sink_event_chunk_count(s) == 3, "3 chunks");
    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire && join_event_plains(wire, wlen, &plain, &plen, &nchunks,
                                     &saw_footer, &footer, &flen),
           "join multi");
    EXPECT(nchunks == 3, "3 event chunks");
    /* Joined plain must reconstruct continuous sites/seqs (not per-chunk reset). */
    EXPECT(decode_packed_time_lines(plain, plen, fids, lines, ticks, seqs, 8,
                                    &n),
           "decode joined");
    EXPECT(n == 3, "3 lines multi");
    EXPECT(seqs[0] == 0 && seqs[1] == 1 && seqs[2] == 2, "seq continuous");
    EXPECT(fids[0] == 1 && lines[0] == 1, "site0");
    EXPECT(fids[1] == 1 && lines[1] == 2, "site1");
    EXPECT(fids[2] == 1 && lines[2] == 3, "site2");
    free(plain);
    nytp_sink_destroy(s);
}

static void test_time_line_run(void)
{
    nytp_v6_sink_options opt;
    nytp_sink *s;
    uint64_t run_ticks[3] = {11, 22, 33};
    const uint8_t *wire;
    size_t wlen = 0;
    uint8_t *plain = NULL;
    size_t plen = 0;
    uint32_t nchunks = 0;
    int saw_footer = 0;
    const uint8_t *footer = NULL;
    size_t flen = 0;
    uint64_t fids[8], lines[8], ticks[8], seqs[8];
    size_t n = 0;

    memset(&opt, 0, sizeof(opt));
    opt.enable_packing = 1;
    s = nytp_v6_sink_create_opts(NULL, &opt);
    EXPECT(s != NULL, "create run");
    EXPECT(nytp_v6_sink_emit_time_line_run(s, 5, 9, run_ticks, 3) == NYTP_OK,
           "run");
    /* Following site-delta must continue from run site (5,9). */
    EXPECT(nytp_emit_time_line(s, 44, 5, 10) == NYTP_OK, "after run");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close run");
    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire && join_event_plains(wire, wlen, &plain, &plen, &nchunks,
                                     &saw_footer, &footer, &flen),
           "join run");
    EXPECT(decode_packed_time_lines(plain, plen, fids, lines, ticks, seqs, 8,
                                    &n),
           "decode run");
    EXPECT(n == 4, "3 expanded + 1");
    EXPECT(fids[0] == 5 && lines[0] == 9 && ticks[0] == 11 && seqs[0] == 0,
           "run0");
    EXPECT(ticks[1] == 22 && seqs[1] == 1, "run1");
    EXPECT(ticks[2] == 33 && seqs[2] == 2, "run2");
    EXPECT(fids[3] == 5 && lines[3] == 10 && ticks[3] == 44 && seqs[3] == 3,
           "after");
    free(plain);
    nytp_sink_destroy(s);
}

static int inflate_zlib(const chunk_view *ch, uint8_t **out, size_t *out_len)
{
    uLongf dest_len;
    uint8_t *buf;
    int zst;
    *out = NULL;
    *out_len = 0;
    if (ch->codec != NYTPROF_V6_CODEC_ZLIB) {
        return 0;
    }
    dest_len = ch->uncompressed_len ? ch->uncompressed_len : 1;
    buf = (uint8_t *)malloc(dest_len);
    if (!buf) {
        return 0;
    }
    zst = uncompress(buf, &dest_len, ch->payload, ch->compressed_len);
    if (zst != Z_OK || dest_len != ch->uncompressed_len) {
        free(buf);
        return 0;
    }
    *out = buf;
    *out_len = (size_t)dest_len;
    return 1;
}

static void test_mid_stream_codec_region(void)
{
    nytp_v6_sink_options opt;
    nytp_sink *s;
    const uint8_t *wire;
    size_t wlen = 0, pos = 0;
    chunk_view ch0, ch1;
    uint8_t *p0 = NULL, *p1 = NULL;
    size_t l0 = 0, l1 = 0;
    uint64_t fids[4], lines[4], ticks[4], seqs[4];
    size_t n = 0;
    uint8_t *joined = NULL;
    size_t jlen = 0;

    memset(&opt, 0, sizeof(opt));
    opt.event_codec = (uint8_t)NYTPROF_V6_CODEC_NONE;
    opt.enable_packing = 1;
    s = nytp_v6_sink_create_opts("build/v6_mid_stream.nytprof", &opt);
    EXPECT(s != NULL, "create mid");
    EXPECT(nytp_emit_time_line(s, 10, 1, 1) == NYTP_OK, "pre tl");
    EXPECT(nytp_v6_sink_begin_codec_region(s, (uint8_t)NYTPROF_V6_CODEC_ZLIB) ==
               NYTP_OK,
           "begin zlib");
    EXPECT(nytp_v6_sink_event_codec(s) == NYTPROF_V6_CODEC_ZLIB, "codec now zlib");
    EXPECT(nytp_v6_sink_event_chunk_count(s) == 1, "pre region sealed 1 chunk");
    EXPECT(nytp_emit_time_line(s, 20, 1, 2) == NYTP_OK, "post tl");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close mid");
    EXPECT(nytp_v6_sink_event_chunk_count(s) == 2, "2 event chunks");
    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire && skip_prefix(wire, wlen, &pos), "prefix");
    EXPECT(parse_chunk_at(wire, wlen, &pos, &ch0), "ch0");
    EXPECT(ch0.kind == NYTPROF_V6_KIND_EVENT && ch0.codec == NYTPROF_V6_CODEC_NONE,
           "pre NONE");
    EXPECT(ch0.sequence == 0 && ch0.logical_count == 2, "pre: tl + START_DEFLATE");
    EXPECT(inflate_none(&ch0, &p0, &l0) && l0 > 0, "pre plain");
    /* Pre body ends with START_DEFLATE after first TIME_LINE. */
    {
        size_t p = 0;
        uint64_t op;
        EXPECT(read_uleb(p0, l0, &p, &op) && op == NYTPROF_V6_OP_TIME_LINE,
               "pre first TIME_LINE");
    }
    EXPECT(parse_chunk_at(wire, wlen, &pos, &ch1), "ch1");
    EXPECT(ch1.kind == NYTPROF_V6_KIND_EVENT && ch1.codec == NYTPROF_V6_CODEC_ZLIB,
           "post ZLIB");
    EXPECT(ch1.sequence == 1 && ch1.logical_count == 1, "post one record");
    EXPECT(ch1.checksum == ref_crc32(ch1.payload, ch1.compressed_len), "post crc");
    EXPECT(inflate_zlib(&ch1, &p1, &l1) && l1 > 0, "post inflate zlib");
    /* Join pre||post plains and assert continuous packing (seq 0,1,2; site 1:1→1:2). */
    joined = (uint8_t *)malloc(l0 + l1);
    EXPECT(joined != NULL, "joined alloc");
    if (joined) {
        memcpy(joined, p0, l0);
        memcpy(joined + l0, p1, l1);
        jlen = l0 + l1;
        EXPECT(decode_packed_time_lines(joined, jlen, fids, lines, ticks, seqs, 4,
                                        &n),
               "decode mid joined");
        EXPECT(n == 2, "two TIME_LINE logical (START_DEFLATE skipped)");
        EXPECT(fids[0] == 1 && lines[0] == 1 && ticks[0] == 10 && seqs[0] == 0,
               "pre site/seq");
        EXPECT(fids[1] == 1 && lines[1] == 2 && ticks[1] == 20 && seqs[1] == 2,
               "post continues after START_DEFLATE seq=1");
        free(joined);
    }
    free(p0);
    free(p1);
    /* Same codec fail-closed */
    {
        nytp_sink *s2 = nytp_v6_sink_create_opts(NULL, &opt);
        EXPECT(nytp_emit_time_line(s2, 1, 1, 1) == NYTP_OK, "s2 tl");
        EXPECT(nytp_v6_sink_begin_codec_region(s2, (uint8_t)NYTPROF_V6_CODEC_NONE) ==
                   NYTP_ERR_UNSUPPORTED,
               "same codec rejected");
        nytp_sink_destroy(s2);
    }
    /* Empty open body: fail-closed before marker. */
    {
        nytp_sink *s3 = nytp_v6_sink_create_opts(NULL, &opt);
        EXPECT(nytp_v6_sink_begin_codec_region(s3, (uint8_t)NYTPROF_V6_CODEC_ZLIB) ==
                   NYTP_ERR_STATE,
               "empty body begin rejected");
        nytp_sink_destroy(s3);
    }
    /* Lifecycle: STOPPED rejects begin and run. */
    {
        nytp_v6_sink_options o2;
        nytp_sink *s4;
        uint64_t rt[1] = {1};
        memset(&o2, 0, sizeof(o2));
        o2.enable_packing = 1;
        s4 = nytp_v6_sink_create_opts(NULL, &o2);
        EXPECT(nytp_sink_activate(s4) == NYTP_OK, "activate");
        EXPECT(nytp_emit_time_line(s4, 1, 1, 1) == NYTP_OK, "tl");
        EXPECT(nytp_sink_stop(s4) == NYTP_OK, "stop");
        EXPECT(nytp_v6_sink_begin_codec_region(s4, (uint8_t)NYTPROF_V6_CODEC_ZLIB) ==
                   NYTP_ERR_STATE,
               "stopped begin rejected");
        EXPECT(nytp_v6_sink_emit_time_line_run(s4, 1, 1, rt, 1) == NYTP_ERR_STATE,
               "stopped run rejected");
        nytp_sink_destroy(s4);
    }
    /* Seal fail after START_DEFLATE: marker rolled back (no double-emit residue). */
    {
        nytp_sink *s5 = nytp_v6_sink_create_opts(NULL, &opt);
        size_t blen0 = 0, blen1 = 0;
        const uint8_t *body;
        EXPECT(nytp_emit_time_line(s5, 1, 1, 1) == NYTP_OK, "s5 tl");
        body = nytp_v6_sink_event_body(s5, &blen0);
        EXPECT(body && blen0 > 0, "body pre");
        nytp_v6_sink_test_fail_seal_after_chunks(s5, 1);
        EXPECT(nytp_v6_sink_begin_codec_region(s5, (uint8_t)NYTPROF_V6_CODEC_ZLIB) ==
                   NYTP_ERR_IO,
               "injected seal fail");
        body = nytp_v6_sink_event_body(s5, &blen1);
        /* Marker rolled back: open body length restored to pre-begin. */
        EXPECT(blen1 == blen0, "START_DEFLATE rolled back on seal fail");
        EXPECT(nytp_sink_get_state(s5) == NYTP_SINK_FAILED, "sticky fail on IO");
        nytp_sink_destroy(s5);
    }
    nytp_sink_destroy(s);
}

/* FOOTER dict intern rolled back when a later field of the same emit fails. */
static void test_dict_intern_rollback_on_emit_fail(void)
{
    nytp_v6_sink_options opt;
    nytp_sink *s;
    nytp_string_view huge;
    uint32_t before;

    memset(&opt, 0, sizeof(opt));
    opt.enable_string_dict = 1;
    s = nytp_v6_sink_create_opts(NULL, &opt);
    EXPECT(s != NULL, "create");
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("keep-me")) == NYTP_OK, "keep");
    before = nytp_v6_sink_dict_entry_count(s);
    EXPECT(before >= 1, "interned keep-me");
    /* Force mid-record fail after first string intern: oversize value. */
    huge.ptr = "x";
    huge.len = (size_t)NYTPROF_V6_MAX_STRING_BYTES + 1u;
    huge.is_utf8 = 0;
    EXPECT(nytp_emit_attribute(s, nytp_sv_cstr("orphan-key"), huge) != NYTP_OK,
           "oversize attr fails");
    /* Orphan key must not remain in the FOOTER table. */
    EXPECT(nytp_v6_sink_dict_entry_count(s) == before, "dict rolled back");
    EXPECT(nytp_sink_close(s) == NYTP_OK || nytp_sink_get_state(s) == NYTP_SINK_FAILED,
           "close or sticky");
    if (nytp_v6_sink_is_sealed(s) && nytp_v6_sink_has_footer_dict(s)) {
        EXPECT(nytp_v6_sink_dict_entry_count(s) == before, "footer no orphan");
    }
    nytp_sink_destroy(s);
}

/* COL-003: TIME_LINE_RUN advances logical sink seq by n_ticks. */
static void test_time_line_run_col003_seq(void)
{
    nytp_v6_sink_options opt;
    nytp_sink *s;
    uint64_t run_ticks[3] = {1, 2, 3};
    nytp_seq last = 0;

    memset(&opt, 0, sizeof(opt));
    opt.enable_packing = 1;
    s = nytp_v6_sink_create_opts(NULL, &opt);
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "activate");
    EXPECT(nytp_sink_logical_count(s) == 0, "seq start 0");
    EXPECT(nytp_v6_sink_emit_time_line_run(s, 1, 1, run_ticks, 3) == NYTP_OK,
           "run3");
    EXPECT(nytp_sink_logical_count(s) == 3, "logical +3");
    EXPECT(nytp_sink_last_seq(s, &last) == NYTP_OK && last == 2, "last seq 2");
    EXPECT(nytp_emit_time_line(s, 4, 1, 2) == NYTP_OK, "tl");
    EXPECT(nytp_sink_logical_count(s) == 4, "logical +1");
    nytp_sink_destroy(s);
}

static void test_footer_string_dict(void)
{
    nytp_v6_sink_options opt;
    nytp_sink *s;
    const uint8_t *wire;
    size_t wlen = 0;
    uint8_t *plain = NULL;
    size_t plen = 0;
    uint32_t nchunks = 0;
    int saw_footer = 0;
    const uint8_t *footer = NULL;
    size_t flen = 0;
    size_t pos = 0;
    uint64_t entry_count = 0, id = 0, blen = 0;
    uint8_t flags;

    memset(&opt, 0, sizeof(opt));
    opt.enable_string_dict = 1;
    s = nytp_v6_sink_create_opts("build/v6_dict.nytprof", &opt);
    EXPECT(s != NULL, "create dict");
    EXPECT(nytp_v6_sink_string_dict_enabled(s), "dict on");
    EXPECT(nytp_emit_attribute(s, nytp_sv_cstr("ticks_per_sec"),
                               nytp_sv_cstr("10000000")) == NYTP_OK,
           "attr");
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("hello-dict")) == NYTP_OK, "comment");
    /* Reuse same comment string → same id interned once more? second comment */
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("hello-dict")) == NYTP_OK,
           "comment2");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close dict");
    EXPECT(nytp_v6_sink_has_footer_dict(s), "has footer");
    EXPECT(nytp_v6_sink_dict_entry_count(s) >= 2, "at least key+value+comment ids");
    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire && join_event_plains(wire, wlen, &plain, &plen, &nchunks,
                                     &saw_footer, &footer, &flen),
           "join dict");
    EXPECT(saw_footer && footer && flen > 0, "footer present");
    EXPECT(read_uleb(footer, flen, &pos, &entry_count) && entry_count >= 2,
           "entry_count");
    EXPECT(read_uleb(footer, flen, &pos, &id) && id != 0, "first id");
    EXPECT(pos < flen, "flags byte");
    flags = footer[pos++];
    (void)flags;
    EXPECT(read_uleb(footer, flen, &pos, &blen), "byte_len");
    EXPECT(pos + blen <= flen, "bytes fit");
    /* EVENT body ATTRIBUTE should use non-zero string_id with empty inline. */
    {
        size_t p = 0;
        uint64_t op, sid, slen;
        uint8_t f;
        EXPECT(read_uleb(plain, plen, &p, &op) && op == NYTPROF_V6_OP_ATTRIBUTE,
               "attr op");
        EXPECT(p < plen && plain[p] == 0, "absolute flags");
        p++;
        EXPECT(read_uleb(plain, plen, &p, &sid) && sid != 0, "key id nonzero");
        EXPECT(read_uleb(plain, plen, &p, &slen) && slen == 0, "key inline empty");
        EXPECT(p < plen, "key flags");
        f = plain[p++];
        (void)f;
    }
    free(plain);
    nytp_sink_destroy(s);
}

static void test_packing_plus_dict_multi_chunk(void)
{
    nytp_v6_sink_options opt;
    nytp_sink *s;
    const uint8_t *wire;
    size_t wlen = 0;
    uint8_t *plain = NULL;
    size_t plen = 0;
    uint32_t nchunks = 0;
    int saw_footer = 0;
    const uint8_t *footer = NULL;
    size_t flen = 0;

    memset(&opt, 0, sizeof(opt));
    opt.event_codec = (uint8_t)NYTPROF_V6_CODEC_NONE;
    opt.max_records_per_chunk = 1;
    opt.enable_packing = 1;
    opt.enable_string_dict = 1;
    s = nytp_v6_sink_create_opts("build/v6_pack_dict.nytprof", &opt);
    EXPECT(s != NULL, "create pack+dict");
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("c")) == NYTP_OK, "c");
    EXPECT(nytp_emit_time_line(s, 1, 1, 1) == NYTP_OK, "tl");
    EXPECT(nytp_emit_time_line(s, 2, 1, 2) == NYTP_OK, "tl2");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    EXPECT(nytp_v6_sink_has_footer_dict(s), "footer");
    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire && join_event_plains(wire, wlen, &plain, &plen, &nchunks,
                                     &saw_footer, &footer, &flen),
           "join");
    EXPECT(nchunks == 3 && saw_footer, "3 EVENT + FOOTER");
    free(plain);
    nytp_sink_destroy(s);
}

int main(void)
{
    test_packing_site_delta_seq_single();
    test_packing_multi_chunk_continuity();
    test_time_line_run();
    test_time_line_run_col003_seq();
    test_mid_stream_codec_region();
    test_footer_string_dict();
    test_dict_intern_rollback_on_emit_fail();
    test_packing_plus_dict_multi_chunk();

    if (failures) {
        fprintf(stderr, "test_v6_packing_footer: %d failure(s)\n", failures);
        return 1;
    }
    printf("test_v6_packing_footer: OK\n");
    return 0;
}

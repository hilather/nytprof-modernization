/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-007 (PR-B07) — codecs + multi-chunk + CRC unit vectors.
 *
 * Validates:
 *   - header CRC32 sealed over fixed-header [0,32)
 *   - per-chunk payload CRC32 over wire payload bytes
 *   - multi-chunk EVENT partition (max_records_per_chunk)
 *   - EVENT codecs NONE / ZLIB / ZSTD / LZ4 inflate roundtrip
 *   - unsupported codec create fails closed
 *   - default create remains codec NONE single-chunk with sealed CRC
 *
 * Build/run: make -C collector test
 */
#include "nytp_sink.h"
#include "nytp_sink_v6.h"
#include "nytprof_v6_ids.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <lz4.h>
#include <zlib.h>
#include <zstd.h>

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

/* Skip fixed header + TLV END; return pos of first chunk or wire_len. */
static int skip_prefix(const uint8_t *wire, size_t wire_len, size_t *out_pos,
                       uint32_t *header_crc_out)
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
    if (header_crc_out) {
        *header_crc_out = (uint32_t)wire[32] | ((uint32_t)wire[33] << 8) |
                          ((uint32_t)wire[34] << 16) |
                          ((uint32_t)wire[35] << 24);
    }
    pos = header_len;
    while (pos < wire_len) {
        uint64_t tid, vlen;
        uint8_t flags;
        size_t p = pos;
        if (!read_uleb(wire, wire_len, &p, &tid) ||
            !read_uleb(wire, wire_len, &p, &vlen) || p >= wire_len) {
            return 0;
        }
        flags = wire[p++];
        (void)flags;
        if (p + (size_t)vlen > wire_len) {
            return 0;
        }
        p += (size_t)vlen;
        pos = p;
        if (tid == NYTPROF_V6_TLV_END) {
            if (vlen != 0) {
                return 0;
            }
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
    {
        size_t i;
        for (i = 0; i < 8; i++) {
            out->sequence |= ((uint64_t)wire[p + 8 + i]) << (8 * i);
        }
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

static int inflate_payload(const chunk_view *ch, uint8_t **out, size_t *out_len)
{
    *out = NULL;
    *out_len = 0;
    if (ch->codec == NYTPROF_V6_CODEC_NONE) {
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
    if (ch->codec == NYTPROF_V6_CODEC_ZLIB) {
        uLongf dest_len = ch->uncompressed_len;
        uint8_t *buf = (uint8_t *)malloc(dest_len ? dest_len : 1);
        int zst;
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
    if (ch->codec == NYTPROF_V6_CODEC_ZSTD) {
        uint8_t *buf = (uint8_t *)malloc(ch->uncompressed_len
                                             ? ch->uncompressed_len
                                             : 1);
        size_t n;
        if (!buf) {
            return 0;
        }
        n = ZSTD_decompress(buf, ch->uncompressed_len, ch->payload,
                            ch->compressed_len);
        if (ZSTD_isError(n) || n != ch->uncompressed_len) {
            free(buf);
            return 0;
        }
        *out = buf;
        *out_len = n;
        return 1;
    }
    if (ch->codec == NYTPROF_V6_CODEC_LZ4) {
        uint8_t *buf = (uint8_t *)malloc(ch->uncompressed_len
                                             ? ch->uncompressed_len
                                             : 1);
        int n;
        if (!buf) {
            return 0;
        }
        if (ch->uncompressed_len == 0) {
            free(buf);
            *out = NULL;
            *out_len = 0;
            return ch->compressed_len == 0;
        }
        n = LZ4_decompress_safe((const char *)ch->payload, (char *)buf,
                                (int)ch->compressed_len,
                                (int)ch->uncompressed_len);
        if (n < 0 || (uint32_t)n != ch->uncompressed_len) {
            free(buf);
            return 0;
        }
        *out = buf;
        *out_len = (size_t)n;
        return 1;
    }
    return 0;
}

static void emit_three_lines(nytp_sink *s)
{
    EXPECT(nytp_emit_time_line(s, 10, 1, 1) == NYTP_OK, "tl1");
    EXPECT(nytp_emit_time_line(s, 20, 1, 2) == NYTP_OK, "tl2");
    EXPECT(nytp_emit_time_line(s, 30, 1, 3) == NYTP_OK, "tl3");
}

static void test_header_crc_sealed(void)
{
    nytp_sink *s = nytp_v6_sink_create(NULL);
    const uint8_t *wire;
    size_t wlen = 0;
    uint32_t stored = 0, expect;
    size_t pos = 0;
    EXPECT(s != NULL, "create");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close empty");
    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire && skip_prefix(wire, wlen, &pos, &stored), "prefix");
    expect = ref_crc32(wire, 32);
    EXPECT(stored == expect, "header CRC sealed");
    EXPECT(stored != 0 || expect == 0, "crc field");
    nytp_sink_destroy(s);
}

static void test_default_none_payload_crc(void)
{
    nytp_sink *s = nytp_v6_sink_create(NULL);
    const uint8_t *wire;
    size_t wlen = 0, pos = 0;
    chunk_view ch;
    uint32_t expect;
    EXPECT(s != NULL, "create");
    EXPECT(nytp_v6_sink_event_codec(s) == NYTPROF_V6_CODEC_NONE, "codec none");
    EXPECT(nytp_emit_time_line(s, 42, 1, 5) == NYTP_OK, "emit");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    EXPECT(nytp_v6_sink_event_chunk_count(s) == 1, "1 chunk");
    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire && skip_prefix(wire, wlen, &pos, NULL), "prefix");
    EXPECT(parse_chunk_at(wire, wlen, &pos, &ch), "chunk");
    EXPECT(ch.kind == NYTPROF_V6_KIND_EVENT, "EVENT");
    EXPECT(ch.codec == NYTPROF_V6_CODEC_NONE, "NONE");
    EXPECT(ch.sequence == 0, "seq 0");
    EXPECT(ch.logical_count == 1, "1 record");
    expect = ref_crc32(ch.payload, ch.compressed_len);
    EXPECT(ch.checksum == expect, "payload CRC");
    EXPECT(pos == wlen, "full consume");
    nytp_sink_destroy(s);
}

static void test_multi_chunk_none(void)
{
    nytp_sink *s =
        nytp_v6_sink_create_codec(NULL, (uint8_t)NYTPROF_V6_CODEC_NONE, 1);
    const uint8_t *wire;
    size_t wlen = 0, pos = 0;
    int i;
    uint32_t total_logical = 0;
    EXPECT(s != NULL, "create multi");
    EXPECT(nytp_v6_sink_max_records_per_chunk(s) == 1, "max 1");
    emit_three_lines(s);
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    EXPECT(nytp_v6_sink_event_chunk_count(s) == 3, "3 chunks");
    EXPECT(nytp_v6_sink_event_count(s) == 3, "3 events");
    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire && skip_prefix(wire, wlen, &pos, NULL), "prefix");
    for (i = 0; i < 3; i++) {
        chunk_view ch;
        uint8_t *plain = NULL;
        size_t plain_len = 0;
        EXPECT(parse_chunk_at(wire, wlen, &pos, &ch), "chunk i");
        EXPECT(ch.kind == NYTPROF_V6_KIND_EVENT, "kind");
        EXPECT(ch.codec == NYTPROF_V6_CODEC_NONE, "codec");
        EXPECT(ch.sequence == (uint64_t)i, "sequence");
        EXPECT(ch.logical_count == 1, "one record each");
        EXPECT(ch.checksum == ref_crc32(ch.payload, ch.compressed_len), "crc");
        EXPECT(inflate_payload(&ch, &plain, &plain_len), "inflate none");
        EXPECT(plain_len > 0 && plain != NULL, "plain");
        free(plain);
        total_logical += ch.logical_count;
    }
    EXPECT(total_logical == 3, "total logical");
    EXPECT(pos == wlen, "consume all");
    nytp_sink_destroy(s);
}

static void test_codec_roundtrip(uint8_t codec, const char *path,
                                 size_t max_per)
{
    nytp_sink *s = nytp_v6_sink_create_codec(path, codec, max_per);
    const uint8_t *wire;
    size_t wlen = 0, pos = 0;
    uint32_t chunks_expected;
    uint32_t nchunks = 0;
    uint32_t total_logical = 0;
    char label[64];

    snprintf(label, sizeof(label), "create codec=%u", (unsigned)codec);
    EXPECT(s != NULL, label);
    if (!s) {
        return;
    }
    EXPECT(nytp_v6_sink_event_codec(s) == codec, "codec getter");
    emit_three_lines(s);
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close codec");
    chunks_expected = (max_per == 0) ? 1u : 3u; /* 3 events, max=1 → 3 chunks */
    if (max_per == 0) {
        chunks_expected = 1;
    } else if (max_per >= 3) {
        chunks_expected = 1;
    } else {
        chunks_expected = (uint32_t)((3 + max_per - 1) / max_per);
    }
    EXPECT(nytp_v6_sink_event_chunk_count(s) == chunks_expected, "chunk count");
    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire && wlen > NYTPROF_V6_HEADER_LEN_FULL, "wire");
    EXPECT(skip_prefix(wire, wlen, &pos, NULL), "prefix");
    while (pos < wlen) {
        chunk_view ch;
        uint8_t *plain = NULL;
        size_t plain_len = 0;
        EXPECT(parse_chunk_at(wire, wlen, &pos, &ch), "parse chunk");
        EXPECT(ch.kind == NYTPROF_V6_KIND_EVENT, "EVENT kind");
        EXPECT(ch.codec == codec, "codec match");
        EXPECT(ch.sequence == nchunks, "seq order");
        EXPECT(ch.checksum == ref_crc32(ch.payload, ch.compressed_len),
               "payload crc");
        EXPECT(inflate_payload(&ch, &plain, &plain_len), "inflate");
        EXPECT(plain_len == ch.uncompressed_len, "unc len");
        /* First byte of absolute TIME_LINE is ULEB opcode 2. */
        EXPECT(plain_len >= 2 && plain[0] == NYTPROF_V6_OP_TIME_LINE,
               "TIME_LINE op");
        free(plain);
        total_logical += ch.logical_count;
        nchunks++;
    }
    EXPECT(nchunks == chunks_expected, "nchunks");
    EXPECT(total_logical == 3, "logical total 3");
    if (path) {
        EXPECT(nytp_v6_sink_file_written(s), "file written");
    }
    nytp_sink_destroy(s);
}

static void test_bad_codec_create(void)
{
    EXPECT(nytp_v6_sink_create_codec(NULL, 99, 0) == NULL, "bad codec");
    EXPECT(nytp_v6_sink_create_codec(NULL, 4, 1) == NULL, "codec 4");
}

static void test_multi_chunk_zlib(void)
{
    test_codec_roundtrip((uint8_t)NYTPROF_V6_CODEC_ZLIB,
                         "build/v6_zlib_multi.nytprof", 1);
}

int main(void)
{
    test_header_crc_sealed();
    test_default_none_payload_crc();
    test_multi_chunk_none();
    test_codec_roundtrip((uint8_t)NYTPROF_V6_CODEC_NONE, NULL, 0);
    test_codec_roundtrip((uint8_t)NYTPROF_V6_CODEC_ZLIB,
                         "build/v6_zlib_one.nytprof", 0);
    test_codec_roundtrip((uint8_t)NYTPROF_V6_CODEC_ZSTD,
                         "build/v6_zstd_one.nytprof", 0);
    test_codec_roundtrip((uint8_t)NYTPROF_V6_CODEC_LZ4,
                         "build/v6_lz4_one.nytprof", 0);
    test_multi_chunk_zlib();
    test_codec_roundtrip((uint8_t)NYTPROF_V6_CODEC_ZSTD,
                         "build/v6_zstd_multi.nytprof", 1);
    test_codec_roundtrip((uint8_t)NYTPROF_V6_CODEC_LZ4,
                         "build/v6_lz4_multi.nytprof", 1);
    test_bad_codec_create();

    if (failures) {
        fprintf(stderr, "test_v6_codec_chunk_crc: %d failure(s)\n", failures);
        return 1;
    }
    printf("test_v6_codec_chunk_crc: OK\n");
    return 0;
}

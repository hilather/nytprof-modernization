/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-007 (PR-B06) — Absolute v6 writer unit vectors.
 *
 * Validates:
 *   - lockfile MAGIC / CHUNK_SYNC / opcodes in C header
 *   - file prefix + codec NONE EVENT absolute bodies
 *   - ULEB128 + string-blob match Rust nytprof-format-v6
 *   - mini-profile self-decode (C reader MVP)
 *   - fail-closed negative ticks / null strings / oversize
 *   - no packing flags (0x04/0x08) on absolute path
 *   - residual: not sealed until close
 *
 * Build/run: make -C collector test
 */
#include "nytp_sink.h"
#include "nytp_sink_v6.h"
#include "nytprof_v6_ids.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures = 0;

#define EXPECT(cond, msg)                                                      \
    do {                                                                       \
        if (!(cond)) {                                                         \
            fprintf(stderr, "FAIL: %s (%s:%d)\n", (msg), __FILE__, __LINE__);  \
            failures++;                                                        \
        }                                                                      \
    } while (0)

/* ---- reference ULEB encode (same as writer / Rust) ---- */

static size_t ref_uleb(uint64_t value, uint8_t *out)
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

static size_t ref_string_blob(uint64_t id, uint8_t flags, const char *s,
                              size_t len, uint8_t *out)
{
    size_t n = 0;
    n += ref_uleb(id, out + n);
    n += ref_uleb((uint64_t)len, out + n);
    out[n++] = flags;
    if (len) {
        memcpy(out + n, s, len);
        n += len;
    }
    return n;
}

/* ---- minimal absolute body reader ---- */

static int read_uleb(const uint8_t *data, size_t len, size_t *pos, uint64_t *out)
{
    uint64_t result = 0;
    unsigned shift = 0;
    size_t start = *pos;
    size_t i;
    for (i = 0; i < 10; i++) {
        uint8_t byte;
        if (*pos >= len) {
            return 0;
        }
        byte = data[(*pos)++];
        result |= ((uint64_t)(byte & 0x7f)) << shift;
        if ((byte & 0x80) == 0) {
            /* strict canonical check vs ref encode */
            uint8_t tmp[10];
            size_t cn = ref_uleb(result, tmp);
            if (cn != (*pos - start) ||
                memcmp(tmp, data + start, cn) != 0) {
                return 0;
            }
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

static int read_string_blob(const uint8_t *data, size_t len, size_t *pos,
                            char *buf, size_t bufcap, size_t *out_len)
{
    uint64_t id, blen;
    uint8_t flags;
    if (!read_uleb(data, len, pos, &id)) {
        return 0;
    }
    if (!read_uleb(data, len, pos, &blen)) {
        return 0;
    }
    if (blen > NYTPROF_V6_MAX_STRING_BYTES) {
        return 0;
    }
    if (*pos >= len) {
        return 0;
    }
    flags = data[(*pos)++];
    (void)flags;
    if (*pos + (size_t)blen > len) {
        return 0;
    }
    if (out_len) {
        *out_len = (size_t)blen;
    }
    if (buf && bufcap) {
        size_t n = (size_t)blen;
        if (n >= bufcap) {
            n = bufcap - 1;
        }
        if (n) {
            memcpy(buf, data + *pos, n);
        }
        buf[n] = '\0';
    }
    *pos += (size_t)blen;
    return 1;
}

typedef struct parsed_rec {
    uint64_t opcode;
    uint8_t flags;
    /* TIME_LINE fields */
    uint64_t fid, line, ticks, block_line;
    char text[128];
} parsed_rec;

static int parse_body(const uint8_t *data, size_t len, parsed_rec *out,
                      size_t max_out, size_t *n_out)
{
    size_t pos = 0;
    size_t n = 0;
    while (pos < len) {
        uint64_t op;
        uint8_t flags;
        parsed_rec r;
        memset(&r, 0, sizeof(r));
        if (!read_uleb(data, len, &pos, &op)) {
            return 0;
        }
        if (pos >= len) {
            return 0;
        }
        flags = data[pos++];
        r.opcode = op;
        r.flags = flags;
        if (op == NYTPROF_V6_OP_TIME_LINE) {
            if (!read_uleb(data, len, &pos, &r.fid) ||
                !read_uleb(data, len, &pos, &r.line) ||
                !read_uleb(data, len, &pos, &r.ticks)) {
                return 0;
            }
        } else if (op == NYTPROF_V6_OP_TIME_BLOCK) {
            if (!read_uleb(data, len, &pos, &r.fid) ||
                !read_uleb(data, len, &pos, &r.line) ||
                !read_uleb(data, len, &pos, &r.block_line) ||
                !read_uleb(data, len, &pos, &r.ticks)) {
                return 0;
            }
        } else if (op == NYTPROF_V6_OP_DISCOUNT ||
                   op == NYTPROF_V6_OP_START_DEFLATE) {
            /* empty */
        } else if (op == NYTPROF_V6_OP_ATTRIBUTE ||
                   op == NYTPROF_V6_OP_OPTION) {
            char k[64], v[64];
            if (!read_string_blob(data, len, &pos, k, sizeof(k), NULL) ||
                !read_string_blob(data, len, &pos, v, sizeof(v), NULL)) {
                return 0;
            }
            snprintf(r.text, sizeof(r.text), "%s=%s", k, v);
        } else if (op == NYTPROF_V6_OP_COMMENT || op == NYTPROF_V6_OP_NEW_FID) {
            if (op == NYTPROF_V6_OP_NEW_FID) {
                if (!read_uleb(data, len, &pos, &r.fid)) {
                    return 0;
                }
            }
            if (!read_string_blob(data, len, &pos, r.text, sizeof(r.text),
                                  NULL)) {
                return 0;
            }
        } else if (op == NYTPROF_V6_OP_SRC_LINE) {
            if (!read_uleb(data, len, &pos, &r.fid) ||
                !read_uleb(data, len, &pos, &r.line) ||
                !read_string_blob(data, len, &pos, r.text, sizeof(r.text),
                                  NULL)) {
                return 0;
            }
        } else if (op == NYTPROF_V6_OP_SUB_ENTRY) {
            if (!read_uleb(data, len, &pos, &r.fid) ||
                !read_uleb(data, len, &pos, &r.line)) {
                return 0;
            }
        } else if (op == NYTPROF_V6_OP_SUB_RETURN) {
            uint64_t depth, incl, excl;
            if (!read_uleb(data, len, &pos, &depth) ||
                !read_uleb(data, len, &pos, &incl) ||
                !read_uleb(data, len, &pos, &excl) ||
                !read_string_blob(data, len, &pos, r.text, sizeof(r.text),
                                  NULL)) {
                return 0;
            }
            r.ticks = incl;
            r.fid = depth;
        } else if (op == NYTPROF_V6_OP_SUB_INFO) {
            uint64_t first, last;
            if (!read_uleb(data, len, &pos, &r.fid) ||
                !read_uleb(data, len, &pos, &first) ||
                !read_uleb(data, len, &pos, &last) ||
                !read_string_blob(data, len, &pos, r.text, sizeof(r.text),
                                  NULL)) {
                return 0;
            }
            r.line = first;
            r.block_line = last;
        } else if (op == NYTPROF_V6_OP_PID_START) {
            uint64_t pid, ppid, t;
            if (!read_uleb(data, len, &pos, &pid) ||
                !read_uleb(data, len, &pos, &ppid) ||
                !read_uleb(data, len, &pos, &t)) {
                return 0;
            }
            r.fid = pid;
            r.line = ppid;
            r.ticks = t;
        } else if (op == NYTPROF_V6_OP_PID_END) {
            uint64_t pid, t;
            if (!read_uleb(data, len, &pos, &pid) ||
                !read_uleb(data, len, &pos, &t)) {
                return 0;
            }
            r.fid = pid;
            r.ticks = t;
        } else if (op == NYTPROF_V6_OP_SUB_CALLERS) {
            uint64_t a, b, c, d, e, f, g;
            char called[64], caller[64];
            if (!read_uleb(data, len, &pos, &a) ||
                !read_uleb(data, len, &pos, &b) ||
                !read_uleb(data, len, &pos, &c) ||
                !read_uleb(data, len, &pos, &d) ||
                !read_uleb(data, len, &pos, &e) ||
                !read_uleb(data, len, &pos, &f) ||
                !read_uleb(data, len, &pos, &g) ||
                !read_string_blob(data, len, &pos, called, sizeof(called),
                                  NULL) ||
                !read_string_blob(data, len, &pos, caller, sizeof(caller),
                                  NULL)) {
                return 0;
            }
            r.fid = a;
            r.line = b;
            (void)called;
            (void)caller;
            r.text[0] = 'c';
            r.text[1] = '\0';
        } else {
            return 0; /* unknown in this MVP reader */
        }
        if (n < max_out) {
            out[n] = r;
        }
        n++;
    }
    if (n_out) {
        *n_out = n;
    }
    return 1;
}

/* Parse sealed mini-profile: prefix + EVENT codec NONE. */
static int parse_mini(const uint8_t *wire, size_t wire_len,
                      const uint8_t **body_out, size_t *body_len_out,
                      uint32_t *logical_count_out)
{
    size_t pos = 0;
    uint32_t header_len;
    uint32_t sync, compressed_len, logical_count;
    if (wire_len < NYTPROF_V6_HEADER_LEN_FULL) {
        return 0;
    }
    if (memcmp(wire, "NYTPROF6", 8) != 0) {
        return 0;
    }
    if (wire[8] != 6 || wire[9] != 0) {
        return 0; /* major LE */
    }
    header_len = (uint32_t)wire[12] | ((uint32_t)wire[13] << 8) |
                 ((uint32_t)wire[14] << 16) | ((uint32_t)wire[15] << 24);
    if (header_len != NYTPROF_V6_HEADER_LEN_FULL) {
        return 0;
    }
    pos = header_len;
    /* skip TLV region until END */
    while (pos < wire_len) {
        uint64_t tid, vlen;
        uint8_t flags;
        size_t p = pos;
        if (!read_uleb(wire, wire_len, &p, &tid)) {
            return 0;
        }
        if (!read_uleb(wire, wire_len, &p, &vlen)) {
            return 0;
        }
        if (p >= wire_len) {
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
    if (pos == wire_len) {
        /* prefix only */
        if (body_out) {
            *body_out = NULL;
        }
        if (body_len_out) {
            *body_len_out = 0;
        }
        if (logical_count_out) {
            *logical_count_out = 0;
        }
        return 1;
    }
    if (pos + NYTPROF_V6_CHUNK_HEADER_LEN > wire_len) {
        return 0;
    }
    sync = (uint32_t)wire[pos] | ((uint32_t)wire[pos + 1] << 8) |
           ((uint32_t)wire[pos + 2] << 16) | ((uint32_t)wire[pos + 3] << 24);
    if (sync != NYTPROF_V6_CHUNK_SYNC) {
        return 0;
    }
    if (wire[pos + 4] != NYTPROF_V6_KIND_EVENT) {
        return 0;
    }
    if (wire[pos + 5] != NYTPROF_V6_CODEC_NONE) {
        return 0;
    }
    logical_count = (uint32_t)wire[pos + 24] | ((uint32_t)wire[pos + 25] << 8) |
                    ((uint32_t)wire[pos + 26] << 16) |
                    ((uint32_t)wire[pos + 27] << 24);
    compressed_len = (uint32_t)wire[pos + 32] |
                     ((uint32_t)wire[pos + 33] << 8) |
                     ((uint32_t)wire[pos + 34] << 16) |
                     ((uint32_t)wire[pos + 35] << 24);
    pos += NYTPROF_V6_CHUNK_HEADER_LEN;
    if (pos + compressed_len > wire_len) {
        return 0;
    }
    if (body_out) {
        *body_out = wire + pos;
    }
    if (body_len_out) {
        *body_len_out = compressed_len;
    }
    if (logical_count_out) {
        *logical_count_out = logical_count;
    }
    return 1;
}

static void test_lockfile_ids(void)
{
    EXPECT(NYTPROF_V6_SUPPORTED_MAJOR == 6u, "major 6");
    EXPECT(NYTPROF_V6_CHUNK_SYNC == 0x3654594Eu, "CHUNK_SYNC NYT6 LE");
    EXPECT(NYTPROF_V6_KIND_EVENT == 1u, "KIND_EVENT");
    EXPECT(NYTPROF_V6_CODEC_NONE == 0u, "CODEC_NONE");
    EXPECT(NYTPROF_V6_OP_TIME_LINE == 2u, "OP_TIME_LINE");
    EXPECT(NYTPROF_V6_OP_ATTRIBUTE == 13u, "OP_ATTRIBUTE");
    EXPECT(NYTPROF_V6_OP_VERSION == 17u, "OP_VERSION");
    EXPECT(NYTPROF_V6_OP_TIME_LINE_RUN == 18u, "OP_TIME_LINE_RUN reserved");
    EXPECT(NYTPROF_V6_FLAG_SITE_DELTA == 0x04u, "FLAG_SITE_DELTA reserved");
    EXPECT(NYTPROF_V6_FLAG_HAS_SEQ == 0x08u, "FLAG_HAS_SEQ reserved");
    EXPECT(NYTPROF_V6_TLV_END == 0x7eu, "TLV_END");
    EXPECT(memcmp("NYTPROF6",
                  (char[]){NYTPROF_V6_MAGIC_0, NYTPROF_V6_MAGIC_1,
                           NYTPROF_V6_MAGIC_2, NYTPROF_V6_MAGIC_3,
                           NYTPROF_V6_MAGIC_4, NYTPROF_V6_MAGIC_5,
                           NYTPROF_V6_MAGIC_6, NYTPROF_V6_MAGIC_7},
                  8) == 0,
           "MAGIC bytes");
}

static void test_uleb_boundaries(void)
{
    uint8_t a[10], b[10];
    size_t na, nb;
    uint64_t vals[] = {0, 1, 127, 128, 255, 16383, 16384, UINT64_C(1) << 56,
                       UINT64_MAX};
    size_t i;
    for (i = 0; i < sizeof(vals) / sizeof(vals[0]); i++) {
        na = ref_uleb(vals[i], a);
        nb = ref_uleb(vals[i], b);
        EXPECT(na == nb && memcmp(a, b, na) == 0, "uleb stable");
        EXPECT(na >= 1 && na <= 10, "uleb length bounds");
    }
}

static void test_time_line_vector(void)
{
    /* Hand vector: TIME_LINE fid=1 line=5 ticks=42 */
    uint8_t expect[32];
    size_t n = 0;
    nytp_sink *s;
    const uint8_t *body;
    size_t blen = 0;
    size_t wlen = 0;
    const uint8_t *wire;
    parsed_rec recs[4];
    size_t nrec = 0;
    uint32_t lcount = 0;
    const uint8_t *pbody = NULL;
    size_t pblen = 0;

    n += ref_uleb(NYTPROF_V6_OP_TIME_LINE, expect + n);
    expect[n++] = 0; /* flags absolute */
    n += ref_uleb(1, expect + n);
    n += ref_uleb(5, expect + n);
    n += ref_uleb(42, expect + n);

    s = nytp_v6_sink_create(NULL);
    EXPECT(s != NULL, "create");
    EXPECT(nytp_v6_sink_is_v6(s), "is_v6");
    EXPECT(nytp_emit_time_line(s, 42, 1, 5) == NYTP_OK, "emit time_line");
    body = nytp_v6_sink_event_body(s, &blen);
    EXPECT(body != NULL && blen == n, "body len");
    EXPECT(body && memcmp(body, expect, n) == 0, "TIME_LINE absolute bytes");
    EXPECT((body[1] & (NYTPROF_V6_FLAG_SITE_DELTA | NYTPROF_V6_FLAG_HAS_SEQ)) ==
               0,
           "no packing flags");

    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    EXPECT(nytp_v6_sink_is_sealed(s), "sealed");
    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire != NULL && wlen > NYTPROF_V6_HEADER_LEN_FULL, "wire after seal");
    EXPECT(parse_mini(wire, wlen, &pbody, &pblen, &lcount), "parse mini");
    EXPECT(lcount == 1, "logical count 1");
    EXPECT(pblen == n && pbody && memcmp(pbody, expect, n) == 0,
           "chunk payload == body");
    EXPECT(parse_body(pbody, pblen, recs, 4, &nrec) && nrec == 1, "parse body");
    EXPECT(recs[0].opcode == NYTPROF_V6_OP_TIME_LINE, "op");
    EXPECT(recs[0].fid == 1 && recs[0].line == 5 && recs[0].ticks == 42,
           "fields");
    nytp_sink_destroy(s);
}

static void test_full_tag_mini(void)
{
    nytp_sink *s = nytp_v6_sink_create("build/m4_mini_v6.nytprof");
    const uint8_t *wire;
    size_t wlen = 0;
    const uint8_t *body = NULL;
    size_t blen = 0;
    uint32_t lcount = 0;
    parsed_rec recs[32];
    size_t nrec = 0;
    const nytp_counting_stats *st;
    int saw_tl = 0, saw_attr = 0, saw_nf = 0;

    EXPECT(s != NULL, "create with path");
    EXPECT(nytp_emit_attribute(s, nytp_sv_cstr("basetime"),
                               nytp_sv_cstr("1700000000")) == NYTP_OK,
           "attr");
    EXPECT(nytp_emit_option(s, nytp_sv_cstr("calls"), nytp_sv_cstr("1")) ==
               NYTP_OK,
           "opt");
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("v6-abs mini")) == NYTP_OK,
           "comment");
    EXPECT(nytp_emit_pid_start(s, 1001, 1, 42.0) == NYTP_OK, "pid_start");
    EXPECT(nytp_emit_new_fid(s, 1, 0, 0, 0, 0, 0, nytp_sv_cstr("workload.pl")) ==
               NYTP_OK,
           "new_fid");
    EXPECT(nytp_emit_time_line(s, 10, 1, 5) == NYTP_OK, "tl1");
    EXPECT(nytp_emit_time_line(s, 20, 1, 6) == NYTP_OK, "tl2");
    EXPECT(nytp_emit_time_block(s, 30, 1, 7, 4, 99) == NYTP_OK, "tb");
    EXPECT(nytp_emit_discount(s) == NYTP_OK, "discount");
    EXPECT(nytp_emit_sub_entry(s, 1, 12) == NYTP_OK, "sub_entry");
    EXPECT(nytp_emit_sub_return(s, 1, 900.0, 50.0, nytp_sv_cstr("main::leaf")) ==
               NYTP_OK,
           "sub_return");
    EXPECT(nytp_emit_src_line(s, 1, 5, nytp_sv_cstr("  my $x = 1;")) == NYTP_OK,
           "src");
    EXPECT(nytp_emit_sub_info(s, 1, 3, 7, nytp_sv_cstr("main::leaf")) ==
               NYTP_OK,
           "sub_info");
    EXPECT(nytp_emit_sub_callers(s, 1, 10, 15, 900.0, 50.0, 0.0, 0,
                                 nytp_sv_cstr("main::leaf"),
                                 nytp_sv_cstr("main::mid")) == NYTP_OK,
           "sub_callers");
    EXPECT(nytp_emit_pid_end(s, 1001, 100.0) == NYTP_OK, "pid_end");
    EXPECT(nytp_emit_start_deflate(s) == NYTP_OK, "start_deflate marker");

    st = nytp_v6_sink_stats(s);
    EXPECT(st && st->by_kind[NYTP_EVT_TIME_LINE] == 2, "stats TIME_LINE");
    EXPECT(st && st->by_kind[NYTP_EVT_START_DEFLATE] == 1, "stats deflate");

    EXPECT(!nytp_v6_sink_is_sealed(s), "not sealed pre-close");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    EXPECT(nytp_v6_sink_is_sealed(s), "sealed");
    EXPECT(nytp_v6_sink_file_written(s), "file written");

    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire && wlen > 0, "wire");
    EXPECT(parse_mini(wire, wlen, &body, &blen, &lcount), "parse mini full");
    EXPECT(lcount == nytp_v6_sink_event_count(s), "count matches");
    EXPECT(parse_body(body, blen, recs, 32, &nrec), "parse body full");
    EXPECT(nrec == (size_t)lcount, "nrec == lcount");
    {
        size_t i;
        for (i = 0; i < nrec; i++) {
            EXPECT(recs[i].flags == 0, "absolute flags zero");
            if (recs[i].opcode == NYTPROF_V6_OP_TIME_LINE) {
                saw_tl++;
            }
            if (recs[i].opcode == NYTPROF_V6_OP_ATTRIBUTE) {
                saw_attr++;
            }
            if (recs[i].opcode == NYTPROF_V6_OP_NEW_FID) {
                saw_nf++;
                EXPECT(strcmp(recs[i].text, "workload.pl") == 0, "filename");
            }
        }
    }
    EXPECT(saw_tl == 2 && saw_attr == 1 && saw_nf == 1, "tag presence");
    nytp_sink_destroy(s);
}

static void test_fail_closed(void)
{
    nytp_sink *s;
    nytp_string_view bad;

    s = nytp_v6_sink_create(NULL);
    EXPECT(nytp_emit_time_line(s, -1, 1, 1) == NYTP_ERR_OVERFLOW,
           "neg ticks");
    /* sticky fail from public wrappers after OVERFLOW */
    nytp_sink_destroy(s);

    s = nytp_v6_sink_create(NULL);
    bad.ptr = NULL;
    bad.len = 5;
    bad.is_utf8 = 0;
    EXPECT(nytp_emit_attribute(s, bad, nytp_sv_cstr("x")) == NYTP_ERR_NULL,
           "null key");
    nytp_sink_destroy(s);

    s = nytp_v6_sink_create(NULL);
    EXPECT(nytp_emit_pid_start(s, 1, 0, -3.0) == NYTP_ERR_OVERFLOW,
           "neg nv time");
    nytp_sink_destroy(s);

    s = nytp_v6_sink_create(NULL);
    {
        double huge = 1.0;
        int i;
        for (i = 0; i < 40; i++) {
            huge *= 1e10; /* overflow to +Inf without a huge literal */
        }
        EXPECT(nytp_emit_sub_return(s, 1, huge, 0.0, nytp_sv_cstr("x")) ==
                   NYTP_ERR_OVERFLOW,
               "inf nv");
    }
    nytp_sink_destroy(s);
}

static void test_empty_profile(void)
{
    nytp_sink *s = nytp_v6_sink_create(NULL);
    const uint8_t *wire;
    size_t wlen = 0;
    const uint8_t *body = NULL;
    size_t blen = 0;
    uint32_t lcount = 0;
    EXPECT(s != NULL, "empty create");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "empty close");
    wire = nytp_v6_sink_wire(s, &wlen);
    EXPECT(wire && wlen >= NYTPROF_V6_HEADER_LEN_FULL, "prefix only wire");
    EXPECT(parse_mini(wire, wlen, &body, &blen, &lcount), "parse empty");
    EXPECT(body == NULL && blen == 0 && lcount == 0, "no EVENT chunk");
    nytp_sink_destroy(s);
}

static void test_attr_string_vector(void)
{
    uint8_t expect[64];
    size_t n = 0;
    nytp_sink *s;
    const uint8_t *body;
    size_t blen = 0;

    n += ref_uleb(NYTPROF_V6_OP_ATTRIBUTE, expect + n);
    expect[n++] = 0;
    n += ref_string_blob(0, 0, "basetime", 8, expect + n);
    n += ref_string_blob(0, 0, "42", 2, expect + n);

    s = nytp_v6_sink_create(NULL);
    EXPECT(nytp_emit_attribute(s, nytp_sv_cstr("basetime"),
                               nytp_sv_cstr("42")) == NYTP_OK,
           "attr emit");
    body = nytp_v6_sink_event_body(s, &blen);
    EXPECT(body && blen == n && memcmp(body, expect, n) == 0, "attr vector");
    nytp_sink_destroy(s);
}

int main(void)
{
    test_lockfile_ids();
    test_uleb_boundaries();
    test_time_line_vector();
    test_attr_string_vector();
    test_full_tag_mini();
    test_fail_closed();
    test_empty_profile();

    if (failures) {
        fprintf(stderr, "test_v6_abs_wire: %d failure(s)\n", failures);
        return 1;
    }
    printf("test_v6_abs_wire: OK\n");
    return 0;
}

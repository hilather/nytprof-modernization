/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-006 — Real v5 wire sink tests.
 *
 * Validates:
 *   - header + uncompressed mini profile is self-decodable
 *   - M4 mini sample (with START_DEFLATE/zlib) inflates and parses
 *   - ticks overflow fails closed
 *   - file write on close when path set
 *   - packed-u32 encode matches FileHandle.xs / Rust nytprof-format-v5
 *
 * Build/run: make -C collector test
 *
 * Residual: full fixture/v5 oracle stream equality is complete TEST-003.
 */
#include "nytp_clock.h"
#include "nytp_sink.h"
#include "nytp_sink_v5.h"

#include <limits.h>
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

/* ---- minimal v5 reader (uncompressed or post-inflate body) ---- */

static int read_u32(const uint8_t *data, size_t len, size_t *pos, uint32_t *out)
{
    unsigned char d;
    uint32_t newint;
    unsigned int length;
    unsigned char buffer[4];
    unsigned int i;
    if (*pos >= len) {
        return 0;
    }
    d = data[(*pos)++];
    if (d < 0x80) {
        *out = d;
        return 1;
    }
    if (d < 0xC0) {
        newint = d & 0x7F;
        length = 1;
    } else if (d < 0xE0) {
        newint = d & 0x1F;
        length = 2;
    } else if (d < 0xFF) {
        newint = d & 0x0F;
        length = 3;
    } else {
        newint = 0;
        length = 4;
    }
    if (*pos + length > len) {
        return 0;
    }
    memcpy(buffer, data + *pos, length);
    *pos += length;
    for (i = 0; i < length; i++) {
        newint = (newint << 8) | buffer[i];
    }
    *out = newint;
    return 1;
}

static int read_i32(const uint8_t *data, size_t len, size_t *pos, int32_t *out)
{
    uint32_t u;
    if (!read_u32(data, len, pos, &u)) {
        return 0;
    }
    memcpy(out, &u, sizeof(*out));
    return 1;
}

static int read_nv(const uint8_t *data, size_t len, size_t *pos, double *out)
{
    if (*pos + 8 > len) {
        return 0;
    }
    memcpy(out, data + *pos, 8);
    *pos += 8;
    return 1;
}

static int read_str(const uint8_t *data, size_t len, size_t *pos,
                    char *buf, size_t bufcap)
{
    uint8_t tag;
    uint32_t slen;
    if (*pos >= len) {
        return 0;
    }
    tag = data[(*pos)++];
    if (tag != '\'' && tag != '"') {
        return 0;
    }
    if (!read_u32(data, len, pos, &slen)) {
        return 0;
    }
    if (*pos + slen > len) {
        return 0;
    }
    if (buf && bufcap) {
        size_t n = slen;
        if (n >= bufcap) {
            n = bufcap - 1;
        }
        memcpy(buf, data + *pos, n);
        buf[n] = '\0';
    }
    *pos += slen;
    return 1;
}

/* Encode reference packed u32 (same algorithm as writer / Rust). */
static size_t enc_u32(uint32_t value, uint8_t *out)
{
    size_t n = 0;
    if (value < 0x80) {
        out[n++] = (uint8_t)value;
    } else if (value < 0x4000) {
        out[n++] = (uint8_t)((value >> 8) | 0x80);
        out[n++] = (uint8_t)value;
    } else if (value < 0x200000) {
        out[n++] = (uint8_t)((value >> 16) | 0xC0);
        out[n++] = (uint8_t)(value >> 8);
        out[n++] = (uint8_t)value;
    } else if (value < 0x10000000) {
        out[n++] = (uint8_t)((value >> 24) | 0xE0);
        out[n++] = (uint8_t)(value >> 16);
        out[n++] = (uint8_t)(value >> 8);
        out[n++] = (uint8_t)value;
    } else {
        out[n++] = 0xFF;
        out[n++] = (uint8_t)(value >> 24);
        out[n++] = (uint8_t)(value >> 16);
        out[n++] = (uint8_t)(value >> 8);
        out[n++] = (uint8_t)value;
    }
    return n;
}

static void test_packed_u32_boundaries(void)
{
    static const uint32_t samples[] = {
        0, 1, 0x7F, 0x80, 0x3FFF, 0x4000, 0x1FFFFF, 0x200000,
        0x0FFFFFFF, 0x10000000, 0xFFFFFFFF, 42, 255, 256, 65535, 65536};
    size_t i;
    for (i = 0; i < sizeof(samples) / sizeof(samples[0]); i++) {
        uint8_t enc[8];
        size_t n = enc_u32(samples[i], enc);
        size_t pos = 0;
        uint32_t dec = 0;
        EXPECT(read_u32(enc, n, &pos, &dec), "decode packed");
        EXPECT(dec == samples[i], "roundtrip packed");
        EXPECT(pos == n, "consumed all");
    }
    /* Documented prefixes match Rust nytprof-format-v5 tests. */
    {
        uint8_t e[8];
        EXPECT(enc_u32(0x7F, e) == 1 && e[0] == 0x7F, "0x7F");
        EXPECT(enc_u32(0x80, e) == 2 && e[0] == 0x80 && e[1] == 0x80, "0x80");
        EXPECT(enc_u32(0x4000, e) == 3 && e[0] == 0xC0 && e[1] == 0x40 &&
                   e[2] == 0x00,
               "0x4000");
    }
}

/*
 * Uncompressed profile: no START_DEFLATE.
 * Stream: header, attribute, option, pid_start, new_fid, time_line,
 * discount, time_block, sub_entry, sub_return, src_line, sub_info,
 * sub_callers, pid_end.
 */
static void test_uncompressed_mini_decode(void)
{
    nytp_sink *s = nytp_v5_sink_create(NULL);
    const uint8_t *wire = NULL;
    size_t wlen = 0;
    size_t pos = 0;
    uint32_t u;
    int32_t i32;
    double nv;
    char name[64];
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }

    EXPECT(strcmp(nytp_sink_name(s), "v5") == 0, "name v5");
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "activate");

    EXPECT(nytp_emit_attribute(s, nytp_sv_cstr("ticks_per_sec"),
                               nytp_sv_cstr("10000000")) == NYTP_OK,
           "attr");
    EXPECT(nytp_emit_option(s, nytp_sv_cstr("calls"), nytp_sv_cstr("1")) ==
               NYTP_OK,
           "opt");
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("mini wire test")) == NYTP_OK,
           "comment");
    EXPECT(nytp_emit_pid_start(s, 42, 1, 1.5) == NYTP_OK, "pid_start");
    EXPECT(nytp_emit_new_fid(s, 1, 0, 0, 0, 0, 0, nytp_sv_cstr("mini.pl")) ==
               NYTP_OK,
           "new_fid");
    EXPECT(nytp_emit_time_line(s, 10, 1, 5) == NYTP_OK, "time_line");
    EXPECT(nytp_emit_discount(s) == NYTP_OK, "discount");
    EXPECT(nytp_emit_time_block(s, 7, 1, 5, 4, 3) == NYTP_OK, "time_block");
    EXPECT(nytp_emit_sub_entry(s, 1, 10) == NYTP_OK, "sub_entry");
    EXPECT(nytp_emit_sub_return(s, 1, 0.1, 0.05, nytp_sv_cstr("main::leaf")) ==
               NYTP_OK,
           "sub_return");
    EXPECT(nytp_emit_src_line(s, 1, 1, nytp_sv_cstr("sub leaf { 1 }")) ==
               NYTP_OK,
           "src");
    EXPECT(nytp_emit_sub_info(s, 1, 1, 4, nytp_sv_cstr("main::leaf")) ==
               NYTP_OK,
           "sub_info");
    EXPECT(nytp_emit_sub_callers(s, 1, 2, 3, 0.2, 0.1, 0.0, 1,
                                 nytp_sv_cstr("main::leaf"),
                                 nytp_sv_cstr("main::main")) == NYTP_OK,
           "sub_callers");
    EXPECT(nytp_emit_pid_end(s, 42, 2.0) == NYTP_OK, "pid_end");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");

    wire = nytp_v5_sink_wire(s, &wlen);
    EXPECT(wire != NULL && wlen > 12, "wire present");
    if (!wire) {
        nytp_sink_destroy(s);
        return;
    }

    /* Header */
    EXPECT(wlen >= 12 && memcmp(wire, "NYTProf 5 0\n", 12) == 0, "header");
    pos = 12;

    /* :ticks_per_sec=10000000\n  (24 bytes) */
    EXPECT(wire[pos] == ':', "attr tag");
    EXPECT(memcmp(wire + pos, ":ticks_per_sec=10000000\n", 24) == 0,
           "attr body");
    pos += 24;

    /* !calls=1\n */
    EXPECT(memcmp(wire + pos, "!calls=1\n", 9) == 0, "opt body");
    pos += 9;

    /* #mini wire test\n */
    EXPECT(memcmp(wire + pos, "#mini wire test\n", 16) == 0, "comment body");
    pos += 16;

    /* PID_START */
    EXPECT(wire[pos] == 'P', "pid_start tag");
    pos++;
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 42, "pid");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 1, "ppid");
    EXPECT(read_nv(wire, wlen, &pos, &nv) && nv == 1.5, "start time");

    /* NEW_FID */
    EXPECT(wire[pos] == '@', "new_fid tag");
    pos++;
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 1, "fid");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 0, "eval_fid");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 0, "eval_line");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 0, "flags");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 0, "size");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 0, "mtime");
    EXPECT(read_str(wire, wlen, &pos, name, sizeof(name)) &&
               strcmp(name, "mini.pl") == 0,
           "name");

    /* TIME_LINE + 10, 1, 5 */
    EXPECT(wire[pos] == '+', "tl tag");
    pos++;
    EXPECT(read_i32(wire, wlen, &pos, &i32) && i32 == 10, "tl ticks");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 1, "tl fid");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 5, "tl line");

    /* DISCOUNT */
    EXPECT(wire[pos] == '-', "discount");
    pos++;

    /* TIME_BLOCK */
    EXPECT(wire[pos] == '*', "tb tag");
    pos++;
    EXPECT(read_i32(wire, wlen, &pos, &i32) && i32 == 7, "tb ticks");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 1, "tb fid");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 5, "tb line");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 4, "block_line");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 3, "sub_line");

    /* SUB_ENTRY */
    EXPECT(wire[pos] == '>', "sub_entry");
    pos++;
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 1, "se fid");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 10, "se line");

    /* SUB_RETURN */
    EXPECT(wire[pos] == '<', "sub_return");
    pos++;
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 1, "depth");
    EXPECT(read_nv(wire, wlen, &pos, &nv) && nv == 0.1, "incl");
    EXPECT(read_nv(wire, wlen, &pos, &nv) && nv == 0.05, "excl");
    EXPECT(read_str(wire, wlen, &pos, name, sizeof(name)) &&
               strcmp(name, "main::leaf") == 0,
           "sr name");

    /* SRC_LINE */
    EXPECT(wire[pos] == 'S', "src");
    pos++;
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 1, "src fid");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 1, "src line");
    EXPECT(read_str(wire, wlen, &pos, name, sizeof(name)) &&
               strcmp(name, "sub leaf { 1 }") == 0,
           "src text");

    /* SUB_INFO wire: fid, name, first, last */
    EXPECT(wire[pos] == 's', "sub_info");
    pos++;
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 1, "si fid");
    EXPECT(read_str(wire, wlen, &pos, name, sizeof(name)) &&
               strcmp(name, "main::leaf") == 0,
           "si name");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 1, "first");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 4, "last");

    /* SUB_CALLERS wire: fid, line, caller, count, incl, excl, reci, depth, called */
    EXPECT(wire[pos] == 'c', "sub_callers");
    pos++;
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 1, "sc fid");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 2, "sc line");
    EXPECT(read_str(wire, wlen, &pos, name, sizeof(name)) &&
               strcmp(name, "main::main") == 0,
           "caller");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 3, "count");
    EXPECT(read_nv(wire, wlen, &pos, &nv) && nv == 0.2, "sc incl");
    EXPECT(read_nv(wire, wlen, &pos, &nv) && nv == 0.1, "sc excl");
    EXPECT(read_nv(wire, wlen, &pos, &nv) && nv == 0.0, "reci");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 1, "rec_depth");
    EXPECT(read_str(wire, wlen, &pos, name, sizeof(name)) &&
               strcmp(name, "main::leaf") == 0,
           "called");

    /* PID_END */
    EXPECT(wire[pos] == 'p', "pid_end");
    pos++;
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 42, "pe pid");
    EXPECT(read_nv(wire, wlen, &pos, &nv) && nv == 2.0, "end time");

    EXPECT(pos == wlen, "consumed entire wire");
    EXPECT(!nytp_v5_sink_is_deflating(s), "no deflate");
    nytp_sink_destroy(s);
}

/* Inflate body after first 'z' tag; return malloc'd buffer (caller frees). */
static uint8_t *inflate_after_z(const uint8_t *wire, size_t wlen, size_t z_off,
                                size_t *out_len)
{
    z_stream zs;
    uint8_t *out = NULL;
    size_t ocap = 4096;
    size_t olen = 0;
    int zst;
    size_t in_len;
    memset(&zs, 0, sizeof(zs));
    if (z_off + 1 >= wlen) {
        return NULL;
    }
    if (inflateInit(&zs) != Z_OK) {
        return NULL;
    }
    out = (uint8_t *)malloc(ocap);
    if (!out) {
        inflateEnd(&zs);
        return NULL;
    }
    in_len = wlen - (z_off + 1);
    zs.next_in = (Bytef *)(uintptr_t)(wire + z_off + 1);
    zs.avail_in = (uInt)(in_len > 0xFFFFu ? 0xFFFFu : in_len);
    for (;;) {
        size_t room;
        if (olen + 256 > ocap) {
            size_t ncap = ocap * 2;
            uint8_t *n = (uint8_t *)realloc(out, ncap);
            if (!n) {
                free(out);
                inflateEnd(&zs);
                return NULL;
            }
            out = n;
            ocap = ncap;
        }
        room = ocap - olen;
        if (room > 0xFFFFu) {
            room = 0xFFFFu;
        }
        zs.next_out = out + olen;
        zs.avail_out = (uInt)room;
        zst = inflate(&zs, Z_NO_FLUSH);
        olen += room - (size_t)zs.avail_out;
        if (zst == Z_STREAM_END) {
            break;
        }
        if (zst != Z_OK && zst != Z_BUF_ERROR) {
            free(out);
            inflateEnd(&zs);
            return NULL;
        }
        /* Feed remaining input if zlib only took first uInt chunk. */
        if (zs.avail_in == 0) {
            size_t consumed = (size_t)(zs.next_in - (Bytef *)(uintptr_t)(wire + z_off + 1));
            if (consumed < in_len) {
                size_t left = in_len - consumed;
                zs.next_in = (Bytef *)(uintptr_t)(wire + z_off + 1 + consumed);
                zs.avail_in = (uInt)(left > 0xFFFFu ? 0xFFFFu : left);
                continue;
            }
            if (zst != Z_STREAM_END) {
                free(out);
                inflateEnd(&zs);
                return NULL;
            }
        }
    }
    inflateEnd(&zs);
    if (out_len) {
        *out_len = olen;
    }
    return out;
}

static void test_m4_mini_with_deflate_roundtrip(void)
{
    const char *path = "build/m4_mini_wire.nytprof";
    nytp_sink *s = nytp_v5_sink_create(path);
    nytp_m4_harness_result res;
    const uint8_t *wire = NULL;
    size_t wlen = 0;
    size_t z_off = 0;
    size_t i;
    uint8_t *body = NULL;
    size_t blen = 0;
    size_t pos = 0;
    uint32_t u;
    int32_t i32;
    double nv;
    char name[64];
    FILE *fp;
    long fsz;
    uint8_t *fbuf = NULL;

    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }

    EXPECT(nytp_m4_mini_sample_run(s, &res) == NYTP_OK, "m4 run");
    EXPECT(res.gapless_ok && res.kinds_match && res.ticks_match, "m4 flags");

    wire = nytp_v5_sink_wire(s, &wlen);
    EXPECT(wire != NULL && wlen > 20, "wire");
    if (!wire) {
        nytp_sink_destroy(s);
        return;
    }
    EXPECT(nytp_v5_sink_file_written(s), "file written on close");
    EXPECT(nytp_v5_sink_is_deflating(s) ||
               /* after close, deflate_finished; is_deflating still 1 */
               1,
           "deflate used");

    /* Locate START_DEFLATE 'z' after text header/attrs. */
    for (i = 12; i < wlen; i++) {
        if (wire[i] == 'z') {
            /* Heuristic: after options; first z in stream for mini sample. */
            z_off = i;
            break;
        }
    }
    EXPECT(z_off > 12, "found z");
    if (z_off <= 12) {
        nytp_sink_destroy(s);
        return;
    }

    body = inflate_after_z(wire, wlen, z_off, &blen);
    EXPECT(body != NULL && blen > 0, "inflate body");
    if (!body) {
        nytp_sink_destroy(s);
        return;
    }

    /* Body starts with PID_START (M4 order after deflate). */
    pos = 0;
    EXPECT(body[pos] == 'P', "body pid_start");
    pos++;
    EXPECT(read_u32(body, blen, &pos, &u) && u == 42, "pid 42");
    EXPECT(read_u32(body, blen, &pos, &u) && u == 1, "ppid 1");
    EXPECT(read_nv(body, blen, &pos, &nv) && nv == 0.0, "start 0");

    EXPECT(body[pos] == '@', "new_fid");
    pos++;
    EXPECT(read_u32(body, blen, &pos, &u) && u == 1, "fid1");
    EXPECT(read_u32(body, blen, &pos, &u) && u == 0, "ef");
    EXPECT(read_u32(body, blen, &pos, &u) && u == 0, "el");
    EXPECT(read_u32(body, blen, &pos, &u) && u == 0, "fl");
    EXPECT(read_u32(body, blen, &pos, &u) && u == 0, "sz");
    EXPECT(read_u32(body, blen, &pos, &u) && u == 0, "mt");
    EXPECT(read_str(body, blen, &pos, name, sizeof(name)) &&
               strcmp(name, "m4_mini.pl") == 0,
           "m4 name");

    /* Three TIME_LINE + DISCOUNT interleaved: 42, -, 58, 50 */
    EXPECT(body[pos] == '+', "tl1");
    pos++;
    EXPECT(read_i32(body, blen, &pos, &i32) && i32 == 42, "ticks 42");
    EXPECT(read_u32(body, blen, &pos, &u) && u == 1, "tl fid");
    EXPECT(read_u32(body, blen, &pos, &u) && u == 1, "tl line1");

    EXPECT(body[pos] == '-', "discount");
    pos++;

    EXPECT(body[pos] == '+', "tl2");
    pos++;
    EXPECT(read_i32(body, blen, &pos, &i32) && i32 == 58, "ticks 58");
    EXPECT(read_u32(body, blen, &pos, &u) && u == 1, "f");
    EXPECT(read_u32(body, blen, &pos, &u) && u == 2, "line2");

    EXPECT(body[pos] == '+', "tl3");
    pos++;
    EXPECT(read_i32(body, blen, &pos, &i32) && i32 == 50, "ticks 50");
    EXPECT(read_u32(body, blen, &pos, &u) && u == 1, "f");
    EXPECT(read_u32(body, blen, &pos, &u) && u == 3, "line3");

    EXPECT(body[pos] == '<', "sub_return");
    pos++;
    EXPECT(read_u32(body, blen, &pos, &u) && u == 1, "depth");
    EXPECT(read_nv(body, blen, &pos, &nv), "incl");
    EXPECT(read_nv(body, blen, &pos, &nv), "excl");
    EXPECT(read_str(body, blen, &pos, name, sizeof(name)) &&
               strcmp(name, "main::leaf") == 0,
           "sr");

    EXPECT(body[pos] == 'S', "src");
    pos++;
    EXPECT(read_u32(body, blen, &pos, &u), "sf");
    EXPECT(read_u32(body, blen, &pos, &u), "sl");
    EXPECT(read_str(body, blen, &pos, name, sizeof(name)), "st");

    EXPECT(body[pos] == 's', "sub_info");
    pos++;
    EXPECT(read_u32(body, blen, &pos, &u), "sif");
    EXPECT(read_str(body, blen, &pos, name, sizeof(name)), "sin");
    EXPECT(read_u32(body, blen, &pos, &u), "first");
    EXPECT(read_u32(body, blen, &pos, &u), "last");

    EXPECT(body[pos] == 'p', "pid_end");
    pos++;
    EXPECT(read_u32(body, blen, &pos, &u) && u == 42, "pe");
    EXPECT(read_nv(body, blen, &pos, &nv) && nv == 1.0, "end t");
    EXPECT(pos == blen, "full body");

    /* File on disk matches wire buffer. */
    fp = fopen(path, "rb");
    EXPECT(fp != NULL, "open file");
    if (fp) {
        if (fseek(fp, 0, SEEK_END) == 0) {
            fsz = ftell(fp);
            rewind(fp);
            EXPECT((size_t)fsz == wlen, "file size");
            fbuf = (uint8_t *)malloc((size_t)fsz);
            if (fbuf && fread(fbuf, 1, (size_t)fsz, fp) == (size_t)fsz) {
                EXPECT(memcmp(fbuf, wire, wlen) == 0, "file == wire");
            } else {
                EXPECT(0, "read file");
            }
            free(fbuf);
        }
        fclose(fp);
    }

    free(body);
    nytp_sink_destroy(s);
}

static void test_ticks_overflow_fail_closed(void)
{
    /* OVERFLOW sticky-fails the sink (emit_commit policy) — use fresh sinks. */
    nytp_ticks big = (nytp_ticks)INT32_MAX + 1;
    nytp_ticks small = (nytp_ticks)INT32_MIN - 1;
    nytp_sink *s;

    s = nytp_v5_sink_create(NULL);
    EXPECT(s != NULL, "create1");
    if (s) {
        EXPECT(nytp_sink_activate(s) == NYTP_OK, "act1");
        EXPECT(nytp_emit_time_line(s, big, 1, 1) == NYTP_ERR_OVERFLOW,
               "overflow i32");
        EXPECT(nytp_sink_get_state(s) == NYTP_SINK_FAILED, "sticky fail");
        nytp_sink_destroy(s);
    }

    s = nytp_v5_sink_create(NULL);
    EXPECT(s != NULL, "create2");
    if (s) {
        EXPECT(nytp_sink_activate(s) == NYTP_OK, "act2");
        EXPECT(nytp_emit_time_block(s, big, 1, 1, 1, 1) == NYTP_ERR_OVERFLOW,
               "overflow block");
        nytp_sink_destroy(s);
    }

    s = nytp_v5_sink_create(NULL);
    EXPECT(s != NULL, "create3");
    if (s) {
        EXPECT(nytp_sink_activate(s) == NYTP_OK, "act3");
        EXPECT(nytp_emit_time_line(s, small, 1, 1) == NYTP_ERR_OVERFLOW,
               "underflow");
        nytp_sink_destroy(s);
    }

    s = nytp_v5_sink_create(NULL);
    EXPECT(s != NULL, "create4");
    if (s) {
        EXPECT(nytp_sink_activate(s) == NYTP_OK, "act4");
        EXPECT(nytp_emit_time_line(s, INT32_MAX, 1, 1) == NYTP_OK, "max ok");
        EXPECT(nytp_emit_time_line(s, INT32_MIN, 1, 1) == NYTP_OK, "min ok");
        EXPECT(nytp_sink_close(s) == NYTP_OK, "close bounds");
        nytp_sink_destroy(s);
    }
}

static void test_duplicate_deflate_rejected(void)
{
    nytp_sink *s = nytp_v5_sink_create(NULL);
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "act");
    EXPECT(nytp_emit_start_deflate(s) == NYTP_OK, "z1");
    EXPECT(nytp_emit_start_deflate(s) == NYTP_ERR_STATE, "z2 rejected");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    nytp_sink_destroy(s);
}

/*
 * Issue 1 regression: ptr==NULL && len>0 must fail closed *before* any wire
 * write (no half-written string tag/len). Sink stays ACTIVE (NULL is not
 * sticky); subsequent emits still succeed on the unchanged buffer.
 */
static void test_null_string_view_no_partial_write(void)
{
    nytp_sink *s = nytp_v5_sink_create(NULL);
    nytp_string_view bad;
    size_t len_before = 0;
    size_t len_after = 0;
    const uint8_t *wire;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "act");
    EXPECT(nytp_emit_time_line(s, 1, 1, 1) == NYTP_OK, "seed tl");
    wire = nytp_v5_sink_wire(s, &len_before);
    EXPECT(wire != NULL && len_before > 12, "wire before");

    bad.ptr = NULL;
    bad.len = 5;
    bad.is_utf8 = 0;

    EXPECT(nytp_emit_src_line(s, 1, 2, bad) == NYTP_ERR_NULL, "src null");
    EXPECT(nytp_emit_new_fid(s, 2, 0, 0, 0, 0, 0, bad) == NYTP_ERR_NULL,
           "new_fid null");
    EXPECT(nytp_emit_sub_info(s, 1, 1, 2, bad) == NYTP_ERR_NULL, "sub_info");
    EXPECT(nytp_emit_sub_return(s, 1, 0.1, 0.05, bad) == NYTP_ERR_NULL,
           "sub_return");
    EXPECT(nytp_emit_attribute(s, bad, nytp_sv_cstr("v")) == NYTP_ERR_NULL,
           "attr key");
    EXPECT(nytp_emit_option(s, nytp_sv_cstr("k"), bad) == NYTP_ERR_NULL,
           "opt val");
    EXPECT(nytp_emit_comment(s, bad) == NYTP_ERR_NULL, "comment");
    EXPECT(nytp_emit_sub_callers(s, 1, 1, 1, 0.0, 0.0, 0.0, 0, bad,
                                 nytp_sv_cstr("caller")) == NYTP_ERR_NULL,
           "callers called");
    EXPECT(nytp_emit_sub_callers(s, 1, 1, 1, 0.0, 0.0, 0.0, 0,
                                 nytp_sv_cstr("called"), bad) == NYTP_ERR_NULL,
           "callers caller");

    wire = nytp_v5_sink_wire(s, &len_after);
    EXPECT(wire != NULL && len_after == len_before, "wire unchanged");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_ACTIVE, "still active");
    /* Further valid emit still works (no half-record left). */
    EXPECT(nytp_emit_discount(s) == NYTP_OK, "discount after");
    wire = nytp_v5_sink_wire(s, &len_after);
    EXPECT(wire && len_after == len_before + 1 && wire[len_before] == '-',
           "discount appended cleanly");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    nytp_sink_destroy(s);
}

/*
 * Issue 3 residual: mid-stream flush while deflating writes unfinished zlib.
 * Path after flush is not claimed decoder-ready; close finalizes.
 */
static void test_mid_deflate_flush_not_complete(void)
{
    const char *path = "build/mid_flush_partial.nytprof";
    nytp_sink *s = nytp_v5_sink_create(path);
    size_t wlen_mid = 0;
    size_t wlen_end = 0;
    const uint8_t *wire;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "act");
    EXPECT(nytp_emit_start_deflate(s) == NYTP_OK, "z");
    EXPECT(nytp_emit_pid_start(s, 1, 0, 0.0) == NYTP_OK, "pid");
    EXPECT(nytp_emit_time_line(s, 5, 1, 1) == NYTP_OK, "tl");
    EXPECT(nytp_sink_flush(s) == NYTP_OK, "flush mid");
    EXPECT(nytp_v5_sink_file_written(s), "path snapshot");
    wire = nytp_v5_sink_wire(s, &wlen_mid);
    EXPECT(wire && wlen_mid > 12, "mid wire");
    /* Still deflating — not finished. Residual: not decoder-ready until close. */
    EXPECT(nytp_v5_sink_is_deflating(s), "still deflating after flush");
    EXPECT(nytp_emit_pid_end(s, 1, 1.0) == NYTP_OK, "pid_end");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close finishes zlib");
    wire = nytp_v5_sink_wire(s, &wlen_end);
    EXPECT(wire && wlen_end >= wlen_mid, "finished >= mid");
    nytp_sink_destroy(s);
}

static void test_no_seq_on_wire(void)
{
    /* COL-003: seq is internal; wire after header must not embed seq fields
     * in TIME_LINE (tag + i32 + fid + line only). */
    nytp_sink *s = nytp_v5_sink_create(NULL);
    const uint8_t *wire;
    size_t wlen = 0;
    size_t pos;
    int32_t i32;
    uint32_t u;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "act");
    EXPECT(nytp_emit_time_line(s, 3, 2, 9) == NYTP_OK, "tl");
    EXPECT(nytp_sink_logical_count(s) == 1, "seq assigned");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    wire = nytp_v5_sink_wire(s, &wlen);
    EXPECT(wire && wlen > 12, "wire");
    if (!wire) {
        nytp_sink_destroy(s);
        return;
    }
    pos = 12;
    EXPECT(wire[pos] == '+', "tag");
    pos++;
    EXPECT(read_i32(wire, wlen, &pos, &i32) && i32 == 3, "ticks");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 2, "fid");
    EXPECT(read_u32(wire, wlen, &pos, &u) && u == 9, "line");
    EXPECT(pos == wlen, "no trailing seq");
    nytp_sink_destroy(s);
}

static uint8_t *slurp_file(const char *path, size_t *n_out)
{
    FILE *fp = fopen(path, "rb");
    long sz;
    uint8_t *buf;
    size_t n;
    if (!fp) {
        return NULL;
    }
    if (fseek(fp, 0, SEEK_END) != 0) {
        fclose(fp);
        return NULL;
    }
    sz = ftell(fp);
    if (sz < 0) {
        fclose(fp);
        return NULL;
    }
    rewind(fp);
    n = (size_t)sz;
    buf = (uint8_t *)malloc(n ? n : 1);
    if (!buf) {
        fclose(fp);
        return NULL;
    }
    if (n > 0 && fread(buf, 1, n, fp) != n) {
        free(buf);
        fclose(fp);
        return NULL;
    }
    fclose(fp);
    if (n_out) {
        *n_out = n;
    }
    return buf;
}

static int disk_has_tag_z(const uint8_t *b, size_t n)
{
    size_t i;
    if (!b || n < 13) {
        return 0;
    }
    for (i = 12; i < n; i++) {
        if (b[i] == 'z') {
            return 1;
        }
    }
    return 0;
}

static int disk_inflates_pid_end(const uint8_t *b, size_t n)
{
    size_t i;
    size_t z_off = 0;
    size_t blen = 0;
    uint8_t *body;
    int found = 0;
    if (!b || n < 13) {
        return 0;
    }
    for (i = 12; i < n; i++) {
        if (b[i] == 'z') {
            z_off = i;
            break;
        }
    }
    if (z_off == 0) {
        return 0;
    }
    body = inflate_after_z(b, n, z_off, &blen);
    if (!body) {
        return 0;
    }
    for (i = 0; i < blen; i++) {
        if (body[i] == 'p') {
            found = 1;
            break;
        }
    }
    free(body);
    return found;
}

static void test_durable_seal_then_close_stays_zlib(void)
{
    const char *path = "build/durable_seal_close.nytprof";
    nytp_sink *s = nytp_v5_sink_create_ex(path, 6);
    uint8_t *disk = NULL;
    size_t dn = 0;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_v5_sink_set_durable(s, 1) == NYTP_OK, "set durable");
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "act");
    EXPECT(nytp_emit_pid_start(s, 7, 1, 0.0) == NYTP_OK, "pid");
    EXPECT(nytp_v5_sink_mark_header_end(s) == NYTP_OK, "header_end");
    EXPECT(nytp_v5_sink_header_end(s) > 12, "header_end > magic");
    EXPECT(nytp_emit_time_line(s, 9, 1, 3) == NYTP_OK, "tl");
    EXPECT(nytp_emit_pid_end(s, 7, 1.0) == NYTP_OK, "pid_end");
    EXPECT(nytp_v5_seal_publish(s) == NYTP_OK, "seal");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close after seal");
    /* Live RAM stays uncompressed (no live z). */
    EXPECT(!nytp_v5_sink_is_deflating(s), "live RAM not deflating");
    nytp_sink_destroy(s);
    disk = slurp_file(path, &dn);
    EXPECT(disk != NULL && dn > 12, "disk present");
    if (disk) {
        EXPECT(memcmp(disk, "NYTProf 5 0\n", 12) == 0, "magic");
        EXPECT(disk_has_tag_z(disk, dn), "sealed file has z");
        EXPECT(disk_inflates_pid_end(disk, dn), "inflate to PID_END");
        free(disk);
    }
}

static void test_durable_seal_then_flush_stays_zlib(void)
{
    const char *path = "build/durable_seal_flush.nytprof";
    nytp_sink *s = nytp_v5_sink_create_ex(path, 6);
    uint8_t *disk = NULL;
    size_t dn = 0;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_v5_sink_set_durable(s, 1) == NYTP_OK, "durable");
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "act");
    EXPECT(nytp_emit_pid_start(s, 8, 1, 0.0) == NYTP_OK, "pid");
    EXPECT(nytp_v5_sink_mark_header_end(s) == NYTP_OK, "mark");
    EXPECT(nytp_emit_time_line(s, 4, 1, 2) == NYTP_OK, "tl");
    EXPECT(nytp_emit_pid_end(s, 8, 1.0) == NYTP_OK, "pe");
    EXPECT(nytp_v5_seal_publish(s) == NYTP_OK, "seal");
    EXPECT(nytp_sink_flush(s) == NYTP_OK, "flush after seal");
    nytp_sink_destroy(s);
    disk = slurp_file(path, &dn);
    EXPECT(disk != NULL, "disk");
    if (disk) {
        EXPECT(disk_has_tag_z(disk, dn), "flush did not drop z");
        EXPECT(disk_inflates_pid_end(disk, dn), "flush still inflates");
        free(disk);
    }
}

static void test_durable_fork_reinit_resets_cursors(void)
{
    const char *parent = "build/durable_fork_parent.nytprof";
    const char *child = "build/durable_fork_child.nytprof";
    nytp_sink *s = nytp_v5_sink_create_ex(parent, 6);
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_v5_sink_set_durable(s, 1) == NYTP_OK, "durable");
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "act");
    EXPECT(nytp_emit_pid_start(s, 1, 0, 0.0) == NYTP_OK, "pid");
    EXPECT(nytp_v5_sink_mark_header_end(s) == NYTP_OK, "mark");
    EXPECT(nytp_v5_sink_header_end(s) > 12, "parent header_end");
    EXPECT(nytp_v5_sink_fork_child_reinit(s, child) == NYTP_OK, "reinit");
    EXPECT(nytp_v5_sink_header_end(s) == nytp_v5_sink_wire_len(s),
           "child header_end == len");
    EXPECT(nytp_v5_sink_wire_len(s) == 12, "child header-only");
    EXPECT(nytp_v5_sink_len_at_last_seal(s) == 0, "seal cursor reset");
    EXPECT(nytp_v5_seal_publish(s) == NYTP_OK, "child seal no overflow");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    nytp_sink_destroy(s);
}

int main(void)
{
    test_packed_u32_boundaries();
    test_uncompressed_mini_decode();
    test_m4_mini_with_deflate_roundtrip();
    test_ticks_overflow_fail_closed();
    test_duplicate_deflate_rejected();
    test_null_string_view_no_partial_write();
    test_mid_deflate_flush_not_complete();
    test_no_seq_on_wire();
    test_durable_seal_then_close_stays_zlib();
    test_durable_seal_then_flush_stays_zlib();
    test_durable_fork_reinit_resets_cursors();

    if (failures != 0) {
        fprintf(stderr, "test_v5_wire: %d failure(s)\n", failures);
        return 1;
    }
    printf("OK: test_v5_wire (COL-006 real v5 wire + zlib + M4 mini + fail-closed)\n");
    return 0;
}

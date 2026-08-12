/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * PR-B09 / COL-007 E3-C — generate C-produced v6 EVENT fixtures for product E3.
 *
 * Writes deterministic profiles under <outdir>/ (default:
 * fixtures/v6/from-c relative to CWD). Matrix:
 *   absolute.nytprof          — absolute EVENT (no packing / no FOOTER dict)
 *   packing.nytprof           — ADR-0001 packing multi-chunk (max 2, ZLIB)
 *   packing_lz4.nytprof       — packing multi-chunk (max 2, LZ4)
 *   dict.nytprof              — ADR-0002 FOOTER string-dict (absolute bodies)
 *   packing_dict.nytprof      — packing + FOOTER dict multi-chunk
 *   mid_stream.nytprof        — packing + mid-stream NONE→ZLIB region
 *   mid_stream_dict.nytprof   — packing + mid-stream + FOOTER dict
 *
 * Usage:
 *   make -C collector gen-e3-fixtures OUTDIR=../fixtures/v6/from-c
 *   ./collector/build/gen_e3_c_fixtures [outdir]
 *
 * Product E3: fixtures must be C-produced only (never Rust stand-in).
 */
#include "nytp_sink.h"
#include "nytp_sink_v6.h"
#include "nytprof_v6_ids.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <errno.h>

static int failures = 0;

#define EXPECT(cond, msg)                                                      \
    do {                                                                       \
        if (!(cond)) {                                                         \
            fprintf(stderr, "FAIL: %s (%s:%d)\n", (msg), __FILE__, __LINE__);  \
            failures++;                                                        \
        }                                                                      \
    } while (0)

static void path_join(char *out, size_t cap, const char *dir, const char *name)
{
    size_t n = (size_t)snprintf(out, cap, "%s/%s", dir, name);
    if (n >= cap) {
        fprintf(stderr, "path too long: %s/%s\n", dir, name);
        exit(2);
    }
}

static int ensure_dir(const char *dir)
{
    struct stat st;
    if (stat(dir, &st) == 0) {
        if (S_ISDIR(st.st_mode)) {
            return 0;
        }
        fprintf(stderr, "not a directory: %s\n", dir);
        return -1;
    }
    if (mkdir(dir, 0755) != 0 && errno != EEXIST) {
        fprintf(stderr, "mkdir %s: %s\n", dir, strerror(errno));
        return -1;
    }
    return 0;
}

/* Shared logical sample used across absolute / packing variants. */
static void emit_sample_events(nytp_sink *s)
{
    EXPECT(nytp_emit_time_line(s, 5, 1, 10) == NYTP_OK, "tl 1:10");
    EXPECT(nytp_emit_time_line(s, 6, 1, 11) == NYTP_OK, "tl 1:11");
    EXPECT(nytp_emit_time_block(s, 20, 1, 12, 4, 0) == NYTP_OK, "tb");
    EXPECT(nytp_emit_sub_entry(s, 1, 10) == NYTP_OK, "sub_entry");
}

static void write_absolute(const char *outdir)
{
    char path[1024];
    nytp_sink *s;
    path_join(path, sizeof(path), outdir, "absolute.nytprof");
    s = nytp_v6_sink_create(path);
    EXPECT(s != NULL, "create absolute");
    emit_sample_events(s);
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close absolute");
    EXPECT(nytp_v6_sink_is_sealed(s), "sealed absolute");
    EXPECT(nytp_v6_sink_file_written(s), "file written absolute");
    EXPECT(!nytp_v6_sink_packing_enabled(s), "no packing");
    EXPECT(!nytp_v6_sink_string_dict_enabled(s), "no dict");
    printf("wrote %s (%zu bytes)\n", path, nytp_v6_sink_wire_len(s));
    nytp_sink_destroy(s);
}

static void write_packing_codec(const char *outdir, const char *name,
                                uint8_t codec)
{
    char path[1024];
    nytp_v6_sink_options opt;
    nytp_sink *s;
    path_join(path, sizeof(path), outdir, name);
    memset(&opt, 0, sizeof(opt));
    opt.event_codec = codec;
    opt.max_records_per_chunk = 2;
    opt.enable_packing = 1;
    s = nytp_v6_sink_create_opts(path, &opt);
    EXPECT(s != NULL, "create packing");
    emit_sample_events(s);
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close packing");
    EXPECT(nytp_v6_sink_is_sealed(s), "sealed packing");
    EXPECT(nytp_v6_sink_event_chunk_count(s) >= 2, "multi-chunk packing");
    printf("wrote %s (%zu bytes, chunks=%u, codec=%u)\n", path,
           nytp_v6_sink_wire_len(s), nytp_v6_sink_event_chunk_count(s),
           (unsigned)codec);
    nytp_sink_destroy(s);
}

static void write_packing(const char *outdir)
{
    write_packing_codec(outdir, "packing.nytprof",
                        (uint8_t)NYTPROF_V6_CODEC_ZLIB);
}

static void write_packing_lz4(const char *outdir)
{
    write_packing_codec(outdir, "packing_lz4.nytprof",
                        (uint8_t)NYTPROF_V6_CODEC_LZ4);
}

static void write_dict(const char *outdir)
{
    char path[1024];
    nytp_v6_sink_options opt;
    nytp_sink *s;
    path_join(path, sizeof(path), outdir, "dict.nytprof");
    memset(&opt, 0, sizeof(opt));
    opt.event_codec = (uint8_t)NYTPROF_V6_CODEC_NONE;
    opt.enable_string_dict = 1;
    s = nytp_v6_sink_create_opts(path, &opt);
    EXPECT(s != NULL, "create dict");
    EXPECT(nytp_emit_attribute(s, nytp_sv_cstr("basetime"),
                               nytp_sv_cstr("1786111723")) == NYTP_OK,
           "attr basetime");
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("e3-c-dict-label")) == NYTP_OK,
           "comment");
    EXPECT(nytp_emit_time_line(s, 5, 1, 10) == NYTP_OK, "tl");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close dict");
    EXPECT(nytp_v6_sink_has_footer_dict(s), "footer dict");
    EXPECT(nytp_v6_sink_dict_entry_count(s) >= 2, "dict entries");
    printf("wrote %s (%zu bytes, dict_entries=%u)\n", path,
           nytp_v6_sink_wire_len(s), nytp_v6_sink_dict_entry_count(s));
    nytp_sink_destroy(s);
}

static void write_packing_dict(const char *outdir)
{
    char path[1024];
    nytp_v6_sink_options opt;
    nytp_sink *s;
    path_join(path, sizeof(path), outdir, "packing_dict.nytprof");
    memset(&opt, 0, sizeof(opt));
    opt.event_codec = (uint8_t)NYTPROF_V6_CODEC_ZLIB;
    opt.max_records_per_chunk = 2;
    opt.enable_packing = 1;
    opt.enable_string_dict = 1;
    s = nytp_v6_sink_create_opts(path, &opt);
    EXPECT(s != NULL, "create packing_dict");
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("pack-dict-mark")) == NYTP_OK,
           "comment");
    EXPECT(nytp_emit_time_line(s, 5, 1, 10) == NYTP_OK, "tl1");
    EXPECT(nytp_emit_time_line(s, 6, 1, 11) == NYTP_OK, "tl2");
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("# pack-dict-end")) == NYTP_OK,
           "comment2");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close packing_dict");
    EXPECT(nytp_v6_sink_has_footer_dict(s), "footer");
    EXPECT(nytp_v6_sink_event_chunk_count(s) >= 2, "multi-chunk");
    printf("wrote %s (%zu bytes, chunks=%u, dict=%u)\n", path,
           nytp_v6_sink_wire_len(s), nytp_v6_sink_event_chunk_count(s),
           nytp_v6_sink_dict_entry_count(s));
    nytp_sink_destroy(s);
}

static void write_mid_stream(const char *outdir)
{
    char path[1024];
    nytp_v6_sink_options opt;
    nytp_sink *s;
    uint64_t run_ticks[2] = {7, 8};
    path_join(path, sizeof(path), outdir, "mid_stream.nytprof");
    memset(&opt, 0, sizeof(opt));
    opt.event_codec = (uint8_t)NYTPROF_V6_CODEC_NONE;
    opt.enable_packing = 1;
    s = nytp_v6_sink_create_opts(path, &opt);
    EXPECT(s != NULL, "create mid_stream");
    EXPECT(nytp_emit_time_line(s, 5, 1, 10) == NYTP_OK, "pre tl");
    EXPECT(nytp_v6_sink_emit_time_line_run(s, 2, 50, run_ticks, 2) == NYTP_OK,
           "pre run");
    EXPECT(nytp_v6_sink_begin_codec_region(s, (uint8_t)NYTPROF_V6_CODEC_ZLIB) ==
               NYTP_OK,
           "begin zlib");
    EXPECT(nytp_emit_time_line(s, 9, 2, 51) == NYTP_OK, "post tl site-delta");
    EXPECT(nytp_emit_sub_entry(s, 2, 50) == NYTP_OK, "post sub_entry");
    EXPECT(nytp_emit_time_block(s, 3, 3, 9, 7, 0) == NYTP_OK, "post tb");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close mid_stream");
    EXPECT(nytp_v6_sink_event_chunk_count(s) == 2, "2 event chunks");
    printf("wrote %s (%zu bytes, chunks=%u)\n", path, nytp_v6_sink_wire_len(s),
           nytp_v6_sink_event_chunk_count(s));
    nytp_sink_destroy(s);
}

static void write_mid_stream_dict(const char *outdir)
{
    char path[1024];
    nytp_v6_sink_options opt;
    nytp_sink *s;
    uint64_t run_ticks[2] = {7, 8};
    path_join(path, sizeof(path), outdir, "mid_stream_dict.nytprof");
    memset(&opt, 0, sizeof(opt));
    opt.event_codec = (uint8_t)NYTPROF_V6_CODEC_NONE;
    opt.enable_packing = 1;
    opt.enable_string_dict = 1;
    s = nytp_v6_sink_create_opts(path, &opt);
    EXPECT(s != NULL, "create mid_stream_dict");
    EXPECT(nytp_emit_time_line(s, 5, 1, 10) == NYTP_OK, "pre tl");
    EXPECT(nytp_v6_sink_emit_time_line_run(s, 2, 50, run_ticks, 2) == NYTP_OK,
           "pre run");
    EXPECT(nytp_v6_sink_begin_codec_region(s, (uint8_t)NYTPROF_V6_CODEC_ZSTD) ==
               NYTP_OK,
           "begin zstd");
    EXPECT(nytp_emit_time_line(s, 9, 2, 51) == NYTP_OK, "post tl");
    EXPECT(nytp_emit_sub_entry(s, 2, 50) == NYTP_OK, "post sub_entry");
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("ms-e3-c-dict-mark")) == NYTP_OK,
           "post mark-comment");
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("# ms-e3-c-dict-end")) == NYTP_OK,
           "post end-comment");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close mid_stream_dict");
    EXPECT(nytp_v6_sink_has_footer_dict(s), "footer");
    EXPECT(nytp_v6_sink_event_chunk_count(s) == 2, "2 chunks");
    printf("wrote %s (%zu bytes, chunks=%u, dict=%u)\n", path,
           nytp_v6_sink_wire_len(s), nytp_v6_sink_event_chunk_count(s),
           nytp_v6_sink_dict_entry_count(s));
    nytp_sink_destroy(s);
}

int main(int argc, char **argv)
{
    const char *outdir =
        (argc >= 2 && argv[1] && argv[1][0]) ? argv[1] : "fixtures/v6/from-c";

    if (ensure_dir(outdir) != 0) {
        return 2;
    }

    write_absolute(outdir);
    write_packing(outdir);
    write_packing_lz4(outdir);
    write_dict(outdir);
    write_packing_dict(outdir);
    write_mid_stream(outdir);
    write_mid_stream_dict(outdir);

    if (failures) {
        fprintf(stderr, "gen_e3_c_fixtures: %d failure(s)\n", failures);
        return 1;
    }
    printf("gen_e3_c_fixtures: OK (outdir=%s)\n", outdir);
    return 0;
}

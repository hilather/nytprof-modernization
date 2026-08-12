/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-014 — Same-run dual writer (test/dev-only, OQ-4).
 *
 * Proves:
 *   - dual fan-out of each semantic emit to v5 + v6 children
 *   - same-run logical equality (multiplicities + seq/kind rings)
 *   - M4 mini sample under dual
 *   - primary-fixture-shaped synthetic streams (default-calls1 /
 *     blocks-calls1 / calls2-default multiplicity patterns)
 *   - test/dev env probe (NYTPROF_DUAL_SINK / NYTPROF_FORMAT=dual)
 *   - out-of-band compare-meta JSON
 *   - secondary fail-closed after primary success
 *
 * Residual: full fixtures/v5/ oracle corpus dual equality needs live hooks
 * + complete TEST-003 / TEST-008 M6 suite — not claimed.
 *
 * Build/run: make -C collector test
 */
#define _POSIX_C_SOURCE 200809L

#include "nytp_clock.h"
#include "nytp_sink.h"
#include "nytp_sink_counting.h"
#include "nytp_sink_dual.h"
#include "nytp_sink_v5.h"
#include "nytp_sink_v6.h"

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

/* ---- helpers ---- */

static void expect_logical_equal(nytp_sink *dual, const char *label)
{
    EXPECT(nytp_dual_sink_logical_equal(dual), label);
    {
        const nytp_dual_compare_meta *m = nytp_dual_sink_meta(dual);
        EXPECT(m && m->last_equal == 1, "meta last_equal");
    }
}

/* Drive a shared lifecycle skeleton used by primary-fixture-shaped streams. */
static nytp_status emit_header(nytp_sink *s, const char *calls_val)
{
    nytp_status st;
    st = nytp_emit_attribute(s, nytp_sv_cstr("ticks_per_sec"),
                             nytp_sv_cstr("10000000"));
    if (st != NYTP_OK) {
        return st;
    }
    st = nytp_emit_option(s, nytp_sv_cstr("calls"), nytp_sv_cstr(calls_val));
    if (st != NYTP_OK) {
        return st;
    }
    st = nytp_sink_activate(s);
    if (st != NYTP_OK) {
        return st;
    }
    st = nytp_emit_pid_start(s, 100, 1, 0.0);
    if (st != NYTP_OK) {
        return st;
    }
    return nytp_emit_new_fid(s, 1, 0, 0, 0, 0, 0,
                             nytp_sv_cstr("workload.pl"));
}

static nytp_status emit_finalize_tail(nytp_sink *s)
{
    nytp_status st;
    st = nytp_sink_begin_finalize(s);
    if (st != NYTP_OK) {
        return st;
    }
    st = nytp_emit_src_line(s, 1, 5, nytp_sv_cstr("    $x++ for 1 .. 50;\n"));
    if (st != NYTP_OK) {
        return st;
    }
    st = nytp_emit_sub_info(s, 1, 3, 7, nytp_sv_cstr("main::leaf"));
    if (st != NYTP_OK) {
        return st;
    }
    st = nytp_emit_sub_info(s, 1, 8, 12, nytp_sv_cstr("main::mid"));
    if (st != NYTP_OK) {
        return st;
    }
    st = nytp_emit_pid_end(s, 100, 1.0);
    if (st != NYTP_OK) {
        return st;
    }
    return nytp_sink_close(s);
}

/*
 * default-calls1 shape (narrowed R2-5-preview):
 *   TIME_LINE dominant, SUB_ENTRY=0, leaf returns 15, mid returns 3,
 *   mid→leaf edges via SUB_CALLERS, DISCOUNT present.
 * Scaled mini (not full oracle 818 discounts / 916 TL) — multiplicity
 * patterns only for dual equality.
 *
 * incl/excl/reci use **integer tick** values (not fractional wall NV) so
 * v5 NV wire and v6 u64 tick truncation yield equal E4-v0 model aggregates
 * (see E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0 / PR-B10).
 */
static nytp_status run_default_calls1_shape(nytp_sink *s)
{
    nytp_status st;
    int i;
    st = emit_header(s, "1");
    if (st != NYTP_OK) {
        return st;
    }
    /* No SUB_ENTRY (calls=1). 15 leaf + 3 mid returns; 15 mid→leaf edges. */
    for (i = 0; i < 15; i++) {
        st = nytp_emit_time_line(s, 10 + i, 1, 5);
        if (st != NYTP_OK) {
            return st;
        }
        st = nytp_emit_discount(s);
        if (st != NYTP_OK) {
            return st;
        }
        st = nytp_emit_sub_return(s, 1, 100.0, 40.0, nytp_sv_cstr("main::leaf"));
        if (st != NYTP_OK) {
            return st;
        }
        st = nytp_emit_sub_callers(s, 1, 10, 1, 200.0, 100.0, 0.0, 1,
                                   nytp_sv_cstr("main::leaf"),
                                   nytp_sv_cstr("main::mid"));
        if (st != NYTP_OK) {
            return st;
        }
    }
    for (i = 0; i < 3; i++) {
        st = nytp_emit_time_line(s, 20 + i, 1, 10);
        if (st != NYTP_OK) {
            return st;
        }
        st = nytp_emit_sub_return(s, 1, 300.0, 50.0, nytp_sv_cstr("main::mid"));
        if (st != NYTP_OK) {
            return st;
        }
    }
    return emit_finalize_tail(s);
}

/* blocks-calls1 shape: TIME_BLOCK dominant (A4/A4b path). */
static nytp_status run_blocks_calls1_shape(nytp_sink *s)
{
    nytp_status st;
    int i;
    st = emit_header(s, "1");
    if (st != NYTP_OK) {
        return st;
    }
    for (i = 0; i < 12; i++) {
        st = nytp_emit_time_block(s, 5 + i, 1, 5, 4, 3);
        if (st != NYTP_OK) {
            return st;
        }
    }
    st = nytp_emit_sub_return(s, 1, 100.0, 40.0, nytp_sv_cstr("main::leaf"));
    if (st != NYTP_OK) {
        return st;
    }
    return emit_finalize_tail(s);
}

/* calls2-default shape: SUB_ENTRY present (calls=2). */
static nytp_status run_calls2_default_shape(nytp_sink *s)
{
    nytp_status st;
    int i;
    st = emit_header(s, "2");
    if (st != NYTP_OK) {
        return st;
    }
    for (i = 0; i < 9; i++) {
        st = nytp_emit_sub_entry(s, 1, 10);
        if (st != NYTP_OK) {
            return st;
        }
        st = nytp_emit_time_line(s, 3 + i, 1, 5);
        if (st != NYTP_OK) {
            return st;
        }
        st = nytp_emit_sub_return(s, 1, 100.0, 40.0, nytp_sv_cstr("main::leaf"));
        if (st != NYTP_OK) {
            return st;
        }
    }
    return emit_finalize_tail(s);
}

/* ---- tests ---- */

static void test_env_probe(void)
{
    /* Save/restore env is process-local; set then clear carefully. */
    unsetenv("NYTPROF_DUAL_SINK");
    unsetenv("NYTPROF_FORMAT");
    EXPECT(!nytp_dual_env_enabled(), "env off by default");

    setenv("NYTPROF_DUAL_SINK", "1", 1);
    EXPECT(nytp_dual_env_enabled(), "DUAL_SINK=1");
    setenv("NYTPROF_DUAL_SINK", "true", 1);
    EXPECT(nytp_dual_env_enabled(), "DUAL_SINK=true");
    setenv("NYTPROF_DUAL_SINK", "0", 1);
    EXPECT(!nytp_dual_env_enabled(), "DUAL_SINK=0");
    unsetenv("NYTPROF_DUAL_SINK");

    setenv("NYTPROF_FORMAT", "dual", 1);
    EXPECT(nytp_dual_env_enabled(), "FORMAT=dual");
    setenv("NYTPROF_FORMAT", "DUAL", 1);
    EXPECT(nytp_dual_env_enabled(), "FORMAT=DUAL");
    setenv("NYTPROF_FORMAT", "v5", 1);
    EXPECT(!nytp_dual_env_enabled(), "FORMAT=v5 not dual");
    unsetenv("NYTPROF_FORMAT");
}

static void test_m4_dual_v5_v6(void)
{
    nytp_sink *dual =
        nytp_dual_sink_create_v5_v6("build/dual_m4_v5.nytprof",
                                    "build/dual_m4_v6.nytprof");
    nytp_m4_harness_result res;
    const nytp_counting_stats *sv5;
    const nytp_counting_stats *sv6;
    size_t w5 = 0, w6 = 0;
    const uint8_t *wire5, *wire6;
    EXPECT(dual != NULL, "create dual v5+v6");
    if (!dual) {
        return;
    }
    EXPECT(nytp_dual_sink_is_dual(dual), "is dual");
    EXPECT(strcmp(nytp_sink_name(dual), "dual") == 0, "name dual");

    EXPECT(nytp_m4_mini_sample_run(dual, &res) == NYTP_OK, "m4 dual run");
    EXPECT(res.gapless_ok, "gapless dual parent");
    EXPECT(res.logical_events == 12, "12 logical");
    EXPECT(res.ticks_match, "ticks");

    expect_logical_equal(dual, "m4 logical equal v5↔v6");

    sv5 = nytp_dual_child_stats(nytp_dual_sink_primary(dual));
    sv6 = nytp_dual_child_stats(nytp_dual_sink_secondary(dual));
    EXPECT(sv5 && sv6, "child stats");
    if (sv5 && sv6) {
        EXPECT(sv5->by_kind[NYTP_EVT_TIME_LINE] == 3, "tl=3");
        EXPECT(sv5->by_kind[NYTP_EVT_DISCOUNT] == 1, "disc=1");
        EXPECT(sv5->by_kind[NYTP_EVT_START_DEFLATE] == 1, "deflate=1");
        EXPECT(sv5->logical_emits == 12, "logical 12");
        EXPECT(sv5->logical_emits == sv6->logical_emits, "logical match");
        EXPECT(sv5->by_kind[NYTP_EVT_TIME_LINE] ==
                   sv6->by_kind[NYTP_EVT_TIME_LINE],
               "tl match");
    }

    wire5 = nytp_v5_sink_wire(nytp_dual_sink_primary(dual), &w5);
    wire6 = nytp_v6_sink_wire(nytp_dual_sink_secondary(dual), &w6);
    EXPECT(wire5 && w5 >= 12 && memcmp(wire5, "NYTProf 5 0\n", 12) == 0,
           "v5 header");
    EXPECT(wire6 && w6 >= 8 && memcmp(wire6, "NYTPROF6", 8) == 0, "v6 magic");
    EXPECT(nytp_v5_sink_file_written(nytp_dual_sink_primary(dual)), "v5 file");
    EXPECT(nytp_v6_sink_is_sealed(nytp_dual_sink_secondary(dual)), "v6 sealed");

    EXPECT(nytp_dual_sink_write_compare_meta(dual, "build/dual_m4_meta.json") ==
               NYTP_OK,
           "write meta");

    nytp_sink_destroy(dual);
}

static void test_primary_fixture_shapes(void)
{
    struct {
        const char *name;
        nytp_status (*run)(nytp_sink *);
        uint64_t expect_sub_entry;
        uint64_t expect_time_line;
        uint64_t expect_time_block;
        uint64_t expect_leaf_style_returns; /* total SUB_RETURN */
    } cases[] = {
        {"default-calls1", run_default_calls1_shape, 0, 18, 0, 18},
        {"blocks-calls1", run_blocks_calls1_shape, 0, 0, 12, 1},
        {"calls2-default", run_calls2_default_shape, 9, 9, 0, 9},
    };
    size_t i;
    for (i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        char path_v5[128], path_v6[128], path_meta[128];
        nytp_sink *dual;
        const nytp_counting_stats *sv5;
        snprintf(path_v5, sizeof(path_v5), "build/dual_%s_v5.nytprof",
                 cases[i].name);
        snprintf(path_v6, sizeof(path_v6), "build/dual_%s_v6.nytprof",
                 cases[i].name);
        snprintf(path_meta, sizeof(path_meta), "build/dual_%s_meta.json",
                 cases[i].name);
        /* sanitize path: replace - with _ */
        {
            char *p;
            for (p = path_v5; *p; p++) {
                if (*p == '-') {
                    *p = '_';
                }
            }
            for (p = path_v6; *p; p++) {
                if (*p == '-') {
                    *p = '_';
                }
            }
            for (p = path_meta; *p; p++) {
                if (*p == '-') {
                    *p = '_';
                }
            }
        }

        dual = nytp_dual_sink_create_v5_v6(path_v5, path_v6);
        EXPECT(dual != NULL, cases[i].name);
        if (!dual) {
            continue;
        }
        EXPECT(cases[i].run(dual) == NYTP_OK, cases[i].name);
        expect_logical_equal(dual, cases[i].name);

        sv5 = nytp_dual_child_stats(nytp_dual_sink_primary(dual));
        EXPECT(sv5 != NULL, "stats");
        if (sv5) {
            EXPECT(sv5->by_kind[NYTP_EVT_SUB_ENTRY] == cases[i].expect_sub_entry,
                   "sub_entry mult");
            EXPECT(sv5->by_kind[NYTP_EVT_TIME_LINE] == cases[i].expect_time_line,
                   "time_line mult");
            EXPECT(sv5->by_kind[NYTP_EVT_TIME_BLOCK] ==
                       cases[i].expect_time_block,
                   "time_block mult");
            EXPECT(sv5->by_kind[NYTP_EVT_SUB_RETURN] ==
                       cases[i].expect_leaf_style_returns,
                   "sub_return mult");
        }
        EXPECT(nytp_dual_sink_write_compare_meta(dual, path_meta) == NYTP_OK,
               "meta");
        nytp_sink_destroy(dual);
    }
}

static void test_counting_dual_fanout(void)
{
    /* Dual of two counting sinks — pure logical without wire. */
    nytp_sink *a = nytp_counting_sink_create();
    nytp_sink *b = nytp_counting_sink_create();
    nytp_sink *dual;
    const nytp_counting_stats *sa, *sb;
    EXPECT(a && b, "counting create");
    if (!a || !b) {
        if (a) {
            nytp_sink_destroy(a);
        }
        if (b) {
            nytp_sink_destroy(b);
        }
        return;
    }
    dual = nytp_dual_sink_create(a, b, 1, 1);
    EXPECT(dual != NULL, "dual counting");
    if (!dual) {
        nytp_sink_destroy(a);
        nytp_sink_destroy(b);
        return;
    }

    EXPECT(nytp_sink_activate(dual) == NYTP_OK, "activate");
    EXPECT(nytp_emit_time_line(dual, 7, 1, 2) == NYTP_OK, "tl");
    EXPECT(nytp_emit_discount(dual) == NYTP_OK, "disc");
    EXPECT(nytp_emit_sub_entry(dual, 1, 9) == NYTP_OK, "entry");
    EXPECT(nytp_sink_logical_count(dual) == 3, "parent logical 3");
    expect_logical_equal(dual, "counting equal");

    sa = nytp_counting_sink_stats(nytp_dual_sink_primary(dual));
    sb = nytp_counting_sink_stats(nytp_dual_sink_secondary(dual));
    EXPECT(sa && sb && sa->seq_ring_len == 3 && sb->seq_ring_len == 3,
           "seq ring 3");
    EXPECT(sa && sa->seq_ring[0] == 0 && sa->seq_ring[2] == 2, "gapless");
    EXPECT(sa && sb && sa->kind_ring[0] == sb->kind_ring[0] &&
               sa->kind_ring[1] == sb->kind_ring[1],
           "kinds match");

    EXPECT(nytp_sink_close(dual) == NYTP_OK, "close");
    nytp_sink_destroy(dual);
}

/*
 * After secondary fail (primary already wrote): dual must sticky-fail.
 * Covers IO (native sticky) and STATE (mapped to FAILED by dual_fanout).
 */
static void assert_secondary_partial_sticky(nytp_status arm_err,
                                            nytp_status expect_emit_err,
                                            const char *label)
{
    nytp_sink *ok = nytp_counting_sink_create();
    nytp_sink *bad = nytp_counting_sink_create();
    nytp_sink *dual;
    const nytp_dual_compare_meta *m;
    const nytp_counting_stats *sa;
    const nytp_counting_stats *sb;
    char msg[96];

    EXPECT(ok && bad, label);
    if (!ok || !bad) {
        if (ok) {
            nytp_sink_destroy(ok);
        }
        if (bad) {
            nytp_sink_destroy(bad);
        }
        return;
    }
    dual = nytp_dual_sink_create(ok, bad, 1, 1);
    EXPECT(dual != NULL, label);
    if (!dual) {
        nytp_sink_destroy(ok);
        nytp_sink_destroy(bad);
        return;
    }
    EXPECT(nytp_sink_activate(dual) == NYTP_OK, label);
    EXPECT(nytp_counting_sink_fail_next(bad, arm_err) == NYTP_OK, label);
    snprintf(msg, sizeof(msg), "%s emit err", label);
    EXPECT(nytp_emit_time_line(dual, 1, 1, 1) == expect_emit_err, msg);

    m = nytp_dual_sink_meta(dual);
    EXPECT(m && m->fanout_fail_secondary == 1, label);
    EXPECT(m && m->fanout_ok == 0, label);

    /* Hard sticky-fail — not soft OR with next-emit check alone. */
    snprintf(msg, sizeof(msg), "%s state FAILED", label);
    EXPECT(nytp_sink_get_state(dual) == NYTP_SINK_FAILED, msg);
    snprintf(msg, sizeof(msg), "%s fail_reason", label);
    EXPECT(nytp_sink_fail_reason(dual) == expect_emit_err, msg);
    snprintf(msg, sizeof(msg), "%s next emit STATE", label);
    EXPECT(nytp_emit_time_line(dual, 2, 1, 1) == NYTP_ERR_STATE, msg);
    /* No logical commit on dual (seq not advanced). */
    EXPECT(nytp_sink_logical_count(dual) == 0, label);
    EXPECT(!nytp_dual_sink_logical_equal(dual), label);

    /* Primary counted emit; secondary did not (fail_next before count). */
    sa = nytp_counting_sink_stats(nytp_dual_sink_primary(dual));
    sb = nytp_counting_sink_stats(nytp_dual_sink_secondary(dual));
    EXPECT(sa && sa->by_kind[NYTP_EVT_TIME_LINE] == 1, label);
    EXPECT(sb && sb->by_kind[NYTP_EVT_TIME_LINE] == 0, label);
    /* Primary seq ring empty — dual never committed logical. */
    EXPECT(sa && sa->logical_emits == 0 && sa->seq_ring_len == 0, label);

    nytp_sink_destroy(dual);
}

static void test_secondary_fail_sticky(void)
{
    /* IO is natively sticky via emit_commit. */
    assert_secondary_partial_sticky(NYTP_ERR_IO, NYTP_ERR_IO, "secondary IO");
    /* STATE maps to FAILED so dual sticky-fails (Issue 1). */
    assert_secondary_partial_sticky(NYTP_ERR_STATE, NYTP_ERR_FAILED,
                                    "secondary STATE→FAILED");
    /* UNSUPPORTED similarly mapped. */
    assert_secondary_partial_sticky(NYTP_ERR_UNSUPPORTED, NYTP_ERR_FAILED,
                                    "secondary UNSUPPORTED→FAILED");
}

static void test_finalize_order_and_lifecycle(void)
{
    nytp_sink *dual = nytp_dual_sink_create_v5_v6(NULL, NULL);
    nytp_sink *v5, *v6;
    EXPECT(dual != NULL, "create");
    if (!dual) {
        return;
    }
    v5 = nytp_dual_sink_primary(dual);
    v6 = nytp_dual_sink_secondary(dual);
    EXPECT(nytp_sink_activate(dual) == NYTP_OK, "activate");
    EXPECT(nytp_sink_get_state(v5) == NYTP_SINK_ACTIVE, "v5 active");
    EXPECT(nytp_sink_get_state(v6) == NYTP_SINK_ACTIVE, "v6 active");
    EXPECT(nytp_emit_pid_start(dual, 1, 0, 0.0) == NYTP_OK, "pid");
    EXPECT(nytp_sink_begin_finalize(dual) == NYTP_OK, "finalize");
    EXPECT(nytp_sink_get_state(v5) == NYTP_SINK_FINALIZING, "v5 finalizing");
    EXPECT(nytp_sink_get_state(v6) == NYTP_SINK_FINALIZING, "v6 finalizing");
    EXPECT(nytp_emit_time_line(dual, 1, 1, 1) == NYTP_ERR_STATE,
           "hot path rejected");
    EXPECT(nytp_emit_pid_end(dual, 1, 1.0) == NYTP_OK, "pid_end");
    EXPECT(nytp_sink_close(dual) == NYTP_OK, "close");
    EXPECT(nytp_sink_get_state(dual) == NYTP_SINK_CLOSED, "dual closed");
    EXPECT(nytp_sink_get_state(v5) == NYTP_SINK_CLOSED, "v5 closed");
    EXPECT(nytp_sink_get_state(v6) == NYTP_SINK_CLOSED, "v6 closed");
    expect_logical_equal(dual, "finalize equal");
    nytp_sink_destroy(dual);
}

int main(void)
{
    test_env_probe();
    test_counting_dual_fanout();
    test_m4_dual_v5_v6();
    test_primary_fixture_shapes();
    test_secondary_fail_sticky();
    test_finalize_order_and_lifecycle();

    if (failures != 0) {
        fprintf(stderr, "test_dual_sink: %d failure(s)\n", failures);
        return 1;
    }
    printf("OK: test_dual_sink (COL-014 dual-sink test/dev-only OQ-4)\n");
    return 0;
}

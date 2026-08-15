/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * TEST-003 / PR-B03 fake-clock + M4 mini-sample harness tests.
 * Build/run: make -C collector test
 *
 * Residual honesty: full M4 oracle v5-via-sink corpus equality needs
 * COL-006 (real wire) + complete TEST-003 — not claimed here.
 */
#include "nytp_clock.h"
#include "nytp_sink.h"
#include "nytp_sink_counting.h"
#include "nytp_sink_v5.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

static int failures = 0;

#define EXPECT(cond, msg)                                                      \
    do {                                                                       \
        if (!(cond)) {                                                         \
            fprintf(stderr, "FAIL: %s (%s:%d)\n", (msg), __FILE__, __LINE__);  \
            failures++;                                                        \
        }                                                                      \
    } while (0)

static void test_fake_clock_script_and_exhaust(void)
{
    static const nytp_ticks script[] = {10, 20, 35};
    nytp_fake_clock fc;
    nytp_ticks v = 0;
    nytp_fake_clock_init(&fc, script, 3);

    EXPECT(nytp_fake_clock_remaining(&fc) == 3, "rem 3");
    EXPECT(nytp_fake_clock_read(&fc, &v) == NYTP_OK && v == 10, "r1");
    EXPECT(nytp_fake_clock_read(&fc, &v) == NYTP_OK && v == 20, "r2");
    EXPECT(nytp_fake_clock_remaining(&fc) == 1, "rem 1");
    EXPECT(nytp_fake_clock_read(&fc, &v) == NYTP_OK && v == 35, "r3");
    EXPECT(nytp_fake_clock_read(&fc, &v) == NYTP_ERR_EXHAUSTED, "exhaust");
    EXPECT(fc.exhausted, "flag");
    EXPECT(nytp_fake_clock_remaining(&fc) == 0, "rem 0");

    nytp_fake_clock_reset(&fc);
    EXPECT(!fc.exhausted && nytp_fake_clock_remaining(&fc) == 3, "reset");
    EXPECT(nytp_fake_clock_read(&fc, &v) == NYTP_OK && v == 10, "after reset");
}

static void test_stmt_driver_attribution(void)
{
    static const nytp_ticks script[] = {100, 150, 180};
    nytp_fake_clock fc;
    nytp_stmt_driver drv;
    nytp_sink *s = nytp_counting_sink_create();
    const nytp_counting_stats *st;
    nytp_ticks attributed = 0;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }

    nytp_fake_clock_init(&fc, script, 3);
    nytp_stmt_driver_init(&drv, &fc, 7);

    EXPECT(nytp_stmt_driver_on_line(&drv, s, 1, &attributed) == NYTP_OK,
           "seed");
    EXPECT(attributed == 0, "seed no ticks");
    st = nytp_counting_sink_stats(s);
    EXPECT(st && st->by_kind[NYTP_EVT_TIME_LINE] == 0, "no tl yet");

    EXPECT(nytp_stmt_driver_on_line(&drv, s, 2, &attributed) == NYTP_OK,
           "enter 2");
    EXPECT(attributed == 50, "delta 50");
    st = nytp_counting_sink_stats(s);
    EXPECT(st && st->by_kind[NYTP_EVT_TIME_LINE] == 1, "one tl");
    EXPECT(st && st->last_ticks == 50 && st->last_fid == 7 &&
               st->last_line == 1,
           "attributed to prev line 1");

    EXPECT(nytp_stmt_driver_on_line(&drv, s, 3, &attributed) == NYTP_OK,
           "enter 3");
    EXPECT(attributed == 30, "delta 30");
    st = nytp_counting_sink_stats(s);
    EXPECT(st && st->last_line == 2 && st->last_ticks == 30, "prev line 2");

    EXPECT(nytp_stmt_driver_on_line(&drv, s, 4, &attributed) ==
               NYTP_ERR_EXHAUSTED,
           "clock exhaust");
    nytp_sink_destroy(s);
}

/*
 * Issue 2 regression: failed emit must not consume the clock tick.
 * seed@100, stop sink, enter L2 peeks 150 and fails, re-activate, enter L2
 * attributes 50 (150-100), not 80 (180-100).
 */
static void test_stmt_driver_no_clock_consume_on_emit_fail(void)
{
    static const nytp_ticks script[] = {100, 150, 180};
    nytp_fake_clock fc;
    nytp_stmt_driver drv;
    nytp_sink *s = nytp_counting_sink_create();
    nytp_ticks attributed = 0;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    nytp_fake_clock_init(&fc, script, 3);
    nytp_stmt_driver_init(&drv, &fc, 1);

    EXPECT(nytp_stmt_driver_on_line(&drv, s, 1, &attributed) == NYTP_OK,
           "seed");
    EXPECT(nytp_fake_clock_remaining(&fc) == 2, "rem 2 after seed");

    EXPECT(nytp_sink_activate(s) == NYTP_OK, "activate");
    EXPECT(nytp_sink_stop(s) == NYTP_OK, "stop");
    /* STOPPED rejects emit with ERR_STATE (does not sticky-fail the sink). */
    EXPECT(nytp_stmt_driver_on_line(&drv, s, 2, &attributed) == NYTP_ERR_STATE,
           "emit fails stopped");
    EXPECT(nytp_fake_clock_remaining(&fc) == 2, "tick not consumed");

    EXPECT(nytp_sink_activate(s) == NYTP_OK, "restart");
    EXPECT(nytp_stmt_driver_on_line(&drv, s, 2, &attributed) == NYTP_OK,
           "retry");
    EXPECT(attributed == 50, "delta 50 not 80");
    EXPECT(nytp_fake_clock_remaining(&fc) == 1, "consumed after success");
    nytp_sink_destroy(s);
}

static void test_stmt_driver_backwards_tick(void)
{
    static const nytp_ticks script[] = {100, 50};
    nytp_fake_clock fc;
    nytp_stmt_driver drv;
    nytp_sink *s = nytp_counting_sink_create();
    nytp_ticks attributed = 0;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    nytp_fake_clock_init(&fc, script, 2);
    nytp_stmt_driver_init(&drv, &fc, 1);
    EXPECT(nytp_stmt_driver_on_line(&drv, s, 1, &attributed) == NYTP_OK,
           "seed");
    EXPECT(nytp_stmt_driver_on_line(&drv, s, 2, &attributed) ==
               NYTP_ERR_OVERFLOW,
           "backwards fail-closed");
    EXPECT(nytp_fake_clock_remaining(&fc) == 1, "no consume on overflow");
    nytp_sink_destroy(s);
}

static void test_m4_mini_sample_counting(void)
{
    nytp_sink *s = nytp_counting_sink_create();
    nytp_m4_harness_result res;
    const nytp_counting_stats *st;
    nytp_seq seqs[32];
    nytp_event_kind kinds[32];
    size_t n = 32;
    size_t nk = 32;
    size_t n_exp = 0;
    size_t i;
    const nytp_m4_step *exp;
    nytp_seq_mismatch mm;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }

    exp = nytp_m4_mini_sample_expected(&n_exp);
    EXPECT(exp != NULL && n_exp == 12, "expected steps");

    EXPECT(nytp_m4_mini_sample_run(s, &res) == NYTP_OK, "m4 run counting");
    EXPECT(res.gapless_ok, "gapless");
    EXPECT(res.kinds_match, "kinds order");
    EXPECT(res.ticks_match, "ticks");
    EXPECT(res.logical_events == n_exp, "logical n");
    EXPECT(res.last_seq == (nytp_seq)(n_exp - 1), "last seq");
    EXPECT(res.run_status == NYTP_OK, "run_status");

    st = nytp_counting_sink_stats(s);
    EXPECT(st != NULL, "stats");
    if (st) {
        EXPECT(st->by_kind[NYTP_EVT_TIME_LINE] == 3, "3 time_line");
        EXPECT(st->by_kind[NYTP_EVT_DISCOUNT] == 1, "1 discount");
        EXPECT(st->by_kind[NYTP_EVT_START_DEFLATE] == 1, "1 deflate control");
        EXPECT(st->by_kind[NYTP_EVT_PID_START] == 1, "pid_start");
        EXPECT(st->by_kind[NYTP_EVT_PID_END] == 1, "pid_end");
        EXPECT(st->logical_emits == n_exp, "logical_emits");
        /* total = logical + 1 control */
        EXPECT(st->total_emits == n_exp + 1, "total with control");
    }

    EXPECT(nytp_counting_sink_copy_seqs(s, seqs, &n) == NYTP_OK, "copy");
    EXPECT(n == n_exp, "seq n");
    EXPECT(nytp_seq_check_gapless(seqs, n, 0, &mm), "ring gapless");
    EXPECT(nytp_counting_sink_copy_kinds(s, kinds, &nk) == NYTP_OK, "kinds");
    EXPECT(nk == n_exp, "kind n");
    for (i = 0; i < n_exp; i++) {
        EXPECT(kinds[i] == exp[i].kind, "kind order match");
    }

    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_CLOSED, "closed");
    nytp_sink_destroy(s);
}

static void test_m4_mini_sample_v5_wire(void)
{
    nytp_sink *s = nytp_v5_sink_create("build/m4-mini-fake-clock.nytprof");
    nytp_m4_harness_result res;
    const nytp_counting_stats *st;
    size_t wlen = 0;
    const uint8_t *wire;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }

    EXPECT(nytp_m4_mini_sample_run(s, &res) == NYTP_OK, "m4 run v5 wire");
    EXPECT(res.gapless_ok && res.ticks_match, "ok flags");
    st = nytp_v5_sink_stats(s);
    EXPECT(st && st->by_kind[NYTP_EVT_TIME_LINE] == 3, "v5 tl count");
    EXPECT(st && st->logical_emits == 12, "v5 logical");
    /* COL-006: real wire bytes present (header + body). */
    wire = nytp_v5_sink_wire(s, &wlen);
    EXPECT(wire != NULL && wlen >= 12, "wire bytes");
    EXPECT(wire && memcmp(wire, "NYTProf 5 0\n", 12) == 0, "v5 header");
    EXPECT(nytp_v5_sink_path(s) != NULL, "path set");
    EXPECT(nytp_v5_sink_file_written(s), "file written");
    nytp_sink_destroy(s);
}

static void test_production_clock_now(void)
{
    nytp_ticks a = 0;
    nytp_ticks b = 0;
    EXPECT(nytp_clock_now(NULL) == NYTP_ERR_NULL, "clock_now null");
    EXPECT(nytp_clock_now(&a) == NYTP_OK && a > 0, "clock_now a");
    EXPECT(nytp_clock_now(&b) == NYTP_OK && b >= a, "clock_now monotonic");
}

int main(void)
{
    test_production_clock_now();
    test_fake_clock_script_and_exhaust();
    test_stmt_driver_attribution();
    test_stmt_driver_no_clock_consume_on_emit_fail();
    test_stmt_driver_backwards_tick();
    test_m4_mini_sample_counting();
    test_m4_mini_sample_v5_wire();

    if (failures != 0) {
        fprintf(stderr, "test_fake_clock: %d failure(s)\n", failures);
        return 1;
    }
    printf("OK: test_fake_clock (TEST-003 + M4 mini via v5 wire)\n");
    return 0;
}

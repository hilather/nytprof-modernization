/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-002 lifecycle + COL-003 sequence unit tests.
 * Build/run: make -C collector test
 */
#include "nytp_sink.h"
#include "nytp_sink_counting.h"
#include "nytp_sink_v5.h"

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

static void test_state_names_and_table(void)
{
    EXPECT(strcmp(nytp_sink_state_name(NYTP_SINK_OPEN), "open") == 0, "open");
    EXPECT(strcmp(nytp_sink_state_name(NYTP_SINK_STOPPED), "stopped") == 0,
           "stopped");
    EXPECT(strcmp(nytp_sink_state_name(NYTP_SINK_FINALIZING), "finalizing") ==
               0,
           "finalizing");
    EXPECT(strcmp(nytp_sink_state_name(NYTP_SINK_FORK_SPLIT), "fork_split") ==
               0,
           "fork_split");

    EXPECT(nytp_sink_transition_allowed(NYTP_SINK_OPEN, NYTP_SINK_ACTIVE),
           "open->active");
    EXPECT(nytp_sink_transition_allowed(NYTP_SINK_ACTIVE, NYTP_SINK_STOPPED),
           "active->stopped");
    EXPECT(nytp_sink_transition_allowed(NYTP_SINK_STOPPED, NYTP_SINK_ACTIVE),
           "stopped->active restart");
    EXPECT(nytp_sink_transition_allowed(NYTP_SINK_ACTIVE, NYTP_SINK_FINALIZING),
           "active->finalizing");
    EXPECT(nytp_sink_transition_allowed(NYTP_SINK_FINALIZING, NYTP_SINK_CLOSED),
           "finalizing->closed");
    EXPECT(nytp_sink_transition_allowed(NYTP_SINK_ACTIVE, NYTP_SINK_FORK_SPLIT),
           "active->fork");
    EXPECT(nytp_sink_transition_allowed(NYTP_SINK_FORK_SPLIT, NYTP_SINK_OPEN),
           "fork->child open");
    EXPECT(nytp_sink_transition_allowed(NYTP_SINK_CLOSED, NYTP_SINK_CLOSED),
           "close idempotent");
    EXPECT(!nytp_sink_transition_allowed(NYTP_SINK_CLOSED, NYTP_SINK_ACTIVE),
           "no reopen");
    EXPECT(!nytp_sink_transition_allowed(NYTP_SINK_STOPPED, NYTP_SINK_FORK_SPLIT),
           "no fork from stopped");
}

static void test_stop_restart(void)
{
    nytp_sink *s = nytp_counting_sink_create();
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }

    EXPECT(nytp_sink_activate(s) == NYTP_OK, "activate");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_ACTIVE, "active");
    EXPECT(nytp_emit_time_line(s, 1, 1, 1) == NYTP_OK, "emit active");

    EXPECT(nytp_sink_stop(s) == NYTP_OK, "stop");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_STOPPED, "stopped");
    EXPECT(nytp_emit_time_line(s, 1, 1, 1) == NYTP_ERR_STATE,
           "no emit stopped");
    EXPECT(nytp_sink_stop(s) == NYTP_ERR_STATE, "double stop");

    EXPECT(nytp_sink_activate(s) == NYTP_OK, "restart");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_ACTIVE, "active again");
    EXPECT(nytp_emit_discount(s) == NYTP_OK, "emit after restart");

    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close idempotent");
    EXPECT(nytp_sink_activate(s) == NYTP_ERR_STATE, "no activate after close");
    nytp_sink_destroy(s);
}

static void test_finalize_gates(void)
{
    nytp_sink *s = nytp_counting_sink_create();
    /* FINALIZING allow/deny matrix (COL-002 freeze). */
    static const struct {
        nytp_event_kind kind;
        int allow;
    } matrix[] = {
        {NYTP_EVT_SRC_LINE, 1},     {NYTP_EVT_SUB_INFO, 1},
        {NYTP_EVT_SUB_CALLERS, 1},  {NYTP_EVT_PID_END, 1},
        {NYTP_EVT_ATTRIBUTE, 1},    {NYTP_EVT_OPTION, 1},
        {NYTP_EVT_COMMENT, 1},      {NYTP_EVT_DISCOUNT, 1},
        {NYTP_EVT_TIME_LINE, 0},    {NYTP_EVT_TIME_BLOCK, 0},
        {NYTP_EVT_SUB_ENTRY, 0},    {NYTP_EVT_SUB_RETURN, 0},
        {NYTP_EVT_PID_START, 0},    {NYTP_EVT_NEW_FID, 0},
        {NYTP_EVT_START_DEFLATE, 0},
    };
    size_t i;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "activate");
    EXPECT(nytp_sink_begin_finalize(s) == NYTP_OK, "finalize");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_FINALIZING, "finalizing");

    for (i = 0; i < sizeof(matrix) / sizeof(matrix[0]); i++) {
        int can = nytp_sink_can_emit(s, matrix[i].kind);
        EXPECT(can == matrix[i].allow, nytp_event_kind_name(matrix[i].kind));
    }

    EXPECT(nytp_emit_time_line(s, 1, 1, 1) == NYTP_ERR_STATE,
           "no time_line in finalize");
    EXPECT(nytp_emit_sub_entry(s, 1, 1) == NYTP_ERR_STATE,
           "no sub_entry in finalize");
    EXPECT(nytp_emit_start_deflate(s) == NYTP_ERR_STATE,
           "no deflate in finalize");
    EXPECT(nytp_emit_src_line(s, 1, 1, nytp_sv_cstr("x")) == NYTP_OK,
           "src_line ok");
    EXPECT(nytp_emit_sub_info(s, 1, 1, 2, nytp_sv_cstr("main::x")) == NYTP_OK,
           "sub_info ok");
    EXPECT(nytp_emit_sub_callers(s, 1, 1, 1, 0.1, 0.1, 0.0, 0,
                                 nytp_sv_cstr("main::x"),
                                 nytp_sv_cstr("main::y")) == NYTP_OK,
           "sub_callers ok");
    EXPECT(nytp_emit_discount(s) == NYTP_OK, "discount ok");
    EXPECT(nytp_emit_attribute(s, nytp_sv_cstr("k"), nytp_sv_cstr("v")) ==
               NYTP_OK,
           "attr ok");
    EXPECT(nytp_emit_pid_end(s, 1, 1.0) == NYTP_OK, "pid_end ok");

    EXPECT(nytp_sink_begin_finalize(s) == NYTP_ERR_STATE, "no double finalize");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    nytp_sink_destroy(s);
}

static void test_fork_split_seq_reset(void)
{
    nytp_sink *s = nytp_counting_sink_create();
    nytp_seq last = 0;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "activate");
    EXPECT(nytp_emit_time_line(s, 1, 1, 1) == NYTP_OK, "tl1");
    EXPECT(nytp_emit_time_line(s, 2, 1, 2) == NYTP_OK, "tl2");
    EXPECT(nytp_sink_last_seq(s, &last) == NYTP_OK, "last");
    EXPECT(last == 1, "last seq 1");
    EXPECT(nytp_sink_logical_count(s) == 2, "count 2");

    EXPECT(nytp_sink_begin_fork(s) == NYTP_OK, "begin fork");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_FORK_SPLIT, "fork_split");
    EXPECT(nytp_emit_discount(s) == NYTP_ERR_STATE, "no emit in fork_split");

    /* Parent path keeps seq. */
    EXPECT(nytp_sink_end_fork_parent(s) == NYTP_OK, "end parent");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_ACTIVE, "parent active");
    EXPECT(nytp_sink_peek_seq(s) == 2, "parent peek continues");
    EXPECT(nytp_emit_discount(s) == NYTP_OK, "parent emit");
    EXPECT(nytp_sink_logical_count(s) == 3, "parent count 3");

    /* Child path: re-enter fork and reset. */
    EXPECT(nytp_sink_begin_fork(s) == NYTP_OK, "begin fork2");
    EXPECT(nytp_sink_end_fork_child(s) == NYTP_OK, "end child");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_OPEN, "child open");
    EXPECT(nytp_sink_peek_seq(s) == 0, "child seq reset");
    EXPECT(nytp_sink_last_seq(s, &last) == NYTP_ERR_STATE, "child no last");
    EXPECT(nytp_emit_pid_start(s, 99, 42, 0.0) == NYTP_OK, "child pid_start");
    EXPECT(nytp_sink_last_seq(s, &last) == NYTP_OK && last == 0,
           "child first seq 0");

    nytp_sink_destroy(s);
}

static void test_mark_failed(void)
{
    nytp_sink *s = nytp_counting_sink_create();
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "activate");
    EXPECT(nytp_sink_mark_failed(s, NYTP_ERR_IO) == NYTP_OK, "fail");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_FAILED, "failed");
    EXPECT(nytp_sink_fail_reason(s) == NYTP_ERR_IO, "reason");
    EXPECT(nytp_emit_discount(s) == NYTP_ERR_STATE, "no emit failed");
    EXPECT(nytp_sink_activate(s) == NYTP_ERR_STATE, "no activate failed");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close from failed");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_CLOSED, "closed");
    EXPECT(nytp_sink_mark_failed(s, NYTP_ERR_IO) == NYTP_ERR_STATE,
           "no fail closed");
    nytp_sink_destroy(s);
}

static void test_sequence_gapless_and_control(void)
{
    nytp_sink *s = nytp_counting_sink_create();
    const nytp_counting_stats *st;
    nytp_seq seqs[16];
    size_t n = 16;
    nytp_seq_mismatch mm;
    nytp_seq last = 0;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }

    EXPECT(nytp_event_kind_is_logical(NYTP_EVT_TIME_LINE), "tl logical");
    EXPECT(nytp_event_kind_is_logical(NYTP_EVT_DISCOUNT), "discount logical");
    EXPECT(!nytp_event_kind_is_logical(NYTP_EVT_START_DEFLATE),
           "deflate not logical");
    EXPECT(!nytp_event_kind_is_logical(NYTP_EVT_NONE), "none not logical");

    EXPECT(nytp_sink_last_seq(s, &last) == NYTP_ERR_STATE, "no last yet");
    EXPECT(nytp_sink_peek_seq(s) == 0, "peek 0");

    EXPECT(nytp_emit_attribute(s, nytp_sv_cstr("k"), nytp_sv_cstr("v")) ==
               NYTP_OK,
           "attr");
    EXPECT(nytp_sink_last_seq(s, &last) == NYTP_OK && last == 0, "seq 0");
    EXPECT(nytp_emit_option(s, nytp_sv_cstr("calls"), nytp_sv_cstr("1")) ==
               NYTP_OK,
           "opt");
    EXPECT(nytp_emit_start_deflate(s) == NYTP_OK, "deflate");
    /* Control must not advance seq. */
    EXPECT(nytp_sink_peek_seq(s) == 2, "peek still 2 after deflate");
    EXPECT(nytp_sink_last_seq(s, &last) == NYTP_OK && last == 1,
           "last still option");

    EXPECT(nytp_emit_time_line(s, 10, 1, 1) == NYTP_OK, "tl");
    EXPECT(nytp_emit_discount(s) == NYTP_OK, "disc");
    EXPECT(nytp_sink_logical_count(s) == 4, "4 logical");

    st = nytp_counting_sink_stats(s);
    EXPECT(st != NULL, "stats");
    if (st) {
        EXPECT(st->logical_emits == 4, "stats logical");
        EXPECT(st->by_kind[NYTP_EVT_START_DEFLATE] == 1, "control counted");
        EXPECT(st->total_emits == 5, "total includes control");
        EXPECT(st->has_last_seq && st->last_seq == 3, "stats last_seq");
    }

    EXPECT(nytp_counting_sink_copy_seqs(s, seqs, &n) == NYTP_OK, "copy seqs");
    EXPECT(n == 4, "n==4");
    EXPECT(nytp_seq_check_gapless(seqs, n, 0, &mm), "gapless from 0");

    /* Inject a gap and ensure comparator reports first mismatch. */
    seqs[2] = 99;
    EXPECT(!nytp_seq_check_gapless(seqs, n, 0, &mm), "detect gap");
    EXPECT(mm.index == 2 && mm.expected_seq == 2 && mm.actual_seq == 99,
           "mismatch detail");

    nytp_sink_destroy(s);
}

static void test_v5_stub_seq_not_wire_claim(void)
{
    /* v5 wire sink still assigns internal seq for dual compare (not on wire). */
    nytp_sink *s = nytp_v5_sink_create(NULL);
    const nytp_counting_stats *st;
    nytp_seq last = 0;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "activate");
    EXPECT(nytp_emit_time_line(s, 5, 1, 1) == NYTP_OK, "tl");
    EXPECT(nytp_emit_start_deflate(s) == NYTP_OK, "deflate");
    EXPECT(nytp_emit_discount(s) == NYTP_OK, "disc");
    EXPECT(nytp_sink_last_seq(s, &last) == NYTP_OK && last == 1,
           "logical last 1");
    EXPECT(nytp_sink_logical_count(s) == 2, "2 logical");
    st = nytp_v5_sink_stats(s);
    EXPECT(st && st->logical_emits == 2, "v5 stats logical");
    EXPECT(st && st->by_kind[NYTP_EVT_START_DEFLATE] == 1, "v5 control");
    EXPECT(st && st->seq_ring_len == 2, "v5 ring len");
    EXPECT(st && st->kind_ring[0] == NYTP_EVT_TIME_LINE, "v5 kind0");
    EXPECT(st && st->kind_ring[1] == NYTP_EVT_DISCOUNT, "v5 kind1");
    nytp_sink_destroy(s);
}

/*
 * Regression: failed emit must not leave a phantom seq in the ring
 * (Issue 3 — backends record only post emit_commit).
 */
static void test_failed_emit_no_phantom_seq(void)
{
    nytp_sink *s = nytp_counting_sink_create();
    const nytp_counting_stats *st;
    nytp_seq seqs[8];
    size_t n = 8;
    nytp_seq last = 0;
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_emit_time_line(s, 1, 1, 1) == NYTP_OK, "tl ok");
    EXPECT(nytp_sink_logical_count(s) == 1, "count 1");

    EXPECT(nytp_counting_sink_fail_next(s, NYTP_ERR_IO) == NYTP_OK, "arm fail");
    EXPECT(nytp_emit_discount(s) == NYTP_ERR_IO, "fail io");
    /* Wrapper marks FAILED on IO. */
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_FAILED, "failed state");
    EXPECT(nytp_sink_logical_count(s) == 1, "seq not advanced");
    EXPECT(nytp_sink_last_seq(s, &last) == NYTP_OK && last == 0, "last still 0");

    st = nytp_counting_sink_stats(s);
    EXPECT(st && st->seq_ring_len == 1, "ring only success");
    EXPECT(st && st->logical_emits == 1, "logical only success");
    EXPECT(st && st->by_kind[NYTP_EVT_DISCOUNT] == 0, "failed not counted");
    EXPECT(nytp_counting_sink_copy_seqs(s, seqs, &n) == NYTP_OK && n == 1 &&
               seqs[0] == 0,
           "ring gapless single");
    nytp_sink_destroy(s);
}

int main(void)
{
    test_state_names_and_table();
    test_stop_restart();
    test_finalize_gates();
    test_fork_split_seq_reset();
    test_mark_failed();
    test_sequence_gapless_and_control();
    test_v5_stub_seq_not_wire_claim();
    test_failed_emit_no_phantom_seq();

    if (failures != 0) {
        fprintf(stderr, "test_lifecycle_seq: %d failure(s)\n", failures);
        return 1;
    }
    printf("OK: test_lifecycle_seq (COL-002 lifecycle + COL-003 sequence)\n");
    return 0;
}

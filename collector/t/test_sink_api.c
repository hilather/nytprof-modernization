/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-001 unit tests: semantic emit surface via counting + stub v5 sinks.
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

static void test_null_sink_guards(void)
{
    EXPECT(nytp_emit_time_line(NULL, 1, 1, 1) == NYTP_ERR_NULL,
           "null time_line");
    EXPECT(nytp_emit_discount(NULL) == NYTP_ERR_NULL, "null discount");
    EXPECT(nytp_sink_activate(NULL) == NYTP_ERR_NULL, "null activate");
    EXPECT(nytp_sink_get_state(NULL) == NYTP_SINK_UNINITIALIZED,
           "null state");
    EXPECT(strcmp(nytp_sink_name(NULL), "unknown") == 0, "null name");
}

static void test_counting_hot_path(void)
{
    nytp_sink *s = nytp_counting_sink_create();
    const nytp_counting_stats *st;
    EXPECT(s != NULL, "counting create");
    if (!s) {
        return;
    }

    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_OPEN, "open state");
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "activate");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_ACTIVE, "active state");
    EXPECT(strcmp(nytp_sink_name(s), "counting") == 0, "name");

    EXPECT(nytp_emit_time_line(s, 42, 1, 5) == NYTP_OK, "time_line");
    EXPECT(nytp_emit_time_block(s, 7, 1, 5, 4, 3) == NYTP_OK, "time_block");
    EXPECT(nytp_emit_discount(s) == NYTP_OK, "discount");
    EXPECT(nytp_emit_sub_return(s, 2, 1.0, 0.5, nytp_sv_cstr("main::leaf")) ==
               NYTP_OK,
           "sub_return");
    /* Fingerprint immediately after sub_return (before later emits overwrite). */
    st = nytp_counting_sink_stats(s);
    EXPECT(st != NULL, "stats after sub_return");
    if (st) {
        EXPECT(st->last_kind == NYTP_EVT_SUB_RETURN, "sub_return kind");
        EXPECT(st->last_depth == 2, "sub_return depth");
        EXPECT(st->last_subname_len == 10, "sub_return subname len");
        EXPECT(strcmp(st->last_subname, "main::leaf") == 0, "sub_return subname");
    }

    EXPECT(nytp_emit_sub_entry(s, 1, 10) == NYTP_OK, "sub_entry");
    EXPECT(nytp_emit_pid_start(s, 100, 1, 0.0) == NYTP_OK, "pid_start");
    EXPECT(nytp_emit_pid_end(s, 100, 1.0) == NYTP_OK, "pid_end");
    EXPECT(nytp_emit_attribute(s, nytp_sv_cstr("ticks_per_sec"),
                               nytp_sv_cstr("10000000")) == NYTP_OK,
           "attribute");
    EXPECT(nytp_emit_option(s, nytp_sv_cstr("calls"), nytp_sv_cstr("1")) ==
               NYTP_OK,
           "option");
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("hello")) == NYTP_OK, "comment");
    EXPECT(nytp_emit_start_deflate(s) == NYTP_OK, "start_deflate");

    st = nytp_counting_sink_stats(s);
    EXPECT(st != NULL, "stats");
    if (st) {
        EXPECT(st->by_kind[NYTP_EVT_TIME_LINE] == 1, "tl count");
        EXPECT(st->by_kind[NYTP_EVT_TIME_BLOCK] == 1, "tb count");
        EXPECT(st->by_kind[NYTP_EVT_DISCOUNT] == 1, "discount count");
        EXPECT(st->by_kind[NYTP_EVT_SUB_RETURN] == 1, "sub_return count");
        EXPECT(st->by_kind[NYTP_EVT_SUB_ENTRY] == 1, "sub_entry count");
        EXPECT(st->by_kind[NYTP_EVT_PID_START] == 1, "pid_start count");
        EXPECT(st->by_kind[NYTP_EVT_PID_END] == 1, "pid_end count");
        EXPECT(st->by_kind[NYTP_EVT_ATTRIBUTE] == 1, "attr count");
        EXPECT(st->by_kind[NYTP_EVT_OPTION] == 1, "opt count");
        EXPECT(st->by_kind[NYTP_EVT_COMMENT] == 1, "comment count");
        EXPECT(st->by_kind[NYTP_EVT_START_DEFLATE] == 1, "deflate count");
        EXPECT(st->total_emits == 11, "total emits");
        EXPECT(st->last_kind == NYTP_EVT_START_DEFLATE, "last kind");
    }

    /* Field routing on time_block last-write. */
    EXPECT(nytp_emit_time_block(s, 99, 2, 8, 7, 6) == NYTP_OK, "tb2");
    st = nytp_counting_sink_stats(s);
    if (st) {
        EXPECT(st->last_ticks == 99, "last ticks");
        EXPECT(st->last_fid == 2, "last fid");
        EXPECT(st->last_line == 8, "last line");
        EXPECT(st->last_block_line == 7, "last block_line");
        EXPECT(st->last_sub_line == 6, "last sub_line");
        EXPECT(st->by_kind[NYTP_EVT_TIME_BLOCK] == 2, "tb count 2");
    }

    EXPECT(nytp_sink_flush(s) == NYTP_OK, "flush");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_CLOSED, "closed");
    EXPECT(nytp_emit_time_line(s, 1, 1, 1) == NYTP_ERR_STATE,
           "emit after close");

    nytp_sink_destroy(s);
}

static void test_v5_stub_routing(void)
{
    nytp_sink *s = nytp_v5_sink_create("nytprof.out");
    const nytp_counting_stats *st;
    EXPECT(s != NULL, "v5 create");
    if (!s) {
        return;
    }

    EXPECT(nytp_v5_sink_is_v5(s), "is v5");
    EXPECT(strcmp(nytp_sink_name(s), "v5-stub") == 0, "v5 name");
    EXPECT(nytp_v5_sink_path(s) != NULL, "path set");
    if (nytp_v5_sink_path(s)) {
        EXPECT(strcmp(nytp_v5_sink_path(s), "nytprof.out") == 0, "path value");
    }

    EXPECT(nytp_sink_activate(s) == NYTP_OK, "v5 activate");

    /* Conceptual v5 route: same emit surface as production hooks will use. */
    EXPECT(nytp_emit_pid_start(s, 42, 1, 0.0) == NYTP_OK, "v5 pid_start");
    EXPECT(nytp_emit_new_fid(s, 1, 0, 0, 0, 0, 0,
                             nytp_sv_cstr("workload.pl")) == NYTP_OK,
           "v5 new_fid");
    EXPECT(nytp_emit_time_line(s, 10, 1, 5) == NYTP_OK, "v5 time_line");
    EXPECT(nytp_emit_discount(s) == NYTP_OK, "v5 discount");
    EXPECT(nytp_emit_sub_return(s, 1, 0.1, 0.05, nytp_sv_cstr("main::mid")) ==
               NYTP_OK,
           "v5 sub_return");
    EXPECT(nytp_emit_pid_end(s, 42, 1.0) == NYTP_OK, "v5 pid_end");

    st = nytp_v5_sink_stats(s);
    EXPECT(st != NULL, "v5 stats");
    if (st) {
        EXPECT(st->by_kind[NYTP_EVT_TIME_LINE] == 1, "v5 tl");
        EXPECT(st->by_kind[NYTP_EVT_DISCOUNT] == 1, "v5 discount");
        EXPECT(st->by_kind[NYTP_EVT_SUB_RETURN] == 1, "v5 return");
        EXPECT(st->by_kind[NYTP_EVT_PID_START] == 1, "v5 pstart");
        EXPECT(st->by_kind[NYTP_EVT_PID_END] == 1, "v5 pend");
        EXPECT(st->by_kind[NYTP_EVT_NEW_FID] == 1, "v5 new_fid");
        EXPECT(st->total_emits == 6, "v5 total");
        EXPECT(st->last_depth == 1, "v5 depth");
        EXPECT(strcmp(st->last_subname, "main::mid") == 0, "v5 subname");
    }

    /* Counting sink stats API must not accept a v5 sink. */
    EXPECT(nytp_counting_sink_stats(s) == NULL, "counting stats rejects v5");

    EXPECT(nytp_sink_close(s) == NYTP_OK, "v5 close");
    nytp_sink_destroy(s);
}

/*
 * Regression: type checks must use ops-pointer identity, never cast
 * counting_impl as v5_impl (ASAN OOB when magic lived at different offsets).
 */
static void test_type_safe_backend_identity(void)
{
    nytp_sink *c = nytp_counting_sink_create();
    nytp_sink *v = nytp_v5_sink_create("nytprof.out");
    EXPECT(c != NULL, "counting create");
    EXPECT(v != NULL, "v5 create");
    if (!c || !v) {
        if (c) {
            nytp_sink_destroy(c);
        }
        if (v) {
            nytp_sink_destroy(v);
        }
        return;
    }

    EXPECT(nytp_v5_sink_is_v5(v), "is_v5 accepts v5");
    EXPECT(!nytp_v5_sink_is_v5(c), "is_v5 rejects counting (no OOB)");
    EXPECT(!nytp_v5_sink_is_v5(NULL), "is_v5 rejects null");
    EXPECT(nytp_v5_sink_stats(c) == NULL, "v5 stats rejects counting");
    EXPECT(nytp_v5_sink_path(c) == NULL, "v5 path rejects counting");
    EXPECT(nytp_v5_sink_stats(v) != NULL, "v5 stats accepts v5");
    EXPECT(nytp_counting_sink_stats(c) != NULL, "counting stats accepts counting");
    EXPECT(nytp_counting_sink_stats(v) == NULL, "counting stats rejects v5");

    nytp_sink_destroy(c);
    nytp_sink_destroy(v);
}

static void test_event_kind_names(void)
{
    EXPECT(strcmp(nytp_event_kind_name(NYTP_EVT_TIME_LINE), "time_line") == 0,
           "kind time_line");
    EXPECT(strcmp(nytp_event_kind_name(NYTP_EVT_SUB_RETURN), "sub_return") == 0,
           "kind sub_return");
    EXPECT(strcmp(nytp_event_kind_name(NYTP_EVT_START_DEFLATE),
                  "start_deflate") == 0,
           "kind start_deflate");
}

static void test_inactive_before_activate(void)
{
    /* OPEN is emit-ready in this scaffold (lazy activate optional). */
    nytp_sink *s = nytp_counting_sink_create();
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    /* OPEN is considered ready for emit (legacy lazy init); activate is optional. */
    EXPECT(nytp_emit_discount(s) == NYTP_OK, "emit in OPEN");
    nytp_sink_destroy(s);
}

int main(void)
{
    test_null_sink_guards();
    test_counting_hot_path();
    test_v5_stub_routing();
    test_type_safe_backend_identity();
    test_event_kind_names();
    test_inactive_before_activate();

    if (failures != 0) {
        fprintf(stderr, "test_sink_api: %d failure(s)\n", failures);
        return 1;
    }
    printf("OK: test_sink_api (COL-001 sink interface)\n");
    return 0;
}

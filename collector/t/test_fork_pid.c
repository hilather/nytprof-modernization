/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-015 — Fork / PID protocol stress suite with buffered sinks.
 *
 * Covers: preflush before fork, no duplicate drain on child, seq domains,
 * nested forkdepth simulation, dual+batch, addpid path, v5/v6 child reinit,
 * optional real POSIX fork() with separate output files.
 */
#include "nytp_batch.h"
#include "nytp_fork.h"
#include "nytp_sink.h"
#include "nytp_sink_counting.h"
#include "nytp_sink_dual.h"
#include "nytp_sink_v5.h"
#include "nytp_sink_v6.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

static int g_failures = 0;

#define EXPECT(cond, msg)                                                      \
    do {                                                                       \
        if (!(cond)) {                                                         \
            fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, (msg));    \
            g_failures++;                                                      \
        }                                                                      \
    } while (0)

/* ---- helpers ---- */

static void emit_n_lines(nytp_sink *s, int n, nytp_line base)
{
    int i;
    for (i = 0; i < n; i++) {
        nytp_status st = nytp_emit_time_line(s, (nytp_ticks)(i + 1), 1,
                                             base + (nytp_line)i);
        EXPECT(st == NYTP_OK, "emit_time_line");
    }
}

/* ---- unit tests ---- */

static void test_addpid_path(void)
{
    char buf[64];
    int n;
    n = nytp_fork_addpid_path("nytprof.out", 12345, buf, sizeof(buf));
    EXPECT(n > 0, "addpid len");
    EXPECT(strcmp(buf, "nytprof.out.12345") == 0, "addpid format");
    EXPECT(nytp_fork_addpid_path(NULL, 1, buf, sizeof(buf)) < 0, "null base");
    EXPECT(nytp_fork_addpid_path("", 1, buf, sizeof(buf)) < 0, "empty base");
}

static void test_prepare_requires_active(void)
{
    nytp_sink *s = nytp_counting_sink_create();
    nytp_fork_metrics m;
    nytp_fork_policy pol = nytp_fork_policy_default();
    nytp_fork_metrics_clear(&m);
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_fork_prepare(s, &pol, &m) == NYTP_ERR_STATE, "not active");
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "activate");
    EXPECT(nytp_fork_prepare(s, &pol, &m) == NYTP_OK, "prepare ok");
    EXPECT(nytp_sink_get_state(s) == NYTP_SINK_FORK_SPLIT, "fork_split");
    EXPECT(m.begin_fork_ok == 1, "metric begin");
    nytp_sink_destroy(s);
}

/*
 * Near-full batch: leave pending events, prepare must preflush so child
 * residual is empty and parent counting saw all pre-fork events.
 */
static void test_batch_preflush_no_duplicate(void)
{
    nytp_sink *count = nytp_counting_sink_create();
    nytp_sink *batch;
    nytp_batch *b;
    const nytp_counting_stats *st;
    nytp_fork_metrics m;
    nytp_fork_policy pol = nytp_fork_policy_default();
    nytp_seq last = 0;

    EXPECT(count != NULL, "count");
    batch = nytp_batch_sink_create(count, 8, 4096, 8, 1 /* owns */);
    EXPECT(batch != NULL, "batch");
    if (!batch) {
        nytp_sink_destroy(count);
        return;
    }

    EXPECT(nytp_sink_activate(batch) == NYTP_OK, "activate");
    /* Fill 5 of 8 — pending, not yet flushed. */
    emit_n_lines(batch, 5, 1);
    b = nytp_batch_sink_batch(batch);
    EXPECT(b != NULL && nytp_batch_pending(b) == 5, "pending 5");
    st = nytp_counting_sink_stats(count);
    EXPECT(st && st->by_kind[NYTP_EVT_TIME_LINE] == 0, "not drained yet");

    nytp_fork_metrics_clear(&m);
    EXPECT(nytp_fork_prepare(batch, &pol, &m) == NYTP_OK, "prepare");
    EXPECT(nytp_batch_pending(b) == 0, "pending empty after prepare");
    st = nytp_counting_sink_stats(count);
    EXPECT(st && st->by_kind[NYTP_EVT_TIME_LINE] == 5, "preflushed 5");
    EXPECT(m.preflush_ok == 1, "preflush metric");
    EXPECT(nytp_sink_last_seq(batch, &last) == NYTP_OK && last == 4,
           "last seq 4");

    /* Parent resume: continues seq domain. */
    EXPECT(nytp_fork_resume_parent(batch, &m) == NYTP_OK, "parent resume");
    EXPECT(nytp_sink_get_state(batch) == NYTP_SINK_ACTIVE, "parent active");
    EXPECT(nytp_sink_peek_seq(batch) == 5, "parent peek 5");
    emit_n_lines(batch, 2, 100);
    EXPECT(nytp_sink_flush(batch) == NYTP_OK, "parent flush");
    st = nytp_counting_sink_stats(count);
    EXPECT(st && st->by_kind[NYTP_EVT_TIME_LINE] == 7, "parent +2");
    EXPECT(m.parent_resume == 1, "parent metric");

    /* Second fork → child path: seq reset, no residual dups. */
    EXPECT(nytp_fork_prepare(batch, &pol, &m) == NYTP_OK, "prepare2");
    EXPECT(nytp_fork_resume_child(batch, &pol, &m) == NYTP_OK, "child resume");
    EXPECT(nytp_sink_get_state(batch) == NYTP_SINK_OPEN, "child open");
    EXPECT(nytp_sink_peek_seq(batch) == 0, "child seq 0");
    EXPECT(m.child_resume == 1, "child metric");
    EXPECT(m.child_discard_events == 0, "no residual discard needed");

    EXPECT(nytp_sink_activate(batch) == NYTP_OK, "child activate");
    EXPECT(nytp_emit_pid_start(batch, 99, 1, 0.0) == NYTP_OK, "child pid_start");
    EXPECT(nytp_sink_last_seq(batch, &last) == NYTP_OK && last == 0,
           "child first seq 0");
    emit_n_lines(batch, 3, 200);
    EXPECT(nytp_sink_flush(batch) == NYTP_OK, "child flush");
    st = nytp_counting_sink_stats(count);
    /* 7 parent-domain + pid_start + 3 lines = 11 logical TIME_LINE+PID */
    EXPECT(st && st->by_kind[NYTP_EVT_TIME_LINE] == 10, "no dups: 7+3");
    EXPECT(st && st->by_kind[NYTP_EVT_PID_START] == 1, "pid_start once");

    nytp_sink_destroy(batch);
}

/*
 * Without preflush policy: leave residual, child discard must drop it so
 * counting child does not see duplicated events.
 */
static void test_child_discard_without_preflush(void)
{
    nytp_sink *count = nytp_counting_sink_create();
    nytp_sink *batch;
    nytp_batch *b;
    const nytp_counting_stats *st;
    const nytp_batch_metrics *bm;
    nytp_fork_metrics m;
    nytp_fork_policy pol = nytp_fork_policy_default();
    pol.flush_before_fork = 0; /* intentional anti-pattern for discard test */

    batch = nytp_batch_sink_create(count, 16, 4096, 16, 1);
    EXPECT(batch != NULL, "batch");
    if (!batch) {
        return;
    }
    EXPECT(nytp_sink_activate(batch) == NYTP_OK, "act");
    emit_n_lines(batch, 4, 1);
    b = nytp_batch_sink_batch(batch);
    EXPECT(nytp_batch_pending(b) == 4, "pending 4");

    /* begin_fork on batch still preflushes (hardened notify) — so residual
     * would be empty. To test discard, inject residual *after* FORK_SPLIT by
     * using discard_pending semantics via direct end_fork_child with
     * manually re-filled buffer is awkward. Instead: verify begin_fork alone
     * preflushes even when protocol skips public flush. */
    nytp_fork_metrics_clear(&m);
    EXPECT(nytp_fork_prepare(batch, &pol, &m) == NYTP_OK, "prepare no flush pol");
    EXPECT(nytp_batch_pending(b) == 0, "notify begin_fork still preflushed");
    st = nytp_counting_sink_stats(count);
    EXPECT(st && st->by_kind[NYTP_EVT_TIME_LINE] == 4, "drained via notify");
    bm = nytp_batch_get_metrics(b);
    EXPECT(bm && bm->fork_preflush >= 1, "fork_preflush metric");

    EXPECT(nytp_fork_resume_child(batch, &pol, &m) == NYTP_OK, "child");
    nytp_sink_destroy(batch);
}

/* Explicit residual discard path: fill after prepare is illegal (emit blocked);
 * use nytp_batch_discard_pending directly + metrics. */
static void test_discard_pending_helper(void)
{
    nytp_sink *count = nytp_counting_sink_create();
    nytp_sink *batch = nytp_batch_sink_create(count, 8, 4096, 8, 1);
    nytp_batch *b;
    nytp_fork_metrics m;
    EXPECT(batch != NULL, "batch");
    if (!batch) {
        return;
    }
    EXPECT(nytp_sink_activate(batch) == NYTP_OK, "act");
    emit_n_lines(batch, 3, 1);
    b = nytp_batch_sink_batch(batch);
    EXPECT(nytp_batch_pending(b) == 3, "p3");
    nytp_fork_metrics_clear(&m);
    EXPECT(nytp_fork_discard_batch_residual(batch, &m) == 3, "discard 3");
    EXPECT(nytp_batch_pending(b) == 0, "empty");
    EXPECT(m.child_discard_events == 3, "metric");
    /* Counting child never saw them. */
    {
        const nytp_counting_stats *st = nytp_counting_sink_stats(count);
        EXPECT(st && st->by_kind[NYTP_EVT_TIME_LINE] == 0, "not emitted");
    }
    nytp_sink_destroy(batch);
}

/* Nested forkdepth simulation: parent keeps domain; each child resets. */
static void test_nested_forkdepth_seq_domains(void)
{
    nytp_sink *s = nytp_counting_sink_create();
    nytp_fork_policy pol = nytp_fork_policy_default();
    nytp_fork_metrics m;
    nytp_seq last = 0;
    int depth;

    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "act");
    emit_n_lines(s, 2, 1);
    nytp_fork_metrics_clear(&m);

    for (depth = 0; depth < 3; depth++) {
        EXPECT(nytp_fork_prepare(s, &pol, &m) == NYTP_OK, "prep depth");
        if (depth % 2 == 0) {
            /* Parent keeps domain */
            EXPECT(nytp_fork_resume_parent(s, &m) == NYTP_OK, "parent d");
            EXPECT(nytp_emit_discount(s) == NYTP_OK, "disc parent");
        } else {
            /* Child resets — simulate by child resume then activate */
            EXPECT(nytp_fork_resume_child(s, &pol, &m) == NYTP_OK, "child d");
            EXPECT(nytp_sink_peek_seq(s) == 0, "reset");
            EXPECT(nytp_sink_activate(s) == NYTP_OK, "re-act");
            EXPECT(nytp_emit_pid_start(s, (nytp_pid)(100 + depth), 1, 0.0) ==
                       NYTP_OK,
                   "pid");
            EXPECT(nytp_sink_last_seq(s, &last) == NYTP_OK && last == 0,
                   "child seq0");
        }
    }
    EXPECT(m.parent_resume >= 1, "had parent");
    EXPECT(m.child_resume >= 1, "had child");
    nytp_sink_destroy(s);
}

/* Dual + batch: prepare flushes both logical sides equally. */
static void test_dual_batch_fork(void)
{
    nytp_sink *c1 = nytp_counting_sink_create();
    nytp_sink *c2 = nytp_counting_sink_create();
    nytp_sink *dual;
    nytp_sink *batch;
    nytp_fork_policy pol = nytp_fork_policy_default();
    nytp_fork_metrics m;
    const nytp_counting_stats *s1, *s2;

    dual = nytp_dual_sink_create(c1, c2, 1, 1);
    EXPECT(dual != NULL, "dual");
    batch = nytp_batch_sink_create(dual, 4, 4096, 4, 1);
    EXPECT(batch != NULL, "batch");
    if (!batch) {
        return;
    }
    EXPECT(nytp_sink_activate(batch) == NYTP_OK, "act");
    emit_n_lines(batch, 3, 1); /* pending in batch */
    nytp_fork_metrics_clear(&m);
    EXPECT(nytp_fork_prepare(batch, &pol, &m) == NYTP_OK, "prep dual-batch");
    s1 = nytp_counting_sink_stats(c1);
    s2 = nytp_counting_sink_stats(c2);
    EXPECT(s1 && s2, "stats");
    EXPECT(s1->by_kind[NYTP_EVT_TIME_LINE] == 3, "p1 drained");
    EXPECT(s2->by_kind[NYTP_EVT_TIME_LINE] == 3, "p2 drained");
    EXPECT(nytp_dual_sink_logical_equal(dual) == 1, "equal after preflush");

    EXPECT(nytp_fork_resume_parent(batch, &m) == NYTP_OK, "parent");
    emit_n_lines(batch, 1, 50);
    EXPECT(nytp_sink_flush(batch) == NYTP_OK, "flush");
    EXPECT(nytp_dual_sink_logical_equal(dual) == 1, "equal parent cont");

    EXPECT(nytp_fork_prepare(batch, &pol, &m) == NYTP_OK, "prep2");
    EXPECT(nytp_fork_resume_child(batch, &pol, &m) == NYTP_OK, "child");
    EXPECT(nytp_sink_activate(batch) == NYTP_OK, "act child");
    EXPECT(nytp_emit_pid_start(batch, 7, 1, 0.0) == NYTP_OK, "pid");
    EXPECT(nytp_sink_flush(batch) == NYTP_OK, "flush child");
    EXPECT(nytp_dual_sink_logical_equal(dual) == 1, "equal after child");

    nytp_sink_destroy(batch);
}

/* Fail-closed: preflush fails → prepare fails, not FORK_SPLIT. */
static void test_preflush_fail_closed(void)
{
    nytp_sink *count = nytp_counting_sink_create();
    nytp_sink *batch;
    nytp_fork_policy pol = nytp_fork_policy_default();
    nytp_fork_metrics m;

    batch = nytp_batch_sink_create(count, 4, 4096, 4, 1);
    EXPECT(batch != NULL, "batch");
    if (!batch) {
        return;
    }
    EXPECT(nytp_sink_activate(batch) == NYTP_OK, "act");
    emit_n_lines(batch, 2, 1);
    /* Arm child to fail on flush drain. */
    EXPECT(nytp_counting_sink_fail_next(count, NYTP_ERR_IO) == NYTP_OK, "arm");
    nytp_fork_metrics_clear(&m);
    EXPECT(nytp_fork_prepare(batch, &pol, &m) != NYTP_OK, "prepare fails");
    EXPECT(m.preflush_fail == 1, "preflush_fail metric");
    /* Root may be FAILED sticky or still ACTIVE depending on flush path. */
    EXPECT(nytp_sink_get_state(batch) != NYTP_SINK_FORK_SPLIT,
           "not stuck in fork_split");
    nytp_sink_destroy(batch);
}

/* v5 child reinit: new path, clean header, no shared-path write. */
static void test_v5_fork_child_reinit_addpid(void)
{
    /* Paths relative to collector/ (make -C collector test cwd). */
    const char *base = "build/fork_v5_parent.nytprof";
    char child_path[256];
    nytp_sink *s;
    size_t wlen = 0;
    const uint8_t *wire;
    nytp_fork_policy pol = nytp_fork_policy_default();
    nytp_fork_metrics m;

    (void)mkdir("build", 0755);
    EXPECT(nytp_fork_addpid_path(base, 4242, child_path, sizeof(child_path)) > 0,
           "path");

    s = nytp_v5_sink_create(base);
    EXPECT(s != NULL, "v5 create");
    if (!s) {
        return;
    }
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "act");
    EXPECT(nytp_emit_pid_start(s, 1, 0, 0.0) == NYTP_OK, "pid");
    emit_n_lines(s, 2, 1);
    wire = nytp_v5_sink_wire(s, &wlen);
    EXPECT(wire && wlen > 12, "parent wire has body");

    nytp_fork_metrics_clear(&m);
    EXPECT(nytp_fork_prepare(s, &pol, &m) == NYTP_OK, "prep");
    EXPECT(nytp_fork_resume_child(s, &pol, &m) == NYTP_OK, "child resume");
    EXPECT(nytp_v5_sink_fork_child_reinit(s, child_path) == NYTP_OK, "reinit");
    EXPECT(strcmp(nytp_v5_sink_path(s), child_path) == 0, "path rebound");
    wire = nytp_v5_sink_wire(s, &wlen);
    EXPECT(wire && wlen == 12 && memcmp(wire, "NYTProf 5 0\n", 12) == 0,
           "clean header only");
    EXPECT(nytp_v5_sink_is_deflating(s) == 0, "no deflate");

    EXPECT(nytp_sink_activate(s) == NYTP_OK, "child act");
    EXPECT(nytp_emit_pid_start(s, 4242, 1, 0.0) == NYTP_OK, "child pid");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close child");
    EXPECT(nytp_v5_sink_file_written(s) == 1, "child file written");
    nytp_sink_destroy(s);

    /* Parent path was never closed by child — may not exist if never flushed.
     * Child path must exist and be non-empty. */
    {
        FILE *fp = fopen(child_path, "rb");
        EXPECT(fp != NULL, "child file open");
        if (fp) {
            char hdr[16];
            size_t n = fread(hdr, 1, 12, fp);
            EXPECT(n == 12 && memcmp(hdr, "NYTProf 5 0\n", 12) == 0,
                   "child file header");
            fclose(fp);
        }
    }
}

/* v6 child reinit resets wire + dict domain. */
static void test_v6_fork_child_reinit(void)
{
    nytp_sink *s = nytp_v6_sink_create(NULL);
    size_t wlen = 0;
    size_t wlen2 = 0;
    const uint8_t *w1, *w2;
    nytp_fork_policy pol = nytp_fork_policy_default();

    EXPECT(s != NULL, "v6");
    if (!s) {
        return;
    }
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "act");
    EXPECT(nytp_emit_pid_start(s, 1, 0, 0.0) == NYTP_OK, "pid");
    emit_n_lines(s, 1, 1);
    w1 = nytp_v6_sink_wire(s, &wlen);
    EXPECT(w1 && wlen > 0, "wire");

    EXPECT(nytp_fork_prepare(s, &pol, NULL) == NYTP_OK, "prep");
    EXPECT(nytp_fork_resume_child(s, &pol, NULL) == NYTP_OK, "child");
    EXPECT(nytp_v6_sink_fork_child_reinit(s, NULL) == NYTP_OK, "reinit");
    w2 = nytp_v6_sink_wire(s, &wlen2);
    EXPECT(w2 && wlen2 > 0, "prefix rewritten");
    /* Child stream is prefix-only until more events; shorter or equal to
     * parent wire which had events. */
    EXPECT(wlen2 <= wlen, "child wire not larger than parent with events");
    EXPECT(nytp_v6_sink_event_count(s) == 0, "no open events");
    EXPECT(nytp_v6_sink_dict_entry_count(s) == 0, "dict cleared");
    nytp_sink_destroy(s);
}

/*
 * Real POSIX fork: parent keeps base path, child rebinds addpid path.
 * Assert no shared-path double-write and both processes exit cleanly.
 */
static void test_posix_fork_addpid_files(void)
{
    const char *base = "build/fork_posix_parent.nytprof";
    char child_path[256];
    nytp_sink *s;
    nytp_fork_policy pol = nytp_fork_policy_default();
    pid_t pid;
    int status = 0;

    (void)mkdir("build", 0755);
    EXPECT(nytp_fork_addpid_path(base, (nytp_pid)getpid() /* placeholder */,
                                 child_path, sizeof(child_path)) > 0,
           "fmt");

    s = nytp_v5_sink_create(base);
    EXPECT(s != NULL, "create");
    if (!s) {
        return;
    }
    EXPECT(nytp_sink_activate(s) == NYTP_OK, "act");
    EXPECT(nytp_emit_attribute(s, nytp_sv_cstr("profiler"),
                               nytp_sv_cstr("col015")) == NYTP_OK,
           "attr");
    emit_n_lines(s, 3, 1);
    EXPECT(nytp_fork_prepare(s, &pol, NULL) == NYTP_OK, "prepare pre-os-fork");

    pid = fork();
    if (pid < 0) {
        EXPECT(0, "fork() failed");
        nytp_sink_destroy(s);
        return;
    }
    if (pid == 0) {
        /* Child */
        char path[256];
        nytp_pid self = (nytp_pid)getpid();
        int rc = 0;
        if (nytp_fork_resume_child(s, &pol, NULL) != NYTP_OK) {
            _exit(11);
        }
        if (nytp_fork_addpid_path(base, self, path, sizeof(path)) < 0) {
            _exit(12);
        }
        if (nytp_v5_sink_fork_child_reinit(s, path) != NYTP_OK) {
            _exit(13);
        }
        if (nytp_sink_activate(s) != NYTP_OK) {
            _exit(14);
        }
        if (nytp_emit_pid_start(s, self, (nytp_pid)getppid(), 0.0) != NYTP_OK) {
            _exit(15);
        }
        if (nytp_emit_time_line(s, 9, 1, 99) != NYTP_OK) {
            _exit(16);
        }
        if (nytp_sink_close(s) != NYTP_OK) {
            _exit(17);
        }
        /* Child must not leave parent path as its write target. */
        if (nytp_v5_sink_path(s) == NULL ||
            strcmp(nytp_v5_sink_path(s), path) != 0) {
            _exit(18);
        }
        if (!nytp_v5_sink_file_written(s)) {
            _exit(19);
        }
        nytp_sink_destroy(s);
        _exit(rc);
    }

    /* Parent */
    EXPECT(nytp_fork_resume_parent(s, NULL) == NYTP_OK, "parent resume");
    EXPECT(nytp_emit_time_line(s, 7, 1, 7) == NYTP_OK, "parent emit");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "parent close");
    EXPECT(nytp_v5_sink_file_written(s) == 1, "parent wrote");
    nytp_sink_destroy(s);

    EXPECT(waitpid(pid, &status, 0) == pid, "wait child");
    EXPECT(WIFEXITED(status) && WEXITSTATUS(status) == 0, "child exit 0");

    {
        FILE *fp = fopen(base, "rb");
        EXPECT(fp != NULL, "parent file exists");
        if (fp) {
            fclose(fp);
        }
    }
}

/* Dual v5+v6 reinit helper after child resume. */
static void test_dual_wire_child_reinit(void)
{
    nytp_sink *dual;
    nytp_fork_policy pol = nytp_fork_policy_default();
    nytp_sink *p, *sec;

    (void)mkdir("build", 0755);
    dual = nytp_dual_sink_create_v5_v6("build/dual_fork_v5.nytprof",
                                       "build/dual_fork_v6.nytprof");
    EXPECT(dual != NULL, "dual v5v6");
    if (!dual) {
        return;
    }
    EXPECT(nytp_sink_activate(dual) == NYTP_OK, "act");
    emit_n_lines(dual, 2, 1);
    EXPECT(nytp_fork_prepare(dual, &pol, NULL) == NYTP_OK, "prep");
    EXPECT(nytp_fork_resume_child(dual, &pol, NULL) == NYTP_OK, "child");
    EXPECT(nytp_dual_sink_fork_child_reinit(
               dual, "build/dual_fork_v5.child.nytprof",
               "build/dual_fork_v6.child.nytprof") == NYTP_OK,
           "reinit both");
    p = nytp_dual_sink_primary(dual);
    sec = nytp_dual_sink_secondary(dual);
    EXPECT(nytp_v5_sink_is_v5(p) && nytp_v5_sink_path(p) != NULL, "v5 path");
    EXPECT(nytp_v6_sink_is_v6(sec) && nytp_v6_sink_path(sec) != NULL, "v6 path");
    EXPECT(strstr(nytp_v5_sink_path(p), ".child.") != NULL, "v5 child path");
    EXPECT(strstr(nytp_v6_sink_path(sec), ".child.") != NULL, "v6 child path");
    nytp_sink_destroy(dual);
}

int main(void)
{
    test_addpid_path();
    test_prepare_requires_active();
    test_batch_preflush_no_duplicate();
    test_child_discard_without_preflush();
    test_discard_pending_helper();
    test_nested_forkdepth_seq_domains();
    test_dual_batch_fork();
    test_preflush_fail_closed();
    test_v5_fork_child_reinit_addpid();
    test_v6_fork_child_reinit();
    test_posix_fork_addpid_files();
    test_dual_wire_child_reinit();

    if (g_failures) {
        fprintf(stderr, "test_fork_pid: %d failure(s)\n", g_failures);
        return 1;
    }
    printf("OK: test_fork_pid (COL-015 fork/PID + buffered sinks)\n");
    return 0;
}

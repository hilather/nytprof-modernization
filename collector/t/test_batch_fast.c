/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-004 / COL-005 unit tests:
 *   - no-alloc statement fast path (POD append + metrics)
 *   - exact order under forced capacities 1..64
 *   - SV / string lifetime (overwrite caller buffer after append)
 *   - emergency oversized payload path
 *   - light microbench (engineering counters only)
 *
 * Build/run: make -C collector test
 */
#include "nytp_batch.h"
#include "nytp_sink.h"
#include "nytp_sink_counting.h"

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

/* Drive a mixed stream into sink; returns logical event count expected. */
static size_t drive_mixed(nytp_sink *s, int n_stmt)
{
    size_t logical = 0;
    int i;
    EXPECT(nytp_emit_attribute(s, nytp_sv_cstr("ticks_per_sec"),
                               nytp_sv_cstr("10000000")) == NYTP_OK,
           "attr");
    logical++;
    EXPECT(nytp_emit_option(s, nytp_sv_cstr("calls"), nytp_sv_cstr("1")) ==
               NYTP_OK,
           "opt");
    logical++;
    EXPECT(nytp_emit_start_deflate(s) == NYTP_OK, "deflate");
    /* control: not logical */
    EXPECT(nytp_emit_pid_start(s, 100, 1, 0.0) == NYTP_OK, "pid_start");
    logical++;
    EXPECT(nytp_emit_new_fid(s, 1, 0, 0, 0, 0, 0, nytp_sv_cstr("t.pl")) ==
               NYTP_OK,
           "new_fid");
    logical++;
    for (i = 0; i < n_stmt; i++) {
        EXPECT(nytp_emit_time_line(s, (nytp_ticks)(10 + i), 1,
                                   (nytp_line)(i + 1)) == NYTP_OK,
               "time_line loop");
        logical++;
        if ((i % 3) == 2) {
            EXPECT(nytp_emit_discount(s) == NYTP_OK, "discount");
            logical++;
        }
        if ((i % 5) == 4) {
            EXPECT(nytp_emit_time_block(s, 3, 1, (nytp_line)(i + 1), 1, 1) ==
                       NYTP_OK,
                   "time_block");
            logical++;
        }
        if ((i % 7) == 6) {
            EXPECT(nytp_emit_sub_entry(s, 1, (nytp_line)(i + 1)) == NYTP_OK,
                   "sub_entry");
            logical++;
            EXPECT(nytp_emit_sub_return(s, 1, 0.1, 0.05,
                                        nytp_sv_cstr("main::f")) == NYTP_OK,
                   "sub_return");
            logical++;
        }
    }
    EXPECT(nytp_emit_comment(s, nytp_sv_cstr("end")) == NYTP_OK, "comment");
    logical++;
    EXPECT(nytp_sink_begin_finalize(s) == NYTP_OK, "finalize");
    EXPECT(nytp_emit_src_line(s, 1, 1, nytp_sv_cstr("print 1;")) == NYTP_OK,
           "src_line");
    logical++;
    EXPECT(nytp_emit_sub_info(s, 1, 1, 10, nytp_sv_cstr("main::f")) == NYTP_OK,
           "sub_info");
    logical++;
    EXPECT(nytp_emit_pid_end(s, 100, 1.0) == NYTP_OK, "pid_end");
    logical++;
    EXPECT(nytp_sink_flush(s) == NYTP_OK, "flush");
    EXPECT(nytp_sink_close(s) == NYTP_OK, "close");
    return logical;
}

static void test_order_vs_direct(size_t capacity)
{
    nytp_sink *direct;
    nytp_sink *child;
    nytp_sink *batched;
    const nytp_counting_stats *sd;
    const nytp_counting_stats *sb;
    nytp_seq seq_d[NYTP_COUNTING_SEQ_RING];
    nytp_seq seq_b[NYTP_COUNTING_SEQ_RING];
    nytp_event_kind kind_d[NYTP_COUNTING_SEQ_RING];
    nytp_event_kind kind_b[NYTP_COUNTING_SEQ_RING];
    size_t nd, nb, i;
    size_t logical;
    char label[64];

    snprintf(label, sizeof(label), "cap=%zu", capacity);

    direct = nytp_counting_sink_create();
    EXPECT(direct != NULL, "direct create");
    EXPECT(nytp_sink_activate(direct) == NYTP_OK, "direct activate");
    logical = drive_mixed(direct, 40);

    child = nytp_counting_sink_create();
    EXPECT(child != NULL, "child create");
    batched = nytp_batch_sink_create(child, capacity, 2048, capacity, 1);
    EXPECT(batched != NULL, "batch create");
    EXPECT(nytp_sink_activate(batched) == NYTP_OK, "batch activate");
    {
        size_t logical_b = drive_mixed(batched, 40);
        EXPECT(logical_b == logical, "same logical count");
    }

    sd = nytp_counting_sink_stats(direct);
    /* Child is owned by batch; stats via child's pointer — need re-get.
     * Child was destroyed with batch? owns_child=1, but we still hold child
     * pointer only until destroy. Stats must be read before destroy.
     * drive_mixed already closed; destroy not yet. Child still alive. */
    sb = nytp_counting_sink_stats(child);
    EXPECT(sd != NULL && sb != NULL, "stats");
    if (sd && sb) {
        EXPECT(sd->logical_emits == sb->logical_emits, "logical_emits equal");
        EXPECT(sd->logical_emits == logical, "logical matches drive");
        EXPECT(sd->by_kind[NYTP_EVT_TIME_LINE] ==
                   sb->by_kind[NYTP_EVT_TIME_LINE],
               "TIME_LINE mult");
        EXPECT(sd->by_kind[NYTP_EVT_TIME_BLOCK] ==
                   sb->by_kind[NYTP_EVT_TIME_BLOCK],
               "TIME_BLOCK mult");
        EXPECT(sd->by_kind[NYTP_EVT_DISCOUNT] == sb->by_kind[NYTP_EVT_DISCOUNT],
               "DISCOUNT mult");
        EXPECT(sd->by_kind[NYTP_EVT_START_DEFLATE] ==
                   sb->by_kind[NYTP_EVT_START_DEFLATE],
               "START_DEFLATE mult");
    }

    nd = NYTP_COUNTING_SEQ_RING;
    nb = NYTP_COUNTING_SEQ_RING;
    EXPECT(nytp_counting_sink_copy_seqs(direct, seq_d, &nd) == NYTP_OK,
           "copy seqs direct");
    EXPECT(nytp_counting_sink_copy_seqs(child, seq_b, &nb) == NYTP_OK,
           "copy seqs batch");
    EXPECT(nd == nb, "seq ring len equal");
    if (nd == nb) {
        EXPECT(nytp_seq_check_gapless(seq_d, nd, 0, NULL), "direct gapless");
        EXPECT(nytp_seq_check_gapless(seq_b, nb, 0, NULL), "batch gapless");
        for (i = 0; i < nd; i++) {
            if (seq_d[i] != seq_b[i]) {
                fprintf(stderr, "FAIL: %s seq mismatch at %zu: %llu vs %llu\n",
                        label, i, (unsigned long long)seq_d[i],
                        (unsigned long long)seq_b[i]);
                failures++;
                break;
            }
        }
    }

    nd = NYTP_COUNTING_SEQ_RING;
    nb = NYTP_COUNTING_SEQ_RING;
    EXPECT(nytp_counting_sink_copy_kinds(direct, kind_d, &nd) == NYTP_OK,
           "copy kinds direct");
    EXPECT(nytp_counting_sink_copy_kinds(child, kind_b, &nb) == NYTP_OK,
           "copy kinds batch");
    EXPECT(nd == nb, "kind ring len equal");
    if (nd == nb) {
        for (i = 0; i < nd; i++) {
            if (kind_d[i] != kind_b[i]) {
                fprintf(stderr,
                        "FAIL: %s kind mismatch at %zu: %s vs %s\n", label, i,
                        nytp_event_kind_name(kind_d[i]),
                        nytp_event_kind_name(kind_b[i]));
                failures++;
                break;
            }
        }
    }

    nytp_sink_destroy(direct);
    nytp_sink_destroy(batched);
}

static void test_capacity_stress(void)
{
    static const size_t caps[] = {1, 2, 3, 4, 8, 16, 32, 64};
    size_t i;
    for (i = 0; i < sizeof(caps) / sizeof(caps[0]); i++) {
        test_order_vs_direct(caps[i]);
    }
}

/* SV lifetime: caller buffer overwritten after emit; flush must still see original. */
static void test_sv_lifetime(void)
{
    nytp_sink *child;
    nytp_sink *batch;
    const nytp_counting_stats *st;
    char buf[64];
    nytp_string_view sv;

    child = nytp_counting_sink_create();
    batch = nytp_batch_sink_create(child, 8, 512, 8, 1);
    EXPECT(batch != NULL, "batch");
    EXPECT(nytp_sink_activate(batch) == NYTP_OK, "activate");

    memcpy(buf, "original-source-line", 20);
    buf[20] = '\0';
    sv = nytp_sv(buf, 20, 0);
    EXPECT(nytp_emit_src_line(batch, 1, 7, sv) == NYTP_OK, "src_line append");

    /* Mutate caller buffer (simulates Perl SV reuse / FREETMPS). */
    memset(buf, 'X', sizeof(buf));

    /* Also mutate a subname after sub_return. */
    {
        char name[32];
        memcpy(name, "main::alive", 11);
        name[11] = '\0';
        EXPECT(nytp_emit_sub_return(batch, 1, 1.0, 0.5, nytp_sv(name, 11, 0)) ==
                   NYTP_OK,
               "sub_return");
        memset(name, 'Y', sizeof(name));
    }

    EXPECT(nytp_sink_flush(batch) == NYTP_OK, "flush after clobber");

    st = nytp_counting_sink_stats(child);
    EXPECT(st != NULL, "stats");
    if (st) {
        EXPECT(st->by_kind[NYTP_EVT_SRC_LINE] == 1, "src_line counted");
        EXPECT(st->by_kind[NYTP_EVT_SUB_RETURN] == 1, "sub_return counted");
        EXPECT(st->last_src_text_len == 20, "src text len preserved");
        EXPECT(strcmp(st->last_src_text, "original-source-line") == 0,
               "src text preserved after caller buffer clobber (no UAF)");
        EXPECT(st->last_src_line == 7, "src line number");
        EXPECT(st->last_subname_len == 11, "subname len preserved");
        EXPECT(strcmp(st->last_subname, "main::alive") == 0,
               "subname bytes preserved after clobber");
    }

    /* Attribute path too. */
    {
        char k[16], v[16];
        memcpy(k, "application", 11);
        k[11] = '\0';
        memcpy(v, "demo-app", 8);
        v[8] = '\0';
        EXPECT(nytp_emit_attribute(batch, nytp_sv(k, 11, 0),
                                   nytp_sv(v, 8, 0)) == NYTP_OK,
               "attr");
        memset(k, 'Z', sizeof(k));
        memset(v, 'Z', sizeof(v));
        EXPECT(nytp_sink_flush(batch) == NYTP_OK, "flush attr");
        st = nytp_counting_sink_stats(child);
        EXPECT(st && st->by_kind[NYTP_EVT_ATTRIBUTE] == 1, "attr counted");
    }

    nytp_sink_destroy(batch);
}

/* Statement path: no heap growth after create; POD only. */
static void test_stmt_fast_no_alloc(void)
{
    nytp_sink *child;
    nytp_sink *batch;
    nytp_batch *b;
    const nytp_batch_metrics *m;
    uint64_t heap_at_start;
    int i;

    child = nytp_counting_sink_create();
    batch = nytp_batch_sink_create(child, 16, 256, 8, 1);
    EXPECT(batch != NULL, "batch");
    EXPECT(nytp_sink_activate(batch) == NYTP_OK, "activate");
    b = nytp_batch_sink_batch(batch);
    EXPECT(b != NULL, "batch ptr");
    m = nytp_batch_get_metrics(b);
    EXPECT(m != NULL, "metrics");
    heap_at_start = m->heap_allocs;

    for (i = 0; i < 200; i++) {
        EXPECT(nytp_fast_emit_time_line(batch, (nytp_ticks)i, 1,
                                        (nytp_line)(i + 1)) == NYTP_OK,
               "fast time_line");
        if ((i % 4) == 3) {
            EXPECT(nytp_fast_emit_time_block(batch, 2, 1, (nytp_line)(i + 1), 1,
                                             1) == NYTP_OK,
                   "fast time_block");
        }
    }
    EXPECT(nytp_sink_flush(batch) == NYTP_OK, "flush");

    m = nytp_batch_get_metrics(b);
    EXPECT(m->heap_allocs == heap_at_start,
           "no heap_allocs after create on stmt path");
    EXPECT(m->stmt_fast_appends == 200 + 50, "stmt_fast_appends count");
    EXPECT(m->arena_bytes_copied == 0, "stmt path uses no arena");
    EXPECT(sizeof(nytp_event) >= 32, "event POD has useful size");
    /* Bounded: capacity fixed. */
    EXPECT(nytp_batch_capacity(b) == 16, "capacity fixed");

    {
        const nytp_counting_stats *st = nytp_counting_sink_stats(child);
        EXPECT(st != NULL, "child stats");
        if (st) {
            EXPECT(st->by_kind[NYTP_EVT_TIME_LINE] == 200, "tl drained");
            EXPECT(st->by_kind[NYTP_EVT_TIME_BLOCK] == 50, "tb drained");
            EXPECT(nytp_seq_check_gapless(st->seq_ring, st->seq_ring_len, 0,
                                          NULL) ||
                       st->seq_ring_len == NYTP_COUNTING_SEQ_RING,
                   "gapless or full ring");
        }
    }

    nytp_sink_destroy(batch);
}

/* High-water vs capacity: flush before exhaustion. */
static void test_high_water(void)
{
    nytp_sink *child;
    nytp_sink *batch;
    nytp_batch *b;
    const nytp_batch_metrics *m;
    int i;

    child = nytp_counting_sink_create();
    /* capacity 8, high_water 3 */
    batch = nytp_batch_sink_create(child, 8, 256, 3, 1);
    EXPECT(batch != NULL, "batch");
    EXPECT(nytp_sink_activate(batch) == NYTP_OK, "activate");
    for (i = 0; i < 9; i++) {
        EXPECT(nytp_emit_time_line(batch, i + 1, 1, (nytp_line)(i + 1)) ==
                   NYTP_OK,
               "tl");
    }
    b = nytp_batch_sink_batch(batch);
    m = nytp_batch_get_metrics(b);
    EXPECT(m->high_water_flushes >= 1, "high_water flushes occurred");
    EXPECT(nytp_batch_count(b) < 3 || nytp_batch_count(b) == 0,
           "pending below high_water or empty after exact multiple");
    EXPECT(nytp_sink_flush(batch) == NYTP_OK, "final flush");
    {
        const nytp_counting_stats *st = nytp_counting_sink_stats(child);
        EXPECT(st && st->by_kind[NYTP_EVT_TIME_LINE] == 9, "all drained");
    }
    nytp_sink_destroy(batch);
}

/* Oversized string: emergency direct path. */
static void test_emergency_oversized(void)
{
    nytp_sink *child;
    nytp_sink *batch;
    nytp_batch *b;
    const nytp_batch_metrics *m;
    char big[512];
    memset(big, 'A', sizeof(big));

    child = nytp_counting_sink_create();
    /* tiny arena 64 — 512-byte payload exceeds */
    batch = nytp_batch_sink_create(child, 4, 64, 4, 1);
    EXPECT(batch != NULL, "batch");
    EXPECT(nytp_sink_activate(batch) == NYTP_OK, "activate");
    EXPECT(nytp_emit_src_line(batch, 1, 1, nytp_sv(big, sizeof(big), 0)) ==
               NYTP_OK,
           "oversized src_line");
    b = nytp_batch_sink_batch(batch);
    m = nytp_batch_get_metrics(b);
    EXPECT(m->emergency_direct >= 1, "emergency path taken");
    {
        const nytp_counting_stats *st = nytp_counting_sink_stats(child);
        EXPECT(st && st->by_kind[NYTP_EVT_SRC_LINE] == 1, "src_line delivered");
    }
    nytp_sink_destroy(batch);
}

/* Fast path equals public emit for seq/order. */
static void test_fast_equals_public(void)
{
    nytp_sink *c1, *c2;
    nytp_sink *b1, *b2;
    const nytp_counting_stats *s1, *s2;
    int i;

    c1 = nytp_counting_sink_create();
    c2 = nytp_counting_sink_create();
    b1 = nytp_batch_sink_create(c1, 4, 128, 4, 1);
    b2 = nytp_batch_sink_create(c2, 4, 128, 4, 1);
    EXPECT(nytp_sink_activate(b1) == NYTP_OK &&
               nytp_sink_activate(b2) == NYTP_OK,
           "activate");
    for (i = 0; i < 20; i++) {
        EXPECT(nytp_emit_time_line(b1, i, 1, (nytp_line)(i + 1)) == NYTP_OK,
               "public");
        EXPECT(nytp_fast_emit_time_line(b2, i, 1, (nytp_line)(i + 1)) ==
                   NYTP_OK,
               "fast");
    }
    EXPECT(nytp_sink_flush(b1) == NYTP_OK && nytp_sink_flush(b2) == NYTP_OK,
           "flush");
    s1 = nytp_counting_sink_stats(c1);
    s2 = nytp_counting_sink_stats(c2);
    EXPECT(s1 && s2, "stats");
    if (s1 && s2) {
        EXPECT(s1->logical_emits == s2->logical_emits, "logical equal");
        EXPECT(s1->by_kind[NYTP_EVT_TIME_LINE] ==
                   s2->by_kind[NYTP_EVT_TIME_LINE],
               "tl equal");
        EXPECT(s1->last_ticks == s2->last_ticks, "last ticks");
        EXPECT(s1->last_line == s2->last_line, "last line");
    }
    EXPECT(nytp_sink_logical_count(b1) == nytp_sink_logical_count(b2),
           "seq count equal");
    nytp_sink_destroy(b1);
    nytp_sink_destroy(b2);
}

static void test_microbench_light(void)
{
    nytp_fast_bench_result r;
    nytp_status st = nytp_fast_bench_time_line(16, 5000, &r);
    EXPECT(st == NYTP_OK, "bench ok");
    EXPECT(r.iterations == 5000, "iters");
    EXPECT(r.stmt_fast_appends == 5000, "stmt appends");
    EXPECT(r.event_sizeof == sizeof(nytp_event), "sizeof event");
    /* elapsed_ns may be 0 on exotic clocks; do not fail. */
    fprintf(stderr,
            "NOTE: light microbench TIME_LINE x%llu cap=%zu sizeof(event)=%zu "
            "elapsed_ns=%llu (engineering only; not BENCH certification)\n",
            (unsigned long long)r.iterations, r.batch_capacity, r.event_sizeof,
            (unsigned long long)r.elapsed_ns);
}

static void test_failed_flush_preserves_order_state(void)
{
    /* Fail-next on child after partial buffer: batch stays FAILED path. */
    nytp_sink *child;
    nytp_sink *batch;
    const nytp_counting_stats *st;

    child = nytp_counting_sink_create();
    batch = nytp_batch_sink_create(child, 8, 256, 8, 1);
    EXPECT(nytp_sink_activate(batch) == NYTP_OK, "activate");
    EXPECT(nytp_emit_time_line(batch, 1, 1, 1) == NYTP_OK, "tl1");
    EXPECT(nytp_emit_time_line(batch, 2, 1, 2) == NYTP_OK, "tl2");
    /* Arm child to fail next emit (first of flush). */
    EXPECT(nytp_counting_sink_fail_next(child, NYTP_ERR_IO) == NYTP_OK,
           "fail_next");
    {
        nytp_status stf = nytp_sink_flush(batch);
        EXPECT(stf == NYTP_ERR_IO, "flush fails IO");
    }
    /* Batch may still hold events; child has 0 logical if fail before commit. */
    st = nytp_counting_sink_stats(child);
    EXPECT(st && st->logical_emits == 0, "no phantom on failed flush");
    /* Pending still in batch. */
    EXPECT(nytp_batch_count(nytp_batch_sink_batch(batch)) == 2,
           "events retained on failed flush");
    nytp_sink_destroy(batch);
}

int main(void)
{
    test_capacity_stress();
    test_sv_lifetime();
    test_stmt_fast_no_alloc();
    test_high_water();
    test_emergency_oversized();
    test_fast_equals_public();
    test_microbench_light();
    test_failed_flush_preserves_order_state();

    if (failures) {
        fprintf(stderr, "FAILED: test_batch_fast (%d failures)\n", failures);
        return 1;
    }
    printf("OK: test_batch_fast (COL-004 fast path + COL-005 bounded batching)\n");
    return 0;
}

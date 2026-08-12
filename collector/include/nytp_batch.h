/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-005 — Bounded event batching (fixed headers + side arena).
 * COL-004 — No-allocation statement-event fast path into the batch.
 *
 * Invariants (plan 05 §2 / §5):
 *   - Common TIME_LINE / TIME_BLOCK append does not call malloc.
 *   - Variable payloads are copied once into a bounded arena (no Perl SV retain).
 *   - Exact event order under every flush position (cap 1..production).
 *   - Reset only after the child sink acknowledges the batch (flush success).
 *   - High-water triggers flush before capacity exhaustion.
 *   - Oversized payload uses emergency direct path after flush attempt.
 *
 * COL-015: begin_fork preflushes pending events; end_fork_child discards residual.
 *
 * Residuals: full fixtures/v5 corpus (complete TEST-003); not live XS hooks;
 * full flush-discount timing ADR still open (BASE-003 / COMPAT-003);
 * full TEST-018 live signal/forkdepth oracle matrix beyond unit stress.
 */
#ifndef NYTP_BATCH_H
#define NYTP_BATCH_H

#include "nytp_event.h"
#include "nytp_sink.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Production-ish defaults (tests force 1..capacity). */
#define NYTP_BATCH_DEFAULT_CAPACITY 64
#define NYTP_BATCH_DEFAULT_ARENA    4096
#define NYTP_BATCH_MIN_CAPACITY     1
#define NYTP_BATCH_MAX_CAPACITY     4096
#define NYTP_BATCH_MAX_ARENA        (1024u * 1024u) /* 1 MiB hard cap */

typedef struct nytp_batch_metrics {
    uint64_t appends;            /* all successful appends */
    uint64_t stmt_fast_appends;  /* TIME_LINE + TIME_BLOCK only (COL-004) */
    uint64_t flushes;            /* successful flush of ≥1 event */
    uint64_t high_water_flushes; /* flushes triggered by high-water */
    uint64_t full_flushes;       /* flushes triggered by count == capacity */
    uint64_t emergency_direct;   /* oversized payload emitted without buffer */
    uint64_t arena_bytes_copied; /* total bytes copied into arena */
    uint64_t heap_allocs;        /* increments only on create / grow (none after create) */
    /* COL-015 fork ownership */
    uint64_t fork_preflush;      /* begin_fork triggered a preflush of pending */
    uint64_t fork_child_discard; /* end_fork_child discarded residual events */
} nytp_batch_metrics;

typedef struct nytp_batch {
    nytp_event *events; /* heap once at create; fixed length */
    size_t capacity;    /* event slots */
    size_t count;       /* pending (unacked) events */
    size_t high_water;  /* flush when count >= high_water (1..capacity) */

    char *arena;
    size_t arena_cap;
    size_t arena_used;
    /* Scratch for compact-after-partial-flush (allocated once at create). */
    char *compact_tmp;

    nytp_batch_metrics metrics;

    /* Child sink that receives drained batches (not owned unless flag set). */
    nytp_sink *child;
    int owns_child; /* 1 => destroy child on batch sink destroy */

    /* Optional prebound ops for COL-004 fast path (set on bind). */
    const nytp_sink_ops *child_ops;

    /*
     * Set after commit_event when a subsequent high-water/full flush fails.
     * Batch-sink emit ops use this to advance COL-003 seq for the buffered
     * event even though the public wrapper sees a hard error.
     */
    int last_append_buffered;
} nytp_batch;

/* ---- Batch buffer (raw module; also used by batch sink) ---- */

/*
 * Allocate batch with fixed capacity and arena. high_water 0 => capacity
 * (flush only when full). Returns NULL on bad args / OOM.
 */
nytp_batch *nytp_batch_create(size_t capacity, size_t arena_cap,
                              size_t high_water);

void nytp_batch_destroy(nytp_batch *batch);

/* Bind / rebind child (does not take ownership unless set_owns_child). */
void nytp_batch_set_child(nytp_batch *batch, nytp_sink *child);
void nytp_batch_set_owns_child(nytp_batch *batch, int owns);

size_t nytp_batch_count(const nytp_batch *batch);
size_t nytp_batch_capacity(const nytp_batch *batch);
size_t nytp_batch_arena_used(const nytp_batch *batch);
const nytp_batch_metrics *nytp_batch_get_metrics(const nytp_batch *batch);

/*
 * Drain all pending events to child via child ops (not public wrappers —
 * preserves COL-003 seq already stamped on each event). Resets count/arena
 * only after full successful drain.
 *
 * On mid-batch failure: already-acked prefix is compacted out (not re-emitted
 * on retry); only unacked events remain. Hard errors mark the child failed.
 */
nytp_status nytp_batch_flush(nytp_batch *batch);

/* 1 if the most recent append committed an event into the buffer. */
int nytp_batch_last_append_buffered(const nytp_batch *batch);

/*
 * COL-015: drop pending events + arena without emitting to child.
 * Used on child post-fork so inherited residual cannot double-drain.
 */
void nytp_batch_discard_pending(nytp_batch *batch);

/* Pending unacked event count (0 if NULL). */
size_t nytp_batch_pending(const nytp_batch *batch);

/* ---- COL-004 no-alloc statement appends (POD only) ---- */

/*
 * Append TIME_LINE / TIME_BLOCK / DISCOUNT / SUB_ENTRY without arena or malloc.
 * Auto-flushes when high-water or capacity is reached (flush may call child).
 * `seq` is the COL-003 value already assigned by the public wrapper.
 */
nytp_status nytp_batch_append_time_line(nytp_batch *batch, nytp_seq seq,
                                        nytp_ticks ticks, nytp_fid fid,
                                        nytp_line line);
nytp_status nytp_batch_append_time_block(nytp_batch *batch, nytp_seq seq,
                                         nytp_ticks ticks, nytp_fid fid,
                                         nytp_line line, nytp_line block_line,
                                         nytp_line sub_line);
nytp_status nytp_batch_append_discount(nytp_batch *batch, nytp_seq seq);
nytp_status nytp_batch_append_sub_entry(nytp_batch *batch, nytp_seq seq,
                                        nytp_fid caller_fid,
                                        nytp_line caller_line);

/* ---- String-bearing appends (copy into arena; no SV retain) ---- */

nytp_status nytp_batch_append_attribute(nytp_batch *batch, nytp_seq seq,
                                        nytp_string_view key,
                                        nytp_string_view value);
nytp_status nytp_batch_append_option(nytp_batch *batch, nytp_seq seq,
                                     nytp_string_view key,
                                     nytp_string_view value);
nytp_status nytp_batch_append_comment(nytp_batch *batch, nytp_seq seq,
                                      nytp_string_view text);
nytp_status nytp_batch_append_new_fid(nytp_batch *batch, nytp_seq seq,
                                      nytp_fid fid, nytp_fid eval_fid,
                                      nytp_line eval_line, uint32_t flags,
                                      uint32_t size, uint32_t mtime,
                                      nytp_string_view name);
nytp_status nytp_batch_append_src_line(nytp_batch *batch, nytp_seq seq,
                                       nytp_fid fid, nytp_line line,
                                       nytp_string_view text);
nytp_status nytp_batch_append_sub_info(nytp_batch *batch, nytp_seq seq,
                                       nytp_fid fid, nytp_line first_line,
                                       nytp_line last_line,
                                       nytp_string_view name);
nytp_status nytp_batch_append_sub_callers(nytp_batch *batch, nytp_seq seq,
                                          nytp_fid fid, nytp_line line,
                                          uint32_t count, double incl,
                                          double excl, double reci,
                                          uint32_t rec_depth,
                                          nytp_string_view called,
                                          nytp_string_view caller);
nytp_status nytp_batch_append_pid_start(nytp_batch *batch, nytp_seq seq,
                                        nytp_pid pid, nytp_pid ppid,
                                        double start_time);
nytp_status nytp_batch_append_pid_end(nytp_batch *batch, nytp_seq seq,
                                      nytp_pid pid, double end_time);
nytp_status nytp_batch_append_sub_return(nytp_batch *batch, nytp_seq seq,
                                         nytp_depth depth, double incl_time,
                                         double excl_time,
                                         nytp_string_view subname);
/* Control: no logical seq; stored with seq=0 */
nytp_status nytp_batch_append_start_deflate(nytp_batch *batch);

/* ---- Batch sink (vtable facade over nytp_batch) ---- */

/*
 * Create a sink that buffers into a new batch and drains to `child`.
 * If owns_child, destroy(child) on batch-sink destroy.
 * Returns OPEN sink; destroy with nytp_sink_destroy.
 */
nytp_sink *nytp_batch_sink_create(nytp_sink *child, size_t capacity,
                                  size_t arena_cap, size_t high_water,
                                  int owns_child);

/* Borrow the underlying batch (metrics / tests). NULL if not a batch sink. */
nytp_batch *nytp_batch_sink_batch(nytp_sink *sink);

/*
 * COL-004 fast-path helpers for a prebound batch sink in ACTIVE state.
 * Skips vtable double-indirection for TIME_LINE / TIME_BLOCK when the sink
 * is a batch sink; falls back to nytp_emit_* otherwise.
 * Still assigns COL-003 seq via the same rules as public wrappers.
 */
nytp_status nytp_fast_emit_time_line(nytp_sink *sink, nytp_ticks ticks,
                                     nytp_fid fid, nytp_line line);
nytp_status nytp_fast_emit_time_block(nytp_sink *sink, nytp_ticks ticks,
                                      nytp_fid fid, nytp_line line,
                                      nytp_line block_line, nytp_line sub_line);

/* Light engineering microbench (not release certification — see BENCH notes). */
typedef struct nytp_fast_bench_result {
    uint64_t iterations;
    uint64_t elapsed_ns; /* 0 if clock unavailable */
    uint64_t stmt_fast_appends;
    size_t event_sizeof;
    size_t batch_capacity;
} nytp_fast_bench_result;

/*
 * Run `iterations` TIME_LINE appends through a capacity-N batch sink into a
 * counting child (auto-flush). Fills *out. Returns NYTP_OK or error.
 */
nytp_status nytp_fast_bench_time_line(size_t capacity, uint64_t iterations,
                                      nytp_fast_bench_result *out);

#ifdef __cplusplus
}
#endif

#endif /* NYTP_BATCH_H */

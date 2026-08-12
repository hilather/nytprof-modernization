/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-015 — Fork / PID transition protocol with buffered sinks.
 *
 * Formalizes pre/post-fork ownership for layered sinks (batch, dual, wire):
 *   1. Pre-fork flush so no pending batch events are duplicated across
 *      parent/child address spaces.
 *   2. FORK_SPLIT gate (no emit) until parent or child resume.
 *   3. Parent keeps COL-003 sequence domain; child resets seq to 0.
 *   4. Child discards residual batch (should be empty after preflush).
 *   5. addpid path helpers + wire-sink child re-init (detach shared path,
 *      fresh stream) to avoid shared-path truncate races / double-write.
 *
 * Residuals (honest):
 *   - Full live Perl/XS opcode hooks + real signal-safe finalize matrix.
 *   - Complete TEST-018 forkdepth/addpid/merge oracle fixture suite.
 *   - Inherited zlib/zstd compressor state mid-deflate under OS fork is
 *     **not** continued in the child — child re-init starts a clean stream
 *     (parent keeps the active compressor). Product 6.15 parity for
 *     mid-deflate fork remains a residual until measured against oracle.
 *   - Nested forkdepth policy vs option parsing is stress-covered only
 *     as depth counters, not as product option wiring.
 */
#ifndef NYTP_FORK_H
#define NYTP_FORK_H

#include "nytp_batch.h"
#include "nytp_sink.h"

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- Metrics (engineering counters; not wire) ---- */

typedef struct nytp_fork_metrics {
    uint64_t prepare_calls;
    uint64_t preflush_ok;
    uint64_t preflush_skipped; /* flush returned OK with nothing to do / no-op */
    uint64_t preflush_fail;
    uint64_t begin_fork_ok;
    uint64_t begin_fork_fail;
    uint64_t parent_resume;
    uint64_t child_resume;
    uint64_t child_discard_events; /* events dropped on child path */
    uint64_t child_discard_arena;  /* arena bytes dropped on child path */
    uint64_t path_rebind;
    uint64_t path_detach;
    uint64_t child_wire_reinit;
} nytp_fork_metrics;

/* ---- Policy ---- */

typedef struct nytp_fork_policy {
    /*
     * 1 (default): call nytp_sink_flush(root) before begin_fork.
     * Fail-closed on hard flush errors (IO/FAILED/OVERFLOW).
     */
    int flush_before_fork;
    /*
     * 1: after successful flush, if a batch root still has pending events,
     * return NYTP_ERR_STATE (should not happen after full flush).
     * Default 0 (best-effort; child discard still runs).
     */
    int require_empty_buffer;
    /*
     * 1 (default): on child resume, discard any residual batch pending
     * (events + arena). Prevents duplicate drain of pre-fork events if
     * preflush was skipped or failed partially.
     */
    int discard_child_buffer;
    /*
     * 1: if residual pending on child resume, fail with NYTP_ERR_STATE
     * before discard. Default 0 (discard silently with metrics).
     */
    int fail_if_child_residual;
} nytp_fork_policy;

/* Default policy: flush, discard residual, no hard residual fail. */
nytp_fork_policy nytp_fork_policy_default(void);

/* Zero metrics (safe no-op if NULL). */
void nytp_fork_metrics_clear(nytp_fork_metrics *m);

/*
 * Prepare for fork:
 *   ACTIVE → (optional flush) → FORK_SPLIT via nytp_sink_begin_fork.
 * Emits rejected until resume_parent / resume_child.
 * Returns NYTP_ERR_STATE if not ACTIVE; propagates flush/begin errors.
 */
nytp_status nytp_fork_prepare(nytp_sink *root, const nytp_fork_policy *pol,
                              nytp_fork_metrics *metrics /* nullable */);

/*
 * Parent post-fork: FORK_SPLIT → ACTIVE; COL-003 sequence continues.
 */
nytp_status nytp_fork_resume_parent(nytp_sink *root,
                                    nytp_fork_metrics *metrics /* nullable */);

/*
 * Child post-fork:
 *   - optional residual batch discard (policy)
 *   - FORK_SPLIT → OPEN with COL-003 seq reset (nytp_sink_end_fork_child)
 * Caller should then activate and emit PID_START for the new process stream.
 * Wire sinks under the tree need nytp_*_sink_fork_child_reinit (or dual helper)
 * when the same sink object is retained across OS fork.
 */
nytp_status nytp_fork_resume_child(nytp_sink *root,
                                   const nytp_fork_policy *pol,
                                   nytp_fork_metrics *metrics /* nullable */);

/*
 * Format addpid-style path: "<base>.<pid>" into buf.
 * Returns required size including NUL (like snprintf). If buflen is too
 * small, buf is left untouched when buflen==0; otherwise truncated NUL-term
 * write may still occur for buflen>0 (standard snprintf rules).
 * base NULL or empty → returns -1.
 */
int nytp_fork_addpid_path(const char *base, nytp_pid pid, char *buf,
                          size_t buflen);

/*
 * If sink is a batch sink, discard residual (via nytp_batch_discard_pending)
 * and accumulate into metrics. Returns number of events discarded. Non-batch → 0.
 */
size_t nytp_fork_discard_batch_residual(nytp_sink *sink,
                                        nytp_fork_metrics *metrics /* nullable */);

#ifdef __cplusplus
}
#endif

#endif /* NYTP_FORK_H */

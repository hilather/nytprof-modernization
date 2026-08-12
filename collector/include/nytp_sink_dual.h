/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-014 — Same-run dual writer (test/dev-only, OQ-4).
 *
 * Fan out each semantic emit to two child sinks (typically v5 + v6) so
 * collector CI can prove same-run logical equality for E4/M6 evidence.
 *
 * **Not product UX.** Product collection remains single-format
 * (`format=v5` default; `format=v6` opt-in). Dual-sink is enabled only via
 * explicit C create APIs and optional test/dev env flags:
 *   NYTPROF_DUAL_SINK=1
 *   NYTPROF_FORMAT=dual   (test/dev alias only — not advertised operators)
 *
 * Residuals:
 *   - Full fixtures/v5/ oracle stream equality under dual needs live hooks
 *     / complete TEST-003 + TEST-008 M6 suite (not claimed here).
 *   - Secondary-fail after primary wire write: dual parent sticky-fails for
 *     all secondary non-OK (STATE/UNSUPPORTED mapped to FAILED); primary
 *     bytes/stats are not rolled back (partial dual residual; COL-018).
 *   - Full live TEST-018 oracle forkdepth/addpid matrices remain residual;
 *     unit stress for dual+batch fork is under COL-015 (nytp_fork + test_fork_pid).
 */
#ifndef NYTP_SINK_DUAL_H
#define NYTP_SINK_DUAL_H

#include "nytp_sink.h"
#include "nytp_sink_counting.h"

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Out-of-band comparison metadata (not written into either profile wire).
 * Filled after a dual run; useful for harness logs and M6 evidence.
 */
typedef struct nytp_dual_compare_meta {
    uint64_t fanout_ok;              /* successful dual emits (all kinds) */
    uint64_t fanout_fail_primary;    /* primary emit failed first */
    uint64_t fanout_fail_secondary;  /* secondary failed after primary OK */
    uint64_t logical_equal_checks;   /* times logical_equal was called */
    uint64_t logical_equal_ok;       /* times logical_equal returned 1 */
    const char *primary_name;        /* borrowed child name or "(null)" */
    const char *secondary_name;
    /* Snapshot of last successful equality probe (by_kind equal + seq rings). */
    int last_equal;                  /* 1 equal, 0 unequal, -1 not probed */
    size_t first_kind_mismatch;      /* index into NYTP_EVT_KIND_COUNT or SIZE_MAX */
    size_t first_seq_mismatch;       /* ring index or SIZE_MAX */
} nytp_dual_compare_meta;

/*
 * Create a dual sink that fans out to existing children.
 * If owns_primary / owns_secondary, destroy() will destroy that child.
 * Both children must be non-NULL. Dual starts OPEN; children keep their
 * current lifecycle states (normally also OPEN at create).
 */
nytp_sink *nytp_dual_sink_create(nytp_sink *primary, nytp_sink *secondary,
                                 int owns_primary, int owns_secondary);

/*
 * Test/dev convenience: create owned v5 + absolute v6 children and wrap them.
 * Paths may be NULL (in-memory only). Intended only for harness use.
 * Does **not** require env flag (explicit create is already opt-in).
 */
nytp_sink *nytp_dual_sink_create_v5_v6(const char *path_v5,
                                       const char *path_v6);

/* True if sink was created by nytp_dual_sink_create / create_v5_v6. */
int nytp_dual_sink_is_dual(const nytp_sink *sink);

/* Borrow children (NULL if not dual). */
nytp_sink *nytp_dual_sink_primary(nytp_sink *sink);
nytp_sink *nytp_dual_sink_secondary(nytp_sink *sink);
const nytp_sink *nytp_dual_sink_primary_const(const nytp_sink *sink);
const nytp_sink *nytp_dual_sink_secondary_const(const nytp_sink *sink);

/* Borrow comparison metadata (valid until destroy). NULL if not dual. */
const nytp_dual_compare_meta *nytp_dual_sink_meta(const nytp_sink *sink);

/*
 * Test/dev env probe (OQ-4):
 *   returns 1 if NYTPROF_DUAL_SINK is 1/true/yes/on (case-insensitive)
 *           or NYTPROF_FORMAT equals "dual" (case-insensitive);
 *   else 0.
 * Product defaults must not treat this as an operator format.
 */
int nytp_dual_env_enabled(void);

/*
 * Compare logical multiplicities + seq rings of the two children when both
 * expose counting-compatible stats (counting / v5 / v6).
 *
 * Equality requires:
 *   - equal by_kind[k] for all kinds (incl. START_DEFLATE control count)
 *   - equal logical_emits
 *   - equal seq_ring_len and identical seq_ring + kind_ring entries
 *
 * Updates dual meta (last_equal, first_*_mismatch counters).
 * Returns 1 if equal, 0 if unequal or stats unavailable.
 */
int nytp_dual_sink_logical_equal(nytp_sink *sink);

/*
 * Fetch counting-compatible stats from a child if it is counting/v5/v6.
 * Returns NULL if unknown backend.
 */
const nytp_counting_stats *nytp_dual_child_stats(const nytp_sink *child);

/*
 * Write out-of-band comparison metadata as a small JSON object to path
 * (create/truncate). Does not touch profile wires. Returns NYTP_OK or error.
 */
nytp_status nytp_dual_sink_write_compare_meta(const nytp_sink *sink,
                                              const char *path);

/*
 * COL-015: re-init both wire children after child resume when they are v5/v6.
 * Counting children are left intact (stats continue / separate trees preferred).
 * path_v5 / path_v6 may be NULL (detach). Non-v5/v6 children are skipped.
 * Returns first hard error; best-effort continues to the other child.
 */
nytp_status nytp_dual_sink_fork_child_reinit(nytp_sink *dual,
                                             const char *path_v5,
                                             const char *path_v6);

#ifdef __cplusplus
}
#endif

#endif /* NYTP_SINK_DUAL_H */

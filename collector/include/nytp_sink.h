/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-001 — Canonical semantic sink interface (v5 path scaffolding).
 * COL-002 — Explicit lifecycle state machine + legal transitions.
 * COL-003 — Monotonic logical event sequence numbers (internal; v5 default
 *           does not write seq unless a future diagnostic mode enables it).
 *
 * Design (ADR-0004 overlay + plan 03/05):
 *   - Emit functions express COMPAT-001 logical events, not wire bytes.
 *   - Vtable dispatch; production may later specialize/inline single-sink builds.
 *   - Stream-neutral: same API fans out to v5 / dual / test sinks.
 *   - No heap allocation required on the common TIME_LINE / TIME_BLOCK path.
 *
 * Not COL-007 (v6 writer). Not COL-006 (real v5 wire). Full M4 oracle
 * v5-via-sink equality remains residual until COL-006 + complete TEST-003.
 */
#ifndef NYTP_SINK_H
#define NYTP_SINK_H

#include "nytp_types.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct nytp_sink nytp_sink;

/*
 * Vtable: backends implement the ops they support.
 * NULL function pointers mean NYTP_ERR_UNSUPPORTED from the public emit wrappers.
 * Hot-path ops (time_line / time_block / discount) should never allocate.
 */
typedef struct nytp_sink_ops {
    /* Identity / diagnostics (optional; may be NULL). */
    const char *(*name)(const nytp_sink *sink);

    /* Backend activate/flush/close/destroy. Lifecycle legality is enforced
     * by public wrappers (COL-002) before these are invoked. */
    nytp_status (*activate)(nytp_sink *sink);
    nytp_status (*flush)(nytp_sink *sink);
    nytp_status (*close)(nytp_sink *sink);
    void (*destroy)(nytp_sink *sink);

    /* Mapped logical events (COMPAT-001). */
    nytp_status (*emit_attribute)(nytp_sink *sink,
                                  nytp_string_view key,
                                  nytp_string_view value);
    nytp_status (*emit_option)(nytp_sink *sink,
                               nytp_string_view key,
                               nytp_string_view value);
    nytp_status (*emit_comment)(nytp_sink *sink, nytp_string_view text);

    nytp_status (*emit_time_line)(nytp_sink *sink,
                                  nytp_ticks ticks,
                                  nytp_fid fid,
                                  nytp_line line);
    nytp_status (*emit_time_block)(nytp_sink *sink,
                                   nytp_ticks ticks,
                                   nytp_fid fid,
                                   nytp_line line,
                                   nytp_line block_line,
                                   nytp_line sub_line);
    nytp_status (*emit_discount)(nytp_sink *sink);

    nytp_status (*emit_new_fid)(nytp_sink *sink,
                                nytp_fid fid,
                                nytp_fid eval_fid,
                                nytp_line eval_line,
                                uint32_t flags,
                                uint32_t size,
                                uint32_t mtime,
                                nytp_string_view name);
    nytp_status (*emit_src_line)(nytp_sink *sink,
                                 nytp_fid fid,
                                 nytp_line line,
                                 nytp_string_view text);
    nytp_status (*emit_sub_info)(nytp_sink *sink,
                                 nytp_fid fid,
                                 nytp_line first_line,
                                 nytp_line last_line,
                                 nytp_string_view name);
    nytp_status (*emit_sub_callers)(nytp_sink *sink,
                                    nytp_fid fid,
                                    nytp_line line,
                                    uint32_t count,
                                    double incl,
                                    double excl,
                                    double reci,
                                    uint32_t rec_depth,
                                    nytp_string_view called,
                                    nytp_string_view caller);

    nytp_status (*emit_pid_start)(nytp_sink *sink,
                                  nytp_pid pid,
                                  nytp_pid ppid,
                                  double start_time);
    nytp_status (*emit_pid_end)(nytp_sink *sink,
                                nytp_pid pid,
                                double end_time);

    nytp_status (*emit_sub_entry)(nytp_sink *sink,
                                  nytp_fid caller_fid,
                                  nytp_line caller_line);
    nytp_status (*emit_sub_return)(nytp_sink *sink,
                                   nytp_depth depth,
                                   double incl_time,
                                   double excl_time,
                                   nytp_string_view subname);

    /* Stream control (not a logical profile event — no COL-003 seq). */
    nytp_status (*emit_start_deflate)(nytp_sink *sink);

    /*
     * Optional: invoked by public wrappers *after* a successful logical
     * emit_commit (seq already assigned). Backends must record seq rings
     * here — never during emit_* — so failed emits leave no phantom seq.
     */
    void (*on_logical_committed)(nytp_sink *sink, nytp_seq seq,
                                 nytp_event_kind kind);

    /*
     * Optional lifecycle forward hooks (NULL ok). Invoked by public
     * COL-002 wrappers after the parent sink state is updated, so layered
     * sinks (batch) can keep a child in sync (stop/finalize/fork).
     */
    nytp_status (*notify_stop)(nytp_sink *sink);
    nytp_status (*notify_begin_finalize)(nytp_sink *sink);
    nytp_status (*notify_begin_fork)(nytp_sink *sink);
    nytp_status (*notify_end_fork_parent)(nytp_sink *sink);
    nytp_status (*notify_end_fork_child)(nytp_sink *sink);
} nytp_sink_ops;

struct nytp_sink {
    const nytp_sink_ops *ops;
    nytp_sink_state state;
    void *impl; /* backend-private */

    /* COL-003 sequence state (wrapper-owned; backends must not mutate). */
    nytp_seq next_seq;     /* next logical seq to assign (starts at 0) */
    nytp_seq last_seq;     /* last successfully assigned logical seq */
    int has_last_seq;      /* 0 until first successful logical emit */

    /* COL-002 failure sticky reason (valid when state == FAILED). */
    nytp_status fail_reason;
};

/* ---- Lifecycle (COL-002) ---- */

const char *nytp_sink_name(const nytp_sink *sink);
nytp_sink_state nytp_sink_get_state(const nytp_sink *sink);
const char *nytp_sink_state_name(nytp_sink_state state);

/* True if from -> to is a legal single-step transition. */
int nytp_sink_transition_allowed(nytp_sink_state from, nytp_sink_state to);

/*
 * OPEN | STOPPED -> ACTIVE (restart supported).
 * Backend activate op is optional (default: set ACTIVE).
 */
nytp_status nytp_sink_activate(nytp_sink *sink);

/* ACTIVE -> STOPPED. Emits rejected until activate (restart). */
nytp_status nytp_sink_stop(nytp_sink *sink);

/*
 * OPEN | ACTIVE | STOPPED -> FINALIZING.
 * Allows finalization emits (src/sub summaries / pid_end); rejects hot-path
 * statement/call emits.
 */
nytp_status nytp_sink_begin_finalize(nytp_sink *sink);

/*
 * ACTIVE -> FORK_SPLIT. Follow with end_fork_parent or end_fork_child.
 * Full fork buffer ownership is COL-015 residual; this freezes the state
 * transitions only.
 */
nytp_status nytp_sink_begin_fork(nytp_sink *sink);
/* FORK_SPLIT -> ACTIVE; sequence continues (parent). */
nytp_status nytp_sink_end_fork_parent(nytp_sink *sink);
/* FORK_SPLIT -> OPEN; sequence resets to 0 (child new stream). */
nytp_status nytp_sink_end_fork_child(nytp_sink *sink);

/* Sticky fail: any non-CLOSED -> FAILED. reason should be non-OK. */
nytp_status nytp_sink_mark_failed(nytp_sink *sink, nytp_status reason);
nytp_status nytp_sink_fail_reason(const nytp_sink *sink);

nytp_status nytp_sink_flush(nytp_sink *sink);
/* Any non-CLOSED -> CLOSED (idempotent if already CLOSED). */
nytp_status nytp_sink_close(nytp_sink *sink);
void nytp_sink_destroy(nytp_sink *sink);

/* Emit readiness for a kind in the current state (COL-002). */
int nytp_sink_can_emit(const nytp_sink *sink, nytp_event_kind kind);

/* ---- Sequence (COL-003) ---- */

/* 1 if kind is a logical profile event (assigns seq); 0 for control/none. */
int nytp_event_kind_is_logical(nytp_event_kind kind);

/* Next seq that would be assigned (does not advance). */
nytp_seq nytp_sink_peek_seq(const nytp_sink *sink);

/*
 * Last successfully assigned logical seq. Returns NYTP_ERR_STATE if none yet.
 * *out may be NULL to only probe has_last_seq.
 */
nytp_status nytp_sink_last_seq(const nytp_sink *sink, nytp_seq *out);

/* How many logical events have been successfully sequenced. */
nytp_seq nytp_sink_logical_count(const nytp_sink *sink);

/*
 * Gapless check for a recorded seq array that should be start, start+1, ...
 * On mismatch fills *mm (if non-NULL) and returns 0; else returns 1.
 */
typedef struct nytp_seq_mismatch {
    size_t index;          /* first bad index */
    nytp_seq expected_seq;
    nytp_seq actual_seq;
} nytp_seq_mismatch;

int nytp_seq_check_gapless(const nytp_seq *seqs, size_t n, nytp_seq start,
                           nytp_seq_mismatch *mm);

/* ---- Public emit wrappers (null- / state- / ops-checked; seq on success) ---- */

nytp_status nytp_emit_attribute(nytp_sink *sink,
                                nytp_string_view key,
                                nytp_string_view value);
nytp_status nytp_emit_option(nytp_sink *sink,
                             nytp_string_view key,
                             nytp_string_view value);
nytp_status nytp_emit_comment(nytp_sink *sink, nytp_string_view text);

nytp_status nytp_emit_time_line(nytp_sink *sink,
                                nytp_ticks ticks,
                                nytp_fid fid,
                                nytp_line line);
nytp_status nytp_emit_time_block(nytp_sink *sink,
                                 nytp_ticks ticks,
                                 nytp_fid fid,
                                 nytp_line line,
                                 nytp_line block_line,
                                 nytp_line sub_line);
nytp_status nytp_emit_discount(nytp_sink *sink);

nytp_status nytp_emit_new_fid(nytp_sink *sink,
                              nytp_fid fid,
                              nytp_fid eval_fid,
                              nytp_line eval_line,
                              uint32_t flags,
                              uint32_t size,
                              uint32_t mtime,
                              nytp_string_view name);
nytp_status nytp_emit_src_line(nytp_sink *sink,
                               nytp_fid fid,
                               nytp_line line,
                               nytp_string_view text);
nytp_status nytp_emit_sub_info(nytp_sink *sink,
                               nytp_fid fid,
                               nytp_line first_line,
                               nytp_line last_line,
                               nytp_string_view name);
nytp_status nytp_emit_sub_callers(nytp_sink *sink,
                                  nytp_fid fid,
                                  nytp_line line,
                                  uint32_t count,
                                  double incl,
                                  double excl,
                                  double reci,
                                  uint32_t rec_depth,
                                  nytp_string_view called,
                                  nytp_string_view caller);

nytp_status nytp_emit_pid_start(nytp_sink *sink,
                                nytp_pid pid,
                                nytp_pid ppid,
                                double start_time);
nytp_status nytp_emit_pid_end(nytp_sink *sink, nytp_pid pid, double end_time);

nytp_status nytp_emit_sub_entry(nytp_sink *sink,
                                nytp_fid caller_fid,
                                nytp_line caller_line);
nytp_status nytp_emit_sub_return(nytp_sink *sink,
                                 nytp_depth depth,
                                 double incl_time,
                                 double excl_time,
                                 nytp_string_view subname);

nytp_status nytp_emit_start_deflate(nytp_sink *sink);

/* Human-readable kind name for mapping tables / tests (never NULL). */
const char *nytp_event_kind_name(nytp_event_kind kind);

#ifdef __cplusplus
}
#endif

#endif /* NYTP_SINK_H */

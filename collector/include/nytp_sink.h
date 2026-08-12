/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-001 — Canonical semantic sink interface (v5 path scaffolding).
 *
 * Design (ADR-0004 overlay + plan 03/05):
 *   - Emit functions express COMPAT-001 logical events, not wire bytes.
 *   - Vtable dispatch; production may later specialize/inline single-sink builds.
 *   - Stream-neutral: same API fans out to v5 / dual / test sinks.
 *   - No heap allocation required on the common TIME_LINE / TIME_BLOCK path.
 *
 * Not COL-007 (v6 writer). Not COL-002 (full lifecycle freeze).
 * Not TEST-003 / PR-B03 (fake-clock).
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

    /* Lifecycle (minimal; COL-002 expands). */
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

    /* Stream control (not a logical profile event). */
    nytp_status (*emit_start_deflate)(nytp_sink *sink);
} nytp_sink_ops;

struct nytp_sink {
    const nytp_sink_ops *ops;
    nytp_sink_state state;
    void *impl; /* backend-private */
};

/* ---- Public emit / lifecycle wrappers (null- and ops-checked) ---- */

const char *nytp_sink_name(const nytp_sink *sink);
nytp_sink_state nytp_sink_get_state(const nytp_sink *sink);

nytp_status nytp_sink_activate(nytp_sink *sink);
nytp_status nytp_sink_flush(nytp_sink *sink);
nytp_status nytp_sink_close(nytp_sink *sink);
void nytp_sink_destroy(nytp_sink *sink);

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

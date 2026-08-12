/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * Public emit wrappers + COL-002 lifecycle + COL-003 sequence assignment.
 */
#include "nytp_sink.h"

#include <stddef.h>

/* ---- Lifecycle helpers (COL-002) ---- */

const char *nytp_sink_state_name(nytp_sink_state state)
{
    switch (state) {
    case NYTP_SINK_UNINITIALIZED:
        return "uninitialized";
    case NYTP_SINK_OPEN:
        return "open";
    case NYTP_SINK_ACTIVE:
        return "active";
    case NYTP_SINK_STOPPED:
        return "stopped";
    case NYTP_SINK_FINALIZING:
        return "finalizing";
    case NYTP_SINK_CLOSED:
        return "closed";
    case NYTP_SINK_FAILED:
        return "failed";
    case NYTP_SINK_FORK_SPLIT:
        return "fork_split";
    default:
        return "unknown";
    }
}

int nytp_sink_transition_allowed(nytp_sink_state from, nytp_sink_state to)
{
    if (from == to && to == NYTP_SINK_CLOSED) {
        return 1; /* idempotent close */
    }
    switch (from) {
    case NYTP_SINK_UNINITIALIZED:
        return to == NYTP_SINK_OPEN;
    case NYTP_SINK_OPEN:
        return to == NYTP_SINK_ACTIVE || to == NYTP_SINK_FINALIZING ||
               to == NYTP_SINK_CLOSED || to == NYTP_SINK_FAILED;
    case NYTP_SINK_ACTIVE:
        return to == NYTP_SINK_STOPPED || to == NYTP_SINK_FINALIZING ||
               to == NYTP_SINK_FORK_SPLIT || to == NYTP_SINK_CLOSED ||
               to == NYTP_SINK_FAILED;
    case NYTP_SINK_STOPPED:
        return to == NYTP_SINK_ACTIVE || to == NYTP_SINK_FINALIZING ||
               to == NYTP_SINK_CLOSED || to == NYTP_SINK_FAILED;
    case NYTP_SINK_FORK_SPLIT:
        return to == NYTP_SINK_ACTIVE || to == NYTP_SINK_OPEN ||
               to == NYTP_SINK_CLOSED || to == NYTP_SINK_FAILED;
    case NYTP_SINK_FINALIZING:
        return to == NYTP_SINK_CLOSED || to == NYTP_SINK_FAILED;
    case NYTP_SINK_FAILED:
        return to == NYTP_SINK_CLOSED;
    case NYTP_SINK_CLOSED:
        return 0;
    default:
        return 0;
    }
}

/*
 * Emit classes by state (COL-002 scaffold):
 *   OPEN, ACTIVE: all kinds
 *   FINALIZING: finalization + meta (not statement/call hot path)
 *   STOPPED / FORK_SPLIT / CLOSED / FAILED / UNINITIALIZED: none
 */
int nytp_sink_can_emit(const nytp_sink *sink, nytp_event_kind kind)
{
    if (!sink) {
        return 0;
    }
    switch (sink->state) {
    case NYTP_SINK_OPEN:
    case NYTP_SINK_ACTIVE:
        return kind != NYTP_EVT_NONE;
    case NYTP_SINK_FINALIZING:
        switch (kind) {
        case NYTP_EVT_SRC_LINE:
        case NYTP_EVT_SUB_INFO:
        case NYTP_EVT_SUB_CALLERS:
        case NYTP_EVT_PID_END:
        case NYTP_EVT_ATTRIBUTE:
        case NYTP_EVT_OPTION:
        case NYTP_EVT_COMMENT:
        case NYTP_EVT_DISCOUNT:
            return 1;
        default:
            return 0;
        }
    default:
        return 0;
    }
}

int nytp_event_kind_is_logical(nytp_event_kind kind)
{
    if (kind == NYTP_EVT_NONE || kind == NYTP_EVT_START_DEFLATE) {
        return 0;
    }
    if ((unsigned)kind >= (unsigned)NYTP_EVT_KIND_COUNT) {
        return 0;
    }
    return 1;
}

nytp_seq nytp_sink_peek_seq(const nytp_sink *sink)
{
    if (!sink) {
        return 0;
    }
    return sink->next_seq;
}

nytp_status nytp_sink_last_seq(const nytp_sink *sink, nytp_seq *out)
{
    if (!sink) {
        return NYTP_ERR_NULL;
    }
    if (!sink->has_last_seq) {
        return NYTP_ERR_STATE;
    }
    if (out) {
        *out = sink->last_seq;
    }
    return NYTP_OK;
}

nytp_seq nytp_sink_logical_count(const nytp_sink *sink)
{
    if (!sink) {
        return 0;
    }
    return sink->next_seq;
}

int nytp_seq_check_gapless(const nytp_seq *seqs, size_t n, nytp_seq start,
                           nytp_seq_mismatch *mm)
{
    size_t i;
    if (n > 0 && !seqs) {
        if (mm) {
            mm->index = 0;
            mm->expected_seq = start;
            mm->actual_seq = 0;
        }
        return 0;
    }
    for (i = 0; i < n; i++) {
        nytp_seq expect = start + (nytp_seq)i;
        if (seqs[i] != expect) {
            if (mm) {
                mm->index = i;
                mm->expected_seq = expect;
                mm->actual_seq = seqs[i];
            }
            return 0;
        }
    }
    return 1;
}

const char *nytp_sink_name(const nytp_sink *sink)
{
    if (!sink || !sink->ops || !sink->ops->name) {
        return "unknown";
    }
    return sink->ops->name(sink);
}

nytp_sink_state nytp_sink_get_state(const nytp_sink *sink)
{
    if (!sink) {
        return NYTP_SINK_UNINITIALIZED;
    }
    return sink->state;
}

nytp_status nytp_sink_activate(nytp_sink *sink)
{
    nytp_status st;
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (sink->state != NYTP_SINK_OPEN && sink->state != NYTP_SINK_STOPPED) {
        return NYTP_ERR_STATE;
    }
    if (!nytp_sink_transition_allowed(sink->state, NYTP_SINK_ACTIVE)) {
        return NYTP_ERR_STATE;
    }
    if (!sink->ops->activate) {
        sink->state = NYTP_SINK_ACTIVE;
        return NYTP_OK;
    }
    st = sink->ops->activate(sink);
    if (st == NYTP_OK) {
        sink->state = NYTP_SINK_ACTIVE;
    } else if (st == NYTP_ERR_IO || st == NYTP_ERR_FAILED) {
        sink->state = NYTP_SINK_FAILED;
        sink->fail_reason = st;
    }
    return st;
}

nytp_status nytp_sink_stop(nytp_sink *sink)
{
    nytp_status st;
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (sink->state != NYTP_SINK_ACTIVE) {
        return NYTP_ERR_STATE;
    }
    sink->state = NYTP_SINK_STOPPED;
    if (sink->ops->notify_stop) {
        st = sink->ops->notify_stop(sink);
        if (st != NYTP_OK) {
            if (st == NYTP_ERR_IO || st == NYTP_ERR_FAILED ||
                st == NYTP_ERR_OVERFLOW) {
                sink->state = NYTP_SINK_FAILED;
                sink->fail_reason = st;
            }
            return st;
        }
    }
    return NYTP_OK;
}

nytp_status nytp_sink_begin_finalize(nytp_sink *sink)
{
    nytp_status st;
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (sink->state != NYTP_SINK_OPEN && sink->state != NYTP_SINK_ACTIVE &&
        sink->state != NYTP_SINK_STOPPED) {
        return NYTP_ERR_STATE;
    }
    sink->state = NYTP_SINK_FINALIZING;
    if (sink->ops->notify_begin_finalize) {
        st = sink->ops->notify_begin_finalize(sink);
        if (st != NYTP_OK) {
            if (st == NYTP_ERR_IO || st == NYTP_ERR_FAILED ||
                st == NYTP_ERR_OVERFLOW) {
                sink->state = NYTP_SINK_FAILED;
                sink->fail_reason = st;
            }
            return st;
        }
    }
    return NYTP_OK;
}

nytp_status nytp_sink_begin_fork(nytp_sink *sink)
{
    nytp_status st;
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (sink->state != NYTP_SINK_ACTIVE) {
        return NYTP_ERR_STATE;
    }
    sink->state = NYTP_SINK_FORK_SPLIT;
    if (sink->ops->notify_begin_fork) {
        st = sink->ops->notify_begin_fork(sink);
        if (st != NYTP_OK) {
            if (st == NYTP_ERR_IO || st == NYTP_ERR_FAILED ||
                st == NYTP_ERR_OVERFLOW) {
                sink->state = NYTP_SINK_FAILED;
                sink->fail_reason = st;
            }
            return st;
        }
    }
    return NYTP_OK;
}

nytp_status nytp_sink_end_fork_parent(nytp_sink *sink)
{
    nytp_status st;
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (sink->state != NYTP_SINK_FORK_SPLIT) {
        return NYTP_ERR_STATE;
    }
    /* Parent keeps sequence continuity. */
    sink->state = NYTP_SINK_ACTIVE;
    if (sink->ops->notify_end_fork_parent) {
        st = sink->ops->notify_end_fork_parent(sink);
        if (st != NYTP_OK) {
            if (st == NYTP_ERR_IO || st == NYTP_ERR_FAILED ||
                st == NYTP_ERR_OVERFLOW) {
                sink->state = NYTP_SINK_FAILED;
                sink->fail_reason = st;
            }
            return st;
        }
    }
    return NYTP_OK;
}

nytp_status nytp_sink_end_fork_child(nytp_sink *sink)
{
    nytp_status st;
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (sink->state != NYTP_SINK_FORK_SPLIT) {
        return NYTP_ERR_STATE;
    }
    /* Child starts a new process stream: reset COL-003 sequence. */
    sink->state = NYTP_SINK_OPEN;
    sink->next_seq = 0;
    sink->last_seq = 0;
    sink->has_last_seq = 0;
    if (sink->ops->notify_end_fork_child) {
        st = sink->ops->notify_end_fork_child(sink);
        if (st != NYTP_OK) {
            if (st == NYTP_ERR_IO || st == NYTP_ERR_FAILED ||
                st == NYTP_ERR_OVERFLOW) {
                sink->state = NYTP_SINK_FAILED;
                sink->fail_reason = st;
            }
            return st;
        }
    }
    return NYTP_OK;
}

nytp_status nytp_sink_mark_failed(nytp_sink *sink, nytp_status reason)
{
    if (!sink) {
        return NYTP_ERR_NULL;
    }
    if (sink->state == NYTP_SINK_CLOSED) {
        return NYTP_ERR_STATE;
    }
    if (reason == NYTP_OK) {
        reason = NYTP_ERR_FAILED;
    }
    sink->state = NYTP_SINK_FAILED;
    sink->fail_reason = reason;
    return NYTP_OK;
}

nytp_status nytp_sink_fail_reason(const nytp_sink *sink)
{
    if (!sink) {
        return NYTP_ERR_NULL;
    }
    if (sink->state != NYTP_SINK_FAILED) {
        return NYTP_OK;
    }
    return sink->fail_reason == NYTP_OK ? NYTP_ERR_FAILED : sink->fail_reason;
}

nytp_status nytp_sink_flush(nytp_sink *sink)
{
    nytp_status st;
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (sink->state != NYTP_SINK_OPEN && sink->state != NYTP_SINK_ACTIVE &&
        sink->state != NYTP_SINK_STOPPED &&
        sink->state != NYTP_SINK_FINALIZING) {
        return NYTP_ERR_STATE;
    }
    if (!sink->ops->flush) {
        return NYTP_OK;
    }
    st = sink->ops->flush(sink);
    /* Sticky-fail parent on hard errors (same policy as emit_commit). */
    if (st == NYTP_ERR_IO || st == NYTP_ERR_FAILED || st == NYTP_ERR_OVERFLOW) {
        sink->state = NYTP_SINK_FAILED;
        sink->fail_reason = st;
    }
    return st;
}

nytp_status nytp_sink_close(nytp_sink *sink)
{
    nytp_status st;
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (sink->state == NYTP_SINK_CLOSED) {
        return NYTP_OK;
    }
    if (sink->state == NYTP_SINK_UNINITIALIZED) {
        return NYTP_ERR_STATE;
    }
    if (!sink->ops->close) {
        sink->state = NYTP_SINK_CLOSED;
        return NYTP_OK;
    }
    st = sink->ops->close(sink);
    if (st == NYTP_OK) {
        sink->state = NYTP_SINK_CLOSED;
    } else {
        sink->state = NYTP_SINK_FAILED;
        sink->fail_reason = st;
    }
    return st;
}

void nytp_sink_destroy(nytp_sink *sink)
{
    if (!sink || !sink->ops || !sink->ops->destroy) {
        return;
    }
    sink->ops->destroy(sink);
}

/* ---- Emit precheck + sequence commit ---- */

static nytp_status emit_precheck(nytp_sink *sink, nytp_event_kind kind,
                                 int has_op)
{
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (!nytp_sink_can_emit(sink, kind)) {
        return NYTP_ERR_STATE;
    }
    if (!has_op) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return NYTP_OK;
}

/*
 * Commit seq only after successful backend return. Optional
 * on_logical_committed lets backends append seq/kind rings without
 * recording during a failed emit (no phantom seq).
 */
static nytp_status emit_commit(nytp_sink *sink, nytp_status st, int logical,
                               nytp_event_kind kind)
{
    if (st == NYTP_OK) {
        if (logical) {
            nytp_seq seq = sink->next_seq;
            sink->last_seq = seq;
            sink->next_seq = seq + 1;
            sink->has_last_seq = 1;
            if (sink->ops->on_logical_committed) {
                sink->ops->on_logical_committed(sink, seq, kind);
            }
        }
        return NYTP_OK;
    }
    if (st == NYTP_ERR_IO || st == NYTP_ERR_FAILED || st == NYTP_ERR_OVERFLOW) {
        sink->state = NYTP_SINK_FAILED;
        sink->fail_reason = st;
    }
    return st;
}

nytp_status nytp_emit_attribute(nytp_sink *sink, nytp_string_view key,
                                nytp_string_view value)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_ATTRIBUTE,
                      sink && sink->ops && sink->ops->emit_attribute);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(sink, sink->ops->emit_attribute(sink, key, value), 1,
                       NYTP_EVT_ATTRIBUTE);
}

nytp_status nytp_emit_option(nytp_sink *sink, nytp_string_view key,
                             nytp_string_view value)
{
    nytp_status st = emit_precheck(sink, NYTP_EVT_OPTION,
                                   sink && sink->ops && sink->ops->emit_option);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(sink, sink->ops->emit_option(sink, key, value), 1,
                       NYTP_EVT_OPTION);
}

nytp_status nytp_emit_comment(nytp_sink *sink, nytp_string_view text)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_COMMENT,
                      sink && sink->ops && sink->ops->emit_comment);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(sink, sink->ops->emit_comment(sink, text), 1,
                       NYTP_EVT_COMMENT);
}

nytp_status nytp_emit_time_line(nytp_sink *sink, nytp_ticks ticks, nytp_fid fid,
                                nytp_line line)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_TIME_LINE,
                      sink && sink->ops && sink->ops->emit_time_line);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(sink, sink->ops->emit_time_line(sink, ticks, fid, line),
                       1, NYTP_EVT_TIME_LINE);
}

nytp_status nytp_emit_time_block(nytp_sink *sink, nytp_ticks ticks, nytp_fid fid,
                                 nytp_line line, nytp_line block_line,
                                 nytp_line sub_line)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_TIME_BLOCK,
                      sink && sink->ops && sink->ops->emit_time_block);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(sink,
                       sink->ops->emit_time_block(sink, ticks, fid, line,
                                                  block_line, sub_line),
                       1, NYTP_EVT_TIME_BLOCK);
}

nytp_status nytp_emit_discount(nytp_sink *sink)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_DISCOUNT,
                      sink && sink->ops && sink->ops->emit_discount);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(sink, sink->ops->emit_discount(sink), 1,
                       NYTP_EVT_DISCOUNT);
}

nytp_status nytp_emit_new_fid(nytp_sink *sink, nytp_fid fid, nytp_fid eval_fid,
                              nytp_line eval_line, uint32_t flags,
                              uint32_t size, uint32_t mtime,
                              nytp_string_view name)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_NEW_FID,
                      sink && sink->ops && sink->ops->emit_new_fid);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(sink,
                       sink->ops->emit_new_fid(sink, fid, eval_fid, eval_line,
                                               flags, size, mtime, name),
                       1, NYTP_EVT_NEW_FID);
}

nytp_status nytp_emit_src_line(nytp_sink *sink, nytp_fid fid, nytp_line line,
                               nytp_string_view text)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_SRC_LINE,
                      sink && sink->ops && sink->ops->emit_src_line);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(sink, sink->ops->emit_src_line(sink, fid, line, text),
                       1, NYTP_EVT_SRC_LINE);
}

nytp_status nytp_emit_sub_info(nytp_sink *sink, nytp_fid fid,
                               nytp_line first_line, nytp_line last_line,
                               nytp_string_view name)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_SUB_INFO,
                      sink && sink->ops && sink->ops->emit_sub_info);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(
        sink, sink->ops->emit_sub_info(sink, fid, first_line, last_line, name),
        1, NYTP_EVT_SUB_INFO);
}

nytp_status nytp_emit_sub_callers(nytp_sink *sink, nytp_fid fid, nytp_line line,
                                  uint32_t count, double incl, double excl,
                                  double reci, uint32_t rec_depth,
                                  nytp_string_view called,
                                  nytp_string_view caller)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_SUB_CALLERS,
                      sink && sink->ops && sink->ops->emit_sub_callers);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(sink,
                       sink->ops->emit_sub_callers(sink, fid, line, count, incl,
                                                   excl, reci, rec_depth, called,
                                                   caller),
                       1, NYTP_EVT_SUB_CALLERS);
}

nytp_status nytp_emit_pid_start(nytp_sink *sink, nytp_pid pid, nytp_pid ppid,
                                double start_time)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_PID_START,
                      sink && sink->ops && sink->ops->emit_pid_start);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(
        sink, sink->ops->emit_pid_start(sink, pid, ppid, start_time), 1,
        NYTP_EVT_PID_START);
}

nytp_status nytp_emit_pid_end(nytp_sink *sink, nytp_pid pid, double end_time)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_PID_END,
                      sink && sink->ops && sink->ops->emit_pid_end);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(sink, sink->ops->emit_pid_end(sink, pid, end_time), 1,
                       NYTP_EVT_PID_END);
}

nytp_status nytp_emit_sub_entry(nytp_sink *sink, nytp_fid caller_fid,
                                nytp_line caller_line)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_SUB_ENTRY,
                      sink && sink->ops && sink->ops->emit_sub_entry);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(
        sink, sink->ops->emit_sub_entry(sink, caller_fid, caller_line), 1,
        NYTP_EVT_SUB_ENTRY);
}

nytp_status nytp_emit_sub_return(nytp_sink *sink, nytp_depth depth,
                                 double incl_time, double excl_time,
                                 nytp_string_view subname)
{
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_SUB_RETURN,
                      sink && sink->ops && sink->ops->emit_sub_return);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(sink,
                       sink->ops->emit_sub_return(sink, depth, incl_time,
                                                  excl_time, subname),
                       1, NYTP_EVT_SUB_RETURN);
}

nytp_status nytp_emit_start_deflate(nytp_sink *sink)
{
    /* Control: no COL-003 sequence / no on_logical_committed. */
    nytp_status st =
        emit_precheck(sink, NYTP_EVT_START_DEFLATE,
                      sink && sink->ops && sink->ops->emit_start_deflate);
    if (st != NYTP_OK) {
        return st;
    }
    return emit_commit(sink, sink->ops->emit_start_deflate(sink), 0,
                       NYTP_EVT_START_DEFLATE);
}

const char *nytp_event_kind_name(nytp_event_kind kind)
{
    switch (kind) {
    case NYTP_EVT_NONE:
        return "none";
    case NYTP_EVT_ATTRIBUTE:
        return "attribute";
    case NYTP_EVT_OPTION:
        return "option";
    case NYTP_EVT_COMMENT:
        return "comment";
    case NYTP_EVT_TIME_LINE:
        return "time_line";
    case NYTP_EVT_TIME_BLOCK:
        return "time_block";
    case NYTP_EVT_DISCOUNT:
        return "discount";
    case NYTP_EVT_NEW_FID:
        return "new_fid";
    case NYTP_EVT_SRC_LINE:
        return "src_line";
    case NYTP_EVT_SUB_INFO:
        return "sub_info";
    case NYTP_EVT_SUB_CALLERS:
        return "sub_callers";
    case NYTP_EVT_PID_START:
        return "pid_start";
    case NYTP_EVT_PID_END:
        return "pid_end";
    case NYTP_EVT_SUB_ENTRY:
        return "sub_entry";
    case NYTP_EVT_SUB_RETURN:
        return "sub_return";
    case NYTP_EVT_START_DEFLATE:
        return "start_deflate";
    default:
        return "unknown";
    }
}

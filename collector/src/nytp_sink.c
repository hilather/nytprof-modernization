/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * Public emit wrappers for COL-001 semantic sink.
 */
#include "nytp_sink.h"

#include <stddef.h>

static int sink_ready(const nytp_sink *sink)
{
    if (!sink || !sink->ops) {
        return 0;
    }
    if (sink->state == NYTP_SINK_FAILED || sink->state == NYTP_SINK_CLOSED ||
        sink->state == NYTP_SINK_UNINITIALIZED) {
        return 0;
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
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (sink->state == NYTP_SINK_FAILED || sink->state == NYTP_SINK_CLOSED) {
        return NYTP_ERR_STATE;
    }
    if (!sink->ops->activate) {
        sink->state = NYTP_SINK_ACTIVE;
        return NYTP_OK;
    }
    return sink->ops->activate(sink);
}

nytp_status nytp_sink_flush(nytp_sink *sink)
{
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (!sink_ready(sink)) {
        return NYTP_ERR_STATE;
    }
    if (!sink->ops->flush) {
        return NYTP_OK;
    }
    return sink->ops->flush(sink);
}

nytp_status nytp_sink_close(nytp_sink *sink)
{
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (sink->state == NYTP_SINK_CLOSED) {
        return NYTP_OK;
    }
    if (!sink->ops->close) {
        sink->state = NYTP_SINK_CLOSED;
        return NYTP_OK;
    }
    return sink->ops->close(sink);
}

void nytp_sink_destroy(nytp_sink *sink)
{
    if (!sink || !sink->ops || !sink->ops->destroy) {
        return;
    }
    sink->ops->destroy(sink);
}

/* Shared null/state/ops checks; returns non-zero status if emit must abort. */
static nytp_status emit_precheck(nytp_sink *sink, int has_op)
{
    if (!sink || !sink->ops) {
        return NYTP_ERR_NULL;
    }
    if (!sink_ready(sink)) {
        return NYTP_ERR_STATE;
    }
    if (!has_op) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return NYTP_OK;
}

nytp_status nytp_emit_attribute(nytp_sink *sink,
                                nytp_string_view key,
                                nytp_string_view value)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_attribute);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_attribute(sink, key, value);
}

nytp_status nytp_emit_option(nytp_sink *sink,
                             nytp_string_view key,
                             nytp_string_view value)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_option);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_option(sink, key, value);
}

nytp_status nytp_emit_comment(nytp_sink *sink, nytp_string_view text)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_comment);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_comment(sink, text);
}

nytp_status nytp_emit_time_line(nytp_sink *sink,
                                nytp_ticks ticks,
                                nytp_fid fid,
                                nytp_line line)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_time_line);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_time_line(sink, ticks, fid, line);
}

nytp_status nytp_emit_time_block(nytp_sink *sink,
                                 nytp_ticks ticks,
                                 nytp_fid fid,
                                 nytp_line line,
                                 nytp_line block_line,
                                 nytp_line sub_line)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_time_block);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_time_block(sink, ticks, fid, line, block_line,
                                      sub_line);
}

nytp_status nytp_emit_discount(nytp_sink *sink)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_discount);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_discount(sink);
}

nytp_status nytp_emit_new_fid(nytp_sink *sink,
                              nytp_fid fid,
                              nytp_fid eval_fid,
                              nytp_line eval_line,
                              uint32_t flags,
                              uint32_t size,
                              uint32_t mtime,
                              nytp_string_view name)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_new_fid);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_new_fid(sink, fid, eval_fid, eval_line, flags, size,
                                   mtime, name);
}

nytp_status nytp_emit_src_line(nytp_sink *sink,
                               nytp_fid fid,
                               nytp_line line,
                               nytp_string_view text)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_src_line);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_src_line(sink, fid, line, text);
}

nytp_status nytp_emit_sub_info(nytp_sink *sink,
                               nytp_fid fid,
                               nytp_line first_line,
                               nytp_line last_line,
                               nytp_string_view name)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_sub_info);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_sub_info(sink, fid, first_line, last_line, name);
}

nytp_status nytp_emit_sub_callers(nytp_sink *sink,
                                  nytp_fid fid,
                                  nytp_line line,
                                  uint32_t count,
                                  double incl,
                                  double excl,
                                  double reci,
                                  uint32_t rec_depth,
                                  nytp_string_view called,
                                  nytp_string_view caller)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_sub_callers);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_sub_callers(sink, fid, line, count, incl, excl, reci,
                                       rec_depth, called, caller);
}

nytp_status nytp_emit_pid_start(nytp_sink *sink,
                                nytp_pid pid,
                                nytp_pid ppid,
                                double start_time)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_pid_start);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_pid_start(sink, pid, ppid, start_time);
}

nytp_status nytp_emit_pid_end(nytp_sink *sink, nytp_pid pid, double end_time)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_pid_end);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_pid_end(sink, pid, end_time);
}

nytp_status nytp_emit_sub_entry(nytp_sink *sink,
                                nytp_fid caller_fid,
                                nytp_line caller_line)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_sub_entry);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_sub_entry(sink, caller_fid, caller_line);
}

nytp_status nytp_emit_sub_return(nytp_sink *sink,
                                 nytp_depth depth,
                                 double incl_time,
                                 double excl_time,
                                 nytp_string_view subname)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_sub_return);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_sub_return(sink, depth, incl_time, excl_time,
                                      subname);
}

nytp_status nytp_emit_start_deflate(nytp_sink *sink)
{
    nytp_status st =
        emit_precheck(sink, sink && sink->ops && sink->ops->emit_start_deflate);
    if (st != NYTP_OK) {
        return st;
    }
    return sink->ops->emit_start_deflate(sink);
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

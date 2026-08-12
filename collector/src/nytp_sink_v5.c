/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * Stub v5 sink adapter (COL-001). Conceptual route for legacy v5 writes;
 * does not encode wire bytes (COL-006). Not COL-007.
 */
#include "nytp_sink_v5.h"

#include <stdlib.h>
#include <string.h>

typedef struct v5_impl {
    nytp_counting_stats stats;
    char *path; /* sink-owned copy; may be NULL */
} v5_impl;

static void note_kind(v5_impl *vi, nytp_event_kind kind)
{
    if ((unsigned)kind < (unsigned)NYTP_EVT_KIND_COUNT) {
        vi->stats.by_kind[kind]++;
    }
    vi->stats.total_emits++;
    vi->stats.last_kind = kind;
}

static void copy_subname(v5_impl *vi, nytp_string_view name)
{
    size_t n = name.len;
    if (n >= sizeof(vi->stats.last_subname)) {
        n = sizeof(vi->stats.last_subname) - 1;
    }
    if (n > 0 && name.ptr) {
        memcpy(vi->stats.last_subname, name.ptr, n);
    }
    vi->stats.last_subname[n] = '\0';
    vi->stats.last_subname_len = n;
}

static const char *v5_name(const nytp_sink *sink)
{
    (void)sink;
    return "v5-stub";
}

static nytp_status v5_activate(nytp_sink *sink)
{
    sink->state = NYTP_SINK_ACTIVE;
    return NYTP_OK;
}

static nytp_status v5_flush(nytp_sink *sink)
{
    /* No I/O in stub; COL-006 will flush the real writer buffer. */
    (void)sink;
    return NYTP_OK;
}

static nytp_status v5_close(nytp_sink *sink)
{
    sink->state = NYTP_SINK_CLOSED;
    return NYTP_OK;
}

static void v5_destroy(nytp_sink *sink)
{
    v5_impl *vi;
    if (!sink) {
        return;
    }
    vi = (v5_impl *)sink->impl;
    if (vi) {
        free(vi->path);
        free(vi);
    }
    free(sink);
}

static v5_impl *vi_of(nytp_sink *sink)
{
    return (v5_impl *)sink->impl;
}

static nytp_status v5_emit_attribute(nytp_sink *sink,
                                     nytp_string_view key,
                                     nytp_string_view value)
{
    (void)key;
    (void)value;
    note_kind(vi_of(sink), NYTP_EVT_ATTRIBUTE);
    return NYTP_OK;
}

static nytp_status v5_emit_option(nytp_sink *sink,
                                  nytp_string_view key,
                                  nytp_string_view value)
{
    (void)key;
    (void)value;
    note_kind(vi_of(sink), NYTP_EVT_OPTION);
    return NYTP_OK;
}

static nytp_status v5_emit_comment(nytp_sink *sink, nytp_string_view text)
{
    (void)text;
    note_kind(vi_of(sink), NYTP_EVT_COMMENT);
    return NYTP_OK;
}

static nytp_status v5_emit_time_line(nytp_sink *sink,
                                     nytp_ticks ticks,
                                     nytp_fid fid,
                                     nytp_line line)
{
    v5_impl *vi = vi_of(sink);
    note_kind(vi, NYTP_EVT_TIME_LINE);
    vi->stats.last_ticks = ticks;
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    vi->stats.last_block_line = 0;
    vi->stats.last_sub_line = 0;
    return NYTP_OK;
}

static nytp_status v5_emit_time_block(nytp_sink *sink,
                                      nytp_ticks ticks,
                                      nytp_fid fid,
                                      nytp_line line,
                                      nytp_line block_line,
                                      nytp_line sub_line)
{
    v5_impl *vi = vi_of(sink);
    note_kind(vi, NYTP_EVT_TIME_BLOCK);
    vi->stats.last_ticks = ticks;
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    vi->stats.last_block_line = block_line;
    vi->stats.last_sub_line = sub_line;
    return NYTP_OK;
}

static nytp_status v5_emit_discount(nytp_sink *sink)
{
    note_kind(vi_of(sink), NYTP_EVT_DISCOUNT);
    return NYTP_OK;
}

static nytp_status v5_emit_new_fid(nytp_sink *sink,
                                   nytp_fid fid,
                                   nytp_fid eval_fid,
                                   nytp_line eval_line,
                                   uint32_t flags,
                                   uint32_t size,
                                   uint32_t mtime,
                                   nytp_string_view name)
{
    (void)eval_fid;
    (void)eval_line;
    (void)flags;
    (void)size;
    (void)mtime;
    (void)name;
    v5_impl *vi = vi_of(sink);
    note_kind(vi, NYTP_EVT_NEW_FID);
    vi->stats.last_fid = fid;
    return NYTP_OK;
}

static nytp_status v5_emit_src_line(nytp_sink *sink,
                                    nytp_fid fid,
                                    nytp_line line,
                                    nytp_string_view text)
{
    (void)text;
    v5_impl *vi = vi_of(sink);
    note_kind(vi, NYTP_EVT_SRC_LINE);
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    return NYTP_OK;
}

static nytp_status v5_emit_sub_info(nytp_sink *sink,
                                    nytp_fid fid,
                                    nytp_line first_line,
                                    nytp_line last_line,
                                    nytp_string_view name)
{
    v5_impl *vi = vi_of(sink);
    note_kind(vi, NYTP_EVT_SUB_INFO);
    vi->stats.last_fid = fid;
    vi->stats.last_line = first_line;
    vi->stats.last_block_line = last_line;
    copy_subname(vi, name);
    return NYTP_OK;
}

static nytp_status v5_emit_sub_callers(nytp_sink *sink,
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
    (void)count;
    (void)incl;
    (void)excl;
    (void)reci;
    (void)rec_depth;
    (void)caller;
    v5_impl *vi = vi_of(sink);
    note_kind(vi, NYTP_EVT_SUB_CALLERS);
    vi->stats.last_fid = fid;
    vi->stats.last_line = line;
    copy_subname(vi, called);
    return NYTP_OK;
}

static nytp_status v5_emit_pid_start(nytp_sink *sink,
                                     nytp_pid pid,
                                     nytp_pid ppid,
                                     double start_time)
{
    (void)ppid;
    (void)start_time;
    v5_impl *vi = vi_of(sink);
    note_kind(vi, NYTP_EVT_PID_START);
    vi->stats.last_fid = (nytp_fid)pid;
    return NYTP_OK;
}

static nytp_status v5_emit_pid_end(nytp_sink *sink,
                                   nytp_pid pid,
                                   double end_time)
{
    (void)end_time;
    v5_impl *vi = vi_of(sink);
    note_kind(vi, NYTP_EVT_PID_END);
    vi->stats.last_fid = (nytp_fid)pid;
    return NYTP_OK;
}

static nytp_status v5_emit_sub_entry(nytp_sink *sink,
                                     nytp_fid caller_fid,
                                     nytp_line caller_line)
{
    v5_impl *vi = vi_of(sink);
    note_kind(vi, NYTP_EVT_SUB_ENTRY);
    vi->stats.last_fid = caller_fid;
    vi->stats.last_line = caller_line;
    return NYTP_OK;
}

static nytp_status v5_emit_sub_return(nytp_sink *sink,
                                      nytp_depth depth,
                                      double incl_time,
                                      double excl_time,
                                      nytp_string_view subname)
{
    (void)incl_time;
    (void)excl_time;
    v5_impl *vi = vi_of(sink);
    note_kind(vi, NYTP_EVT_SUB_RETURN);
    vi->stats.last_depth = depth;
    copy_subname(vi, subname);
    return NYTP_OK;
}

static nytp_status v5_emit_start_deflate(nytp_sink *sink)
{
    note_kind(vi_of(sink), NYTP_EVT_START_DEFLATE);
    return NYTP_OK;
}

static const nytp_sink_ops v5_ops = {
    .name = v5_name,
    .activate = v5_activate,
    .flush = v5_flush,
    .close = v5_close,
    .destroy = v5_destroy,
    .emit_attribute = v5_emit_attribute,
    .emit_option = v5_emit_option,
    .emit_comment = v5_emit_comment,
    .emit_time_line = v5_emit_time_line,
    .emit_time_block = v5_emit_time_block,
    .emit_discount = v5_emit_discount,
    .emit_new_fid = v5_emit_new_fid,
    .emit_src_line = v5_emit_src_line,
    .emit_sub_info = v5_emit_sub_info,
    .emit_sub_callers = v5_emit_sub_callers,
    .emit_pid_start = v5_emit_pid_start,
    .emit_pid_end = v5_emit_pid_end,
    .emit_sub_entry = v5_emit_sub_entry,
    .emit_sub_return = v5_emit_sub_return,
    .emit_start_deflate = v5_emit_start_deflate,
};

nytp_sink *nytp_v5_sink_create(const char *path)
{
    nytp_sink *sink = (nytp_sink *)calloc(1, sizeof(*sink));
    v5_impl *vi = (v5_impl *)calloc(1, sizeof(*vi));
    if (!sink || !vi) {
        free(sink);
        free(vi);
        return NULL;
    }
    if (path) {
        size_t n = strlen(path);
        vi->path = (char *)malloc(n + 1);
        if (!vi->path) {
            free(vi);
            free(sink);
            return NULL;
        }
        memcpy(vi->path, path, n + 1);
    }
    sink->ops = &v5_ops;
    sink->state = NYTP_SINK_OPEN;
    sink->impl = vi;
    return sink;
}

int nytp_v5_sink_is_v5(const nytp_sink *sink)
{
    /* Ops-pointer identity: safe for any sink backend (no impl cast / OOB). */
    return sink != NULL && sink->ops == &v5_ops;
}

const nytp_counting_stats *nytp_v5_sink_stats(const nytp_sink *sink)
{
    v5_impl *vi;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return NULL;
    }
    vi = (v5_impl *)sink->impl;
    return &vi->stats;
}

const char *nytp_v5_sink_path(const nytp_sink *sink)
{
    v5_impl *vi;
    if (!nytp_v5_sink_is_v5(sink) || !sink->impl) {
        return NULL;
    }
    vi = (v5_impl *)sink->impl;
    return vi->path;
}

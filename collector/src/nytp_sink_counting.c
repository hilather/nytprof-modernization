/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * Counting sink implementation for COL-001..003 unit tests.
 * Seq/kind rings are filled only via on_logical_committed (post emit_commit).
 */
#include "nytp_sink_counting.h"

#include <stdlib.h>
#include <string.h>

typedef struct counting_impl {
    nytp_counting_stats stats;
} counting_impl;

static void note_kind(counting_impl *ci, nytp_event_kind kind)
{
    if ((unsigned)kind < (unsigned)NYTP_EVT_KIND_COUNT) {
        ci->stats.by_kind[kind]++;
    }
    ci->stats.total_emits++;
    ci->stats.last_kind = kind;
}

/* Post-commit only: never call from emit_* on a failing path. */
static void counting_on_logical_committed(nytp_sink *sink, nytp_seq seq,
                                          nytp_event_kind kind)
{
    counting_impl *ci;
    if (!sink || !sink->impl) {
        return;
    }
    ci = (counting_impl *)sink->impl;
    ci->stats.logical_emits++;
    ci->stats.last_seq = seq;
    ci->stats.has_last_seq = 1;
    if (ci->stats.seq_ring_len < NYTP_COUNTING_SEQ_RING) {
        ci->stats.seq_ring[ci->stats.seq_ring_len] = seq;
        ci->stats.kind_ring[ci->stats.seq_ring_len] = kind;
        ci->stats.seq_ring_len++;
    } else {
        memmove(ci->stats.seq_ring, ci->stats.seq_ring + 1,
                (NYTP_COUNTING_SEQ_RING - 1) * sizeof(nytp_seq));
        memmove(ci->stats.kind_ring, ci->stats.kind_ring + 1,
                (NYTP_COUNTING_SEQ_RING - 1) * sizeof(nytp_event_kind));
        ci->stats.seq_ring[NYTP_COUNTING_SEQ_RING - 1] = seq;
        ci->stats.kind_ring[NYTP_COUNTING_SEQ_RING - 1] = kind;
    }
}

static void copy_subname(counting_impl *ci, nytp_string_view name)
{
    size_t n = name.len;
    if (n >= sizeof(ci->stats.last_subname)) {
        n = sizeof(ci->stats.last_subname) - 1;
    }
    if (n > 0 && name.ptr) {
        memcpy(ci->stats.last_subname, name.ptr, n);
    }
    ci->stats.last_subname[n] = '\0';
    ci->stats.last_subname_len = n;
}

static const char *counting_name(const nytp_sink *sink)
{
    (void)sink;
    return "counting";
}

static nytp_status counting_activate(nytp_sink *sink)
{
    (void)sink;
    return NYTP_OK;
}

static nytp_status counting_flush(nytp_sink *sink)
{
    (void)sink;
    return NYTP_OK;
}

static nytp_status counting_close(nytp_sink *sink)
{
    (void)sink;
    return NYTP_OK;
}

static void counting_destroy(nytp_sink *sink)
{
    if (!sink) {
        return;
    }
    free(sink->impl);
    free(sink);
}

static counting_impl *ci_of(nytp_sink *sink)
{
    return (counting_impl *)sink->impl;
}

/* If fail_next is armed, return it once without counting. */
static nytp_status maybe_fail_next(counting_impl *ci)
{
    nytp_status err;
    if (ci->stats.fail_next_emit == NYTP_OK) {
        return NYTP_OK;
    }
    err = ci->stats.fail_next_emit;
    ci->stats.fail_next_emit = NYTP_OK;
    return err;
}

static nytp_status counting_emit_attribute(nytp_sink *sink,
                                           nytp_string_view key,
                                           nytp_string_view value)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    (void)key;
    (void)value;
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_ATTRIBUTE);
    return NYTP_OK;
}

static nytp_status counting_emit_option(nytp_sink *sink, nytp_string_view key,
                                        nytp_string_view value)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    (void)key;
    (void)value;
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_OPTION);
    return NYTP_OK;
}

static nytp_status counting_emit_comment(nytp_sink *sink, nytp_string_view text)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    (void)text;
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_COMMENT);
    return NYTP_OK;
}

static nytp_status counting_emit_time_line(nytp_sink *sink, nytp_ticks ticks,
                                           nytp_fid fid, nytp_line line)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_TIME_LINE);
    ci->stats.last_ticks = ticks;
    ci->stats.last_fid = fid;
    ci->stats.last_line = line;
    ci->stats.last_block_line = 0;
    ci->stats.last_sub_line = 0;
    return NYTP_OK;
}

static nytp_status counting_emit_time_block(nytp_sink *sink, nytp_ticks ticks,
                                            nytp_fid fid, nytp_line line,
                                            nytp_line block_line,
                                            nytp_line sub_line)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_TIME_BLOCK);
    ci->stats.last_ticks = ticks;
    ci->stats.last_fid = fid;
    ci->stats.last_line = line;
    ci->stats.last_block_line = block_line;
    ci->stats.last_sub_line = sub_line;
    return NYTP_OK;
}

static nytp_status counting_emit_discount(nytp_sink *sink)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_DISCOUNT);
    return NYTP_OK;
}

static nytp_status counting_emit_new_fid(nytp_sink *sink, nytp_fid fid,
                                         nytp_fid eval_fid, nytp_line eval_line,
                                         uint32_t flags, uint32_t size,
                                         uint32_t mtime, nytp_string_view name)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    (void)eval_fid;
    (void)eval_line;
    (void)flags;
    (void)size;
    (void)mtime;
    (void)name;
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_NEW_FID);
    ci->stats.last_fid = fid;
    return NYTP_OK;
}

static nytp_status counting_emit_src_line(nytp_sink *sink, nytp_fid fid,
                                          nytp_line line,
                                          nytp_string_view text)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    size_t n;
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_SRC_LINE);
    ci->stats.last_fid = fid;
    ci->stats.last_line = line;
    ci->stats.last_src_fid = fid;
    ci->stats.last_src_line = line;
    n = text.len;
    if (n >= sizeof(ci->stats.last_src_text)) {
        n = sizeof(ci->stats.last_src_text) - 1;
    }
    if (n > 0 && text.ptr) {
        memcpy(ci->stats.last_src_text, text.ptr, n);
    }
    ci->stats.last_src_text[n] = '\0';
    ci->stats.last_src_text_len = n;
    return NYTP_OK;
}

static nytp_status counting_emit_sub_info(nytp_sink *sink, nytp_fid fid,
                                          nytp_line first_line,
                                          nytp_line last_line,
                                          nytp_string_view name)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_SUB_INFO);
    ci->stats.last_fid = fid;
    ci->stats.last_line = first_line;
    ci->stats.last_block_line = last_line;
    copy_subname(ci, name);
    return NYTP_OK;
}

static nytp_status counting_emit_sub_callers(
    nytp_sink *sink, nytp_fid fid, nytp_line line, uint32_t count, double incl,
    double excl, double reci, uint32_t rec_depth, nytp_string_view called,
    nytp_string_view caller)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    (void)count;
    (void)incl;
    (void)excl;
    (void)reci;
    (void)rec_depth;
    (void)caller;
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_SUB_CALLERS);
    ci->stats.last_fid = fid;
    ci->stats.last_line = line;
    copy_subname(ci, called);
    return NYTP_OK;
}

static nytp_status counting_emit_pid_start(nytp_sink *sink, nytp_pid pid,
                                           nytp_pid ppid, double start_time)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    (void)ppid;
    (void)start_time;
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_PID_START);
    ci->stats.last_fid = (nytp_fid)pid;
    return NYTP_OK;
}

static nytp_status counting_emit_pid_end(nytp_sink *sink, nytp_pid pid,
                                         double end_time)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    (void)end_time;
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_PID_END);
    ci->stats.last_fid = (nytp_fid)pid;
    return NYTP_OK;
}

static nytp_status counting_emit_sub_entry(nytp_sink *sink, nytp_fid caller_fid,
                                           nytp_line caller_line)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_SUB_ENTRY);
    ci->stats.last_fid = caller_fid;
    ci->stats.last_line = caller_line;
    return NYTP_OK;
}

static nytp_status counting_emit_sub_return(nytp_sink *sink, nytp_depth depth,
                                            double incl_time, double excl_time,
                                            nytp_string_view subname)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    (void)incl_time;
    (void)excl_time;
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_SUB_RETURN);
    ci->stats.last_depth = depth;
    copy_subname(ci, subname);
    return NYTP_OK;
}

static nytp_status counting_emit_start_deflate(nytp_sink *sink)
{
    counting_impl *ci = ci_of(sink);
    nytp_status st = maybe_fail_next(ci);
    if (st != NYTP_OK) {
        return st;
    }
    note_kind(ci, NYTP_EVT_START_DEFLATE);
    return NYTP_OK;
}

static const nytp_sink_ops counting_ops = {
    .name = counting_name,
    .activate = counting_activate,
    .flush = counting_flush,
    .close = counting_close,
    .destroy = counting_destroy,
    .emit_attribute = counting_emit_attribute,
    .emit_option = counting_emit_option,
    .emit_comment = counting_emit_comment,
    .emit_time_line = counting_emit_time_line,
    .emit_time_block = counting_emit_time_block,
    .emit_discount = counting_emit_discount,
    .emit_new_fid = counting_emit_new_fid,
    .emit_src_line = counting_emit_src_line,
    .emit_sub_info = counting_emit_sub_info,
    .emit_sub_callers = counting_emit_sub_callers,
    .emit_pid_start = counting_emit_pid_start,
    .emit_pid_end = counting_emit_pid_end,
    .emit_sub_entry = counting_emit_sub_entry,
    .emit_sub_return = counting_emit_sub_return,
    .emit_start_deflate = counting_emit_start_deflate,
    .on_logical_committed = counting_on_logical_committed,
};

nytp_sink *nytp_counting_sink_create(void)
{
    nytp_sink *sink = (nytp_sink *)calloc(1, sizeof(*sink));
    counting_impl *ci = (counting_impl *)calloc(1, sizeof(*ci));
    if (!sink || !ci) {
        free(sink);
        free(ci);
        return NULL;
    }
    sink->ops = &counting_ops;
    sink->state = NYTP_SINK_OPEN;
    sink->impl = ci;
    sink->next_seq = 0;
    sink->last_seq = 0;
    sink->has_last_seq = 0;
    sink->fail_reason = NYTP_OK;
    return sink;
}

const nytp_counting_stats *nytp_counting_sink_stats(const nytp_sink *sink)
{
    counting_impl *ci;
    if (!sink || sink->ops != &counting_ops || !sink->impl) {
        return NULL;
    }
    ci = (counting_impl *)sink->impl;
    return &ci->stats;
}

nytp_status nytp_counting_sink_fail_next(nytp_sink *sink, nytp_status err)
{
    counting_impl *ci;
    if (!sink || sink->ops != &counting_ops || !sink->impl) {
        return NYTP_ERR_NULL;
    }
    if (err == NYTP_OK) {
        return NYTP_ERR_STATE;
    }
    ci = (counting_impl *)sink->impl;
    ci->stats.fail_next_emit = err;
    return NYTP_OK;
}

nytp_status nytp_counting_sink_copy_seqs(const nytp_sink *sink, nytp_seq *out,
                                         size_t *out_n)
{
    const nytp_counting_stats *st;
    size_t need;
    size_t cap;
    if (!sink || !out_n) {
        return NYTP_ERR_NULL;
    }
    st = nytp_counting_sink_stats(sink);
    if (!st) {
        return NYTP_ERR_NULL;
    }
    need = st->seq_ring_len;
    cap = *out_n;
    *out_n = need;
    if (need == 0) {
        return NYTP_OK;
    }
    if (!out || cap < need) {
        if (out && cap > 0) {
            memcpy(out, st->seq_ring, cap * sizeof(nytp_seq));
        }
        return NYTP_ERR_OVERFLOW;
    }
    memcpy(out, st->seq_ring, need * sizeof(nytp_seq));
    return NYTP_OK;
}

nytp_status nytp_counting_sink_copy_kinds(const nytp_sink *sink,
                                          nytp_event_kind *out, size_t *out_n)
{
    const nytp_counting_stats *st;
    size_t need;
    size_t cap;
    if (!sink || !out_n) {
        return NYTP_ERR_NULL;
    }
    st = nytp_counting_sink_stats(sink);
    if (!st) {
        return NYTP_ERR_NULL;
    }
    need = st->seq_ring_len;
    cap = *out_n;
    *out_n = need;
    if (need == 0) {
        return NYTP_OK;
    }
    if (!out || cap < need) {
        if (out && cap > 0) {
            memcpy(out, st->kind_ring, cap * sizeof(nytp_event_kind));
        }
        return NYTP_ERR_OVERFLOW;
    }
    memcpy(out, st->kind_ring, need * sizeof(nytp_event_kind));
    return NYTP_OK;
}

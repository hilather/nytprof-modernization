/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-014 — Same-run dual writer (test/dev-only, OQ-4).
 * Fan out each canonical emit to primary + secondary children.
 */
#define _POSIX_C_SOURCE 200809L

#include "nytp_sink_dual.h"

#include "nytp_sink_v5.h"
#include "nytp_sink_v6.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct dual_impl {
    nytp_sink *primary;
    nytp_sink *secondary;
    int owns_primary;
    int owns_secondary;
    nytp_dual_compare_meta meta;
} dual_impl;

static const nytp_sink_ops DUAL_OPS;

static dual_impl *di_of(nytp_sink *sink)
{
    return sink ? (dual_impl *)sink->impl : NULL;
}

static const dual_impl *di_of_c(const nytp_sink *sink)
{
    return sink ? (const dual_impl *)sink->impl : NULL;
}

static void meta_init(nytp_dual_compare_meta *m)
{
    memset(m, 0, sizeof(*m));
    m->primary_name = "(null)";
    m->secondary_name = "(null)";
    m->last_equal = -1;
    m->first_kind_mismatch = (size_t)-1;
    m->first_seq_mismatch = (size_t)-1;
}

static void meta_refresh_names(dual_impl *di)
{
    if (!di) {
        return;
    }
    di->meta.primary_name =
        (di->primary && di->primary->ops && di->primary->ops->name)
            ? di->primary->ops->name(di->primary)
            : "(null)";
    di->meta.secondary_name =
        (di->secondary && di->secondary->ops && di->secondary->ops->name)
            ? di->secondary->ops->name(di->secondary)
            : "(null)";
    if (!di->meta.primary_name) {
        di->meta.primary_name = "(null)";
    }
    if (!di->meta.secondary_name) {
        di->meta.secondary_name = "(null)";
    }
}

/* Align child COL-003 seq state to the dual-assigned seq after a successful
 * dual emit (mirrors batch flush dual-compare path). */
static void commit_child_seq(nytp_sink *child, nytp_seq seq,
                             nytp_event_kind kind)
{
    if (!child || !child->ops) {
        return;
    }
    child->last_seq = seq;
    child->next_seq = seq + 1;
    child->has_last_seq = 1;
    if (child->ops->on_logical_committed) {
        child->ops->on_logical_committed(child, seq, kind);
    }
}

/*
 * Fan-out helper: run op on primary then secondary.
 * On primary fail, secondary is not called.
 * On secondary fail after primary OK, count fail_secondary and force a
 * sticky-fail status so public emit_commit marks dual FAILED for *all*
 * secondary error codes (IO/FAILED/OVERFLOW already sticky; STATE /
 * UNSUPPORTED / EXHAUSTED / NULL mapped to FAILED). Partial dual residual:
 * primary already wrote; no rollback (COL-018).
 */
typedef nytp_status (*dual_child_emit_fn)(nytp_sink *child, void *ctx);

/* Statuses that emit_commit already sticky-fails on. */
static int dual_status_is_sticky(nytp_status st)
{
    return st == NYTP_ERR_IO || st == NYTP_ERR_FAILED ||
           st == NYTP_ERR_OVERFLOW;
}

static nytp_status dual_fanout(nytp_sink *sink, dual_child_emit_fn fn,
                               void *ctx, int logical, nytp_event_kind kind)
{
    dual_impl *di = di_of(sink);
    nytp_status st;

    (void)logical;
    (void)kind;

    if (!di || !di->primary || !di->secondary || !di->primary->ops ||
        !di->secondary->ops) {
        return NYTP_ERR_STATE;
    }

    st = fn(di->primary, ctx);
    if (st != NYTP_OK) {
        di->meta.fanout_fail_primary++;
        return st;
    }

    st = fn(di->secondary, ctx);
    if (st != NYTP_OK) {
        di->meta.fanout_fail_secondary++;
        /*
         * Primary already advanced: any secondary non-OK is partial dual.
         * Map non-sticky codes (STATE/UNSUPPORTED/…) to FAILED so
         * emit_commit sticky-fails the dual parent (Issue 1 review).
         */
        if (!dual_status_is_sticky(st)) {
            return NYTP_ERR_FAILED;
        }
        return st;
    }

    di->meta.fanout_ok++;
    /* COL-003 seq is assigned by the public emit wrapper after we return OK;
     * dual_on_logical_committed then aligns both children. */
    return NYTP_OK;
}

static void dual_on_logical_committed(nytp_sink *sink, nytp_seq seq,
                                      nytp_event_kind kind)
{
    dual_impl *di = di_of(sink);
    if (!di) {
        return;
    }
    commit_child_seq(di->primary, seq, kind);
    commit_child_seq(di->secondary, seq, kind);
}

/* ---- lifecycle ---- */

static const char *dual_name(const nytp_sink *sink)
{
    (void)sink;
    return "dual";
}

static nytp_status activate_one(nytp_sink *c)
{
    nytp_sink_state cs;
    if (!c) {
        return NYTP_ERR_NULL;
    }
    cs = nytp_sink_get_state(c);
    if (cs == NYTP_SINK_OPEN || cs == NYTP_SINK_STOPPED) {
        return nytp_sink_activate(c);
    }
    if (cs == NYTP_SINK_ACTIVE) {
        return NYTP_OK;
    }
    return NYTP_ERR_STATE;
}

static nytp_status dual_activate(nytp_sink *sink)
{
    dual_impl *di = di_of(sink);
    nytp_status st;
    if (!di) {
        return NYTP_ERR_NULL;
    }
    st = activate_one(di->primary);
    if (st != NYTP_OK) {
        return st;
    }
    return activate_one(di->secondary);
}

static nytp_status dual_flush(nytp_sink *sink)
{
    dual_impl *di = di_of(sink);
    nytp_status st;
    if (!di) {
        return NYTP_ERR_NULL;
    }
    st = nytp_sink_flush(di->primary);
    if (st != NYTP_OK) {
        return st;
    }
    return nytp_sink_flush(di->secondary);
}

static nytp_status dual_close(nytp_sink *sink)
{
    dual_impl *di = di_of(sink);
    nytp_status st1, st2;
    if (!di) {
        return NYTP_ERR_NULL;
    }
    /* Finalization ordering: primary then secondary (identical order every run). */
    st1 = NYTP_OK;
    st2 = NYTP_OK;
    if (nytp_sink_get_state(di->primary) != NYTP_SINK_CLOSED) {
        st1 = nytp_sink_close(di->primary);
    }
    if (nytp_sink_get_state(di->secondary) != NYTP_SINK_CLOSED) {
        st2 = nytp_sink_close(di->secondary);
    }
    if (st1 != NYTP_OK) {
        return st1;
    }
    return st2;
}

static nytp_status dual_notify_stop(nytp_sink *sink)
{
    dual_impl *di = di_of(sink);
    nytp_status st;
    if (!di) {
        return NYTP_ERR_STATE;
    }
    if (nytp_sink_get_state(di->primary) == NYTP_SINK_ACTIVE) {
        st = nytp_sink_stop(di->primary);
        if (st != NYTP_OK) {
            return st;
        }
    }
    if (nytp_sink_get_state(di->secondary) == NYTP_SINK_ACTIVE) {
        return nytp_sink_stop(di->secondary);
    }
    return NYTP_OK;
}

static nytp_status begin_finalize_one(nytp_sink *c)
{
    nytp_sink_state cs;
    if (!c) {
        return NYTP_ERR_NULL;
    }
    cs = nytp_sink_get_state(c);
    if (cs == NYTP_SINK_OPEN || cs == NYTP_SINK_ACTIVE ||
        cs == NYTP_SINK_STOPPED) {
        return nytp_sink_begin_finalize(c);
    }
    if (cs == NYTP_SINK_FINALIZING) {
        return NYTP_OK;
    }
    return NYTP_ERR_STATE;
}

static nytp_status dual_notify_begin_finalize(nytp_sink *sink)
{
    dual_impl *di = di_of(sink);
    nytp_status st;
    if (!di) {
        return NYTP_ERR_STATE;
    }
    st = begin_finalize_one(di->primary);
    if (st != NYTP_OK) {
        return st;
    }
    return begin_finalize_one(di->secondary);
}

static nytp_status dual_notify_begin_fork(nytp_sink *sink)
{
    dual_impl *di = di_of(sink);
    nytp_status st;
    if (!di) {
        return NYTP_ERR_STATE;
    }
    if (nytp_sink_get_state(di->primary) == NYTP_SINK_ACTIVE) {
        st = nytp_sink_begin_fork(di->primary);
        if (st != NYTP_OK) {
            return st;
        }
    } else {
        return NYTP_ERR_STATE;
    }
    if (nytp_sink_get_state(di->secondary) == NYTP_SINK_ACTIVE) {
        return nytp_sink_begin_fork(di->secondary);
    }
    return NYTP_ERR_STATE;
}

static nytp_status dual_notify_end_fork_parent(nytp_sink *sink)
{
    dual_impl *di = di_of(sink);
    nytp_status st;
    if (!di) {
        return NYTP_ERR_STATE;
    }
    if (nytp_sink_get_state(di->primary) == NYTP_SINK_FORK_SPLIT) {
        st = nytp_sink_end_fork_parent(di->primary);
        if (st != NYTP_OK) {
            return st;
        }
    }
    if (nytp_sink_get_state(di->secondary) == NYTP_SINK_FORK_SPLIT) {
        return nytp_sink_end_fork_parent(di->secondary);
    }
    return NYTP_OK;
}

static nytp_status dual_notify_end_fork_child(nytp_sink *sink)
{
    dual_impl *di = di_of(sink);
    nytp_status st;
    if (!di) {
        return NYTP_ERR_STATE;
    }
    if (nytp_sink_get_state(di->primary) == NYTP_SINK_FORK_SPLIT) {
        st = nytp_sink_end_fork_child(di->primary);
        if (st != NYTP_OK) {
            return st;
        }
    }
    if (nytp_sink_get_state(di->secondary) == NYTP_SINK_FORK_SPLIT) {
        return nytp_sink_end_fork_child(di->secondary);
    }
    return NYTP_OK;
}

static void dual_destroy(nytp_sink *sink)
{
    dual_impl *di;
    if (!sink) {
        return;
    }
    di = di_of(sink);
    if (di) {
        if (di->owns_primary && di->primary) {
            nytp_sink_destroy(di->primary);
            di->primary = NULL;
        }
        if (di->owns_secondary && di->secondary) {
            nytp_sink_destroy(di->secondary);
            di->secondary = NULL;
        }
        free(di);
    }
    free(sink);
}

/* ---- emit fan-out (ctx packs args) ---- */

typedef struct {
    nytp_string_view key;
    nytp_string_view value;
} kv_ctx;

static nytp_status emit_attr_fn(nytp_sink *c, void *ctx)
{
    kv_ctx *a = (kv_ctx *)ctx;
    if (!c->ops->emit_attribute) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_attribute(c, a->key, a->value);
}

static nytp_status dual_emit_attribute(nytp_sink *sink, nytp_string_view key,
                                       nytp_string_view value)
{
    kv_ctx ctx = {key, value};
    return dual_fanout(sink, emit_attr_fn, &ctx, 1, NYTP_EVT_ATTRIBUTE);
}

static nytp_status emit_opt_fn(nytp_sink *c, void *ctx)
{
    kv_ctx *a = (kv_ctx *)ctx;
    if (!c->ops->emit_option) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_option(c, a->key, a->value);
}

static nytp_status dual_emit_option(nytp_sink *sink, nytp_string_view key,
                                    nytp_string_view value)
{
    kv_ctx ctx = {key, value};
    return dual_fanout(sink, emit_opt_fn, &ctx, 1, NYTP_EVT_OPTION);
}

typedef struct {
    nytp_string_view text;
} text_ctx;

static nytp_status emit_comment_fn(nytp_sink *c, void *ctx)
{
    text_ctx *a = (text_ctx *)ctx;
    if (!c->ops->emit_comment) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_comment(c, a->text);
}

static nytp_status dual_emit_comment(nytp_sink *sink, nytp_string_view text)
{
    text_ctx ctx = {text};
    return dual_fanout(sink, emit_comment_fn, &ctx, 1, NYTP_EVT_COMMENT);
}

typedef struct {
    nytp_ticks ticks;
    nytp_fid fid;
    nytp_line line;
} tl_ctx;

static nytp_status emit_tl_fn(nytp_sink *c, void *ctx)
{
    tl_ctx *a = (tl_ctx *)ctx;
    if (!c->ops->emit_time_line) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_time_line(c, a->ticks, a->fid, a->line);
}

static nytp_status dual_emit_time_line(nytp_sink *sink, nytp_ticks ticks,
                                       nytp_fid fid, nytp_line line)
{
    tl_ctx ctx = {ticks, fid, line};
    return dual_fanout(sink, emit_tl_fn, &ctx, 1, NYTP_EVT_TIME_LINE);
}

typedef struct {
    nytp_ticks ticks;
    nytp_fid fid;
    nytp_line line;
    nytp_line block_line;
    nytp_line sub_line;
} tb_ctx;

static nytp_status emit_tb_fn(nytp_sink *c, void *ctx)
{
    tb_ctx *a = (tb_ctx *)ctx;
    if (!c->ops->emit_time_block) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_time_block(c, a->ticks, a->fid, a->line, a->block_line,
                                   a->sub_line);
}

static nytp_status dual_emit_time_block(nytp_sink *sink, nytp_ticks ticks,
                                        nytp_fid fid, nytp_line line,
                                        nytp_line block_line, nytp_line sub_line)
{
    tb_ctx ctx = {ticks, fid, line, block_line, sub_line};
    return dual_fanout(sink, emit_tb_fn, &ctx, 1, NYTP_EVT_TIME_BLOCK);
}

static nytp_status emit_discount_fn(nytp_sink *c, void *ctx)
{
    (void)ctx;
    if (!c->ops->emit_discount) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_discount(c);
}

static nytp_status dual_emit_discount(nytp_sink *sink)
{
    return dual_fanout(sink, emit_discount_fn, NULL, 1, NYTP_EVT_DISCOUNT);
}

typedef struct {
    nytp_fid fid;
    nytp_fid eval_fid;
    nytp_line eval_line;
    uint32_t flags;
    uint32_t size;
    uint32_t mtime;
    nytp_string_view name;
} new_fid_ctx;

static nytp_status emit_new_fid_fn(nytp_sink *c, void *ctx)
{
    new_fid_ctx *a = (new_fid_ctx *)ctx;
    if (!c->ops->emit_new_fid) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_new_fid(c, a->fid, a->eval_fid, a->eval_line, a->flags,
                                a->size, a->mtime, a->name);
}

static nytp_status dual_emit_new_fid(nytp_sink *sink, nytp_fid fid,
                                     nytp_fid eval_fid, nytp_line eval_line,
                                     uint32_t flags, uint32_t size,
                                     uint32_t mtime, nytp_string_view name)
{
    new_fid_ctx ctx = {fid, eval_fid, eval_line, flags, size, mtime, name};
    return dual_fanout(sink, emit_new_fid_fn, &ctx, 1, NYTP_EVT_NEW_FID);
}

typedef struct {
    nytp_fid fid;
    nytp_line line;
    nytp_string_view text;
} src_ctx;

static nytp_status emit_src_fn(nytp_sink *c, void *ctx)
{
    src_ctx *a = (src_ctx *)ctx;
    if (!c->ops->emit_src_line) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_src_line(c, a->fid, a->line, a->text);
}

static nytp_status dual_emit_src_line(nytp_sink *sink, nytp_fid fid,
                                      nytp_line line, nytp_string_view text)
{
    src_ctx ctx = {fid, line, text};
    return dual_fanout(sink, emit_src_fn, &ctx, 1, NYTP_EVT_SRC_LINE);
}

typedef struct {
    nytp_fid fid;
    nytp_line first_line;
    nytp_line last_line;
    nytp_string_view name;
} sub_info_ctx;

static nytp_status emit_sub_info_fn(nytp_sink *c, void *ctx)
{
    sub_info_ctx *a = (sub_info_ctx *)ctx;
    if (!c->ops->emit_sub_info) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_sub_info(c, a->fid, a->first_line, a->last_line,
                                 a->name);
}

static nytp_status dual_emit_sub_info(nytp_sink *sink, nytp_fid fid,
                                      nytp_line first_line, nytp_line last_line,
                                      nytp_string_view name)
{
    sub_info_ctx ctx = {fid, first_line, last_line, name};
    return dual_fanout(sink, emit_sub_info_fn, &ctx, 1, NYTP_EVT_SUB_INFO);
}

typedef struct {
    nytp_fid fid;
    nytp_line line;
    uint32_t count;
    double incl;
    double excl;
    double reci;
    uint32_t rec_depth;
    nytp_string_view called;
    nytp_string_view caller;
} sub_callers_ctx;

static nytp_status emit_sub_callers_fn(nytp_sink *c, void *ctx)
{
    sub_callers_ctx *a = (sub_callers_ctx *)ctx;
    if (!c->ops->emit_sub_callers) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_sub_callers(c, a->fid, a->line, a->count, a->incl,
                                    a->excl, a->reci, a->rec_depth, a->called,
                                    a->caller);
}

static nytp_status dual_emit_sub_callers(nytp_sink *sink, nytp_fid fid,
                                         nytp_line line, uint32_t count,
                                         double incl, double excl, double reci,
                                         uint32_t rec_depth,
                                         nytp_string_view called,
                                         nytp_string_view caller)
{
    sub_callers_ctx ctx = {fid,   line, count,  incl,   excl,
                           reci,  rec_depth, called, caller};
    return dual_fanout(sink, emit_sub_callers_fn, &ctx, 1,
                       NYTP_EVT_SUB_CALLERS);
}

typedef struct {
    nytp_pid pid;
    nytp_pid ppid;
    double start_time;
} pid_start_ctx;

static nytp_status emit_pid_start_fn(nytp_sink *c, void *ctx)
{
    pid_start_ctx *a = (pid_start_ctx *)ctx;
    if (!c->ops->emit_pid_start) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_pid_start(c, a->pid, a->ppid, a->start_time);
}

static nytp_status dual_emit_pid_start(nytp_sink *sink, nytp_pid pid,
                                       nytp_pid ppid, double start_time)
{
    pid_start_ctx ctx = {pid, ppid, start_time};
    return dual_fanout(sink, emit_pid_start_fn, &ctx, 1, NYTP_EVT_PID_START);
}

typedef struct {
    nytp_pid pid;
    double end_time;
} pid_end_ctx;

static nytp_status emit_pid_end_fn(nytp_sink *c, void *ctx)
{
    pid_end_ctx *a = (pid_end_ctx *)ctx;
    if (!c->ops->emit_pid_end) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_pid_end(c, a->pid, a->end_time);
}

static nytp_status dual_emit_pid_end(nytp_sink *sink, nytp_pid pid,
                                     double end_time)
{
    pid_end_ctx ctx = {pid, end_time};
    return dual_fanout(sink, emit_pid_end_fn, &ctx, 1, NYTP_EVT_PID_END);
}

typedef struct {
    nytp_fid caller_fid;
    nytp_line caller_line;
} sub_entry_ctx;

static nytp_status emit_sub_entry_fn(nytp_sink *c, void *ctx)
{
    sub_entry_ctx *a = (sub_entry_ctx *)ctx;
    if (!c->ops->emit_sub_entry) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_sub_entry(c, a->caller_fid, a->caller_line);
}

static nytp_status dual_emit_sub_entry(nytp_sink *sink, nytp_fid caller_fid,
                                       nytp_line caller_line)
{
    sub_entry_ctx ctx = {caller_fid, caller_line};
    return dual_fanout(sink, emit_sub_entry_fn, &ctx, 1, NYTP_EVT_SUB_ENTRY);
}

typedef struct {
    nytp_depth depth;
    double incl_time;
    double excl_time;
    nytp_string_view subname;
} sub_return_ctx;

static nytp_status emit_sub_return_fn(nytp_sink *c, void *ctx)
{
    sub_return_ctx *a = (sub_return_ctx *)ctx;
    if (!c->ops->emit_sub_return) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_sub_return(c, a->depth, a->incl_time, a->excl_time,
                                   a->subname);
}

static nytp_status dual_emit_sub_return(nytp_sink *sink, nytp_depth depth,
                                        double incl_time, double excl_time,
                                        nytp_string_view subname)
{
    sub_return_ctx ctx = {depth, incl_time, excl_time, subname};
    return dual_fanout(sink, emit_sub_return_fn, &ctx, 1, NYTP_EVT_SUB_RETURN);
}

static nytp_status emit_start_deflate_fn(nytp_sink *c, void *ctx)
{
    (void)ctx;
    if (!c->ops->emit_start_deflate) {
        return NYTP_ERR_UNSUPPORTED;
    }
    return c->ops->emit_start_deflate(c);
}

static nytp_status dual_emit_start_deflate(nytp_sink *sink)
{
    /* Control: no COL-003 seq; still fans out to both children. */
    return dual_fanout(sink, emit_start_deflate_fn, NULL, 0,
                       NYTP_EVT_START_DEFLATE);
}

static const nytp_sink_ops DUAL_OPS = {
    .name = dual_name,
    .activate = dual_activate,
    .flush = dual_flush,
    .close = dual_close,
    .destroy = dual_destroy,
    .emit_attribute = dual_emit_attribute,
    .emit_option = dual_emit_option,
    .emit_comment = dual_emit_comment,
    .emit_time_line = dual_emit_time_line,
    .emit_time_block = dual_emit_time_block,
    .emit_discount = dual_emit_discount,
    .emit_new_fid = dual_emit_new_fid,
    .emit_src_line = dual_emit_src_line,
    .emit_sub_info = dual_emit_sub_info,
    .emit_sub_callers = dual_emit_sub_callers,
    .emit_pid_start = dual_emit_pid_start,
    .emit_pid_end = dual_emit_pid_end,
    .emit_sub_entry = dual_emit_sub_entry,
    .emit_sub_return = dual_emit_sub_return,
    .emit_start_deflate = dual_emit_start_deflate,
    .on_logical_committed = dual_on_logical_committed,
    .notify_stop = dual_notify_stop,
    .notify_begin_finalize = dual_notify_begin_finalize,
    .notify_begin_fork = dual_notify_begin_fork,
    .notify_end_fork_parent = dual_notify_end_fork_parent,
    .notify_end_fork_child = dual_notify_end_fork_child,
};

/* ---- public API ---- */

nytp_sink *nytp_dual_sink_create(nytp_sink *primary, nytp_sink *secondary,
                                 int owns_primary, int owns_secondary)
{
    nytp_sink *s;
    dual_impl *di;
    if (!primary || !secondary) {
        return NULL;
    }
    di = (dual_impl *)calloc(1, sizeof(*di));
    s = (nytp_sink *)calloc(1, sizeof(*s));
    if (!di || !s) {
        free(di);
        free(s);
        return NULL;
    }
    di->primary = primary;
    di->secondary = secondary;
    di->owns_primary = owns_primary ? 1 : 0;
    di->owns_secondary = owns_secondary ? 1 : 0;
    meta_init(&di->meta);
    meta_refresh_names(di);

    s->ops = &DUAL_OPS;
    s->state = NYTP_SINK_OPEN;
    s->impl = di;
    s->next_seq = 0;
    s->last_seq = 0;
    s->has_last_seq = 0;
    s->fail_reason = NYTP_OK;
    return s;
}

nytp_sink *nytp_dual_sink_create_v5_v6(const char *path_v5, const char *path_v6)
{
    nytp_sink *v5 = nytp_v5_sink_create(path_v5);
    nytp_sink *v6 = nytp_v6_sink_create(path_v6);
    nytp_sink *dual;
    if (!v5 || !v6) {
        if (v5) {
            nytp_sink_destroy(v5);
        }
        if (v6) {
            nytp_sink_destroy(v6);
        }
        return NULL;
    }
    dual = nytp_dual_sink_create(v5, v6, 1, 1);
    if (!dual) {
        nytp_sink_destroy(v5);
        nytp_sink_destroy(v6);
        return NULL;
    }
    return dual;
}

int nytp_dual_sink_is_dual(const nytp_sink *sink)
{
    return sink && sink->ops == &DUAL_OPS;
}

nytp_sink *nytp_dual_sink_primary(nytp_sink *sink)
{
    dual_impl *di = di_of(sink);
    if (!nytp_dual_sink_is_dual(sink) || !di) {
        return NULL;
    }
    return di->primary;
}

nytp_sink *nytp_dual_sink_secondary(nytp_sink *sink)
{
    dual_impl *di = di_of(sink);
    if (!nytp_dual_sink_is_dual(sink) || !di) {
        return NULL;
    }
    return di->secondary;
}

const nytp_sink *nytp_dual_sink_primary_const(const nytp_sink *sink)
{
    const dual_impl *di = di_of_c(sink);
    if (!nytp_dual_sink_is_dual(sink) || !di) {
        return NULL;
    }
    return di->primary;
}

const nytp_sink *nytp_dual_sink_secondary_const(const nytp_sink *sink)
{
    const dual_impl *di = di_of_c(sink);
    if (!nytp_dual_sink_is_dual(sink) || !di) {
        return NULL;
    }
    return di->secondary;
}

const nytp_dual_compare_meta *nytp_dual_sink_meta(const nytp_sink *sink)
{
    const dual_impl *di = di_of_c(sink);
    if (!nytp_dual_sink_is_dual(sink) || !di) {
        return NULL;
    }
    return &di->meta;
}

static int env_truthy(const char *v)
{
    if (!v || !*v) {
        return 0;
    }
    if (v[0] == '1' && v[1] == '\0') {
        return 1;
    }
    if (v[0] == '0' && v[1] == '\0') {
        return 0;
    }
    /* case-insensitive true/yes/on */
    {
        char buf[8];
        size_t n = 0;
        while (v[n] && n < sizeof(buf) - 1) {
            buf[n] = (char)tolower((unsigned char)v[n]);
            n++;
        }
        buf[n] = '\0';
        if (strcmp(buf, "true") == 0 || strcmp(buf, "yes") == 0 ||
            strcmp(buf, "on") == 0) {
            return 1;
        }
    }
    return 0;
}

static int env_eq_ci(const char *v, const char *expect)
{
    size_t i;
    if (!v || !expect) {
        return 0;
    }
    for (i = 0; expect[i] || v[i]; i++) {
        unsigned char a = (unsigned char)v[i];
        unsigned char b = (unsigned char)expect[i];
        if (tolower(a) != tolower(b)) {
            return 0;
        }
    }
    return 1;
}

int nytp_dual_env_enabled(void)
{
    const char *dual = getenv("NYTPROF_DUAL_SINK");
    const char *fmt = getenv("NYTPROF_FORMAT");
    if (env_truthy(dual)) {
        return 1;
    }
    if (fmt && env_eq_ci(fmt, "dual")) {
        return 1;
    }
    return 0;
}

const nytp_counting_stats *nytp_dual_child_stats(const nytp_sink *child)
{
    const nytp_counting_stats *st;
    if (!child) {
        return NULL;
    }
    st = nytp_counting_sink_stats(child);
    if (st) {
        return st;
    }
    st = nytp_v5_sink_stats(child);
    if (st) {
        return st;
    }
    st = nytp_v6_sink_stats(child);
    if (st) {
        return st;
    }
    return NULL;
}

int nytp_dual_sink_logical_equal(nytp_sink *sink)
{
    dual_impl *di = di_of(sink);
    const nytp_counting_stats *a;
    const nytp_counting_stats *b;
    size_t i;

    if (!nytp_dual_sink_is_dual(sink) || !di) {
        return 0;
    }
    meta_refresh_names(di);
    di->meta.logical_equal_checks++;
    di->meta.first_kind_mismatch = (size_t)-1;
    di->meta.first_seq_mismatch = (size_t)-1;

    a = nytp_dual_child_stats(di->primary);
    b = nytp_dual_child_stats(di->secondary);
    if (!a || !b) {
        di->meta.last_equal = 0;
        return 0;
    }

    if (a->logical_emits != b->logical_emits) {
        di->meta.last_equal = 0;
        return 0;
    }
    for (i = 0; i < (size_t)NYTP_EVT_KIND_COUNT; i++) {
        if (a->by_kind[i] != b->by_kind[i]) {
            di->meta.first_kind_mismatch = i;
            di->meta.last_equal = 0;
            return 0;
        }
    }
    if (a->seq_ring_len != b->seq_ring_len) {
        di->meta.last_equal = 0;
        return 0;
    }
    for (i = 0; i < a->seq_ring_len; i++) {
        if (a->seq_ring[i] != b->seq_ring[i] ||
            a->kind_ring[i] != b->kind_ring[i]) {
            di->meta.first_seq_mismatch = i;
            di->meta.last_equal = 0;
            return 0;
        }
    }

    di->meta.last_equal = 1;
    di->meta.logical_equal_ok++;
    return 1;
}

nytp_status nytp_dual_sink_write_compare_meta(const nytp_sink *sink,
                                              const char *path)
{
    const dual_impl *di = di_of_c(sink);
    FILE *fp;
    const nytp_counting_stats *a;
    const nytp_counting_stats *b;
    if (!nytp_dual_sink_is_dual(sink) || !di || !path || !*path) {
        return NYTP_ERR_NULL;
    }
    fp = fopen(path, "wb");
    if (!fp) {
        return NYTP_ERR_IO;
    }
    a = nytp_dual_child_stats(di->primary);
    b = nytp_dual_child_stats(di->secondary);
    fprintf(fp,
            "{\n"
            "  \"mode\": \"test_dev_only\",\n"
            "  \"oq4\": \"COL-014 dual-sink not product UX\",\n"
            "  \"primary\": \"%s\",\n"
            "  \"secondary\": \"%s\",\n"
            "  \"fanout_ok\": %llu,\n"
            "  \"fanout_fail_primary\": %llu,\n"
            "  \"fanout_fail_secondary\": %llu,\n"
            "  \"logical_equal_checks\": %llu,\n"
            "  \"logical_equal_ok\": %llu,\n"
            "  \"last_equal\": %d,\n"
            "  \"primary_logical_emits\": %llu,\n"
            "  \"secondary_logical_emits\": %llu\n"
            "}\n",
            di->meta.primary_name ? di->meta.primary_name : "(null)",
            di->meta.secondary_name ? di->meta.secondary_name : "(null)",
            (unsigned long long)di->meta.fanout_ok,
            (unsigned long long)di->meta.fanout_fail_primary,
            (unsigned long long)di->meta.fanout_fail_secondary,
            (unsigned long long)di->meta.logical_equal_checks,
            (unsigned long long)di->meta.logical_equal_ok, di->meta.last_equal,
            (unsigned long long)(a ? a->logical_emits : 0),
            (unsigned long long)(b ? b->logical_emits : 0));
    if (fclose(fp) != 0) {
        return NYTP_ERR_IO;
    }
    return NYTP_OK;
}

nytp_status nytp_dual_sink_fork_child_reinit(nytp_sink *dual,
                                             const char *path_v5,
                                             const char *path_v6)
{
    dual_impl *di;
    nytp_status st = NYTP_OK;
    nytp_status st2;
    if (!nytp_dual_sink_is_dual(dual) || !dual->impl) {
        return NYTP_ERR_NULL;
    }
    di = (dual_impl *)dual->impl;
    if (di->primary) {
        if (nytp_v5_sink_is_v5(di->primary)) {
            st2 = nytp_v5_sink_fork_child_reinit(di->primary, path_v5);
            if (st2 != NYTP_OK && st == NYTP_OK) {
                st = st2;
            }
        } else if (nytp_v6_sink_is_v6(di->primary)) {
            st2 = nytp_v6_sink_fork_child_reinit(di->primary, path_v6);
            if (st2 != NYTP_OK && st == NYTP_OK) {
                st = st2;
            }
        }
    }
    if (di->secondary) {
        if (nytp_v5_sink_is_v5(di->secondary)) {
            st2 = nytp_v5_sink_fork_child_reinit(di->secondary, path_v5);
            if (st2 != NYTP_OK && st == NYTP_OK) {
                st = st2;
            }
        } else if (nytp_v6_sink_is_v6(di->secondary)) {
            st2 = nytp_v6_sink_fork_child_reinit(di->secondary, path_v6);
            if (st2 != NYTP_OK && st == NYTP_OK) {
                st = st2;
            }
        }
    }
    return st;
}

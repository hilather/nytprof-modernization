/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-004 no-alloc statement fast path + COL-005 bounded batching.
 */
#define _POSIX_C_SOURCE 200809L

#include "nytp_batch.h"
#include "nytp_sink_counting.h"

#include <stdlib.h>
#include <string.h>
#include <time.h>

/* ---- helpers ---- */

static void metrics_zero(nytp_batch_metrics *m)
{
    memset(m, 0, sizeof(*m));
}

static nytp_status ensure_slot(nytp_batch *b)
{
    if (!b) {
        return NYTP_ERR_NULL;
    }
    b->last_append_buffered = 0;
    if (!b->child || !b->child->ops) {
        return NYTP_ERR_STATE;
    }
    if (b->count < b->capacity) {
        return NYTP_OK;
    }
    /* Capacity full: must flush before append. */
    {
        nytp_status st = nytp_batch_flush(b);
        if (st != NYTP_OK) {
            return st;
        }
        b->metrics.full_flushes++;
    }
    if (b->count >= b->capacity) {
        return NYTP_ERR_OVERFLOW;
    }
    return NYTP_OK;
}

static nytp_status maybe_high_water(nytp_batch *b)
{
    if (b->count >= b->high_water) {
        nytp_status st = nytp_batch_flush(b);
        if (st != NYTP_OK) {
            return st;
        }
        b->metrics.high_water_flushes++;
    }
    return NYTP_OK;
}

/*
 * Copy bytes into arena. On insufficient space: flush once, then retry.
 * If still too large for empty arena: return NYTP_ERR_OVERFLOW (caller may
 * take emergency direct path).
 */
static nytp_status arena_copy(nytp_batch *b, const char *ptr, size_t len,
                              int is_utf8, nytp_arena_str *out)
{
    if (!out) {
        return NYTP_ERR_NULL;
    }
    out->off = 0;
    out->len = 0;
    out->is_utf8 = is_utf8 ? 1 : 0;
    if (len == 0) {
        return NYTP_OK;
    }
    if (!ptr) {
        return NYTP_ERR_NULL;
    }
    if (len > b->arena_cap) {
        return NYTP_ERR_OVERFLOW; /* larger than entire arena */
    }
    if (b->arena_used + len > b->arena_cap) {
        nytp_status st = nytp_batch_flush(b);
        if (st != NYTP_OK) {
            return st;
        }
        /* After flush arena_used == 0; re-check. */
        if (b->arena_used + len > b->arena_cap) {
            return NYTP_ERR_OVERFLOW;
        }
    }
    out->off = (uint32_t)b->arena_used;
    out->len = (uint32_t)len;
    out->is_utf8 = is_utf8 ? 1 : 0;
    memcpy(b->arena + b->arena_used, ptr, len);
    b->arena_used += len;
    b->metrics.arena_bytes_copied += (uint64_t)len;
    return NYTP_OK;
}

static nytp_string_view arena_to_sv(const nytp_batch *b, nytp_arena_str s)
{
    nytp_string_view sv;
    if (s.len == 0) {
        sv.ptr = NULL;
        sv.len = 0;
        sv.is_utf8 = s.is_utf8;
        return sv;
    }
    sv.ptr = b->arena + s.off;
    sv.len = s.len;
    sv.is_utf8 = s.is_utf8;
    return sv;
}

/* Stamp event into slot; advances count; updates metrics. */
static void commit_event(nytp_batch *b, const nytp_event *ev, int is_stmt_fast)
{
    b->events[b->count] = *ev;
    b->count++;
    b->metrics.appends++;
    b->last_append_buffered = 1;
    if (is_stmt_fast) {
        b->metrics.stmt_fast_appends++;
    }
}

/* ---- create / destroy ---- */

nytp_batch *nytp_batch_create(size_t capacity, size_t arena_cap,
                              size_t high_water)
{
    nytp_batch *b;
    if (capacity < NYTP_BATCH_MIN_CAPACITY ||
        capacity > NYTP_BATCH_MAX_CAPACITY) {
        return NULL;
    }
    if (arena_cap == 0 || arena_cap > NYTP_BATCH_MAX_ARENA) {
        return NULL;
    }
    if (high_water == 0) {
        high_water = capacity;
    }
    if (high_water > capacity) {
        high_water = capacity;
    }
    if (high_water < 1) {
        high_water = 1;
    }

    b = (nytp_batch *)calloc(1, sizeof(*b));
    if (!b) {
        return NULL;
    }
    b->events = (nytp_event *)calloc(capacity, sizeof(nytp_event));
    b->arena = (char *)malloc(arena_cap);
    b->compact_tmp = (char *)malloc(arena_cap);
    if (!b->events || !b->arena || !b->compact_tmp) {
        free(b->events);
        free(b->arena);
        free(b->compact_tmp);
        free(b);
        return NULL;
    }
    b->capacity = capacity;
    b->high_water = high_water;
    b->arena_cap = arena_cap;
    b->count = 0;
    b->arena_used = 0;
    metrics_zero(&b->metrics);
    /* create-time heap: batch + events + arena + compact_tmp (4). No grow. */
    b->metrics.heap_allocs = 4;
    b->child = NULL;
    b->owns_child = 0;
    b->child_ops = NULL;
    b->last_append_buffered = 0;
    return b;
}

void nytp_batch_destroy(nytp_batch *batch)
{
    if (!batch) {
        return;
    }
    if (batch->owns_child && batch->child) {
        nytp_sink_destroy(batch->child);
        batch->child = NULL;
    }
    free(batch->events);
    free(batch->arena);
    free(batch->compact_tmp);
    free(batch);
}

void nytp_batch_set_child(nytp_batch *batch, nytp_sink *child)
{
    if (!batch) {
        return;
    }
    batch->child = child;
    batch->child_ops = (child && child->ops) ? child->ops : NULL;
}

void nytp_batch_set_owns_child(nytp_batch *batch, int owns)
{
    if (batch) {
        batch->owns_child = owns ? 1 : 0;
    }
}

size_t nytp_batch_count(const nytp_batch *batch)
{
    return batch ? batch->count : 0;
}

size_t nytp_batch_capacity(const nytp_batch *batch)
{
    return batch ? batch->capacity : 0;
}

size_t nytp_batch_arena_used(const nytp_batch *batch)
{
    return batch ? batch->arena_used : 0;
}

const nytp_batch_metrics *nytp_batch_get_metrics(const nytp_batch *batch)
{
    return batch ? &batch->metrics : NULL;
}

int nytp_batch_last_append_buffered(const nytp_batch *batch)
{
    return batch ? batch->last_append_buffered : 0;
}

size_t nytp_batch_pending(const nytp_batch *batch)
{
    return batch ? batch->count : 0;
}

void nytp_batch_discard_pending(nytp_batch *batch)
{
    if (!batch) {
        return;
    }
    batch->count = 0;
    batch->arena_used = 0;
    batch->last_append_buffered = 0;
}

/* ---- compact after partial flush (drop acked prefix; rebuild arena) ---- */

static nytp_status reloc_str(char *dst, size_t *used, size_t cap,
                             const char *src_arena, nytp_arena_str *s)
{
    if (!s) {
        return NYTP_ERR_NULL;
    }
    if (s->len == 0) {
        s->off = 0;
        return NYTP_OK;
    }
    if (*used + s->len > cap) {
        return NYTP_ERR_OVERFLOW;
    }
    memcpy(dst + *used, src_arena + s->off, s->len);
    s->off = (uint32_t)(*used);
    *used += s->len;
    return NYTP_OK;
}

static nytp_status reloc_event_strings(char *dst, size_t *used, size_t cap,
                                       const char *src, nytp_event *ev)
{
    nytp_status st;
    switch (ev->kind) {
    case NYTP_EVT_ATTRIBUTE:
    case NYTP_EVT_OPTION:
        st = reloc_str(dst, used, cap, src, &ev->u.attr.key);
        if (st != NYTP_OK) {
            return st;
        }
        return reloc_str(dst, used, cap, src, &ev->u.attr.value);
    case NYTP_EVT_COMMENT:
        return reloc_str(dst, used, cap, src, &ev->u.comment.text);
    case NYTP_EVT_NEW_FID:
        return reloc_str(dst, used, cap, src, &ev->u.new_fid.name);
    case NYTP_EVT_SRC_LINE:
        return reloc_str(dst, used, cap, src, &ev->u.src_line.text);
    case NYTP_EVT_SUB_INFO:
        return reloc_str(dst, used, cap, src, &ev->u.sub_info.name);
    case NYTP_EVT_SUB_CALLERS:
        st = reloc_str(dst, used, cap, src, &ev->u.sub_callers.called);
        if (st != NYTP_OK) {
            return st;
        }
        return reloc_str(dst, used, cap, src, &ev->u.sub_callers.caller);
    case NYTP_EVT_SUB_RETURN:
        return reloc_str(dst, used, cap, src, &ev->u.sub_return.subname);
    default:
        return NYTP_OK; /* POD-only kinds */
    }
}

/*
 * Drop events[0..first_unacked) (already acked). Rebuild arena for remaining.
 * Uses preallocated compact_tmp (no heap on fail path).
 */
static nytp_status compact_unacked(nytp_batch *b, size_t first_unacked)
{
    size_t n;
    size_t i;
    size_t used = 0;
    nytp_status st;

    if (first_unacked == 0) {
        return NYTP_OK;
    }
    if (first_unacked >= b->count) {
        b->count = 0;
        b->arena_used = 0;
        return NYTP_OK;
    }
    n = b->count - first_unacked;
    /* Move headers first; string offsets still valid in old arena. */
    memmove(b->events, b->events + first_unacked, n * sizeof(nytp_event));
    b->count = n;

    if (!b->compact_tmp) {
        return NYTP_ERR_OVERFLOW;
    }
    for (i = 0; i < n; i++) {
        st = reloc_event_strings(b->compact_tmp, &used, b->arena_cap, b->arena,
                                 &b->events[i]);
        if (st != NYTP_OK) {
            return st;
        }
    }
    if (used > 0) {
        memcpy(b->arena, b->compact_tmp, used);
    }
    b->arena_used = used;
    return NYTP_OK;
}

/* ---- flush / replay ---- */

static nytp_status replay_one(nytp_batch *b, const nytp_event *ev)
{
    nytp_sink *c = b->child;
    const nytp_sink_ops *ops = c->ops;
    nytp_status st = NYTP_ERR_UNSUPPORTED;
    int logical = nytp_event_kind_is_logical(ev->kind);

    switch (ev->kind) {
    case NYTP_EVT_ATTRIBUTE:
        if (!ops->emit_attribute) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_attribute(c, arena_to_sv(b, ev->u.attr.key),
                                 arena_to_sv(b, ev->u.attr.value));
        break;
    case NYTP_EVT_OPTION:
        if (!ops->emit_option) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_option(c, arena_to_sv(b, ev->u.attr.key),
                              arena_to_sv(b, ev->u.attr.value));
        break;
    case NYTP_EVT_COMMENT:
        if (!ops->emit_comment) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_comment(c, arena_to_sv(b, ev->u.comment.text));
        break;
    case NYTP_EVT_TIME_LINE:
        if (!ops->emit_time_line) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_time_line(c, ev->u.time_line.ticks, ev->u.time_line.fid,
                                 ev->u.time_line.line);
        break;
    case NYTP_EVT_TIME_BLOCK:
        if (!ops->emit_time_block) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_time_block(c, ev->u.time_block.ticks,
                                  ev->u.time_block.fid, ev->u.time_block.line,
                                  ev->u.time_block.block_line,
                                  ev->u.time_block.sub_line);
        break;
    case NYTP_EVT_DISCOUNT:
        if (!ops->emit_discount) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_discount(c);
        break;
    case NYTP_EVT_NEW_FID:
        if (!ops->emit_new_fid) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_new_fid(c, ev->u.new_fid.fid, ev->u.new_fid.eval_fid,
                               ev->u.new_fid.eval_line, ev->u.new_fid.flags,
                               ev->u.new_fid.size, ev->u.new_fid.mtime,
                               arena_to_sv(b, ev->u.new_fid.name));
        break;
    case NYTP_EVT_SRC_LINE:
        if (!ops->emit_src_line) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_src_line(c, ev->u.src_line.fid, ev->u.src_line.line,
                                arena_to_sv(b, ev->u.src_line.text));
        break;
    case NYTP_EVT_SUB_INFO:
        if (!ops->emit_sub_info) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_sub_info(c, ev->u.sub_info.fid, ev->u.sub_info.first_line,
                                ev->u.sub_info.last_line,
                                arena_to_sv(b, ev->u.sub_info.name));
        break;
    case NYTP_EVT_SUB_CALLERS:
        if (!ops->emit_sub_callers) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_sub_callers(
            c, ev->u.sub_callers.fid, ev->u.sub_callers.line,
            ev->u.sub_callers.count, ev->u.sub_callers.incl,
            ev->u.sub_callers.excl, ev->u.sub_callers.reci,
            ev->u.sub_callers.rec_depth, arena_to_sv(b, ev->u.sub_callers.called),
            arena_to_sv(b, ev->u.sub_callers.caller));
        break;
    case NYTP_EVT_PID_START:
        if (!ops->emit_pid_start) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_pid_start(c, ev->u.pid_start.pid, ev->u.pid_start.ppid,
                                 ev->u.pid_start.start_time);
        break;
    case NYTP_EVT_PID_END:
        if (!ops->emit_pid_end) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_pid_end(c, ev->u.pid_end.pid, ev->u.pid_end.end_time);
        break;
    case NYTP_EVT_SUB_ENTRY:
        if (!ops->emit_sub_entry) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_sub_entry(c, ev->u.sub_entry.caller_fid,
                                 ev->u.sub_entry.caller_line);
        break;
    case NYTP_EVT_SUB_RETURN:
        if (!ops->emit_sub_return) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_sub_return(c, ev->u.sub_return.depth,
                                  ev->u.sub_return.incl_time,
                                  ev->u.sub_return.excl_time,
                                  arena_to_sv(b, ev->u.sub_return.subname));
        break;
    case NYTP_EVT_START_DEFLATE:
        if (!ops->emit_start_deflate) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = ops->emit_start_deflate(c);
        logical = 0;
        break;
    default:
        return NYTP_ERR_UNSUPPORTED;
    }

    if (st != NYTP_OK) {
        return st;
    }

    /*
     * Preserve batch-assigned COL-003 seq on the child for dual compare:
     * update child seq state + optional post-commit hook. Do not call
     * public nytp_emit_* (would re-assign seq).
     */
    if (logical) {
        c->last_seq = ev->seq;
        c->next_seq = ev->seq + 1;
        c->has_last_seq = 1;
        if (ops->on_logical_committed) {
            ops->on_logical_committed(c, ev->seq, ev->kind);
        }
    }
    return NYTP_OK;
}

nytp_status nytp_batch_flush(nytp_batch *batch)
{
    size_t i;
    if (!batch) {
        return NYTP_ERR_NULL;
    }
    if (!batch->child || !batch->child->ops) {
        return NYTP_ERR_STATE;
    }
    if (batch->count == 0) {
        return NYTP_OK;
    }

    for (i = 0; i < batch->count; i++) {
        nytp_status st = replay_one(batch, &batch->events[i]);
        if (st != NYTP_OK) {
            /*
             * Drop already-acked prefix so a later flush cannot re-emit it.
             * Retain only events[i..count) as the new pending set.
             */
            (void)compact_unacked(batch, i);
            if (st == NYTP_ERR_IO || st == NYTP_ERR_FAILED ||
                st == NYTP_ERR_OVERFLOW) {
                (void)nytp_sink_mark_failed(batch->child, st);
            }
            return st;
        }
    }

    /* Full ack: reset only after all events drained. */
    batch->count = 0;
    batch->arena_used = 0;
    batch->metrics.flushes++;
    return NYTP_OK;
}

/* ---- POD fast appends (COL-004) ---- */

nytp_status nytp_batch_append_time_line(nytp_batch *batch, nytp_seq seq,
                                        nytp_ticks ticks, nytp_fid fid,
                                        nytp_line line)
{
    nytp_status st;
    nytp_event ev;
    st = ensure_slot(batch);
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_TIME_LINE;
    ev.seq = seq;
    ev.u.time_line.ticks = ticks;
    ev.u.time_line.fid = fid;
    ev.u.time_line.line = line;
    commit_event(batch, &ev, 1);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_time_block(nytp_batch *batch, nytp_seq seq,
                                         nytp_ticks ticks, nytp_fid fid,
                                         nytp_line line, nytp_line block_line,
                                         nytp_line sub_line)
{
    nytp_status st;
    nytp_event ev;
    st = ensure_slot(batch);
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_TIME_BLOCK;
    ev.seq = seq;
    ev.u.time_block.ticks = ticks;
    ev.u.time_block.fid = fid;
    ev.u.time_block.line = line;
    ev.u.time_block.block_line = block_line;
    ev.u.time_block.sub_line = sub_line;
    commit_event(batch, &ev, 1);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_discount(nytp_batch *batch, nytp_seq seq)
{
    nytp_status st;
    nytp_event ev;
    st = ensure_slot(batch);
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_DISCOUNT;
    ev.seq = seq;
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_sub_entry(nytp_batch *batch, nytp_seq seq,
                                        nytp_fid caller_fid,
                                        nytp_line caller_line)
{
    nytp_status st;
    nytp_event ev;
    st = ensure_slot(batch);
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_SUB_ENTRY;
    ev.seq = seq;
    ev.u.sub_entry.caller_fid = caller_fid;
    ev.u.sub_entry.caller_line = caller_line;
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

/* ---- string-bearing appends ---- */

/*
 * Ensure room for one event + `need` arena bytes. Flushes if needed.
 * If need > arena_cap after empty flush, returns OVERFLOW (emergency path).
 */
static nytp_status prepare_with_arena(nytp_batch *b, size_t need)
{
    nytp_status st = ensure_slot(b);
    if (st != NYTP_OK) {
        return st;
    }
    if (need == 0) {
        return NYTP_OK;
    }
    if (need > b->arena_cap) {
        return NYTP_ERR_OVERFLOW;
    }
    if (b->arena_used + need > b->arena_cap) {
        st = nytp_batch_flush(b);
        if (st != NYTP_OK) {
            return st;
        }
        /* After flush, need a free slot again. */
        st = ensure_slot(b);
        if (st != NYTP_OK) {
            return st;
        }
        if (b->arena_used + need > b->arena_cap) {
            return NYTP_ERR_OVERFLOW;
        }
    }
    return NYTP_OK;
}

/* Emergency: emit one string-bearing event straight to child (no buffer). */
static nytp_status emergency_emit_src_line(nytp_batch *b, nytp_seq seq,
                                           nytp_fid fid, nytp_line line,
                                           nytp_string_view text)
{
    nytp_event ev;
    nytp_status st;
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_SRC_LINE;
    ev.seq = seq;
    ev.u.src_line.fid = fid;
    ev.u.src_line.line = line;
    /* Temporarily place text: flush any pending first, then direct ops. */
    st = nytp_batch_flush(b);
    if (st != NYTP_OK) {
        return st;
    }
    if (!b->child || !b->child->ops || !b->child->ops->emit_src_line) {
        return NYTP_ERR_UNSUPPORTED;
    }
    st = b->child->ops->emit_src_line(b->child, fid, line, text);
    if (st != NYTP_OK) {
        return st;
    }
    b->child->last_seq = seq;
    b->child->next_seq = seq + 1;
    b->child->has_last_seq = 1;
    if (b->child->ops->on_logical_committed) {
        b->child->ops->on_logical_committed(b->child, seq, NYTP_EVT_SRC_LINE);
    }
    b->metrics.emergency_direct++;
    b->metrics.appends++;
    return NYTP_OK;
}

nytp_status nytp_batch_append_attribute(nytp_batch *batch, nytp_seq seq,
                                        nytp_string_view key,
                                        nytp_string_view value)
{
    nytp_status st;
    nytp_event ev;
    size_t need = key.len + value.len;
    st = prepare_with_arena(batch, need);
    if (st == NYTP_ERR_OVERFLOW) {
        /* emergency: flush + direct */
        st = nytp_batch_flush(batch);
        if (st != NYTP_OK) {
            return st;
        }
        if (!batch->child || !batch->child->ops ||
            !batch->child->ops->emit_attribute) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = batch->child->ops->emit_attribute(batch->child, key, value);
        if (st != NYTP_OK) {
            return st;
        }
        batch->child->last_seq = seq;
        batch->child->next_seq = seq + 1;
        batch->child->has_last_seq = 1;
        if (batch->child->ops->on_logical_committed) {
            batch->child->ops->on_logical_committed(batch->child, seq,
                                                    NYTP_EVT_ATTRIBUTE);
        }
        batch->metrics.emergency_direct++;
        batch->metrics.appends++;
        return NYTP_OK;
    }
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_ATTRIBUTE;
    ev.seq = seq;
    st = arena_copy(batch, key.ptr, key.len, key.is_utf8, &ev.u.attr.key);
    if (st != NYTP_OK) {
        return st;
    }
    st = arena_copy(batch, value.ptr, value.len, value.is_utf8, &ev.u.attr.value);
    if (st != NYTP_OK) {
        return st;
    }
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_option(nytp_batch *batch, nytp_seq seq,
                                     nytp_string_view key,
                                     nytp_string_view value)
{
    nytp_status st;
    nytp_event ev;
    size_t need = key.len + value.len;
    st = prepare_with_arena(batch, need);
    if (st == NYTP_ERR_OVERFLOW) {
        st = nytp_batch_flush(batch);
        if (st != NYTP_OK) {
            return st;
        }
        if (!batch->child || !batch->child->ops || !batch->child->ops->emit_option) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = batch->child->ops->emit_option(batch->child, key, value);
        if (st != NYTP_OK) {
            return st;
        }
        batch->child->last_seq = seq;
        batch->child->next_seq = seq + 1;
        batch->child->has_last_seq = 1;
        if (batch->child->ops->on_logical_committed) {
            batch->child->ops->on_logical_committed(batch->child, seq,
                                                    NYTP_EVT_OPTION);
        }
        batch->metrics.emergency_direct++;
        batch->metrics.appends++;
        return NYTP_OK;
    }
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_OPTION;
    ev.seq = seq;
    st = arena_copy(batch, key.ptr, key.len, key.is_utf8, &ev.u.attr.key);
    if (st != NYTP_OK) {
        return st;
    }
    st = arena_copy(batch, value.ptr, value.len, value.is_utf8, &ev.u.attr.value);
    if (st != NYTP_OK) {
        return st;
    }
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_comment(nytp_batch *batch, nytp_seq seq,
                                      nytp_string_view text)
{
    nytp_status st;
    nytp_event ev;
    st = prepare_with_arena(batch, text.len);
    if (st == NYTP_ERR_OVERFLOW) {
        st = nytp_batch_flush(batch);
        if (st != NYTP_OK) {
            return st;
        }
        if (!batch->child || !batch->child->ops ||
            !batch->child->ops->emit_comment) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = batch->child->ops->emit_comment(batch->child, text);
        if (st != NYTP_OK) {
            return st;
        }
        batch->child->last_seq = seq;
        batch->child->next_seq = seq + 1;
        batch->child->has_last_seq = 1;
        if (batch->child->ops->on_logical_committed) {
            batch->child->ops->on_logical_committed(batch->child, seq,
                                                    NYTP_EVT_COMMENT);
        }
        batch->metrics.emergency_direct++;
        batch->metrics.appends++;
        return NYTP_OK;
    }
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_COMMENT;
    ev.seq = seq;
    st = arena_copy(batch, text.ptr, text.len, text.is_utf8, &ev.u.comment.text);
    if (st != NYTP_OK) {
        return st;
    }
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_new_fid(nytp_batch *batch, nytp_seq seq,
                                      nytp_fid fid, nytp_fid eval_fid,
                                      nytp_line eval_line, uint32_t flags,
                                      uint32_t size, uint32_t mtime,
                                      nytp_string_view name)
{
    nytp_status st;
    nytp_event ev;
    st = prepare_with_arena(batch, name.len);
    if (st == NYTP_ERR_OVERFLOW) {
        st = nytp_batch_flush(batch);
        if (st != NYTP_OK) {
            return st;
        }
        if (!batch->child || !batch->child->ops || !batch->child->ops->emit_new_fid) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = batch->child->ops->emit_new_fid(batch->child, fid, eval_fid,
                                             eval_line, flags, size, mtime, name);
        if (st != NYTP_OK) {
            return st;
        }
        batch->child->last_seq = seq;
        batch->child->next_seq = seq + 1;
        batch->child->has_last_seq = 1;
        if (batch->child->ops->on_logical_committed) {
            batch->child->ops->on_logical_committed(batch->child, seq,
                                                    NYTP_EVT_NEW_FID);
        }
        batch->metrics.emergency_direct++;
        batch->metrics.appends++;
        return NYTP_OK;
    }
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_NEW_FID;
    ev.seq = seq;
    ev.u.new_fid.fid = fid;
    ev.u.new_fid.eval_fid = eval_fid;
    ev.u.new_fid.eval_line = eval_line;
    ev.u.new_fid.flags = flags;
    ev.u.new_fid.size = size;
    ev.u.new_fid.mtime = mtime;
    st = arena_copy(batch, name.ptr, name.len, name.is_utf8, &ev.u.new_fid.name);
    if (st != NYTP_OK) {
        return st;
    }
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_src_line(nytp_batch *batch, nytp_seq seq,
                                       nytp_fid fid, nytp_line line,
                                       nytp_string_view text)
{
    nytp_status st;
    nytp_event ev;
    st = prepare_with_arena(batch, text.len);
    if (st == NYTP_ERR_OVERFLOW) {
        return emergency_emit_src_line(batch, seq, fid, line, text);
    }
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_SRC_LINE;
    ev.seq = seq;
    ev.u.src_line.fid = fid;
    ev.u.src_line.line = line;
    st = arena_copy(batch, text.ptr, text.len, text.is_utf8, &ev.u.src_line.text);
    if (st != NYTP_OK) {
        return st;
    }
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_sub_info(nytp_batch *batch, nytp_seq seq,
                                       nytp_fid fid, nytp_line first_line,
                                       nytp_line last_line,
                                       nytp_string_view name)
{
    nytp_status st;
    nytp_event ev;
    st = prepare_with_arena(batch, name.len);
    if (st == NYTP_ERR_OVERFLOW) {
        st = nytp_batch_flush(batch);
        if (st != NYTP_OK) {
            return st;
        }
        if (!batch->child || !batch->child->ops ||
            !batch->child->ops->emit_sub_info) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = batch->child->ops->emit_sub_info(batch->child, fid, first_line,
                                              last_line, name);
        if (st != NYTP_OK) {
            return st;
        }
        batch->child->last_seq = seq;
        batch->child->next_seq = seq + 1;
        batch->child->has_last_seq = 1;
        if (batch->child->ops->on_logical_committed) {
            batch->child->ops->on_logical_committed(batch->child, seq,
                                                    NYTP_EVT_SUB_INFO);
        }
        batch->metrics.emergency_direct++;
        batch->metrics.appends++;
        return NYTP_OK;
    }
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_SUB_INFO;
    ev.seq = seq;
    ev.u.sub_info.fid = fid;
    ev.u.sub_info.first_line = first_line;
    ev.u.sub_info.last_line = last_line;
    st = arena_copy(batch, name.ptr, name.len, name.is_utf8, &ev.u.sub_info.name);
    if (st != NYTP_OK) {
        return st;
    }
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_sub_callers(nytp_batch *batch, nytp_seq seq,
                                          nytp_fid fid, nytp_line line,
                                          uint32_t count, double incl,
                                          double excl, double reci,
                                          uint32_t rec_depth,
                                          nytp_string_view called,
                                          nytp_string_view caller)
{
    nytp_status st;
    nytp_event ev;
    size_t need = called.len + caller.len;
    st = prepare_with_arena(batch, need);
    if (st == NYTP_ERR_OVERFLOW) {
        st = nytp_batch_flush(batch);
        if (st != NYTP_OK) {
            return st;
        }
        if (!batch->child || !batch->child->ops ||
            !batch->child->ops->emit_sub_callers) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = batch->child->ops->emit_sub_callers(batch->child, fid, line, count,
                                                 incl, excl, reci, rec_depth,
                                                 called, caller);
        if (st != NYTP_OK) {
            return st;
        }
        batch->child->last_seq = seq;
        batch->child->next_seq = seq + 1;
        batch->child->has_last_seq = 1;
        if (batch->child->ops->on_logical_committed) {
            batch->child->ops->on_logical_committed(batch->child, seq,
                                                    NYTP_EVT_SUB_CALLERS);
        }
        batch->metrics.emergency_direct++;
        batch->metrics.appends++;
        return NYTP_OK;
    }
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_SUB_CALLERS;
    ev.seq = seq;
    ev.u.sub_callers.fid = fid;
    ev.u.sub_callers.line = line;
    ev.u.sub_callers.count = count;
    ev.u.sub_callers.incl = incl;
    ev.u.sub_callers.excl = excl;
    ev.u.sub_callers.reci = reci;
    ev.u.sub_callers.rec_depth = rec_depth;
    st = arena_copy(batch, called.ptr, called.len, called.is_utf8,
                    &ev.u.sub_callers.called);
    if (st != NYTP_OK) {
        return st;
    }
    st = arena_copy(batch, caller.ptr, caller.len, caller.is_utf8,
                    &ev.u.sub_callers.caller);
    if (st != NYTP_OK) {
        return st;
    }
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_pid_start(nytp_batch *batch, nytp_seq seq,
                                        nytp_pid pid, nytp_pid ppid,
                                        double start_time)
{
    nytp_status st;
    nytp_event ev;
    st = ensure_slot(batch);
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_PID_START;
    ev.seq = seq;
    ev.u.pid_start.pid = pid;
    ev.u.pid_start.ppid = ppid;
    ev.u.pid_start.start_time = start_time;
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_pid_end(nytp_batch *batch, nytp_seq seq,
                                      nytp_pid pid, double end_time)
{
    nytp_status st;
    nytp_event ev;
    st = ensure_slot(batch);
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_PID_END;
    ev.seq = seq;
    ev.u.pid_end.pid = pid;
    ev.u.pid_end.end_time = end_time;
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_sub_return(nytp_batch *batch, nytp_seq seq,
                                         nytp_depth depth, double incl_time,
                                         double excl_time,
                                         nytp_string_view subname)
{
    nytp_status st;
    nytp_event ev;
    st = prepare_with_arena(batch, subname.len);
    if (st == NYTP_ERR_OVERFLOW) {
        st = nytp_batch_flush(batch);
        if (st != NYTP_OK) {
            return st;
        }
        if (!batch->child || !batch->child->ops ||
            !batch->child->ops->emit_sub_return) {
            return NYTP_ERR_UNSUPPORTED;
        }
        st = batch->child->ops->emit_sub_return(batch->child, depth, incl_time,
                                                excl_time, subname);
        if (st != NYTP_OK) {
            return st;
        }
        batch->child->last_seq = seq;
        batch->child->next_seq = seq + 1;
        batch->child->has_last_seq = 1;
        if (batch->child->ops->on_logical_committed) {
            batch->child->ops->on_logical_committed(batch->child, seq,
                                                    NYTP_EVT_SUB_RETURN);
        }
        batch->metrics.emergency_direct++;
        batch->metrics.appends++;
        return NYTP_OK;
    }
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_SUB_RETURN;
    ev.seq = seq;
    ev.u.sub_return.depth = depth;
    ev.u.sub_return.incl_time = incl_time;
    ev.u.sub_return.excl_time = excl_time;
    st = arena_copy(batch, subname.ptr, subname.len, subname.is_utf8,
                    &ev.u.sub_return.subname);
    if (st != NYTP_OK) {
        return st;
    }
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

nytp_status nytp_batch_append_start_deflate(nytp_batch *batch)
{
    nytp_status st;
    nytp_event ev;
    st = ensure_slot(batch);
    if (st != NYTP_OK) {
        return st;
    }
    memset(&ev, 0, sizeof(ev));
    ev.kind = NYTP_EVT_START_DEFLATE;
    ev.seq = 0; /* control: no COL-003 seq */
    commit_event(batch, &ev, 0);
    return maybe_high_water(batch);
}

/* ---- Batch sink facade ---- */

typedef struct batch_sink_impl {
    nytp_batch *batch;
} batch_sink_impl;

static const char *batch_sink_name(const nytp_sink *sink)
{
    (void)sink;
    return "batch";
}

static nytp_status batch_sink_activate(nytp_sink *sink)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    if (!bi || !bi->batch || !bi->batch->child) {
        return NYTP_ERR_STATE;
    }
    /* Activate child if still OPEN/STOPPED. */
    {
        nytp_sink_state cs = nytp_sink_get_state(bi->batch->child);
        if (cs == NYTP_SINK_OPEN || cs == NYTP_SINK_STOPPED) {
            return nytp_sink_activate(bi->batch->child);
        }
        if (cs == NYTP_SINK_ACTIVE) {
            return NYTP_OK;
        }
        return NYTP_ERR_STATE;
    }
}

static nytp_status batch_sink_flush(nytp_sink *sink)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_status st;
    nytp_sink *child;
    if (!bi || !bi->batch) {
        return NYTP_ERR_NULL;
    }
    st = nytp_batch_flush(bi->batch);
    if (st != NYTP_OK) {
        return st;
    }
    child = bi->batch->child;
    if (!child) {
        return NYTP_ERR_STATE;
    }
    /* Child already terminal: drain succeeded; do not report STATE as half-success. */
    if (nytp_sink_get_state(child) == NYTP_SINK_FAILED ||
        nytp_sink_get_state(child) == NYTP_SINK_CLOSED) {
        return NYTP_OK;
    }
    return nytp_sink_flush(child);
}

static nytp_status batch_sink_close(nytp_sink *sink)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_status st;
    nytp_sink *child;
    if (!bi || !bi->batch) {
        return NYTP_ERR_NULL;
    }
    st = nytp_batch_flush(bi->batch);
    if (st != NYTP_OK) {
        return st;
    }
    child = bi->batch->child;
    if (!child) {
        return NYTP_ERR_STATE;
    }
    if (nytp_sink_get_state(child) == NYTP_SINK_CLOSED) {
        return NYTP_OK;
    }
    if (nytp_sink_get_state(child) == NYTP_SINK_FAILED) {
        /* Best-effort close of a failed child. */
        return nytp_sink_close(child);
    }
    return nytp_sink_close(child);
}

/* COL-002: forward lifecycle to child so finalization gates stay aligned. */
static nytp_status batch_notify_stop(nytp_sink *sink)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_sink *c;
    if (!bi || !bi->batch || !bi->batch->child) {
        return NYTP_ERR_STATE;
    }
    c = bi->batch->child;
    if (nytp_sink_get_state(c) == NYTP_SINK_ACTIVE) {
        return nytp_sink_stop(c);
    }
    return NYTP_OK;
}

static nytp_status batch_notify_begin_finalize(nytp_sink *sink)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_sink *c;
    nytp_sink_state cs;
    if (!bi || !bi->batch || !bi->batch->child) {
        return NYTP_ERR_STATE;
    }
    c = bi->batch->child;
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

static nytp_status batch_notify_begin_fork(nytp_sink *sink)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_sink *c;
    nytp_status st;
    if (!bi || !bi->batch || !bi->batch->child) {
        return NYTP_ERR_STATE;
    }
    /*
     * COL-015: drain pending before FORK_SPLIT so parent and child do not
     * both inherit the same unacked events (duplicate drain risk).
     * Public nytp_fork_prepare also flushes; this covers begin_fork alone.
     */
    if (bi->batch->count > 0) {
        st = nytp_batch_flush(bi->batch);
        if (st != NYTP_OK) {
            return st;
        }
        bi->batch->metrics.fork_preflush++;
    }
    c = bi->batch->child;
    if (nytp_sink_get_state(c) == NYTP_SINK_ACTIVE) {
        return nytp_sink_begin_fork(c);
    }
    return NYTP_ERR_STATE;
}

static nytp_status batch_notify_end_fork_parent(nytp_sink *sink)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_sink *c;
    if (!bi || !bi->batch || !bi->batch->child) {
        return NYTP_ERR_STATE;
    }
    c = bi->batch->child;
    if (nytp_sink_get_state(c) == NYTP_SINK_FORK_SPLIT) {
        return nytp_sink_end_fork_parent(c);
    }
    return NYTP_OK;
}

static nytp_status batch_notify_end_fork_child(nytp_sink *sink)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_sink *c;
    size_t residual;
    if (!bi || !bi->batch || !bi->batch->child) {
        return NYTP_ERR_STATE;
    }
    /* COL-015: never drain inherited residual into the child stream. */
    residual = bi->batch->count;
    if (residual > 0 || bi->batch->arena_used > 0) {
        bi->batch->metrics.fork_child_discard += (uint64_t)residual;
        nytp_batch_discard_pending(bi->batch);
    }
    c = bi->batch->child;
    if (nytp_sink_get_state(c) == NYTP_SINK_FORK_SPLIT) {
        return nytp_sink_end_fork_child(c);
    }
    /* Align child to OPEN + seq reset if already open. */
    return NYTP_OK;
}

static void batch_sink_destroy(nytp_sink *sink)
{
    batch_sink_impl *bi;
    if (!sink) {
        return;
    }
    bi = (batch_sink_impl *)sink->impl;
    if (bi) {
        if (bi->batch) {
            nytp_batch_destroy(bi->batch);
        }
        free(bi);
    }
    free(sink);
}

/*
 * Peek next_seq for stamping; emit_commit advances on OK.
 * If append buffers the event but HW/full flush fails, pre-advance seq so
 * logical_count matches buffered work (Issue 3); emit_commit then sticky-fails
 * without double-advancing.
 */
static nytp_seq peek_seq(nytp_sink *sink)
{
    return sink->next_seq;
}

static nytp_status bs_finish(nytp_sink *sink, nytp_batch *batch, nytp_seq seq,
                             int logical, nytp_status st)
{
    if (st == NYTP_OK) {
        return NYTP_OK;
    }
    if (logical && nytp_batch_last_append_buffered(batch) &&
        sink->next_seq == seq) {
        sink->last_seq = seq;
        sink->next_seq = seq + 1;
        sink->has_last_seq = 1;
    }
    return st;
}

static nytp_status bs_emit_attribute(nytp_sink *sink, nytp_string_view key,
                                     nytp_string_view value)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_attribute(bi->batch, seq, key, value));
}

static nytp_status bs_emit_option(nytp_sink *sink, nytp_string_view key,
                                  nytp_string_view value)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_option(bi->batch, seq, key, value));
}

static nytp_status bs_emit_comment(nytp_sink *sink, nytp_string_view text)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_comment(bi->batch, seq, text));
}

static nytp_status bs_emit_time_line(nytp_sink *sink, nytp_ticks ticks,
                                     nytp_fid fid, nytp_line line)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_time_line(bi->batch, seq, ticks, fid,
                                                 line));
}

static nytp_status bs_emit_time_block(nytp_sink *sink, nytp_ticks ticks,
                                      nytp_fid fid, nytp_line line,
                                      nytp_line block_line, nytp_line sub_line)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_time_block(bi->batch, seq, ticks, fid,
                                                  line, block_line, sub_line));
}

static nytp_status bs_emit_discount(nytp_sink *sink)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_discount(bi->batch, seq));
}

static nytp_status bs_emit_new_fid(nytp_sink *sink, nytp_fid fid,
                                   nytp_fid eval_fid, nytp_line eval_line,
                                   uint32_t flags, uint32_t size, uint32_t mtime,
                                   nytp_string_view name)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_new_fid(bi->batch, seq, fid, eval_fid,
                                               eval_line, flags, size, mtime,
                                               name));
}

static nytp_status bs_emit_src_line(nytp_sink *sink, nytp_fid fid, nytp_line line,
                                    nytp_string_view text)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_src_line(bi->batch, seq, fid, line, text));
}

static nytp_status bs_emit_sub_info(nytp_sink *sink, nytp_fid fid,
                                    nytp_line first_line, nytp_line last_line,
                                    nytp_string_view name)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_sub_info(bi->batch, seq, fid, first_line,
                                                last_line, name));
}

static nytp_status bs_emit_sub_callers(nytp_sink *sink, nytp_fid fid,
                                       nytp_line line, uint32_t count,
                                       double incl, double excl, double reci,
                                       uint32_t rec_depth,
                                       nytp_string_view called,
                                       nytp_string_view caller)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_sub_callers(bi->batch, seq, fid, line,
                                                   count, incl, excl, reci,
                                                   rec_depth, called, caller));
}

static nytp_status bs_emit_pid_start(nytp_sink *sink, nytp_pid pid,
                                     nytp_pid ppid, double start_time)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_pid_start(bi->batch, seq, pid, ppid,
                                                 start_time));
}

static nytp_status bs_emit_pid_end(nytp_sink *sink, nytp_pid pid,
                                   double end_time)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_pid_end(bi->batch, seq, pid, end_time));
}

static nytp_status bs_emit_sub_entry(nytp_sink *sink, nytp_fid caller_fid,
                                     nytp_line caller_line)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_sub_entry(bi->batch, seq, caller_fid,
                                                 caller_line));
}

static nytp_status bs_emit_sub_return(nytp_sink *sink, nytp_depth depth,
                                      double incl_time, double excl_time,
                                      nytp_string_view subname)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    nytp_seq seq = peek_seq(sink);
    return bs_finish(sink, bi->batch, seq, 1,
                     nytp_batch_append_sub_return(bi->batch, seq, depth,
                                                  incl_time, excl_time,
                                                  subname));
}

static nytp_status bs_emit_start_deflate(nytp_sink *sink)
{
    batch_sink_impl *bi = (batch_sink_impl *)sink->impl;
    /* Control event: no logical seq. */
    return nytp_batch_append_start_deflate(bi->batch);
}

static const nytp_sink_ops BATCH_OPS = {
    .name = batch_sink_name,
    .activate = batch_sink_activate,
    .flush = batch_sink_flush,
    .close = batch_sink_close,
    .destroy = batch_sink_destroy,
    .emit_attribute = bs_emit_attribute,
    .emit_option = bs_emit_option,
    .emit_comment = bs_emit_comment,
    .emit_time_line = bs_emit_time_line,
    .emit_time_block = bs_emit_time_block,
    .emit_discount = bs_emit_discount,
    .emit_new_fid = bs_emit_new_fid,
    .emit_src_line = bs_emit_src_line,
    .emit_sub_info = bs_emit_sub_info,
    .emit_sub_callers = bs_emit_sub_callers,
    .emit_pid_start = bs_emit_pid_start,
    .emit_pid_end = bs_emit_pid_end,
    .emit_sub_entry = bs_emit_sub_entry,
    .emit_sub_return = bs_emit_sub_return,
    .emit_start_deflate = bs_emit_start_deflate,
    .on_logical_committed = NULL,
    .notify_stop = batch_notify_stop,
    .notify_begin_finalize = batch_notify_begin_finalize,
    .notify_begin_fork = batch_notify_begin_fork,
    .notify_end_fork_parent = batch_notify_end_fork_parent,
    .notify_end_fork_child = batch_notify_end_fork_child,
};

nytp_sink *nytp_batch_sink_create(nytp_sink *child, size_t capacity,
                                  size_t arena_cap, size_t high_water,
                                  int owns_child)
{
    nytp_sink *s;
    batch_sink_impl *bi;
    nytp_batch *b;
    if (!child) {
        return NULL;
    }
    b = nytp_batch_create(capacity, arena_cap, high_water);
    if (!b) {
        return NULL;
    }
    nytp_batch_set_child(b, child);
    nytp_batch_set_owns_child(b, owns_child);

    bi = (batch_sink_impl *)calloc(1, sizeof(*bi));
    s = (nytp_sink *)calloc(1, sizeof(*s));
    if (!bi || !s) {
        free(bi);
        free(s);
        /* Don't destroy child if we haven't taken ownership yet — but we set
         * owns_child on batch; if owns_child, destroy would free child. Only
         * destroy batch without child ownership if create fails mid-way. */
        b->owns_child = 0;
        nytp_batch_destroy(b);
        return NULL;
    }
    bi->batch = b;
    s->ops = &BATCH_OPS;
    s->state = NYTP_SINK_OPEN;
    s->impl = bi;
    s->next_seq = 0;
    s->last_seq = 0;
    s->has_last_seq = 0;
    s->fail_reason = NYTP_OK;
    return s;
}

nytp_batch *nytp_batch_sink_batch(nytp_sink *sink)
{
    batch_sink_impl *bi;
    if (!sink || sink->ops != &BATCH_OPS || !sink->impl) {
        return NULL;
    }
    bi = (batch_sink_impl *)sink->impl;
    return bi->batch;
}

/* ---- COL-004 fast emit (prebound batch sink) ---- */

nytp_status nytp_fast_emit_time_line(nytp_sink *sink, nytp_ticks ticks,
                                     nytp_fid fid, nytp_line line)
{
    nytp_batch *b;
    nytp_status st;
    nytp_seq seq;
    if (!sink) {
        return NYTP_ERR_NULL;
    }
    b = nytp_batch_sink_batch(sink);
    if (!b) {
        /* Fallback: generic public path. */
        return nytp_emit_time_line(sink, ticks, fid, line);
    }
    if (!nytp_sink_can_emit(sink, NYTP_EVT_TIME_LINE)) {
        return NYTP_ERR_STATE;
    }
    seq = sink->next_seq;
    st = nytp_batch_append_time_line(b, seq, ticks, fid, line);
    if (st != NYTP_OK) {
        /* Buffered-but-flush-failed still consumes seq (Issue 3). */
        if (nytp_batch_last_append_buffered(b) && sink->next_seq == seq) {
            sink->last_seq = seq;
            sink->next_seq = seq + 1;
            sink->has_last_seq = 1;
        }
        if (st == NYTP_ERR_IO || st == NYTP_ERR_FAILED ||
            st == NYTP_ERR_OVERFLOW) {
            sink->state = NYTP_SINK_FAILED;
            sink->fail_reason = st;
        }
        return st;
    }
    /* Manual emit_commit for logical event. */
    sink->last_seq = seq;
    sink->next_seq = seq + 1;
    sink->has_last_seq = 1;
    return NYTP_OK;
}

nytp_status nytp_fast_emit_time_block(nytp_sink *sink, nytp_ticks ticks,
                                      nytp_fid fid, nytp_line line,
                                      nytp_line block_line, nytp_line sub_line)
{
    nytp_batch *b;
    nytp_status st;
    nytp_seq seq;
    if (!sink) {
        return NYTP_ERR_NULL;
    }
    b = nytp_batch_sink_batch(sink);
    if (!b) {
        return nytp_emit_time_block(sink, ticks, fid, line, block_line,
                                    sub_line);
    }
    if (!nytp_sink_can_emit(sink, NYTP_EVT_TIME_BLOCK)) {
        return NYTP_ERR_STATE;
    }
    seq = sink->next_seq;
    st = nytp_batch_append_time_block(b, seq, ticks, fid, line, block_line,
                                      sub_line);
    if (st != NYTP_OK) {
        if (nytp_batch_last_append_buffered(b) && sink->next_seq == seq) {
            sink->last_seq = seq;
            sink->next_seq = seq + 1;
            sink->has_last_seq = 1;
        }
        if (st == NYTP_ERR_IO || st == NYTP_ERR_FAILED ||
            st == NYTP_ERR_OVERFLOW) {
            sink->state = NYTP_SINK_FAILED;
            sink->fail_reason = st;
        }
        return st;
    }
    sink->last_seq = seq;
    sink->next_seq = seq + 1;
    sink->has_last_seq = 1;
    return NYTP_OK;
}

/* ---- light microbench (engineering only) ---- */

static uint64_t mono_ns(void)
{
#if defined(CLOCK_MONOTONIC)
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) {
        return 0;
    }
    return (uint64_t)ts.tv_sec * 1000000000ull + (uint64_t)ts.tv_nsec;
#else
    return 0;
#endif
}

nytp_status nytp_fast_bench_time_line(size_t capacity, uint64_t iterations,
                                      nytp_fast_bench_result *out)
{
    nytp_sink *child;
    nytp_sink *batch;
    nytp_batch *b;
    uint64_t i;
    uint64_t t0, t1;
    const nytp_batch_metrics *m;
    if (!out || iterations == 0) {
        return NYTP_ERR_NULL;
    }
    memset(out, 0, sizeof(*out));
    out->event_sizeof = sizeof(nytp_event);
    out->batch_capacity = capacity;
    out->iterations = iterations;

    child = nytp_counting_sink_create();
    if (!child) {
        return NYTP_ERR_NULL;
    }
    /* Need counting header — include via nytp_batch.h? Pull it. */
    batch = nytp_batch_sink_create(child, capacity, 256, capacity, 1);
    if (!batch) {
        nytp_sink_destroy(child);
        return NYTP_ERR_NULL;
    }
    if (nytp_sink_activate(batch) != NYTP_OK) {
        nytp_sink_destroy(batch);
        return NYTP_ERR_STATE;
    }

    t0 = mono_ns();
    for (i = 0; i < iterations; i++) {
        nytp_status st =
            nytp_fast_emit_time_line(batch, (nytp_ticks)(i + 1), 1,
                                     (nytp_line)((i % 100) + 1));
        if (st != NYTP_OK) {
            nytp_sink_destroy(batch);
            return st;
        }
    }
    (void)nytp_sink_flush(batch);
    t1 = mono_ns();
    out->elapsed_ns = (t1 > t0) ? (t1 - t0) : 0;

    b = nytp_batch_sink_batch(batch);
    m = b ? nytp_batch_get_metrics(b) : NULL;
    out->stmt_fast_appends = m ? m->stmt_fast_appends : 0;

    nytp_sink_destroy(batch);
    return NYTP_OK;
}

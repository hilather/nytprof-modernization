/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * TEST-003 / PR-B03 fake-clock + BASE-003 statement driver + M4 mini sample.
 */
#include "nytp_clock.h"

#include <stddef.h>
#include <string.h>

void nytp_fake_clock_init(nytp_fake_clock *fc, const nytp_ticks *script,
                          size_t len)
{
    if (!fc) {
        return;
    }
    fc->script = (len == 0) ? NULL : script;
    fc->len = len;
    fc->pos = 0;
    fc->last_read = 0;
    fc->has_read = 0;
    fc->exhausted = 0;
}

void nytp_fake_clock_reset(nytp_fake_clock *fc)
{
    if (!fc) {
        return;
    }
    fc->pos = 0;
    fc->last_read = 0;
    fc->has_read = 0;
    fc->exhausted = 0;
}

nytp_status nytp_fake_clock_read(nytp_fake_clock *fc, nytp_ticks *out)
{
    nytp_ticks v;
    if (!fc || !out) {
        return NYTP_ERR_NULL;
    }
    if (fc->pos >= fc->len || !fc->script) {
        fc->exhausted = 1;
        return NYTP_ERR_EXHAUSTED;
    }
    v = fc->script[fc->pos++];
    fc->last_read = v;
    fc->has_read = 1;
    *out = v;
    return NYTP_OK;
}

size_t nytp_fake_clock_remaining(const nytp_fake_clock *fc)
{
    if (!fc || fc->pos >= fc->len) {
        return 0;
    }
    return fc->len - fc->pos;
}

void nytp_stmt_driver_init(nytp_stmt_driver *d, nytp_fake_clock *clock,
                           nytp_fid fid)
{
    if (!d) {
        return;
    }
    d->clock = clock;
    d->last = 0;
    d->has_last = 0;
    d->fid = fid;
    d->prev_line = 0;
    d->has_prev = 0;
}

nytp_status nytp_stmt_driver_on_line(nytp_stmt_driver *d, nytp_sink *sink,
                                     nytp_line line,
                                     nytp_ticks *attributed_ticks)
{
    nytp_ticks now;
    nytp_status st;
    if (!d || !d->clock || !sink) {
        return NYTP_ERR_NULL;
    }
    st = nytp_fake_clock_read(d->clock, &now);
    if (st != NYTP_OK) {
        return st;
    }
    if (!d->has_prev) {
        /* First breakable entry: seed last + prev_line; no prior attribution. */
        d->last = now;
        d->has_last = 1;
        d->prev_line = line;
        d->has_prev = 1;
        if (attributed_ticks) {
            *attributed_ticks = 0;
        }
        return NYTP_OK;
    }
    {
        nytp_ticks delta = now - d->last;
        /* Attribute (now - last) to the previous statement site. */
        st = nytp_emit_time_line(sink, delta, d->fid, d->prev_line);
        if (st != NYTP_OK) {
            return st;
        }
        d->last = now;
        d->prev_line = line;
        if (attributed_ticks) {
            *attributed_ticks = delta;
        }
        return NYTP_OK;
    }
}

nytp_status nytp_stmt_driver_discount(nytp_sink *sink)
{
    return nytp_emit_discount(sink);
}

/*
 * Mini M4 sample — synthetic, not fixture/v5/default-calls1.
 *
 * Clock script (absolute readings):
 *   1000, 1042, 1100, 1150
 * Statement model after seed at line 1:
 *   enter L1 @1000 (seed)
 *   enter L2 @1042 -> TIME_LINE ticks=42 fid=1 line=1
 *   discount
 *   enter L3 @1100 -> TIME_LINE ticks=58 fid=1 line=2
 *   enter L4 @1150 -> TIME_LINE ticks=50 fid=1 line=3
 *
 * Full logical order includes header attrs, pid, sub_return, finalize.
 */

static const nytp_ticks m4_clock_script[] = {1000, 1042, 1100, 1150};

/* Expected *logical* events in order (no START_DEFLATE). */
static const nytp_m4_step m4_expected[] = {
    {NYTP_EVT_ATTRIBUTE, 0, 0, 0},   /* ticks_per_sec */
    {NYTP_EVT_OPTION, 0, 0, 0},      /* calls=1 */
    {NYTP_EVT_PID_START, 0, 0, 0},
    {NYTP_EVT_NEW_FID, 0, 1, 0},
    {NYTP_EVT_TIME_LINE, 42, 1, 1},
    {NYTP_EVT_DISCOUNT, 0, 0, 0},
    {NYTP_EVT_TIME_LINE, 58, 1, 2},
    {NYTP_EVT_TIME_LINE, 50, 1, 3},
    {NYTP_EVT_SUB_RETURN, 0, 0, 0},
    {NYTP_EVT_SRC_LINE, 0, 1, 1},
    {NYTP_EVT_SUB_INFO, 0, 1, 0},
    {NYTP_EVT_PID_END, 0, 0, 0},
};

const nytp_m4_step *nytp_m4_mini_sample_expected(size_t *out_n)
{
    if (out_n) {
        *out_n = sizeof(m4_expected) / sizeof(m4_expected[0]);
    }
    return m4_expected;
}

nytp_status nytp_m4_mini_sample_run(nytp_sink *sink, nytp_m4_harness_result *out)
{
    nytp_fake_clock fc;
    nytp_stmt_driver drv;
    nytp_status st;
    nytp_ticks attributed;
    nytp_m4_harness_result local;
    size_t n_exp =
        sizeof(m4_expected) / sizeof(m4_expected[0]);
    nytp_seq seqs[32];
    size_t n_seq = 0;
    size_t i;

    memset(&local, 0, sizeof(local));
    local.first_kind_mismatch = (size_t)-1;
    local.gapless_ok = 1;
    local.kinds_match = 1;
    local.ticks_match = 1;

    if (!sink) {
        return NYTP_ERR_NULL;
    }

    /* Header phase in OPEN. */
    st = nytp_emit_attribute(sink, nytp_sv_cstr("ticks_per_sec"),
                             nytp_sv_cstr("10000000"));
    if (st != NYTP_OK) {
        return st;
    }
    st = nytp_emit_option(sink, nytp_sv_cstr("calls"), nytp_sv_cstr("1"));
    if (st != NYTP_OK) {
        return st;
    }

    st = nytp_sink_activate(sink);
    if (st != NYTP_OK) {
        return st;
    }

    /* Control — no seq. */
    st = nytp_emit_start_deflate(sink);
    if (st != NYTP_OK) {
        return st;
    }

    st = nytp_emit_pid_start(sink, 42, 1, 0.0);
    if (st != NYTP_OK) {
        return st;
    }
    st = nytp_emit_new_fid(sink, 1, 0, 0, 0, 0, 0, nytp_sv_cstr("m4_mini.pl"));
    if (st != NYTP_OK) {
        return st;
    }

    nytp_fake_clock_init(&fc, m4_clock_script,
                         sizeof(m4_clock_script) / sizeof(m4_clock_script[0]));
    nytp_stmt_driver_init(&drv, &fc, 1);

    /* Enter L1 @1000 — seed only. */
    st = nytp_stmt_driver_on_line(&drv, sink, 1, &attributed);
    if (st != NYTP_OK) {
        return st;
    }

    /* Enter L2 @1042 -> attribute to L1: delta 42. */
    st = nytp_stmt_driver_on_line(&drv, sink, 2, &attributed);
    if (st != NYTP_OK) {
        return st;
    }
    if (attributed != 42) {
        local.ticks_match = 0;
        local.first_kind_mismatch = 4;
    }

    st = nytp_stmt_driver_discount(sink);
    if (st != NYTP_OK) {
        return st;
    }

    /* Enter L3 @1100 -> attribute to L2: delta 58. */
    st = nytp_stmt_driver_on_line(&drv, sink, 3, &attributed);
    if (st != NYTP_OK) {
        return st;
    }
    if (attributed != 58) {
        local.ticks_match = 0;
        if (local.first_kind_mismatch == (size_t)-1) {
            local.first_kind_mismatch = 6;
        }
    }

    /* Enter L4 @1150 -> attribute to L3: delta 50. */
    st = nytp_stmt_driver_on_line(&drv, sink, 4, &attributed);
    if (st != NYTP_OK) {
        return st;
    }
    if (attributed != 50) {
        local.ticks_match = 0;
        if (local.first_kind_mismatch == (size_t)-1) {
            local.first_kind_mismatch = 7;
        }
    }

    /* One more clock read would exhaust — harness does not require it. */
    if (nytp_fake_clock_remaining(&fc) != 0) {
        /* All four reads consumed by seed + 3 attributions. */
        return NYTP_ERR_STATE;
    }

    st = nytp_emit_sub_return(sink, 1, 0.01, 0.005, nytp_sv_cstr("main::leaf"));
    if (st != NYTP_OK) {
        return st;
    }

    st = nytp_sink_begin_finalize(sink);
    if (st != NYTP_OK) {
        return st;
    }

    /* Hot-path must fail in FINALIZING. */
    if (nytp_emit_time_line(sink, 1, 1, 1) != NYTP_ERR_STATE) {
        return NYTP_ERR_STATE;
    }

    st = nytp_emit_src_line(sink, 1, 1, nytp_sv_cstr("sub leaf { 1 }"));
    if (st != NYTP_OK) {
        return st;
    }
    st = nytp_emit_sub_info(sink, 1, 1, 4, nytp_sv_cstr("main::leaf"));
    if (st != NYTP_OK) {
        return st;
    }
    st = nytp_emit_pid_end(sink, 42, 1.0);
    if (st != NYTP_OK) {
        return st;
    }

    st = nytp_sink_close(sink);
    if (st != NYTP_OK) {
        return st;
    }
    if (nytp_sink_get_state(sink) != NYTP_SINK_CLOSED) {
        return NYTP_ERR_STATE;
    }

    local.logical_events = (size_t)nytp_sink_logical_count(sink);
    if (local.logical_events != n_exp) {
        local.kinds_match = 0;
    }
    if (nytp_sink_last_seq(sink, &local.last_seq) != NYTP_OK) {
        return NYTP_ERR_STATE;
    }
    local.first_seq = 0;

    /* Build expected gapless seq 0..n_exp-1. */
    for (i = 0; i < n_exp && i < sizeof(seqs) / sizeof(seqs[0]); i++) {
        seqs[i] = (nytp_seq)i;
        n_seq++;
    }
    local.gapless_ok =
        nytp_seq_check_gapless(seqs, n_seq, 0, &local.mismatch);
    /* Also verify sink reported next_seq == n_exp and last == n_exp-1. */
    if (nytp_sink_peek_seq(sink) != (nytp_seq)n_exp ||
        local.last_seq != (nytp_seq)(n_exp - 1)) {
        local.gapless_ok = 0;
        local.mismatch.index = n_exp;
        local.mismatch.expected_seq = (nytp_seq)(n_exp - 1);
        local.mismatch.actual_seq = local.last_seq;
    }

    if (out) {
        *out = local;
    }

    if (!local.gapless_ok || !local.kinds_match || !local.ticks_match) {
        return NYTP_ERR_STATE;
    }
    return NYTP_OK;
}

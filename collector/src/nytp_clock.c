/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * TEST-003 / PR-B03 fake-clock + BASE-003 statement driver + M4 mini sample.
 */
#include "nytp_clock.h"

#include "nytp_sink_counting.h"
#include "nytp_sink_v5.h"

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

nytp_status nytp_fake_clock_peek(const nytp_fake_clock *fc, nytp_ticks *out)
{
    if (!fc || !out) {
        return NYTP_ERR_NULL;
    }
    if (fc->pos >= fc->len || !fc->script) {
        return NYTP_ERR_EXHAUSTED;
    }
    *out = fc->script[fc->pos];
    return NYTP_OK;
}

nytp_status nytp_fake_clock_consume(nytp_fake_clock *fc)
{
    nytp_ticks v;
    if (!fc) {
        return NYTP_ERR_NULL;
    }
    if (fc->pos >= fc->len || !fc->script) {
        fc->exhausted = 1;
        return NYTP_ERR_EXHAUSTED;
    }
    v = fc->script[fc->pos++];
    fc->last_read = v;
    fc->has_read = 1;
    return NYTP_OK;
}

nytp_status nytp_fake_clock_read(nytp_fake_clock *fc, nytp_ticks *out)
{
    nytp_status st = nytp_fake_clock_peek(fc, out);
    if (st != NYTP_OK) {
        if (fc) {
            fc->exhausted = 1;
        }
        return st;
    }
    return nytp_fake_clock_consume(fc);
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
    /* Peek first: do not advance clock until emit (or seed) succeeds. */
    st = nytp_fake_clock_peek(d->clock, &now);
    if (st != NYTP_OK) {
        d->clock->exhausted = 1;
        return st;
    }
    if (!d->has_prev) {
        st = nytp_fake_clock_consume(d->clock);
        if (st != NYTP_OK) {
            return st;
        }
        d->last = now;
        d->has_last = 1;
        d->prev_line = line;
        d->has_prev = 1;
        if (attributed_ticks) {
            *attributed_ticks = 0;
        }
        return NYTP_OK;
    }
    /* Fail closed on non-monotonic scripts (signed overflow / adversarial). */
    if (now < d->last) {
        return NYTP_ERR_OVERFLOW;
    }
    {
        nytp_ticks delta = now - d->last;
        st = nytp_emit_time_line(sink, delta, d->fid, d->prev_line);
        if (st != NYTP_OK) {
            /* Clock not consumed: retry will re-peek the same tick. */
            return st;
        }
        st = nytp_fake_clock_consume(d->clock);
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
 */

static const nytp_ticks m4_clock_script[] = {1000, 1042, 1100, 1150};

/* Expected *logical* events in order (no START_DEFLATE). */
static const nytp_m4_step m4_expected[] = {
    {NYTP_EVT_ATTRIBUTE, 0, 0, 0}, /* ticks_per_sec */
    {NYTP_EVT_OPTION, 0, 0, 0},    /* calls=1 */
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

static void finish_out(nytp_m4_harness_result *out, nytp_m4_harness_result *local,
                       nytp_status st)
{
    local->run_status = st;
    if (out) {
        *out = *local;
    }
}

/* Verify observed counting/v5 stats against m4_expected (real checks). */
static void verify_observed_stats(nytp_sink *sink, nytp_m4_harness_result *local,
                                  size_t n_exp)
{
    const nytp_counting_stats *st = NULL;
    nytp_seq seqs[32];
    nytp_event_kind kinds[32];
    size_t n_seq = 32;
    size_t n_kind = 32;
    size_t i;
    nytp_seq_mismatch mm;

    if (nytp_counting_sink_stats(sink)) {
        st = nytp_counting_sink_stats(sink);
    } else if (nytp_v5_sink_stats(sink)) {
        st = nytp_v5_sink_stats(sink);
    }

    local->logical_events = (size_t)nytp_sink_logical_count(sink);
    if (local->logical_events != n_exp) {
        local->kinds_match = 0;
    }
    if (nytp_sink_last_seq(sink, &local->last_seq) != NYTP_OK) {
        local->gapless_ok = 0;
        return;
    }
    local->first_seq = 0;

    if (!st) {
        /* No stats backend: still check wrapper seq counters. */
        if (nytp_sink_peek_seq(sink) != (nytp_seq)n_exp ||
            local->last_seq != (nytp_seq)(n_exp - 1)) {
            local->gapless_ok = 0;
            local->mismatch.index = n_exp;
            local->mismatch.expected_seq = (nytp_seq)(n_exp - 1);
            local->mismatch.actual_seq = local->last_seq;
        }
        return;
    }

    /* Observed seq ring must be gapless. */
    n_seq = sizeof(seqs) / sizeof(seqs[0]);
    if (nytp_counting_sink_stats(sink)) {
        if (nytp_counting_sink_copy_seqs(sink, seqs, &n_seq) != NYTP_OK) {
            local->gapless_ok = 0;
            return;
        }
        n_kind = sizeof(kinds) / sizeof(kinds[0]);
        if (nytp_counting_sink_copy_kinds(sink, kinds, &n_kind) != NYTP_OK) {
            local->kinds_match = 0;
            return;
        }
    } else {
        /* v5 wire: copy from stats rings directly. */
        n_seq = st->seq_ring_len;
        if (n_seq > sizeof(seqs) / sizeof(seqs[0])) {
            n_seq = sizeof(seqs) / sizeof(seqs[0]);
        }
        memcpy(seqs, st->seq_ring, n_seq * sizeof(nytp_seq));
        n_kind = n_seq;
        memcpy(kinds, st->kind_ring, n_kind * sizeof(nytp_event_kind));
    }

    if (n_seq != n_exp || n_kind != n_exp) {
        local->kinds_match = 0;
        local->gapless_ok = 0;
        return;
    }
    local->gapless_ok = nytp_seq_check_gapless(seqs, n_seq, 0, &mm);
    if (!local->gapless_ok) {
        local->mismatch = mm;
    }
    if (nytp_sink_peek_seq(sink) != (nytp_seq)n_exp ||
        local->last_seq != (nytp_seq)(n_exp - 1)) {
        local->gapless_ok = 0;
        local->mismatch.index = n_exp;
        local->mismatch.expected_seq = (nytp_seq)(n_exp - 1);
        local->mismatch.actual_seq = local->last_seq;
    }

    /* Kind order must match m4_expected. */
    for (i = 0; i < n_exp; i++) {
        if (kinds[i] != m4_expected[i].kind) {
            local->kinds_match = 0;
            if (local->first_kind_mismatch == (size_t)-1) {
                local->first_kind_mismatch = i;
            }
            break;
        }
    }

    /* Multiplicity cross-check. */
    {
        uint64_t expect_by[NYTP_EVT_KIND_COUNT];
        memset(expect_by, 0, sizeof(expect_by));
        for (i = 0; i < n_exp; i++) {
            expect_by[m4_expected[i].kind]++;
        }
        for (i = 0; i < (size_t)NYTP_EVT_KIND_COUNT; i++) {
            if (st->by_kind[i] != expect_by[i] &&
                (nytp_event_kind)i != NYTP_EVT_START_DEFLATE) {
                /* START_DEFLATE is control: expected 1 in harness, not in
                 * m4_expected. */
                if ((nytp_event_kind)i == NYTP_EVT_START_DEFLATE) {
                    continue;
                }
                local->kinds_match = 0;
            }
        }
        if (st->by_kind[NYTP_EVT_START_DEFLATE] != 1) {
            local->kinds_match = 0;
        }
        if (st->logical_emits != n_exp) {
            local->kinds_match = 0;
        }
    }
}

nytp_status nytp_m4_mini_sample_run(nytp_sink *sink, nytp_m4_harness_result *out)
{
    nytp_fake_clock fc;
    nytp_stmt_driver drv;
    nytp_status st;
    nytp_ticks attributed;
    nytp_m4_harness_result local;
    size_t n_exp = sizeof(m4_expected) / sizeof(m4_expected[0]);
    /* Track TIME_LINE ticks in order for field checks against expected. */
    nytp_ticks tl_ticks[8];
    size_t n_tl = 0;

    memset(&local, 0, sizeof(local));
    local.first_kind_mismatch = (size_t)-1;
    local.gapless_ok = 1;
    local.kinds_match = 1;
    local.ticks_match = 1;
    local.run_status = NYTP_OK;

    if (!sink) {
        finish_out(out, &local, NYTP_ERR_NULL);
        return NYTP_ERR_NULL;
    }

#define STEP(expr)                                                             \
    do {                                                                       \
        st = (expr);                                                           \
        if (st != NYTP_OK) {                                                   \
            finish_out(out, &local, st);                                       \
            return st;                                                         \
        }                                                                      \
    } while (0)

    /* Header phase in OPEN. */
    STEP(nytp_emit_attribute(sink, nytp_sv_cstr("ticks_per_sec"),
                             nytp_sv_cstr("10000000")));
    STEP(nytp_emit_option(sink, nytp_sv_cstr("calls"), nytp_sv_cstr("1")));
    STEP(nytp_sink_activate(sink));

    /* Control — no seq. */
    STEP(nytp_emit_start_deflate(sink));
    STEP(nytp_emit_pid_start(sink, 42, 1, 0.0));
    STEP(nytp_emit_new_fid(sink, 1, 0, 0, 0, 0, 0, nytp_sv_cstr("m4_mini.pl")));

    nytp_fake_clock_init(&fc, m4_clock_script,
                         sizeof(m4_clock_script) / sizeof(m4_clock_script[0]));
    nytp_stmt_driver_init(&drv, &fc, 1);

    STEP(nytp_stmt_driver_on_line(&drv, sink, 1, &attributed)); /* seed */

    STEP(nytp_stmt_driver_on_line(&drv, sink, 2, &attributed));
    if (attributed != 42) {
        local.ticks_match = 0;
        local.first_kind_mismatch = 4;
    }
    if (n_tl < sizeof(tl_ticks) / sizeof(tl_ticks[0])) {
        tl_ticks[n_tl++] = attributed;
    }

    STEP(nytp_stmt_driver_discount(sink));

    STEP(nytp_stmt_driver_on_line(&drv, sink, 3, &attributed));
    if (attributed != 58) {
        local.ticks_match = 0;
        if (local.first_kind_mismatch == (size_t)-1) {
            local.first_kind_mismatch = 6;
        }
    }
    if (n_tl < sizeof(tl_ticks) / sizeof(tl_ticks[0])) {
        tl_ticks[n_tl++] = attributed;
    }

    STEP(nytp_stmt_driver_on_line(&drv, sink, 4, &attributed));
    if (attributed != 50) {
        local.ticks_match = 0;
        if (local.first_kind_mismatch == (size_t)-1) {
            local.first_kind_mismatch = 7;
        }
    }
    if (n_tl < sizeof(tl_ticks) / sizeof(tl_ticks[0])) {
        tl_ticks[n_tl++] = attributed;
    }

    if (nytp_fake_clock_remaining(&fc) != 0) {
        finish_out(out, &local, NYTP_ERR_STATE);
        return NYTP_ERR_STATE;
    }

    /* Integer ticks so v5 NV + v6 u64 truncation stay E4-equal (PR-B10). */
    STEP(nytp_emit_sub_return(sink, 1, 100.0, 40.0, nytp_sv_cstr("main::leaf")));
    STEP(nytp_sink_begin_finalize(sink));

    /* Hot-path must fail in FINALIZING. */
    if (nytp_emit_time_line(sink, 1, 1, 1) != NYTP_ERR_STATE) {
        finish_out(out, &local, NYTP_ERR_STATE);
        return NYTP_ERR_STATE;
    }

    STEP(nytp_emit_src_line(sink, 1, 1, nytp_sv_cstr("sub leaf { 1 }")));
    STEP(nytp_emit_sub_info(sink, 1, 1, 4, nytp_sv_cstr("main::leaf")));
    STEP(nytp_emit_pid_end(sink, 42, 1.0));
    STEP(nytp_sink_close(sink));

    if (nytp_sink_get_state(sink) != NYTP_SINK_CLOSED) {
        finish_out(out, &local, NYTP_ERR_STATE);
        return NYTP_ERR_STATE;
    }

#undef STEP

    /* Cross-check TIME_LINE ticks against expected steps. */
    {
        size_t ti = 0;
        size_t i;
        for (i = 0; i < n_exp; i++) {
            if (m4_expected[i].kind == NYTP_EVT_TIME_LINE) {
                if (ti >= n_tl || tl_ticks[ti] != m4_expected[i].ticks) {
                    local.ticks_match = 0;
                    if (local.first_kind_mismatch == (size_t)-1) {
                        local.first_kind_mismatch = i;
                    }
                }
                ti++;
            }
        }
        if (ti != n_tl) {
            local.ticks_match = 0;
        }
    }

    verify_observed_stats(sink, &local, n_exp);

    if (!local.gapless_ok || !local.kinds_match || !local.ticks_match) {
        finish_out(out, &local, NYTP_ERR_STATE);
        return NYTP_ERR_STATE;
    }
    finish_out(out, &local, NYTP_OK);
    return NYTP_OK;
}

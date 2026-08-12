/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * TEST-003 / PR-B03 — Deterministic fake-clock harness (scaffold).
 *
 * Development/test only. Production collectors must not enable this by
 * default. Full M4 oracle v5-via-sink equality under fake-clock remains
 * residual until complete TEST-003 corpus (COL-006 real wire is mini-only).
 *
 * BASE-003 model (statement attribution):
 *   on statement entry: now = clock_read(); attribute (now - last) to previous;
 *   last = now; emit TIME_LINE / TIME_BLOCK with fid/line.
 */
#ifndef NYTP_CLOCK_H
#define NYTP_CLOCK_H

#include "nytp_sink.h"
#include "nytp_types.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ---- Scripted fake clock ---- */

typedef struct nytp_fake_clock {
    const nytp_ticks *script; /* absolute tick readings; may be NULL if len==0 */
    size_t len;
    size_t pos;               /* next index to return */
    nytp_ticks last_read;     /* last successful read (0 if none) */
    int has_read;
    int exhausted;            /* 1 after a failed read past end */
} nytp_fake_clock;

/* script may be NULL only when len == 0. Does not copy; caller owns script. */
void nytp_fake_clock_init(nytp_fake_clock *fc, const nytp_ticks *script,
                          size_t len);
void nytp_fake_clock_reset(nytp_fake_clock *fc);

/*
 * Read next absolute tick and advance. Returns NYTP_ERR_EXHAUSTED when script
 * is spent (does not wrap). *out written only on NYTP_OK.
 */
nytp_status nytp_fake_clock_read(nytp_fake_clock *fc, nytp_ticks *out);

/*
 * Peek next absolute tick without advancing. Exhausted does not set
 * fc->exhausted (only a failed consume/read does).
 */
nytp_status nytp_fake_clock_peek(const nytp_fake_clock *fc, nytp_ticks *out);

/*
 * Consume one tick after a successful peek+use. Returns NYTP_ERR_EXHAUSTED
 * if already spent; otherwise advances pos and updates last_read.
 */
nytp_status nytp_fake_clock_consume(nytp_fake_clock *fc);

size_t nytp_fake_clock_remaining(const nytp_fake_clock *fc);

/* ---- Statement-attribution driver (BASE-003) ---- */

typedef struct nytp_stmt_driver {
    nytp_fake_clock *clock;
    nytp_ticks last; /* last absolute reading used for attribution */
    int has_last;
    nytp_fid fid;
    nytp_line prev_line; /* site that receives the next attributed delta */
    int has_prev;
} nytp_stmt_driver;

void nytp_stmt_driver_init(nytp_stmt_driver *d, nytp_fake_clock *clock,
                           nytp_fid fid);

/*
 * On breakable statement entry at `line` (BASE-003):
 *   peek clock; if a previous site exists, emit TIME_LINE(delta, fid, prev_line);
 *   on emit success (or seed path), consume clock and update last/prev_line.
 *   Failed emit does **not** consume the clock tick (retry-safe).
 *   Backwards ticks (now < last) fail closed with NYTP_ERR_OVERFLOW.
 *
 * Returns status from clock or emit. *attributed_ticks (optional) receives
 * the emitted delta, or 0 on seed-only first call.
 */
nytp_status nytp_stmt_driver_on_line(nytp_stmt_driver *d, nytp_sink *sink,
                                     nytp_line line,
                                     nytp_ticks *attributed_ticks);

/*
 * Emit DISCOUNT (logical event) without advancing the statement clock.
 * Used when profiler overhead is excluded between statements.
 */
nytp_status nytp_stmt_driver_discount(nytp_sink *sink);

/* ---- M4 mini-sample harness (scaffold, not full corpus) ---- */

/*
 * One expected logical step after header for the mini M4 sample.
 * seq is the expected COL-003 sequence (logical-only; START_DEFLATE omitted).
 */
typedef struct nytp_m4_step {
    nytp_event_kind kind;
    nytp_ticks ticks; /* TIME_LINE / TIME_BLOCK elapsed; else 0 */
    nytp_fid fid;
    nytp_line line;
} nytp_m4_step;

/*
 * Run a synthetic mini sample that exercises:
 *   OPEN header attrs -> activate -> PID_START -> statements under fake-clock
 *   -> DISCOUNT -> SUB_RETURN -> begin_finalize -> SRC/SUB_INFO -> PID_END
 *   -> close
 * against a counting or v5 wire sink. Verifies gapless seq + kind/ticks
 * order for the *logical* stream (control START_DEFLATE may appear without seq).
 *
 * This is **not** full fixture/v5/default-calls1 oracle equality
 * (complete TEST-003 residual; COL-006 real wire is mini-only here).
 * Returns NYTP_OK on match; NYTP_ERR_STATE on mismatch (details via *out).
 * When `out` is non-NULL it is always written (partial progress on early fail).
 *
 * Verification (counting / v5 wire with stats):
 *   - observed seq ring is gapless 0..n-1
 *   - observed kind ring matches m4_expected[] order
 *   - TIME_LINE ticks match expected steps (via attributed + kind ring)
 */
typedef struct nytp_m4_harness_result {
    size_t logical_events;
    nytp_seq first_seq;
    nytp_seq last_seq;
    int gapless_ok;
    nytp_seq_mismatch mismatch; /* valid if gapless_ok == 0 */
    int kinds_match;
    int ticks_match;
    size_t first_kind_mismatch; /* index into expected steps, or SIZE_MAX */
    nytp_status run_status;     /* last non-OK step status, or NYTP_OK */
} nytp_m4_harness_result;

nytp_status nytp_m4_mini_sample_run(nytp_sink *sink,
                                    nytp_m4_harness_result *out);

/* Expected logical steps for the mini sample (excluding control). */
const nytp_m4_step *nytp_m4_mini_sample_expected(size_t *out_n);

#ifdef __cplusplus
}
#endif

#endif /* NYTP_CLOCK_H */

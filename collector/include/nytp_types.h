/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * Common types for the modernization collector overlay (ADR-0004 / COL-001..003).
 * Stream-neutral: no v5/v6 wire dependency.
 */
#ifndef NYTP_TYPES_H
#define NYTP_TYPES_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Status codes shared by sink open/emit/close paths. */
typedef enum nytp_status {
    NYTP_OK = 0,
    NYTP_ERR_NULL = 1,       /* null sink / required pointer */
    NYTP_ERR_STATE = 2,      /* illegal lifecycle transition (COL-002) */
    NYTP_ERR_IO = 3,         /* write / flush failure */
    NYTP_ERR_OVERFLOW = 4,   /* capacity / encoding overflow */
    NYTP_ERR_FAILED = 5,     /* sink permanently failed */
    NYTP_ERR_UNSUPPORTED = 6,/* operation not implemented by this sink */
    NYTP_ERR_EXHAUSTED = 7   /* fake-clock / script exhausted (TEST-003) */
} nytp_status;

/* Explicit byte string view: no ownership; caller guarantees lifetime. */
typedef struct nytp_string_view {
    const char *ptr; /* may be NULL only when len == 0 */
    size_t len;
    int is_utf8; /* 1 = UTF-8 semantic flag (v5 STRING_UTF8 path); 0 = bytes */
} nytp_string_view;

static inline nytp_string_view nytp_sv(const char *ptr, size_t len, int is_utf8)
{
    nytp_string_view sv;
    sv.ptr = (len == 0) ? NULL : ptr;
    sv.len = len;
    sv.is_utf8 = is_utf8 ? 1 : 0;
    return sv;
}

static inline nytp_string_view nytp_sv_cstr(const char *cstr)
{
    size_t n = 0;
    if (cstr) {
        while (cstr[n] != '\0') {
            n++;
        }
    }
    return nytp_sv(cstr, n, 0);
}

/* Signed 64-bit logical ticks (composition of v5 I32+overflow is open OI-003-01). */
typedef int64_t nytp_ticks;

/* Monotonic logical event sequence (COL-003). Gapless per process stream. */
typedef uint64_t nytp_seq;

/* Opaque process / profile ids as unsigned 32-bit (matches common v5 domains). */
typedef uint32_t nytp_fid;
typedef uint32_t nytp_line;
typedef uint32_t nytp_pid;
typedef uint32_t nytp_depth;

/*
 * Logical event kinds (COMPAT-001 mapped tags + stream controls).
 * Not a v6 opcode freeze. Values are stable for counting / mapping tests only.
 */
typedef enum nytp_event_kind {
    NYTP_EVT_NONE = 0,
    NYTP_EVT_ATTRIBUTE = 1,
    NYTP_EVT_OPTION = 2,
    NYTP_EVT_COMMENT = 3,
    NYTP_EVT_TIME_LINE = 4,
    NYTP_EVT_TIME_BLOCK = 5,
    NYTP_EVT_DISCOUNT = 6,
    NYTP_EVT_NEW_FID = 7,
    NYTP_EVT_SRC_LINE = 8,
    NYTP_EVT_SUB_INFO = 9,
    NYTP_EVT_SUB_CALLERS = 10,
    NYTP_EVT_PID_START = 11,
    NYTP_EVT_PID_END = 12,
    NYTP_EVT_SUB_ENTRY = 13,
    NYTP_EVT_SUB_RETURN = 14,
    /* Stream control (not a logical profile event per COMPAT-001). */
    NYTP_EVT_START_DEFLATE = 15,
    NYTP_EVT_KIND_COUNT
} nytp_event_kind;

/*
 * COL-002 sink lifecycle (plan §9). Scaffold freezes the common path;
 * full fork/signal/embedded matrix remains COL-015 / residual.
 *
 *   UNINITIALIZED -> OPEN -> ACTIVE -> STOPPED -> FINALIZING -> CLOSED
 *                          |          |
 *                          |          +-> ACTIVE (restart)
 *                          +-> FORK_SPLIT -> parent ACTIVE / child OPEN
 *                          +-> FAILED
 *   Any non-CLOSED may transition to FAILED; FAILED may close.
 */
typedef enum nytp_sink_state {
    NYTP_SINK_UNINITIALIZED = 0,
    NYTP_SINK_OPEN = 1,
    NYTP_SINK_ACTIVE = 2,
    NYTP_SINK_STOPPED = 3,
    NYTP_SINK_FINALIZING = 4,
    NYTP_SINK_CLOSED = 5,
    NYTP_SINK_FAILED = 6,
    NYTP_SINK_FORK_SPLIT = 7
} nytp_sink_state;

#ifdef __cplusplus
}
#endif

#endif /* NYTP_TYPES_H */

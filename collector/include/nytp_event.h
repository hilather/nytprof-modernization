/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * Fixed in-memory logical event representation (COL-004/005).
 * Stream-neutral POD headers; variable bytes live in the batch arena.
 * Not a v5/v6 wire layout.
 */
#ifndef NYTP_EVENT_H
#define NYTP_EVENT_H

#include "nytp_types.h"

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Arena-resident string: offset into the owning batch arena + length + UTF-8 flag.
 * Never a borrowed Perl SV pointer — lifetime is the batch until reset-after-flush.
 */
typedef struct nytp_arena_str {
    uint32_t off;  /* byte offset into arena (0 when len==0) */
    uint32_t len;
    int is_utf8;
} nytp_arena_str;

/*
 * Fixed-size event header. Common statement events (TIME_LINE / TIME_BLOCK /
 * DISCOUNT / SUB_ENTRY) use only POD fields — no arena traffic on that path.
 */
typedef struct nytp_event {
    nytp_event_kind kind;
    nytp_seq seq; /* COL-003: assigned at successful append by batch sink */
    union {
        struct {
            nytp_ticks ticks;
            nytp_fid fid;
            nytp_line line;
        } time_line;
        struct {
            nytp_ticks ticks;
            nytp_fid fid;
            nytp_line line;
            nytp_line block_line;
            nytp_line sub_line;
        } time_block;
        struct {
            nytp_arena_str key;
            nytp_arena_str value;
        } attr; /* ATTRIBUTE / OPTION share shape */
        struct {
            nytp_arena_str text;
        } comment;
        struct {
            nytp_fid fid;
            nytp_fid eval_fid;
            nytp_line eval_line;
            uint32_t flags;
            uint32_t size;
            uint32_t mtime;
            nytp_arena_str name;
        } new_fid;
        struct {
            nytp_fid fid;
            nytp_line line;
            nytp_arena_str text;
        } src_line;
        struct {
            nytp_fid fid;
            nytp_line first_line;
            nytp_line last_line;
            nytp_arena_str name;
        } sub_info;
        struct {
            nytp_fid fid;
            nytp_line line;
            uint32_t count;
            double incl;
            double excl;
            double reci;
            uint32_t rec_depth;
            nytp_arena_str called;
            nytp_arena_str caller;
        } sub_callers;
        struct {
            nytp_pid pid;
            nytp_pid ppid;
            double start_time;
        } pid_start;
        struct {
            nytp_pid pid;
            double end_time;
        } pid_end;
        struct {
            nytp_fid caller_fid;
            nytp_line caller_line;
        } sub_entry;
        struct {
            nytp_depth depth;
            double incl_time;
            double excl_time;
            nytp_arena_str subname;
        } sub_return;
        /* START_DEFLATE / DISCOUNT: no payload */
    } u;
} nytp_event;

#ifdef __cplusplus
}
#endif

#endif /* NYTP_EVENT_H */

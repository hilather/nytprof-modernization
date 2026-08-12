/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * Test / dual-mode counting sink: records per-kind multiplicities without I/O.
 * Used by collector unit tests and as a dual-sink companion scaffold.
 */
#ifndef NYTP_SINK_COUNTING_H
#define NYTP_SINK_COUNTING_H

#include "nytp_sink.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct nytp_counting_stats {
    uint64_t by_kind[NYTP_EVT_KIND_COUNT];
    uint64_t total_emits;
    nytp_event_kind last_kind;
    /* Last TIME_LINE / TIME_BLOCK fingerprint for field-routing tests. */
    nytp_ticks last_ticks;
    nytp_fid last_fid;
    nytp_line last_line;
    nytp_line last_block_line;
    nytp_line last_sub_line;
    /* Last sub_return fingerprint. */
    nytp_depth last_depth;
    char last_subname[128];
    size_t last_subname_len;
} nytp_counting_stats;

/* Heap-allocate a counting sink (OPEN). Destroy with nytp_sink_destroy. */
nytp_sink *nytp_counting_sink_create(void);

/* Borrow stats; valid until destroy. Returns NULL if not a counting sink. */
const nytp_counting_stats *nytp_counting_sink_stats(const nytp_sink *sink);

#ifdef __cplusplus
}
#endif

#endif /* NYTP_SINK_COUNTING_H */

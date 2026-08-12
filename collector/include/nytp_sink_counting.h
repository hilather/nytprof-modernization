/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * Test / dual-mode counting sink: records per-kind multiplicities without I/O.
 * Used by collector unit tests and as a dual-sink companion scaffold.
 * COL-003: records last_seq and a bounded seq ring for gapless checks.
 */
#ifndef NYTP_SINK_COUNTING_H
#define NYTP_SINK_COUNTING_H

#include "nytp_sink.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Bounded ring of recent logical seq numbers for comparator tests. */
#define NYTP_COUNTING_SEQ_RING 256

typedef struct nytp_counting_stats {
    uint64_t by_kind[NYTP_EVT_KIND_COUNT];
    uint64_t total_emits;       /* includes control (START_DEFLATE) */
    uint64_t logical_emits;     /* COL-003 sequenced events only */
    nytp_event_kind last_kind;
    nytp_seq last_seq;          /* last logical seq observed (0 if none) */
    int has_last_seq;
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
    /* Seq ring (logical emits only), oldest→newest for first min(n, RING). */
    nytp_seq seq_ring[NYTP_COUNTING_SEQ_RING];
    size_t seq_ring_len; /* number of valid entries (≤ RING; drops oldest) */
} nytp_counting_stats;

/* Heap-allocate a counting sink (OPEN). Destroy with nytp_sink_destroy. */
nytp_sink *nytp_counting_sink_create(void);

/* Borrow stats; valid until destroy. Returns NULL if not a counting sink. */
const nytp_counting_stats *nytp_counting_sink_stats(const nytp_sink *sink);

/*
 * Copy logical seq ring into out[0..*out_n). *out_n in/out capacity/count.
 * Returns NYTP_ERR_NULL if not a counting sink; NYTP_ERR_OVERFLOW if buffer
 * too small (still fills what fits and sets *out_n to needed size).
 */
nytp_status nytp_counting_sink_copy_seqs(const nytp_sink *sink, nytp_seq *out,
                                         size_t *out_n);

#ifdef __cplusplus
}
#endif

#endif /* NYTP_SINK_COUNTING_H */

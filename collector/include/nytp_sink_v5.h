/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-001 stub v5 adapter: implements the semantic sink API as the
 * conceptual route for legacy v5 writes.
 *
 * This PR does NOT encode real v5 wire bytes (that is COL-006 once the
 * legacy writer is adapted). The stub:
 *   - accepts the full emit surface used by the counting sink;
 *   - tracks multiplicities + last event for unit tests;
 *   - remains stream-neutral at the call boundary (no format=v6 path).
 *
 * COL-007 (C v6 writer) is explicitly out of scope.
 */
#ifndef NYTP_SINK_V5_H
#define NYTP_SINK_V5_H

#include "nytp_sink.h"
#include "nytp_sink_counting.h"

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Create a stub v5 sink. Optional path is stored for diagnostics only
 * (not opened; no file I/O in this scaffold). path may be NULL.
 */
nytp_sink *nytp_v5_sink_create(const char *path);

/* True if sink was created by nytp_v5_sink_create. */
int nytp_v5_sink_is_v5(const nytp_sink *sink);

/* Stats share the counting layout so tests can compare routing. */
const nytp_counting_stats *nytp_v5_sink_stats(const nytp_sink *sink);

/* Stored path pointer (may be NULL); not a copy of caller memory beyond create. */
const char *nytp_v5_sink_path(const nytp_sink *sink);

#ifdef __cplusplus
}
#endif

#endif /* NYTP_SINK_V5_H */

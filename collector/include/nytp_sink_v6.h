/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-007 (PR-B06) — Absolute v6 writer MVP (staged).
 *
 * Routes COMPAT-001 emits through provisional format-v6 absolute EVENT
 * bodies (codec NONE). Layout matches crates/nytprof-format-v6 +
 * collector/include/nytprof_v6_ids.h lockfile constants.
 *
 * Residuals (honest):
 *   - Not packing (ADR-0001 site-delta / TIME_*_RUN / FLAG_HAS_SEQ) — PR-B08.
 *   - Not multi-chunk / CRC / ZLIB|ZSTD|LZ4 payload codecs — PR-B07.
 *   - Not FOOTER string dict (ADR-0002) — PR-B08.
 *   - Not wire freeze; not board COL-007 done (E3-C = PR-B09).
 *   - NEW_FID drops eval_*, flags, size, mtime (provisional absolute shape).
 *   - TIME_BLOCK drops sub_line (provisional absolute shape).
 *   - NV doubles projected to non-negative integer ULEB (fail closed).
 *   - START_DEFLATE is empty marker only (no mid-stream codec switch).
 *   - Default does not write COL-003 seq on the wire (no FLAG_HAS_SEQ).
 */
#ifndef NYTP_SINK_V6_H
#define NYTP_SINK_V6_H

#include "nytp_sink.h"
#include "nytp_sink_counting.h"

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Create an absolute v6 wire sink (codec NONE EVENT).
 * - path may be NULL: wire accumulates in memory only (tests).
 * - path non-NULL: on successful flush/close, sealed buffer is written
 *   to path (create/truncate). Path is copied; caller may free.
 * Writes provisional file-prefix (fixed header + empty TLV END) immediately.
 */
nytp_sink *nytp_v6_sink_create(const char *path);

/*
 * Create with explicit header minor / feature bits / optional header CRC
 * (stored as-is; B06 does not compute real CRC — PR-B07).
 */
nytp_sink *nytp_v6_sink_create_ex(const char *path, uint16_t minor,
                                  uint64_t required_features,
                                  uint64_t optional_features,
                                  uint32_t header_crc);

/* True if sink was created by nytp_v6_sink_create / create_ex. */
int nytp_v6_sink_is_v6(const nytp_sink *sink);

/* Stats share the counting layout so tests can compare routing. */
const nytp_counting_stats *nytp_v6_sink_stats(const nytp_sink *sink);

/*
 * Returns sink-owned path copy (or NULL if create had no path).
 * Valid until nytp_sink_destroy; do not free.
 */
const char *nytp_v6_sink_path(const nytp_sink *sink);

/*
 * Borrow sealed wire buffer after successful close, or prefix-only mid-stream.
 * Decoder-ready mini-profile only after nytp_sink_close (EVENT chunk sealed).
 * *out_len receives byte length. Returns NULL if not a v6 sink.
 */
const uint8_t *nytp_v6_sink_wire(const nytp_sink *sink, size_t *out_len);

/* Byte length of current wire buffer (0 if not a v6 sink). */
size_t nytp_v6_sink_wire_len(const nytp_sink *sink);

/* 1 if a path was configured and the buffer has been written to it. */
int nytp_v6_sink_file_written(const nytp_sink *sink);

/* 1 after close has sealed the EVENT chunk into the wire buffer. */
int nytp_v6_sink_is_sealed(const nytp_sink *sink);

/* Header major/minor (always major 6 for this adapter). */
void nytp_v6_sink_version(const nytp_sink *sink, uint16_t *major,
                          uint16_t *minor);

/* Logical event count that will be written into the EVENT chunk header. */
uint32_t nytp_v6_sink_event_count(const nytp_sink *sink);

/*
 * Borrow the open event-body buffer (absolute records, not yet framed).
 * Valid until next emit or seal/destroy. NULL if not v6 or already sealed.
 */
const uint8_t *nytp_v6_sink_event_body(const nytp_sink *sink, size_t *out_len);

/*
 * Test hook: force open event-body length (zeros reserved capacity).
 * Used to exercise mid-record fail-closed rollback near MAX_EVENT_BODY_BYTES
 * without multi-gigabyte emit loops. No-op / ERR if not v6 or sealed or
 * len > MAX_EVENT_BODY_BYTES.
 */
nytp_status nytp_v6_sink_test_force_body_len(nytp_sink *sink, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* NYTP_SINK_V6_H */

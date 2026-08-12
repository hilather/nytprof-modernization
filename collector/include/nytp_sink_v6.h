/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-007 (PR-B06..B08) — Provisional v6 wire sink.
 *
 * Routes COMPAT-001 emits through provisional format-v6 EVENT bodies.
 * Layout matches crates/nytprof-format-v6 + nytprof_v6_ids.h.
 *
 * PR-B06: absolute EVENT bodies + file-prefix
 * PR-B07: EVENT codecs NONE/ZLIB/ZSTD/LZ4; multi-chunk; header+payload CRC
 * PR-B08: ADR-0001 packing (site-delta + FLAG_HAS_SEQ continuity);
 *         mid-stream codec region + empty START_DEFLATE marker;
 *         ADR-0002 FOOTER-local string dictionary
 *
 * Residuals (honest):
 *   - Board COL-007 done for product E3-EVENT (PR-B09 fixtures/v6/from-c/).
 *   - E3-mixed multi-kind C fixtures residual; not wire freeze; not CLI v6 default.
 *   - NEW_FID drops eval_*, flags, size, mtime (provisional absolute shape).
 *   - TIME_BLOCK drops sub_line (provisional absolute shape).
 *   - NV doubles projected to non-negative integer ULEB (fail closed).
 *   - Default create remains absolute / no packing / no FOOTER dict.
 */
#ifndef NYTP_SINK_V6_H
#define NYTP_SINK_V6_H

#include "nytp_sink.h"
#include "nytp_sink_counting.h"
#include "nytprof_v6_ids.h"

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Optional create knobs (zero-init = absolute NONE single-chunk, no packing/dict).
 * header_crc is ignored — CRC is always sealed over the first 32 header bytes.
 */
typedef struct nytp_v6_sink_options {
    uint16_t minor;
    uint64_t required_features;
    uint64_t optional_features;
    uint8_t event_codec; /* NYTPROF_V6_CODEC_* */
    size_t max_records_per_chunk; /* 0 = unlimited single chunk per region */
    int enable_packing; /* ADR-0001: site-delta + FLAG_HAS_SEQ continuity */
    int enable_string_dict; /* ADR-0002: FOOTER-local string dictionary */
} nytp_v6_sink_options;

/*
 * Create an absolute v6 wire sink (codec NONE EVENT, unlimited single chunk).
 * Header CRC and chunk payload CRC are sealed on write/close.
 * - path may be NULL: wire accumulates in memory only (tests).
 * - path non-NULL: on successful flush/close, sealed buffer is written
 *   to path (create/truncate). Path is copied; caller may free.
 * Writes provisional file-prefix (fixed header + empty TLV END) immediately.
 */
nytp_sink *nytp_v6_sink_create(const char *path);

/*
 * Create with explicit header minor / feature bits.
 * header_crc is ignored — CRC is always sealed over the first 32 header bytes
 * (PR-B07; matches format-v6 encode_fixed_header_full_sealed).
 */
nytp_sink *nytp_v6_sink_create_ex(const char *path, uint16_t minor,
                                  uint64_t required_features,
                                  uint64_t optional_features,
                                  uint32_t header_crc);

/*
 * Create with EVENT payload codec + multi-chunk partition limit.
 * event_codec: NYTPROF_V6_CODEC_{NONE,ZLIB,ZSTD,LZ4}; others → NULL.
 * max_records_per_chunk:
 *   0  = unlimited → at most one EVENT chunk (mini-profile shape)
 *   n>=1 = split into EVENT chunks of at most n records each (sequence 0..k-1)
 * Header + payload CRCs always sealed. Packing/dict off.
 */
nytp_sink *nytp_v6_sink_create_codec(const char *path, uint8_t event_codec,
                                     size_t max_records_per_chunk);

/*
 * Full create: minor/features + codec + multi-chunk. header_crc ignored (sealed).
 */
nytp_sink *nytp_v6_sink_create_codec_ex(const char *path, uint16_t minor,
                                        uint64_t required_features,
                                        uint64_t optional_features,
                                        uint8_t event_codec,
                                        size_t max_records_per_chunk);

/*
 * Full create with packing / string-dict options (PR-B08).
 * NULL opt → same as nytp_v6_sink_create(path). Unsupported codec → NULL.
 */
nytp_sink *nytp_v6_sink_create_opts(const char *path,
                                    const nytp_v6_sink_options *opt);

/* True if sink was created by nytp_v6_sink_create* / create_codec* / create_opts. */
int nytp_v6_sink_is_v6(const nytp_sink *sink);

/* Stats share the counting layout so tests can compare routing. */
const nytp_counting_stats *nytp_v6_sink_stats(const nytp_sink *sink);

/*
 * Returns sink-owned path copy (or NULL if create had no path).
 * Valid until nytp_sink_destroy; do not free.
 */
const char *nytp_v6_sink_path(const nytp_sink *sink);

/*
 * Borrow sealed wire buffer after successful close, or prefix (+ sealed regions)
 * mid-stream. Decoder-ready complete profile only after nytp_sink_close
 * (EVENT region(s) sealed; FOOTER dict when enabled).
 * *out_len receives byte length. Returns NULL if not a v6 sink.
 */
const uint8_t *nytp_v6_sink_wire(const nytp_sink *sink, size_t *out_len);

/* Byte length of current wire buffer (0 if not a v6 sink). */
size_t nytp_v6_sink_wire_len(const nytp_sink *sink);

/* 1 if a path was configured and the buffer has been written to it. */
int nytp_v6_sink_file_written(const nytp_sink *sink);

/* 1 after close has sealed EVENT region(s) (+ optional FOOTER) into wire. */
int nytp_v6_sink_is_sealed(const nytp_sink *sink);

/* Header major/minor (always major 6 for this adapter). */
void nytp_v6_sink_version(const nytp_sink *sink, uint16_t *major,
                          uint16_t *minor);

/* Logical event count in the open body region (not cumulative mid-stream). */
uint32_t nytp_v6_sink_event_count(const nytp_sink *sink);

/* Configured EVENT payload codec for the **current** region (NYTPROF_V6_CODEC_*). */
uint8_t nytp_v6_sink_event_codec(const nytp_sink *sink);

/* Configured max records per EVENT chunk (0 = unlimited). */
size_t nytp_v6_sink_max_records_per_chunk(const nytp_sink *sink);

/*
 * Number of EVENT chunks sealed so far (region seals + final). 0 before any seal.
 */
uint32_t nytp_v6_sink_event_chunk_count(const nytp_sink *sink);

/* 1 if packing (ADR-0001) is enabled. */
int nytp_v6_sink_packing_enabled(const nytp_sink *sink);

/* 1 if FOOTER string-dict (ADR-0002) is enabled. */
int nytp_v6_sink_string_dict_enabled(const nytp_sink *sink);

/* 1 after final close sealed a FOOTER string-dictionary chunk. */
int nytp_v6_sink_has_footer_dict(const nytp_sink *sink);

/* Dictionary entry count (interned strings); 0 if dict disabled. */
uint32_t nytp_v6_sink_dict_entry_count(const nytp_sink *sink);

/*
 * Mid-stream codec region switch (PR-B08 / ADR-0001 §6):
 *   1. Emit empty START_DEFLATE marker into the current open body.
 *   2. Seal current body as EVENT chunk(s) under the current codec (region seal;
 *      not final profile seal — packing state continues).
 *   3. Subsequent emits use next_codec for chunk payloads.
 * next_codec must be supported and **must differ** from the current codec.
 * Fail-closed if sealed / sticky FAILED / lifecycle not OPEN|ACTIVE /
 * empty open body before marker / unsupported or same codec.
 * On seal failure after emitting START_DEFLATE, the marker is rolled back so
 * a later retry does not double-emit.
 */
nytp_status nytp_v6_sink_begin_codec_region(nytp_sink *sink, uint8_t next_codec);

/*
 * Optional packed same-site run (ADR-0001 TIME_LINE_RUN). Only when packing is
 * enabled. Emits one wire record expanding to n_ticks logical TIME_LINE events
 * for FLAG_HAS_SEQ base..base+N-1. n_ticks must be 1..MAX_TIME_RUN_LEN.
 * Advances packing SiteCursor to (fid,line).
 * Gated like nytp_emit_time_line (OPEN/ACTIVE; sticky FAILED rejected).
 * On success, advances COL-003 logical sink seq by n_ticks.
 */
nytp_status nytp_v6_sink_emit_time_line_run(nytp_sink *sink, nytp_fid fid,
                                            nytp_line line,
                                            const uint64_t *ticks,
                                            size_t n_ticks);

/*
 * Borrow the open event-body buffer (not yet framed). Valid until next emit,
 * region seal, or seal/destroy. NULL if not v6 or already final-sealed.
 */
const uint8_t *nytp_v6_sink_event_body(const nytp_sink *sink, size_t *out_len);

/*
 * Test hook: force open event-body length (zeros reserved capacity).
 * Used to exercise mid-record fail-closed rollback near MAX_EVENT_BODY_BYTES
 * without multi-gigabyte emit loops. No-op / ERR if not v6 or sealed or
 * len > MAX_EVENT_BODY_BYTES.
 */
nytp_status nytp_v6_sink_test_force_body_len(nytp_sink *sink, size_t len);

/*
 * Test hook: fail seal after successfully framing N EVENT chunks in the current
 * seal attempt, rewinding wire to the pre-attempt mark (atomic multi-chunk seal
 * regression). 0 disables.
 */
void nytp_v6_sink_test_fail_seal_after_chunks(nytp_sink *sink, uint32_t n);

/*
 * Test hook: run EVENT seal of open body without lifecycle close transition.
 * Does not emit FOOTER (use public close for full finalization). Used to
 * exercise mid-seal abort + successful retry.
 */
nytp_status nytp_v6_sink_test_try_seal(nytp_sink *sink);

#ifdef __cplusplus
}
#endif

#endif /* NYTP_SINK_V6_H */

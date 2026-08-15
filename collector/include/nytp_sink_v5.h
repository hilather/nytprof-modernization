/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-006 — Legacy v5 writer on the semantic sink API.
 *
 * Routes COMPAT-001 emits through real NYTProf v5 wire encoding
 * (FileHandle.xs / 6.15 protocol). Unmodified 6.15 tools and the
 * independent Rust v5 decoder (nytprof-format-v5) should accept output
 * when the stream is well-formed.
 *
 * Residuals (honest):
 *   - Not full M4 oracle corpus byte/stream equality (complete TEST-003).
 *   - nytp_ticks > I32 range fails closed (OI-003-01 overflow composition).
 *   - Not COL-007 (v6 writer); not live XS hooks.
 *   - Default does not write COL-003 seq on the wire.
 *   - START_DEFLATE enables zlib (windowBits=15); compress level default 6.
 */
#ifndef NYTP_SINK_V5_H
#define NYTP_SINK_V5_H

#include "nytp_sink.h"
#include "nytp_sink_counting.h"

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Create a v5 wire sink.
 * - path may be NULL: wire accumulates in memory only (tests).
 * - path non-NULL: on successful flush/close, buffer is written to path
 *   (create/truncate). Path is copied; caller may free.
 * Writes "NYTProf 5 0\n" header immediately on create.
 */
nytp_sink *nytp_v5_sink_create(const char *path);

/*
 * Create with explicit zlib compression level for START_DEFLATE
 * (1..9, or 0 to leave zlib default of 6 when deflate starts).
 * Level is applied only when emit_start_deflate is called.
 */
nytp_sink *nytp_v5_sink_create_ex(const char *path, int compress_level);

/* True if sink was created by nytp_v5_sink_create / create_ex. */
int nytp_v5_sink_is_v5(const nytp_sink *sink);

/* Stats share the counting layout so tests can compare routing. */
const nytp_counting_stats *nytp_v5_sink_stats(const nytp_sink *sink);

/*
 * Returns sink-owned path copy (or NULL if create had no path).
 * Valid until nytp_sink_destroy; do not free.
 */
const char *nytp_v5_sink_path(const nytp_sink *sink);

/*
 * Borrow current wire buffer (header + events; may include zlib body after
 * START_DEFLATE). Valid until next emit/flush that grows the buffer or destroy.
 * *out_len receives byte length. Returns NULL if not a v5 sink.
 *
 * Decoder-ready: only after nytp_sink_close (deflate finished if active).
 * Mid-stream flush while deflating writes an unfinished zlib snapshot to
 * path — not a complete profile for nytprof-dump / 6.15 readers.
 */
const uint8_t *nytp_v5_sink_wire(const nytp_sink *sink, size_t *out_len);

/* Byte length of current wire buffer (0 if not a v5 sink). */
size_t nytp_v5_sink_wire_len(const nytp_sink *sink);

/* 1 if a path was configured and the buffer has been written to it. */
int nytp_v5_sink_file_written(const nytp_sink *sink);

/* 1 if START_DEFLATE has been emitted (subsequent body is zlib). */
int nytp_v5_sink_is_deflating(const nytp_sink *sink);

/* Major/minor written in the header (always 5 / 0 for this adapter). */
void nytp_v5_sink_version(const nytp_sink *sink, uint32_t *major,
                          uint32_t *minor);

/*
 * COL-015 path ownership:
 * Detach output path so subsequent flush/close will not write a file
 * (in-memory wire retained). Avoids child double-write to parent addpid base.
 */
nytp_status nytp_v5_sink_detach_path(nytp_sink *sink);

/*
 * COL-015: rebind output path (copied). NULL path == detach.
 * Does not clear the wire buffer (use fork_child_reinit for a clean stream).
 */
nytp_status nytp_v5_sink_rebind_path(nytp_sink *sink, const char *path);

/*
 * COL-015 child post-fork re-init (same sink object retained across fork):
 *   - rebind path to new_path (NULL => detach; typical addpid child path)
 *   - abort mid-stream zlib if active (child does not inherit compressor)
 *   - clear wire buffer and rewrite "NYTProf 5 0\n" header
 *   - reset counting stats and file_written
 * Lifecycle/seq are owned by nytp_fork_resume_child — call reinit after that
 * (or while FORK_SPLIT / OPEN). Fails if CLOSED/FAILED.
 */
nytp_status nytp_v5_sink_fork_child_reinit(nytp_sink *sink, const char *new_path);

/*
 * Durable sealed publish (item 2 / D2):
 * Live RAM stays uncompressed. flush/close become this publish when
 * nytp_v5_sink_set_durable(sink, 1). Snapshot is tmp+fsync+rename.
 * compress_level > 0 inserts `z` + Z_FINISH copy; live buffer is unchanged.
 * Idempotent when live length equals the last successful seal.
 */
nytp_status nytp_v5_sink_set_durable(nytp_sink *sink, int durable);
int nytp_v5_sink_is_durable(const nytp_sink *sink);
nytp_status nytp_v5_sink_mark_header_end(nytp_sink *sink);
size_t nytp_v5_sink_header_end(const nytp_sink *sink);
size_t nytp_v5_sink_len_at_last_seal(const nytp_sink *sink);
nytp_status nytp_v5_seal_publish(nytp_sink *sink);

#ifdef __cplusplus
}
#endif

#endif /* NYTP_SINK_V5_H */

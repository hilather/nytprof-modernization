/**
 * nytprof_ffi.h — stable C ABI for NYTProf profile open / query / close
 *
 * Crate: nytprof-ffi (cdylib + rlib)
 * Schema: https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/ffi-cdylib-mvp-v0.md
 * Plan: RUST-010 MVP (PR-A05 / OQ-2)
 *
 * Residual honesty (MVP, not full RUST-010):
 *  - no per-event callbacks / batch event walk
 *  - no automated ABI freeze tooling (BUILD-007)
 *  - no production shared-library install path (CLI remains primary preview surface)
 *  - no XS Data / ReadStream (PERL-004 / PERL-005 — PR-A06)
 *
 * Panic policy: no Rust panic crosses this ABI (contained → NYTPROF_ERR_PANIC).
 * Dual-path: legacy installs must work without loading this library.
 *
 * Link: -lnytprof_ffi  (cargo build -p nytprof-ffi produces libnytprof_ffi.so / .dylib / .dll)
 */

#ifndef NYTPROF_FFI_H
#define NYTPROF_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** ABI major version implemented by this header / library. */
#define NYTPROF_FFI_ABI_VERSION 1u

/** Open flags */
#define NYTPROF_OPEN_ALLOW_INCOMPLETE 1u

/** Status codes (non-zero = error) */
enum {
    NYTPROF_OK = 0,
    NYTPROF_ERR_NULL = 1,
    NYTPROF_ERR_INVALID_UTF8 = 2,
    NYTPROF_ERR_IO_DECODE = 3,
    NYTPROF_ERR_INCOMPLETE = 4,
    NYTPROF_ERR_NOT_FOUND = 5,
    NYTPROF_ERR_PANIC = 6,
    NYTPROF_ERR_ABI = 7,
    NYTPROF_ERR_INVALID_HANDLE = 8
};

/** Opaque owned profile handle. */
typedef struct nytprof_profile nytprof_profile_t;

/**
 * Aggregate counters filled by nytprof_profile_stats.
 * Layout is part of the ABI; append-only across minor revisions of major=1.
 */
typedef struct nytprof_profile_stats {
    uint64_t total_events;
    uint64_t discount_events;
    uint64_t sub_entry_events;
    uint64_t sub_return_events;
    uint64_t time_line_events;
    uint64_t time_block_events;
    uint64_t pid_start_events;
    uint64_t pid_end_events;
    uint64_t new_fid_events;
    uint64_t sub_callers_events;
    uint64_t src_line_events;
    uint64_t sub_info_events;
    int is_stream_complete; /* 1 or 0 */
} nytprof_profile_stats_t;

/** Library ABI version (runtime). */
uint32_t nytprof_ffi_abi_version(void);

/** Return 1 if want is compatible, else 0. MVP: exact match on major 1. */
int nytprof_ffi_abi_compatible(uint32_t want);

/**
 * Thread-local last error message (UTF-8, never null).
 * Valid until the next FFI call on this thread that sets/clears the error.
 */
const char *nytprof_last_error(void);

/**
 * Open a v5 profile path into an owned handle.
 * flags=0: fail closed on incomplete streams (COMPAT-010).
 * flags|=NYTPROF_OPEN_ALLOW_INCOMPLETE: allow incomplete models when decode succeeds.
 * On success *out is non-null and must be closed with nytprof_profile_close.
 * On failure *out is null (when out is non-null).
 */
int nytprof_profile_open(const char *path, uint32_t flags, nytprof_profile_t **out);

/** Free a profile handle. NULL is a no-op. */
void nytprof_profile_close(nytprof_profile_t *profile);

/** Fill aggregate counters (coarse-grained; no per-event FFI). */
int nytprof_profile_stats(const nytprof_profile_t *profile, nytprof_profile_stats_t *out);

/** SUB_RETURN return count for subname (0 if absent). */
int nytprof_profile_sub_returns(
    const nytprof_profile_t *profile,
    const char *subname,
    uint64_t *returns_out);

/** A7 call-edge count for (caller, called) (0 if absent). */
int nytprof_profile_call_edge_count(
    const nytprof_profile_t *profile,
    const char *caller,
    const char *called,
    uint64_t *count_out);

/** A4 line call count for (fid, line) (0 if absent). */
int nytprof_profile_line_calls(
    const nytprof_profile_t *profile,
    uint32_t fid,
    uint32_t line,
    uint64_t *calls_out);

/** A4b block-line call count for (fid, block_line) (0 if absent). */
int nytprof_profile_block_line_calls(
    const nytprof_profile_t *profile,
    uint32_t fid,
    uint32_t block_line,
    uint64_t *calls_out);

#ifdef __cplusplus
}
#endif

#endif /* NYTPROF_FFI_H */

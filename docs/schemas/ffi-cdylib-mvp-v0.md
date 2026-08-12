# FFI cdylib open/query/close MVP (v0)

**Board ID:** `FFI-CDYLIB-MVP` (PR-A05 / **OQ-2** / toward **RUST-010**)  
**Status:** implemented (product path MVP — **not** full RUST-010 freeze)  
**Crate:** [`crates/nytprof-ffi/`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-ffi/)  
**C header:** [`crates/nytprof-ffi/include/nytprof_ffi.h`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-ffi/include/nytprof_ffi.h)  
**Policy ADR:** [`docs/adrs/0003-r1-full-residual-policy.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) (OQ-2: FFI must **CLOSE**, not waive)

## Goal

Ship a **stable, panic-safe C ABI** for embedders and future XS (PR-A06) to open a v5 profile, query coarse aggregates / semantic counts, and close the handle — without requiring per-event FFI or the CLI subprocess bridge.

This is the **product path** for residual row “No production C ABI / FFI / cdylib.” It does **not** claim full RUST-010 completeness (batch structures, ASan harness package, BUILD-007 header automation, production dylib install).

## Non-goals (residual honesty)

| Residual | Notes |
|----------|--------|
| Full RUST-010 | No streaming event callbacks, batch walk APIs, or versioned callback vtables |
| BUILD-007 | Header is hand-maintained; no automated ABI freeze / cbindgen pipeline |
| BUILD-004 production install | Shared library is built via `cargo build -p nytprof-ffi`; not installed by `install_native.sh` in this MVP |
| PERL-004 / PERL-005 | XS ReadStream / Data materializer remains PR-A06 |
| Dual-path without dylib | Legacy / CLI-only installs must keep working **without** loading `libnytprof_ffi` |
| v6 / COL-007 | v5 `ProfileModel::from_path` only |
| Perf claims | No public SLOs for FFI path |

## ABI version

| Symbol | Rule |
|--------|------|
| `NYTPROF_FFI_ABI_VERSION` | Compile-time major = **1** (header + `nytprof_ffi_abi_version()`) |
| `nytprof_ffi_abi_compatible(want)` | MVP: returns **1** iff `want == 1`, else **0** |
| Mismatch | Callers must not open profiles when incompatible; library fails cleanly (no partial open) |

## Lifecycle

```text
nytprof_ffi_abi_compatible(1) == 1
        |
        v
nytprof_profile_open(path, flags, &handle)  -- fail-closed incomplete unless ALLOW
        |
        +--> nytprof_profile_stats / sub_returns / call_edge_count / line_calls / ...
        |
        v
nytprof_profile_close(handle)
```

| Function | Semantics |
|----------|-----------|
| `nytprof_profile_open` | Decode v5 path → `ProfileModel`; default `flags=0` requires `is_stream_complete` (COMPAT-010). `NYTPROF_OPEN_ALLOW_INCOMPLETE` permits incomplete models when decode succeeds. |
| `nytprof_profile_close` | Free handle; NULL is a no-op |
| `nytprof_profile_stats` | Single bulk counter fill (`total_events`, `discount_events`, `sub_entry_events`, stream/PID/tag multiplicities, `is_stream_complete`) |
| `nytprof_profile_sub_returns` | A5 return count for subname (0 if absent) |
| `nytprof_profile_call_edge_count` | A7 edge `count` (0 if absent) |
| `nytprof_profile_line_calls` | A4 `(fid,line)` calls (0 if absent) |
| `nytprof_profile_block_line_calls` | A4b `(fid,block_line)` calls (0 if absent) |
| `nytprof_last_error` | Thread-local UTF-8 message; never null; valid until next error-setting call on the same thread |

## Status codes

| Code | Name | Meaning |
|------|------|---------|
| 0 | `NYTPROF_OK` | Success |
| 1 | `NYTPROF_ERR_NULL` | Required pointer was null |
| 2 | `NYTPROF_ERR_INVALID_UTF8` | Path/name not valid UTF-8 |
| 3 | `NYTPROF_ERR_IO_DECODE` | I/O or v5 decode/model error |
| 4 | `NYTPROF_ERR_INCOMPLETE` | Stream incomplete under COMPAT-010 (default open) |
| 5 | `NYTPROF_ERR_NOT_FOUND` | Reserved (query absence currently returns OK + 0) |
| 6 | `NYTPROF_ERR_PANIC` | Panic contained at FFI boundary |
| 7 | `NYTPROF_ERR_ABI` | Reserved for future ABI negotiation failures |
| 8 | `NYTPROF_ERR_INVALID_HANDLE` | Reserved |

## Panic / ownership

- Every `extern "C"` entry point wraps work in `catch_unwind`; panics become `NYTPROF_ERR_PANIC` and never unwind into C.
- Handles are owned `Box`es; only `open` allocates, only `close` frees.
- No interior pointers into Rust-owned strings are returned except `nytprof_last_error` (thread-local, invalidated by next error-setting call).

## Semantic golden checks (cargo tests)

Real fixture paths via `ProfileModel` / C ABI (no stub counts):

| Fixture | Check | Expected |
|---------|-------|----------|
| `fixtures/v5/default-calls1` | `main::leaf` returns | **15** |
| `fixtures/v5/default-calls1` | `main::mid` returns | **3** |
| `fixtures/v5/default-calls1` | mid→leaf edge count | **15** |
| `fixtures/v5/default-calls1` | `discount_events` | **818** |
| `fixtures/v5/default-calls1` | `sub_entry_events` | **0** |
| `fixtures/v5/default-calls1` | model `total_events` (decoded tags) | **2473** |
| `fixtures/v5/calls2-default` | `sub_entry_events` | **27** |
| `fixtures/v5/blocks-calls1` | line `(1,5)` calls / block `(1,4)` calls | **780** / **810** |
| incomplete 500-byte prefix | default open | **≠ OK** (`IO_DECODE` or `INCOMPLETE`) |

Evidence: `cargo test -p nytprof-ffi`.

## Build

```sh
cargo build -p nytprof-ffi
cargo test -p nytprof-ffi
# artifacts: target/debug/libnytprof_ffi.so (Linux) / .dylib (macOS) / .dll (Windows)
```

Header is **not** auto-installed; consumers include `crates/nytprof-ffi/include/nytprof_ffi.h` (or a packaging copy).

## Relationship to full R1

| Claim | Status after PR-A05 |
|-------|---------------------|
| FFI product path exists (open/query/close) | **yes** (this MVP) |
| Residual “no production C ABI” row | **partial close** — MVP shipped; full RUST-010 residual remains (batch, install, BUILD-007, sanitizer package) |
| OQ-2 waiver | **forbidden** — this PR implements close path; does not waive |
| XS Data / ReadStream | still residual (**PR-A06**) |
| Preview dual-path without dylib | unchanged — CLI + pure-Perl JsonlData remain |

## Board placement

| ID | Status | Evidence |
|----|--------|----------|
| `FFI-CDYLIB-MVP` | **done** (MVP) | this schema + `crates/nytprof-ffi` + header + `cargo test -p nytprof-ffi` |
| `RUST-010` (full) | **partial** | remaining: batch APIs, sanitizer/Miri package, BUILD-007, production library install |

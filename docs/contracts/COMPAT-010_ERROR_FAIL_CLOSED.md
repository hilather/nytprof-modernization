# COMPAT-010 — Error / corruption fail-closed policy (provisional)

**Status:** provisional — **not** a full COMPAT-010 freeze  
**Task:** COMPAT-010 (slice of full plan task)  
**Board ID:** `COMPAT-010-ERR`  
**Date:** 2026-08-07  
**Depends on:** VERIFY-CLI, DECODE-ROBUST, RUST-001/004 (v5 decode)  
**Related:**  
- Verify CLI MVP: [`docs/schemas/verify-cli-mvp-v0.md`](../schemas/verify-cli-mvp-v0.md)  
- Decode robustness (format-v5): empty / bad header / truncate / unsupported tag → `Err`  
- Full plan task: [`docs/plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](../plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) (COMPAT-010)  
- Security / recovery context: [`docs/plan/13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](../plan/13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md)

---

## Scope and non-claims

This document freezes a **provisional fail-closed policy** for **corrupt or unreadable v5 profile input** on the **shipped native paths** of the first slice:

| Path | Entry point | Model load / decode |
|------|-------------|---------------------|
| `nytprof-cli verify` / `inspect` | `nytprof_report::verify_profile` | `ProfileModel::from_path` → v5 decode |
| `nytprof-cli dump` | `decode_path` (format-v5) | event stream only |
| `nytprof-cli report` / `summary` | `ProfileModel::from_path` | same as verify |

It does **not**:

- freeze the full COMPAT-010 error taxonomy (capability fallback vs resource limits vs I/O classes vs internal errors);
- freeze salvage / partial recovery semantics for truncated v5 or v6 (SEC-003 / recovery chapters remain open);
- freeze legacy-engine or Perl ReadStream failure wording / exit codes beyond “non-zero / Err”;
- require identical error *messages* across dump vs report vs verify (only fail-closed behavior);
- define automatic engine fallback when native fails (fallback that hides corruption is forbidden; see binding principles).

Full plan COMPAT-010 remains **proposed / in-progress** until a complete taxonomy and decision table land.

---

## Binding principles (provisional)

1. **Fail closed on corruption.** Empty, truncated, bad-magic, or otherwise malformed profile bytes must **not** produce a successful decode, model, verify summary, dump stream, or report as if the input were valid.  
2. **No panic on bad input.** Shipped library and CLI paths return `Result::Err` / non-zero exit; they must not abort via panic for the covered corrupt inputs.  
3. **No silent success.** Exit status 0 / `Ok` is reserved for successfully decoded (and, for report/verify, modeled) input.  
4. **No automatic fallback that hides failure.** Selecting `legacy` / another engine must not be used to paper over native decode/model failure of corrupt bytes without an explicit user action and clear diagnostics (full fallback matrix still open).  
5. **Diagnostics are best-effort.** stderr / `Display` of the error should mention the failure class when cheap (format, unexpected EOF, unsupported tag, I/O); exact strings are not frozen here.

---

## Covered input classes (must Err / non-zero)

These classes are **required fail-closed** for the shipped native paths above.

| Class | Example | Expected behavior |
|-------|---------|-------------------|
| **Empty file** | 0-byte tempfile | `Err` / exit ≠ 0; no `OK:` summary; no empty “success” dump/report |
| **Truncated mid-file** | first half of `fixtures/v5/default-calls1/nytprof.out` | `Err` / exit ≠ 0 (UnexpectedEof / Format / Zlib / Decode — any error is fine) |
| **Bad header / magic** | `"NOTPROF 5 0\n"` or non-header garbage | `Err` / exit ≠ 0 |
| **Malformed after header** | valid `NYTProf 5 0\n` + unsupported tag / garbage body | `Err` / exit ≠ 0 (DECODE-ROBUST) |
| **Incomplete stream (record-aligned prefix)** | first ~500 bytes of default-calls1 (header + ATTR/OPTION; no timing / missing `PID_END`) | **verify / report:** `Err` / exit ≠ 0 by default (see incomplete-stream contract). **dump** may still emit decoded events (lenient salvage surface). |

Incomplete / short-prefix completeness rules, PID balance, timing requirement, and opt-in salvage (`NYTPROF_ALLOW_INCOMPLETE=1`) are defined in:

**[`docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md`](COMPAT-010_INCOMPLETE_STREAM.md)** (board `INCOMPLETE-STREAM`).

Related lower-layer evidence (must remain green):

| Layer | Evidence |
|-------|----------|
| format-v5 | `decode_empty_input_errors`, `decode_bad_header_errors`, `decode_truncated_after_header_errors`, `decode_truncated_mid_file_errors`, `decode_garbage_tag_after_header_errors`; DECODE-FUZZ-MVP `decode_fuzz_no_panic_*` / `fuzz_truncated_mutations` |
| model | `from_path_truncated_profile_errors` |
| report / verify | `verify_profile_*_err` suite (empty / truncated / bad magic) + report-path model load on same inputs; DECODE-FUZZ-MVP `decode_fuzz_no_panic_verify_*` / `fuzz_truncated_mutations_verify` |
| CLI (process) | fail-closed tests / `tools/oracle/selftest_fail_closed.sh` for `verify` (and dump/report when exercised) |
| Fuzz MVP smoke | `tools/oracle/selftest_decode_fuzz.sh` (schema [decode-fuzz-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/decode-fuzz-mvp-v0.md)) |

---

## Exit / result contract (shipped native CLI)

| Outcome | CLI | Library |
|---------|-----|---------|
| Valid profile | exit **0**; stdout has content (`OK:` for verify; JSONL for dump; text for report) | `Ok(...)` |
| Corrupt / unreadable (classes above) | exit **≠ 0** (native path currently **1**); error on stderr (`nytprof-cli: …`) | `Err(...)` |
| Panic | **forbidden** for covered corrupt inputs | **forbidden** |

Notes:

- Usage / flag errors also exit non-zero; they are not corruption but share the fail-closed exit style.  
- Engine `legacy` currently exits **2** (not wired); that is orthogonal to corruption handling.  
- A truncated file that happens to end on a valid record boundary is still **out of policy as success** if the file is a known partial copy of a longer golden — the decoder may report UnexpectedEof only when mid-record; half-of-default-calls1 is known to fail today and is the regression fixture.

---

## Explicit non-goals (this provisional slice)

| Topic | Status |
|-------|--------|
| Full error taxonomy (capability vs codec vs resource vs internal) | deferred to full COMPAT-010 |
| v6 framing / checksum salvage | SEC-003 / later |
| Soft “best effort” partial report on truncated v5 | **not** allowed as silent success; any future salvage must be **opt-in and labeled incomplete** |
| Continuous fuzz program | SEC-002 (lightweight DECODE-FUZZ-MVP battery is in-tree; see decode-fuzz-mvp-v0.md) |
| Identical message text across tools | not frozen |

---

## Tests and how to run

```bash
# Library: verify + report-path model load on corrupt inputs
cargo test -p nytprof-report fail_closed -- --nocapture
cargo test -p nytprof-report verify_profile -- --nocapture

# Incomplete stream (record-aligned short prefix) — see COMPAT-010_INCOMPLETE_STREAM.md
cargo test -p nytprof-report verify_profile_incomplete default_calls1_model_is_stream
cargo test -p nytprof-model stream_completeness incomplete_prefix

# Lower layers (must stay green)
cargo test -p nytprof-format-v5 decode_empty decode_bad_header decode_truncated
cargo test -p nytprof-model from_path_truncated

# DECODE-FUZZ-MVP (no panic on truncate/mutate; see decode-fuzz-mvp-v0.md)
# Note: cargo ANDs multiple name filters — run separately (or use selftest_decode_fuzz.sh)
cargo test -p nytprof-format-v5 decode_fuzz_no_panic
cargo test -p nytprof-format-v5 fuzz_truncated_mutations
cargo test -p nytprof-report decode_fuzz_no_panic
cargo test -p nytprof-report fuzz_truncated_mutations_verify

# CLI process (when present)
cargo test -p nytprof-cli fail_closed incomplete_stream

# Optional shell smoke (cargo or prefix binary)
bash tools/oracle/selftest_fail_closed.sh
bash tools/oracle/selftest_incomplete_stream.sh
bash tools/oracle/selftest_decode_fuzz.sh
```

---

## Open items (toward full COMPAT-010)

| ID | Item |
|----|------|
| OI-C010-01 | Full error taxonomy and fallback decision table (capability vs corruption vs I/O) |
| OI-C010-02 | ~~Whether dump of a *complete* prefix of records is allowed~~ — **resolved provisionally** in [`COMPAT-010_INCOMPLETE_STREAM.md`](COMPAT-010_INCOMPLETE_STREAM.md): dump lenient; verify/report fail closed; salvage via `NYTPROF_ALLOW_INCOMPLETE=1` |
| OI-C010-03 | Resource-limit errors vs corruption (SEC-001) sharing exit codes |
| OI-C010-04 | Legacy Perl ReadStream / nytprofhtml behavior matrix for the same corrupt fixtures |
| OI-C010-05 | Explicit salvage CLI surface (if any) must not be the default verify/dump/report path — env salvage only for this slice |

---

## Change control

This is a **provisional** first-slice contract. Behavior changes that make corrupt input succeed silently on `verify` / `dump` / `report` require an explicit board/ADR update and test changes. Expanding the taxonomy into full COMPAT-010 acceptance is a separate deliverable.

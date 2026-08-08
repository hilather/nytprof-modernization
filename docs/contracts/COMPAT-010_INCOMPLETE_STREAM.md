# COMPAT-010 — Incomplete stream fail-closed (provisional)

**Status:** provisional — **not** a full COMPAT-010 / SEC recovery freeze  
**Board ID:** `INCOMPLETE-STREAM`  
**Date:** 2026-08-07  
**Depends on:** COMPAT-010-ERR, VERIFY-CLI, RUST-006 (ProfileModel)  
**Related:**  
- Parent fail-closed policy: [`docs/contracts/COMPAT-010_ERROR_FAIL_CLOSED.md`](COMPAT-010_ERROR_FAIL_CLOSED.md)  
- Verify CLI MVP: [`docs/schemas/verify-cli-mvp-v0.md`](../schemas/verify-cli-mvp-v0.md)  
- Security / recovery (later): [`docs/plan/13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](../plan/13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md)

---

## Problem

The v5 decoder can succeed on **record-aligned short prefixes** of a real profile (e.g. the first ~500 bytes of `fixtures/v5/default-calls1/nytprof.out`): a valid header plus `ATTRIBUTE` / `OPTION` records, **no statement timing** (`TIME_LINE` / `TIME_BLOCK`), and often **no `PID_END`** after `PID_START`. Without an extra completeness check, `verify` would print `OK` with `TIME_LINE: 0` — silent success on an incomplete stream.

This is distinct from mid-record truncation (decoder `Err` / UnexpectedEof), which COMPAT-010-ERR already covers.

---

## Incomplete stream classes

| Class | Description | Typical signal on `ProfileModel` |
|-------|-------------|----------------------------------|
| **Missing process end** | One or more `PID_START` without a matching `PID_END` | `pid_start_events > 0` and `pid_end_events < pid_start_events` |
| **No statement timing** | Stream never carries statement samples (header / options only, or truncated before timing) | `time_line_events + time_block_events == 0` |

Both classes are **incomplete for default verify/report success**. A stream may match one or both.

---

## Completeness rules (testable)

A model is **stream-complete** iff **all** of:

1. **PID balance:** if `pid_start_events > 0`, then `pid_end_events >= pid_start_events`.  
2. **Statement timing:** `time_line_events + time_block_events > 0`.

API (Rust model):

- `ProfileModel::pid_start_events` / `pid_end_events` (counted in `accumulate`)
- `ProfileModel::stream_incompleteness_reasons() -> Vec<&'static str>`
- `ProfileModel::is_stream_complete() -> bool`

API (pure-Perl dump JSONL; same rules — board **PERL-STREAM-COMPLETE**):

- `Devel::NYTProf::JsonlData` → `time_line_events` / `time_block_events` / `pid_start_events` / `pid_end_events`
- `is_stream_complete()` / `stream_incompleteness_reasons()` (arrayref of reason strings)
- Schema: [`docs/schemas/perl-jsonl-data-mvp-v0.md`](../schemas/perl-jsonl-data-mvp-v0.md)

---

## Shipped path policy

| Path | Default | Salvage |
|------|---------|---------|
| **`verify` / `inspect`** (`nytprof_report::verify_profile`) | **Fail closed:** incomplete → `Err` / CLI exit ≠ 0; no `OK:` line | `NYTPROF_ALLOW_INCOMPLETE=1` → success with clear `INCOMPLETE:` header + note |
| **`report` / `summary` / `csv` / `html` / `folded` / `callgrind`** | **Fail closed:** after model load, `require_complete_stream` → `Err` / exit ≠ 0 | Same env allows the report/export to proceed (best-effort salvage) |
| **`report --json` / `aggregates` / `agg`** (**JSON-REPORT-INCOMPLETE-FAILCLOSED**) | **Fail closed:** same `load_model_for_report` / `require_complete_stream` path as text report → exit ≠ 0; must **not** emit a successful complete `{"ok":true,"is_stream_complete":true,...}` object | Same env may allow JSON emit; must not claim `is_stream_complete:true` on an incomplete stream |
| **`dump`** | **Lenient:** emit whatever the decoder produced (JSONL of decoded events). Incomplete prefixes that still decode cleanly may dump successfully | N/A — dump is the salvage surface for tooling |

Notes:

- Decode / `ProfileModel::from_path` **may succeed** on incomplete streams; fail-closed is enforced at verify/report, not by rejecting model load.
- Mid-record / bad-magic / empty inputs remain fail-closed at decode (COMPAT-010-ERR).
- Exact error strings are not frozen; they should mention incompleteness when cheap.
- This is **not** full SEC-003 recovery / partial-report UX freeze.

---

## Environment salvage

| Variable | Value | Effect |
|----------|-------|--------|
| `NYTPROF_ALLOW_INCOMPLETE` | `1` | Opt-in: incomplete streams are accepted on verify/report. Verify prints `INCOMPLETE:` (not bare `OK:`) plus a note listing reasons. Other values / unset → fail closed. |

No CLI flag is required for this provisional slice; env is enough for salvage tooling.

---

## Explicit non-goals

| Topic | Status |
|-------|--------|
| Full COMPAT-010 error taxonomy | deferred |
| Automatic salvage as default | **forbidden** |
| Dump fail-closed on incomplete prefixes | not required; dump stays lenient |
| v6 checksum / chunk recovery | SEC / later |
| Identical wording to legacy Perl tools | not frozen |

---

## Tests and how to run

```bash
# Model completeness rules + prefix
cargo test -p nytprof-model stream_completeness incomplete_prefix default_calls1_pid -- --nocapture

# Library verify / report completeness
cargo test -p nytprof-report verify_profile_incomplete default_calls1_model_is_stream -- --nocapture
cargo test -p nytprof-report verify_profile_default_calls1_ok -- --nocapture

# CLI process
cargo test -p nytprof-cli incomplete_stream verify_cli_default -- --nocapture
cargo test -p nytprof-cli incomplete_stream_report_json report_json_incomplete -- --nocapture

# Shell smoke (verify/report fail on 500-byte prefix; golden OK; optional salvage)
bash tools/oracle/selftest_incomplete_stream.sh

# Packaging: report --json / aggregates fail-closed on incomplete prefix (native CLI)
./scripts/packaging/json_report_incomplete_smoke.sh
```

---

## Change control

Provisional first-slice contract. Making incomplete streams succeed silently on default `verify` / `report` requires a board/ADR update and test changes. Full recovery UX is a separate deliverable.

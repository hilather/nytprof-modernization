# R1 residual readiness matrix (provisional) — v0

**Status:** provisional readiness snapshot for **offline R0 / R1-preview**, **R2-preview opt-in**, and **R2-stable** (PR-C05 honesty cut); residual work for full R1 / R3 / R4
**Board ID:** `R1-RESIDUAL-MATRIX` (honesty: `R1-HONESTY-SYNC`; R2-preview: `R2-PREVIEW-READINESS-CUT`; R2-stable: `R2-STABLE-READINESS-CUT`; P1/P2: `R2-P1P2-METHODOLOGY`)
| `SEC-FUZZ-HARDENING-MVP` | **done** (package MVP) | PR-C03 security/fuzz package: contract [`SECURITY_FUZZ_HARDENING_PACKAGE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/SECURITY_FUZZ_HARDENING_PACKAGE_v0.md); schema [`security-fuzz-hardening-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/security-fuzz-hardening-mvp-v0.md); v6 `tests/decode_fuzz.rs`; smoke `tools/oracle/selftest_security_fuzz.sh` (cargo-required). P02 wrapper honest `SKIP:` without cargo. **Not** an offline_gate step. **Not** full SEC-002 cargo-fuzz/AFL; COL-015 residual. |
| Full continuous fuzz / SEC-012 independent sign-off | **SEC-002** full + independent **SEC-012** remain residual; P02 landed **checklist / job MVP** only | [`SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md) + [`sec002_continuous_fuzz_mvp.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/sec002_continuous_fuzz_mvp.sh) wrap existing `decode_fuzz` — **not** independent sign-off, **not** cargo-fuzz/AFL/deep corpus, **not** GA marketing | Checklist/job MVP ready; independent review + full continuous fuzz still residual |
**Date:** 2026-08-07 (R2-preview amendment 2026-08-12)
**Status:** provisional readiness snapshot for **offline R0 / R1-preview** + **full R1 MVP product cut** (PR-A10) vs residual beyond-MVP / OUT-OF-R1 work  
**Board ID:** `R1-RESIDUAL-MATRIX` (honesty sync: `R1-HONESTY-SYNC`; full cut: `R1-FULL-READINESS-CUT`)  
**Date:** 2026-08-11  
**Depends on:** REPORT-CONTRACT-FREEZE, CI-OFFLINE-GATE, CAPABILITY-SELFTEST, ENGINE-AUTO-SMOKE, PERL-* JSONL / engine rows (incl. SUB_ENTRY multiplicity), NATIVE-AGG-JSON, **JSON-NATIVE-STREAM-MVP**, **JSON-TIME-BLOCK-MVP**, **JSON-REPORT-INCOMPLETE-FAILCLOSED**, **JSON-SUBDEF-SOURCE-MVP**, **JSON-META-FILES-MVP**, **JSON-EVENT-COUNTS-MVP**, **JSON-FILE-BASENAME-MVP**, **JSON-TOTAL-EVENTS-MVP**, **JSON-ATTR-BASETIME-MVP**, NATIVE-QUERY-JSON-CROSS / **NATIVE-QUERY-JSON-CROSS-EXPAND** / **NATIVE-QUERY-JSON-CROSS-BLOCKS** / **NATIVE-QUERY-JSON-CROSS-META** / **NATIVE-QUERY-JSON-CROSS-TIMEBLOCK** / **NATIVE-QUERY-JSON-CROSS-COUNTS** / **NATIVE-QUERY-JSON-CROSS-TOTAL**, JSON-SUB-ENTRY-MVP, JSON-BLOCKS-MVP, QUERY-JSON-*, BUILD-DUAL-PATH / BUILD-MAKEMAKER-OPT, DUMP-PARITY-EXPAND, DECODE-FUZZ-MVP, INCOMPLETE-STREAM, **FMT-V6-HEADER-PROVISIONAL** / **FMT-V6-HEADER-PARSE-MVP** / **FMT-V6-CHUNK-PROVISIONAL** / **FMT-V6-CHUNK-PARSE-MVP** / **FMT-V6-VARINT-PROVISIONAL** / **FMT-V6-VARINT-MVP** / **FMT-V6-SVARINT-PROVISIONAL** / **FMT-V6-SVARINT-MVP** / **FMT-V6-STRING-PROVISIONAL** / **FMT-V6-STRING-MVP** / **FMT-V6-TLV-PROVISIONAL** / **FMT-V6-TLV-MVP** / **FMT-V6-TLV-REGION-PROVISIONAL** / **FMT-V6-TLV-REGION-MVP** / **FMT-V6-FILE-PREFIX-PROVISIONAL** / **FMT-V6-FILE-PREFIX-MVP** / **FMT-V6-PREFIX-CHUNK-STREAM-PROVISIONAL** / **FMT-V6-PREFIX-CHUNK-STREAM-MVP** / **FMT-V6-EVENT-BODY-PROVISIONAL** / **FMT-V6-EVENT-BODY-MVP** / **FMT-V6-MINI-PROFILE-PROVISIONAL** / **FMT-V6-MINI-PROFILE-MVP** / **FMT-V6-MULTI-CHUNK-EVENT-PROVISIONAL** / **FMT-V6-MULTI-CHUNK-EVENT-MVP** / **FMT-V6-SOURCE-BODY-PROVISIONAL** / **FMT-V6-SOURCE-BODY-MVP** / **FMT-V6-INDEX-BODY-PROVISIONAL** / **FMT-V6-INDEX-BODY-MVP** / **FMT-V6-SUMMARY-BODY-PROVISIONAL** / **FMT-V6-SUMMARY-BODY-MVP** / **FMT-V6-FOOTER-BODY-PROVISIONAL** / **FMT-V6-FOOTER-BODY-MVP** / **FMT-V6-CRC-PROVISIONAL** / **FMT-V6-CRC-MVP** / **FMT-V6-PAYLOAD-ZLIB-PROVISIONAL** / **FMT-V6-PAYLOAD-ZLIB-MVP** / **FMT-V6-PAYLOAD-ZSTD-PROVISIONAL** / **FMT-V6-PAYLOAD-ZSTD-MVP** / **FMT-V6-PAYLOAD-LZ4-PROVISIONAL** / **FMT-V6-PAYLOAD-LZ4-MVP** / **FMT-V6-COMPRESSED-PROFILE-PROVISIONAL** / **FMT-V6-COMPRESSED-PROFILE-MVP** / **FMT-V6-MULTI-CHUNK-COMPRESSED-PROVISIONAL** / **FMT-V6-MULTI-CHUNK-COMPRESSED-MVP** / **FMT-V6-COMPRESSED-MIXED-PROVISIONAL** / **FMT-V6-COMPRESSED-MIXED-MVP** / **FMT-V6-PER-KIND-CODEC-PROVISIONAL** / **FMT-V6-PER-KIND-CODEC-MVP** / **FMT-V6-MULTI-CHUNK-KIND-PROVISIONAL** / **FMT-V6-MULTI-CHUNK-KIND-MVP** / **FMT-V6-MULTI-CHUNK-SOURCE-PROVISIONAL** / **FMT-V6-MULTI-CHUNK-SOURCE-MVP** / **FMT-V6-MULTI-CHUNK-INDEX-PROVISIONAL** / **FMT-V6-MULTI-CHUNK-INDEX-MVP** / **FMT-V6-MULTI-CHUNK-SUMMARY-PROVISIONAL** / **FMT-V6-MULTI-CHUNK-SUMMARY-MVP** / **FMT-V6-MID-RECORD-SPAN-PROVISIONAL** / **FMT-V6-MID-RECORD-SPAN-MVP** / **FMT-V6-MID-RECORD-SOURCE-PROVISIONAL** / **FMT-V6-MID-RECORD-SOURCE-MVP** / **FMT-V6-MID-RECORD-INDEX-PROVISIONAL** / **FMT-V6-MID-RECORD-INDEX-MVP** / **FMT-V6-MID-RECORD-SUMMARY-PROVISIONAL** / **FMT-V6-MID-RECORD-SUMMARY-MVP** / **FMT-V6-DECODED-CHUNK-PROVISIONAL** / **FMT-V6-DECODED-CHUNK-MVP** / **FMT-V6-DECODED-STREAM-PROVISIONAL** / **FMT-V6-DECODED-STREAM-MVP** / **FMT-V6-DECODED-EVENT-PROVISIONAL** / **FMT-V6-DECODED-EVENT-MVP** / **FMT-V6-DECODED-SOURCE-PROVISIONAL** / **FMT-V6-DECODED-SOURCE-MVP** / **FMT-V6-DECODED-INDEX-PROVISIONAL** / **FMT-V6-DECODED-INDEX-MVP** / **FMT-V6-DECODED-SUMMARY-PROVISIONAL** / **FMT-V6-DECODED-SUMMARY-MVP** / **FMT-V6-DECODED-MIXED-PROVISIONAL** / **FMT-V6-DECODED-MIXED-MVP** / **FMT-V6-DECODED-MIXED-MULTI-CHUNK-PROVISIONAL** / **FMT-V6-DECODED-MIXED-MULTI-CHUNK-MVP** / **FMT-V6-DECODED-MIXED-MID-RECORD-PROVISIONAL** / **FMT-V6-DECODED-MIXED-MID-RECORD-MVP** / **FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-PROVISIONAL** / **FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-MVP** (COL-007 runway preflight only), and related parity gates below  
**Gate:** offline R0/R1-preview **before** full R1; COL-007 product E3-EVENT **done** (PR-B09); wire freeze **done** (PR-B11 / ADR-0006); COL-009 C baseline **reaffirmed** (PR-B13 / ADR-0007); R2-preview **opt-in** packaging honesty (PR-B13); E3-mixed / COL-008 / convert/merge **done** (PR-C01/C02 residual for lossy/packing only); R2-stable cut **PR-C05** (tools/security/perf honesty); R3/R4 **policy** when present (flips **not executed**); R5 **governance** (PR-F01 / ADR-0009; **no component retired**); **not** R3/R4 **runtime** defaults

---

## Scope and non-claims

This matrix freezes what the first-slice program **advertises as ready** for offline developer preview (charter **R0**) and an **opt-in native v5 read/report R1-preview**, versus what remains **explicit residual** before a full charter **R1** product claim. It also records **R2-preview** (v6 opt-in) honesty after Track B.

It is **not**:

- a release certification or CPAN readiness statement;
- a performance certification (see residual row; light bench only);
- a CLI v6 **collection** default or R2-stable claim (wire **IDs** are frozen by ADR-0006; writer backend is C by ADR-0007; E3-mixed / COL-008 / convert/merge **done** (PR-C01/C02 residual for lossy/packing only));
This matrix freezes what the first-slice program **advertises as ready** for offline developer preview (charter **R0**) and an **opt-in native v5 read/report R1-preview**, plus the **full R1 MVP product cut** (PR-A10 / ADR-0003) after Phase A closes, versus what remains **explicit residual** (beyond-MVP completeness, waived classes, OUT-OF-R1).

It is **not**:

- a CPAN upload or full BUILD-003 XS dual-build certification;
- a public performance certification (A09 default = **WAIVE** public claims; light bench only);
- a v6 wire freeze, CLI v6 default, or COL-007 C writer claim;
- a full oracle `nytprofhtml` DOM claim;
- full RUST-010 beyond open/query/close MVP, or full PERL-004/005 (COMPAT-007 / pure-XS);
- permission to flip defaults (`engine=auto` product policy, format defaults — charter R3/R4).

**Operator runbook (offline R0 / R1-preview stack):**  
[`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) (`R1-PREVIEW-RUNBOOK`)

**Primary offline operator gate:** [`scripts/ci/offline_gate.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/offline_gate.sh) (`make offline-gate`).

**Report surface freeze (advertised CLI report outputs + semantic counts):**  
[`docs/contracts/REPORT_SURFACE_CONTRACT_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_SURFACE_CONTRACT_v0.md)

**HTML artifact residual inventory (oracle `nytprofhtml` vs native paths):**  
[`docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md)

Related program docs:

| Doc | Path |
|-----|------|
| Operator runbook (R0 / R1-preview) | [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) |
| Program charter (R0–R5) | [`docs/PROGRAM_CHARTER.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md) |
| Dual-path packaging policy | [`docs/BUILD_SUPPORT_POLICY.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) |
| First-slice board | [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md) |
| Dual-equality readiness | [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md) |
| R2-preview release notes | [`docs/RELEASE_NOTES_R2_PREVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_PREVIEW.md) |
| COL-009 C baseline ADR | [`docs/adrs/0007-production-v6-writer-backend-c-baseline.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0007-production-v6-writer-backend-c-baseline.md) |
| Feature-to-test inventory | [`baseline/inventories/feature-to-test-matrix.md`](https://github.com/hilather/nytprof-modernization/blob/main/baseline/inventories/feature-to-test-matrix.md) |
| Benchmark notes (P1/P2 methodology; not cert) | [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md) |

### Frozen semantic counts (shared by advertised surfaces)

| Fixture | Check | Expected |
|---------|-------|----------|
| `fixtures/v5/default-calls1` | `main::leaf` returns | **15** |
| `fixtures/v5/default-calls1` | `main::mid` returns | **3** |
| `fixtures/v5/default-calls1` | `main::mid` → `main::leaf` edge | **15** |
| `fixtures/v5/default-calls1` | `discount_events` (A3 multiplicity) | **818** |
| `fixtures/v5/default-calls1` | `sub_entry_events` / `sub_entry_count` (`calls=1`) | **0** |
| `fixtures/v5/calls2-default` | `sub_entry_events` / `sub_entry_count` (`calls=2`) | **27** |
| `fixtures/v5/blocks-calls1` | `line_total(1,5).calls` (TIME_BLOCK) | **780** |
| `fixtures/v5/blocks-calls1` | JSON `line_calls_1_5` / `block_line_calls_1_4` (**JSON-BLOCKS-MVP**) | **780** / **810** |

Counts are exact; tick/time strings only under COMPAT-003 (not frozen as identity).

---

## Advertised ready — offline R0 / R1-preview

### 1. Native CLI surfaces

Disposition for each surface is frozen in the report surface contract. Paths are repo-relative.

| Surface | Entry | Code / package path | Contract / schema | Notes |
|---------|-------|---------------------|-------------------|-------|
| dump | `nytprof-cli dump` | `crates/nytprof-cli/`, `crates/nytprof-format-v5/` | [`docs/contracts/REPORT_SURFACE_CONTRACT_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_SURFACE_CONTRACT_v0.md), [`docs/schemas/native-dump-parity-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-dump-parity-mvp-v0.md), [`docs/schemas/canonical-event-dump-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/canonical-event-dump-v0.md) | Canonical JSONL decode spine |
| verify / inspect | `nytprof-cli verify` / `inspect` | `crates/nytprof-report/`, `crates/nytprof-cli/` | [`docs/schemas/verify-cli-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/verify-cli-mvp-v0.md), incomplete-stream contract below | Fail-closed health check |
| report / summary | `nytprof-cli report` / `summary` | `crates/nytprof-report/` | REPORT_SURFACE + [`docs/schemas/report-semantic-parity-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/report-semantic-parity-mvp-v0.md) | Text summary from compact model |
| aggregates JSON | `nytprof-cli report --json` / `aggregates` / `agg` | `crates/nytprof-cli/` | REPORT_SURFACE + [`docs/schemas/native-aggregates-json-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-aggregates-json-mvp-v0.md) | Structured JSON aggregates from ProfileModel (15/3/15 default-calls1) + `sub_entry_events` (**JSON-SUB-ENTRY-MVP**: default **0** / calls2 **27**) + greppable A4/A4b `line_calls_1_5` / `block_line_calls_1_4` (**JSON-BLOCKS-MVP**: blocks-calls1 **780** / **810**) + greppable A9/A8 samples `sub_def_leaf` / `sub_def_mid` / `source_line_1_5` (**JSON-SUBDEF-SOURCE-MVP**: leaf **1/3–7**, mid **1/8–12**, hot-loop text) + greppable ATTRIBUTE/OPTION/NEW_FID samples `attribute_ticks_per_sec` / `option_calls` / `file_1` (**JSON-META-FILES-MVP**: default-calls1 ticks **10000000**, option calls **1**, path contains **workload.pl**; null when absent) + stream/PID (**JSON-NATIVE-STREAM-MVP**: `is_stream_complete` / `incompleteness_reasons` / `time_line_events` / `pid_start_events` / `pid_end_events`) + A2 `time_block_events` (**JSON-TIME-BLOCK-MVP**: default **0** / blocks **916**); fail-closed incomplete streams (**JSON-REPORT-INCOMPLETE-FAILCLOSED**) + **`total_events` 2474** (**JSON-TOTAL-EVENTS-MVP**) + **`attribute_basetime`** (**JSON-ATTR-BASETIME-MVP**); smoke `scripts/packaging/native_agg_json_smoke.sh` + `json_native_stream_smoke.sh` + `json_time_block_smoke.sh` + `json_report_incomplete_smoke.sh` + `json_subdef_source_smoke.sh`; cargo `crates/nytprof-cli/tests/native_agg_json.rs` + incomplete CLI tests; cross vs Perl query JSON: `scripts/packaging/native_query_json_cross_smoke.sh` (**NATIVE-QUERY-JSON-CROSS** / **CROSS-EXPAND** / **CROSS-BLOCKS** / **CROSS-META** / **CROSS-TIMEBLOCK** / **CROSS-TOTAL**) |
| html (± `-o` / `--out-dir`) | `nytprof-cli html` | `crates/nytprof-report/` | REPORT_SURFACE + [`docs/schemas/html-report-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-report-mvp-v0.md), [`html-multifile-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md), [`html-per-file-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-per-file-mvp-v0.md), [`html-outdir-safety-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-outdir-safety-mvp-v0.md) | Single-file + multi-file site; atomic publish + out-dir safety |
| csv | `nytprof-cli csv` | `crates/nytprof-report/` | REPORT_SURFACE + [`docs/schemas/csv-semantic-parity-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/csv-semantic-parity-mvp-v0.md) | Dual-section subs + call_edges |
| folded | `nytprof-cli folded` | `crates/nytprof-report/` | REPORT_SURFACE + [`docs/schemas/export-formats-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/export-formats-mvp-v0.md), [`export-semantic-parity-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/export-semantic-parity-mvp-v0.md) | Folded stacks from call edges |
| callgrind / cg | `nytprof-cli callgrind` / `cg` | `crates/nytprof-report/` | same export schemas | Callgrind-style text (not full `nytprofcg` byte identity) |
| capability / selftest | `nytprof-cli capability` / `selftest` / `capabilities` (+ `--json` / `--format=json`) | `crates/nytprof-cli/` | [`docs/schemas/capability-selftest-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/capability-selftest-mvp-v0.md) | Offline capability probe; human greppable default + JSON object (`ok`/`decode`/`report`/`verify`/`profile_ok`) |
| engine select | `--engine` / `NYTPROF_ENGINE` | `crates/nytprof-cli/src/engine.rs` + Perl `EngineDispatch` | [`docs/schemas/engine-selection-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/engine-selection-mvp-v0.md) | Default `native`; Perl facade: `auto` prefer-native / fall-back-legacy; Rust CLI residual: `auto` → `native` |

Install aliases: `scripts/packaging/install_native.sh` → `prefix/bin/nytprof-cli` (+ `nytprof-dump`); schema [`docs/schemas/native-install-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-install-mvp-v0.md).

### 2. Parity / safety gates (advertised offline)

| Gate | Evidence path | What it freezes |
|------|---------------|-----------------|
| Dump parity (multi-fixture) | `tools/oracle/selftest_native_dump_parity.sh`, `tools/oracle/selftest_native_dump_parity_all.sh` (default-calls1 + calls2-default + blocks-calls1); optional cargo tag-count tests in `crates/nytprof-format-v5` | Structural JSONL equality vs golden after normalize |
| Report semantic parity | `tools/oracle/report_semantic_parity.sh`; cargo `report_semantic_parity_default_calls1` | leaf **15** / mid **3** / mid→leaf **15** on HTML paths |
| Blocks semantic parity | `tools/oracle/blocks_semantic_parity.sh`; cargo `blocks_semantic_parity_blocks_calls1` | line5 calls **780** + leaf/mid on blocks-calls1 |
| CSV semantic parity | `tools/oracle/csv_semantic_parity.sh` | dual-section CSV leaf/mid/edge |
| Export semantic parity | `tools/oracle/export_semantic_parity.sh` | folded + callgrind leaf/mid/edge |
| Incomplete stream | `tools/oracle/selftest_incomplete_stream.sh`; contract [`docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md) | verify/report fail closed; no silent OK |
| JSON report incomplete fail-closed | `scripts/packaging/json_report_incomplete_smoke.sh` (**JSON-REPORT-INCOMPLETE-FAILCLOSED**); cargo `incomplete_stream_report_json_*` / `report_json_incomplete_prefix_*` | `report --json` / `aggregates` exit ≠ 0 on record-aligned incomplete prefix; no complete `ok:true` + `is_stream_complete:true` |
| Fail-closed corrupt input | `tools/oracle/selftest_fail_closed.sh`; [`docs/contracts/COMPAT-010_ERROR_FAIL_CLOSED.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-010_ERROR_FAIL_CLOSED.md) | empty / bad magic / truncated → Err |
| Decode fuzz (deterministic) | `tools/oracle/selftest_decode_fuzz.sh`; `crates/nytprof-format-v5/tests/decode_fuzz.rs`, `crates/nytprof-report/tests/decode_fuzz.rs` | decode+verify never panic |
| Capability selftest (+ JSON) | `scripts/packaging/capability_selftest_smoke.sh` | claimed native tools respond / verify; `--json` fields `ok`/`decode`/`report`/`verify` true + `profile_ok` |
| Native aggregates JSON | `scripts/packaging/native_agg_json_smoke.sh`; cargo `crates/nytprof-cli/tests/native_agg_json.rs` | `report --json` / `aggregates` leaf **15** / mid **3** / edge **15** + `discount_events` + `sub_entry_events` + A4/A4b convenience ints + stream/PID fields when present |
| Native stream/PID on JSON | `scripts/packaging/json_native_stream_smoke.sh` (**JSON-NATIVE-STREAM-MVP**) | native `report --json` emits `is_stream_complete` **true**, `incompleteness_reasons` **[]**, `time_line_events` / `pid_start_events` / `pid_end_events` from ProfileModel (default-calls1 TL **916**, pid ≥1); optional equal-compare vs Perl `query --json` |
| TIME_BLOCK multiplicity on JSON | `scripts/packaging/json_time_block_smoke.sh` (**JSON-TIME-BLOCK-MVP**); cargo `native_agg_json.rs` | `time_block_events` default-calls1 **0** / blocks-calls1 **916** on native + Perl JSON (model/JsonlData only) |
| Native↔query JSON cross-parity | `scripts/packaging/native_query_json_cross_smoke.sh` (**NATIVE-QUERY-JSON-CROSS** / **NATIVE-QUERY-JSON-CROSS-EXPAND** / **NATIVE-QUERY-JSON-CROSS-BLOCKS** / **NATIVE-QUERY-JSON-CROSS-META** / **NATIVE-QUERY-JSON-CROSS-TIMEBLOCK** / **NATIVE-QUERY-JSON-CROSS-COUNTS** / **NATIVE-QUERY-JSON-CROSS-TOTAL**) | Shared fields equal on default-calls1: `leaf_returns` **15**, `mid_returns` **3**, `mid_leaf_edge` **15**, `discount_events` **818**, and `sub_entry_events` **0** when **both** sides expose SUB_ENTRY; **expand:** calls2-default `sub_entry_events` **27** when both expose (fixture-scoped); **blocks:** blocks-calls1 `line_calls_1_5` **780** / `block_line_calls_1_4` **810** equal native↔perl (pair ×2); **meta (CROSS-META):** when both sides expose stream/PID + A9/A8 samples, equal `is_stream_complete` **true**, `incompleteness_reasons` **[]**, `time_line_events` / `pid_start_events` / `pid_end_events`, `sub_def_leaf` / `sub_def_mid` / `source_line_1_5`; **timeblock (CROSS-TIMEBLOCK):** when both expose `time_block_events`, default **0** / blocks **916**, and greppable meta required equal when both expose; **counts (CROSS-COUNTS):** when both expose event counters, equal `sub_return_events` **27** / `new_fid_events` **3** / `sub_callers_events` **13** / `src_line_events` **632** / `sub_info_events` **31**, and `file_1_basename` exact equal **or** both contain **`workload.pl`** (absolute `file_1` remains volatile); run when native CLI available |
| SUB_ENTRY on JSON surfaces | native `report --json` + Perl `query --json` (`sub_entry_events`) | **JSON-SUB-ENTRY-MVP** — multiplicity only; default-calls1 **0**, calls2-default **27** |
| Blocks A4/A4b JSON convenience | native `report --json` + Perl `query --json` (`line_calls_1_5` / `block_line_calls_1_4`) | **JSON-BLOCKS-MVP** — greppable ints (0 when absent); blocks-calls1 **780** / **810**; not full A4/A4b maps; native↔query equality: **NATIVE-QUERY-JSON-CROSS-BLOCKS** |
| A9/A8 JSON samples | native `report --json` + Perl `query --json` (`sub_def_leaf` / `sub_def_mid` / `source_line_1_5`) | **JSON-SUBDEF-SOURCE-MVP** — greppable samples (null when absent); default-calls1 leaf **1/3–7**, mid **1/8–12**, source `$x++` / `1 .. 50`; not full A8/A9 maps; smoke `scripts/packaging/json_subdef_source_smoke.sh` |
| ATTRIBUTE/OPTION/file JSON samples | native `report --json` + Perl `query --json` (`attribute_ticks_per_sec` / `option_calls` / `file_1`) | **JSON-META-FILES-MVP** — greppable samples (null when absent); default-calls1 `attribute_ticks_per_sec` **10000000**, `option_calls` **1**, `file_1` path contains **workload.pl**; not full attributes/options/files maps; smoke `scripts/packaging/json_meta_files_smoke.sh`; cargo `native_agg_json.rs`; cross optional equal when both expose: **NATIVE-QUERY-JSON-CROSS-META** |
| Event tag multiplicities on JSON | native `report --json` + Perl `query --json` (`sub_return_events` / `new_fid_events` / `sub_callers_events` / `src_line_events` / `sub_info_events`) | **JSON-EVENT-COUNTS-MVP** — default-calls1 **27** / **3** / **13** / **632** / **31**; smoke `scripts/packaging/json_event_counts_smoke.sh`; cross: **NATIVE-QUERY-JSON-CROSS-COUNTS** |
| Fid-1 basename sample on JSON | native `report --json` + Perl `query --json` (`file_1_basename`) | **JSON-FILE-BASENAME-MVP** — greppable stable basename (typically **`workload.pl`**); absolute `file_1` remains volatile; smoke `scripts/packaging/json_file_basename_smoke.sh`; cross: **NATIVE-QUERY-JSON-CROSS-COUNTS** |
| Query JSON (pure-Perl) | `scripts/packaging/perl_query_json_smoke.sh` (**CI-QUERY-JSON-GATE**) | golden `--jsonl` `query --json` MVP + expand fields incl. `sub_entry_events` + meta samples when present (no cargo) |
| Oracle harness roll-up | `tools/oracle/selftest_harness.sh` | nests dump parity, fail-closed, incomplete-stream, decode-fuzz, normalize, etc. |

### 3. Packaging (advertised offline)

| Item | Path | Evidence | Notes |
|------|------|----------|-------|
| Dual-path policy | `docs/BUILD_SUPPORT_POLICY.md` | `scripts/packaging/dual_path_smoke.sh` | legacy-only without Cargo; optional-native when cargo present |
| MakeMaker facade | `Makefile.PL` | `scripts/packaging/makemaker_dual_path_smoke.sh` | candidate entry only — **not** full BUILD-003 XS CPAN dual-build |
| Offline R1 gate | `scripts/ci/offline_gate.sh` | `make offline-gate` | cargo tests (honest skip) → harness → dual_path → engine_auto_fallback → **perl_jsonl_data_all** (incl. SUB_ENTRY multiplicity) → **perl_query_json** (CI-QUERY-JSON-GATE; required pure-Perl golden `--jsonl`) → **json_sub_entry** / **json_blocks** / **json_subdef_source** / **json_meta_files** / **json_time_block** (JSON-SUBDEF-SOURCE-MVP / **JSON-META-FILES-MVP** / **JSON-TIME-BLOCK-MVP** steps 6b–6i) → **native_agg_json** + **json_native_stream** + **json_report_incomplete** when native (**NATIVE-AGG-JSON** / **JSON-NATIVE-STREAM-MVP** / **JSON-REPORT-INCOMPLETE-FAILCLOSED**) → **native_query_json_cross** when native (**NATIVE-QUERY-JSON-CROSS** / **CROSS-EXPAND** / **CROSS-BLOCKS** / **CROSS-META** / **CROSS-TIMEBLOCK** / **CROSS-TOTAL**, shared fields incl. `sub_entry` on default-calls1 + calls2 **27** + blocks-calls1 **780**/**810** + `time_block_events` **0**/**916** + stream/PID + A9/A8 + meta samples on default-calls1) → capability_selftest when cargo/prefix/target present (CI-CAPABILITY-GATE; honest skip otherwise) → **collector_sink** step 10 (COL-001..007 + COL-014 dual; honest CC skip) → **e3_c_writer_parity** step 11 when cargo (**COL-007 product E3-EVENT**; E3-mixed residual) → **e4_v5_v6_semantic_smoke --full** step 12 when native (**E4 product CLI** on dual-sink pairs; honest skip otherwise; dual-sink fixtures required; full oracle dual residual TEST-008). **Not** multi-OS CI (**BUILD-006**) |
| `COL-015-FORK-PID-MVP` | **done (scaffold)** | Fork/PID protocol with buffered sinks: `nytp_fork_prepare/resume_*`, batch preflush + child residual discard, addpid path helper, v5/v6 child reinit, dual reinit, stress `test_fork_pid` (incl. POSIX fork). Schema [`docs/schemas/collector-fork-pid-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-fork-pid-mvp-v0.md); smoke step 10 artifacts. **Residuals:** full **TEST-018** oracle forkdepth/addpid/merge; live XS hooks; mid-deflate continue-in-child vs 6.15; product option wiring; OI-003-05 file-switch; multi-OS fork stress. |
| `COL-014-DUAL-SINK-MVP` | **done** (test/dev-only) | Same-run dual writer fan-out v5+v6 (OQ-4 **not** product UX): `nytp_dual_sink_*`, `test_dual_sink`, schema [`docs/schemas/collector-dual-sink-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-dual-sink-mvp-v0.md); smoke step 10. Logical equality on M4 + primary-fixture-shaped streams. **Residuals:** full fixtures oracle dual (TEST-003/TEST-008); full **TEST-018** fork under dual (unit dual+batch fork covered by **COL-015-FORK-PID-MVP**); E4 aggregate enforcement; product `format=dual` rejected. |
| `COL-002-LIFECYCLE-MVP` | **done (scaffold)** | Explicit sink lifecycle + emit gates; COL-015 fork/PID **protocol MVP done** (`nytp_fork_*` / `test_fork_pid`); residual full **TEST-018** oracle forkdepth/addpid + signal/file-switch matrix |
| COL-001..007 + COL-014 dual + COL-015 fork scaffold (not product collector) | **COL-001-SINK-MVP** + **COL-002-LIFECYCLE-MVP** + **COL-003-SEQ-MVP** + **COL-004-FAST-PATH-MVP** + **COL-005-BATCH-MVP** + **COL-006-V5-WIRE-MVP** + **COL-007-ABS-MVP** + **COL-007-CODEC-MVP** + **COL-007-PACK-MVP** + **TEST-003-FAKE-CLOCK-MVP** + **COL-014-DUAL-SINK-MVP** + **COL-015-FORK-PID-MVP** board **done** (scaffold) | Overlay `collector/` semantic sink + lifecycle + gapless seq + **bounded batch + no-alloc stmt fast path** + **real v5 wire writer (zlib)** + **absolute v6 writer (EVENT codecs NONE/ZLIB/ZSTD/LZ4 + multi-chunk + CRC + ADR-0001 packing + ADR-0002 FOOTER dict + mid-stream region)** + fake-clock/M4 **mini** harness + **fork/PID protocol with buffered sinks** (`nytp_fork_*`, batch preflush/discard, v5/v6 child reinit, POSIX fork stress) + unit tests + offline_gate step 10. **Residuals:** no live Perl/XS hooks; full M4 oracle corpus residual; full TEST-018 oracle forkdepth/addpid residual; board **COL-007** **done** (E3-EVENT PR-B09; E3-mixed residual); **COL-014 dual-sink test/dev-only (OQ-4)** ready — **not** product UX; full fixtures dual equality residual (TEST-003/TEST-008). Evidence: [`collector/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/collector/README.md), [`docs/schemas/collector-v5-wire-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v5-wire-mvp-v0.md), [`docs/schemas/collector-v6-absolute-wire-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-absolute-wire-mvp-v0.md), [`docs/schemas/collector-v6-codecs-multi-chunk-crc-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-codecs-multi-chunk-crc-mvp-v0.md), [`docs/schemas/collector-v6-packing-footer-dict-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-packing-footer-dict-mvp-v0.md), [`docs/schemas/collector-dual-sink-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-dual-sink-mvp-v0.md), [`docs/schemas/collector-fork-pid-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-fork-pid-mvp-v0.md), `scripts/packaging/collector_sink_smoke.sh` | R2 runway scaffolding; **not** a claim that collection is modernized in-process
| Offline R1 gate | `scripts/ci/offline_gate.sh` | `make offline-gate` | cargo tests (honest skip; includes `-p nytprof-ffi` when present) → harness → dual_path → engine_auto_fallback → **perl_jsonl_data_all** (incl. SUB_ENTRY) → **perl_xs_data_readstream** (step **5b** / **PERL-XS-DATA-READSTREAM-MVP**: product Data/ReadStream; golden required; binary when CLI; thin path only — **not** COMPAT-007 / pure-XS) → **perl_query_json** (CI-QUERY-JSON-GATE) → json surface smokes 6b–6i → **native_agg_json** + stream + incomplete when native → **native_query_json_cross** when native → capability_selftest when cargo/prefix/target present (honest skip otherwise). Multi-OS entry is separate: **BUILD-006-MVP** (`scripts/ci/matrix_gate.sh` + GHA); **not** full multi-OS product certification (**BUILD-006** residual) |
| Multi-OS CI matrix MVP | `scripts/ci/matrix_gate.sh`, [`.github/workflows/ci-matrix.yml`](https://github.com/hilather/nytprof-modernization/blob/main/.github/workflows/ci-matrix.yml) | board **BUILD-006-MVP** | GHA **ubuntu-latest** (`linux-x86_64`) + **macos-latest** (`macos-arm64`) (≥1 additional OS/arch); host-local oracle ensure; same offline_gate; honest skips preserved; portable oracle hash for macOS. **Not** full BUILD-006 (multi-Perl / multi-rustc / Windows / coverage dashboard / product multi-OS certification) |
| BUILD-003 depth (partial) | `Makefile.PL` + `install_facade.sh` + `dual-install` | `scripts/packaging/makemaker_build003_depth_smoke.sh` | closer dual-build: pure-Perl facade + native CLI under shared `prefix/`; stamps `packaging_depth=BUILD-003-depth-v0`, **`full_build003=0`**; legacy-only unbroken (`install-facade` without cargo). **Not** BUILD-003 full |
| Install prefix | `scripts/packaging/install_native.sh` → `prefix/bin/` | `scripts/packaging/native_install_smoke.sh` | stable CLI install for Perl bridge discovery |
| Broader packaging suite | `scripts/packaging/packaging_gate.sh` | legacy + engine + Perl dispatch + native when present | Super-set of dual-path; not the offline_gate packaging primary |

Isolation rule: **never** put `crates/` on oracle `PERL5LIB`.

### 4. Perl facade — `nytprof-engine` actions

| Action | Engine path | Code path | Schema / smoke |
|--------|-------------|-----------|----------------|
| `report` / `summary` | native subprocess / legacy dump smoke | `perl/bin/nytprof-engine`, `perl/lib/Devel/NYTProf/EngineDispatch.pm` | [`docs/schemas/perl-engine-dispatch-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-engine-dispatch-mvp-v0.md); `scripts/packaging/perl_engine_dispatch_smoke.sh` |
| `verify` / `inspect` | same | same | same |
| `html` / `csv` / `dump` | native (legacy = stream-dump smoke + NOTE) | same | same |
| `folded` / `callgrind` / `cg` | native subprocess only (no Perl reimplementation) | same | `scripts/packaging/perl_engine_export_smoke.sh` |
| `query` / `data-query` | native dump → JsonlData, or `--jsonl` golden; always-full MVP human: returns/edges + `sub_def` + `source_line` + `line_calls` / A4b `block_line_calls` samples + PID lifecycle (`pid_start`/`pid_end` counts) + ATTRIBUTE/OPTION (key names first); optional `--json` / `--format=json` → single JSON object (`ok`/`subs`/`edges`/`leaf_returns`/`mid_returns`/`mid_leaf_edge` + `discount_events` + `sub_entry_events` + A4/A4b `line_calls_1_5`/`block_line_calls_1_4` + A9/A8 samples `sub_def_leaf`/`sub_def_mid`/`source_line_1_5` + ATTRIBUTE/OPTION/file samples `attribute_ticks_per_sec`/`option_calls`/`file_1` + `is_stream_complete`/`incompleteness_reasons`/`time_line_events`/`pid_start_events`/`pid_end_events`) | `EngineDispatch::run_query` + `print_query_results` + `JsonlData` | `scripts/packaging/perl_engine_query_smoke.sh` + `perl_engine_query_expand_smoke.sh` + `perl_engine_query_pid_meta_smoke.sh` + `perl_query_json_smoke.sh` + `json_subdef_source_smoke.sh` + `native_query_json_cross_smoke.sh` (shared fields vs native `report --json`; **CROSS-EXPAND** / **CROSS-META**); `perl/t/engine_query_default_calls1.t` |
| `--engine=auto` / `NYTPROF_ENGINE=auto` | **Perl:** prefer native, fall back to legacy | `resolve_engine` + `select_runtime_engine` in EngineDispatch; Rust CLI still `auto`→`native` | `scripts/packaging/engine_auto_smoke.sh` + `engine_auto_fallback_smoke.sh` — **not** charter R3 product default flip (see residual) |

### 5. Pure-Perl `JsonlData` / ReadStream (advertised subset)

| Capability | Status in preview | Code path | Evidence |
|------------|-------------------|-----------|----------|
| returns (`sub_returns` / `sub_return_totals`) | **ready** | `perl/lib/Devel/NYTProf/JsonlData.pm` | `perl/t/jsonl_data_default_calls1.t`, `scripts/packaging/perl_jsonl_data_smoke.sh` |
| edges (`call_edge_count` / callers) | **ready** | same | same (mid→leaf **15**) |
| line_totals / `line_calls` (A4 from TIME_LINE + TIME_BLOCK) | **ready** | same | `perl/t/jsonl_data_blocks_calls1_line_totals.t`, `scripts/packaging/perl_line_totals_smoke.sh` (line5 **780**) |
| block_line_totals / `block_line_calls` (A4b from TIME_BLOCK) | **ready** | same | `perl/t/jsonl_data_a4b_blocks_calls1.t`, `scripts/packaging/perl_a4b_smoke.sh` (`"1:4".calls` **810**, A4 line5 **780**) |
| sub_defs (A9 from SUB_INFO) | **ready** | same | `perl/t/jsonl_data_subdefs_default_calls1.t`, `scripts/packaging/perl_subdefs_smoke.sh` |
| files (NEW_FID) | **ready** | same | same subdefs smoke |
| source_lines (A8 from SRC_LINE) | **ready** | same | `perl/t/jsonl_data_source_default_calls1.t`, `scripts/packaging/perl_source_smoke.sh` |
| attributes / options (ATTRIBUTE + OPTION) | **ready** | same | `perl/t/jsonl_data_meta_default_calls1.t`, `scripts/packaging/perl_meta_smoke.sh` |
| PID lifecycle (`pid_starts` / `pid_ends` / counts from PID_START + PID_END) | **ready** | same | `perl/t/jsonl_data_pid_default_calls1.t`, `scripts/packaging/perl_pid_smoke.sh` (default-calls1 start/end ≥1, pid **2975381**) |
| Stream completeness (`is_stream_complete` / `stream_incompleteness_reasons`; COMPAT-010) | **ready** | same (`time_line_events` / `time_block_events` + pid balance) | `perl/t/jsonl_data_stream_complete_default_calls1.t`, `scripts/packaging/perl_stream_complete_smoke.sh` (default-calls1 complete; header-only incomplete craft from real golden lines) |
| Discount events A3 (`discount_events` / `discount_count` from DISCOUNT) | **ready** (multiplicity only — not exclusive-time policy freeze) | same | `perl/t/jsonl_data_discount_default_calls1.t`, `scripts/packaging/perl_discount_smoke.sh` (default-calls1 stream re-count **818**) |
| SUB_ENTRY multiplicity (`sub_entry_events` / `sub_entry_count`) | **ready** (multiplicity only — not call-stack / arg freeze) | same | `perl/t/jsonl_data_sub_entry.t`, `scripts/packaging/perl_sub_entry_smoke.sh` (**PERL-SUB-ENTRY-JSONL**): default-calls1 **0** (`calls=1`); calls2-default **27** (`calls=2`); stream re-count + optional native dump. Wired into `perl_jsonl_data_all_smoke.sh` |
| JsonlReadStream chunk walk | **ready** | `perl/lib/Devel/NYTProf/JsonlReadStream.pm` | `perl/t/jsonl_readstream_default_calls1.t`, `scripts/packaging/perl_jsonl_readstream_smoke.sh` |

Schema roll-up: [`docs/schemas/perl-jsonl-data-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-jsonl-data-mvp-v0.md), [`docs/schemas/perl-jsonl-readstream-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-jsonl-readstream-mvp-v0.md).

### 5b. Product Data / ReadStream over binary profiles (thin MVP — PR-A06)

| Capability | Status | Code path | Evidence |
|------------|--------|-----------|----------|
| Binary profile → Data queries (`from_profile` / `new({filename})`) | **ready** (thin native-cli-jsonl) | `perl/lib/Devel/NYTProf/Data.pm` | `perl/t/data_product_default_calls1.t`, `scripts/packaging/perl_xs_data_readstream_smoke.sh` — leaf **15** / mid **3** / edge **15** / discount **818** |
| Binary profile → ReadStream `for_chunks(filename => …)` | **ready** (thin native-cli-jsonl) | `perl/lib/Devel/NYTProf/ReadStream.pm` | `perl/t/readstream_product_default_calls1.t` |
| Blocks A4/A4b on product Data | **ready** | same Data facade | `perl/t/data_product_blocks_calls1.t` — line5 **780** / block 1:4 **810** |
| Incomplete fail-closed on product Data | **ready** | default open croaks; `allow_incomplete` | `data_product_default_calls1.t` |
| COMPAT-007 bless-array fidelity | **not** claimed (`claims_compat007_shapes=0`) | — | residual for full PERL-005 |
| Pure-XS wire decode (no CLI) | **not** claimed | — | residual for full PERL-004 |

Schema: [`docs/schemas/perl-xs-data-readstream-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-xs-data-readstream-mvp-v0.md). Board: **PERL-XS-DATA-READSTREAM-MVP**.

---

## Residual for full R1 (explicit)

Binding close-or-waive map: [`docs/adrs/0003-r1-full-residual-policy.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) (**ADR-0003**, PR-A04).  
Release notes for this cut: [`docs/RELEASE_NOTES_R1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R1.md) (**PR-A10**).

| Residual | Plan / board refs | Why residual | Preview honesty |
|----------|-------------------|--------------|-----------------|
| No production C ABI / FFI / cdylib | **RUST-010**, `nytprof-ffi` (charter crate list only) | No shipped stable native library ABI for embedders; Perl bridge is **subprocess CLI**, not FFI | Pure-Rust crates + CLI only |
| No XS ReadStream over binary profiles | **PERL-004** | Preview is dump-JSONL pure-Perl `JsonlReadStream` only | [`perl-jsonl-readstream-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-jsonl-readstream-mvp-v0.md) non-goal |
| No XS / bless-array Data materializer | **PERL-005** (+ COMPAT-007 shapes) | Preview is pure-Perl `JsonlData` query subset from dump JSONL | Not full `Devel::NYTProf::Data` fidelity |
| No full nytprofhtml DOM / REPORT-001..020 | **REPORT-001..020**, BASE-005, **REPORT-HTML-RESIDUAL-INV** | Native HTML is MVP summary + multi-file site; not oracle DOM/CSS/tablesorter/flame/Graphviz | REPORT_SURFACE_CONTRACT **not advertised** list; **artifact residual matrix:** [`docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md) (oracle vs native classes on default-calls1) |
| COL-001..007 + COL-014 dual (test/dev) scaffold only (not product collector) | **COL-001-SINK-MVP** + **COL-002-LIFECYCLE-MVP** + **COL-003-SEQ-MVP** + **COL-004-FAST-PATH-MVP** + **COL-005-BATCH-MVP** + **COL-006-V5-WIRE-MVP** + **COL-007-ABS-MVP** + **COL-007-CODEC-MVP** + **COL-007-PACK-MVP** + **TEST-003-FAKE-CLOCK-MVP** + **COL-014-DUAL-SINK-MVP** board **done** (scaffold) | Overlay `collector/` semantic sink + lifecycle + gapless seq + **bounded batch + no-alloc stmt fast path** + **real v5 wire writer (zlib)** + **absolute v6 writer (EVENT codecs NONE/ZLIB/ZSTD/LZ4 + multi-chunk + CRC + ADR-0001 packing + ADR-0002 FOOTER dict + mid-stream region)** + fake-clock/M4 **mini** harness + unit tests + offline_gate step 10. **Residuals:** no live Perl/XS hooks; full M4 oracle corpus residual; COL-015 open; board **COL-007** **done** (E3-EVENT PR-B09; E3-mixed residual); **COL-014 dual-sink test/dev-only (OQ-4)** ready — **not** product UX; full fixtures dual equality residual (TEST-003/TEST-008). Evidence: [`collector/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/collector/README.md), [`docs/schemas/collector-v5-wire-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v5-wire-mvp-v0.md), [`docs/schemas/collector-v6-absolute-wire-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-absolute-wire-mvp-v0.md), [`docs/schemas/collector-v6-codecs-multi-chunk-crc-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-codecs-multi-chunk-crc-mvp-v0.md), [`docs/schemas/collector-v6-packing-footer-dict-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-packing-footer-dict-mvp-v0.md), [`docs/schemas/collector-dual-sink-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-dual-sink-mvp-v0.md), `scripts/packaging/collector_sink_smoke.sh` | R2 runway scaffolding; **not** a claim that collection is modernized in-process
| v6 wire freeze (IDs) | format plan / ADR-0006 | **done** (PR-B11): major=6 numeric IDs frozen; golden vectors `fixtures/v6/vectors/`; catalog `v6-wire-ids-frozen-v1.md`. **Residuals:** E3-mixed; CLI v6 default; full oracle E4 | v5 default collection; v6 opt-in after E5 |
| COL-007 done (E3-EVENT); COL-008 deferred | **COL-007** board **done** (product E3-EVENT with C); **COL-008** non-baseline; **E3-mixed** residual | C v6 EVENT writer product path green: absolute + codecs/multi-chunk/CRC + ADR-0001 packing + ADR-0002 FOOTER dict + mid-stream; fixtures `fixtures/v6/from-c/**`; tests `e3_c_*`; `tools/oracle/e3_c_writer_parity.sh`; offline_gate step 11. **Honest residuals:** E3-mixed multi-kind C fixtures; default-parse always-inflate residual; CLI v6 default; full oracle E4 residual (E4-v0 + E4 product CLI ready); live XS hooks; COL-008; LZ4 covered in product packing_lz4 fixture; edge codec fail-closed remains unit-suite residual. Wire freeze: **FMT-V6-WIRE-FREEZE** done. Preflight always-inflate stack retained under `nytprof-format-v6` / `docs/schemas/v6-*-provisional-v0.md`. | Product E3-EVENT ready with C; wire IDs frozen; not CLI v6 default |
| Product v6→ProfileModel ingest | **PRODUCT-V6-MODEL-INGEST-MVP** **done** (PR-B11a) | `ProfileModel::from_path` dual dispatch (`NYTPROF6` vs `NYTProf 5`); always-inflate + FOOTER dict; dump-aligned Event map; CLI dump/verify prelim; tests vs C abs/packing/dict + stand-in pair aggregates. Schema [`docs/schemas/product-v6-profilemodel-ingest-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-v6-profilemodel-ingest-mvp-v0.md). | Load path for E5; wire IDs frozen ADR-0006 |
| CLI E5 v6 opt-in surfaces | **CLI-E5-V6-OPT-IN-MVP** **done** (PR-B12) | report/html/csv/folded/callgrind/dump/verify on v6; capability `v6_decode`/`v6_report` true; **`convert`/`merge` false**; **`collection_default: v5`** (no default flip); tests `cli_e5_v6` + capability; schema [`docs/schemas/cli-e5-v6-opt-in-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cli-e5-v6-opt-in-mvp-v0.md). | **Not** collection default flip (R4); **not** convert/merge |
| E4-v0 model-level v5↔v6 aggregates | **E4-V0-MODEL-SEMANTIC-MVP** **done** (PR-B10) | Dual-sink same-run pairs [`fixtures/e4/dual-sink/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/e4/dual-sink/) → `ProfileModel::from_path` → `e4_v0_aggregates_equal`; `cargo test -p nytprof-model e4_v0_`; smoke `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only`. Schema [`docs/schemas/e4-v0-model-semantic-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-v0-model-semantic-mvp-v0.md). | **Not** full oracle dual (TEST-003/TEST-008); dual-sink test/dev-only; product CLI: **E4-PRODUCT-CLI-SMOKE-MVP** |
| E4 product CLI smoke | **E4-PRODUCT-CLI-SMOKE-MVP** **done** (PR-B12b) | Real CLIs on dual-sink pairs; offline_gate step 12; schema [`docs/schemas/e4-product-cli-smoke-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-product-cli-smoke-mvp-v0.md). | Full oracle dual residual |
| COL-009 C baseline reaffirm | **COL-009** / ADR-0007 **done** (PR-B13) | Production v6 writer backend = **C** (COL-007); COL-008 remains deferred non-baseline. ADR [`docs/adrs/0007-production-v6-writer-backend-c-baseline.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0007-production-v6-writer-backend-c-baseline.md). | **Not** COL-008 bake-off; **not** public perf claim |
| Convert / merge / salvage tooling | plan TOOL convert/merge; PR-C01/C02 class | **R2-preview **done** (PR-C01/C02; lossy residual)** on this branch — capability `convert`/`merge` **false**. Parallel tracks may implement tools; **do not advertise** until capability + gates green. | Honest **done** (PR-C01/C02; lossy residual) even if convert lands elsewhere first |
| No full MakeMaker XS dual-build CPAN | **BUILD-003** (full) | Candidate `Makefile.PL` facade only (**BUILD-MAKEMAKER-OPT** done) | Not a complete XS CPAN tarball dual-build |
| No multi-OS CI matrix | **BUILD-006** | Single-host offline gate only | `offline_gate.sh` is not multi-OS CI |
| No performance certification claims | WP-13 / BENCH-001; plan P1/P2 | **Public claims waived** until R2-stable gates green. Methodology + light harness only (PR-C04). | [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md) (P1/P2 methodology), `tools/bench/light_bench.sh` (`size`, `collector_micro`, dump/report proxies) — **no public SLOs / “% faster”** |
| `engine=auto` full product policy / default flip | charter **R3**; ENGINE-AUTO-FALLBACK done for **Perl facade** | Perl `nytprof-engine`: prefer-native / fall-back-legacy is shipped. Residual: R3 product **default** flip + field window/ADR; pure-Rust `nytprof-cli` still maps `auto`→`native` (no in-process legacy) | Facade smokes prove dual-path auto; **not** “auto is the product default” |
| Default engine/format flips | charter R3/R4 | Explicitly out of first slice and out of R2-preview / R2-stable **runtime** flips. **R4 field-window instrumentation (PR-E01):** [`docs/R4_FIELD_WINDOW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_FIELD_WINDOW.md) + collect/smoke scripts — **does not** flip defaults. **R4 policy (PR-E02):** [ADR-0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) + flip/rollback [`docs/R4_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md) — **accepted (policy)**; **flip not executed**; require accepted field report recommendation **Promote** before any runtime change | Native remains opt-in; `collection_default: v5` until flip |
| Legacy retirement | charter **R5** / REL-012 | **Never automatic.** **PR-F01 governance:** [ADR-0009](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0009-r5-legacy-retirement-governance.md) + [`docs/R5_RETIREMENT_REVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R5_RETIREMENT_REVIEW.md) — **accepted (policy)**; **no component retired**; absence of retirement is valid success | Do not claim legacy removed/deprecated; R3/R4 flips do not authorize removal |

---

## R2-preview advertised ready (opt-in only)

Charter **R2-preview** after Track B (PR-B13 packaging honesty). **Not** R2-stable. Release notes: [`docs/RELEASE_NOTES_R2_PREVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_PREVIEW.md).

| Surface / gate | Status | Evidence |
|----------------|--------|----------|
| CLI E5 v6 opt-in report/html/csv/… | **ready** | PR-B12; capability `v6_decode`/`v6_report` |
| Collection default | **v5** (runtime flip **not** executed; ADR-0008 policy ready) | capability `collection_default: v5` |
| COL-007 C writer E3-EVENT | **done** (since PR-B09) | `fixtures/v6/from-c/**`; offline_gate step 11 |
| Wire freeze major=6 IDs | **done** | ADR-0006; golden vectors |
| COL-009 writer backend | **C reaffirmed** | ADR-0007 |
| E4-v0 + E4 product CLI | **ready** (scaled dual-sink) | offline_gate step 12; full oracle residual |
| COL-014 dual-sink | **test/dev only** | OQ-4; not product UX |
| Dual-path legacy v5 / 6.15 | **unchanged** | dual_path_smoke; no `crates/` on oracle PERL5LIB |
| Convert / merge / salvage | **done** (PR-C01/C02; lossy residual) | capability `convert`/`merge`/`repack`/`salvage` true |
| E3-mixed / COL-008 / COL-015 / R3 / R4 | **residual / not started** | see residual rows |

### R2-preview residual (explicit)

| Residual | Why residual under R2-preview |
|----------|-------------------------------|
| **No convert/merge/salvage claim** | Capability must stay false until tools + gates green (Phase C) |
| **No R2-stable claim** | Fork suite, security/fuzz, **P1/P2 public cert** (methodology only until gates green), platform depth, convert tooling |
| **No COL-008** | Deferred non-baseline (ADR-0007) |
| **No R3/R4 defaults** | engine/format product default flips out of scope |
| **E3-mixed residual** | **done (MVP)** — `mixed.nytprof` + `e3_c_mixed_*`; not TEST-008 / COL-008 / CLI v6 collection default |
| **Full oracle E4 residual** | TEST-008 class |
**Vocabulary:** **closed (MVP)** = Phase A product path shipped with tests; **beyond-MVP residual** = deeper completeness still open; **WAIVE** = not required for full R1 native posture; **OUT-OF-R1** = R2+ / R3–R4.

| Residual | Plan / board refs | Full R1 cut status (PR-A10) | Honesty |
|----------|-------------------|----------------------------|---------|
| Production C ABI / FFI / cdylib | **RUST-010**, **FFI-CDYLIB-MVP** / **PR-A05** | **closed (MVP)** — open/query/close cdylib over `ProfileModel` ([`ffi-cdylib-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/ffi-cdylib-mvp-v0.md)); dual-path works without loading the dylib | **Beyond-MVP residual:** batch/event-walk APIs, BUILD-007 header automation, production dylib install (BUILD-004), sanitizer/Miri package, ABI freeze tooling. **Do not claim full RUST-010.** **OQ-2** — not waive |
| XS ReadStream over binary profiles | **PERL-004**, **PERL-XS-DATA-READSTREAM-MVP** / **PR-A06** | **closed (MVP)** — product `ReadStream` opens binary profiles via native `nytprof-cli dump` → `JsonlReadStream` ([`perl-xs-data-readstream-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-xs-data-readstream-mvp-v0.md)) | **Beyond-MVP residual:** pure-XS wire decode (no CLI), full scalar-flag package, dual-engine callback fidelity. Dump-JSONL bridge remains |
| XS / bless-array Data materializer | **PERL-005**, **PERL-XS-DATA-READSTREAM-MVP** / **PR-A06** (+ COMPAT-007) | **closed (MVP)** — product `Data` opens binary profiles via native dump → `JsonlData` query surface; `claims_compat007_shapes=0` | **Beyond-MVP residual:** COMPAT-007 bless-array / AV-HV fidelity, full oracle Data method set, in-process FFI-backed materializer. **Do not claim COMPAT-007** |
| Full nytprofhtml DOM / REPORT-001..020 | **REPORT-001..020**, **REPORT-HTML-RESIDUAL-INV**, **REPORT-HTML-SHARED-CSS**, **REPORT-HTML-SUBS-EXCL**, **M01-HTML-JS-WAIVE** | **Partial CLOSE + OPEN residual + WAIVE** per ADR-0003 HTML map + **PR-M01** amendment — **not** full oracle DOM. **CLOSED (MVP):** A01 shared CSS/structure (`style.css` / inline policy); A02 exclusive sub index (`index-subs-excl.html`). **OPEN residual (CLOSE path still):** A03 optional flame SVG + call-stack site inputs (native `folded` remains related export). **WAIVE** (residual-honest): Shared JS / tablesorter (**PR-M01** / Q4 user-final — documentation residual, not CLOSE; jquery **not** shipped); Graphviz **image render** (native `.dot` source is closed MVP), treemap, JIT, block/sub page modes, oracle per-file naming, browser `--open`, exact `-d`, mergeevals, oracle footer | Inventory: [`REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md). **Do not claim full oracle `nytprofhtml` DOM** |
| Multi-OS CI matrix | **BUILD-006**, **BUILD-006-MVP** / **PR-A07** | **closed (MVP)** — GHA `linux-x86_64` + `macos-arm64` via `matrix_gate.sh` + offline_gate | **Beyond-MVP residual:** multi-Perl / multi-rustc / Windows / coverage dashboard / product multi-OS certification. **Not full BUILD-006** |
| MakeMaker dual-build / packaging | **BUILD-003**, **BUILD-MAKEMAKER-OPT**, **BUILD-003-DEPTH** / **PR-A08** | **closed (depth MVP)** — facade + `install-facade` / `dual-install` / depth smoke (`full_build003=0`) | **Beyond-MVP residual:** full XS CPAN tarball dual-build with collector/XS in root Makefile. **Not full BUILD-003** |
| Performance certification / public SLOs | WP-13 / BENCH-001 / **PR-A09** | **WAIVED** public claims (default full-R1 posture). Light harness + methodology only (`docs/BENCH_NOTES.md`, `tools/bench/light_bench.sh`) | **Do not** publish P3/P4 SLOs or “% faster” without certified BENCH package. Closing public claims later requires green PR-A09 certification path |
| v6 wire freeze | format plan / Phase-0 | **OUT-OF-R1** | v5 read/report product path only; provisional v6 preflight ≠ freeze |
| COL-007 / COL-008 | **COL-007** deferred; **COL-008** non-baseline | **OUT-OF-R1** | Preflight crate `nytprof-format-v6` only — **not** C v6 writer, dictionaries, always-on inflate, or CLI v6 default |
| `engine=auto` product default flip | charter **R3** | **OUT-OF-R1** (policy ready; **flip not executed**) | Perl facade prefer-native/fallback shipped under **explicit** `auto`; product default when flag/env omitted remains **`native`**. Rust CLI `auto`→`native` residual. **PR-D01** field pack: [`docs/R3_FIELD_WINDOW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md). **PR-D02 policy:** [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) + flip/rollback procedure [`docs/R3_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) — **do not** claim R3 complete until flip checklist + accepted field report (recommendation **Promote**) |
| Default engine/format flips | charter R3/R4 | **OUT-OF-R1** | Native remains product opt-in default; R3 policy ADR accepted but flip gated; R4 separate (ADR-Q025) |
| Legacy retirement (per component) | charter **R5** / REL-012 | **OUT-OF-R1** (governance ready; **no component retired**) | **Never automatic.** [ADR-0009](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0009-r5-legacy-retirement-governance.md) + [`docs/R5_RETIREMENT_REVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R5_RETIREMENT_REVIEW.md) bind per-component process only. Absence of retirement is valid success. Do **not** claim any legacy path removed or deprecated by PR-F01 |

---

## Full R1 ready (product scope cut — PR-A10)

This section is the **advertised full R1 MVP product claim** after Phase A. It is **honest** under ADR-0003: MVP closes + explicit waives, with beyond-MVP residuals listed above.

### Claim summary

| Claim level | Status | Meaning |
|-------------|--------|---------|
| **Offline R0 / R1-preview ready** | **yes** | Documented surfaces + gates pass when cargo/oracle fixtures are present; dual-path legacy works without Cargo |
| **Full R1 ready (MVP product scope)** | **yes (scoped)** | ADR-0003 Phase A product cut: OQ-2 FFI/XS **MVP closed**; HTML **A01/A02 closed**; Shared JS/tablesorter **WAIVE** (**PR-M01** / Q4 — not a remaining CLOSE requirement); **A03 flame closed (MVP)** (CLI default-on 2026-08-15; `--no-flame` opt-out; `flamegraph.pl` parity residual); WAIVE classes residual-honest; **BUILD-006-MVP** + **BUILD-003-DEPTH** closed; public perf **WAIVED**. See non-claims |
| **Full R1 ready (complete residual table)** | **no** | Beyond-MVP rows remain (full RUST-010, COMPAT-007 / pure-XS, flame A03, full BUILD-003/006, oracle DOM) |
| **Not claimed (binding non-claims)** | — | COL-007 C writer; v6 wire freeze; CLI v6 default; full oracle `nytprofhtml` DOM; full BUILD-003 XS CPAN dual-build; full multi-OS product certification; full RUST-010 beyond open/query/close MVP; public perf SLOs; R3/R4 product default flips; CPAN upload |

### Phase A roll-up (what this cut advertises)

| PR | Role | Cut status |
|----|------|------------|
| **PR-A01** | Shared CSS + structure | **closed (MVP)** for CSS/structure only — Shared JS/tablesorter **WAIVE** for GA-candidate (**PR-M01** / Q4; not a remaining CLOSE requirement; jquery **not** shipped) |
| **PR-A02** | `index-subs-excl.html` exclusive ranking | **closed (MVP)** — not oracle DOM |
| **PR-A03** | Flame path | **closed (MVP)** — call-tree SVG + folded from `call_edges`; CLI **default-on** with `--no-flame` opt-out since 2026-08-15 (oracle `flame!`=1 parity); `flamegraph.pl`/`nytprofcalls` multi-frame remains residual |
| **PR-A04** | ADR-0003 residual policy map | **done** (policy) |
| **PR-A05** | FFI cdylib open/query/close | **closed (MVP)** — not full RUST-010 |
| **PR-A06** | Product Data/ReadStream over binary | **closed (MVP)** — not COMPAT-007 / pure-XS |
| **PR-A07** | Multi-OS CI matrix MVP | **closed (MVP)** — not full BUILD-006 |
| **PR-A08** | Packaging depth toward BUILD-003 | **closed (depth MVP)** — not full XS CPAN |
| **PR-A09** | R1-scoped perf certification | **WAIVE** public claims (default) |
| **PR-A10** | This readiness cut (matrix + release notes + runbook) | **done** |

### Operator re-verify (full R1 MVP cut)

Same offline gate as preview, plus product smokes when native:

```sh
./scripts/ci/offline_gate.sh
# Multi-OS matrix entry (MVP):
./scripts/ci/matrix_gate.sh
# Product Data/ReadStream (A06):
./scripts/packaging/perl_xs_data_readstream_smoke.sh
# Packaging depth (A08):
./scripts/packaging/makemaker_build003_depth_smoke.sh
# FFI (A05) when cargo present:
cargo test -p nytprof-ffi
```

---

## What “ready” means here

| Claim level | Meaning |
|-------------|---------|
| **Offline R0 / R1-preview ready** | Documented surfaces + gates above pass on this host when cargo/oracle fixtures are present; dual-path legacy still works without Cargo |
| **R2-preview ready (opt-in)** | v6 offline CLI surfaces + COL-007 E3-EVENT + wire freeze + dual-equality E3-EVENT/E4 product/E5 as listed; collection default still v5; convert/merge **not** claimed |
| **Full R1 ready** | Residual table closed (or explicitly waived by ADR) with product packaging, API materializers, report completeness, and certification policy as required by the plan DoD |
| **R2-stable ready** | **done (PR-C05 honesty cut)** — convert/merge/salvage, COL-015 MVP, SEC-FUZZ offline, P1/P2 methodology; residuals: E3-mixed, full oracle E4, public perf, R3/R4 |
| **R4 field-window pack (PR-E01)** | **instrumentation only** — [`docs/R4_FIELD_WINDOW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_FIELD_WINDOW.md) + [`scripts/field/r4_field_window_collect.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/r4_field_window_collect.sh) + report template; **does not** flip `collection_default` (stays **v5**) |
| **R4 default policy (PR-E02 / ADR-0008)** | **policy accepted**; flip **not executed** — [ADR-0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) + [`docs/R4_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md); eligible-tier `format=v6` default only after accepted field **Promote**; **`format=v5` retained**; do not claim R4 complete or `collection_default: v6` until flip checklist runs |
| **R5 retirement governance (PR-F01 / ADR-0009)** | **governance accepted**; **no component retired** — [ADR-0009](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0009-r5-legacy-retirement-governance.md) + [`docs/R5_RETIREMENT_REVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R5_RETIREMENT_REVIEW.md); per-component only; never automatic; absence of retirement is valid success |
| **Drop-in completion (PR-G01 docs-landed + PR-G02 scaffold + PR-G03a–e emit + PR-G04 attach-MVP)** | DoD + annex + isolation profiles **docs-landed**. G02 **v5-only archive** landed. G03a **debugger load** landed (no `nytprof.out` on trivial `-e`). G03b–G03e **emit-MVP** landed. G04 **attach-MVP** landed (live `-d:NYTProf` default-calls1 **15/3/15**). RPM / CPAN-TRIAL / full BUILD-003 / G05 / G06 **not ready**. See section below. |
| **Not claimed** | CPAN upload, performance SLOs / **P1–P4 certification**, R3/R4 **runtime** default flips, legacy **component** removal, full nytprofmerge option parity, COL-008 baseline |
| **Full R1 ready (MVP product scope)** | Per **ADR-0003** + **PR-A10** + **PR-M01**: required OQ-2 CLOSE **A05/A06 MVP** landed; preferred CLOSE **A07/A08 MVP/depth** landed; HTML **A01/A02** closed; Shared JS/tablesorter **WAIVE** (Q4 user-final — not CLOSE); **A03 flame OPEN CLOSE residual**; other WAIVE classes residual-honest; public perf **WAIVED**. Explicit non-claims: COL-007, wire freeze, CLI v6 default, full oracle DOM, full BUILD-003, full RUST-010 beyond MVP, R3–R4 defaults |
| **Not claimed** | Full multi-OS product certification; CPAN upload; performance SLOs; v6 collection / COL-007 encoder; full RUST-010 ABI freeze / production dylib install; full PERL-004/005 pure-XS / COMPAT-007; full oracle `nytprofhtml` DOM; Shared JS/tablesorter as native-ready (WAIVE, not shipped); oracle `flamegraph.pl`/`nytprofcalls` flame parity (A03 native flame MVP shipped; CLI default-on 2026-08-15) |

### Drop-in completion (PR-G01 docs-landed + PR-G02 scaffold + PR-G03a load + PR-G03b/G03c/G03d/G03e emit)

Binding contracts landed; G02 landed the D1-B **v5-only link artifact** and a **load-only** bootstrap XS. G03a landed product `perl -d:NYTProf` **load** (no `nytprof.out` on trivial `-e`). G03b landed statement-path **emit** through `nytp_emit_time_line` / `time_block` / `discount` (fake-clock mini + overflow fail-closed). G03c landed call-path **emit** through `nytp_emit_sub_entry` / `sub_return` (mini `NYTProf 5` + dump tags). G03d landed meta/finalize **emit** through `nytp_emit_attribute` / `option` / `new_fid` / `src_line` / `sub_info` / `pid_start` / `pid_end` (mini `NYTProf 5` + dump tags). G03e landed compress **emit** through `nytp_emit_start_deflate` (mini `NYTProf 5`; dump/verify inflate recovers a post-deflate event; **mid-deflate fork residual**). **Do not** treat this as opcode collection attach or packaging ship.

| ID | Honesty |
|----|---------|
| `DROP-IN-DOD-V0` | **done (docs-landed)** — [`DROP_IN_DOD_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md) |
| `PRODUCT-OPTIONS-MATRIX` | **done (docs + tests)** — G05 unknown/`dual` fail-closed; D1-B `format=v6` fail-closed; D1-A `NYTPROF6` |
| `G01-DESIGN-LAND` | **done** — [`PRODUCT_COMPLETION_DROP_IN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PRODUCT_COMPLETION_DROP_IN_v0.md) |
| `G02-V5-PRODUCT-LINK` | **done (scaffold)** — `libnytp_sink_v5.a` + CollectorBootstrap load; **not** D1 attach |
| `G03A-LOAD-ONLY` | **done** — product `perl -d:NYTProf` loads; no `nytprof.out` on trivial `-e`; `$PRODUCT_XS_ATTACH` stays false |
| `G03B-STMT-EMIT` | **done** — `nytp_emit_time_line` / `time_block` / `discount` via product XS; fake-clock mini gate landed; no G04 fixture parity |
| `G03C-SUB-EMIT` | **done** — `nytp_emit_sub_entry` / `sub_return` via product XS; dump `SUB_ENTRY` / `SUB_RETURN` on a real `NYTProf 5` mini; no G04 fixture parity |
| `G03D-META-EMIT` | **done** — `nytp_emit_attribute` / `option` / `new_fid` / `src_line` / `sub_info` / `pid_start` / `pid_end` via product XS; dump seven meta tags on a real `NYTProf 5` mini; no G04 fixture parity |
| `G03E-COMPRESS-EMIT` | **done** — `nytp_emit_start_deflate` via product XS; dump/verify inflate recovers a post-deflate event on a real `NYTProf 5` mini; **mid-deflate fork residual**; no G04 fixture parity |
| `PRODUCT-XS-ATTACH-MVP` | **done (MVP)** — live `-d:NYTProf` + `NYTPROF file=`; E1b default `OP_ENTERSUB` (`$^P` 0x01 off); **E2 landed** `OP_GOTO` on default. dump/report leaf **15** / mid **3** / mid→leaf **15**. **Residuals:** E3 leave / E4 full slowops |
| `PRODUCT-FORK-ADDPID-MVP` | **done (MVP)** — live `fork` + `addpid=1`; parent + `<file>.<childpid>` `NYTProf 5` via `nytp_fork_*`. **Residuals:** mid-deflate-in-child / TEST-018 / `_exit` flush |
| `PRODUCT-LEGACY-SMOKE` | **done (MVP)** — cargo-free prefix install + live `-d:NYTProf` 15/3/15; **not** BUILD-003-FULL / S2 dual_path |
| `I02-MAKEMAKER-NATIVE` | **done (MVP)** — `NYTPROF_NATIVE=1` fail-closed without cargo; `auto`/`=0` cargo-free; cargo-present install `nytprof-cli` 15/3/15 |
| `I03-DIST-SCRIPTS` | **done (MVP)** — cargo-free EngineDispatch + `nytprofhtml`/`nytprofcsv` in product prefix; installed `query --json --jsonl` 15/3/15; **not** 6.15 nytprofhtml DOM / BUILD-003-FULL / S2 |
| `J01-CPAN-HYGIENE` | **done (MVP)** — `Makefile.PL` NAME `Devel::NYTProf`, VERSION_FROM product `.pm` **7.00**; MANIFEST excludes `baseline/` `target/` `prefix/` |
| `MIG01-MIGRATION-GUIDE` | **done (docs)** — [`docs/MIGRATION_DROP_IN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/MIGRATION_DROP_IN_v0.md) |
| `K03-PREBUILT-CLI-ADR` | **done (docs)** — [ADR-0010](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0010-signed-ci-prebuilt-native-cli.md); K02 spec landed |
| `PRODUCT-V6-COLLECT-EL8` | **residual** |
| `BUILD-003-FULL` | **residual** |
| `CPAN-TRIAL-READY` | **done (notes-ready / MVP)** — attach-preview notes; **not** PAUSE uploaded |
| `EL8-RPM-MODULE` | **done (MVP)** — D1-B spec + smoke; **not** mock-certified multi-stream |
| `EL8-RPM-TOOLS` | **done (MVP)** — tools companion spec; **not** signed-pipeline complete |
| `P01-GA-CANDIDATE` | **done (MVP)** — collection drop-in preview on advertised flavors; Rocky default D1-B only; **not** SEC-012 complete / R3–R4 |
| `P02-SEC-CUT` | **done (MVP / checklist / job)** — SEC-012 checklist + SEC-002 job wrapping `selftest_security_fuzz.sh`; **not** independent sign-off / full continuous fuzz / GA marketing / S2 |
| `E4-02-ORACLE-PAIR-MVP` | **done (MVP)** — second oracle dual pair (blocks-calls1) count surfaces; **not** full TEST-008 / A4 780 attach / S2 |
| `E4-03-ORACLE-PAIR-MVP` | **done (MVP)** — third oracle dual pair (calls2-default) count surfaces; **not** full TEST-008 / SUB_ENTRY 27 attach / S2 |
| `NS-NYTPROFM-IDENTITY` | **done (MVP / Option B)** — CPAN **NYTProfM**, `Devel::NYTProfM` **6.15**, `-d:NYTProfM`; not PAUSE; not Provides stock Devel::NYTProf |
| `DROP-IN-REMAINING` | **residual** — opcode/full TEST-003/S2/COMPAT-007/publish. **DI-01 780/810**, **DI-02 27+CORE:**, **DI-04 mini projected kinds** landed. **DI-03 opcode/`entersub` — in progress, not done** (E1b default omit installs `OP_ENTERSUB`; **E2 landed** `OP_GOTO` on default; wrap list still `wrap=1` only; **E3** opt-in `leave=1` DISCOUNT, default `leave` stays 0; `wrap=1` / `use_db_sub=1` / `entersub=0` escape; E4 full slowops residual). |
| `TOOL-MERGE-AGGREGATE-SUM-MVP` | **done (MVP)** — opt-in `--aggregate-sum`; stream-concat default; **not** full nytprofmerge option parity / S2 |
| `SEC-012-CHECKLIST-MVP` | **done (MVP / checklist)** — not independent sign-off |
| `SEC-002-CONTINUOUS-FUZZ-MVP` | **done (MVP / job)** — not cargo-fuzz / AFL / deep corpus |
| `API-DATA-COMPAT007` | **residual** |

Do **not** mark EL8 RPM or `BUILD-003-FULL` as ready. `CPAN-TRIAL-READY` is **notes-ready MVP** only (**not** PAUSE uploaded). G04 **attach-MVP** is landed (live `-d:NYTProf` default-calls1 **15/3/15**; E1b default call attach is opcode `OP_ENTERSUB`, not wrap). G03a **load** (no `nytprof.out` on trivial `-e`) remains; G03b–G03e emit-MVP remain. **Residuals:** mid-deflate continue-in-child, full TEST-018. DI-01 live **780/810** is landed (not full opcode). **DI-03 opcode/`entersub` — in progress, not done** (E1b default omit installs `OP_ENTERSUB`, emit after INIT, `$^P` 0x01 off; **E2 landed** `OP_GOTO` on default, wrap list still `wrap=1` only; **E3** opt-in `leave=1` installs leave ops + `nytp_emit_discount`, default `leave` stays **0**; `wrap=1` / `use_db_sub=1` / `entersub=0` escape; E4 full slowops residual). G05 options/`format=v6` and G06 fork/`addpid` MVP landed. Annex: [`product-xs-graft-annex-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-graft-annex-v0.md). Dual-path remains **oracle-primary** until S2 ([BUILD_SUPPORT_POLICY](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md)).

### Operator re-verify (preview)

Full operator map: [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md).

```sh
# Single offline gate (recommended)
./scripts/ci/offline_gate.sh
# or: make offline-gate

# Report contract evidence (oracle PERL5LIB isolated — never crates/)
bash tools/oracle/report_semantic_parity.sh

# Capability self-test (native CLI required)
./scripts/packaging/capability_selftest_smoke.sh
```

---

## Re-verify checklist

1. Advertised tables still match REPORT_SURFACE_CONTRACT + schemas linked above (incl. NATIVE-AGG-JSON + QUERY-JSON shared fields + **JSON-SUB-ENTRY-MVP** + **JSON-BLOCKS-MVP** + **JSON-META-FILES-MVP** + **NATIVE-QUERY-JSON-CROSS-EXPAND** + **NATIVE-QUERY-JSON-CROSS-BLOCKS** + **NATIVE-QUERY-JSON-CROSS-META**).  
2. Residual table still lists **no production FFI/XS Data** (RUST-010, PERL-004/005), **no full nytprofhtml DOM** (REPORT-001..020 + HTML residual inventory), **COL-007 E3-EVENT done**; wire freeze **done** (ADR-0006); residual **E3-mixed** / **COL-008**, **no multi-OS CI** (BUILD-006), **no perf claims**, R3 default flip, and Rust CLI `auto`→native residual.
3. `./scripts/ci/offline_gate.sh` still green (or cargo honestly skipped with harness + dual-path + expand + query-JSON steps green; native_agg + **native_query_json_cross** (incl. CROSS-EXPAND + CROSS-BLOCKS + CROSS-META) + capability skip only when no native CLI is available).  
4. Pure-Perl SUB_ENTRY multiplicity still green: `./scripts/packaging/perl_sub_entry_smoke.sh` (default **0** / calls2 **27**); JSON surfaces + cross (incl. blocks **780**/**810** + stream/PID + A9/A8 + meta samples): `./scripts/packaging/native_query_json_cross_smoke.sh` when native.  
5. Any **new** advertised surface requires a contract/matrix revision — do not silently expand “ready.”  
6. Board rows `R1-RESIDUAL-MATRIX` and `R1-HONESTY-SYNC` remain **done**; COL-007 product E3-EVENT is **done** (PR-B09) with evidence paths pointing at this file / runbook / `fixtures/v6/from-c/**`.  
7. Operator runbook [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) (`R1-PREVIEW-RUNBOOK`) still matches this matrix’s ready vs residual claims.  
8. R2-preview packaging honesty (`R2-PREVIEW-READINESS-CUT`, release notes, COL-009 ADR-0007) still lists convert/merge residual and no R3/R4 claims.
2. Residual / full-R1 cut table still lists **beyond-MVP** honesty: full RUST-010 beyond **FFI-CDYLIB-MVP**; full PERL-004/005 beyond **PERL-XS-DATA-READSTREAM-MVP** (no COMPAT-007 / pure-XS); **no full nytprofhtml DOM** (A01/A02 MVP only; A03 flame open; WAIVE classes residual); **no v6 / COL-007** product writer; **no full multi-OS** beyond **BUILD-006-MVP**; **no full BUILD-003** beyond **BUILD-003-DEPTH**; public perf **WAIVED**; R3/R4 defaults OUT-OF-R1; Rust CLI `auto`→native residual.  
3. `./scripts/ci/offline_gate.sh` still green (or cargo honestly skipped with harness + dual-path + expand + query-JSON steps green; native_agg + **native_query_json_cross** + capability skip only when no native CLI is available).  
4. Pure-Perl SUB_ENTRY multiplicity still green: `./scripts/packaging/perl_sub_entry_smoke.sh` (default **0** / calls2 **27**); product Data/ReadStream: `./scripts/packaging/perl_xs_data_readstream_smoke.sh`; JSON cross when native: `./scripts/packaging/native_query_json_cross_smoke.sh`.  
5. Any **new** advertised surface requires a contract/matrix revision — do not silently expand “ready.”  
6. Board rows `R1-RESIDUAL-MATRIX`, `R1-HONESTY-SYNC`, and `R1-FULL-READINESS-CUT` remain **done before COL-007** with evidence paths pointing at this file / runbook / release notes.  
7. Operator runbook [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) still matches this matrix’s ready vs residual / full-R1 cut claims.

---

## Board placement

| ID | Status | Evidence |
|----|--------|----------|
| `R1-RESIDUAL-MATRIX` | done | this file (`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`) |
| `R1-RESIDUAL-POLICY-ADR` | **done** (policy; PR-A04) | [`docs/adrs/0003-r1-full-residual-policy.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) — full R1 CLOSE/WAIVE map; **OQ-2** FFI/XS → CLOSE PR-A05/A06 |
| `FFI-CDYLIB-MVP` | **done** (MVP; PR-A05) | [`docs/schemas/ffi-cdylib-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/ffi-cdylib-mvp-v0.md); crate `crates/nytprof-ffi` (`cdylib`+`rlib`); header `crates/nytprof-ffi/include/nytprof_ffi.h`; `cargo test -p nytprof-ffi` (default-calls1 leaf/mid/edge **15/3/15**, discount **818**, calls2 `sub_entry` **27**, blocks **780/810**, incomplete fail-closed). **OQ-2** product path — not waive. Full RUST-010 residual: batch APIs, BUILD-007, production dylib install, sanitizer package. Offline_gate step 1 includes `-p nytprof-ffi`. **Before COL-007.** |
| `PERL-XS-DATA-READSTREAM-MVP` | **done** (MVP; PR-A06) | [`docs/schemas/perl-xs-data-readstream-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-xs-data-readstream-mvp-v0.md); `perl/lib/Devel/NYTProf/Data.pm` + `ReadStream.pm` (binary via native dump → JsonlData/JsonlReadStream); tests `perl/t/data_product_*.t` + `readstream_product_default_calls1.t`; smoke `scripts/packaging/perl_xs_data_readstream_smoke.sh` (default-calls1 **15/3/15** + discount **818**, blocks **780/810**, incomplete fail-closed, `claims_compat007_shapes=0`). **OQ-2** product path — not waive. Full PERL-004/005 residual: pure-XS wire decode, COMPAT-007 bless-array. Offline_gate includes product smoke. **Before COL-007.** |
| `R1-PREVIEW-RUNBOOK` | done | [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) |
| `R2-PREVIEW-READINESS-CUT` | **done** (PR-B13) | R2-preview opt-in honesty: [`docs/RELEASE_NOTES_R2_PREVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_PREVIEW.md); COL-009 ADR-0007; dual-equality readiness R2-preview row; convert/merge residual; dual-path legacy unchanged |
| `COL-009` | **done** (PR-B13) | [`docs/adrs/0007-production-v6-writer-backend-c-baseline.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0007-production-v6-writer-backend-c-baseline.md) — reaffirm C baseline; COL-008 deferred |
| `R2-P1P2-METHODOLOGY` | **done** (PR-C04) | P1/P2 methodology + light harness proxies; **public claims waived** until R2-stable gates green — [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md) |
| `R1-HONESTY-SYNC` | **done** | this matrix + runbook re-synced to advertise **NATIVE-QUERY-JSON-CROSS** / **CROSS-EXPAND** / **CROSS-BLOCKS** / **CROSS-META** / **CROSS-TIMEBLOCK** / **CROSS-TOTAL** / **CROSS-COUNTS**, **JSON-EVENT-COUNTS-MVP**, **JSON-FILE-BASENAME-MVP** (absolute `file_1` volatile; basename greppable stable sample), **JSON-TIME-BLOCK-MVP**, **JSON-REPORT-INCOMPLETE-FAILCLOSED**, **JSON-SUB-ENTRY-MVP**, **JSON-BLOCKS-MVP**, **JSON-META-FILES-MVP**, + **PERL-SUB-ENTRY-JSONL** while retaining full-R1 residual honesty (no production FFI/XS Data, no full nytprofhtml DOM, COL-007 E3-EVENT done with C; wire freeze done (residual E3-mixed / COL-008); no multi-OS CI; no perf claims). Evidence: `scripts/packaging/native_query_json_cross_smoke.sh`, `scripts/packaging/json_time_block_smoke.sh`, `scripts/packaging/json_report_incomplete_smoke.sh`, `scripts/packaging/perl_sub_entry_smoke.sh`, `perl/t/jsonl_data_sub_entry.t`, `crates/nytprof-cli/tests/native_agg_json.rs`, offline_gate steps 6f–8 when native |
| `R1-HONESTY-SYNC` | **done** | this matrix + runbook advertise preview surfaces (**NATIVE-QUERY-JSON-CROSS** family, JSON-* MVPs, **PERL-SUB-ENTRY-JSONL**) + Phase A product MVPs (**FFI-CDYLIB-MVP**, **PERL-XS-DATA-READSTREAM-MVP**, **BUILD-006-MVP**, **BUILD-003-DEPTH**, HTML A01/A02) while retaining beyond-MVP / OUT-OF-R1 residual honesty (full RUST-010 / PERL-004/005 / COMPAT-007; no full oracle DOM; A03 flame open; no v6/COL-007 product writer; no full multi-OS / full BUILD-003; public perf **WAIVED**). Evidence: offline_gate, `perl_xs_data_readstream_smoke.sh`, `cargo test -p nytprof-ffi`, matrix_gate, depth smoke |
| `R1-FULL-READINESS-CUT` | **done** (PR-A10) | Full R1 **MVP product scope** advertised: residual table § Full R1 ready + disposition column; release notes [`docs/RELEASE_NOTES_R1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R1.md); runbook residual honesty for full cut. **Does not** claim COL-007, wire freeze, CLI v6 default, full oracle DOM, full BUILD-003 XS CPAN, full RUST-010 beyond MVP, public perf SLOs. **Before COL-007.** |
| `BUILD-006-MVP` | **done** | GHA [`.github/workflows/ci-matrix.yml`](https://github.com/hilather/nytprof-modernization/blob/main/.github/workflows/ci-matrix.yml) matrix **ubuntu-latest** (`linux-x86_64`) + **macos-latest** (`macos-arm64`); entry [`scripts/ci/matrix_gate.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/matrix_gate.sh) (host oracle ensure + `offline_gate.sh`); portable `sha256_file` for macOS; honest skips preserved; policy [`docs/BUILD_SUPPORT_POLICY.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md). **Not** full BUILD-006. **Before COL-007.** |
| `BUILD-003-DEPTH` | **done** (PR-A08) | Packaging depth toward BUILD-003: `install-facade` / `dual-install` / depth smoke; stamps `full_build003=0`. **Not** full XS CPAN dual-build. **Before COL-007.** |
| `NATIVE-AGG-JSON` | done | [`docs/schemas/native-aggregates-json-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-aggregates-json-mvp-v0.md); `scripts/packaging/native_agg_json_smoke.sh` |
| `NATIVE-QUERY-JSON-CROSS` | done | `scripts/packaging/native_query_json_cross_smoke.sh` — native `report --json` ↔ Perl `query --json` shared fields **15/3/15** + discount **818**; offline_gate when native |
| `NATIVE-QUERY-JSON-CROSS-EXPAND` | done | same smoke — shared fields include `sub_entry_events` **0** on default-calls1 when both sides expose SUB_ENTRY; calls2-default side-by-side **27** (fixture-scoped); schemas [`native-aggregates-json-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-aggregates-json-mvp-v0.md) + [`perl-engine-dispatch-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-engine-dispatch-mvp-v0.md). **Before COL-007.** |
| `NATIVE-QUERY-JSON-CROSS-BLOCKS` | **done** | same smoke — blocks-calls1 pair ×2: `line_calls_1_5` **780** / `block_line_calls_1_4` **810** equal native↔perl (real CLIs only). **Before COL-007.** |
| `NATIVE-QUERY-JSON-CROSS-META` | **done** | same smoke — default-calls1 pair ×2 (+ dump path): when both sides expose stream/PID + A9/A8 samples, equal `is_stream_complete` **true**, `incompleteness_reasons` **[]**, `time_line_events` / `pid_*_events`, `sub_def_leaf` / `sub_def_mid` / `source_line_1_5`; optional equal greppable meta (`attribute_ticks_per_sec`) when both expose (**JSON-META-FILES-MVP**). **Before COL-007.** |
| `NATIVE-QUERY-JSON-CROSS-TIMEBLOCK` | **done** | same smoke — when both expose `time_block_events`: default-calls1 **0**, blocks-calls1 **916**; greppable meta required equal when both expose. **Before COL-007.** |
| `NATIVE-QUERY-JSON-CROSS-COUNTS` | **done** | same smoke — default-calls1 pair ×2 (+ dump path): when both expose, equal event counts **27/3/13/632/31** + `file_1_basename` (exact or both contain **`workload.pl`**). Absolute `file_1` remains volatile. **Before COL-007.** |
| `JSON-EVENT-COUNTS-MVP` | **done** | `sub_return_events` / `new_fid_events` / `sub_callers_events` / `src_line_events` / `sub_info_events` on both JSON surfaces; default-calls1 **27/3/13/632/31**. **Before COL-007.** |
| `JSON-FILE-BASENAME-MVP` | **done** | `file_1_basename` greppable stable sample (**`workload.pl`**); absolute `file_1` volatile. **Before COL-007.** |
| `JSON-TOTAL-EVENTS-MVP` | **done** | `total_events` dump stream incl. synthetic `_END` on both JSON surfaces; default-calls1 **2474**; smoke `scripts/packaging/json_total_basetime_smoke.sh`; offline_gate step 6i. **Before COL-007.** |
| `JSON-ATTR-BASETIME-MVP` | **done** | greppable `attribute_basetime` on both JSON surfaces; default-calls1 often **`"1786111723"`**; same smoke; offline_gate step 6i. **Before COL-007.** |
| `NATIVE-QUERY-JSON-CROSS-TOTAL` | **done** | same cross smoke — default-calls1 pair ×2 (+ dump path): when both expose, equal `total_events` **2474** + equal `attribute_basetime`. **Before COL-007.** |
| `JSON-NATIVE-STREAM-MVP` | **done** | native `report --json` stream/PID fields from ProfileModel (`is_stream_complete`, `incompleteness_reasons`, `time_line_events`, `pid_start_events`, `pid_end_events`); smoke `scripts/packaging/json_native_stream_smoke.sh`. **Before COL-007.** |
| `JSON-TIME-BLOCK-MVP` | **done** | `time_block_events` A2 on native + Perl JSON (default **0** / blocks **916**); smoke `scripts/packaging/json_time_block_smoke.sh`; offline_gate step 6f. **Before COL-007.** |
| `JSON-REPORT-INCOMPLETE-FAILCLOSED` | **done** | `report --json` / aggregates fail closed on incomplete streams (COMPAT-010); smoke `scripts/packaging/json_report_incomplete_smoke.sh`; cargo incomplete JSON CLI tests; offline_gate when native. **Before COL-007.** |
| `JSON-SUBDEF-SOURCE-MVP` | **done** | greppable `sub_def_leaf` / `sub_def_mid` / `source_line_1_5` on native + Perl JSON; smoke `scripts/packaging/json_subdef_source_smoke.sh`. **Before COL-007.** |
| `JSON-META-FILES-MVP` | **done** | greppable `attribute_ticks_per_sec` / `option_calls` / `file_1` on native `report --json` + Perl `query --json` (null when absent); default-calls1 ticks **10000000**, option calls **1**, `file_1` contains **workload.pl**; smoke `scripts/packaging/json_meta_files_smoke.sh` (offline_gate step 6e); cargo `crates/nytprof-cli/tests/native_agg_json.rs`; not full attribute/option/file maps. **Before COL-007.** |
| `JSON-SUB-ENTRY-MVP` | done | `sub_entry_events` on native `report --json` + Perl `query --json`; default **0** / calls2 **27** |
| `JSON-BLOCKS-MVP` | done | greppable `line_calls_1_5` / `block_line_calls_1_4` on native + Perl JSON (blocks-calls1 **780** / **810**; 0 when absent) |
| `PERL-SUB-ENTRY-JSONL` | done | `JsonlData` `sub_entry_events` / `sub_entry_count`; `perl/t/jsonl_data_sub_entry.t`; `scripts/packaging/perl_sub_entry_smoke.sh` (default **0**, calls2 **27**); roll-up via `perl_jsonl_data_all_smoke.sh` |
| `REPORT-CONTRACT-FREEZE` | done | [`docs/contracts/REPORT_SURFACE_CONTRACT_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_SURFACE_CONTRACT_v0.md) |
| `REPORT-HTML-RESIDUAL-INV` | done | [`docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md); lister `tools/oracle/list_html_artifacts.sh` |
| `REPORT-HTML-SHARED-CSS` | **done** | multi-file `style.css` + single-file inline policy; structure contract [`docs/schemas/html-shared-css-structure-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md); cargo `html_shared_css_structure_contract_default_calls1` (**15/3/15**). Not oracle CSS/JS/tablesorter. **Before COL-007.** |
| `REPORT-HTML-SUBS-EXCL` | **done** | multi-file `index-subs-excl.html` exclusive ranking; schema [`docs/schemas/html-subs-excl-index-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-subs-excl-index-mvp-v0.md); cargo `html_subs_excl_index_default_calls1` (leaf **15** / mid **3**). Inventory Full sub index → **partial**. Not oracle DOM/tablesorter. **Before COL-007.** |
| `M01-HTML-JS-WAIVE` | **done (docs)** | **PR-M01** / Q4 user-final: Shared JS/tablesorter **WAIVE** for GA-candidate (documentation residual, not CLOSE). jquery **not** shipped. Native HTML remains MVP. Flame A03 **closed (MVP)**; CLI default-on 2026-08-15 (`--no-flame` opt-out). |
| `CI-OFFLINE-GATE` | done | `scripts/ci/offline_gate.sh` |
| `CI-OFFLINE-GATE-EXPAND` | done | offline_gate steps 4–5: `engine_auto_fallback_smoke` + `perl_jsonl_data_all_smoke` (incl. SUB_ENTRY); step **5b** `perl_xs_data_readstream_smoke` (**PERL-XS-DATA-READSTREAM-MVP**) |
| `CI-QUERY-JSON-GATE` | done | offline_gate step 6: required `perl_query_json_smoke` (QUERY-JSON-MVP / QUERY-JSON-EXPAND golden `--jsonl`; no cargo; after step 5b) |
| `CI-CAPABILITY-GATE` | done | offline_gate step 9: `capability_selftest_smoke` when cargo/prefix/target present |
| `FMT-V6-HEADER-PROVISIONAL` | **done** | provisional v6 fixed-header contract (not wire freeze); `docs/schemas/v6-fixed-header-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-HEADER-PARSE-MVP` | **done** | `cargo test -p nytprof-format-v6` drives `parse_fixed_header` (valid/bad magic/truncated/unsupported major). **Before full COL-007.** |
| `FMT-V6-CHUNK-PROVISIONAL` | **done** | provisional v6 chunk-frame contract (not wire freeze); `docs/schemas/v6-chunk-frame-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-CHUNK-PARSE-MVP` | **done** | `parse_chunk_frame` fail-closed tests (bad sync/truncated/oversize/unknown required kind). **Before full COL-007.** |
| `FMT-V6-VARINT-PROVISIONAL` | **done** | provisional ULEB128 contract (not freeze); `docs/schemas/v6-varint-uleb128-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-VARINT-MVP` | **done** | `encode_u64`/`decode_u64` strict; round-trip + truncated + overlong tests. **Before full COL-007.** |
| `FMT-V6-SVARINT-PROVISIONAL` | **done** | provisional ZigZag+ULEB128 signed contract (SLEB residual); `docs/schemas/v6-svarint-zigzag-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-SVARINT-MVP` | **done** | `encode_i64`/`decode_i64` + tests (negatives/truncated/overlong). **Before full COL-007.** |
| `FMT-V6-STRING-PROVISIONAL` | **done** | provisional length-prefixed string/blob contract; `docs/schemas/v6-string-blob-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-STRING-MVP` | **done** | `encode_string_blob`/`decode_string_blob` + tests. **Before full COL-007.** |
| `FMT-V6-TLV-PROVISIONAL` | **done** | provisional header TLV contract; `docs/schemas/v6-header-tlv-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-TLV-MVP` | **done** | `encode_tlv`/`decode_tlv` + tests. **Before full COL-007.** |
| `FMT-V6-TLV-REGION-PROVISIONAL` | **done** | multi-TLV region + END terminator contract; `docs/schemas/v6-tlv-region-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-TLV-REGION-MVP` | **done** | `encode_tlv_region`/`decode_tlv_region` + tests. **Before full COL-007.** |
| `FMT-V6-FILE-PREFIX-PROVISIONAL` | **done** | fixed header + multi-TLV file-prefix contract; `docs/schemas/v6-file-prefix-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-FILE-PREFIX-MVP` | **done** | `encode_file_prefix`/`decode_file_prefix` + tests. **Before full COL-007.** |
| `FMT-V6-PREFIX-CHUNK-STREAM-PROVISIONAL` | **done** | prefix + chunk stream layout; codec NONE MVP; `docs/schemas/v6-prefix-chunk-stream-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-PREFIX-CHUNK-STREAM-MVP` | **done** | `encode_prefix_chunk_stream`/`decode_prefix_chunk_stream` + tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-PROVISIONAL` | **done** | event-body opcode codec (codec NONE payload); `docs/schemas/v6-event-body-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-MVP` | **done** | `encode_event_body`/`decode_event_body` + tests. **Before full COL-007.** |
| `FMT-V6-MINI-PROFILE-PROVISIONAL` | **done** | mini-profile composition; `docs/schemas/v6-mini-profile-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-MINI-PROFILE-MVP` | **done** | `encode_mini_profile`/`decode_mini_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-EVENT-PROVISIONAL` | **done** | multi-chunk EVENT framing; `docs/schemas/v6-multi-chunk-event-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-EVENT-MVP` | **done** | `encode_multi_chunk_event_profile`/`decode_multi_chunk_event_profile` + tests. **Before full COL-007.** |
| `FMT-V6-SOURCE-BODY-PROVISIONAL` | **done** | SOURCE chunk body codec NONE; `docs/schemas/v6-source-body-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-SOURCE-BODY-MVP` | **done** | `encode_source_body`/`decode_source_body` + EVENT+SOURCE composition + tests. **Before full COL-007.** |
| `FMT-V6-INDEX-BODY-PROVISIONAL` | **done** | INDEX chunk body codec NONE; `docs/schemas/v6-index-body-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-INDEX-BODY-MVP` | **done** | `encode_index_body`/`decode_index_body` + mixed EVENT+SOURCE+INDEX composition + tests. **Before full COL-007.** |
| `FMT-V6-SUMMARY-BODY-PROVISIONAL` | **done** | SUMMARY chunk body codec NONE; `docs/schemas/v6-summary-body-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-SUMMARY-BODY-MVP` | **done** | `encode_summary_body`/`decode_summary_body` + mixed EVENT+SOURCE+INDEX+SUMMARY composition + tests. **Before full COL-007.** |
| `FMT-V6-FOOTER-BODY-PROVISIONAL` | **done** | FOOTER chunk body codec NONE; `docs/schemas/v6-footer-body-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-FOOTER-BODY-MVP` | **done** | `encode_footer_body`/`decode_footer_body` + mixed composition with FOOTER last + tests. **Before full COL-007.** |
| `FMT-V6-CRC-PROVISIONAL` | **done** | CRC32 IEEE header/payload contract; `docs/schemas/v6-crc-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-CRC-MVP` | **done** | `crc32_ieee` / sealed header+chunk encode / optional verify + tests. **Before full COL-007.** |
| `FMT-V6-PAYLOAD-ZLIB-PROVISIONAL` | **done** | ZLIB payload codec contract; `docs/schemas/v6-payload-zlib-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-PAYLOAD-ZLIB-MVP` | **done** | `deflate_zlib`/`inflate_zlib`/`decode_chunk_payload`/`encode_chunk_frame_zlib` + tests. **Before full COL-007.** |
| `FMT-V6-PAYLOAD-ZSTD-PROVISIONAL` | **done** | ZSTD payload codec contract; `docs/schemas/v6-payload-zstd-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-PAYLOAD-ZSTD-MVP` | **done** | `compress_zstd`/`decompress_zstd`/`encode_chunk_frame_zstd` + `decode_chunk_payload` + tests. **Before full COL-007.** |
| `FMT-V6-PAYLOAD-LZ4-PROVISIONAL` | **done** | LZ4 payload codec contract; `docs/schemas/v6-payload-lz4-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-PAYLOAD-LZ4-MVP` | **done** | `compress_lz4`/`decompress_lz4`/`encode_chunk_frame_lz4` + `decode_chunk_payload` + tests. **Before full COL-007.** |
| `FMT-V6-COMPRESSED-PROFILE-PROVISIONAL` | **done** | Compressed multi-codec mini-profile contract; `docs/schemas/v6-compressed-profile-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-COMPRESSED-PROFILE-MVP` | **done** | `encode_compressed_mini_profile`/`decode_compressed_mini_profile` + NONE/ZLIB/ZSTD/LZ4 tests. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-COMPRESSED-PROVISIONAL` | **done** | Multi-chunk EVENT + compressed payloads contract; `docs/schemas/v6-multi-chunk-compressed-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-COMPRESSED-MVP` | **done** | `encode_multi_chunk_compressed_profile`/`decode_multi_chunk_compressed_profile` + ≥2-chunk tests. **Before full COL-007.** |
| `FMT-V6-COMPRESSED-MIXED-PROVISIONAL` | **done** | Compressed multi-kind mixed contract; `docs/schemas/v6-compressed-mixed-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-COMPRESSED-MIXED-MVP` | **done** | `encode_compressed_mixed_profile`/`decode_compressed_mixed_profile` + EVENT/SOURCE/INDEX/SUMMARY tests. **Before full COL-007.** |
| `FMT-V6-PER-KIND-CODEC-PROVISIONAL` | **done** | Per-kind payload codecs contract; `docs/schemas/v6-per-kind-codec-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-PER-KIND-CODEC-MVP` | **done** | `KindCodecs` + `encode_compressed_mixed_profile_per_kind` + multi-codec tests. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-KIND-PROVISIONAL` | **done** | Multi-chunk EVENT under mixed contract; `docs/schemas/v6-multi-chunk-kind-mixed-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-KIND-MVP` | **done** | `encode_multi_chunk_kind_mixed_profile` + ≥2 EVENT + SOURCE tests. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-SOURCE-PROVISIONAL` | **done** | Multi-chunk SOURCE under mixed contract; `docs/schemas/v6-multi-chunk-source-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-SOURCE-MVP` | **done** | `partition_source_records` + `encode_multi_chunk_source_mixed_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-INDEX-PROVISIONAL` | **done** | Multi-chunk INDEX under mixed contract; `docs/schemas/v6-multi-chunk-index-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-INDEX-MVP` | **done** | `partition_index_records` + `encode_multi_chunk_index_mixed_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-SUMMARY-PROVISIONAL` | **done** | Multi-chunk SUMMARY under mixed contract; `docs/schemas/v6-multi-chunk-summary-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-SUMMARY-MVP` | **done** | `partition_summary_records` + `encode_multi_chunk_summary_mixed_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-SPAN-PROVISIONAL` | **done** | Mid-record EVENT span contract; `docs/schemas/v6-mid-record-span-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-SPAN-MVP` | **done** | `encode_mid_record_span_event_profile`/`decode_mid_record_span_event_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-SOURCE-PROVISIONAL` | **done** | Mid-record SOURCE span contract; `docs/schemas/v6-mid-record-source-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-SOURCE-MVP` | **done** | `encode_mid_record_span_source_profile`/`decode_mid_record_span_source_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-INDEX-PROVISIONAL` | **done** | Mid-record INDEX span contract; `docs/schemas/v6-mid-record-index-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-INDEX-MVP` | **done** | `encode_mid_record_span_index_profile`/`decode_mid_record_span_index_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-SUMMARY-PROVISIONAL` | **done** | Mid-record SUMMARY span contract; `docs/schemas/v6-mid-record-summary-provisional-v0.md`. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-SUMMARY-MVP` | **done** | `encode_mid_record_span_summary_profile`/`decode_mid_record_span_summary_profile` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-CHUNK-PROVISIONAL` | **done** | Always-inflate consumer path + optional CRC contract; `docs/schemas/v6-decoded-chunk-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-CHUNK-MVP` | **done** | `decode_chunk`/`decode_chunk_frame_plain` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-STREAM-PROVISIONAL` | **done** | Always-inflate multi-chunk stream contract; `docs/schemas/v6-decoded-stream-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-STREAM-MVP` | **done** | `decode_prefix_chunk_stream_plain`/`encode_prefix_sealed_chunks` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-EVENT-PROVISIONAL` | **done** | Stream→inflate→event-body contract; `docs/schemas/v6-decoded-event-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-EVENT-MVP` | **done** | `encode_decoded_event_profile`/`decode_decoded_event_profile` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-SOURCE-PROVISIONAL` | **done** | Stream→inflate→source-body contract; `docs/schemas/v6-decoded-source-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-SOURCE-MVP` | **done** | `encode_decoded_source_profile`/`decode_decoded_source_profile` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-INDEX-PROVISIONAL` | **done** | Stream→inflate→index-body contract; `docs/schemas/v6-decoded-index-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-INDEX-MVP` | **done** | `encode_decoded_index_profile`/`decode_decoded_index_profile` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-SUMMARY-PROVISIONAL` | **done** | Stream→inflate→summary-body contract; `docs/schemas/v6-decoded-summary-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-SUMMARY-MVP` | **done** | `encode_decoded_summary_profile`/`decode_decoded_summary_profile` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-PROVISIONAL` | **done** | Multi-kind always-inflate + optional CRC contract; `docs/schemas/v6-decoded-mixed-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MVP` | **done** | `encode_decoded_mixed_profile`/`decode_decoded_mixed_profile` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MULTI-CHUNK-PROVISIONAL` | **done** | Multi-chunk record-aligned always-inflate mixed contract; `docs/schemas/v6-decoded-mixed-multi-chunk-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MULTI-CHUNK-MVP` | **done** | `encode_decoded_mixed_multi_chunk_profile` + `decode_decoded_mixed_profile` multi-chunk tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-PROVISIONAL` | **done** | Mid-record span on always-inflate multi-kind mixed contract; `docs/schemas/v6-decoded-mixed-mid-record-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-MVP` | **done** | `encode_decoded_mixed_mid_record_event_profile` + mid-record mixed tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-PROVISIONAL` | **done** | SOURCE mid-record on always-inflate multi-kind mixed contract; `docs/schemas/v6-decoded-mixed-mid-record-source-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-MVP` | **done** | `encode_decoded_mixed_mid_record_source_profile` + SOURCE mid-on-mixed tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-INDEX-PROVISIONAL` | **done** | INDEX mid-record on always-inflate multi-kind mixed contract; `docs/schemas/v6-decoded-mixed-mid-record-index-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-INDEX-MVP` | **done** | `encode_decoded_mixed_mid_record_index_profile` + INDEX mid-on-mixed tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-SUMMARY-PROVISIONAL` | **done** | SUMMARY mid-record on always-inflate multi-kind mixed contract; `docs/schemas/v6-decoded-mixed-mid-record-summary-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-SUMMARY-MVP` | **done** | `encode_decoded_mixed_mid_record_summary_profile` + SUMMARY mid-on-mixed tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-CONCURRENT-PROVISIONAL` | **done** | Concurrent multi-kind mid-record on always-inflate mixed contract; `docs/schemas/v6-decoded-mixed-mid-record-concurrent-provisional-v0.md`. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-CONCURRENT-MVP` | **done** | `encode_decoded_mixed_mid_record_concurrent_profile` + concurrent mid-on-mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-TIME-BLOCK-SUB-ENTRY-PROVISIONAL` | **done** | TIME_BLOCK + SUB_ENTRY provisional opcodes; `docs/schemas/v6-event-body-time-block-sub-entry-provisional-v0.md`. Not full catalog freeze. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-TIME-BLOCK-SUB-ENTRY-MVP` | **done** | Event-body TIME_BLOCK/SUB_ENTRY encode/decode + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SUB-RETURN-SUB-INFO-PROVISIONAL` | **done** | SUB_RETURN + SUB_INFO provisional opcodes; `docs/schemas/v6-event-body-sub-return-sub-info-provisional-v0.md`. Not full catalog freeze. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SUB-RETURN-SUB-INFO-MVP` | **done** | Event-body SUB_RETURN/SUB_INFO encode/decode + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SRC-LINE-NEW-FID-PROVISIONAL` | **done** | SRC_LINE + NEW_FID provisional opcodes; `docs/schemas/v6-event-body-src-line-new-fid-provisional-v0.md`. Not full catalog freeze. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SRC-LINE-NEW-FID-MVP` | **done** | Event-body SRC_LINE/NEW_FID encode/decode + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-PID-START-END-PROVISIONAL` | **done** | PID_START + PID_END provisional opcodes; `docs/schemas/v6-event-body-pid-start-end-provisional-v0.md`. Not full catalog freeze / COL-015. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-PID-START-END-MVP` | **done** | Event-body PID_START/PID_END encode/decode + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SUB-CALLERS-DISCOUNT-PROVISIONAL` | **done** | SUB_CALLERS + DISCOUNT provisional opcodes; `docs/schemas/v6-event-body-sub-callers-discount-provisional-v0.md`. Not full catalog freeze / DISCOUNT accounting freeze. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SUB-CALLERS-DISCOUNT-MVP` | **done** | Event-body SUB_CALLERS/DISCOUNT encode/decode + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-ATTRIBUTE-OPTION-PROVISIONAL` | **done** | ATTRIBUTE + OPTION provisional opcodes; `docs/schemas/v6-event-body-attribute-option-provisional-v0.md`. Not full catalog freeze / key vocabulary freeze. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-ATTRIBUTE-OPTION-MVP` | **done** | Event-body ATTRIBUTE/OPTION encode/decode + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-COMMENT-PROVISIONAL` | **done** | COMMENT provisional opcode; `docs/schemas/v6-event-body-comment-provisional-v0.md`. Not START_DEFLATE-as-event freeze. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-COMMENT-MVP` | **done** | Event-body COMMENT encode/decode + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-START-DEFLATE-PROVISIONAL` | **done** | START_DEFLATE provisional opcode (marker only); `docs/schemas/v6-event-body-start-deflate-provisional-v0.md`. Not VERSION prelude freeze / mid-stream codec switch. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-START-DEFLATE-MVP` | **done** | Event-body START_DEFLATE encode/decode + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-VERSION-PROVISIONAL` | **done** | VERSION provisional opcode (major/minor); `docs/schemas/v6-event-body-version-provisional-v0.md`. Not OI-001-03 sequence-number freeze / auto-emit from header. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-VERSION-MVP` | **done** | Event-body VERSION encode/decode + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-DUAL-OUTPUT-SEQUENCE-PROVISIONAL` | **done** | Dual-output multi-record order preflight; `docs/schemas/v6-event-body-dual-output-sequence-provisional-v0.md`. Not OI-001-03 sequence-number freeze / auto-emit VERSION. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-DUAL-OUTPUT-SEQUENCE-MVP` | **done** | Dual-output sequence encode/decode order+fields + always-inflate EVENT/mixed (NONE/ZLIB/ZSTD/LZ4; SOURCE co-kind). **Before full COL-007.** |
| `FMT-V6-START-DEFLATE-MID-STREAM-CODEC-SWITCH-PROVISIONAL` | **done** | START_DEFLATE mid-stream chunk-codec switch preflight; `docs/schemas/v6-event-body-start-deflate-mid-stream-codec-switch-provisional-v0.md`. Not v5 mid-payload stream deflate freeze / OI-001-03 seq-number freeze. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-START-DEFLATE-MID-STREAM-CODEC-SWITCH-MVP` | **done** | Mid-stream codec-switch encode/decode + always-inflate EVENT/mixed (NONE→ZLIB/ZSTD/LZ4; SOURCE co-kind). **Before full COL-007.** |
| `FMT-V6-AUTO-EMIT-VERSION-PROVISIONAL` | **done** | Auto-emit VERSION from fixed-header preflight; `docs/schemas/v6-auto-emit-version-provisional-v0.md`. Not OI-001-03 / full key-vocab freeze. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-AUTO-EMIT-VERSION-MVP` | **done** | Auto-emit VERSION encode/decode helpers + always-inflate EVENT/mixed (NONE/ZLIB/ZSTD/LZ4; SOURCE co-kind; mismatch fail-closed). **Before full COL-007.** |
| `FMT-V6-ATTR-OPTION-KNOWN-KEY-PROVISIONAL` | **done** | ATTRIBUTE/OPTION known-key preflight; `docs/schemas/v6-attr-option-known-key-provisional-v0.md`. Not complete OI-002-03/04 inventory. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-ATTR-OPTION-KNOWN-KEY-MVP` | **done** | Known-key table + body/always-inflate EVENT/mixed tests (basetime, ticks_per_sec, application, calls, blocks; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-UNKNOWN-OPTIONAL-SKIP-PROVISIONAL` | **done** | Unknown optional length-framed skip preflight; `docs/schemas/v6-event-body-unknown-optional-skip-provisional-v0.md`. Not permanent flag-bit freeze. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-UNKNOWN-OPTIONAL-SKIP-MVP` | **done** | Length-framed unknown-optional skip + always-inflate EVENT/mixed (order+fields; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `COL-001-SINK-MVP` | **done (scaffold)** | Semantic sink under `collector/` (ADR-0004); counting + v5 wire sink; `make -C collector test`; smoke + offline_gate step 10. **Not** COL-007 / live hooks. |
| `COL-002-LIFECYCLE-MVP` | **done (scaffold)** | Explicit sink lifecycle + emit gates; **not** COL-015 full fork/signal matrix |
| `COL-003-SEQ-MVP` | **done (scaffold)** | Internal gapless logical seq; not on default v5 wire |
| `COL-004-FAST-PATH-MVP` | **done (scaffold)** | No-alloc TIME_LINE/TIME_BLOCK batch append + `nytp_fast_emit_*`; light microbench engineering only — **not** BENCH cert |
| `COL-005-BATCH-MVP` | **done (scaffold)** | Bounded event batch + arena; order under cap 1..64; SV lifetime; emergency oversized; flush-discount residual |
| `COL-006-V5-WIRE-MVP` | **done (scaffold)** | Real v5 wire via sink (FileHandle.xs protocol + zlib); mini samples accepted by Rust decoder; **not** full oracle corpus / live hooks / COL-007 |
| `TEST-003-FAKE-CLOCK-MVP` | **done (scaffold)** | Fake-clock + M4 **mini** sample (via counting + v5 wire); full corpus residual until complete TEST-003 |
| `ADR-0001-V6-PACKING-ACCEPTED` | **done** (accepted ADR) | [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md): packing intent freeze **accepted** (OQ-1 as-is; not superseded). **Not** wire freeze; **not** COL-007. **Before full COL-007.** |
| `ADR-0002-V6-STRING-POOL-ACCEPTED` | **done** (accepted ADR) | [`docs/adrs/0002-v6-string-pool-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md): FOOTER-local dict intent **accepted** (OQ-1). Not global pool; not COL-007. **Before full COL-007.** |
| `FMT-V6-PROVISIONAL-ID-LOCKFILE` | **done** (status **frozen** by ADR-0006) | [`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md) + [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h). Plan **FMT-002..010 deviation** closed for freeze class by **FMT-V6-WIRE-FREEZE**. |
| `FMT-V6-WIRE-FREEZE` | **done** (PR-B11) | [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md) + [`docs/schemas/v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md): major=6 IDs frozen after E3-EVENT(C)+E4-v0; OQ-5 seq policy; OQ-6 deferred. |
| `FMT-V6-GOLDEN-VECTORS` | **done** (PR-B11) | [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/) + SHA256SUMS; `cargo test -p nytprof-format-v6 golden_vector_` / `wire_freeze_`. |
| `DUAL-EQUALITY-READINESS-MVP` | **done** | [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md). E1–E5 checklist; ADRs accepted; **E3-EVENT ready with C**; COL-014 dual-sink test/dev harness ready (OQ-4); **E4-v0 model ready**; E3-mixed / full oracle E4 residual; E4 product + E5 ready; **R2-preview opt-in ready (PR-B13)**; COL-009 C baseline (ADR-0007); COL-008 deferred; convert/merge residual.  |
| `COL-007-ABS-MVP` | **done (scaffold)** | Absolute v6 writer MVP (`nytp_sink_v6`, codec NONE EVENT, unit vectors). Schema [`docs/schemas/collector-v6-absolute-wire-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-absolute-wire-mvp-v0.md). **Not** packing/dict/codecs/multi-chunk; scaffold only; board COL-007 product E3-EVENT closed in PR-B09. |
| `COL-007-PACK-MVP` | **done (scaffold)** | Packing + FOOTER dict + mid-stream region (PR-B08). Schema [`docs/schemas/collector-v6-packing-footer-dict-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-packing-footer-dict-mvp-v0.md). |
| `COL-007` | **done** (E3-EVENT) | C v6 writer product E3-EVENT with C: `fixtures/v6/from-c/**`, `crates/nytprof-format-v6/tests/e3_c.rs` (`e3_c_*`), `tools/oracle/e3_c_writer_parity.sh`, offline_gate step 11, schema [`docs/schemas/collector-v6-e3-c-fixtures-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-e3-c-fixtures-mvp-v0.md). **Residuals:** E3-mixed multi-kind C fixtures; full oracle E4 residual (E4-v0 + E4 product CLI ready); CLI v6 default; COL-008; live XS hooks. Wire freeze: **FMT-V6-WIRE-FREEZE**. |
| `COL-014-DUAL-SINK-MVP` | **done** (test/dev-only) | Same-run dual writer fan-out v5+v6 (OQ-4 **not** product UX): `nytp_dual_sink_*`, `test_dual_sink`, schema [`docs/schemas/collector-dual-sink-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-dual-sink-mvp-v0.md); smoke step 10. Logical equality on M4 + primary-fixture-shaped streams. **Residuals:** full fixtures oracle dual (TEST-003/TEST-008); COL-015; product `format=dual` rejected. E4-v0 model aggregates on dual pairs: **E4-V0-MODEL-SEMANTIC-MVP**. |
| `FMT-V6-STRING-DICTIONARY-PROVISIONAL` | **done** | String-dictionary intern preflight; `docs/schemas/v6-string-dictionary-provisional-v0.md`. Not permanent global pool freeze. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-STRING-DICTIONARY-MVP` | **done** | Dictionary encode/decode + resolve + always-inflate EVENT/mixed (NONE/ZLIB/ZSTD/LZ4; SOURCE co-kind). **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SITE-DELTA-PROVISIONAL` | **done** | Location/site-delta preflight; `docs/schemas/v6-event-body-site-delta-provisional-v0.md`. Not permanent packing ADR. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SITE-DELTA-MVP` | **done** | Site-delta encode/decode + always-inflate EVENT/mixed (TIME_LINE/TIME_BLOCK/SUB_ENTRY absolute reconstruction; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-TIME-LINE-RUN-PROVISIONAL` | **done** | TIME_LINE_RUN packed-run preflight; `docs/schemas/v6-event-body-time-line-run-provisional-v0.md`. Not permanent packing ADR. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-TIME-LINE-RUN-MVP` | **done** | TIME_LINE_RUN encode/decode expands to ordered TIME_LINE (every ticks retained) + always-inflate EVENT/mixed (NONE/ZLIB/ZSTD/LZ4; SOURCE co-kind). **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-TIME-BLOCK-RUN-PROVISIONAL` | **done** | TIME_BLOCK_RUN packed-run preflight; `docs/schemas/v6-event-body-time-block-run-provisional-v0.md`. Not permanent packing ADR. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-TIME-BLOCK-RUN-MVP` | **done** | TIME_BLOCK_RUN encode/decode expands to ordered TIME_BLOCK (every ticks retained) + always-inflate EVENT/mixed (NONE/ZLIB/ZSTD/LZ4; SOURCE co-kind; TIME_LINE_RUN coexistence). **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SEQ-NUMBER-PROVISIONAL` | **done** | Logical event sequence-number preflight (OI-001-03 runway); `docs/schemas/v6-event-body-seq-number-provisional-v0.md`. Not full OI-001-03 freeze. Default parse non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SEQ-NUMBER-MVP` | **done** | `FLAG_HAS_SEQ` + `encode_event_body_with_seq` / `decode_event_body_full` + always-inflate EVENT/mixed (dual-output order+seq; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-ATTR-OPTION-KNOWN-KEY-EXPAND-PROVISIONAL` | **done** | Expand known-key inventory from golden fixture dumps; `docs/schemas/v6-attr-option-known-key-provisional-v0.md` (9 ATTRIBUTE + 18 OPTION). Not full OI-002 freeze. **Before full COL-007.** |
| `FMT-V6-ATTR-OPTION-KNOWN-KEY-EXPAND-MVP` | **done** | Fixture JSONL membership + expanded sample encode/decode + always-inflate EVENT/mixed (NONE/ZLIB/ZSTD/LZ4); free-form unknown still Ok. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SITE-DELTA-SEQ-COMPOSE-PROVISIONAL` | **done** | Composed site-delta+seq packing preflight; `docs/schemas/v6-event-body-site-delta-seq-compose-provisional-v0.md`. Not permanent packing ADR. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SITE-DELTA-SEQ-COMPOSE-MVP` | **done** | `encode_event_body_with_site_deltas_and_seq` + always-inflate EVENT/mixed (absolute sites + per-event seq; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-STRING-DICT-SITE-DELTA-SEQ-COMPOSE-PROVISIONAL` | **done** | FOOTER string-dictionary + site-delta/seq packing compose; `docs/schemas/v6-string-dict-site-delta-seq-compose-provisional-v0.md`. Not permanent pool/packing ADR. **Before full COL-007.** |
| `FMT-V6-STRING-DICT-SITE-DELTA-SEQ-COMPOSE-MVP` | **done** | `encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq` + mixed sibling; resolved strings + absolute sites + seq; NONE/ZLIB/ZSTD/LZ4. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Multi-chunk packing continuity preflight; `docs/schemas/v6-multi-chunk-site-delta-seq-packing-provisional-v0.md`. Not permanent packing ADR. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-MVP` | **done** | `PackingEncodeState` + multi-chunk packing encode; join = single-chunk; always-inflate EVENT/mixed (NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-STRING-DICT-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | FOOTER string-dict + multi-chunk site-delta/seq packing continuity; `docs/schemas/v6-string-dict-multi-chunk-site-delta-seq-packing-provisional-v0.md`. Not permanent pool/packing ADR. **Before full COL-007.** |
| `FMT-V6-STRING-DICT-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-MVP` | **done** | `encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq` (`max_events_per_chunk≥1`) + mixed sibling; multi-chunk join = single-chunk dict+packing; NONE/ZLIB/ZSTD/LZ4. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-TIME-RUN-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Multi-chunk packing + TIME_*_RUN continuity; `docs/schemas/v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md`. Not permanent packing ADR. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-TIME-RUN-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Multi-chunk packing with TIME_LINE_RUN/TIME_BLOCK_RUN; post-run site-delta across chunks; always-inflate EVENT/mixed NONE/ZLIB/ZSTD/LZ4. **Before full COL-007.** |
| `FMT-V6-STRING-DICT-MULTI-CHUNK-TIME-RUN-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | FOOTER string-dict + multi-chunk packing + TIME_*_RUN continuity; `docs/schemas/v6-string-dict-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md`. Not permanent pool/packing ADR. **Before full COL-007.** |
| `FMT-V6-STRING-DICT-MULTI-CHUNK-TIME-RUN-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Multi-chunk dict+packing with TIME_LINE_RUN/TIME_BLOCK_RUN; post-run site-delta across chunks; resolved strings; always-inflate EVENT/mixed NONE/ZLIB/ZSTD/LZ4. **Before full COL-007.** |
| `FMT-V6-AUTO-VERSION-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Auto-VERSION + multi-chunk packing continuity; `docs/schemas/v6-auto-version-multi-chunk-site-delta-seq-packing-provisional-v0.md`. Not dual-equality / permanent packing ADR. **Before full COL-007.** |
| `FMT-V6-AUTO-VERSION-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-MVP` | **done** | `encode_decoded_event_profile_auto_version_with_site_deltas_and_seq` + mixed sibling; multi-chunk packing with VERSION inject; TIME_*_RUN post-run across chunks; NONE/ZLIB/ZSTD/LZ4. **Before full COL-007.** |
| `FMT-V6-AUTO-VERSION-STRING-DICT-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Auto-VERSION + FOOTER dict + multi-chunk packing; `docs/schemas/v6-auto-version-string-dict-multi-chunk-site-delta-seq-packing-provisional-v0.md`. Not dual-equality / permanent pool/packing ADR. **Before full COL-007.** |
| `FMT-V6-AUTO-VERSION-STRING-DICT-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Auto-VERSION packing with FOOTER dict multi-chunk + TIME_*_RUN; always-inflate EVENT/mixed NONE/ZLIB/ZSTD/LZ4; VERSION mismatch + unknown id fail-closed. **Before full COL-007.** |
| `FMT-V6-MID-STREAM-CODEC-SWITCH-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Mid-stream codec-switch + packing continuity; `docs/schemas/v6-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md`. Not permanent packing ADR. **Before full COL-007.** |
| `FMT-V6-MID-STREAM-CODEC-SWITCH-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Shared PackingEncodeState pre→post; TIME_*_RUN post-run into post; always-inflate EVENT/mixed NONE→ZLIB/ZSTD/LZ4. **Before full COL-007.** |
| `FMT-V6-MID-STREAM-CODEC-SWITCH-STRING-DICT-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Mid-stream packing + FOOTER dict; `docs/schemas/v6-mid-stream-codec-switch-string-dict-site-delta-seq-packing-provisional-v0.md`. Not permanent pool/packing ADR. **Before full COL-007.** |
| `FMT-V6-MID-STREAM-CODEC-SWITCH-STRING-DICT-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Mid-stream packing + FOOTER dict; TIME_*_RUN post-run into post; always-inflate EVENT/mixed NONE→ZLIB/ZSTD/LZ4; unknown id fail-closed. **Before full COL-007.** |
| `FMT-V6-AUTO-VERSION-MID-STREAM-CODEC-SWITCH-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Auto-VERSION + mid-stream packing; `docs/schemas/v6-auto-version-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md`. Not dual-equality / permanent packing ADR. **Before full COL-007.** |
| `FMT-V6-AUTO-VERSION-MID-STREAM-CODEC-SWITCH-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Auto-VERSION mid-stream packing; TIME_*_RUN post-run into post; always-inflate EVENT/mixed NONE→ZLIB/ZSTD/LZ4. **Before full COL-007.** |
| `ADR-0001-V6-PACKING-CANDIDATE` | **done** (proposed ADR) | [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md): packing intent freeze candidate. **Not** wire freeze; **not** COL-007. **Before full COL-007.** |
| `DUAL-EQUALITY-READINESS-PROVISIONAL` | **done** | [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md): E1–E5 matrix + open gates. **Not** dual-equality product freeze. **Before full COL-007.** |
| `DUAL-EQUALITY-READINESS-MVP` | **done** | residual honesty sync for dual-equality readiness. COL-007 E3-EVENT done with C; E3-mixed residual. |
| `ADR-0002-V6-STRING-POOL-CANDIDATE` | **done** (proposed ADR) | [`docs/adrs/0002-v6-string-pool-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md). FOOTER-local; not global pool; not COL-007. **Before full COL-007.** |
| `E3-DUAL-EQUALITY-HARNESS-MVP` | **done** | `dual_equality` E3 harness + stand-in absolute/packing/string-dict/`expect_string_dict`/mid-stream packing tests. Stand-in **not** product dual-equality evidence. Product E3-EVENT with C is **done** (`e3_c_*` / `fixtures/v6/from-c/**`); E3-mixed residual. |
| `E4-V5-V6-SEMANTIC-EQUALITY-POLICY-PROVISIONAL` | **done** | [`docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md). Surfaces + packing policy; E4-v0 model enforcement ready (PR-B10). |
| `E4-V5-V6-SEMANTIC-EQUALITY-POLICY-MVP` | **done** | residual honesty for E4 policy. COL-007 E3-EVENT done with C; E4-v0 model + E4 product CLI ready (PR-B12b / **E4-PRODUCT-CLI-SMOKE-MVP**); full oracle dual residual (TEST-008). |
| `E4-V0-MODEL-SEMANTIC-MVP` | **done** (PR-B10) | Model-level v5↔v6 aggregate equality: `e4_v0_aggregates_equal`, fixtures `fixtures/e4/dual-sink/**`, `cargo test -p nytprof-model e4_v0_`, smoke `scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only`. Schema [`docs/schemas/e4-v0-model-semantic-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-v0-model-semantic-mvp-v0.md). **Residuals:** full oracle dual. Product CLI: **E4-PRODUCT-CLI-SMOKE-MVP**. |
| `CLI-E5-V6-OPT-IN-MVP` | **done** (PR-B12) | Full offline CLI product surfaces on v6 (report/html/csv/folded/callgrind/dump/verify); capability honesty `v6_decode`/`v6_report` true, convert/merge false, `collection_default: v5`; tests `cli_e5_v6` + capability_selftest; schema [`docs/schemas/cli-e5-v6-opt-in-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cli-e5-v6-opt-in-mvp-v0.md). **Residuals:** collection default flip (R4); convert/merge (PR-C01+). E4 product: **E4-PRODUCT-CLI-SMOKE-MVP**. |
| `E4-PRODUCT-CLI-SMOKE-MVP` | **done** (PR-B12b) | Real CLIs on v5+v6 dual-sink pairs: `e4_v5_v6_semantic_smoke.sh --full`; `cargo test -p nytprof-cli e4_product_`; offline_gate step 12 when native; schema [`docs/schemas/e4-product-cli-smoke-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-product-cli-smoke-mvp-v0.md). **Residuals:** full oracle dual (TEST-008); CLI v6 collection default; product `format=dual` rejected. |
| `E4-02-ORACLE-PAIR-MVP` | **done (MVP)** | Second oracle dual pair `blocks_calls1`; count surfaces only; not A4 780 attach / full TEST-008 / S2 |
| `E4-03-ORACLE-PAIR-MVP` | **done (MVP)** | Third oracle dual pair `calls2_default`; count surfaces only; not SUB_ENTRY 27 attach / full TEST-008 / S2 |
| `COL-007` | **done** (E3-EVENT) | C v6 writer product E3-EVENT with C: `fixtures/v6/from-c/**`, `crates/nytprof-format-v6/tests/e3_c.rs` (`e3_c_*`), `tools/oracle/e3_c_writer_parity.sh`, offline_gate step 11, schema [`docs/schemas/collector-v6-e3-c-fixtures-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-e3-c-fixtures-mvp-v0.md). **Residuals:** E3-mixed multi-kind C fixtures; full oracle E4 residual (E4-v0 + E4 product CLI ready); CLI v6 default; COL-008; live XS hooks. Wire freeze: **FMT-V6-WIRE-FREEZE**. Board flipped at PR-B09 — **not** at R2 packaging PR-B13. |
| `COL-009` | **done** (PR-B13) | Production v6 writer backend = C; COL-008 deferred. ADR [`docs/adrs/0007-production-v6-writer-backend-c-baseline.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0007-production-v6-writer-backend-c-baseline.md). |
| `R2-PREVIEW-READINESS-CUT` | **done** (PR-B13) | Release notes [`docs/RELEASE_NOTES_R2_PREVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_PREVIEW.md); dual-equality + residual + runbook honesty; opt-in only; convert/merge residual; R3/R4 not claimed. |
| `R2-P1P2-METHODOLOGY` | **done** (PR-C04) | P1/P2 methodology package; light_bench `size`/`collector_micro`; **no public perf claims** until R2-stable BENCH gates green. |
| `COL-008` | deferred | Batched Rust writer — non-baseline (reaffirmed ADR-0007 / COL-009) |
| `DROP-IN-DOD-V0` | **done (docs-landed)** | PR-G01 contract [`docs/contracts/DROP_IN_DOD_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md). **Not** live attach. |
| `PRODUCT-OPTIONS-MATRIX` | **done (docs + tests)** | G05 [`g05_options_format_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g05_options_format_smoke.sh): unknown + `format=dual` fail-closed; D1-B `format=v6` fail-closed (`v6_collect`, no `NYTPROF6`); D1-A `xs-nytprof-v6` writes `NYTPROF6`; v5 attach 15/3/15. **Not** EL8 RPM. |
| `PRODUCT-FORK-ADDPID-MVP` | **done (MVP)** | G06 [`g06_fork_addpid_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g06_fork_addpid_smoke.sh): live `fork` + `addpid=1`; parent + `<file>.<childpid>` `NYTProf 5`. **Not** TEST-018 / mid-deflate-in-child. |
| `G01-DESIGN-LAND` | **done** | [`docs/PRODUCT_COMPLETION_DROP_IN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PRODUCT_COMPLETION_DROP_IN_v0.md) + annex [`docs/schemas/product-xs-graft-annex-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-graft-annex-v0.md) |
| `G02-V5-PRODUCT-LINK` | **done (scaffold)** | `libnytp_sink_v5.a` + `-lz` probe + CollectorBootstrap load. Schema [`docs/schemas/product-xs-attach-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-attach-mvp-v0.md). **Not** D1 attach. |
| `G03A-LOAD-ONLY` | **done** | Product `perl -d:NYTProf` loads (in-memory sink; no `nytprof.out`). Smoke [`product_attach_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/product_attach_smoke.sh). **Not** collection attach. |
| `G03B-STMT-EMIT` | **done** | Statement emit via `nytp_emit_*` + fake-clock mini. Smoke [`g03b_stmt_emit_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03b_stmt_emit_smoke.sh). **Not** opcode attach / G04. |
| `G03C-SUB-EMIT` | **done** | Sub emit via `nytp_emit_sub_entry` / `sub_return`. Smoke [`g03c_sub_emit_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03c_sub_emit_smoke.sh). **Not** opcode attach / G04. |
| `G03D-META-EMIT` | **done** | Meta/finalize emit via `nytp_emit_attribute` / `option` / `new_fid` / `src_line` / `sub_info` / `pid_*`. Smoke [`g03d_meta_emit_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03d_meta_emit_smoke.sh). **Not** opcode attach / G04. |
| `G03E-COMPRESS-EMIT` | **done** | Compress emit via `nytp_emit_start_deflate`. Smoke [`g03e_compress_emit_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03e_compress_emit_smoke.sh). **Residual:** mid-deflate fork. Live attach is G04. |
| `PRODUCT-XS-ATTACH-MVP` | **done (MVP)** | Live `-d:NYTProf` + `file=`; E1b default `OP_ENTERSUB` (`$^P` 0x01 off); **E2 landed** `OP_GOTO` (g18). wrap remains `wrap=1`. Smoke [`g04_v5_parity_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g04_v5_parity_smoke.sh) leaf **15** / mid **3** / edge **15**. **Not** E3 leave / E4 full slowops. |
| `PRODUCT-GETOPT-COMPILE-MVP` | **done (MVP)** | PR-7: `INIT` `$DB::single` + `goto &$raw` for Exporter/Getopt/`vars`. Smoke [`g07_getopt_compile_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g07_getopt_compile_smoke.sh). **Not** Rocky ack retry / full opcode. |
| `PRODUCT-DATETIME-HINTS-MVP` | **done (MVP)** | PR-10: do **not** wrap `CORE::require`; preload BHES/Variable::Magic/namespace::* and `CvNODEBUG` before `$^P` 0x01 (`DB::sub` during `on_scope_end` breaks `%^H` / `DateTime::Duration`). Do **not** defer 0x01 to `INIT`. Smoke [`g10_datetime_hints_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g10_datetime_hints_smoke.sh). **Not** full opcode / XSUB. |
| `PRODUCT-NODEBUG-STASH-NOGP` | **done (MVP)** | PR-11: `DB::nodebug_stash` / `rebind_stash_slowops` skip GP-less stash GVs (`GvCV` SEGV). `CvISXSUB` skip before OP walk. Smoke [`g11_nodebug_stash_nogp_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g11_nodebug_stash_nogp_smoke.sh). **Not** full opcode / XSUB. |
| `PRODUCT-MEMOIZE-CALLER-MVP` | **done (MVP)** | PR-12: goto `Memoize::` so `memoize('fn')` does not look up `DB::fn` (`Cannot operate on nonexistent function`). Smoke [`g12_memoize_caller_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g12_memoize_caller_smoke.sh). **Not** full opcode / all caller-sensitive CPAN. |
| `PRODUCT-LOGGER-CALLER-MVP` | **done (MVP)** | PR-13: no `eval` around `&$raw` (loggers reported `NYTProfM.pm:308`). DESTROY emits on die. Smoke [`g13_logger_caller_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g13_logger_caller_smoke.sh). **Not** full opcode. |
| `PRODUCT-NESTED-EXCL-MVP` | **done (MVP)** | PR-14: exclusive = incl − Σ child inclusive; `stmts=0` skips TIME_LINE. Smoke [`g14_nested_excl_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g14_nested_excl_smoke.sh); g17 re-drives on default opcode. E2 times user `goto &sub` (g18). Wrap-path DateTime/Moo still skip via wrap list (`wrap=1` only). |
| `PRODUCT-DBSTATE-TIMELINE-MVP` | **done (MVP)** | PR-15: default `stmts=1` TIME_LINE from C `OP_DBSTATE`; `$DB::single=0`. Smoke [`g15_dbstate_timeline_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g15_dbstate_timeline_smoke.sh). Default calls are E1b opcode; wrap remains `wrap=1` only. **Not** NEXTSTATE / leave / full `slowops.h`. |
| `PRODUCT-WRAP-ENTER-MVP` | **done (MVP)** | PR-16 + E1b: `wrap=1` escape `wrap_push`/`wrap_pop`; `WRAP_SLOW` nested under that escape. Default attach is opcode (g17). Smoke [`g16_wrap_enter_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g16_wrap_enter_smoke.sh). Wrap list remains wrap=1 only (E2 opcode `OP_GOTO` is the default `goto &sub` path). **Not** stock 6.15 XS. |
| `PRODUCT-LEGACY-SMOKE` | **done (MVP)** | I01 [`install_product_xs.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/install_product_xs.sh) + [`product_legacy_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/product_legacy_smoke.sh): cargo-free D1-B install + live attach 15/3/15. **Not** BUILD-003-FULL / S2 dual_path / CPAN-TRIAL / EL8 RPM. |
| `I02-MAKEMAKER-NATIVE` | **done (MVP)** | [`i02_makemaker_native_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/i02_makemaker_native_smoke.sh): `NYTPROF_NATIVE=1` fail-closed without cargo; `=0`/`auto` cargo-free; cargo-present `make native-install` / `auto` `make all` → `nytprof-cli` report **15/3/15**. **Not** BUILD-003-FULL / S2. |
| `I03-DIST-SCRIPTS` | **done (MVP)** | [`install_product_scripts.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/install_product_scripts.sh) + [`i03_dist_scripts_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/i03_dist_scripts_smoke.sh): cargo-free EngineDispatch + `nytprof-engine` / `nytprofhtml` / `nytprofcsv`; installed `query --json --jsonl` **15/3/15**. Schema [`product-dist-scripts-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-dist-scripts-mvp-v0.md). **Not** 6.15 nytprofhtml DOM / COMPAT-007 / BUILD-003-FULL / S2. |
| `J01-CPAN-HYGIENE` | **done (MVP)** | [`j01_cpan_hygiene_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/j01_cpan_hygiene_smoke.sh): real `Makefile.PL` MYMETA **NYTProfM** **6.15** via `VERSION_FROM` product `.pm`; `MANIFEST.SKIP` excludes `baseline/` `target/` `prefix/`. Schema [`cpan-hygiene-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cpan-hygiene-mvp-v0.md). **Not** CPAN-TRIAL / PAUSE / BUILD-003-FULL / S2. |
| `MIG01-MIGRATION-GUIDE` | **done (docs)** | [`docs/MIGRATION_DROP_IN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/MIGRATION_DROP_IN_v0.md). **Not** CPAN-TRIAL / EL8 RPM ship. |
| `K03-PREBUILT-CLI-ADR` | **done (docs)** | [ADR-0010](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0010-signed-ci-prebuilt-native-cli.md) signed CI prebuilts for EL8 `nytprof-cli`. **Not** K02 spec / artifact pipeline. |
| `PRODUCT-V6-COLLECT-EL8` | **residual** | Default EL8 = D1-B; D1-A via `--with v6_collect`. |
| `BUILD-003-FULL` | **residual** | `full_build003=1` (I01–I02). Distinct from **BUILD-003-DEPTH** MVP. |
| `CPAN-TRIAL-READY` | **done (notes-ready / MVP)** | J02 [`RELEASE_NOTES_CPAN_TRIAL_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_CPAN_TRIAL_v0.md) + [`j02_cpan_trial_notes_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/j02_cpan_trial_notes_smoke.sh). **Not** PAUSE uploaded. |
| `EL8-RPM-MODULE` | **done (MVP)** | K01 [`perl-NYTProfM.spec`](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/perl-NYTProfM.spec) + [`k01_el8_module_rpm_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/k01_el8_module_rpm_smoke.sh). **Not** mock-certified / D1-A default / tools RPM. |
| `EL8-RPM-TOOLS` | **done (MVP)** | K02 [`nytprof-cli.spec`](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/nytprof-cli.spec) + [`k02_el8_tools_rpm_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/k02_el8_tools_rpm_smoke.sh). **Not** signed-pipeline complete / tools-alone drop-in. |
| `ROCKY8-DOCKER-PROFILE-LAB` | **done (MVP)** | Field lab [`rocky8_docker_profile_demo.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/rocky8_docker_profile_demo.sh) + smoke [`rocky8_docker_profile_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/rocky8_docker_profile_smoke.sh) (`--lab`, honest docker SKIP). Schema [`rocky8-docker-profile-lab-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/rocky8-docker-profile-lab-mvp-v0.md). **Not** `offline_gate` / mock-certified / ack attach. |
| `COMPLEX-APP-DOCKER-PROFILE-LAB` | **done (MVP)** | Rex + DateTime field lab [`complex_app_docker_profile.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/complex_app_docker_profile.sh) + smoke [`complex_app_docker_profile_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/complex_app_docker_profile_smoke.sh). Schema [`complex-app-docker-profile-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/complex-app-docker-profile-mvp-v0.md). **Not** `offline_gate` / SSH Rex. |
| `COMPLEX-APP-CATALOG` | **done (MVP)** | 20-app catalog + 10 diverse families + `--app` drivers. Tests [`t/complex_app_catalog.t`](https://github.com/hilather/nytprof-modernization/blob/main/t/complex_app_catalog.t) / [`t/attach_survival_failclosed.t`](https://github.com/hilather/nytprof-modernization/blob/main/t/attach_survival_failclosed.t). Findings [`complex-app-findings-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/complex-app-findings-v0.md). **Not** 20-app HTML sweep. |
| `P01-GA-CANDIDATE` | **done (MVP)** | [`RELEASE_NOTES_GA_CANDIDATE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_GA_CANDIDATE_v0.md) + [`p01_ga_candidate_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/p01_ga_candidate_smoke.sh). Rocky default D1-B only. **Not** SEC-012 complete / R3–R4 / S2. |
| `P02-SEC-CUT` | **done (MVP / checklist / job)** | [`SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md) + [`sec002_continuous_fuzz_mvp.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/sec002_continuous_fuzz_mvp.sh) + [`p02_sec_cut_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/p02_sec_cut_smoke.sh). **Not** independent sign-off / full continuous fuzz / GA marketing / S2. |
| `SEC-012-CHECKLIST-MVP` | **done (MVP / checklist)** | Reviewer checklist; **not** independent sign-off. |
| `SEC-002-CONTINUOUS-FUZZ-MVP` | **done (MVP / job)** | Workflow + wrapper invoke shipped `selftest_security_fuzz.sh` / `decode_fuzz`; honest `SKIP:` without cargo. **Not** cargo-fuzz / AFL / deep corpus. |
| `API-DATA-COMPAT007` | **residual** | Bless-array / COMPAT-007 until PERL-005. |

---

## Revision rule

Expanding or shrinking advertised readiness, or closing a residual row, requires a **matrix revision** (new vN or explicit amendment), board update, and linked evidence. This v0 is a **provisional readiness snapshot**, not release certification.


### Board IDs — R2-stable tooling (PR-C01 / PR-C02)

| Board ID | Status | Notes |
|----------|--------|-------|
| `TOOL-CONVERT-STRICT-MVP` | **done** (PR-C01) | Strict v5↔v6 convert; capability `convert: yes` |
| `TOOL-MERGE-REPACK-SALVAGE-MVP` | **done** (PR-C02) | Merge/repack/salvage; capability markers true |
| `TOOL-MERGE-AGGREGATE-SUM-MVP` | **done (MVP)** (L02) | Opt-in `--aggregate-sum`; concat default; not full nytprofmerge options |

## R2-stable ready + residual (PR-C05)

**Board ID:** `R2-STABLE-READINESS-CUT`  
**Release notes:** [`docs/RELEASE_NOTES_R2_STABLE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_STABLE.md)

| Item | Status under R2-stable |
|------|------------------------|
| Wire freeze ADR-0006 + vectors | **done** |
| Convert / merge / repack / salvage | **done** (PR-C01/C02); capability true; L01 lossy + L02 aggregate-sum MVP; full nytprofmerge options residual |
| COL-015 fork/PID MVP | **done** (PR-C02b stress); TEST-018 oracle residual |
| SEC-FUZZ offline package | **done** (PR-C03); P02 SEC-002 **job MVP** landed; full cargo-fuzz/AFL residual |
| P1/P2 methodology | **done** (PR-C04); **public claims waived** |
| E3-mixed multi-kind C | **done (MVP)** (`mixed.nytprof`); TEST-008 residual |
| E4-01 oracle dual pair | **done (MVP)** — `fixtures/e4/oracle-pair/` default-calls1 count surfaces; full TEST-008 residual |
| E4-02 oracle dual pair | **done (MVP)** — `blocks_calls1` count surfaces; not A4 780 attach / full TEST-008 |
| E4-03 oracle dual pair | **done (MVP)** — `calls2_default` count surfaces; not SUB_ENTRY 27 attach / full TEST-008 |
| L01 / `--allow-lossy` convert | **done (MVP)** — opt-in only; strict default; packing convert residual |
| L02 / `--aggregate-sum` merge | **done (MVP)** — opt-in only; stream-concat default; full nytprofmerge options residual |
| Full oracle E4 dual | **residual** (TEST-008) |
| COL-008 / R3 / R4 | deferred / not claimed |
| Dual-path legacy | unchanged |

| Board ID | Status | Notes |
|----------|--------|-------|
| `R2-STABLE-READINESS-CUT` | **done** (PR-C05) | Promote R2-preview → R2-stable honesty; integrate Phase C |
| `TOOL-CONVERT-STRICT-MVP` | **done** (PR-C01) | Strict convert; capability `convert: yes` |
| `TOOL-MERGE-REPACK-SALVAGE-MVP` | **done** (PR-C02) | Merge/repack/salvage |
| `TOOL-MERGE-AGGREGATE-SUM-MVP` | **done (MVP)** (L02) | Opt-in `--aggregate-sum`; not full nytprofmerge options |
| `COL-015-FORK-PID-MVP` | **done** (PR-C02b) | Fork protocol + stress MVP |
| `SEC-FUZZ-HARDENING-MVP` | **done** (PR-C03) | Offline security/fuzz package |
| `R2-P1P2-METHODOLOGY` | **done** (PR-C04) | Methodology only; no public SLOs |
| `DROP-IN-DOD-V0` | **done (docs-landed)** | PR-G01; attach/RPM/CPAN-TRIAL/BUILD-003-FULL **not** ready |
| `G02-V5-PRODUCT-LINK` | **done (scaffold)** | v5-only archive + load-only XS; **not** D1 attach |
| `G03A-LOAD-ONLY` | **done** | Product `-d:NYTProf` load; no `nytprof.out` on trivial `-e` |
| `G03B-STMT-EMIT` | **done** | `nytp_emit_*` statement path + fake-clock mini; no G04 fixture parity |
| `G03C-SUB-EMIT` | **done** | `nytp_emit_sub_*` call path; dump SUB_ENTRY / SUB_RETURN; no G04 fixture parity |
| `G03D-META-EMIT` | **done** | Meta/finalize emit; dump ATTRIBUTE / OPTION / NEW_FID / SRC_LINE / SUB_INFO / PID_START / PID_END; no G04 fixture parity |
| `G03E-COMPRESS-EMIT` | **done** | Compress emit via `nytp_emit_start_deflate`; dump inflate recovers post-deflate event; mid-deflate fork residual |
| `PRODUCT-XS-ATTACH-MVP` | **done (MVP)** | Live `-d:NYTProf` default-calls1 **15/3/15**; E1b default `OP_ENTERSUB`; **E2 landed** `OP_GOTO`. **Not** E3 leave / E4 full slowops |
| `PRODUCT-FORK-ADDPID-MVP` | **done (MVP)** | Live `fork` + `addpid=1` parent + `<file>.<pid>` `NYTProf 5` |


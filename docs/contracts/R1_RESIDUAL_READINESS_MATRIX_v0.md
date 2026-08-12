# R1 residual readiness matrix (provisional) — v0

**Status:** provisional readiness snapshot for **offline R0 / R1-preview** vs residual work for **full R1**  
**Board ID:** `R1-RESIDUAL-MATRIX` (honesty sync: `R1-HONESTY-SYNC`)  
**Date:** 2026-08-07  
**Depends on:** REPORT-CONTRACT-FREEZE, CI-OFFLINE-GATE, CAPABILITY-SELFTEST, ENGINE-AUTO-SMOKE, PERL-* JSONL / engine rows (incl. SUB_ENTRY multiplicity), NATIVE-AGG-JSON, **JSON-NATIVE-STREAM-MVP**, **JSON-TIME-BLOCK-MVP**, **JSON-REPORT-INCOMPLETE-FAILCLOSED**, **JSON-SUBDEF-SOURCE-MVP**, **JSON-META-FILES-MVP**, **JSON-EVENT-COUNTS-MVP**, **JSON-FILE-BASENAME-MVP**, **JSON-TOTAL-EVENTS-MVP**, **JSON-ATTR-BASETIME-MVP**, NATIVE-QUERY-JSON-CROSS / **NATIVE-QUERY-JSON-CROSS-EXPAND** / **NATIVE-QUERY-JSON-CROSS-BLOCKS** / **NATIVE-QUERY-JSON-CROSS-META** / **NATIVE-QUERY-JSON-CROSS-TIMEBLOCK** / **NATIVE-QUERY-JSON-CROSS-COUNTS** / **NATIVE-QUERY-JSON-CROSS-TOTAL**, JSON-SUB-ENTRY-MVP, JSON-BLOCKS-MVP, QUERY-JSON-*, BUILD-DUAL-PATH / BUILD-MAKEMAKER-OPT, DUMP-PARITY-EXPAND, DECODE-FUZZ-MVP, INCOMPLETE-STREAM, **FMT-V6-HEADER-PROVISIONAL** / **FMT-V6-HEADER-PARSE-MVP** / **FMT-V6-CHUNK-PROVISIONAL** / **FMT-V6-CHUNK-PARSE-MVP** / **FMT-V6-VARINT-PROVISIONAL** / **FMT-V6-VARINT-MVP** / **FMT-V6-SVARINT-PROVISIONAL** / **FMT-V6-SVARINT-MVP** / **FMT-V6-STRING-PROVISIONAL** / **FMT-V6-STRING-MVP** / **FMT-V6-TLV-PROVISIONAL** / **FMT-V6-TLV-MVP** / **FMT-V6-TLV-REGION-PROVISIONAL** / **FMT-V6-TLV-REGION-MVP** / **FMT-V6-FILE-PREFIX-PROVISIONAL** / **FMT-V6-FILE-PREFIX-MVP** / **FMT-V6-PREFIX-CHUNK-STREAM-PROVISIONAL** / **FMT-V6-PREFIX-CHUNK-STREAM-MVP** / **FMT-V6-EVENT-BODY-PROVISIONAL** / **FMT-V6-EVENT-BODY-MVP** / **FMT-V6-MINI-PROFILE-PROVISIONAL** / **FMT-V6-MINI-PROFILE-MVP** / **FMT-V6-MULTI-CHUNK-EVENT-PROVISIONAL** / **FMT-V6-MULTI-CHUNK-EVENT-MVP** / **FMT-V6-SOURCE-BODY-PROVISIONAL** / **FMT-V6-SOURCE-BODY-MVP** / **FMT-V6-INDEX-BODY-PROVISIONAL** / **FMT-V6-INDEX-BODY-MVP** / **FMT-V6-SUMMARY-BODY-PROVISIONAL** / **FMT-V6-SUMMARY-BODY-MVP** / **FMT-V6-FOOTER-BODY-PROVISIONAL** / **FMT-V6-FOOTER-BODY-MVP** / **FMT-V6-CRC-PROVISIONAL** / **FMT-V6-CRC-MVP** / **FMT-V6-PAYLOAD-ZLIB-PROVISIONAL** / **FMT-V6-PAYLOAD-ZLIB-MVP** / **FMT-V6-PAYLOAD-ZSTD-PROVISIONAL** / **FMT-V6-PAYLOAD-ZSTD-MVP** / **FMT-V6-PAYLOAD-LZ4-PROVISIONAL** / **FMT-V6-PAYLOAD-LZ4-MVP** / **FMT-V6-COMPRESSED-PROFILE-PROVISIONAL** / **FMT-V6-COMPRESSED-PROFILE-MVP** / **FMT-V6-MULTI-CHUNK-COMPRESSED-PROVISIONAL** / **FMT-V6-MULTI-CHUNK-COMPRESSED-MVP** / **FMT-V6-COMPRESSED-MIXED-PROVISIONAL** / **FMT-V6-COMPRESSED-MIXED-MVP** / **FMT-V6-PER-KIND-CODEC-PROVISIONAL** / **FMT-V6-PER-KIND-CODEC-MVP** / **FMT-V6-MULTI-CHUNK-KIND-PROVISIONAL** / **FMT-V6-MULTI-CHUNK-KIND-MVP** / **FMT-V6-MULTI-CHUNK-SOURCE-PROVISIONAL** / **FMT-V6-MULTI-CHUNK-SOURCE-MVP** / **FMT-V6-MULTI-CHUNK-INDEX-PROVISIONAL** / **FMT-V6-MULTI-CHUNK-INDEX-MVP** / **FMT-V6-MULTI-CHUNK-SUMMARY-PROVISIONAL** / **FMT-V6-MULTI-CHUNK-SUMMARY-MVP** / **FMT-V6-MID-RECORD-SPAN-PROVISIONAL** / **FMT-V6-MID-RECORD-SPAN-MVP** / **FMT-V6-MID-RECORD-SOURCE-PROVISIONAL** / **FMT-V6-MID-RECORD-SOURCE-MVP** / **FMT-V6-MID-RECORD-INDEX-PROVISIONAL** / **FMT-V6-MID-RECORD-INDEX-MVP** / **FMT-V6-MID-RECORD-SUMMARY-PROVISIONAL** / **FMT-V6-MID-RECORD-SUMMARY-MVP** / **FMT-V6-DECODED-CHUNK-PROVISIONAL** / **FMT-V6-DECODED-CHUNK-MVP** / **FMT-V6-DECODED-STREAM-PROVISIONAL** / **FMT-V6-DECODED-STREAM-MVP** / **FMT-V6-DECODED-EVENT-PROVISIONAL** / **FMT-V6-DECODED-EVENT-MVP** / **FMT-V6-DECODED-SOURCE-PROVISIONAL** / **FMT-V6-DECODED-SOURCE-MVP** / **FMT-V6-DECODED-INDEX-PROVISIONAL** / **FMT-V6-DECODED-INDEX-MVP** / **FMT-V6-DECODED-SUMMARY-PROVISIONAL** / **FMT-V6-DECODED-SUMMARY-MVP** / **FMT-V6-DECODED-MIXED-PROVISIONAL** / **FMT-V6-DECODED-MIXED-MVP** / **FMT-V6-DECODED-MIXED-MULTI-CHUNK-PROVISIONAL** / **FMT-V6-DECODED-MIXED-MULTI-CHUNK-MVP** / **FMT-V6-DECODED-MIXED-MID-RECORD-PROVISIONAL** / **FMT-V6-DECODED-MIXED-MID-RECORD-MVP** / **FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-PROVISIONAL** / **FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-MVP** (COL-007 runway preflight only), and related parity gates below  
**Gate:** done **before COL-007** (C v6 writer)

---

## Scope and non-claims

This matrix freezes what the first-slice program **advertises as ready** for offline developer preview (charter **R0**) and an **opt-in native v5 read/report R1-preview**, versus what remains **explicit residual** before a full charter **R1** product claim.

It is **not**:

- a release certification or CPAN readiness statement;
- a performance certification (see residual row; light bench only);
- a v6 wire freeze or collector-side completion claim;
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
| Feature-to-test inventory | [`baseline/inventories/feature-to-test-matrix.md`](https://github.com/hilather/nytprof-modernization/blob/main/baseline/inventories/feature-to-test-matrix.md) |

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
| Offline R1 gate | `scripts/ci/offline_gate.sh` | `make offline-gate` | cargo tests (honest skip) → harness → dual_path → engine_auto_fallback → **perl_jsonl_data_all** (incl. SUB_ENTRY multiplicity) → **perl_query_json** (CI-QUERY-JSON-GATE; required pure-Perl golden `--jsonl`) → **json_sub_entry** / **json_blocks** / **json_subdef_source** / **json_meta_files** / **json_time_block** (JSON-SUBDEF-SOURCE-MVP / **JSON-META-FILES-MVP** / **JSON-TIME-BLOCK-MVP** steps 6b–6i) → **native_agg_json** + **json_native_stream** + **json_report_incomplete** when native (**NATIVE-AGG-JSON** / **JSON-NATIVE-STREAM-MVP** / **JSON-REPORT-INCOMPLETE-FAILCLOSED**) → **native_query_json_cross** when native (**NATIVE-QUERY-JSON-CROSS** / **CROSS-EXPAND** / **CROSS-BLOCKS** / **CROSS-META** / **CROSS-TIMEBLOCK** / **CROSS-TOTAL**, shared fields incl. `sub_entry` on default-calls1 + calls2 **27** + blocks-calls1 **780**/**810** + `time_block_events` **0**/**916** + stream/PID + A9/A8 + meta samples on default-calls1) → capability_selftest when cargo/prefix/target present (CI-CAPABILITY-GATE; honest skip otherwise). **Not** multi-OS CI (**BUILD-006**) |
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

---

## Residual for full R1 (explicit)

These items are **not** advertised as ready under offline R0 / R1-preview. Do not imply product completeness until each residual is closed with its own board/ADR/plan evidence.

| Residual | Plan / board refs | Why residual | Preview honesty |
|----------|-------------------|--------------|-----------------|
| No production C ABI / FFI / cdylib | **RUST-010**, `nytprof-ffi` (charter crate list only) | No shipped stable native library ABI for embedders; Perl bridge is **subprocess CLI**, not FFI | Pure-Rust crates + CLI only |
| No XS ReadStream over binary profiles | **PERL-004** | Preview is dump-JSONL pure-Perl `JsonlReadStream` only | [`perl-jsonl-readstream-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-jsonl-readstream-mvp-v0.md) non-goal |
| No XS / bless-array Data materializer | **PERL-005** (+ COMPAT-007 shapes) | Preview is pure-Perl `JsonlData` query subset from dump JSONL | Not full `Devel::NYTProf::Data` fidelity |
| No full nytprofhtml DOM / REPORT-001..020 | **REPORT-001..020**, BASE-005, **REPORT-HTML-RESIDUAL-INV** | Native HTML is MVP summary + multi-file site; not oracle DOM/CSS/tablesorter/flame/Graphviz | REPORT_SURFACE_CONTRACT **not advertised** list; **artifact residual matrix:** [`docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md) (oracle vs native classes on default-calls1) |
| No v6 wire freeze | format plan / charter Phase-0 | Event contract provisional; no stable v6 numeric/wire IDs | v5 read/report path only |
| COL-001..006 sink/batch/v5-wire scaffold only (not product collector) | **COL-001-SINK-MVP** + **COL-002-LIFECYCLE-MVP** + **COL-003-SEQ-MVP** + **COL-004-FAST-PATH-MVP** + **COL-005-BATCH-MVP** + **COL-006-V5-WIRE-MVP** + **TEST-003-FAKE-CLOCK-MVP** board **done** (scaffold) | Overlay `collector/` semantic sink + lifecycle + gapless seq + **bounded batch + no-alloc stmt fast path** + **real v5 wire writer (zlib)** + fake-clock/M4 **mini** harness + unit tests + offline_gate step 10. **Residuals:** no live Perl/XS hooks; **full M4 oracle corpus** v5-via-sink under fake-clock **not** claimed (needs complete TEST-003); COL-015 full fork/signal/**file-switch**/enable-disable matrix open (OI-003-05); COL-003 internal logical seq excludes `START_DEFLATE` — does **not** close **OI-001-03** dump dual-index; I32 tick projection (OI-003-01) fail-closed not full overflow composition; flush/compression **discount timing** vs BASE-003 open; light microbench **not** BENCH certification; no dual/v6 product sink. Evidence: [`collector/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/collector/README.md), [`docs/schemas/collector-v5-wire-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v5-wire-mvp-v0.md), `scripts/packaging/collector_sink_smoke.sh` | R2 runway scaffolding; **not** a claim that collection is modernized in-process
| COL-007 / COL-008 deferred | **COL-007** board deferred; **COL-008** non-baseline | C v6 writer after report-side evidence; batched Rust writer deferred until dual-equality + ADR. **Preflight only:** provisional fixed-header + chunk-frame + ULEB128 + ZigZag signed + length-prefixed string + header TLV + multi-TLV region + file-prefix + prefix+chunk stream + event-body (incl. TIME_BLOCK/SUB_ENTRY/SUB_RETURN/SUB_INFO/SRC_LINE/NEW_FID/PID_START/PID_END/SUB_CALLERS/DISCOUNT/ATTRIBUTE/OPTION/COMMENT/START_DEFLATE/VERSION/dual-output-sequence/mid-stream-codec-switch/auto-emit-VERSION/known-key-attr-option/unknown-optional-skip) + mini-profile + multi-chunk EVENT + SOURCE/INDEX/SUMMARY/FOOTER bodies + CRC32 optional verify + ZLIB/ZSTD/LZ4 payload codecs + compressed multi-codec mini-profile + multi-chunk compressed EVENT + compressed multi-kind mixed + per-kind codecs + multi-chunk EVENT under mixed + multi-chunk SOURCE + multi-chunk INDEX + multi-chunk SUMMARY + mid-record EVENT/SOURCE/INDEX/SUMMARY span + decoded-chunk + decoded-stream + decoded-EVENT/SOURCE/INDEX/SUMMARY + decoded-mixed + multi-chunk decoded-mixed + mid-record decoded-mixed (EVENT+SOURCE+INDEX+SUMMARY + concurrent multi-kind) always-inflate (`FMT-V6-HEADER-*` / `FMT-V6-CHUNK-*` / `FMT-V6-VARINT-*` / `FMT-V6-SVARINT-*` / `FMT-V6-STRING-*` / `FMT-V6-TLV-*` / `FMT-V6-TLV-REGION-*` / `FMT-V6-FILE-PREFIX-*` / `FMT-V6-PREFIX-CHUNK-STREAM-*` / `FMT-V6-EVENT-BODY-*` / `FMT-V6-MINI-PROFILE-*` / `FMT-V6-MULTI-CHUNK-EVENT-*` / `FMT-V6-SOURCE-BODY-*` / `FMT-V6-INDEX-BODY-*` / `FMT-V6-SUMMARY-BODY-*` / `FMT-V6-FOOTER-BODY-*` / `FMT-V6-CRC-*` / `FMT-V6-PAYLOAD-ZLIB-*` / `FMT-V6-PAYLOAD-ZSTD-*` / `FMT-V6-PAYLOAD-LZ4-*` / `FMT-V6-COMPRESSED-PROFILE-*` / `FMT-V6-MULTI-CHUNK-COMPRESSED-*` / `FMT-V6-COMPRESSED-MIXED-*` / `FMT-V6-PER-KIND-CODEC-*` / `FMT-V6-MULTI-CHUNK-KIND-*` / `FMT-V6-MULTI-CHUNK-SOURCE-*` / `FMT-V6-MULTI-CHUNK-INDEX-*` / `FMT-V6-MULTI-CHUNK-SUMMARY-*` / `FMT-V6-MID-RECORD-SPAN-*` / `FMT-V6-MID-RECORD-SOURCE-*` / `FMT-V6-MID-RECORD-INDEX-*` / `FMT-V6-MID-RECORD-SUMMARY-*` / `FMT-V6-DECODED-CHUNK-*` / `FMT-V6-DECODED-STREAM-*` / `FMT-V6-DECODED-EVENT-*` / `FMT-V6-DECODED-SOURCE-*` / `FMT-V6-DECODED-INDEX-*` / `FMT-V6-DECODED-SUMMARY-*` / `FMT-V6-DECODED-MIXED-*` / `FMT-V6-DECODED-MIXED-MULTI-CHUNK-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-INDEX-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-SUMMARY-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-CONCURRENT-*` / `FMT-V6-EVENT-BODY-TIME-BLOCK-SUB-ENTRY-*` / `FMT-V6-EVENT-BODY-SUB-RETURN-SUB-INFO-*` / `FMT-V6-EVENT-BODY-SRC-LINE-NEW-FID-*` / `FMT-V6-EVENT-BODY-PID-START-END-*` / `FMT-V6-EVENT-BODY-SUB-CALLERS-DISCOUNT-*` / `FMT-V6-EVENT-BODY-ATTRIBUTE-OPTION-*` / `FMT-V6-EVENT-BODY-COMMENT-*` / `FMT-V6-EVENT-BODY-START-DEFLATE-*` / `FMT-V6-EVENT-BODY-VERSION-*`; schemas under `docs/schemas/v6-*-provisional-v0.md`; crate `nytprof-format-v6`) — **not** full COL-007 encoder / dictionaries / always-on inflate / CLI v6 default | Collector remains 6.15 oracle / v5; no default v6 CLI report |
| No full MakeMaker XS dual-build CPAN | **BUILD-003** (full) | Candidate `Makefile.PL` facade only (**BUILD-MAKEMAKER-OPT** done) | Not a complete XS CPAN tarball dual-build |
| No multi-OS CI matrix | **BUILD-006** | Single-host offline gate only | `offline_gate.sh` is not multi-OS CI |
| No performance certification claims | WP-13 / BENCH-001 | Light wall-time notes only | `docs/BENCH_NOTES.md`, `tools/bench/light_bench.sh` — **no public perf claims** |
| `engine=auto` full product policy / default flip | charter **R3**; ENGINE-AUTO-FALLBACK done for **Perl facade** | Perl `nytprof-engine`: prefer-native / fall-back-legacy is shipped. Residual: R3 product **default** flip + field window/ADR; pure-Rust `nytprof-cli` still maps `auto`→`native` (no in-process legacy) | Facade smokes prove dual-path auto; **not** “auto is the product default” |
| Default engine/format flips | charter R3/R4 | Explicitly out of first slice | Native remains opt-in |

---

## What “ready” means here

| Claim level | Meaning |
|-------------|---------|
| **Offline R0 / R1-preview ready** | Documented surfaces + gates above pass on this host when cargo/oracle fixtures are present; dual-path legacy still works without Cargo |
| **Full R1 ready** | Residual table closed (or explicitly waived by ADR) with product packaging, API materializers, report completeness, and certification policy as required by the plan DoD |
| **Not claimed** | Multi-OS CI green matrix, CPAN upload, performance SLOs, v6 collection, FFI ABI stability |

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
2. Residual table still lists **no production FFI/XS Data** (RUST-010, PERL-004/005), **no full nytprofhtml DOM** (REPORT-001..020 + HTML residual inventory), **no v6 / COL-007** (COL-007/008), **no multi-OS CI** (BUILD-006), **no perf claims**, R3 default flip, and Rust CLI `auto`→native residual.
3. `./scripts/ci/offline_gate.sh` still green (or cargo honestly skipped with harness + dual-path + expand + query-JSON steps green; native_agg + **native_query_json_cross** (incl. CROSS-EXPAND + CROSS-BLOCKS + CROSS-META) + capability skip only when no native CLI is available).  
4. Pure-Perl SUB_ENTRY multiplicity still green: `./scripts/packaging/perl_sub_entry_smoke.sh` (default **0** / calls2 **27**); JSON surfaces + cross (incl. blocks **780**/**810** + stream/PID + A9/A8 + meta samples): `./scripts/packaging/native_query_json_cross_smoke.sh` when native.  
5. Any **new** advertised surface requires a contract/matrix revision — do not silently expand “ready.”  
6. Board rows `R1-RESIDUAL-MATRIX` and `R1-HONESTY-SYNC` remain **done before COL-007** with evidence paths pointing at this file / runbook.  
7. Operator runbook [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) (`R1-PREVIEW-RUNBOOK`) still matches this matrix’s ready vs residual claims.

---

## Board placement

| ID | Status | Evidence |
|----|--------|----------|
| `R1-RESIDUAL-MATRIX` | done | this file (`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`) |
| `R1-PREVIEW-RUNBOOK` | done | [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) |
| `R1-HONESTY-SYNC` | **done** | this matrix + runbook re-synced to advertise **NATIVE-QUERY-JSON-CROSS** / **CROSS-EXPAND** / **CROSS-BLOCKS** / **CROSS-META** / **CROSS-TIMEBLOCK** / **CROSS-TOTAL** / **CROSS-COUNTS**, **JSON-EVENT-COUNTS-MVP**, **JSON-FILE-BASENAME-MVP** (absolute `file_1` volatile; basename greppable stable sample), **JSON-TIME-BLOCK-MVP**, **JSON-REPORT-INCOMPLETE-FAILCLOSED**, **JSON-SUB-ENTRY-MVP**, **JSON-BLOCKS-MVP**, **JSON-META-FILES-MVP**, + **PERL-SUB-ENTRY-JSONL** while retaining full-R1 residual honesty (no production FFI/XS Data, no full nytprofhtml DOM, no v6/COL-007, no multi-OS CI, no perf claims). Evidence: `scripts/packaging/native_query_json_cross_smoke.sh`, `scripts/packaging/json_time_block_smoke.sh`, `scripts/packaging/json_report_incomplete_smoke.sh`, `scripts/packaging/perl_sub_entry_smoke.sh`, `perl/t/jsonl_data_sub_entry.t`, `crates/nytprof-cli/tests/native_agg_json.rs`, offline_gate steps 6f–8 when native |
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
| `CI-OFFLINE-GATE` | done | `scripts/ci/offline_gate.sh` |
| `CI-OFFLINE-GATE-EXPAND` | done | offline_gate steps 4–5: `engine_auto_fallback_smoke` + `perl_jsonl_data_all_smoke` (incl. SUB_ENTRY) |
| `CI-QUERY-JSON-GATE` | done | offline_gate step 6: required `perl_query_json_smoke` (QUERY-JSON-MVP / QUERY-JSON-EXPAND golden `--jsonl`; no cargo) |
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
| `COL-007` | deferred | C v6 writer — unblocked for *start* after report-side evidence; not implemented here |
| `COL-008` | deferred | Batched Rust writer — non-baseline |

---

## Revision rule

Expanding or shrinking advertised readiness, or closing a residual row, requires a **matrix revision** (new vN or explicit amendment), board update, and linked evidence. This v0 is a **provisional readiness snapshot**, not release certification.

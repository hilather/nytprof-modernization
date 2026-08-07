# BASE-006 — Feature-to-test traceability matrix (Phase-0)

**Status:** first-slice inventory (not full plan BASE-006 machine-readable freeze)  
**Board ID:** `BASE-006-MATRIX`  
**Oracle:** Devel::NYTProf 6.15 (`baseline/6.15/`, tag `v6.15`)  
**Depends on:** BASE-002 (v5 records), BASE-004 (`perl-api-surface.md`), BASE-005 (`cli-report-surface.md`), FIXTURE-0  
**Date:** 2026-08-07

## Purpose

Map **BASE-004 / BASE-005 public surfaces** to **existing fixtures, oracle selftests, cargo tests, and packaging smokes**. Goal: make coverage and honest gaps visible before deeper PERL/REPORT/FFI work.

This is **not**:

- Full plan `BASE-006` machine-readable coverage JSON
- A claim that every inventory method has a dedicated test
- Full `nytprofhtml` DOM / CSS / flame parity

## Primary fixtures

| Fixture | Path | NYTPROF options (sans `file=`) | Role |
|---------|------|--------------------------------|------|
| **default-calls1** (primary) | `fixtures/v5/default-calls1/` | `trace=0:start=begin:calls=1` | Main golden: `TIME_LINE`, mid/leaf returns 15/3, mid→leaf edge 15 |
| default-calls2 | `fixtures/v5/default-calls2/` | `trace=0:start=begin:calls=2` | Richer call-site (`SUB_ENTRY`); same workload |
| **calls2-default** | `fixtures/v5/calls2-default/` | `trace=0:start=begin:calls=2` | Independent `calls=2` capture (FIXTURE-EXPAND-2); dump parity set |
| **blocks-calls1** | `fixtures/v5/blocks-calls1/` | `trace=0:start=begin:calls=1:blocks=1` | `TIME_BLOCK` path; A4 line_totals + A4b `block_line_totals` |

Per-fixture artifacts used by tests:

| Artifact | Purpose |
|----------|---------|
| `nytprof.out` | Binary profile (native decode / model / CLI) |
| `readstream.jsonl` | Oracle ReadStream dump (golden event stream) |
| `aggregates.oracle.json` | Generated aggregate baseline (A1–A9) |
| `workload.pl` | Profiled script (3×`mid` × 5×`leaf`) |

Capture: `./tools/oracle/capture_fixture.sh <name> "<opts>"`  
Layout notes: [`fixtures/README.md`](../../fixtures/README.md)

## Contracts / inventories linked

| Doc | Role |
|-----|------|
| [`perl-api-surface.md`](perl-api-surface.md) | BASE-004 Perl API dispositions |
| [`cli-report-surface.md`](cli-report-surface.md) | BASE-005 CLI/report dispositions |
| [`docs/schemas/canonical-event-dump-v0.md`](../../docs/schemas/canonical-event-dump-v0.md) | Dump JSONL schema |
| [`docs/schemas/aggregate-comparison-v0.md`](../../docs/schemas/aggregate-comparison-v0.md) | Aggregate A1–A9 |
| [`docs/schemas/report-semantic-parity-mvp-v0.md`](../../docs/schemas/report-semantic-parity-mvp-v0.md) | Leaf/mid/edge semantic checklist |
| [`docs/schemas/native-dump-parity-mvp-v0.md`](../../docs/schemas/native-dump-parity-mvp-v0.md) | CLI dump vs golden JSONL structural equality |
| [`docs/schemas/html-*-mvp-v0.md`](../../docs/schemas/) | Native HTML shape (not oracle DOM) |
| [`docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md`](../../docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md) | Logical events |
| [`docs/contracts/COMPAT-002_VOLATILE_NORMALIZATION.md`](../../docs/contracts/COMPAT-002_VOLATILE_NORMALIZATION.md) | Structural normalize |
| [`docs/contracts/COMPAT-003_PRECISION_NUMERIC_POLICY.md`](../../docs/contracts/COMPAT-003_PRECISION_NUMERIC_POLICY.md) | Tick / float policy |
| [`docs/contracts/COMPAT-004_SURFACE_CLASSIFICATION.md`](../../docs/contracts/COMPAT-004_SURFACE_CLASSIFICATION.md) | Surface class × native disposition (provisional) |

---

## Feature → test matrix

Columns: **Surface** | **Fixture/path** | **Covered by (tests/scripts)** | **Gaps**

### 1. Data (Perl / aggregates model)

Maps BASE-004 `Devel::NYTProf::Data` (mapped contract) and compact model aggregates A1–A9 — **not** full Perl object materializer.

| Surface | Fixture/path | Covered by (tests/scripts) | Gaps |
|---------|--------------|----------------------------|------|
| Aggregate model A1–A9 from binary | `fixtures/v5/default-calls1/nytprof.out` + `aggregates.oracle.json` | Cargo: `default_calls1_native_matches_aggregates_oracle_json` (`nytprof-model`); also `default_calls1_binary_matches_oracle_jsonl`, `default_calls1_workload_subs`, `default_calls1_call_edges_and_source`, `default_calls1_sub_defs`. Scripts: `tools/oracle/selftest_aggregates.sh`, `tools/oracle/compare_native_aggregates.sh`, harness via `selftest_harness.sh` | No live Perl `Data->new` round-trip tests in-repo; no `package_subinfo_map` / eval collapse |
| Same on calls2 | `fixtures/v5/default-calls2/` | Cargo: `default_calls2_native_matches_aggregates_oracle_json`, `default_calls2_binary_matches_oracle_jsonl`; `selftest_aggregates.sh` | Same Perl Data gap |
| Block/line totals (A4 + A4b) | `fixtures/v5/blocks-calls1/` | Cargo: `blocks_calls1_native_matches_aggregates_oracle_json`, `blocks_calls1_binary_matches_oracle_jsonl`, `blocks_calls1_workload_subs`, `blocks_calls1_sub_defs`, `accumulate_time_block_fills_line_and_block_totals`; `selftest_aggregates.sh` | No Perl `get_fid_line_data` / FileInfo `line_time_data` parity suite |
| Synthetic accumulate unit paths | (in-memory events) | Cargo: `accumulate_single_time_line`, `accumulate_sub_return`, `accumulate_sub_callers_merges_sites`, `accumulate_src_line_last_write_wins`, `accumulate_sub_info_last_write_wins`, `from_path_truncated_profile_errors` | Not fixture-driven for every tag |
| Oracle re-aggregate from JSONL | `*/readstream.jsonl` → `aggregates.oracle.json` | `tools/oracle/aggregate_from_jsonl.py`; `selftest_aggregates.sh` (re-aggregate ≡ committed baseline) | Python helper only — not XS Data loader |
| Report text summary from model | `default-calls1` | Cargo: `summary_default_calls1_real_render_path` (`nytprof-report`); CLI `nytprof-cli report` / packaging smokes | Not full Data.pm dump_profile_data |

### 2. ReadStream (oracle dump / native dump)

Maps BASE-004 `Devel::NYTProf::ReadStream` and native stream dump equality.

| Surface | Fixture/path | Covered by (tests/scripts) | Gaps |
|---------|--------------|----------------------------|------|
| Oracle ReadStream dump capture | `fixtures/v5/*/readstream.jsonl` | `tools/oracle/dump_readstream.pl` + `capture_fixture.sh`; committed goldens | Capture requires oracle env (`tools/oracle/env.sh`) |
| Structural normalize + compare | `default-calls1` (+2, blocks when present) | `tools/oracle/normalize_jsonl.py`, `compare_jsonl.pl`; **`selftest_harness.sh`** (identity, tag flip, tick mutate, volatiles); **`selftest_normalize_compat.sh`** (COMPAT-002/003) | Full option-matrix predicates still open (BASE-005 / COMPAT-004) |
| Native dump vs oracle JSONL (via model/binary) | `default-calls1`, `default-calls2`, `blocks-calls1` | Cargo: `*_binary_matches_oracle_jsonl` (`nytprof-model`); format-v5: `decode_default_calls1`, `decode_default_calls2`; CLI `nytprof-cli dump` / default path | Model-level equality ≠ full CLI stdout path (see next row) |
| **Shipped CLI dump structural parity** | `default-calls1`, `calls2-default`, `blocks-calls1` | **`tools/oracle/selftest_native_dump_parity.sh`** (optional fixture args; default default-calls1) + **`selftest_native_dump_parity_all.sh`** (all three); dump×2 + normalize + `compare_jsonl.pl` full match + per-fixture TIME_LINE/TIME_BLOCK/SUB_RETURN multiplicity; nested from `selftest_harness.sh` via `_all.sh`; optional cargo: `native_dump_tag_counts_match_golden_*` (`nytprof-format-v5`); schema `native-dump-parity-mvp-v0.md` | Board DUMP-PARITY-EXPAND; multiplicity from each fixture golden (blocks uses TIME_BLOCK, not default-calls1 counts) |
| Decode robustness | truncated / malformed bytes | Cargo format-v5: `decode_empty_input_errors`, `decode_bad_header_errors`, `decode_truncated_*`, `decode_garbage_tag_*`, `unknown_binary_tag_errors`; model `from_path_truncated_profile_errors` | Not every corrupt tag shape |
| Decode/verify fuzz MVP (DECODE-FUZZ-MVP) | empty / bad magic / half + stepped prefixes + ~32–64 XOR flips of default-calls1 | Cargo: `decode_fuzz_no_panic_*`, `fuzz_truncated_mutations` (`nytprof-format-v5`); `decode_fuzz_no_panic_verify_*`, `fuzz_truncated_mutations_verify` (`nytprof-report`); smoke `tools/oracle/selftest_decode_fuzz.sh`; schema `docs/schemas/decode-fuzz-mvp-v0.md` | Deterministic battery only — not full SEC-002 continuous fuzz |
| Packaging oracle dump smoke | `default-calls1/nytprof.out` | `scripts/packaging/legacy_only_smoke.sh` (optional `dump_readstream.pl` line count); `perl_engine_dispatch_smoke.sh` legacy bridge | Optional / skip when oracle absent |

### 3. nytprofhtml / native html

Maps BASE-005 `nytprofhtml` → native `nytprof-cli html` (MVP).

| Surface | Fixture/path | Covered by (tests/scripts) | Gaps |
|---------|--------------|----------------------------|------|
| Semantic leaf/mid/edge (counts) | `fixtures/v5/default-calls1/` | Cargo: **`report_semantic_parity_default_calls1`** (`nytprof-report`); smoke: **`tools/oracle/report_semantic_parity.sh`** (oracle `nytprofhtml` under isolated PERL5LIB + native `-o` / `--out-dir`) | **No full nytprofhtml DOM/CSS/JS/tablesorter/flame parity**; ticks not asserted for parity (COMPAT-003) |
| Blocks semantic (A4 line5 + leaf/mid) | `fixtures/v5/blocks-calls1/` | Cargo: **`blocks_semantic_parity_blocks_calls1`** (`nytprof-report`); smoke: **`tools/oracle/blocks_semantic_parity.sh`** (native `report` + `html -o` / `--out-dir`); schema `docs/schemas/blocks-semantic-parity-mvp-v0.md` | Exact **780**/15/3; not full DOM; ticks under COMPAT-003 only |
| Single-file HTML summary | `default-calls1` | Cargo: `html_summary_default_calls1_real_render_path`, `escape_html_basic`, `html_escapes_angle_brackets_in_source` | Layout not oracle-identical |
| Multi-file HTML site | `default-calls1` | Cargo: `html_site_default_calls1_render_html_site`, `write_html_site_default_calls1_tempdir` | No graphviz `.dot`, no flame SVG, no block/sub-level pages like full nytprofhtml |
| HTML out-dir path safety | `default-calls1` | Cargo: `write_html_site_rejects_dotdot_component`, `write_html_site_rejects_null_byte`, `write_html_site_rejects_empty_path`; atomic safe-path tests | Not full chroot/sandbox; absolute OK without `..`/`\0` |
| Blocks: A4 line calls in HTML | `blocks-calls1` | Cargo: `html_summary_blocks_calls1_line_calls`, `html_site_blocks_calls1_source_line_calls`, **`blocks_semantic_parity_blocks_calls1`** | Not full REPORT-006 block pages |
| Blocks: A4b block_line_totals in HTML | `blocks-calls1` | Cargo: `html_summary_blocks_calls1_block_line_totals`, `html_site_blocks_calls1_block_line_totals` | Oracle HTML block UI not compared |
| HTML schemas (docs) | — | `docs/schemas/html-report-mvp-v0.md`, `html-multifile-mvp-v0.md`, `html-per-file-mvp-v0.md`, `html-outdir-safety-mvp-v0.md`, `blocks-semantic-parity-mvp-v0.md` | Schemas are MVP contracts, not test runners |

### 4. nytprofcsv / native csv

Maps BASE-005 `nytprofcsv` (Reader dialect) → native `nytprof-cli csv` (subs/edges).

| Surface | Fixture/path | Covered by (tests/scripts) | Gaps |
|---------|--------------|----------------------------|------|
| Native subs CSV (A5 fields) | `default-calls1` | Cargo: `subs_csv_default_calls1_real_render`, `csv_report_dual_section`, `csv_escape_quotes_when_needed`; gate **`csv_semantic_parity_default_calls1`** | **Not** legacy Reader per-line CSV layout |
| Native edges CSV (A7 fields) | `default-calls1` | Cargo: `edges_csv_default_calls1_real_render`; gate **`csv_semantic_parity_default_calls1`** | mid→leaf count 15 asserted; not all edge sites |
| CSV semantic parity (board CSV-SEMANTIC-PARITY) | `default-calls1` | Schema `docs/schemas/csv-semantic-parity-mvp-v0.md`; Cargo `csv_semantic_parity_default_calls1`; smoke `tools/oracle/csv_semantic_parity.sh` (csv ×2 + harness step) | Exact leaf **15** / mid **3** / mid→leaf **15**; ticks not compared |
| Oracle nytprofcsv spot-check | `default-calls1` | Optional non-fatal in `legacy_only_smoke` / `perl_engine_dispatch` when `nytprofcsv` present | **No automated byte or dialect parity** to Reader CSV; `--delim` / `--annotated` untested on native |

### 5. Dump (native dump parity)

Maps stream dump path used for differential equality (native CLI + oracle ReadStream). Board: **NATIVE-DUMP-PARITY**, **DUMP-PARITY-EXPAND**.

| Surface | Fixture/path | Covered by (tests/scripts) | Gaps |
|---------|--------------|----------------------------|------|
| Native CLI dump vs golden JSONL | `default-calls1`, `calls2-default`, `blocks-calls1` (`nytprof.out` + `readstream.jsonl`) | **`selftest_native_dump_parity.sh`** [fixture…]; **`selftest_native_dump_parity_all.sh`**; harness nests `_all.sh`; CLI `dump`; cargo `native_dump_tag_counts_match_golden_{default_calls1,calls2_default,blocks_calls1}` | Full structural gate on three fixtures; other captures (e.g. default-calls2) optional later |
| Model/binary dump equality | default-calls1/2, blocks-calls1 | Cargo `*_binary_matches_oracle_jsonl`; format-v5 decode tests | Distinct from shipped-CLI stdout path |
| Dump + bench wall-time | `default-calls*` | `tools/bench/light_bench.sh` | Not a correctness gate |
| Engine flag on dump/report | `default-calls1` | Cargo engine unit tests (`default_is_native`, `flag_overrides_env`, …); `scripts/packaging/engine_select_smoke.sh` | `legacy` engine exits 2 (not wired) |
| Verify/inspect health | `default-calls1` | Cargo: `verify_profile_default_calls1_ok`, `verify_profile_truncated_default_calls1_err`; CLI `verify` / `inspect` | Not full profile salvage |
| Fail-closed corrupt input (COMPAT-010-ERR) | empty / half default-calls1 / `NOTPROF 5 0\n` | Cargo: `fail_closed_*` (`nytprof-report`, `nytprof-cli` tests); format-v5 `decode_*_errors`; model `from_path_truncated_profile_errors`; `tools/oracle/selftest_fail_closed.sh` | Not full COMPAT-010 taxonomy; no salvage |
| Incomplete stream fail-closed (INCOMPLETE-STREAM) | first 500 bytes of default-calls1 (record-aligned short prefix) | Cargo: `verify_profile_incomplete_prefix_*`, `stream_completeness_*`, `incomplete_stream_cli_*`; smoke `tools/oracle/selftest_incomplete_stream.sh`; salvage `NYTPROF_ALLOW_INCOMPLETE=1` | Dump may stay lenient; not full SEC recovery |

### 6. Related export surfaces (BASE-005 partial)

| Surface | Fixture/path | Covered by (tests/scripts) | Gaps |
|---------|--------------|----------------------------|------|
| Callgrind-ish export | `default-calls1` | Cargo: `callgrind_default_calls1_real_render`, named gate `export_semantic_parity_default_calls1`; CLI `callgrind` / `cg`; smoke `tools/oracle/export_semantic_parity.sh` | Not full Valgrind tool acceptance; not byte-identical `nytprofcg` |
| Folded stacks | `default-calls1` | Cargo: `folded_stacks_default_calls1_real_render`, named gate `export_semantic_parity_default_calls1`; CLI `folded`; smoke `tools/oracle/export_semantic_parity.sh` | Not full `nytprofcalls` dialect / multi-file merge |
| Export semantic parity (board EXPORT-SEMANTIC-PARITY) | `default-calls1` | Schema `docs/schemas/export-semantic-parity-mvp-v0.md`; Cargo `export_semantic_parity_default_calls1`; smoke `tools/oracle/export_semantic_parity.sh` (folded + callgrind/cg ×2 + harness step) | Exact leaf **15** / mid **3** / mid→leaf **15**; ticks not compared |
| nytprofmerge | — | — | **No native merge**; legacy-only (OI-BASE005 / FileHandle writer) |
| flamegraph.pl | — | only via oracle nytprofhtml optional path in full product | **No native flame**; not first-slice |

### 7. Packaging / dual-path (cross-cutting)

| Surface | Fixture/path | Covered by (tests/scripts) | Gaps |
|---------|--------------|----------------------------|------|
| Packaging fail-fast gate | `default-calls1` | `scripts/packaging/packaging_gate.sh` → legacy_only → engine_select → perl_engine_dispatch → install_native + native_install_smoke → native_optional_smoke → capability_selftest (when cargo/prefix/target CLI) | Not multi-OS CI matrix (BUILD-006) |
| Native capability self-test | `default-calls1` (optional probe) | CLI `capability` / `selftest` / `capabilities` (+ `--json` / `--format=json`); smoke `scripts/packaging/capability_selftest_smoke.sh` (human×2 + JSON×2 + markers); schema `capability-selftest-mvp-v0.md` | Not full BUILD-005 codec/ABI manifest |
| Dual-path policy | — | `scripts/packaging/dual_path_smoke.sh`; `docs/BUILD_SUPPORT_POLICY.md` | Full MakeMaker dual-build (BUILD-003) out of scope |
| Candidate MakeMaker entry | — | root `Makefile.PL`; `makemaker_dual_path_smoke.sh`; `make legacy-smoke` / `dual-path-smoke` / `native-install` | Not full XS CPAN; multi-OS CI out of scope |
| Perl engine dispatch | `default-calls1` | `perl/t/engine_dispatch.t` (if run); `perl_engine_dispatch_smoke.sh`; native bridge finds prefix/target/cargo | **No PERL-004 XS ReadStream**; no FFI; no full API facade |
| Native install prefix | `default-calls1` | `install_native.sh`, `native_install_smoke.sh` | Prebuilts not shipped |

---

## Cargo test name index (by crate)

Useful for `cargo test -p <crate> <filter>`.

### `nytprof-model` (`crates/nytprof-model/src/model_tests.rs`)

| Pattern / name | Surface |
|----------------|---------|
| `default_calls1_native_matches_aggregates_oracle_json` | Data/aggregates |
| `default_calls2_native_matches_aggregates_oracle_json` | Data/aggregates |
| `blocks_calls1_native_matches_aggregates_oracle_json` | Data/aggregates + blocks |
| `default_calls1_binary_matches_oracle_jsonl` | Dump/model vs oracle stream |
| `default_calls2_binary_matches_oracle_jsonl` | Dump/model |
| `blocks_calls1_binary_matches_oracle_jsonl` | Dump/model + blocks |
| `default_calls1_workload_subs` / `blocks_calls1_workload_subs` | A5/A6 |
| `default_calls1_call_edges_and_source` | A7/A8 |
| `default_calls1_sub_defs` / `blocks_calls1_sub_defs` | A9 |
| `accumulate_*` / `from_path_truncated_profile_errors` | Unit aggregate / errors |

### `nytprof-report` (`crates/nytprof-report/src/lib.rs` tests)

| Pattern / name | Surface |
|----------------|---------|
| `report_semantic_parity_default_calls1` | HTML + model semantic (default-calls1 leaf/mid/edge) |
| `blocks_semantic_parity_blocks_calls1` | HTML + model semantic (blocks-calls1 line5=780, leaf=15, mid=3) |
| `html_summary_default_calls1_*` / `html_site_default_calls1_*` / `write_html_site_default_calls1_*` | native html |
| `html_*_blocks_calls1_*` | native html + blocks |
| `subs_csv_*` / `edges_csv_*` / `csv_report_*` | native csv |
| `summary_default_calls1_*` | report text |
| `verify_profile_default_calls1_*` | verify CLI |
| `fail_closed_*` (report + cli) | COMPAT-010-ERR fail-closed |
| `decode_fuzz_no_panic_verify_*` / `fuzz_truncated_mutations_verify` | DECODE-FUZZ-MVP verify no-panic battery |
| `verify_profile_incomplete_prefix_*` / `stream_completeness_*` / `incomplete_stream_cli_*` | INCOMPLETE-STREAM fail-closed |
| `tools/oracle/selftest_incomplete_stream.sh` | INCOMPLETE-STREAM smoke |
| `tools/oracle/selftest_decode_fuzz.sh` | DECODE-FUZZ-MVP smoke |
| `callgrind_default_calls1_*` / `folded_stacks_default_calls1_*` | exports |
| `export_semantic_parity_default_calls1` | EXPORT-SEMANTIC-PARITY named gate (model + folded + callgrind) |
| `tools/oracle/export_semantic_parity.sh` | EXPORT-SEMANTIC-PARITY smoke |

### `nytprof-format-v5`

| Pattern / name | Surface |
|----------------|---------|
| `decode_default_calls1` / `decode_default_calls2` | binary decode |
| `native_dump_tag_counts_match_golden_default_calls1` | dump tag multiplicity vs golden |
| `native_dump_tag_counts_match_golden_calls2_default` | DUMP-PARITY-EXPAND calls2-default |
| `native_dump_tag_counts_match_golden_blocks_calls1` | DUMP-PARITY-EXPAND blocks-calls1 (TIME_BLOCK) |
| `decode_*_errors` / `unknown_binary_tag_errors` | robust decode |
| `decode_fuzz_no_panic_*` / `fuzz_truncated_mutations` | DECODE-FUZZ-MVP decode no-panic battery |
| varint unit tests | encoding helpers |

### `nytprof-cli` (`engine.rs`)

| Pattern / name | Surface |
|----------------|---------|
| `default_is_native`, `flag_overrides_env`, `peel_engine_*`, … | engine selection |

---

## Script index

| Script | What it covers |
|--------|----------------|
| `tools/oracle/selftest_harness.sh` | Normalize identity/mutations/volatiles on default-calls1 (+2, blocks, calls2); nests normalize_compat + aggregates + native_dump_parity_all |
| `tools/oracle/selftest_compare.sh` | Alias → harness |
| `tools/oracle/selftest_aggregates.sh` | Re-aggregate ≡ `aggregates.oracle.json` |
| `tools/oracle/selftest_normalize_compat.sh` | COMPAT-002 structural + COMPAT-003 float dump |
| `tools/oracle/selftest_native_dump_parity.sh` | Shipped CLI dump vs golden JSONL; args = fixture names (default default-calls1) |
| `tools/oracle/selftest_native_dump_parity_all.sh` | DUMP-PARITY-EXPAND: default-calls1 + calls2-default + blocks-calls1 |
| `tools/oracle/selftest_fail_closed.sh` | COMPAT-010-ERR corrupt input CLI fail-closed |
| `tools/oracle/selftest_incomplete_stream.sh` | INCOMPLETE-STREAM short-prefix fail-closed |
| `tools/oracle/selftest_decode_fuzz.sh` | DECODE-FUZZ-MVP cargo decode/verify fuzz batteries |
| `tools/oracle/report_semantic_parity.sh` | Oracle nytprofhtml + native html semantic counts |
| `tools/oracle/compare_native_aggregates.sh` | Wraps cargo `native_matches_aggregates_oracle_json` |
| `tools/oracle/dump_readstream.pl` | Oracle ReadStream → JSONL |
| `tools/oracle/normalize_jsonl.py` / `compare_jsonl.pl` / `aggregate_from_jsonl.py` | Dump normalize/compare/aggregate helpers |
| `scripts/packaging/packaging_gate.sh` | Ordered packaging smokes |
| `scripts/packaging/legacy_only_smoke.sh` | Oracle isolation, no crates on PERL5LIB |
| `scripts/packaging/engine_select_smoke.sh` | `--engine` / `NYTPROF_ENGINE` |
| `scripts/packaging/engine_auto_smoke.sh` | ENGINE-AUTO-SMOKE: `--engine=auto` / `NYTPROF_ENGINE=auto` → native leaf=15/mid=3 when native present |
| `scripts/packaging/engine_auto_fallback_smoke.sh` | ENGINE-AUTO-FALLBACK: auto prefer-native; FORCE_NO_NATIVE → legacy exit 0 |
| `scripts/packaging/perl_engine_dispatch_smoke.sh` | Perl facade native + legacy |
| `scripts/packaging/native_optional_smoke.sh` | cargo tests when cargo present |
| `scripts/packaging/native_install_smoke.sh` / `install_native.sh` | prefix install |
| `scripts/packaging/capability_selftest_smoke.sh` | CAPABILITY-SELFTEST + CAPABILITY-JSON-MVP (capability×2 + --json×2) |
| `scripts/packaging/dual_path_smoke.sh` | BUILD dual-path policy |
| `tools/bench/light_bench.sh` | Offline wall-time (not correctness) |

---

## Honest open gaps

These are **known untested or out-of-scope** for first-slice; do not mark closed without new evidence.

| ID | Gap | Related |
|----|-----|---------|
| G-006-01 | **No PERL-004 XS ReadStream** facade over native decoder — only oracle `ReadStream.pm` + Rust dump | BASE-004, PERL-* |
| G-006-02 | **No full Perl `Data` / FileInfo / SubInfo materializer** parity tests | BASE-004 mapped methods |
| G-006-03 | **No full nytprofhtml DOM / CSS / JS / flame / graphviz parity** — semantic counts only | BASE-005, REPORT-001..020 |
| G-006-04 | **No merge** (`nytprofmerge` / FileHandle writer) native path or tests | BASE-005, TOOL-009 |
| G-006-05 | **No FFI** / batched Rust writer (COL-008 deferred); no per-event FFI | COL-007/008 |
| G-006-06 | Native CSV is **subs/edges**, not Reader **line-level** `nytprofcsv` dialect | BASE-005 OI-BASE005-02 |
| G-006-07 | **No option-matrix** for full NYTPROF env / CLI flag cartesian product | BASE-005, COMPAT-004 |
| G-006-08 | **No fork / addpid / multi-PID** fixture in first-slice corpus | BASE-007 |
| G-006-09 | **No eval-heavy / savesrc / compress** fixture matrix in this inventory | BASE-002 open items |
| G-006-10 | Tick/time **display** not part of report semantic parity (counts only) | COMPAT-003 |
| G-006-11 | Machine-readable BASE-006 JSON / full plan acceptance not produced here | plan BASE-006 |
| G-006-12 | Upstream oracle `t/*.t` suite not re-mapped row-by-row to native owners | plan BASE-006 work |

---

## Recommended operators (smoke set)

From repo root, typical confidence loop for surfaces above:

```sh
# Oracle-side (no Rust required for pure path except dump parity step)
./tools/oracle/selftest_harness.sh
./tools/oracle/selftest_aggregates.sh
./tools/oracle/selftest_normalize_compat.sh
./tools/oracle/selftest_native_dump_parity.sh
./tools/oracle/selftest_native_dump_parity_all.sh

# Rust
cargo test -p nytprof-model native_matches_aggregates
cargo test -p nytprof-report report_semantic_parity_default_calls1
cargo test -p nytprof-format-v5
cargo test -p nytprof-cli

# Semantic HTML smoke (needs oracle + cargo)
bash tools/oracle/report_semantic_parity.sh

# Packaging
./scripts/packaging/packaging_gate.sh
```

---

## How to extend

1. New fixture under `fixtures/v5/<name>/` → add matrix rows + regenerate `aggregates.oracle.json`.  
2. New native surface → add cargo test name to the crate index and a matrix row with explicit gaps.  
3. Closing a gap → remove or rewrite the corresponding **G-006-*** entry and link evidence on the first-slice board.  
4. Full plan BASE-006 still owns a future machine-readable matrix; this file is the Phase-0 human SoT.

## Open items (process)

| ID | Item |
|----|------|
| OI-BASE006-01 | Promote high-value G-006-* into TEST-*/PERL-*/REPORT-* owners with explicit owners |
| OI-BASE006-02 | Optional JSON export of this matrix for CI inventory checks |
| OI-BASE006-03 | Map remaining BASE-004 method rows (FileInfo/SubInfo internals) to planned PERL-001 suite |

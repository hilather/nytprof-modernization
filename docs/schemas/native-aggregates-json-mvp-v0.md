# Native aggregates JSON MVP (v0)

**Board ID:** NATIVE-AGG-JSON  
**Status:** implemented  
**Not:** full aggregate-comparison dump of every A1–A9 map, full Data.pm query surface, or Perl `query --json` replacement (that path remains under `nytprof-engine`)

## Goal

The **shipped native CLI** must emit a **stable structured JSON object** of ProfileModel aggregates for machine consumers (packaging gates, agent harnesses, differential tooling). Values come only from real `ProfileModel::from_path` APIs after the same fail-closed stream checks as text `report`.

Primary fixture: `fixtures/v5/default-calls1/nytprof.out` → leaf returns **15**, mid **3**, mid→leaf **15**.

Never put `crates/` on oracle `PERL5LIB`.

## Chosen CLI form

**Primary (documented):**

```sh
nytprof-cli report --json fixtures/v5/default-calls1/nytprof.out
```

**Also accepted:**

```sh
nytprof-cli report fixtures/v5/default-calls1/nytprof.out --json
nytprof-cli report --format=json fixtures/v5/default-calls1/nytprof.out
nytprof-cli report --format json fixtures/v5/default-calls1/nytprof.out
nytprof-cli summary --json fixtures/v5/default-calls1/nytprof.out
nytprof-cli aggregates fixtures/v5/default-calls1/nytprof.out
nytprof-cli agg fixtures/v5/default-calls1/nytprof.out
```

| Form | Notes |
|------|-------|
| `report --json PATH` | Preferred; path may also precede `--json` |
| `--format=json` / `--format json` | Aliases for `--json` on `report` / `summary` |
| `aggregates` / `agg` | Always JSON (optional redundant `--json` allowed) |
| `report PATH` (no JSON flag) | Unchanged human text summary |

Binary name `nytprof-dump` is the same package (same argv).

## Fail-closed load

Uses the same path as text report:

1. `ProfileModel::from_path(path)`
2. `require_complete_stream` (INCOMPLETE-STREAM / COMPAT-010)

Corrupt, truncated, empty, bad-magic, or incomplete streams → non-zero exit, **no** `ok: true` object on stdout. Dump remains lenient; aggregates JSON does not.

## Success output (exit 0)

Stdout is a **single JSON object** (compact one line + trailing newline). Required fields:

| Field | Type | Meaning / default-calls1 contract |
|-------|------|-----------------------------------|
| `ok` | boolean | `true` on success |
| `profile` | string | Profile path as given on the CLI |
| `leaf_returns` | integer | A5 returns for `main::leaf` (**15**); `0` if absent |
| `mid_returns` | integer | A5 returns for `main::mid` (**3**); `0` if absent |
| `mid_leaf_edge` | integer | A7 count for `main::mid` → `main::leaf` (**15**); `0` if absent |
| `discount_events` | integer | A3 `DISCOUNT` multiplicity from model |
| `sub_entry_events` | integer | `SUB_ENTRY` multiplicity from `ProfileModel.sub_entry_events` (**JSON-SUB-ENTRY-MVP**); default-calls1 **0**, calls2-default **27** |
| `is_stream_complete` | boolean | `ProfileModel::is_stream_complete()` (**JSON-NATIVE-STREAM-MVP**); default-calls1 **true** |
| `incompleteness_reasons` | array of strings | `ProfileModel::stream_incompleteness_reasons()` as JSON array; **[]** when complete (field name matches Perl `query --json`, not the Rust method name) |
| `time_line_events` | integer | `ProfileModel.time_line_events` (A1 count); default-calls1 **≥ 1** (golden observes **916**); blocks-calls1 **0** |
| `time_block_events` | integer | `ProfileModel.time_block_events` (A2 `TIME_BLOCK` multiplicity) — **JSON-TIME-BLOCK-MVP**; default-calls1 **0**; blocks-calls1 **916** (match stream recount / model) |
| `pid_start_events` | integer | `ProfileModel.pid_start_events`; default-calls1 **≥ 1** |
| `pid_end_events` | integer | `ProfileModel.pid_end_events`; default-calls1 **≥ 1** |
| `line_calls_1_5` | integer | A4 `line_total(1,5).calls` (**780** on default-calls1 / blocks-calls1; **0** if absent) — **JSON-BLOCKS-MVP** |
| `block_line_calls_1_4` | integer | A4b `block_line_total(1,4).calls` (**0** on default-calls1; **810** on blocks-calls1) — **JSON-BLOCKS-MVP** |
| `sub_def_leaf` | object or null | A9 `sub_def("main::leaf")` → **`{"fid":1,"first_line":3,"last_line":7}`** on default-calls1 (**null** if absent) — **JSON-SUBDEF-SOURCE-MVP** |
| `sub_def_mid` | object or null | A9 `sub_def("main::mid")` → **`{"fid":1,"first_line":8,"last_line":12}`** on default-calls1 (**null** if absent) — **JSON-SUBDEF-SOURCE-MVP** |
| `source_line_1_5` | string or null | A8 `source_line(1, 5)` exact text (**`    $x++ for 1 .. 50;\n`**; contains `$x++` and `1 .. 50`) — **JSON-SUBDEF-SOURCE-MVP** |
| `attribute_ticks_per_sec` | string or null | `attributes["ticks_per_sec"]` (**`"10000000"`** on default-calls1; **null** if absent) — **JSON-META-FILES-MVP** |
| `attribute_basetime` | string or null | `attributes["basetime"]` (**`"1786111723"`** on default-calls1; **null** if absent) — **JSON-ATTR-BASETIME-MVP** (greppable dump sample; not wall-clock freeze) |
| `option_calls` | string or null | `options["calls"]` (**`"1"`** on default-calls1; **null** if absent) — **JSON-META-FILES-MVP** |
| `file_1` | string or null | `file_name(1)` / `files[1]` path (contains **`workload.pl`** on default-calls1; **null** if absent) — **JSON-META-FILES-MVP** |
| `file_1_basename` | string or null | `fid_basename(1)` (**`"workload.pl"`** on default-calls1; **null** if absent) — **JSON-FILE-BASENAME-MVP** (stable; absolute `file_1` is volatile) |
| `total_events` | integer | dump stream multiplicity incl. synthetic `_END` = `ProfileModel.total_events + 1` — **JSON-TOTAL-EVENTS-MVP**; default-calls1 **2474** (model decoded tags **2473**) |
| `sub_return_events` | integer | `ProfileModel.sub_return_events` (`SUB_RETURN` tag multiplicity) — **JSON-EVENT-COUNTS-MVP**; default-calls1 **27** |
| `new_fid_events` | integer | `ProfileModel.new_fid_events` (`NEW_FID` tag multiplicity) — **JSON-EVENT-COUNTS-MVP**; default-calls1 **3** |
| `sub_callers_events` | integer | `ProfileModel.sub_callers_events` (`SUB_CALLERS` tag multiplicity, not edge-count sum) — **JSON-EVENT-COUNTS-MVP**; default-calls1 **13** |
| `src_line_events` | integer | `ProfileModel.src_line_events` (`SRC_LINE` stream count) — **JSON-EVENT-COUNTS-MVP**; default-calls1 **632** |
| `sub_info_events` | integer | `ProfileModel.sub_info_events` (`SUB_INFO` stream count) — **JSON-EVENT-COUNTS-MVP**; default-calls1 **31** |
| `subs` | object string→int | All A5 subnames → **return counts** only |
| `edges` | object string→int | All A7 edges; key is `"caller\\tcalled"` (TAB-joined), value is **count** |

Example (field order may vary by encoder; values must match model):

```json
{
  "ok": true,
  "profile": "fixtures/v5/default-calls1/nytprof.out",
  "leaf_returns": 15,
  "mid_returns": 3,
  "mid_leaf_edge": 15,
  "discount_events": 818,
  "sub_entry_events": 0,
  "total_events": 2474,
  "is_stream_complete": true,
  "incompleteness_reasons": [],
  "time_line_events": 916,
  "time_block_events": 0,
  "pid_start_events": 1,
  "pid_end_events": 1,
  "line_calls_1_5": 780,
  "block_line_calls_1_4": 0,
  "sub_def_leaf": {"fid":1,"first_line":3,"last_line":7},
  "sub_def_mid": {"fid":1,"first_line":8,"last_line":12},
  "source_line_1_5": "    $x++ for 1 .. 50;\n",
  "attribute_ticks_per_sec": "10000000",
  "attribute_basetime": "1786111723",
  "option_calls": "1",
  "file_1": "/tmp/.../workload.pl",
  "file_1_basename": "workload.pl",
  "sub_return_events": 27,
  "new_fid_events": 3,
  "sub_callers_events": 13,
  "src_line_events": 632,
  "sub_info_events": 31,
  "subs": {
    "main::leaf": 15,
    "main::mid": 3
  },
  "edges": {
    "main::mid\tmain::leaf": 15,
    "main::RUNTIME\tmain::mid": 3
  }
}
```

Notes:

- `subs` / `edges` include **all** model totals (not only leaf/mid); example above is abbreviated.
- Edge keys use a **TAB** between caller and called (same convention as Perl QUERY-JSON-MVP / `JsonlData` edge maps).
- Convenience integers always present so greps like `"leaf_returns":15` work without walking maps.
- Source of truth: `ProfileModel::sub_total`, `ProfileModel::call_edge`, `ProfileModel::discount_events`, `ProfileModel::sub_entry_events`, `ProfileModel::sub_return_events` / `new_fid_events` / `sub_callers_events` / `src_line_events` / `sub_info_events`, `ProfileModel::is_stream_complete` / `stream_incompleteness_reasons`, `ProfileModel.time_line_events` / `time_block_events` / `pid_start_events` / `pid_end_events`, `ProfileModel::line_total` / `block_line_total`, `ProfileModel::sub_def` / `source_line`, `ProfileModel.attributes` / `options` / `file_name` / `fid_basename`, plus full maps from `sub_return_totals` / `call_edges`.
- Field name **`sub_entry_events`** matches `JsonlData->sub_entry_events` / Perl `query --json` (no separate `sub_entry_count` alias on the native JSON object).
- Stream/PID field names match Perl `query --json` (**JSON-NATIVE-STREAM-MVP**): `is_stream_complete`, `incompleteness_reasons` (JSON name; API is `stream_incompleteness_reasons`), `time_line_events`, `pid_start_events`, `pid_end_events`.
- Field **`time_block_events`** matches `JsonlData->time_block_events` / Perl `query --json` (**JSON-TIME-BLOCK-MVP**); always present; dump/model-derived only (not invented).
- Stream tag multiplicities **`sub_return_events`** / **`new_fid_events`** / **`sub_callers_events`** / **`src_line_events`** / **`sub_info_events`** match `JsonlData` / Perl `query --json` (**JSON-EVENT-COUNTS-MVP**); always present; dump/model-derived only (not invented); default-calls1 **27** / **3** / **13** / **632** / **31**.
- Greppable A4/A4b keys **`line_calls_1_5`** / **`block_line_calls_1_4`** match Perl `query --json` (JSON-BLOCKS-MVP).
- Greppable A9/A8 sample keys **`sub_def_leaf`** / **`sub_def_mid`** / **`source_line_1_5`** match Perl `query --json` (JSON-SUBDEF-SOURCE-MVP); not a full A8/A9 map dump.
- Greppable ATTRIBUTE/OPTION/NEW_FID sample keys **`attribute_ticks_per_sec`** / **`option_calls`** / **`file_1`** match Perl `query --json` (JSON-META-FILES-MVP); always present as string-or-null; not a full attributes/options/files map dump.
- Greppable **`file_1_basename`** matches Perl `query --json` (**JSON-FILE-BASENAME-MVP**); string-or-null from `fid_basename(1)`; stable identity vs volatile absolute `file_1` (`/tmp/...`).
- Human text `report` is unchanged when `--json` is absent.

## Failure (non-zero exit)

- Missing path / unknown option / unknown `--format` value  
- Decode / model load error  
- Incomplete stream (`require_complete_stream`)  

Print error to stderr. Do **not** emit `{"ok":true,...}` on failure.

## Evidence

| Check | Path |
|-------|------|
| Schema (this doc) | `docs/schemas/native-aggregates-json-mvp-v0.md` |
| CLI | `crates/nytprof-cli/src/main.rs` (`report --json`, `aggregates`) |
| Cargo tests | `crates/nytprof-cli/tests/native_agg_json.rs` |
| Smoke | `./scripts/packaging/native_agg_json_smoke.sh` (run ×2; assert 15/3/15 + `sub_entry_events` **0** + stream/PID) |
| Stream/PID JSON | `./scripts/packaging/json_native_stream_smoke.sh` (**JSON-NATIVE-STREAM-MVP**; optional Perl compare) |
| SUB_ENTRY JSON | `./scripts/packaging/json_sub_entry_smoke.sh` (default **0** / calls2 **27** on both surfaces) |
| Event counts JSON | `./scripts/packaging/json_event_counts_smoke.sh` (**JSON-EVENT-COUNTS-MVP**: default-calls1 **27/3/13/632/31** on both surfaces; optional golden tag recount) |
| A4/A4b JSON | `./scripts/packaging/json_blocks_smoke.sh` (blocks-calls1 **780** / **810**; optional native) |
| A9/A8 sample JSON | `./scripts/packaging/json_subdef_source_smoke.sh` (default-calls1 leaf **1/3–7**, mid **1/8–12**, source `$x++` / `1 .. 50`; optional native) |
| ATTRIBUTE/OPTION/NEW_FID samples | `./scripts/packaging/json_meta_files_smoke.sh` (**JSON-META-FILES-MVP**: `attribute_ticks_per_sec` **10000000**, `option_calls` **1**, `file_1` contains **workload.pl**; optional native) |
| Fid-1 basename sample | `./scripts/packaging/json_file_basename_smoke.sh` (**JSON-FILE-BASENAME-MVP**: `file_1_basename` equals/contains **workload.pl**; optional native) |
| TIME_BLOCK multiplicity | `./scripts/packaging/json_time_block_smoke.sh` (**JSON-TIME-BLOCK-MVP**: default-calls1 **0**, blocks-calls1 **916** on both surfaces; optional golden TIME_BLOCK tag recount) |
| Incomplete stream JSON | `./scripts/packaging/json_report_incomplete_smoke.sh` (**JSON-REPORT-INCOMPLETE-FAILCLOSED**; COMPAT-010: incomplete prefix → exit ≠ 0; no complete `ok:true` + `is_stream_complete:true`) |
| Cross-check | `./scripts/packaging/native_query_json_cross_smoke.sh` (NATIVE-QUERY-JSON-CROSS / CROSS-EXPAND / CROSS-BLOCKS / CROSS-META / CROSS-TIMEBLOCK) |

```sh
cargo test -p nytprof-cli --test native_agg_json
cargo test -p nytprof-cli --test fail_closed incomplete_stream_report_json
./scripts/packaging/native_agg_json_smoke.sh
./scripts/packaging/json_native_stream_smoke.sh
./scripts/packaging/json_time_block_smoke.sh
./scripts/packaging/json_report_incomplete_smoke.sh
./scripts/packaging/json_sub_entry_smoke.sh
./scripts/packaging/json_blocks_smoke.sh
./scripts/packaging/json_subdef_source_smoke.sh
./scripts/packaging/json_meta_files_smoke.sh
./scripts/packaging/json_file_basename_smoke.sh
cargo run -q -p nytprof-cli -- report --json fixtures/v5/default-calls1/nytprof.out
cargo run -q -p nytprof-cli -- report --json fixtures/v5/blocks-calls1/nytprof.out
./scripts/packaging/native_query_json_cross_smoke.sh
```

## Cross-check vs Perl `query --json` (NATIVE-QUERY-JSON-CROSS / CROSS-EXPAND / CROSS-BLOCKS / CROSS-META / CROSS-TIMEBLOCK / CROSS-COUNTS / CROSS-TOTAL)

Shared convenience fields are stable across **native** `report --json` and **Perl** `nytprof-engine query --json` (golden `--jsonl` and optional dump-of-profile). Do **not** reimplement aggregation in the smoke — invoke real CLIs and parse JSON.

| Shared field | default-calls1 | calls2-default | blocks-calls1 |
|--------------|----------------|----------------|---------------|
| `leaf_returns` | **15** | (not asserted in cross-expand; fixture has same shape) | (same shape; not re-asserted in blocks section) |
| `mid_returns` | **3** | (not asserted in cross-expand) | (not re-asserted) |
| `mid_leaf_edge` | **15** | (not asserted in cross-expand) | (not re-asserted) |
| `discount_events` | **818** (equal between sides; golden contract) | (not asserted in cross-expand) | (not re-asserted) |
| `sub_entry_events` | **0** (`calls=1`) when **both** sides expose the field | **27** (`calls=2`) when **both** sides expose the field (**fixture-scoped** expand) | (not re-asserted) |
| `time_block_events` | **0** when both expose (**NATIVE-QUERY-JSON-CROSS-TIMEBLOCK**) | — | **916** when both expose (**CROSS-TIMEBLOCK**) |
| `sub_return_events` | **27** when both expose (**NATIVE-QUERY-JSON-CROSS-COUNTS** / **JSON-EVENT-COUNTS-MVP**) | — | — |
| `new_fid_events` | **3** when both expose (**CROSS-COUNTS**) | — | — |
| `sub_callers_events` | **13** when both expose (**CROSS-COUNTS**) | — | — |
| `src_line_events` | **632** when both expose (**CROSS-COUNTS**) | — | — |
| `sub_info_events` | **31** when both expose (**CROSS-COUNTS**) | — | — |
| `file_1_basename` | exact equal **or** both contain **`workload.pl`** when both expose (**CROSS-COUNTS** / **JSON-FILE-BASENAME-MVP**); absolute `file_1` remains volatile | — | — |
| `total_events` | **2474** when both expose (**NATIVE-QUERY-JSON-CROSS-TOTAL** / **JSON-TOTAL-EVENTS-MVP**; dump stream incl. `_END`) | — | — |
| `attribute_basetime` | equal dump string when both expose (**CROSS-TOTAL** / **JSON-ATTR-BASETIME-MVP**; golden often **`"1786111723"`**) | — | — |
| `is_stream_complete` | **true** when both expose (**NATIVE-QUERY-JSON-CROSS-META**) | — | — |
| `incompleteness_reasons` | **[]** equal when both expose (cheap; CROSS-META) | — | — |
| `time_line_events` | equal when both expose (golden observes **916**) | — | — |
| `pid_start_events` / `pid_end_events` | equal when both expose (typically **1**/**1**) | — | — |
| `sub_def_leaf` | equal `{fid,first_line,last_line}` → **1/3–7** when both expose | — | — |
| `sub_def_mid` | equal → **1/8–12** when both expose | — | — |
| `source_line_1_5` | equal exact text when both expose | — | — |
| `attribute_ticks_per_sec` | equal when both expose (**required**, not optional-only; CROSS-TIMEBLOCK / JSON-META-FILES-MVP) | — | — |
| `option_calls` | equal when both expose (**required** when both expose) | — | — |
| `file_1` | equal exact **or** both paths contain **`workload.pl`** when both expose | — | — |
| `line_calls_1_5` | (present; not the blocks cross contract) | — | **780** (**NATIVE-QUERY-JSON-CROSS-BLOCKS**) |
| `block_line_calls_1_4` | **0** on this fixture (no TIME_BLOCK) | — | **810** (**NATIVE-QUERY-JSON-CROSS-BLOCKS**) |

Smoke: `./scripts/packaging/native_query_json_cross_smoke.sh` (**NATIVE-QUERY-JSON-CROSS** + **NATIVE-QUERY-JSON-CROSS-EXPAND** + **NATIVE-QUERY-JSON-CROSS-BLOCKS** + **NATIVE-QUERY-JSON-CROSS-META** + **NATIVE-QUERY-JSON-CROSS-TIMEBLOCK** + **NATIVE-QUERY-JSON-CROSS-COUNTS**):

1. default-calls1 pair ×2: equal `leaf_returns`/`mid_returns`/`mid_leaf_edge`/`discount_events` (**15/3/15/818**); when both sides expose `sub_entry_events`, equal and **0**; when both expose `time_block_events`, equal and **0**
2. **Counts (NATIVE-QUERY-JSON-CROSS-COUNTS):** when both sides expose event counters (**JSON-EVENT-COUNTS-MVP**), equal `sub_return_events` **27**, `new_fid_events` **3**, `sub_callers_events` **13**, `src_line_events` **632**, `sub_info_events` **31**; when both expose `file_1_basename` (**JSON-FILE-BASENAME-MVP**), exact equal **or** both contain **`workload.pl`** (absolute `file_1` remains volatile)
3. **Meta (NATIVE-QUERY-JSON-CROSS-META + greppable required under CROSS-TIMEBLOCK):** on default-calls1 pair ×2 (+ dump path), when both sides expose stream/PID fields (**JSON-NATIVE-STREAM-MVP**), equal `is_stream_complete` (**true**), `time_line_events`, `pid_start_events`, `pid_end_events`, and `incompleteness_reasons` when present (prefer empty); when both expose A9/A8 samples (**JSON-SUBDEF-SOURCE-MVP**), equal `sub_def_leaf` / `sub_def_mid` (fid/first_line/last_line) and `source_line_1_5`. **Greppable meta** (`attribute_ticks_per_sec`, `option_calls`, `file_1`) **must** equal when **both** sides expose them (skip-with-NOTE only if missing on one side).
4. optional `query --json <profile>` dump path matches native shared fields (incl. event counts + basename + `sub_entry_events` + `time_block_events` + meta fingerprint when present)
5. **Expand (fixture-scoped):** `fixtures/v5/calls2-default` side-by-side native `report --json` + Perl `query --json --jsonl` → `sub_entry_events` **27** when both sides expose SUB_ENTRY
6. **Blocks (NATIVE-QUERY-JSON-CROSS-BLOCKS + CROSS-TIMEBLOCK):** `fixtures/v5/blocks-calls1` pair ×2 — native `report --json` vs Perl `query --json --jsonl` → `line_calls_1_5` **780** and `block_line_calls_1_4` **810**, equal native↔perl; when both expose `time_block_events`, equal and **916** (real CLIs only; no re-aggregation)

If only one side omits `sub_entry_events` / `time_block_events` / event-count / basename (pre-MVP landings) or stream/A9/A8/greppable-meta fields, the smoke logs a NOTE and skips the equal assert for that group/field rather than failing closed on partial landings.

Related emit smoke (when present): `./scripts/packaging/json_sub_entry_smoke.sh` (JSON-SUB-ENTRY-MVP); `./scripts/packaging/json_blocks_smoke.sh` (JSON-BLOCKS-MVP emit); `./scripts/packaging/json_native_stream_smoke.sh` (JSON-NATIVE-STREAM-MVP); `./scripts/packaging/json_subdef_source_smoke.sh` (JSON-SUBDEF-SOURCE-MVP); `./scripts/packaging/json_time_block_smoke.sh` (JSON-TIME-BLOCK-MVP). Wired into `offline_gate.sh` step 8 when native CLI available. Perl-only schema: [`perl-engine-dispatch-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-engine-dispatch-mvp-v0.md).

## JSON-TIME-BLOCK-MVP (A2 TIME_BLOCK multiplicity)

Emit dump/model-derived `time_block_events` on native `report --json` / `aggregates` so it matches pure-Perl `query --json` / `JsonlData->time_block_events`. Values come **only** from `ProfileModel.time_block_events` after the same fail-closed load as text report.

| Field | Type | Source | default-calls1 | blocks-calls1 |
|-------|------|--------|----------------|---------------|
| `time_block_events` | integer | `model.time_block_events` | **0** | **916** |

Always present. Complements `time_line_events` (A1) from JSON-NATIVE-STREAM-MVP. Stream completeness still uses `TIME_LINE + TIME_BLOCK > 0` per COMPAT-010.

Smoke: `./scripts/packaging/json_time_block_smoke.sh` (both surfaces; optional golden `TIME_BLOCK` tag recount). Cargo: `native_agg_json.rs` matches `ProfileModel::from_path` (default **0**, blocks **916**).

## JSON-BLOCKS-MVP (A4 / A4b greppable ints)

| Field | default-calls1 | blocks-calls1 | Source |
|-------|----------------|---------------|--------|
| `line_calls_1_5` | **780** | **780** | `line_total(1, 5).calls` |
| `block_line_calls_1_4` | **0** | **810** | `block_line_total(1, 4).calls` |

Smoke: `./scripts/packaging/json_blocks_smoke.sh` (Perl golden required; native profile path when CLI available). Same keys on Perl `query --json`. **Cross-parity** of the greppable ints on blocks-calls1 is **NATIVE-QUERY-JSON-CROSS-BLOCKS** (same `native_query_json_cross_smoke.sh`).

## JSON-NATIVE-STREAM-MVP (stream completeness + PID/timing counts)

Emit dump/model-derived stream fields on native `report --json` / `aggregates` so they match pure-Perl `query --json` keys (QUERY-JSON-EXPAND). Values come **only** from `ProfileModel` after the same fail-closed load as text report. Do **not** remove Perl fields.

| Field | Type | Source | default-calls1 contract |
|-------|------|--------|-------------------------|
| `is_stream_complete` | boolean | `model.is_stream_complete()` | **true** |
| `incompleteness_reasons` | array of strings | `model.stream_incompleteness_reasons()` | **[]** (empty when complete) |
| `time_line_events` | integer | `model.time_line_events` | **≥ 1** (golden observes **916**) |
| `pid_start_events` | integer | `model.pid_start_events` | **≥ 1** |
| `pid_end_events` | integer | `model.pid_end_events` | **≥ 1** |

Contract: [`docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md). Completeness rules: PID balance when starts seen **and** `TIME_LINE + TIME_BLOCK > 0`.

Smokes:

1. `./scripts/packaging/native_agg_json_smoke.sh` — asserts stream fields on native `report --json` ×2
2. `./scripts/packaging/json_native_stream_smoke.sh` — focused stream/PID asserts; when Perl engine + golden present, optionally compares shared stream/PID fields to `query --json --jsonl`

Cargo: `crates/nytprof-cli/tests/native_agg_json.rs` loads the same fixture via `ProfileModel::from_path` and requires JSON counters to match the model. Wired into `offline_gate.sh` with the native_agg step when native CLI available.

## JSON-META-FILES-MVP (ATTRIBUTE / OPTION / NEW_FID greppable samples)

Emit dump/model-derived metadata samples on native `report --json` / `aggregates` so they match pure-Perl `query --json` keys. Values come **only** from `ProfileModel.attributes` / `options` / `file_name(fid)` (or `files`). Always present as **string-or-null** (null when the dump lacks that key/fid). **Not** a full attributes/options/files map dump.

| Field | Type | Source | default-calls1 contract |
|-------|------|--------|-------------------------|
| `attribute_ticks_per_sec` | string or null | `attributes["ticks_per_sec"]` | **`"10000000"`** (ATTRIBUTE) |
| `option_calls` | string or null | `options["calls"]` | **`"1"`** (OPTION) |
| `file_1` | string or null | `file_name(1)` | path contains **`workload.pl`** (NEW_FID fid 1) |

Smoke: `./scripts/packaging/json_meta_files_smoke.sh` (Perl golden `--jsonl` required; optional native `report --json`; compares to JsonlData / model or independent golden ATTRIBUTE/OPTION/NEW_FID recount). Cargo asserts model match in `native_agg_json.rs`. Wired into `offline_gate.sh` step 6e.

## JSON-FILE-BASENAME-MVP (stable fid-1 basename sample)

Emit dump/model-derived **basename** of NEW_FID fid 1 on native `report --json` / `aggregates` so it matches pure-Perl `query --json`. Value comes **only** from `ProfileModel::fid_basename(1)`. Always present as **string-or-null** (null when the dump lacks that fid/path). Absolute `file_1` remains under **JSON-META-FILES-MVP** but is **volatile** (`/tmp/...`); **do not** freeze absolute paths as identity — basename is the stable contract.

| Field | Type | Source | default-calls1 contract |
|-------|------|--------|-------------------------|
| `file_1_basename` | string or null | `fid_basename(1)` | equals or contains **`workload.pl`** (typically exact **`"workload.pl"`**) |

Smoke: `./scripts/packaging/json_file_basename_smoke.sh` (Perl golden `--jsonl` required; optional native `report --json`; compares to JsonlData `file_basename(1)` / model). Cargo asserts model match in `native_agg_json.rs`. Wired into `offline_gate.sh` step 6g.

## Explicit non-requirements

| Out of scope | Notes |
|--------------|-------|
| Full A4/A4b/A8/A9 JSON maps | Convenience greppable ints only (`line_calls_1_5` / `block_line_calls_1_4` via **JSON-BLOCKS-MVP**); not full maps |
| Full ATTRIBUTE/OPTION/files maps | Greppable samples only (`attribute_ticks_per_sec` / `option_calls` / `file_1` via **JSON-META-FILES-MVP**; `file_1_basename` via **JSON-FILE-BASENAME-MVP**); not full maps |
| Freezing absolute `file_1` paths | Volatile under `/tmp`; use `file_1_basename` for stable greppable identity |
| Tick/time fields in JSON | Counts only; ticks under COMPAT-003 |
| Pretty multi-line JSON | Compact single-line is the contract |
| Replacing `nytprof-engine query --json` | Perl path remains; native path is independent; cross-smoke only asserts shared fields |
| Oracle PERL5LIB mutation | Never put `crates/` on oracle `PERL5LIB` |

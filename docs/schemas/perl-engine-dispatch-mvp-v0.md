# Perl engine-dispatch facade MVP (v0)

**Status:** thin operator entry before full XS Data/ReadStream facade  
**Complements:** `docs/schemas/engine-selection-mvp-v0.md` (same engine names)  
**Board IDs:** `PERL-ENGINE-DISPATCH`, `PERL-ENGINE-QUERY`, `PERL-ENGINE-QUERY-EXPAND`, `PERL-QUERY-PID-META`, `QUERY-JSON-MVP`, `QUERY-JSON-EXPAND`, `PERL-ENGINE-EXPORT`

## Entry points

| Path | Role |
|------|------|
| `perl/bin/nytprof-engine` | Operator CLI (preferred) |
| `perl/lib/Devel/NYTProf/EngineDispatch.pm` | Parse engine + dispatch actions (`resolve_engine`, `select_runtime_engine`, `run_native`, `run_legacy`, `run_query`, `dispatch`) |
| `perl/lib/Devel/NYTProf/LegacyBridge.pm` | Isolated oracle report path (`run_legacy_report`) — install-only `PERL5LIB` |
| `perl/lib/Devel/NYTProf/JsonlData.pm` | Pure-Perl sub/edge aggregates used by `query` |

```text
perl -Iperl/lib perl/bin/nytprof-engine [--engine=native|legacy|auto] report <profile.out>
perl -Iperl/lib perl/bin/nytprof-engine [--engine=...] verify <profile.out>
perl -Iperl/lib perl/bin/nytprof-engine [--engine=native] csv <profile.out>
perl -Iperl/lib perl/bin/nytprof-engine [--engine=native] html <profile.out>
perl -Iperl/lib perl/bin/nytprof-engine [--engine=native] folded <profile.out>
perl -Iperl/lib perl/bin/nytprof-engine [--engine=native] callgrind <profile.out>
perl -Iperl/lib perl/bin/nytprof-engine [--engine=native] query <profile.out>
perl -Iperl/lib perl/bin/nytprof-engine query --jsonl fixtures/v5/default-calls1/readstream.jsonl
perl -Iperl/lib perl/bin/nytprof-engine query --json --jsonl fixtures/v5/default-calls1/readstream.jsonl
# Env: NYTPROF_ENGINE (CLI flag overrides)
```

## Engine semantics

| Name | Behavior |
|------|----------|
| `native` | Subprocess to built `nytprof-cli` (or `cargo run -q -p nytprof-cli --` if binary missing and cargo available). Passes through `report` / `summary` / `verify` / `inspect` / `html` / `csv` / `dump` / `folded` / `callgrind` / `cg` to native CLI. Action `query` uses native dump → `JsonlData` (not full CLI report text). **Missing native CLI → fail** (clear error; no silent legacy). |
| `auto` | **Prefer native, fall back to legacy** when native CLI is not discoverable (`select_runtime_engine`). CLI flag or `NYTPROF_ENGINE=auto`. On fallback: STDERR note `auto: native CLI not found; using legacy`, then oracle stream-dump path (same isolation as `legacy`). When native is present: real native report/query (default-calls1 leaf **15** / mid **3**). Smokes: `./scripts/packaging/engine_auto_smoke.sh` (native present) + `./scripts/packaging/engine_auto_fallback_smoke.sh` (present + forced missing). |
| `legacy` | No Cargo. Delegates to `LegacyBridge::run_legacy_report`: oracle `PERL5LIB` from `baseline/6.15/oracle-perl5lib.txt` (install tree only), verify `Devel::NYTProf` under `baseline/6.15/install`, run `tools/oracle/dump_readstream.pl` (require exit 0 and line count > 0), optionally try `install/bin/nytprofcsv` (non-fatal if deps missing). `html` / `csv` / `dump` / `folded` / `callgrind` / `cg` map to the same stream-dump smoke (not full export). Must not put `crates/` or candidate `perl/` on oracle `PERL5LIB`. |
| invalid | Fail closed, non-zero exit, list allowed names. |

### Resolve vs runtime

| API | Returns | Notes |
|-----|---------|-------|
| `resolve_engine($cli, $env)` | `native` \| `legacy` \| `auto` | Does **not** collapse `auto` → `native`. |
| `select_runtime_engine($repo, $requested)` | `native` \| `legacy` | Used by `dispatch` / `run_cli` for the actual path. `auto` → eval `find_native_cli`; ok → native; else legacy + STDERR. |

**Residual:** pure-Rust `nytprof-cli` still maps `auto` → `native` and has no in-process legacy oracle path. The Perl facade is the required auto-fallback surface for this wave.

## Finding native binary

Order (see also `docs/schemas/native-install-mvp-v0.md`):
1. If `$ENV{NYTPROF_FORCE_NO_NATIVE}` is truthy (`1` / non-empty, not `0`/`false`/`no`/`off`) → fail immediately (**test hook only**; ENGINE-AUTO-FALLBACK smokes)
2. `$ENV{NYTPROF_NATIVE_CLI}` if set and executable  
3. `$REPO/prefix/bin/nytprof-cli` or `prefix/bin/nytprof-dump` (stable install via `scripts/packaging/install_native.sh`)  
4. `$REPO/target/release/nytprof-dump` then `target/debug/nytprof-dump` (binary name from nytprof-cli package)  
5. `cargo run -q -p nytprof-cli --` from repo root when cargo exists  

## Legacy success contract

Implemented by `Devel::NYTProf::LegacyBridge::run_legacy_report($repo, $profile)` (also via `EngineDispatch::run_legacy` / `nytprof-engine --engine=legacy report|verify`):

1. Build child env from `baseline/6.15/oracle-perl5lib.txt` + check `oracle-module-path.txt` (all under `install/`).
2. Verify `Devel/NYTProf.pm` resolves under `baseline/6.15/install` (path scan; does not start the collector).
3. Run `tools/oracle/dump_readstream.pl`; require exit 0 and JSONL line count **> 0** (includes `_END`).
4. If `install/bin/nytprofcsv` exists, try `-f <profile> -o <tempdir>`; on failure print `NOTE:` and still exit 0 when dump worked.

Parent may load the facade with `-Iperl/lib`; oracle children never inherit candidate `perl/` or `crates/` on `PERL5LIB`.

## Native success contract

Stdout from native `report` must include `main::leaf` with `returns=15` and `main::mid` with `returns=3` for default-calls1.

## Action `query` (PERL-ENGINE-QUERY / PERL-ENGINE-QUERY-EXPAND / PERL-QUERY-PID-META / QUERY-JSON-MVP / QUERY-JSON-EXPAND)

Shipped Perl entry that answers **dump-derived queries** by consuming dump JSONL via `Devel::NYTProf::JsonlData` (no XS, no oracle `PERL5LIB`). Uses JsonlData APIs only (no reimplementation of aggregation).

| Input | Path |
|-------|------|
| `query <profile.out>` | `find_native_cli` → `dump` subprocess → `JsonlData->from_cli` |
| `query --jsonl PATH` | `JsonlData->from_jsonl` (golden / saved dump; no cargo) |
| `data-query` | Alias for `query` |
| `query --json` / `--format=json` | Same load path; **JSON object** on stdout (QUERY-JSON-MVP + QUERY-JSON-EXPAND). Distinct from `--jsonl` (input). |

Implementation: `EngineDispatch::run_query` / `print_query_results` / `_parse_query_extra`.

MVP default output is **always-full** and kept readable (no separate `--full` flag required). Human greppable lines remain the default when neither `--json` nor `--format=json` is given.

### Output shape

1. All subroutine return totals (sorted by name)
2. All call edges (sorted)
3. A9 `sub_def` ranges: prefer key names `main::leaf` / `main::mid` if present, then remaining names sorted
4. A8 `source_line 1:5=…` when present (trailing newline chomped for one-line display)
5. A4 `line_calls 1:5=N` when non-zero
6. Up to a few A4b `block_line_calls fid:bl=N` samples when non-empty (prefer `1:4` first)
7. PID lifecycle when starts/ends present: `pid_start_count=N`, `pid_end_count=N`, then each `pid_start pid=… [ppid=…] [start_time=…]` and `pid_end pid=… [end_time=…]` (only fields JsonlData returns; no invented values)
8. ATTRIBUTE / OPTION: prefer key attributes (`ticks_per_sec`, `basetime`, `application`, `xs_version`) and key options (`calls`, `blocks`, `stmts`, `compress`); then remaining sorted when the map is short enough, else key ones + `attribute_count` / `option_count`

```text
main::leaf returns=15
main::mid returns=3
main::mid -> main::leaf count=15
sub_def main::leaf fid=1 first=3 last=7
sub_def main::mid fid=1 first=8 last=12
source_line 1:5=    $x++ for 1 .. 50;
pid_start_count=1
pid_end_count=1
pid_start pid=2975381 ppid=2975366 start_time=1786111723.96777
pid_end pid=2975381 end_time=1786111723.97052
attribute ticks_per_sec=10000000
attribute basetime=1786111723
attribute application=/tmp/tmp.WWUAKCFFFY/workload.pl
attribute xs_version=6.15
option calls=1
option blocks=0
option stmts=1
option compress=6
```

On **blocks-calls1** (TIME_BLOCK path), also e.g.:

```text
line_calls 1:5=780
block_line_calls 1:4=810
```

### JSON output (QUERY-JSON-MVP / QUERY-JSON-EXPAND)

Flags (any one):

| Flag | Notes |
|------|-------|
| `--json` | Structured JSON on stdout |
| `--format=json` | Same (case-insensitive value) |
| `--format json` | Two-arg form |

Emits a **single JSON object** (core `JSON::PP`, canonical key order, trailing newline). All values come from existing **JsonlData APIs only** (no re-aggregation). Stable fields required for smoke / machine consumers:

| Field | Type | Source API | Smoke contract (default-calls1) |
|-------|------|------------|----------------------------------|
| `ok` | boolean | — | `true` on successful query |
| `subs` | object name→int | `sub_return_totals` | `subs["main::leaf"]` **15**, `subs["main::mid"]` **3** |
| `edges` | object `"caller\\tcallee"`→int | `call_edge_totals` | `edges["main::mid\\tmain::leaf"]` **15** (TAB-joined keys) |
| `leaf_returns` | int | convenience from `subs` | **15** (0 if missing) |
| `mid_returns` | int | convenience from `subs` | **3** |
| `mid_leaf_edge` | int | convenience from `edges` | **15** |
| `discount_events` | int | `discount_events` | **818** (A3 DISCOUNT multiplicity only) |
| `sub_entry_events` | int | `sub_entry_events` | **0** on default-calls1 (`calls=1`); **27** on calls2-default (`calls=2`) — **JSON-SUB-ENTRY-MVP** |
| `is_stream_complete` | boolean | `is_stream_complete` | **true** |
| `incompleteness_reasons` | array of strings | `stream_incompleteness_reasons` | **[]** (empty when complete) |
| `time_line_events` | int | `time_line_events` | **≥ 1** on default-calls1 (golden **916**); **0** on blocks-calls1 |
| `time_block_events` | int | `time_block_events` | **0** on default-calls1; **916** on blocks-calls1 — **JSON-TIME-BLOCK-MVP** (A2; match stream recount / JsonlData) |
| `pid_start_events` | int | `pid_start_events` | **≥ 1** |
| `pid_end_events` | int | `pid_end_events` | **≥ 1** |
| `line_calls_1_5` | int | `line_calls(1, 5)` | **780** on default-calls1 / blocks-calls1 (A4; **0** if location absent) — **JSON-BLOCKS-MVP** |
| `block_line_calls_1_4` | int | `block_line_calls(1, 4)` | **0** on default-calls1 (no TIME_BLOCK); **810** on blocks-calls1 (A4b) — **JSON-BLOCKS-MVP** |
| `sub_def_leaf` | object or null | `sub_def('main::leaf')` | **`{"fid":1,"first_line":3,"last_line":7}`** on default-calls1 — **JSON-SUBDEF-SOURCE-MVP** |
| `sub_def_mid` | object or null | `sub_def('main::mid')` | **`{"fid":1,"first_line":8,"last_line":12}`** on default-calls1 — **JSON-SUBDEF-SOURCE-MVP** |
| `source_line_1_5` | string or null | `source_line(1, 5)` | exact dump text **`    $x++ for 1 .. 50;\n`** (contains `$x++` and `1 .. 50`) — **JSON-SUBDEF-SOURCE-MVP** |
| `attribute_ticks_per_sec` | string or null | `attribute('ticks_per_sec')` | **`"10000000"`** on default-calls1 — **JSON-META-FILES-MVP** |
| `attribute_basetime` | string or null | `attribute('basetime')` | **`"1786111723"`** on default-calls1 — **JSON-ATTR-BASETIME-MVP** (greppable dump sample) |
| `option_calls` | string or null | `option('calls')` | **`"1"`** on default-calls1 — **JSON-META-FILES-MVP** |
| `file_1` | string or null | `file(1)` | path contains **`workload.pl`** on default-calls1 — **JSON-META-FILES-MVP** |
| `file_1_basename` | string or null | `file_basename(1)` | equals or contains **`workload.pl`** (typically **`"workload.pl"`**) — **JSON-FILE-BASENAME-MVP** |
| `total_events` | int | `records_seen` | **2474** — **JSON-TOTAL-EVENTS-MVP** (dump JSONL lines incl. synthetic `_END`; shared key with native) |
| `sub_return_events` | int | `sub_return_events` | **27** — **JSON-EVENT-COUNTS-MVP** (`SUB_RETURN` tag multiplicity) |
| `new_fid_events` | int | `new_fid_events` | **3** — **JSON-EVENT-COUNTS-MVP** |
| `sub_callers_events` | int | `sub_callers_events` | **13** — **JSON-EVENT-COUNTS-MVP** (tag count, not edge-count sum) |
| `src_line_events` | int | `src_line_events` | **632** — **JSON-EVENT-COUNTS-MVP** |
| `sub_info_events` | int | `sub_info_events` | **31** — **JSON-EVENT-COUNTS-MVP** |

Example (fields may appear in any order; encoder uses canonical sort):

```json
{
  "attribute_ticks_per_sec": "10000000",
  "block_line_calls_1_4": 0,
  "discount_events": 818,
  "edges": {"main::mid\tmain::leaf": 15},
  "file_1": "/tmp/.../workload.pl",
  "file_1_basename": "workload.pl",
  "incompleteness_reasons": [],
  "is_stream_complete": true,
  "leaf_returns": 15,
  "line_calls_1_5": 780,
  "mid_leaf_edge": 15,
  "mid_returns": 3,
  "new_fid_events": 3,
  "ok": true,
  "option_calls": "1",
  "pid_end_events": 1,
  "pid_start_events": 1,
  "source_line_1_5": "    $x++ for 1 .. 50;\n",
  "src_line_events": 632,
  "sub_callers_events": 13,
  "sub_def_leaf": {"fid":1,"first_line":3,"last_line":7},
  "sub_def_mid": {"fid":1,"first_line":8,"last_line":12},
  "sub_entry_events": 0,
  "sub_info_events": 31,
  "sub_return_events": 27,
  "subs": {"main::leaf": 15, "main::mid": 3},
  "time_block_events": 0,
  "time_line_events": 916
}
```

(`time_line_events` / `pid_*_events` values are dump-derived; smoke asserts only **≥ 1** for those. Default-calls1 golden observes TL **916** / TB **0** / pid **1** / **1**. Blocks-calls1: TL **0** / TB **916**.)

`subs` / `edges` include all dump-derived totals (not only leaf/mid). Convenience integers always present so greps like `"leaf_returns":15` work without walking maps. Field name **`sub_entry_events`** matches `JsonlData->sub_entry_events` and native `report --json` (no separate `sub_entry_count` alias on the query JSON object; the JsonlData method alias still exists). Field **`time_block_events`** matches `JsonlData->time_block_events` and native `report --json` (**JSON-TIME-BLOCK-MVP**). Stream tag multiplicities **`sub_return_events`** / **`new_fid_events`** / **`sub_callers_events`** / **`src_line_events`** / **`sub_info_events`** match `JsonlData` and native `report --json` (**JSON-EVENT-COUNTS-MVP**; default-calls1 **27** / **3** / **13** / **632** / **31**). Greppable A4/A4b keys **`line_calls_1_5`** / **`block_line_calls_1_4`** always present (**0** when location absent). Greppable A9/A8 sample keys **`sub_def_leaf`** / **`sub_def_mid`** / **`source_line_1_5`** always present (**null** when dump lacks that sub/source). Greppable ATTRIBUTE/OPTION/NEW_FID sample keys **`attribute_ticks_per_sec`** / **`option_calls`** / **`file_1`** always present as string-or-null (**JSON-META-FILES-MVP**). Greppable **`file_1_basename`** always present as string-or-null from `file_basename(1)` (**JSON-FILE-BASENAME-MVP**; stable vs volatile absolute `file_1`). **Not** a full re-export of every sub_def / source / attribute / option / files map.

#### JSON-EVENT-COUNTS-MVP (stream tag multiplicities)

| Field | Type | default-calls1 | Source API |
|-------|------|----------------|------------|
| `sub_return_events` | int | **27** | `sub_return_events` (JsonlData / ProfileModel) |
| `new_fid_events` | int | **3** | `new_fid_events` |
| `sub_callers_events` | int | **13** | `sub_callers_events` (tag count, not sum of edge counts) |
| `src_line_events` | int | **632** | `src_line_events` |
| `sub_info_events` | int | **31** | `sub_info_events` |

Always present. Values are dump/model-derived only (one increment per matching tag successfully ingested; match independent golden tag recount). Complements `discount_events` / `sub_entry_events` / `time_*_events` already on the JSON object. Human greppable path unchanged.

Smoke: `./scripts/packaging/json_event_counts_smoke.sh` (default-calls1 **27/3/13/632/31** on both surfaces; optional golden tag recount).

#### JSON-TIME-BLOCK-MVP (A2 TIME_BLOCK multiplicity)

| Field | Type | default-calls1 | blocks-calls1 | Source API |
|-------|------|----------------|---------------|------------|
| `time_block_events` | int | **0** | **916** | `time_block_events` (JsonlData / ProfileModel) |

Always present. Values are dump/model-derived only (match independent golden `TIME_BLOCK` tag recount). Complements `time_line_events` (A1) already on QUERY-JSON-EXPAND / JSON-NATIVE-STREAM-MVP. Human greppable path unchanged.

Smoke: `./scripts/packaging/json_time_block_smoke.sh` (default **0** / blocks **916** on both surfaces; optional golden tag recount).

#### JSON-BLOCKS-MVP (A4 / A4b greppable ints)

| Field | blocks-calls1 | default-calls1 | Notes |
|-------|---------------|----------------|-------|
| `line_calls_1_5` | **780** | **780** (TIME_LINE) | A4 statement line; always present; **0** if missing |
| `block_line_calls_1_4` | **810** | **0** | A4b block start line; always present; **0** when no TIME_BLOCK |

Smoke: `./scripts/packaging/json_blocks_smoke.sh` (blocks-calls1 golden `--jsonl` → **780** / **810**; optional native `report --json` when CLI available). Human `line_calls 1:5=` / `block_line_calls 1:4=` path unchanged.

#### JSON-SUBDEF-SOURCE-MVP (A9 / A8 greppable samples)

| Field | Type | default-calls1 | Source API |
|-------|------|----------------|------------|
| `sub_def_leaf` | `{fid,first_line,last_line}` or null | **fid=1 first=3 last=7** | `sub_def('main::leaf')` |
| `sub_def_mid` | `{fid,first_line,last_line}` or null | **fid=1 first=8 last=12** | `sub_def('main::mid')` |
| `source_line_1_5` | string or null | **`    $x++ for 1 .. 50;\n`** | `source_line(1, 5)` |

Not a full A8/A9 map dump — only greppable workload samples (same spirit as `line_calls_1_5`). Human greppable `sub_def …` / `source_line 1:5=` lines remain on the non-JSON path.

Smoke: `./scripts/packaging/json_subdef_source_smoke.sh` (default-calls1 golden `--jsonl` required; optional native `report --json`).

#### JSON-META-FILES-MVP (ATTRIBUTE / OPTION / NEW_FID greppable samples)

| Field | Type | default-calls1 | Source API |
|-------|------|----------------|------------|
| `attribute_ticks_per_sec` | string or null | **`"10000000"`** | `attribute('ticks_per_sec')` |
| `option_calls` | string or null | **`"1"`** | `option('calls')` |
| `file_1` | string or null | path contains **`workload.pl`** | `file(1)` |

Not a full attributes/options/files map dump — only greppable samples (same spirit as `line_calls_1_5`). Human greppable `attribute …` / `option …` lines remain on the non-JSON path (PERL-QUERY-PID-META). Values are dump-derived strings only (do not invent). Absolute `file_1` is **volatile** (`/tmp/...`); see **JSON-FILE-BASENAME-MVP** for the stable basename contract.

Smoke: `./scripts/packaging/json_meta_files_smoke.sh` (default-calls1 golden `--jsonl` required; optional native `report --json`; compares to JsonlData/model or independent golden ATTRIBUTE/OPTION/NEW_FID recount).

#### JSON-FILE-BASENAME-MVP (stable fid-1 basename sample)

| Field | Type | default-calls1 | Source API |
|-------|------|----------------|------------|
| `file_1_basename` | string or null | equals or contains **`workload.pl`** (typically exact **`"workload.pl"`**) | `file_basename(1)` |

Always present as string-or-null. Values from JsonlData `file_basename` / ProfileModel `fid_basename` only (do not invent). Not a full files map. Absolute `file_1` remains available but is not identity.

Smoke: `./scripts/packaging/json_file_basename_smoke.sh` (default-calls1 golden `--jsonl` required; optional native `report --json`).

CLI:

```sh
perl -Iperl/lib perl/bin/nytprof-engine query --json --jsonl fixtures/v5/default-calls1/readstream.jsonl
perl -Iperl/lib perl/bin/nytprof-engine query --format=json --jsonl fixtures/v5/default-calls1/readstream.jsonl
perl -Iperl/lib perl/bin/nytprof-engine query --json --jsonl fixtures/v5/blocks-calls1/readstream.jsonl
./scripts/packaging/perl_query_json_smoke.sh
./scripts/packaging/json_sub_entry_smoke.sh
./scripts/packaging/json_blocks_smoke.sh
./scripts/packaging/json_subdef_source_smoke.sh
./scripts/packaging/json_meta_files_smoke.sh
./scripts/packaging/json_file_basename_smoke.sh
```

### Cross-check vs native `report --json` (NATIVE-QUERY-JSON-CROSS / CROSS-EXPAND / CROSS-BLOCKS / CROSS-META / CROSS-TIMEBLOCK / CROSS-COUNTS / CROSS-TOTAL)

Shared convenience integers (`leaf_returns` / `mid_returns` / `mid_leaf_edge` / `discount_events` / `sub_entry_events` / `time_block_events`) must match native aggregates JSON on default-calls1 (**15** / **3** / **15** / **818** / **0** / **0** when both sides expose SUB_ENTRY / TIME_BLOCK). Greppable A4/A4b ints on blocks-calls1 (**`line_calls_1_5` 780** / **`block_line_calls_1_4` 810**) must match native↔perl; **`time_block_events` 916** when both expose. Stream/PID + A9/A8 + greppable meta must match when both sides expose them (**NATIVE-QUERY-JSON-CROSS-META** / **CROSS-TIMEBLOCK**). Event multiplicity + basename (**NATIVE-QUERY-JSON-CROSS-COUNTS**): when both expose, equal **27/3/13/632/31** + `file_1_basename` (exact or both contain **`workload.pl`**). Smoke invokes real CLIs only (no re-aggregation):

```sh
# native
nytprof-cli report --json fixtures/v5/default-calls1/nytprof.out
nytprof-cli report --json fixtures/v5/blocks-calls1/nytprof.out
# perl golden
perl -Iperl/lib perl/bin/nytprof-engine query --json --jsonl fixtures/v5/default-calls1/readstream.jsonl
perl -Iperl/lib perl/bin/nytprof-engine query --json --jsonl fixtures/v5/blocks-calls1/readstream.jsonl
# cross (pair ×2 + dump path + event counts 27/3/13/632/31 + basename + meta + calls2 expand + blocks 780/810 + time_block 0/916)
./scripts/packaging/native_query_json_cross_smoke.sh
```

**Expand (NATIVE-QUERY-JSON-CROSS-EXPAND):** on `fixtures/v5/calls2-default`, when both sides expose `sub_entry_events`, require **27** (fixture-scoped — only the SUB_ENTRY count is cross-asserted there; leaf/mid/edge remain default-calls1-scoped). If a side omits the field, smoke notes and skips rather than failing.

**Blocks (NATIVE-QUERY-JSON-CROSS-BLOCKS):** on `fixtures/v5/blocks-calls1`, pair ×2 native `report --json` vs Perl `query --json --jsonl` → `line_calls_1_5` **780** and `block_line_calls_1_4` **810**, equal native↔perl (JSON-BLOCKS-MVP greppable ints; not full A4/A4b maps).

**Timeblock (NATIVE-QUERY-JSON-CROSS-TIMEBLOCK):** when both sides expose `time_block_events`, default-calls1 → **0**; blocks-calls1 → **916**. Skip-with-NOTE if only one side exposes the field.

**Counts (NATIVE-QUERY-JSON-CROSS-COUNTS):** on default-calls1 pair ×2 (+ dump path), when both sides expose **JSON-EVENT-COUNTS-MVP** fields, equal `sub_return_events` **27**, `new_fid_events` **3**, `sub_callers_events` **13**, `src_line_events` **632**, `sub_info_events` **31**; when both expose `file_1_basename` (**JSON-FILE-BASENAME-MVP**), exact equal **or** both contain **`workload.pl`**. Absolute `file_1` remains volatile under `/tmp`. Skip-with-NOTE if only one side exposes a field.

**Meta (NATIVE-QUERY-JSON-CROSS-META + greppable required under CROSS-TIMEBLOCK):** on `fixtures/v5/default-calls1` pair ×2 (+ optional dump path), when both sides expose the field set:

| Field group | Keys | Contract when both expose |
|-------------|------|---------------------------|
| Stream/PID (**JSON-NATIVE-STREAM-MVP**) | `is_stream_complete`, `time_line_events`, `pid_start_events`, `pid_end_events` | equal native↔perl; `is_stream_complete` **true** |
| Reasons (cheap) | `incompleteness_reasons` | equal when both expose; prefer **[]** when complete |
| A9/A8 samples (**JSON-SUBDEF-SOURCE-MVP**) | `sub_def_leaf`, `sub_def_mid`, `source_line_1_5` | equal objects/string (leaf **1/3–7**, mid **1/8–12**, hot-loop text) |
| Greppable meta (**JSON-META-FILES-MVP**; **required** when both expose) | `attribute_ticks_per_sec`, `option_calls`, `file_1` | equal native↔perl; `file_1` exact **or** both paths contain **`workload.pl`**; skip-with-NOTE only if missing on one side |
| Event counts (**JSON-EVENT-COUNTS-MVP** / **CROSS-COUNTS**) | `sub_return_events`, `new_fid_events`, `sub_callers_events`, `src_line_events`, `sub_info_events` | equal **27/3/13/632/31** when both expose |
| Basename (**JSON-FILE-BASENAME-MVP** / **CROSS-COUNTS**) | `file_1_basename` | exact equal **or** both contain **`workload.pl`** |
| Other optional meta (nice-to-have) | e.g. `ticks_per_sec`, `files_count` | equal only if **both** sides expose the key |

Not a full attribute/files map dump. Skip-with-NOTE if only one side exposes a group.

Native schema: [`native-aggregates-json-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-aggregates-json-mvp-v0.md). Wired into `scripts/ci/offline_gate.sh` when native CLI available (pure-Perl query path alone remains step 6 `perl_query_json_smoke`).

### Fixture contract (default-calls1)

| Field | Value |
|-------|-------|
| Profile | `fixtures/v5/default-calls1/nytprof.out` |
| Golden JSONL | `fixtures/v5/default-calls1/readstream.jsonl` |
| `main::leaf` returns | **15** |
| `main::mid` returns | **3** |
| `main::mid` → `main::leaf` edge count | **15** |
| `discount_events` (JSON) | **818** |
| `sub_entry_events` (JSON) | **0** (`calls=1`; calls2-default → **27**) |
| `is_stream_complete` (JSON) | **true** |
| `sub_def main::leaf` | **fid=1 first=3 last=7** |
| `sub_def main::mid` | **fid=1 first=8 last=12** |
| `source_line 1:5` | **`    $x++ for 1 .. 50;`** (contains `$x++` and `1 .. 50`) |
| `pid_start_count` / `pid_end_count` | **≥ 1** each (golden observes **1** / **1**) |
| matching pid | golden dump-derived **2975381** on both `pid_start` and `pid_end` |
| attribute | at least **`ticks_per_sec`** (golden **10000000**) |
| option | at least **`calls`** (golden **1**) |

### Fixture contract (blocks-calls1, expand / JSON-BLOCKS-MVP)

| Field | Value |
|-------|-------|
| Golden JSONL | `fixtures/v5/blocks-calls1/readstream.jsonl` |
| Profile | `fixtures/v5/blocks-calls1/nytprof.out` |
| `line_calls 1:5` (human) | **780** |
| `block_line_calls 1:4` (human) | **810** |
| `line_calls_1_5` (JSON) | **780** |
| `block_line_calls_1_4` (JSON) | **810** |

Related data module schema: [`perl-jsonl-data-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-jsonl-data-mvp-v0.md).

## Actions `folded` / `callgrind` / `cg` (PERL-ENGINE-EXPORT)

Shipped Perl entry that **dispatches** machine-oriented exports to the native CLI. Export formats are **not** reimplemented in Perl.

| Action | Native subcommand | Notes |
|--------|-------------------|-------|
| `folded` | `nytprof-cli folded <profile>` | Folded-stack lines (flamegraph input) |
| `callgrind` | `nytprof-cli callgrind <profile>` | Callgrind-style text |
| `cg` | `nytprof-cli cg <profile>` | Alias for `callgrind` |

Related export contract: [`export-formats-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/export-formats-mvp-v0.md).

### Fixture contract (default-calls1, native)

| Field | Value |
|-------|-------|
| Profile | `fixtures/v5/default-calls1/nytprof.out` |
| Folded mid→leaf | `main::mid;main::leaf 15` |
| Folded RUNTIME→mid | `main::RUNTIME;main::mid 3` |
| Callgrind | contains `fn=main::leaf` (or `cfn=`) and `calls` |

Legacy path: honest stream-dump smoke with a `NOTE:` that real export requires `--engine=native` (same pattern as `html` / `csv` / `dump`).

## Smoke

```sh
./scripts/packaging/perl_engine_dispatch_smoke.sh
./scripts/packaging/perl_engine_query_smoke.sh
./scripts/packaging/perl_engine_query_expand_smoke.sh
./scripts/packaging/perl_engine_query_pid_meta_smoke.sh
./scripts/packaging/perl_query_json_smoke.sh
./scripts/packaging/json_sub_entry_smoke.sh
./scripts/packaging/json_blocks_smoke.sh
./scripts/packaging/perl_engine_export_smoke.sh
./scripts/packaging/engine_auto_smoke.sh
./scripts/packaging/engine_auto_fallback_smoke.sh
prove -Iperl/lib perl/t/engine_dispatch.t
prove -Iperl/lib perl/t/engine_query_default_calls1.t
```

`perl_engine_query_expand_smoke.sh` (PERL-ENGINE-QUERY-EXPAND): default-calls1 `--jsonl` asserts **15/3/15**, `sub_def` leaf/mid ranges, `source_line 1:5` hot-loop; blocks-calls1 `--jsonl` asserts `line_calls 1:5=780` and `block_line_calls 1:4=810`; optional native profile path + prove.

`perl_engine_query_pid_meta_smoke.sh` (PERL-QUERY-PID-META): default-calls1 `--jsonl` asserts **15/3/15**, `pid_start_count`/`pid_end_count` ≥ 1, matching golden pid **2975381**, `attribute ticks_per_sec=…`, `option calls=…`; optional native profile path + prove.

`perl_query_json_smoke.sh` (QUERY-JSON-MVP / QUERY-JSON-EXPAND): default-calls1 `query --json --jsonl` ×2 + parse; asserts `ok`, `leaf_returns=15`, `mid_returns=3`, `mid_leaf_edge=15`, `subs`/`edges` maps, `discount_events=818`, `sub_entry_events=0`, `is_stream_complete` true, `incompleteness_reasons` empty, `time_line_events` / `pid_start_events` / `pid_end_events` ≥ 1; consistent fingerprint across runs; `--format=json` / `--format json` aliases; human path still greppable when `--json` absent.

`json_sub_entry_smoke.sh` (JSON-SUB-ENTRY-MVP): native `report --json` + Perl `query --json` (golden `--jsonl`; optional native dump) on default-calls1 (**`sub_entry_events` 0**) and calls2-default (**27**); real CLIs only.

`json_blocks_smoke.sh` (JSON-BLOCKS-MVP): blocks-calls1 `query --json --jsonl` ×2 + parse; asserts `line_calls_1_5=780` and `block_line_calls_1_4=810` from real dump; also asserts leaf/mid/edge **15/3/15**; default-calls1 asserts `line_calls_1_5` ≥ 1 and `block_line_calls_1_4=0`; optional native `report --json` on blocks-calls1 profile when CLI available; never `crates/` on oracle PERL5LIB.

`json_time_block_smoke.sh` (JSON-TIME-BLOCK-MVP): default-calls1 `query --json --jsonl` → `time_block_events` **0**; blocks-calls1 → **916**; optional native `report --json` when CLI available; optional golden `TIME_BLOCK` tag recount; never `crates/` on oracle PERL5LIB.

`json_file_basename_smoke.sh` (JSON-FILE-BASENAME-MVP): default-calls1 `query --json --jsonl` → `file_1_basename` equals/contains **`workload.pl`** (typically exact **`"workload.pl"`**); optional native `report --json` when CLI available; never `crates/` on oracle PERL5LIB.

`native_query_json_cross_smoke.sh` (NATIVE-QUERY-JSON-CROSS / **NATIVE-QUERY-JSON-CROSS-EXPAND** / **NATIVE-QUERY-JSON-CROSS-BLOCKS** / **NATIVE-QUERY-JSON-CROSS-META** / **NATIVE-QUERY-JSON-CROSS-TIMEBLOCK** / **NATIVE-QUERY-JSON-CROSS-COUNTS**): default-calls1 native `report --json` vs Perl `query --json --jsonl` pair ×2; asserts equal shared fields **15/3/15** + `discount_events` **818** (+ `sub_entry_events` **0** and `time_block_events` **0** when **both** sides expose); when both expose event counts equal **27/3/13/632/31** + `file_1_basename` (exact or both contain **`workload.pl`**); greppable meta (`attribute_ticks_per_sec` / `option_calls` / `file_1`) **required** equal when both expose; optional `query --json <profile>` dump path; **expand:** calls2-default side-by-side `sub_entry_events` **27** when both expose SUB_ENTRY (fixture-scoped); **blocks:** blocks-calls1 pair ×2 → `line_calls_1_5` **780** / `block_line_calls_1_4` **810** equal native↔perl + `time_block_events` **916** when both expose; fails closed without native CLI.

`engine_auto_smoke.sh` (ENGINE-AUTO-SMOKE): `--engine=auto` report ×2 + `NYTPROF_ENGINE=auto` report + `--engine=auto` query on `fixtures/v5/default-calls1/nytprof.out`; asserts `main::leaf returns=15` and `main::mid returns=3` when native is discoverable.

`engine_auto_fallback_smoke.sh` (ENGINE-AUTO-FALLBACK): (1) native present → auto report ×2 leaf **15** / mid **3**; (2) `NYTPROF_FORCE_NO_NATIVE=1` → auto report/verify exit 0 via legacy stream-dump + STDERR fallback note, no `crates/` on PERL5LIB; (3) explicit `--engine=native` + force hook → fail closed. Unit: `resolve_engine('auto')` → `auto`; `select_runtime_engine` + force hook → `legacy` in `perl/t/engine_dispatch.t`.

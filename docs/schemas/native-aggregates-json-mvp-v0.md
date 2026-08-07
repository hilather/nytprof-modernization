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
- Source of truth: `ProfileModel::sub_total`, `ProfileModel::call_edge`, `ProfileModel::discount_events`, plus full maps from `sub_return_totals` / `call_edges`.
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
| Smoke | `./scripts/packaging/native_agg_json_smoke.sh` (run ×2; assert 15/3/15) |
| Cross-check | `./scripts/packaging/native_query_json_cross_smoke.sh` (NATIVE-QUERY-JSON-CROSS) |

```sh
cargo test -p nytprof-cli --test native_agg_json
./scripts/packaging/native_agg_json_smoke.sh
cargo run -q -p nytprof-cli -- report --json fixtures/v5/default-calls1/nytprof.out
./scripts/packaging/native_query_json_cross_smoke.sh
```

## Cross-check vs Perl `query --json` (NATIVE-QUERY-JSON-CROSS)

Shared convenience fields are stable across **native** `report --json` and **Perl** `nytprof-engine query --json` (golden `--jsonl` and optional dump-of-profile). Do **not** reimplement aggregation in the smoke — invoke real CLIs and parse JSON.

| Shared field | default-calls1 |
|--------------|----------------|
| `leaf_returns` | **15** |
| `mid_returns` | **3** |
| `mid_leaf_edge` | **15** |
| `discount_events` | **818** (equal between sides; golden contract) |

Smoke: `./scripts/packaging/native_query_json_cross_smoke.sh` (pair ×2 + optional `query --json <profile>`). Wired into `offline_gate.sh` when native CLI available. Perl-only schema: [`perl-engine-dispatch-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-engine-dispatch-mvp-v0.md).

## Explicit non-requirements

| Out of scope | Notes |
|--------------|-------|
| Full A4/A4b/A8/A9 JSON maps | Later expansion; this MVP is returns/edges/discount + convenience ints |
| Tick/time fields in JSON | Counts only; ticks under COMPAT-003 |
| Pretty multi-line JSON | Compact single-line is the contract |
| Replacing `nytprof-engine query --json` | Perl path remains; native path is independent; cross-smoke only asserts shared fields |
| Oracle PERL5LIB mutation | Never put `crates/` on oracle `PERL5LIB` |

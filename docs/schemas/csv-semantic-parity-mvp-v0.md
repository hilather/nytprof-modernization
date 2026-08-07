# CSV semantic parity MVP (v0)

**Status:** first-slice semantic checklist for the **native CSV** path (not full Reader / `nytprofcsv` dialect)  
**Board ID:** `CSV-SEMANTIC-PARITY`  
**Related:** [report-semantic-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/report-semantic-parity-mvp-v0.md) (default-calls1 leaf/mid/edge HTML), [aggregate-comparison-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/aggregate-comparison-v0.md) (A5/A7)  
**Not:** full legacy `nytprofcsv` Reader per-line CSV dialect / `--delim` / `--annotated` / byte-identical oracle CSV

## Profile under test

| Field | Value |
|-------|-------|
| Fixture | `fixtures/v5/default-calls1/nytprof.out` |
| Workload | `fixtures/v5/default-calls1/workload.pl` (`mid` ×3 → `leaf` ×5) |
| Oracle aggregates | `fixtures/v5/default-calls1/aggregates.oracle.json` |
| Aggregate contract | [aggregate-comparison-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/aggregate-comparison-v0.md) |

Golden counts (oracle A5 / A7):

| Check | Source | Expected |
|-------|--------|----------|
| `main::leaf` returns | A5 `sub_return_totals` | **15** |
| `main::mid` returns | A5 `sub_return_totals` | **3** |
| `main::mid` → `main::leaf` call count | A7 `call_edges` | **15** |

These numbers are fixed by the committed fixture and `aggregates.oracle.json`. Tests **must** load the real profile via `ProfileModel::from_path` (or the shipped CLI path) and assert against real model APIs / rendered CSV — not invent unrelated constants.

## Semantic checklist (required)

1. **Model API** — after `ProfileModel::from_path` on `fixtures/v5/default-calls1/nytprof.out`:
   - `sub_total("main::leaf").returns == 15` (or `sub_returns`)
   - `sub_total("main::mid").returns == 3`
   - `call_edge("main::mid", "main::leaf").count == 15`
2. **Native subs CSV** — `render_subs_csv` contains:
   - header `name,returns,incl,excl`
   - row prefix `main::leaf,15,`
   - row prefix `main::mid,3,`
3. **Native edges CSV** — `render_edges_csv` contains:
   - header `caller,called,count,incl,excl`
   - row prefix `main::mid,main::leaf,15,`
4. **Native dual-section CSV** — `render_csv_report` (default `nytprof-cli csv` stdout) contains:
   - section markers `# subroutines` and `# call_edges`
   - the same leaf/mid/edge row prefixes as above
5. **CLI smoke** — `nytprof-cli csv` on the fixture (cargo or prefix) emits the dual-section form with those patterns; run **twice** for stability (identical stdout preferred; both runs must match the semantic patterns).

## Native side

```sh
# Dual-section (default)
cargo run -q -p nytprof-cli -- csv fixtures/v5/default-calls1/nytprof.out

# Subs or edges only
cargo run -q -p nytprof-cli -- csv --subs fixtures/v5/default-calls1/nytprof.out
cargo run -q -p nytprof-cli -- csv --edges fixtures/v5/default-calls1/nytprof.out

# Prefix install (if present)
prefix/bin/nytprof-cli csv fixtures/v5/default-calls1/nytprof.out
```

Library entry points (shipped report path):

- `nytprof_report::render_subs_csv`
- `nytprof_report::render_edges_csv`
- `nytprof_report::render_csv_report`
- `ProfileModel::from_path` / `sub_total` / `call_edge`

Native CSV shape (A5/A7 columns, not Reader layout):

```text
# subroutines
name,returns,incl,excl
main::leaf,15,<incl>,<excl>
main::mid,3,<incl>,<excl>
# call_edges
caller,called,count,incl,excl
main::mid,main::leaf,15,<incl>,<excl>
```

Tick columns (`incl`/`excl`) are present but **not** required to match oracle floating formatting for this checklist (see COMPAT-003).

## Oracle side (optional / out of scope for equality)

Full `nytprofcsv` dialect comparison is **not** required. Optional operator spot-check under isolated oracle `PERL5LIB` may invoke `nytprofcsv` for exploratory comparison only; do not fail this board on Reader layout differences.

```sh
source tools/oracle/env.sh
# PERL5LIB must not contain crates/
# nytprofcsv -f fixtures/v5/default-calls1/nytprof.out   # optional only
```

## How ticks / time are treated

| Field class | Parity rule for this MVP |
|-------------|---------------------------|
| **Counts** (returns, call-edge count) | **Exact** — must match oracle aggregates / model |
| **Time ticks** (incl/excl in CSV columns) | **Not required** for this checklist. Compare only under **COMPAT-003** when that contract is frozen |

Do not fail semantic parity solely because displayed tick strings differ in formatting or floating conversion.

## Explicit non-requirements

- Full legacy `nytprofcsv` / Reader per-line CSV dialect parity
- `--delim` / `--annotated` / per-file CSV directories
- Byte-identical CSV to oracle
- Tick/time equality (see COMPAT-003)
- Full REPORT-012 plan matrix beyond A5/A7 dual-section MVP
- COL-008 (batched Rust writer) — out of scope

## Verification

| Gate | Command |
|------|---------|
| Schema + checklist (this file) | Read / review |
| Rust model + CSV render | `cargo test -p nytprof-report csv_semantic_parity_default_calls1` |
| Operator smoke (native CLI ×2) | `bash tools/oracle/csv_semantic_parity.sh` |

Evidence paths land on the first-slice board as `CSV-SEMANTIC-PARITY`.

## Relation to existing tests

Existing coverage that this board consolidates under a clearly named gate:

| Test | Crate | Overlap |
|------|-------|---------|
| `subs_csv_default_calls1_real_render` | nytprof-report | leaf,15 / mid,3 row prefixes |
| `edges_csv_default_calls1_real_render` | nytprof-report | mid→leaf count 15 |
| `csv_report_dual_section` | nytprof-report | dual-section markers + rows |
| `csv_escape_quotes_when_needed` | nytprof-report | RFC 4180 escaping only |

The dedicated test `csv_semantic_parity_default_calls1` is the named gate for this board: real `from_path` + exact model 15/3/15 + all three CSV render paths.

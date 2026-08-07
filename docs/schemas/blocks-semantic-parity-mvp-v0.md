# Blocks semantic parity MVP (v0)

**Status:** first-slice semantic checklist for the **blocks** fixture path (not full DOM parity)  
**Board ID:** `BLOCKS-SEMANTIC-PARITY`  
**Related:** [report-semantic-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/report-semantic-parity-mvp-v0.md) (default-calls1 leaf/mid/edge), [aggregate-comparison-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/aggregate-comparison-v0.md) (A4/A4b), [html-report-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-report-mvp-v0.md)  
**Not:** full `nytprofhtml` multi-file DOM / CSS / tablesorter / REPORT-001..020 block pages

## Profile under test

| Field | Value |
|-------|-------|
| Fixture | `fixtures/v5/blocks-calls1/nytprof.out` |
| Workload | `fixtures/v5/blocks-calls1/workload.pl` (`mid` ×3 → `leaf` ×5; hot loop `$x++ for 1 .. 50`) |
| NYTPROF env | `trace=0:start=begin:calls=1:blocks=1` (statement timing via **`TIME_BLOCK`**, not `TIME_LINE`) |
| Oracle aggregates | `fixtures/v5/blocks-calls1/aggregates.oracle.json` |
| Aggregate contract | [aggregate-comparison-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/aggregate-comparison-v0.md) |

Golden counts (oracle A4 / A5; fixed by the committed fixture):

| Check | Source | Expected |
|-------|--------|----------|
| `line_total(1, 5).calls` (hot loop) | A4 `line_totals` from **TIME_BLOCK** | **780** |
| `main::leaf` returns | A5 `sub_return_totals` | **15** |
| `main::mid` returns | A5 `sub_return_totals` | **3** |

Supporting (same fixture; asserted when present on the model path):

| Check | Source | Expected |
|-------|--------|----------|
| `time_block_events` | model counters | **> 0** |
| `time_line_events` | model counters | **0** (blocks=1 → no TIME_LINE) |
| A4b `block_line_totals` | non-empty map | at least one entry with positive calls |

These numbers are fixed by the committed fixture and `aggregates.oracle.json`. Tests **must** load the real profile via `ProfileModel::from_path` (or the shipped CLI path) and assert against real model APIs / rendered output — not invent unrelated constants. It is OK to `assert_eq!(780)` (etc.) **after** a real `from_path` load.

## Semantic checklist (required)

1. **Model API** — after `ProfileModel::from_path` on `fixtures/v5/blocks-calls1/nytprof.out`:
   - `line_total(1, 5).calls == 780` (A4 from TIME_BLOCK)
   - `sub_total("main::leaf").returns == 15` (or `sub_returns`)
   - `sub_total("main::mid").returns == 3`
   - Prefer also: `time_block_events > 0`, `time_line_events == 0`, non-empty `block_line_totals`
2. **Native single-file HTML** — `render_html_summary` (or `nytprof-cli html <profile> -o <path.html>`) contains:
   - names `main::leaf` and `main::mid`
   - returns **15** and **3** in the subroutine table context
   - line-calls evidence for **780** (source / line table cell for line 5, or any clear A4 calls cell matching the model)
3. **Native multi-file HTML (preferred)** — `render_html_site` / `nytprof-cli html <profile> --out-dir DIR` surfaces the same leaf/mid numbers on the index and line-calls **780** on a source page (`source.html` or `file-1.html`)

## Oracle side (optional for this board)

Full oracle `nytprofhtml` generation under isolated `PERL5LIB` is covered by `REPORT-SEMANTIC-PARITY` for default-calls1. For blocks-calls1 this MVP **does not require** oracle HTML DOM comparison. Operator smoke may still exercise native CLI only.

If oracle HTML is run:

```sh
source tools/oracle/env.sh
# PERL5LIB must not contain crates/
nytprofhtml -o <tmpdir>/oracle-html -f fixtures/v5/blocks-calls1/nytprof.out
```

Success criterion if run: non-empty HTML under the output dir. Full DOM / visual parity with native remains **out of scope**.

## Native side

```sh
# Single-file
cargo run -q -p nytprof-cli -- html fixtures/v5/blocks-calls1/nytprof.out -o <tmpdir>/native.html

# Multi-file site
cargo run -q -p nytprof-cli -- html fixtures/v5/blocks-calls1/nytprof.out --out-dir <tmpdir>/native-site
```

Library entry points (shipped report path):

- `nytprof_report::render_html_summary`
- `nytprof_report::render_html_site` / `write_html_site`
- `ProfileModel::from_path` / `line_total` / `sub_total`

Related HTML shape schemas (not full oracle DOM):  
[html-report-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-report-mvp-v0.md),  
[html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md),  
[html-per-file-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-per-file-mvp-v0.md).

## How ticks / time are treated

| Field class | Parity rule for this MVP |
|-------------|---------------------------|
| **Counts** (line calls 780, sub returns 15/3, event presence) | **Exact** — must match oracle aggregates / model |
| **Time ticks** (line/block ticks, sub incl/excl) | **Not required** for this checklist. Compare only under **COMPAT-003** when that contract is frozen |

Do not fail semantic parity solely because displayed tick strings differ in formatting or floating conversion.

## Explicit non-requirements

- Full `nytprofhtml` DOM / CSS / tablesorter / flame / JS visualization parity
- REPORT-001..020 complete report matrix / dedicated block pages
- Byte-identical HTML to oracle
- Tick/time equality (see COMPAT-003)
- A4b block_line value hard-coding in the primary test (A4b non-empty is supporting; detailed A4b HTML is HTML-A4B-BLOCKS)
- COL-008 (batched Rust writer) — out of scope

## Verification

| Gate | Command |
|------|---------|
| Schema + checklist (this file) | Read / review |
| Rust model + HTML render | `cargo test -p nytprof-report blocks_semantic_parity_blocks_calls1` |
| Operator smoke (native CLI) | `bash tools/oracle/blocks_semantic_parity.sh` |

Evidence paths land on the first-slice board as `BLOCKS-SEMANTIC-PARITY`.

## Relation to existing tests

Existing coverage that this board consolidates under a clearly named gate:

| Test | Crate | Overlap |
|------|-------|---------|
| `blocks_calls1_workload_subs` | nytprof-model | line 5 calls 780; leaf 15; mid 3 |
| `html_summary_blocks_calls1_line_calls` | nytprof-report | HTML A4 line calls + leaf/mid |
| `html_site_blocks_calls1_source_line_calls` | nytprof-report | multi-file source line calls |
| `html_*_blocks_calls1_block_line_totals` | nytprof-report | A4b HTML (HTML-A4B-BLOCKS) |

The dedicated test `blocks_semantic_parity_blocks_calls1` is the named gate for this board: real `from_path` + exact 780/15/3 + HTML evidence.

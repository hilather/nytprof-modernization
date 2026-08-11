# Report surface contract (provisional freeze) — v0

**Status:** provisional freeze of **advertised** native report outputs for this program slice  
**Board ID:** `REPORT-CONTRACT-FREEZE`  
**Date:** 2026-08-07  
**Depends on:** REPORT-SEMANTIC-PARITY, BLOCKS-SEMANTIC-PARITY, CSV-SEMANTIC-PARITY, EXPORT-SEMANTIC-PARITY, VERIFY-CLI, NATIVE-DUMP-PARITY / DUMP-PARITY-EXPAND, HTML-*, BASE-005-INV, COMPAT-004-CLASS  
**Gate:** done **before COL-007** (C v6 writer)

---

## Scope and non-claims

This document freezes what the **native** first-slice program **advertises** as operator-facing report/export/verify surfaces, and the **semantic counts** those surfaces must prove on committed fixtures.

It does **not**:

- freeze full plan **REPORT-001..020** (artifact catalog, report IR, flame, Graphviz, parallel render, visual a11y, etc.);
- freeze full `nytprofhtml` multi-file DOM / CSS / tablesorter / JS parity;
- freeze full Reader / `nytprofcsv` per-line dialect or full `nytprofcg` / KCacheGrind byte-id layout;
- freeze tick/time string equality (counts are exact; ticks only under COMPAT-003);
- claim legacy-tool retirement or engine-default flips.

Downstream of this freeze: report-side evidence is sufficient to start **COL-007** without expanding the advertised report matrix in the same slice.

Related inventories / classification (not re-frozen here):

| Doc | Path |
|-----|------|
| CLI / report inventory (BASE-005) | [`baseline/inventories/cli-report-surface.md`](https://github.com/hilather/nytprof-modernization/blob/main/baseline/inventories/cli-report-surface.md) |
| Surface classification (COMPAT-004 provisional) | [`docs/contracts/COMPAT-004_SURFACE_CLASSIFICATION.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-004_SURFACE_CLASSIFICATION.md) |
| Full plan report tasks | [`docs/plan/08_REPORT_GENERATION_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/08_REPORT_GENERATION_TASKS.md) |

---

## Advertised native surfaces (frozen disposition)

| Surface | CLI | Status | Residual vs full nytprofhtml / REPORT-001..020 |
|---------|-----|--------|--------------------------------------------------|
| text report | `report` / `summary` | **advertised** | Human text summary from model (A5/A7 counts). Not full REPORT-001..020 artifact matrix, not HTML/CSS/JS site. |
| aggregates JSON | `report --json` / `aggregates` | **advertised** | Structured JSON from real ProfileModel (`ok`/`profile`/`leaf_returns`/`mid_returns`/`mid_leaf_edge`/`discount_events`/`subs`/`edges`). Schema [`docs/schemas/native-aggregates-json-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-aggregates-json-mvp-v0.md). Not full A1–A9 JSON export or Data.pm. |
| HTML single-file | `html -o` | **advertised** | Single HTML summary (subs, edges, excl ranking, workload source/line totals, optional A4b table) with **inline** shared MVP CSS (`SHARED_STYLE_CSS`). Not full `nytprofhtml` DOM, tablesorter, flame, or Graphviz. Structure: [html-shared-css-structure-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md). |
| HTML multi-file | `html --out-dir` | **advertised** | `index.html` + **`index-subs-excl.html`** (exclusive ranking) + per-fid `file-*.html` + `source.html` + shared **`style.css`** (MVP); atomic publish + out-dir safety. Not full oracle site layout, Shared JS/tablesorter, oracle `get_css()`, flame SVG, or Graphviz `.dot`. Schema [html-subs-excl-index-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-subs-excl-index-mvp-v0.md). |
| CSV dual-section | `csv` | **advertised** | Dual-section subs + call_edges CSV (`# subroutines` / `# call_edges`). **Not** Reader line-CSV dialect / per-file oracle `nytprofcsv` layout / `--delim` / `--annotated`. |
| callgrind | `callgrind` / `cg` | **advertised** | Callgrind-style text with contracted names/counts. **Not** full KCacheGrind byte-id / microsecond scaling / legacy `nytprofcg` byte identity. |
| folded | `folded` | **advertised** | Folded stacks from call edges (`caller;called count`). Not full multi-file `nytprofcalls` stream path / `--calls` dialect freeze. |
| verify | `verify` / `inspect` | **advertised** | Profile health check (`OK:` + event/file/sub counters); fail-closed on corrupt/incomplete input. Not full SEC recovery / salvage taxonomy. |
| dump | `dump` | **advertised** | Canonical event stream (JSONL) structural parity vs golden after normalize. Dump is the decode spine, not a human report matrix. |

**Not advertised** on the native path in this slice (remain legacy-only / open; do not claim native parity):

- Flame graph / `flamegraph.pl` integration  
- Graphviz / `.dot` call-graph pages  
- Full block/sub-level report pages beyond A4/A4b tables already on HTML MVP  
- Shared **JS** / tablesorter / treemap and other oracle-only site assets (native MVP **`style.css`** / inline CSS and multi-file **`index-subs-excl.html`** **are** advertised — not oracle CSS/DOM parity)  
- `nytprofmerge`  
- Full plan REPORT-001..020 deliverables (report IR, parallel scheduler, visual regression suite, compact mode, etc.)

**Artifact-level residual honesty** (oracle `nytprofhtml` vs native paths on default-calls1 — which classes exist, which are residual):  
[`docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md) (`REPORT-HTML-RESIDUAL-INV`).

---

## Frozen semantic counts

### default-calls1 (primary report parity fixture)

| Check | Source | Expected |
|-------|--------|----------|
| `main::leaf` returns | A5 `sub_return_totals` | **15** |
| `main::mid` returns | A5 `sub_return_totals` | **3** |
| `main::mid` → `main::leaf` call count | A7 `call_edges` | **15** |

Fixture: `fixtures/v5/default-calls1/nytprof.out`  
Workload: `fixtures/v5/default-calls1/workload.pl` (`mid` ×3 → `leaf` ×5)  
Oracle aggregates: `fixtures/v5/default-calls1/aggregates.oracle.json`

These counts are **exact** for every advertised surface that exposes sub returns and/or call edges (text report, HTML single/multi, CSV, callgrind, folded, model APIs). Tests must load the real profile (or shipped CLI) — not hard-code theater without a real `from_path` / CLI path.

### blocks-calls1 (A4 / TIME_BLOCK reference)

| Check | Source | Expected |
|-------|--------|----------|
| `line_total(1, 5).calls` (hot loop) | A4 `line_totals` from **TIME_BLOCK** | **780** |

Fixture: `fixtures/v5/blocks-calls1/nytprof.out`  
Also required on that path when asserted: leaf returns **15**, mid returns **3** (same workload shape).  
Schema: [blocks-semantic-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/blocks-semantic-parity-mvp-v0.md).

### How ticks / time are treated

| Field class | Rule under this freeze |
|-------------|------------------------|
| **Counts** (returns, call-edge count, line calls) | **Exact** — must match model / oracle aggregates |
| **Time ticks** (incl/excl, line ticks, displayed time strings) | **Not** frozen here; compare only under **COMPAT-003** |

---

## Explicit NOT full REPORT-001..020 / DOM / flame / Graphviz

This provisional freeze is **not** completion of plan package REPORT-001..020:

| Plan area (examples) | Status relative to this freeze |
|----------------------|--------------------------------|
| REPORT-001 full artifact catalog + golden corpus | **not** done — this v0 freezes **advertised MVP surfaces + counts only** |
| REPORT-002 deterministic report IR | **not** done |
| Full index / source / block / sub pages (REPORT-004..006) | **partial** HTML MVP only |
| Flame (REPORT-008) / Graphviz (REPORT-009) | **not** advertised native |
| Parallel render / telemetry / visual a11y (REPORT-010, 018, 019) | **not** in this slice |
| Full `nytprofhtml` DOM / CSS / tablesorter / JS | **out of scope** |
| Byte-identical HTML / CSV / callgrind vs oracle tools | **out of scope** |

Claims of “report parity” in this program slice mean **exact semantic counts** on the fixtures above for advertised surfaces — not visual or byte-identical legacy reports.

---

## Links to existing MVP schemas

| Surface / gate | Schema |
|----------------|--------|
| Aggregate spine (A4/A5/A7/…) | [aggregate-comparison-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/aggregate-comparison-v0.md) |
| Report semantic parity (HTML leaf/mid/edge) | [report-semantic-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/report-semantic-parity-mvp-v0.md) |
| HTML residual inventory (oracle vs native artifacts) | [REPORT_HTML_RESIDUAL_INVENTORY_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md) |
| Blocks semantic parity (line 5 calls 780) | [blocks-semantic-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/blocks-semantic-parity-mvp-v0.md) |
| HTML single-file | [html-report-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-report-mvp-v0.md) |
| HTML multi-file | [html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md) |
| HTML exclusive sub index | [html-subs-excl-index-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-subs-excl-index-mvp-v0.md) |
| HTML shared CSS + structure | [html-shared-css-structure-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md) |
| HTML per-file pages | [html-per-file-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-per-file-mvp-v0.md) |
| HTML `--out-dir` safety | [html-outdir-safety-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-outdir-safety-mvp-v0.md) |
| CSV semantic parity | [csv-semantic-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/csv-semantic-parity-mvp-v0.md) |
| Export formats (callgrind / folded shape) | [export-formats-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/export-formats-mvp-v0.md) |
| Export semantic parity | [export-semantic-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/export-semantic-parity-mvp-v0.md) |
| Verify / inspect | [verify-cli-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/verify-cli-mvp-v0.md) |
| Dump structural parity | [native-dump-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-dump-parity-mvp-v0.md) |
| Canonical event dump shape | [canonical-event-dump-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/canonical-event-dump-v0.md) |
| Fail-closed corrupt input | [COMPAT-010_ERROR_FAIL_CLOSED.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-010_ERROR_FAIL_CLOSED.md) |
| Incomplete stream | [COMPAT-010_INCOMPLETE_STREAM.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md) |

---

## Contract evidence smoke (default-calls1 → 15 / 3 / 15)

Primary operator proof that **oracle** `nytprofhtml` (isolated `PERL5LIB` only — **never** `crates/`) and **native** HTML agree on frozen counts:

```sh
# From repo root
bash tools/oracle/report_semantic_parity.sh

# Optional durable HTML capture for review / CI artifacts:
REPORT_PARITY_KEEP_DIR=/tmp/report-contract-freeze-evidence \
  bash tools/oracle/report_semantic_parity.sh
```

What the smoke proves:

1. Oracle `nytprofhtml -o … -f fixtures/v5/default-calls1/nytprof.out` under `tools/oracle/env.sh` (baseline/6.15 only; assert no `/crates/` on `PERL5LIB`) produces a non-empty HTML site.  
2. Native `html -o` contains `main::leaf` returns **15**, `main::mid` returns **3**, mid→leaf edge **15**.  
3. Native `html --out-dir` index contains the same counts.

Optional companion gates (same count contract on other surfaces):

| Gate | Command |
|------|---------|
| Rust HTML + model | `cargo test -p nytprof-report report_semantic_parity_default_calls1` |
| Blocks line 5 = 780 | `bash tools/oracle/blocks_semantic_parity.sh` |
| CSV dual-section | `bash tools/oracle/csv_semantic_parity.sh` |
| Folded + callgrind | `bash tools/oracle/export_semantic_parity.sh` |
| Dump structural | `bash tools/oracle/selftest_native_dump_parity.sh` |

---

## Re-verify checklist

1. Read this contract — advertised table + frozen counts still match what operators are told.  
2. Re-run `bash tools/oracle/report_semantic_parity.sh` (must pass with leaf=15, mid=3, mid→leaf=15).  
3. Optionally re-run blocks / CSV / export / dump smokes above before expanding COL-007 or advertising new surfaces.  
4. Any **new** advertised surface or count change requires a **contract revision** (new vN or explicit amendment) and board update — do not silently expand the freeze.

---

## Board placement

| ID | Status | Evidence |
|----|--------|----------|
| `REPORT-CONTRACT-FREEZE` | **done** (this slice) | this file + `tools/oracle/report_semantic_parity.sh` (and related MVP schemas) |
| `REPORT-HTML-RESIDUAL-INV` | done | [`REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md) |
| `COL-007` | deferred until after report-side evidence | C v6 writer — unblocked for *start* by this freeze; not implemented here |

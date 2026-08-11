# Report HTML residual inventory (oracle `nytprofhtml` vs native) — v0

**Status:** in-repo inventory of oracle HTML site artifacts vs advertised native HTML paths  
**Board ID:** `REPORT-HTML-RESIDUAL-INV`  
**Date:** 2026-08-07  
**Profile under test:** `fixtures/v5/default-calls1`  
**Gate:** done **before COL-007** (C v6 writer)

**Depends on / honesty anchors:**

| Doc | Role |
|-----|------|
| [`REPORT_SURFACE_CONTRACT_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_SURFACE_CONTRACT_v0.md) | Advertised native report freeze + residual honesty |
| [`R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) | Offline R0/R1-preview vs residual full R1 |
| [`report-semantic-parity-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/report-semantic-parity-mvp-v0.md) | Exact leaf/mid/edge counts; non-empty oracle site |
| [`html-report-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-report-mvp-v0.md) | Native single-file HTML |
| [`html-multifile-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md) | Native multi-file site |
| [`html-per-file-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-per-file-mvp-v0.md) | Native `file-<fid>.html` |
| [`baseline/inventories/cli-report-surface.md`](https://github.com/hilather/nytprof-modernization/blob/main/baseline/inventories/cli-report-surface.md) | BASE-005 oracle CLI inventory |

This document **does not** claim full `nytprofhtml` DOM / CSS / tablesorter / flame / Graphviz parity. It freezes an **honest residual map** so COL-007 and later report work know what native ships today versus what remains legacy-only.

---

## Scope and non-claims

| In scope | Out of scope |
|----------|--------------|
| Artifact **classes** produced by 6.15 `nytprofhtml` on default-calls1 | Byte-identical HTML to oracle |
| Whether native has a path for each class | Tick/time string equality (COMPAT-003) |
| Exact semantic counts on native HTML (15 / 3 / 15) | Full plan REPORT-001..020 |
| How to regenerate the oracle site under isolated `PERL5LIB` | Closing residuals in this board |

**Residual honesty (from REPORT_SURFACE_CONTRACT):** native HTML is an MVP summary + multi-file site (including MVP shared `style.css` / documented inline CSS policy — **not** oracle CSS/JS). Optional `--flame` (off by default) publishes folded+native SVG site artifacts — **not** oracle default-on `flamegraph.pl` / multi-frame stacks. Graphviz, treemap, Shared JS / tablesorter, block/sub-level oracle page modes, and full DOM remain residual.

---

## How to produce the oracle site

Isolation is mandatory: **never** put `crates/` (or the candidate `perl/` facade) on oracle `PERL5LIB`.

```sh
# From repo root — shared helper sets PERL5LIB from baseline/6.15 only
source tools/oracle/env.sh

# Assert no crates/ on PERL5LIB
case ":${PERL5LIB-}:" in *"/crates/"*) echo "ERROR: crates/ on PERL5LIB"; exit 1;; esac

# Runtime dep: nytprofhtml requires File::Which (oracle Makefile.PL).
# Install into the local gitignored tree if missing:
#   cpanm -L baseline/6.15/test-deps File::Which
# tools/oracle/report_semantic_parity.sh bootstraps this when cpanm/cpan is available.

nytprofhtml -o /tmp/oracle-html-default-calls1 \
  -f fixtures/v5/default-calls1/nytprof.out
# equivalent bare form:
# nytprofhtml -o /tmp/oracle-html-default-calls1 fixtures/v5/default-calls1/nytprof.out
```

Operator smoke (oracle + native counts + non-empty oracle site):

```sh
bash tools/oracle/report_semantic_parity.sh

# Durable capture of both trees:
REPORT_PARITY_KEEP_DIR=/tmp/report-html-residual-evidence \
  bash tools/oracle/report_semantic_parity.sh
```

Optional artifact lister (oracle site generate + classify; optional native listing):

```sh
bash tools/oracle/list_html_artifacts.sh
# or:
OUT_DIR=/tmp/oracle-html-list bash tools/oracle/list_html_artifacts.sh
LIST_NATIVE=1 bash tools/oracle/list_html_artifacts.sh
```

Sources for oracle artifact names: `baseline/6.15/src/bin/nytprofhtml` (index, flame, dots, treemap, style.css, `output_js_files`), `baseline/6.15/src/lib/Devel/NYTProf/Reader.pm` (`fname_for_fileinfo` → `{safe}-{fid}-{level}.html`), share tree under `baseline/6.15/install/lib/.../Devel/NYTProf/js/`.

---

## Frozen semantic counts (default-calls1)

These are exact for every advertised native surface that exposes them (including HTML). Oracle HTML is checked only for a **non-empty site** under isolated env (not DOM equality).

| Check | Source | Expected |
|-------|--------|----------|
| `main::leaf` returns | A5 `sub_return_totals` | **15** |
| `main::mid` returns | A5 `sub_return_totals` | **3** |
| `main::mid` → `main::leaf` call count | A7 `call_edges` | **15** |

Fixture: `fixtures/v5/default-calls1/nytprof.out`  
Workload: `mid` ×3 → `leaf` ×5 (`fixtures/v5/default-calls1/workload.pl`)  
Oracle aggregates: `fixtures/v5/default-calls1/aggregates.oracle.json`  
Evidence smoke: `tools/oracle/report_semantic_parity.sh` (native leaf **15** / mid **3** / mid→leaf **15**; non-empty oracle `index.html` or any non-empty `*.html`).

---

## Artifact class matrix (default-calls1)

Legend for **residual?**:

| Value | Meaning |
|-------|---------|
| **no** | Native path exists and is **advertised** for this class (MVP shape, not oracle DOM) |
| **partial** | Native covers a related concern with different layout/names/contents |
| **yes** | Oracle-only (or native explicitly not advertised) — residual honesty |

| Artifact class | Oracle (yes / example names) | Native (yes / path) | residual? |
|----------------|------------------------------|---------------------|-----------|
| Index / home page | **yes** — `index.html` (summary, top subs, file table, optional flame embed, Graphviz links) | **yes** — multi-file: `{out-dir}/index.html`; single-file: entire summary is one HTML document (`html -o` / stdout) | **partial** — counts advertised; layout/DOM not oracle |
| Full sub index (excl sort) | **yes** — `index-subs-excl.html` | **no** dedicated page; sub table lives on `index.html` / single-file summary | **yes** |
| Exclusive-time ranking | **yes** — top-N on index + full excl index page | **yes** — “Top exclusive” section on single-file and multi-file index | **partial** — section only, not oracle page/CSS |
| Per-file / line source pages | **yes** — `{html_safe}-{fid}-line.html` (e.g. `workload-pl-1-line.html`, `warnings-pm-2-line.html` after path sanitization; names from `Reader::fname_for_fileinfo`) | **yes** — `{out-dir}/file-<fid>.html` (e.g. `file-1.html` workload, `file-2.html` warnings); primary alias `source.html` | **partial** — different naming; MVP tables (A4 calls/ticks + source), not oracle DOM |
| Block-level report pages | **yes** when profile levels include `block` — `{safe}-{fid}-block.html` (default-calls1 has `time_block_events: 0`; typically **absent** unless blocks enabled) | **partial** — A4b **Block line totals** table on HTML when model has `block_line_totals` (blocks-calls1); no oracle-style block *page mode* | **yes** (oracle page mode) / **partial** (A4b table only) |
| Sub-level report pages | **yes** when levels include `sub` — `{safe}-{fid}-sub.html` | **no** | **yes** |
| Shared CSS | **yes** — `style.css` (from `get_css()` / `_output_additional`) | **yes** — multi-file `{out-dir}/style.css` (`SHARED_STYLE_CSS`); single-file embeds the **same** CSS inline (self-contained policy). Schema [html-shared-css-structure-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md) | **partial** — native MVP shared asset **advertised**; **not** oracle `get_css()` / tablesorter CSS byte or visual parity |
| Shared JS (jquery / tablesorter / floatThead) | **yes** — `js/jquery-min.js`, `js/jquery.tablesorter.min.js`, `js/style-tablesorter.css`, sort icons `js/asc.png` `js/bg.png` `js/desc.png`; HTML also references `js/jquery.floatThead.min.js` when jquery headers emit | **no** | **yes** |
| JIT / treemap assets | **yes** — `js/jit/*` (jit.js, gradients, Treemap.css) when treemap page is generated | **no** | **yes** |
| Treemap HTML page | **yes** when `JSON::MaybeXS` (or compatible) available — `subs-treemap-excl.html` | **no** | **yes** |
| Flame graph SVG | **yes** when `--flame` (default) and `calls` option on — `all_stacks_by_time.svg` via bundled `flamegraph.pl` + `nytprofcalls` | **partial** — opt-in `html --flame` writes native `all_stacks_by_time.svg` (folded-based icicle; **not** `flamegraph.pl`); default HTML has **no** SVG (no default bloat) | **partial** — path advertised; not oracle tool/visual parity |
| Call-stack flame inputs | **yes** — `all_stacks_by_time.calls`, `flamegraph_subattr.txt` | **partial** — opt-in `all_stacks_by_time.folded` (= `render_folded_stacks` / call_edges); separate `folded` CLI remains; no `flamegraph_subattr.txt` / oracle `.calls` dialect | **partial** (site folded) / export **mapped** |
| Packages call graph (Graphviz) | **yes** non-minimal — `packages-callgraph.dot` | **no** | **yes** |
| Subs call graph (Graphviz) | **yes** non-minimal — `subs-callgraph.dot` | **no** | **yes** |
| Per-file call graph `.dot` | **yes** non-minimal — `{html_safe_filename(filestr)}.dot` for files with subs | **no** | **yes** |
| Call-edges table (caller/called/count) | **yes** (embedded in index/sub tables / call-site UI; not a separate dual-section file) | **yes** — Call edges table on single-file + multi-file index (mid→leaf **15**) | **partial** — semantic counts yes; oracle presentation residual |
| Subroutine returns table | **yes** — tablesorter sub tables on index / sub index | **yes** — Subroutines table with leaf **15** / mid **3** | **partial** |
| Source line table (A4) | **yes** — line reports with statements / time / calls columns | **yes** — source table on `file-*.html` / `source.html` / single-file source section | **partial** |
| A4b block_line totals | oracle block-mode pages when blocks profiled | **yes** on blocks fixtures via dedicated table; empty on default-calls1 | **partial** (native MVP only) |
| Multi-file site directory publish | **yes** — `-o DIR` | **yes** — `html --out-dir DIR` (atomic temp-then-rename + out-dir safety) | **no** (site path advertised; content residual as above) |
| Single self-contained HTML | **no** first-class (oracle is multi-file) | **yes** — `html -o report.html` / stdout (`render_html_summary`) | **no** (native-only convenience; not an oracle gap) |
| Browser open helper | **yes** — `--open` | **no** | **yes** |
| Delete-out-dir flag | **yes** — `-d` / `--delete` | **partial** — atomic overwrite of `--out-dir`; no exact `-d` flag | **partial** |
| Eval merge UI / `--mergeevals` | **yes** flag on oracle | **open** / not a native HTML flag | **yes** |
| Footer / version branding | **yes** — Devel::NYTProf version footer | **partial** — native titles mention NYTProf / profile path; not oracle branding | **partial** |

### Oracle share assets (copied into site `js/`)

From `baseline/6.15` install share (`Devel/NYTProf/js/`):

| Path under oracle out dir | Residual vs native |
|---------------------------|--------------------|
| `js/jquery-min.js` | **yes** |
| `js/jquery.tablesorter.min.js` | **yes** |
| `js/style-tablesorter.css` | **yes** |
| `js/asc.png`, `js/bg.png`, `js/desc.png` | **yes** |
| `js/jit/jit.js`, `jit-yc.js`, `Treemap.css`, `gradient*.png` | **yes** |

### Native multi-file layout (advertised MVP)

```text
{out-dir}/
  index.html          # summary: subs, edges, excl, links to file pages; links style.css
  source.html         # copy of primary workload file page
  file-1.html         # workload (fid 1)
  file-2.html         # typically warnings.pm (or next eligible fid)
  file-N.html         # other eligible fids
  style.css           # SHARED_STYLE_CSS (MVP; not oracle get_css)
  # only when html --flame (opt-in; default absent):
  all_stacks_by_time.svg     # native folded-based SVG
  all_stacks_by_time.folded  # call_edges folded stacks
```

No `js/`, no Graphviz `*.dot`, no `index-subs-excl.html` on this branch base. Shared CSS is MVP-native (partial residual vs oracle stylesheet). Flame files appear **only** with `--flame`.

### Native single-file layout (advertised MVP)

```text
# stdout or -o path.html — one self-contained document with:
#   inline SHARED_STYLE_CSS (<style>), event counts, subroutines, call edges,
#   top exclusive, source, optional A4b
#   optional section.flame with embedded SVG when --flame
report.html
```

---

## Residual highlights (operator summary)

1. **Semantic counts are not residual** on default-calls1: native HTML must show leaf **15**, mid **3**, mid→leaf **15**; oracle must produce a non-empty HTML site under isolated `PERL5LIB`.  
2. **Shared CSS (MVP closed as partial):** multi-file sites ship `style.css`; single-file inlines the same body — structure/CSS policy in [html-shared-css-structure-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md). Oracle `get_css()` / tablesorter CSS remain residual.  
3. **Optional flame (MVP partial):** `html --flame` (default **off**) publishes `all_stacks_by_time.svg` + `.folded` and index links — **not** oracle default-on, not `flamegraph.pl`, not multi-frame `nytprofcalls`. Schema [html-optional-flame-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-optional-flame-mvp-v0.md).  
4. **Largest residual classes** vs oracle site: shared **JS/tablesorter**, oracle **flamegraph.pl** visual/multi-frame path, **Graphviz** (`packages-callgraph.dot`, `subs-callgraph.dot`, per-file `.dot`), **treemap**, **index-subs-excl.html** (unless closed by another PR), **block/sub page modes** (`*-block.html` / `*-sub.html`).  
5. **Partial maps** (do not overclaim): multi-file `index.html` + `file-*.html` + `source.html` + MVP `style.css` vs oracle `{safe}-{fid}-line.html` + oracle CSS/JS tree; call-edges and excl ranking as **tables**, not oracle interactive widgets; opt-in folded flame ≠ oracle SVG.  
6. **Native-only (not an oracle residual to “fix”):** single-file `html -o` summary (inline CSS); flame default-off policy.  
7. **Never** put `crates/` on oracle `PERL5LIB` when regenerating this inventory.

These residuals match REPORT_SURFACE_CONTRACT **not advertised** list and R1 residual row “No full nytprofhtml DOM / REPORT-001..020”.

---

## Evidence capture procedure

| Step | Command / note |
|------|----------------|
| 1. Semantic parity smoke | `bash tools/oracle/report_semantic_parity.sh` → pass with leaf=15, mid=3, mid→leaf=15 + non-empty oracle site |
| 2. Optional durable trees | `REPORT_PARITY_KEEP_DIR=… bash tools/oracle/report_semantic_parity.sh` → `oracle-html/`, `native.html`, `native-site/` |
| 3. Artifact listing | `bash tools/oracle/list_html_artifacts.sh` (or point at an existing oracle out dir) |
| 4. Model/HTML unit gate | `cargo test -p nytprof-report report_semantic_parity_default_calls1` |
| 5. Inventory source of truth | this file + oracle `nytprofhtml` / `Reader.pm` sources under `baseline/6.15/` |

### Expected oracle classes on a typical default-calls1 run

Default flags (flame on, non-minimal) with `calls=1` profile:

| Present (typical) | May be absent / conditional |
|-------------------|-----------------------------|
| `index.html`, `index-subs-excl.html`, `style.css`, `js/**` | `*-block.html` / `*-sub.html` when only line level is active (`time_block_events: 0` on this fixture) |
| `*-{fid}-line.html` for workload + other FIDs | `subs-treemap-excl.html` without JSON::MaybeXS |
| `packages-callgraph.dot`, `subs-callgraph.dot`, per-file `.dot` | Flame artifacts if `--no-flame` or `flamegraph.pl` / `nytprofcalls` failure (oracle warns; site still has HTML) |
| Flame: `all_stacks_by_time.calls`, `all_stacks_by_time.svg`, `flamegraph_subattr.txt` when flame path succeeds | |

Exact per-file basename prefixes depend on profile FID paths and `html_safe_filename` (see `Devel::NYTProf::Util`). Re-list with `list_html_artifacts.sh` after generate rather than hard-coding full basenames in CI.

### Expected native classes on default-calls1

| Path | Notes |
|------|-------|
| single-file HTML | contains `main::leaf`/`main::mid` and table cells for 15 / 3 / mid→leaf 15; inline `SHARED_STYLE_CSS` |
| `{out-dir}/index.html` | same counts; links to `file-*.html`, `source.html`, and `style.css` |
| `{out-dir}/file-1.html` (+ ≥1 other `file-*.html`) | source + A4; hot loop text; link `style.css` |
| `{out-dir}/source.html` | copy of primary workload page |
| `{out-dir}/style.css` | shared MVP stylesheet body (`SHARED_STYLE_CSS`) |
| `{out-dir}/all_stacks_by_time.svg` + `.folded` | **only** with `html --flame`; absent by default |

---

## Linkage: residual matrix and report residual honesty

| Consumer | How this inventory is referenced |
|----------|----------------------------------|
| [`REPORT_SURFACE_CONTRACT_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_SURFACE_CONTRACT_v0.md) | Residual honesty / not-advertised list points here for **artifact-level** detail |
| [`R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) | Residual row “No full nytprofhtml DOM / REPORT-001..020” **points to this inventory** under report residual |
| Board `REPORT-HTML-RESIDUAL-INV` | **done before COL-007** with evidence path = this file |

Closing a residual class (e.g. shipping native flame SVG) requires: board/ADR + contract revision + tests — do **not** silently mark residual **no**.

---

## Re-verify checklist

1. Read this inventory — residual column still honest vs current `nytprof-report` / CLI.  
2. `bash tools/oracle/report_semantic_parity.sh` green (15 / 3 / 15 + non-empty oracle).  
3. Optionally `bash tools/oracle/list_html_artifacts.sh` and confirm oracle classes still match the table.  
4. Any new **advertised** native HTML artifact → update REPORT_SURFACE_CONTRACT + this inventory together.

---

## Board placement

| ID | Status | Evidence |
|----|--------|----------|
| `REPORT-HTML-RESIDUAL-INV` | **done** (this slice) | this file + `tools/oracle/list_html_artifacts.sh` + `tools/oracle/report_semantic_parity.sh` |
| `REPORT-HTML-SHARED-CSS` | **done** (PR-A01) | native multi-file `style.css` + single-file inline policy; structure contract [html-shared-css-structure-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md); cargo `html_shared_css_structure_contract_default_calls1` (15/3/15) |
| `REPORT-HTML-OPTIONAL-FLAME` | **done** (PR-A03) | opt-in `html --flame` (default off); `all_stacks_by_time.svg` + `.folded`; schema [html-optional-flame-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-optional-flame-mvp-v0.md); cargo `html_site_optional_flame_default_calls1` + CLI `html_optional_flame` (15/3/15). Not flamegraph.pl / Graphviz / treemap |
| `REPORT-CONTRACT-FREEZE` | done | [`REPORT_SURFACE_CONTRACT_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_SURFACE_CONTRACT_v0.md) |
| `REPORT-SEMANTIC-PARITY` | done | [`report-semantic-parity-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/report-semantic-parity-mvp-v0.md) |
| `COL-007` | deferred | C v6 writer — after report-side evidence; not implemented here |

---

## Revision rule

Expanding native HTML advertising or closing a residual class requires a **vN revision** (or explicit amendment), board update, and linked smoke/tests. This v0 is an inventory for residual honesty — not visual or byte-identical report certification.

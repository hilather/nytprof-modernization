# HTML shared CSS + structure contract (MVP v0)

**Status:** implemented (PR-A01 + operator HTML v1 + **v2** chrome/tokens; ADR-0012)  
**Board / residual:** native **shared CSS asset** + operator v2 header tokens / cell `heat-*`; does **not** claim oracle `get_css()` / tablesorter / jquery parity  
**v2 contract:** [html-operator-v2-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-operator-v2-mvp-v0.md)  
**Complements:** [html-report-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-report-mvp-v0.md), [html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md), [html-per-file-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-per-file-mvp-v0.md), [html-sort-js-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-sort-js-mvp-v0.md)  
**Inventory:** [REPORT_HTML_RESIDUAL_INVENTORY_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md)

## CSS policy (shared vs inline)

Native HTML uses **one** shared stylesheet body: `nytprof_report::SHARED_STYLE_CSS` (constant).

| Surface | How CSS is loaded | Rationale |
|---------|-------------------|-----------|
| Multi-file site (`html --out-dir DIR` / `render_html_site` / `write_html_site`) | Sibling **`style.css`** written under `DIR`; every HTML page uses `<link rel="stylesheet" href="style.css">` (**no** inline `<style>` on multi-file pages) | Matches residual inventory “shared CSS” artifact class; single asset for all pages |
| Single-file summary (`html` stdout / `-o report.html` / `render_html_summary`) | **Inline** `<style>…SHARED_STYLE_CSS…</style>` in the document head | Self-contained document — no external asset dependency |

**Rules:**

1. Multi-file and single-file CSS **text is identical** (`SHARED_STYLE_CSS`).  
2. Multi-file `write_html_site` publishes `style.css` (and `nytprof-sort.js`) atomically with `index.html` / `file-*.html` / `source.html` (same temp-then-rename path).  
3. This is **not** oracle `style.css` from `get_css()` / `_output_additional`, and **not** tablesorter / jquery / floatThead CSS or JS.  
4. Vanilla sort is a **separate** sibling asset ([html-sort-js-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-sort-js-mvp-v0.md)); jquery / tablesorter remain **WAIVE**.  
5. Heat classes are **`heat-hot` / `heat-high` / `heat-mid` / `heat-low`** only (quartile rank). Applied on **numeric cells** (`td.num.heat-*`); `tr.heat-*` remains as a fallback. Unused / zero source rows have **no** heat class. **Not** oracle `.c0`–`.c3` class names (CSS variables `--nyt-c0`…`--nyt-c3` hold the oracle palette).  
6. HTML time cells use `format_compact_secs` (6.15 `fmt_time` units) when `attributes["ticks_per_sec"]` parses as unsigned greater than 0, with `title="{ticks} ticks"`. Text, CSV, and `report --json` stay integer ticks.  
7. Multi-file pages emit `div.header` / `.siteTitle` / `.siteSubtitle` / `.header_back` (CSS linear-gradient; not stacked-div oracle chrome).

## Multi-file layout (advertised MVP, after this contract)

```text
{out-dir}/
  index.html          # summary; <link rel="stylesheet" href="style.css"> + script defer nytprof-sort.js
  source.html         # primary workload alias
  file-<fid>.html     # per-fid pages (same link + script)
  style.css           # SHARED_STYLE_CSS body (includes heat + sort-header CSS)
  nytprof-sort.js     # SHARED_SORT_JS (vanilla; not jquery)
```

## Document structure contract

Every native HTML document (single-file and multi-file pages) must:

1. Start with `<!DOCTYPE html>`, `<html lang="en">`, `<meta charset="utf-8">`, and a `<title>` mentioning NYTProf or the profile basename.  
2. Escape `<`, `>`, `&`, `"` (and `'` as `&#39;`) in all text/source via `escape_html`.  
3. Use the stable section / class markers below so operators and tests can greppably assert structure (not oracle DOM identity).

### Index / single-file summary sections

| Section | Marker(s) | Required content notes |
|---------|-----------|------------------------|
| Profile path | `p.profile-path` | Escaped profile path in `<code>` |
| Event counts | `<h2>Event counts</h2>` | At least `time_line_events` (and `time_block_events` / `discount_events` when useful) |
| Chrome | `div.header` + `.siteTitle` (`Performance Profile Index` on the multi-file index) | No back link on index; inner pages use `.header_back` `← Index` |
| Index summary | `div.index_summary` | “Profile of {app} for {secs}…”; wall `(of …)` only when a duration attribute exists |
| Subroutines | `table#subs_table.sortable` | Columns: Calls, P, F, Exclusive Time, Inclusive Time, Subroutine; index is **top 15** exclusive-desc; **default-calls1:** `main::leaf` **15**, `main::mid` **3**; names link via `model.sub_def` when present; time cells are compact units + `title=` raw ticks |
| Source files (multi-file index only) | `table#filestable.sortable` + `p.source-link` | Stmts, Exclusive Time, Reports (`line` → `file-<fid>.html`), Source File + tfoot; greppable `href="source.html"` |
| See all | `div.table_footer` + `p.subs-excl-link` | `See all N subroutines` → `index-subs-excl.html` |
| Subroutine definitions | `<h2>Subroutine definitions</h2>` + `table.sub-defs` | After `#filestable` on the index; optional when `sub_defs` empty |
| Call edges | `<h2>Call edges</h2>` + `table.call-edges` | After `#filestable`; caller, called, count, incl, excl; **default-calls1:** mid→leaf count **15** |
| Top exclusive | `<h2>Top exclusive</h2>` + `table.top-exclusive` | **Single-file only** (index uses `#subs_table`) |
| Source (single-file) | `<h2>Source — …</h2>` + `table.source` | Primary workload fid; hot loop text visible |
| Block line totals | `<h2>Block line totals</h2>` + `table.block-line-totals` | Only when A4b data present |

### Per-fid / `source.html` pages

| Marker | Role |
|--------|------|
| Link back to index | `.header_back` `← Index` / `&larr; Index` → `index.html` |
| File summary | `table.file_summary` Filename + Statements |
| Per-file subs | `table#subs_table` (same 6 columns; no top-N) |
| Source heading | `<h2>Source — {label} (fid N)</h2>` |
| `table.source` | Line, Statements, Time on line, Calls, Time in subs, Code (`td.s`); rows are the **sorted union** of `source_lines` ∪ `line_totals` ∪ `block_line_totals` for that fid; missing text is `—`; each row has `id="L{line}"`; unused/zero-tick rows have **no** heat; Calls / Time in subs are `—` until usable sites exist |
| `table.block-line-totals` | When A4b present for that fid |

### Numeric cells

Return counts, call-edge counts, and line/block calls use `td.num` (plus `data-sort-value`) so greppable cells look like `>15<` / `>3<` next to subroutine names. Incl/excl/source-ticks cells are HTML-only compact units when `ticks_per_sec` is present (`title=` holds raw ticks). Positive numeric time/count cells carry `heat-hot|heat-high|heat-mid|heat-low`.

## Semantic counts (default-calls1) — not residual

These remain **exact** on every advertised HTML path that surfaces them (single-file, multi-file index, disk publish):

| Check | Expected |
|-------|----------|
| `main::leaf` returns | **15** |
| `main::mid` returns | **3** |
| `main::mid` → `main::leaf` call count | **15** |

Fixture: `fixtures/v5/default-calls1/nytprof.out`  
Tests must load the real profile via `ProfileModel::from_path` (or shipped CLI) — no hardcoded theater detached from the model.

## Library / CLI

| API / flag | Behavior |
|------------|----------|
| `SHARED_STYLE_CSS` / `STYLE_CSS_FILENAME` | Public constant + `"style.css"` name (includes heat + `th[data-sort]` / `sort-asc` / `sort-desc`) |
| `SHARED_SORT_JS` / `SORT_JS_FILENAME` | Public constant + `"nytprof-sort.js"` (vanilla; see sort-js schema) |
| `HtmlSite.style_css` / `style_filename` | Rendered stylesheet body + name |
| `HtmlSite.sort_js` / `sort_js_filename` | Sort script body + name |
| `render_html_site` | Sets CSS/JS fields; pages use linked stylesheet + `defer` script |
| `write_html_site` | Writes `style.css` + `nytprof-sort.js` with the other site files |
| `render_html_summary` | Inlines `SHARED_STYLE_CSS` **and** `SHARED_SORT_JS` |
| `nytprof-cli html … --out-dir DIR` | Publishes `DIR/style.css` and `DIR/nytprof-sort.js`; stderr lists both |

## Explicit non-requirements (still residual)

- Oracle `get_css()` byte identity or visual polish parity  
- jquery / tablesorter / floatThead / oracle sort icons (`js/asc.png`)  
- Treemap / JIT CSS  
- Oracle `c0`–`c3` severity class names  
- Flame / Graphviz (other PR slices / map)

## Tests

| Test | Asserts |
|------|---------|
| `html_shared_css_structure_contract_default_calls1` | Real fixture; multi-file `style.css` + link policy; single-file inline policy; structure classes; **15/3/15** (incl. published disk index mid→leaf) |
| `html_site_default_calls1_render_html_site` | Index links `style.css`; no multi-file inline `<style>` |
| `write_html_site_default_calls1_tempdir` | Disk `style.css` equals `SHARED_STYLE_CSS` |
| `write_html_site_atomic_default_calls1` / `write_html_site_atomic_overwrite_same_outdir` | Atomic publish/overwrite keep `style.css` + content equality |
| `html_summary_default_calls1_real_render_path` | Inline shared CSS; no external `style.css` dependency |
| `report_semantic_parity_default_calls1` | Leaf/mid/edge counts on HTML paths |
| CLI `html_out_dir_writes_style_css_and_lists_on_stderr` | Real binary: `--out-dir` writes `style.css`, stderr lists it, index links + **15/3** |
| CLI `html_operator_v1_cli_default_calls1` | Seconds `title=`, source `id="L"`, `heat-hot` in CSS, `nytprof-sort.js`, leaf href from `model.sub_def` |

## Residual honesty

Shared CSS remains native MVP **advertised** (`style.css` + documented inline policy + heat). It does **not** close full nytprofhtml DOM, Shared JS/tablesorter/jquery, flame, or Graphviz. Vanilla sort is a **new** class (not tablesorter). See residual inventory + [REPORT_SURFACE_CONTRACT_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_SURFACE_CONTRACT_v0.md).

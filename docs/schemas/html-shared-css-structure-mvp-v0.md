# HTML shared CSS + structure contract (MVP v0)

**Status:** implemented (PR-A01 / HTML residual slice)  
**Board / residual:** closes native **shared CSS asset** residual for MVP; does **not** claim oracle `get_css()` / tablesorter / JS parity  
**Complements:** [html-report-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-report-mvp-v0.md), [html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md), [html-per-file-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-per-file-mvp-v0.md)  
**Inventory:** [REPORT_HTML_RESIDUAL_INVENTORY_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md)

## CSS policy (shared vs inline)

Native HTML uses **one** shared stylesheet body: `nytprof_report::SHARED_STYLE_CSS` (constant).

| Surface | How CSS is loaded | Rationale |
|---------|-------------------|-----------|
| Multi-file site (`html --out-dir DIR` / `render_html_site` / `write_html_site`) | Sibling **`style.css`** written under `DIR`; every HTML page uses `<link rel="stylesheet" href="style.css">` (**no** inline `<style>` on multi-file pages) | Matches residual inventory “shared CSS” artifact class; single asset for all pages |
| Single-file summary (`html` stdout / `-o report.html` / `render_html_summary`) | **Inline** `<style>…SHARED_STYLE_CSS…</style>` in the document head | Self-contained document — no external asset dependency |

**Rules:**

1. Multi-file and single-file CSS **text is identical** (`SHARED_STYLE_CSS`).  
2. Multi-file `write_html_site` publishes `style.css` atomically with `index.html` / `file-*.html` / `source.html` (same temp-then-rename path).  
3. This is **not** oracle `style.css` from `get_css()` / `_output_additional`, and **not** tablesorter / jquery / floatThead CSS or JS.  
4. No client-side sort JS in this slice (tablesorter residual remains **yes**). Zebra/hover rows are pure CSS only.

## Multi-file layout (advertised MVP, after this contract)

```text
{out-dir}/
  index.html          # summary; <link rel="stylesheet" href="style.css">
  source.html         # primary workload alias
  file-<fid>.html     # per-fid pages (same link)
  style.css           # SHARED_STYLE_CSS body
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
| Subroutines | `<h2>Subroutines</h2>` + `table.subs` | Columns: name, returns, incl, excl; **default-calls1:** `main::leaf` returns **15**, `main::mid` returns **3** |
| Subroutine definitions | `<h2>Subroutine definitions</h2>` + `table.sub-defs` | Optional when `sub_defs` empty; prefer leaf/mid fid/first/last when present |
| Call edges | `<h2>Call edges</h2>` + `table.call-edges` | caller, called, count, incl, excl; **default-calls1:** mid→leaf count **15** |
| Top exclusive | `<h2>Top exclusive</h2>` + `table.top-exclusive` | name, excl, returns; include workload subs |
| Source files (multi-file index only) | `<h2>Source files</h2>` + `ul.source-files` + `p.source-link` | Relative `href` to every `file-*.html` and to `source.html` |
| Source (single-file) | `<h2>Source — …</h2>` + `table.source` | Primary workload fid; hot loop text visible |
| Block line totals | `<h2>Block line totals</h2>` + `table.block-line-totals` | Only when A4b data present |

### Per-fid / `source.html` pages

| Marker | Role |
|--------|------|
| Link back to index | `<a href="index.html">` |
| Source heading | `<h2>Source — {label} (fid N)</h2>` |
| `table.source` | line, calls, ticks, source (`td.src-line`) |
| `table.block-line-totals` | When A4b present for that fid |

### Numeric cells

Return counts, call-edge counts, and line/block calls use `td.num` so greppable cells look like `>15<` / `>3<` next to subroutine names.

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
| `SHARED_STYLE_CSS` / `STYLE_CSS_FILENAME` | Public constant + `"style.css"` name |
| `HtmlSite.style_css` / `style_filename` | Rendered stylesheet body + name |
| `render_html_site` | Sets CSS fields; pages use linked stylesheet mode |
| `write_html_site` | Writes `style.css` with the other site files |
| `render_html_summary` | Inlines `SHARED_STYLE_CSS` |
| `nytprof-cli html … --out-dir DIR` | Publishes `DIR/style.css`; stderr lists it with other paths |

## Explicit non-requirements (still residual)

- Oracle `get_css()` byte identity or visual polish parity  
- jquery / tablesorter / floatThead / sort icons  
- Treemap / JIT CSS  
- Client-side column sort  
- Flame / Graphviz / `index-subs-excl.html` (other PR slices / map)

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

## Residual honesty

Closing this slice flips the inventory **Shared CSS** row to native MVP **advertised** (`style.css` + documented inline policy). It does **not** close full nytprofhtml DOM, Shared JS/tablesorter, flame, or Graphviz. See residual inventory + [REPORT_SURFACE_CONTRACT_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_SURFACE_CONTRACT_v0.md).

# HTML exclusive sub index page (MVP v0)

**Status:** implemented (PR-A02 / HTML residual slice)  
**Board / residual:** closes native **`index-subs-excl.html`** residual for MVP exclusive ranking page; does **not** claim oracle DOM / tablesorter / severity coloring  
**Complements:** [html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md), [html-shared-css-structure-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md), [html-report-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-report-mvp-v0.md)  
**Inventory:** [REPORT_HTML_RESIDUAL_INVENTORY_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md)

## Purpose

Oracle `nytprofhtml` writes a full exclusive-time subroutine ranking page named **`index-subs-excl.html`** (all subs, sort by `excl_time`). Native multi-file HTML previously only had a “Top exclusive” section embedded in `index.html`. This slice adds a dedicated exclusive sub index page with the **same oracle filename** so operators and residual inventory can treat the class as native-advertised MVP.

## Multi-file layout (after this contract)

```text
{out-dir}/
  index.html              # summary; links to index-subs-excl.html
  index-subs-excl.html    # full exclusive-time sub ranking (this slice)
  source.html             # primary workload alias
  file-<fid>.html         # per-fid pages
  style.css               # SHARED_STYLE_CSS
```

## Page requirements (`index-subs-excl.html`)

| Requirement | Detail |
|-------------|--------|
| Document shell | `<!DOCTYPE html>`, `<html lang="en">`, `<meta charset="utf-8">`, title mentioning exclusive index / profile basename |
| CSS | `<link rel="stylesheet" href="style.css">` — **no** multi-file inline `<style>` |
| Navigation | Link back to `index.html` |
| Profile path | `p.profile-path` with escaped profile path |
| Ranking table | `<h2>Subroutines by exclusive time</h2>` + `table.subs-excl` |
| Columns | name, returns, incl, excl |
| Sort | Exclusive ticks descending, then name ascending (stable) |
| Coverage | Every key in `ProfileModel::sub_return_totals` appears once |
| Escape | All names/paths via `escape_html` |
| Semantic (default-calls1) | `main::leaf` returns **15**, `main::mid` returns **3** |

## Index.html linkage

Multi-file `index.html` must include a relative link to the exclusive page:

- Marker: `p.subs-excl-link` with `href="index-subs-excl.html"`
- Placed after the summary “Top exclusive” section (summary table remains)

Single-file `render_html_summary` does **not** emit a separate exclusive page (self-contained summary keeps the embedded “Top exclusive” table only).

## Library / CLI

| API / flag | Behavior |
|------------|----------|
| `INDEX_SUBS_EXCL_FILENAME` | Public constant `"index-subs-excl.html"` |
| `HtmlSite.index_subs_excl_html` / `index_subs_excl_filename` | Rendered page body + name |
| `render_html_site` | Builds exclusive page; index links it |
| `write_html_site` | Publishes `index-subs-excl.html` atomically with the rest of the site |
| `nytprof-cli html … --out-dir DIR` | Writes `DIR/index-subs-excl.html`; stderr lists the path |

## Explicit non-requirements (still residual)

- Oracle `subroutine_table` DOM, MAD severity coloring, tablesorter / floatHeaders  
- Inclusive-time alternate index page (oracle only emits excl sort page by default)  
- Treemap / Graphviz / flame  
- Shared JS  

## Tests

| Test | Asserts |
|------|---------|
| `html_subs_excl_index_default_calls1` | Real fixture; render + disk page; structure classes; **15/3**; all model subs; index link |
| `html_site_default_calls1_render_html_site` | `HtmlSite` filename + index `href` |
| `write_html_site_atomic_default_calls1` / `write_html_site_default_calls1_tempdir` / overwrite | Disk publish keeps exclusive page |
| `html_shared_css_structure_contract_default_calls1` | Exclusive page links `style.css` (no inline) |
| CLI `html_out_dir_writes_index_subs_excl_and_lists_on_stderr` | Real binary: `--out-dir` writes page, stderr lists it, **15/3** |

## Residual honesty

Closing this slice flips inventory **Full sub index (excl sort)** to native MVP **partial** (dedicated page **advertised**; not oracle DOM). Exclusive-time ranking remains **partial** (summary section + full page; not interactive tablesorter). See residual inventory + [REPORT_SURFACE_CONTRACT_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_SURFACE_CONTRACT_v0.md).

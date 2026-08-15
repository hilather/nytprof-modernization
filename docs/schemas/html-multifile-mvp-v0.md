# Multi-file HTML report MVP (v0)

**Status:** implemented (MVP)  
**Complements:** single-file `html` stdout/`-o` (still supported); shared CSS + structure in [html-shared-css-structure-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md); operator v2 IA in [html-operator-v2-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-operator-v2-mvp-v0.md)

**Library:** `nytprof_report::{HtmlSite, render_html_site, write_html_site, SHARED_STYLE_CSS, STYLE_CSS_FILENAME}`  
**CLI:** `nytprof-cli html <profile.out> --out-dir DIR` (alias `--dir`; mutually exclusive with `-o`)

## CLI

```text
# Existing single document (unchanged):
nytprof-cli html <profile.out>
nytprof-cli html <profile.out> -o report.html

# Multi-file site (this wave):
nytprof-cli html <profile.out> --out-dir DIR
# alias acceptable: html --dir DIR
```

When `--out-dir DIR` is set:
1. Render the full site in memory (`render_html_site`).
2. **Atomic publish (fail-closed):** write all files into a sibling temp directory under `DIR`'s parent (e.g. `.nytprof-html-<pid>-<nanos>`), then rename into place:
   - If `DIR` does not exist: `rename(temp, DIR)`.
   - If `DIR` exists: `rename(DIR, bak)` → `rename(temp, DIR)` → remove `bak` (best-effort). On failure after moving `DIR` aside, restore from `bak` when possible.
   - On any failure before the final rename succeeds, leave an existing `DIR` unchanged (when restore works) and remove the temp dir.
3. Final layout under `DIR`:
   - **`DIR/index.html`** — summary index.
   - **`DIR/file-<fid>.html`** for each eligible fid (source and/or line/block totals) — see [html-per-file-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-per-file-mvp-v0.md).
   - **`DIR/source.html`** as a copy of the primary workload `file-<fid>.html` (back-compat).
   - **`DIR/style.css`** — shared MVP stylesheet (`SHARED_STYLE_CSS`); all HTML pages use `<link rel="stylesheet" href="style.css">` (no multi-file inline `<style>`). See [html-shared-css-structure-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md).
   - **`DIR/nytprof-sort.js`** — vanilla sort (`SHARED_SORT_JS`); pages use `<script src="nytprof-sort.js" defer></script>`. **Not** jquery / tablesorter. See [html-sort-js-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-sort-js-mvp-v0.md).
   - **`DIR/packages-callgraph.dot`** / **`DIR/subs-callgraph.dot`** — Graphviz source from `call_edges` (not `dot` PNG). Index links both.
4. Index first screen: chrome → `div.index_summary` → (optional `--flame`) → `table#subs_table` (top 15 exclusive-desc) → “See all N” → `table#filestable` → greppable `href="source.html"`. Event counts, call-edges, and sub-defs stay **below** the files table.
5. Do **not** require writing stdout HTML when `--out-dir` is used (print the DIR path or file list is fine; include `style.css` in the written-path list when listing files).

Mid-write failures must not leave a half-written final `DIR`. Same-filesystem sibling temp is preferred so `std::fs::rename` is atomic on Linux.

## index.html required content

- Chrome: `.siteTitle` **Performance Profile Index**; subtitle `For {application basename}`
- `div.index_summary` (“Profile of …”)
- `table#subs_table` (Calls / P / F / Exclusive / Inclusive / Subroutine); **default-calls1** leaf/mid **15** / **3**
- `div.table_footer` “See all N subroutines” → `index-subs-excl.html`
- `table#filestable` (Stmts, Exclusive Time, Reports `line`, Source File)
- Greppable `href="source.html"`
- `time_line_events` (and/or `time_block_events`) **below** the files table
- Call edges / sub-defs after files (when present)

## source.html required content

- Workload file label (basename)
- Table or pre listing source lines with line number, **calls** (from `line_totals` A4 when present), ticks optional, escaped source text
- Hot loop text visible (`$x++` or `for 1 .. 50`)
- On **blocks-calls1**, at least one workload line (e.g. line 5) shows **calls=780** (or matching model `line_total`)

## Library API (suggested)

```rust
pub struct HtmlSite {
  pub index_html: String,
  pub source_html: String,           // primary workload page (also source.html)
  pub source_filename: String,       // "source.html"
  pub file_pages: Vec<(String, String)>, // ("file-N.html", html)...
  pub style_css: String,             // SHARED_STYLE_CSS body
  pub style_filename: String,        // "style.css"
  pub sort_js: String,               // SHARED_SORT_JS body
  pub sort_js_filename: String,      // "nytprof-sort.js"
}
pub fn render_html_site(model: &ProfileModel, profile_path: &str) -> HtmlSite
pub fn write_html_site(...) -> Result<HtmlSite> // temp-then-rename → index + file-*.html + source.html + style.css
```

Reuse `escape_html` and primary-workload-fid selection from single-file HTML.

`write_html_site` is fail-closed: render fully, write into a sibling temp dir, then rename to `out_dir` (see CLI steps above). If `out_dir` already exists and is not a directory, return an error without publishing.

**Path safety (HTML-OUTDIR-SAFETY):** before create/write, `validate_html_out_dir` rejects empty paths, embedded null bytes, and any `..` path component. Absolute paths without those issues are allowed. See [html-outdir-safety-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-outdir-safety-mvp-v0.md).

## Tests

- Real `ProfileModel::from_path` on default-calls1
- Site render contains leaf/mid 15/3 and source hot loop
- Index contains `href` to source file
- Shared CSS: disk `style.css` == `SHARED_STYLE_CSS`; pages use `<link rel="stylesheet" href="style.css">` (`html_shared_css_structure_contract_default_calls1`, `write_html_site_default_calls1_tempdir`, atomic publish tests)
- Real CLI: `html ... --out-dir DIR` produces index + file pages + `style.css`; stderr lists `style.css` (`crates/nytprof-cli/tests/html_shared_css.rs`)
- blocks-calls1: source page has positive calls on a workload line matching model
- Atomic publish: `write_html_site_atomic_default_calls1` (disk index leaf 15 / mid 3 / mid→leaf 15 + `style.css`); `write_html_site_atomic_overwrite_same_outdir` (second write drops prior files, still has `style.css`); fail-closed when `out_dir` or its parent is a file
- Out-dir safety: `write_html_site_rejects_dotdot_component`, `write_html_site_rejects_null_byte`, `write_html_site_rejects_empty_path`

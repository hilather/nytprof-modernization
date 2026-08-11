# Multi-file HTML report MVP (v0)

**Status:** implemented (MVP)  
**Complements:** single-file `html` stdout/`-o` (still supported); shared CSS + structure in [html-shared-css-structure-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md)

**Library:** `nytprof_report::{HtmlSite, HtmlRenderOptions, render_html_site, write_html_site, SHARED_STYLE_CSS, STYLE_CSS_FILENAME}`  
**CLI:** `nytprof-cli html <profile.out> --out-dir DIR` (alias `--dir`; mutually exclusive with `-o`); optional `--flame` (default off)

## CLI

```text
# Existing single document (unchanged):
nytprof-cli html <profile.out>
nytprof-cli html <profile.out> -o report.html

# Multi-file site (this wave):
nytprof-cli html <profile.out> --out-dir DIR
# alias acceptable: html --dir DIR

# Opt-in flame artifacts (default off — no bloat):
nytprof-cli html <profile.out> --out-dir DIR --flame
# → also all_stacks_by_time.svg + all_stacks_by_time.folded
# See html-optional-flame-mvp-v0.md
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
4. Index links to all `file-*.html` pages (and to `source.html` as primary alias).
5. Do **not** require writing stdout HTML when `--out-dir` is used (print the DIR path or file list is fine; include `style.css` in the written-path list when listing files).

Mid-write failures must not leave a half-written final `DIR`. Same-filesystem sibling temp is preferred so `std::fs::rename` is atomic on Linux.

## index.html required content

- Profile path (escaped)
- `time_line_events` and/or `time_block_events`
- Subroutines table with `main::leaf` / `main::mid` and **returns 15 and 3** on default-calls1
- Optional: call edges / exclusive ranking (nice-to-have if already easy)
- Link to source page, e.g. `<a href="source.html">` or `file-1.html`
- If SUB_INFO present: show definition fid/first/last for leaf/mid (optional but preferred)

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

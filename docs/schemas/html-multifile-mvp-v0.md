# Multi-file HTML report MVP (v0)

**Status:** implemented (MVP)  
**Complements:** single-file `html` stdout/`-o` (still supported)

**Library:** `nytprof_report::{HtmlSite, render_html_site, write_html_site}`  
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
4. Index links to all `file-*.html` pages (and to `source.html` as primary alias).
5. Do **not** require writing stdout HTML when `--out-dir` is used (print the DIR path or file list is fine).

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
}
pub fn render_html_site(model: &ProfileModel, profile_path: &str) -> HtmlSite
pub fn write_html_site(...) -> Result<HtmlSite> // temp-then-rename → index + file-*.html + source.html
```

Reuse `escape_html` and primary-workload-fid selection from single-file HTML.

`write_html_site` is fail-closed: render fully, write into a sibling temp dir, then rename to `out_dir` (see CLI steps above). If `out_dir` already exists and is not a directory, return an error without publishing.

**Path safety (HTML-OUTDIR-SAFETY):** before create/write, `validate_html_out_dir` rejects empty paths, embedded null bytes, and any `..` path component. Absolute paths without those issues are allowed. See [html-outdir-safety-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-outdir-safety-mvp-v0.md).

## Tests

- Real `ProfileModel::from_path` on default-calls1
- Site render contains leaf/mid 15/3 and source hot loop
- Index contains `href` to source file
- Real CLI: `html ... --out-dir tmp` produces both files
- blocks-calls1: source page has positive calls on a workload line matching model
- Atomic publish: `write_html_site_atomic_default_calls1` (disk index leaf 15 / mid 3 / mid→leaf 15); `write_html_site_atomic_overwrite_same_outdir` (second write drops prior files); fail-closed when `out_dir` or its parent is a file
- Out-dir safety: `write_html_site_rejects_dotdot_component`, `write_html_site_rejects_null_byte`, `write_html_site_rejects_empty_path`

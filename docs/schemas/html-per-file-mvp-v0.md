# Per-file multi-page HTML MVP (v0)

**Extends:** `html-multifile-mvp-v0.md` (`html --out-dir DIR`)

## Naming

| File | Role |
|------|------|
| `index.html` | Summary index (subs, edges, links) |
| `file-<fid>.html` | One page per fid that has source text and/or line_totals / block_line_totals |

Legacy single `source.html` may remain as a **redirect/alias** to the primary workload `file-<fid>.html`, or be omitted if index links only to `file-*.html`. Prefer generating **all** `file-<fid>.html` pages plus optional `source.html` copy of the primary for back-compat.

## Index requirements

- `main::leaf` / `main::mid` returns **15** / **3** on default-calls1
- Relative links to **at least two** `file-*.html` pages for default-calls1 (workload + at least one other fid such as warnings/strict)
- Link text may use basename of the file

## File page requirements

- Title/heading with fid and path basename
- Source table: line, calls (A4), ticks, source text (escaped)
- Hot loop on workload page: `$x++` / `for 1 .. 50` and positive calls
- On blocks-calls1: A4 line 5 calls from model; **A4b section or column** showing block_line totals (see below)

## A4b in HTML

Single-file summary and/or multi-file pages for blocks fixtures must surface `block_line_totals`:

- Either a dedicated **“Block line totals”** table (fid, block_line, calls, ticks), or
- An extra **block_line / block_calls** column on the source table when A4b data exists for that fid

At least one positive A4b calls value must appear for blocks-calls1 (model-derived, not hard-coded in tests).

## CLI

Unchanged entry: `nytprof-cli html <profile.out> --out-dir DIR`

## Library

```rust
// Expanded site: index + map fid -> html body or filenames
pub fn write_html_site(...) // temp-then-rename publish of index.html + file-<fid>.html for each eligible fid
```

**Atomic publish:** `write_html_site` stages all pages in a sibling temp directory under `out_dir`'s parent, then renames into `out_dir` (bak-swap when `out_dir` already exists). Failures before the final rename leave an existing `out_dir` intact and clean up the temp dir. See [html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md).

**Path safety:** `validate_html_out_dir` rejects empty / `\0` / `..` components before create/write. See [html-outdir-safety-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-outdir-safety-mvp-v0.md).

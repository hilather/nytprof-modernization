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

- Chrome: `.siteTitle` **NYTProf Performance Profile**; subtitle includes `« line view »`; `.header_back` `← Index`
- `table.file_summary`: Filename, Statements
- Per-file `table#subs_table` (same 6 columns as the index; no top-N; `data-sort-default="desc"` on Exclusive Time)
- Source table columns: **Line**, **Statements**, **Time on line**, **Calls**, **Time in subs**, **Code**
- **Row union:** iterate the sorted union of `source_lines` **and** `line_totals` (and `block_line_totals` lines that belong to that fid). Missing source text is `—`.
- Each source `<tr>` has `id="L{line}"` so sub links (`file-{fid}.html#L{first_line}`) land on the line.
- Unused / zero-tick lines have **no** heat class on the `<tr>` or cells. Heat is on numeric time/count cells only when the value is &gt; 0.
- Calls / Time in subs come from **usable** `call_sites` only (`(fid,line)==(1,1)` is unusable unless that caller’s `sub_def` starts there). Otherwise **`—`** (never `0` / `0s`). Usable sites also emit `.calls` / `.calls_in` / `.calls_out` in the Code cell.
- Code cell is `<td class="s">` with escaped source.
- After source lines, opcode stub rows for `CORE:` names that have `call_edges` but no `sub_def`: `id="main__CORE_match"` (non-alnum → `_`).
- Source times are **HTML-only compact units** when `ticks_per_sec` is present (`title=` raw ticks); text/CSV/JSON stay integer ticks.
- Not oracle `c0`–`c3` class names.
- `table.source.sortable` with `th[data-sort]` and `data-sort-value` on numeric cells; vanilla `nytprof-sort.js` (see [html-sort-js-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-sort-js-mvp-v0.md)).
- Hot loop on workload page: `$x++` / `for 1 .. 50` and positive calls
- On blocks-calls1: A4 line 5 calls from model; **A4b section or column** showing block_line totals (see below)
- `source.html` is the **application** fid (attributes `application` basename, else hottest non-`@INC` `.pl`), not `warnings.pm`

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

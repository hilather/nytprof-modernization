# HTML out-dir path safety MVP (v0)

**Status:** implemented (MVP)  
**Board:** `HTML-OUTDIR-SAFETY`  
**Extends:** [html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md) (`html --out-dir DIR` / `write_html_site`)

**Library:** `nytprof_report::{validate_html_out_dir, write_html_site}`  
**CLI:** `nytprof-cli html <profile.out> --out-dir DIR` (goes through `write_html_site`)

## Goal

Fail closed on unsafe or ambiguous `out_dir` paths **before** any create/write of the multi-file HTML site. Complements atomic temp-then-rename publish ([ATOMIC-HTML-PUBLISH](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md)).

## Rules

`validate_html_out_dir(out_dir: &Path) -> io::Result<()>` is called at the start of `write_html_site` (and again in the publish helper for defense in depth).

| Rule | Behavior |
|------|----------|
| **Empty path** | `out_dir` with empty OS string (`""`) → `Err(InvalidInput)` |
| **Null byte** | Any `\0` in the path's OS representation → `Err(InvalidInput)` |
| **Path traversal intent** | Any path component equal to `..` (`Component::ParentDir`) → `Err(InvalidInput)` (relative or absolute) |
| **Absolute paths** | **Allowed** when they have no `..` component and no `\0` (CLI may pass absolute dirs) |
| **Existing non-directory** | Unchanged: if `out_dir` exists and is not a directory → `Err` without publishing (ATOMIC-HTML-PUBLISH) |

Not in scope for this MVP:

- Resolving symlinks / “must stay under a chroot”
- Banning absolute paths
- Canonicalizing away `..` then accepting (explicit `..` components are always rejected)

## Atomic publish (unchanged)

After validation succeeds:

1. Render site in memory.
2. Stage under a sibling `.nytprof-html-*` temp dir under `out_dir`'s parent.
3. Rename-publish into `out_dir` (bak-swap when the directory already exists).

See [html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md).

## Tests

| Test | Expectation |
|------|-------------|
| `write_html_site_rejects_dotdot_component` | PathBuf with a `..` component → `InvalidInput`; no site published |
| `write_html_site_rejects_null_byte` | (Unix) OsString with embedded `\0` → `InvalidInput` |
| `write_html_site_rejects_empty_path` | Empty path → `InvalidInput` |
| `write_html_site_atomic_default_calls1` | Safe path still writes index with leaf **15** / mid **3** / mid→leaf **15** |
| `write_html_site_atomic_outdir_is_file_err` / `write_html_site_atomic_parent_is_file_err` | Existing non-dir / unusable parent still fail closed |

## Non-goals

- Full SEC path sandboxing for untrusted multi-tenant hosts
- Windows-specific drive/UNC edge cases beyond “no empty / no ParentDir / best-effort NUL”

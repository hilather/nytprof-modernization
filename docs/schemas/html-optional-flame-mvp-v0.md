# HTML optional flame path (MVP v0)

**Status:** implemented (PR-A03 / HTML residual slice)  
**Board / residual:** closes native **optional flame** residual for MVP folded+SVG site path; does **not** claim oracle `flamegraph.pl` / multi-frame `nytprofcalls` / Graphviz / treemap  
**Complements:** [html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md), [html-report-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-report-mvp-v0.md), [export-formats-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/export-formats-mvp-v0.md)  
**Inventory:** [REPORT_HTML_RESIDUAL_INVENTORY_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md)

## Purpose

Oracle `nytprofhtml` generates flame artifacts by default (`--flame`, default on) via bundled `flamegraph.pl` + `nytprofcalls` when the profile has calls data. Native HTML previously had **no** site flame path (only a separate `folded` CLI export).

This slice adds an **opt-in** native flame path:

| Rule | Detail |
|------|--------|
| **Default off** | No flame files or sections unless the operator passes `--flame` — **no default bloat** (unlike oracle default-on) |
| **Folded-based** | Stacks come from `call_edges` via `render_folded_stacks` (two-frame `caller;called count`) — not full multi-frame call-stream reconstruction |
| **Native SVG** | Simple icicle/flame SVG from those folded stacks — **not** `flamegraph.pl` output or visual parity |
| **Still residual** | Graphviz `.dot`, treemap, Shared JS, oracle flame DOM embed polish |

## CLI

```text
# Default multi-file site (no flame artifacts):
nytprof-cli html <profile.out> --out-dir DIR

# Opt-in flame:
nytprof-cli html <profile.out> --out-dir DIR --flame
nytprof-cli html <profile.out> --flame              # single-file: embed SVG inline
nytprof-cli html <profile.out> -o report.html --flame
nytprof-cli html … --no-flame                       # explicit off (default)
```

`--flame` and `--no-flame` conflict if both are passed.

## Multi-file layout when `--flame`

```text
{out-dir}/
  index.html                     # summary; section.flame links
  source.html
  file-<fid>.html
  style.css
  all_stacks_by_time.svg         # native folded-based SVG (oracle-aligned basename)
  all_stacks_by_time.folded      # render_folded_stacks body (not oracle .calls dialect)
```

Default (no `--flame`) layout is unchanged — **no** `all_stacks_by_time.*` files.

## Artifact requirements

| Artifact | Requirement |
|----------|-------------|
| `all_stacks_by_time.folded` | Same text as `render_folded_stacks`; default-calls1 includes `main::mid;main::leaf 15` |
| `all_stacks_by_time.svg` | Well-formed SVG; labels include `main::leaf` / `main::mid`; count **15** visible for mid→leaf (title/tooltip or text) |
| Index `section.flame` | `p.flame-links` with relative `href` to both files; optional `<object>` preview of SVG |
| Atomic publish | Flame files written in the same temp-then-rename path as `style.css` |
| stderr listing | CLI lists flame paths with other published files when flame is on |

## Single-file when `--flame`

- One self-contained HTML document
- `section.flame` with **inlined** SVG (no separate files)
- Default single-file path has **no** flame section

## Library API

```rust
pub struct HtmlRenderOptions {
    pub flame: bool,  // default false
}

pub const FLAME_FOLDED_FILENAME: &str; // "all_stacks_by_time.folded"
pub const FLAME_SVG_FILENAME: &str;    // "all_stacks_by_time.svg"

pub fn render_flame_svg(model: &ProfileModel) -> String;
pub fn render_html_summary_with_options(model, path, HtmlRenderOptions) -> String;
pub fn render_html_site_with_options(model, path, HtmlRenderOptions) -> HtmlSite;
pub fn write_html_site_with_options(model, path, out_dir, HtmlRenderOptions) -> io::Result<HtmlSite>;

// Defaults (flame off) remain:
pub fn render_html_summary(...) -> String;
pub fn render_html_site(...) -> HtmlSite;
pub fn write_html_site(...) -> io::Result<HtmlSite>;
```

`HtmlSite` optional fields when flame on: `flame_folded` / `flame_folded_filename` / `flame_svg` / `flame_svg_filename`; all `None` when flame off.

## Semantic counts (default-calls1)

Flame does **not** replace advertised HTML counts. With or without `--flame`:

| Check | Expected |
|-------|----------|
| `main::leaf` returns | **15** |
| `main::mid` returns | **3** |
| `main::mid` → `main::leaf` | **15** (folded line + greppable HTML tables) |

## Explicit non-requirements (still residual)

- Oracle `flamegraph.pl` / `nytprofcalls` multi-frame stacks / `flamegraph_subattr.txt`
- Oracle default-on flame behavior
- Graphviz packages/subs/per-file `.dot`
- Treemap / JIT assets
- Visual or byte identity with oracle SVG
- Full plan REPORT-008 integration suite

## Tests

| Test | Asserts |
|------|---------|
| `flame_svg_default_calls1_real_render` | Real model → SVG has leaf/mid and count 15 |
| `html_site_default_no_flame_artifacts` | Default site/disk has no flame files or index links |
| `html_site_optional_flame_default_calls1` | Opt-in render + disk publish; folded mid→leaf 15; index links; CSS still present |
| `html_summary_optional_flame_embed_default_calls1` | Single-file default vs embed |
| CLI `html_out_dir_default_has_no_flame_files` | Real binary: default `--out-dir` no flame |
| CLI `html_out_dir_flame_writes_svg_and_folded_and_lists_on_stderr` | Real binary: `--flame` writes + stderr + **15/3** |
| CLI `html_single_file_flame_embeds_svg_on_stdout` | Real binary: single-file embed |

## Residual honesty

Closing this slice flips inventory **Flame graph SVG** and **Call-stack flame inputs** to native MVP **partial** (opt-in path **advertised**; not oracle default-on, not `flamegraph.pl`, not multi-frame call stream). Graphviz and treemap remain residual **yes**. See residual inventory + [REPORT_SURFACE_CONTRACT_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_SURFACE_CONTRACT_v0.md).

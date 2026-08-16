# HTML optional flame path (MVP v0)

**Status:** implemented (PR-A03 / HTML residual slice)  
**Board / residual:** closes native **optional flame** residual for MVP folded+SVG site path; does **not** claim oracle `flamegraph.pl` / multi-frame `nytprofcalls` / Graphviz / treemap  
**Complements:** [html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md), [html-report-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-report-mvp-v0.md), [export-formats-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/export-formats-mvp-v0.md)  
**Inventory:** [REPORT_HTML_RESIDUAL_INVENTORY_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md)

## Purpose

Oracle `nytprofhtml` generates flame artifacts by default (`--flame`, default on) via bundled `flamegraph.pl` + `nytprofcalls` when the profile has calls data. Native HTML previously had **no** site flame path (only a separate `folded` CLI export).

This slice adds a native flame path. **Amended 2026-08-15:** the CLI now matches the oracle default — flame is **on** unless `--no-flame` (the original MVP shipped opt-in `--flame`, default off; KD-FLAME's "do not match 6.15 default-on" is superseded by user direction for oracle parity). The library default is unchanged (`HtmlRenderOptions::default()` keeps `flame: false` for embedders).

| Rule | Detail |
|------|--------|
| **Default on (CLI)** | `nytprof-cli html` writes flame files + section by default (oracle `flame!` = 1 parity); `--no-flame` opts out. Library `HtmlRenderOptions::default()` stays `flame: false` |
| **Skipped without calls** | Profiles with no non-zero `call_edges` get **no** flame files or section even when flame is on (oracle skips flame when the profile has no calls data) — never ship an empty SVG |
| **Folded-based** | Stacks come from `call_edges` via `render_folded_stacks` (two-frame `caller;called count`) — not full multi-frame call-stream reconstruction |
| **Native SVG** | Call-tree flame from `call_edges`: same caller is **one** frame; children stack **up**; width is **inclusive time** when `sub_return_totals` has it, else edge incl, else call count. **Not** a two-level equal-width column per edge, and **not** `flamegraph.pl` visual parity. |
| **Bounded paint** | Frames narrower than 1 CSS px in the 1200-wide viewBox are **omitted** (no one-rect-per-tiny-edge flood). Labels only when width ≥ 48 px. Depth capped at 16. |
| **No extra profile walk** | Multi-file flame collects `call_edges` **once** for folded + SVG. Index **inlines** that SVG (hover details + click-to-source) and still links the sibling files. **Not** `<img>` / `<object>` (those cannot host SVG links or titles under `file://`). |
| **Still residual** | `flamegraph.pl` / `nytprofcalls` multi-frame stacks, treemap, Shared JS |

## CLI

```text
# Default multi-file site (flame artifacts included — oracle parity):
nytprof-cli html <profile.out> --out-dir DIR

# Opt out:
nytprof-cli html <profile.out> --out-dir DIR --no-flame
nytprof-cli html <profile.out> --no-flame           # single-file: no flame section
nytprof-cli html <profile.out> -o report.html --no-flame
nytprof-cli html … --flame                          # explicit on (already default)
```

`--flame` and `--no-flame` conflict if both are passed.

## Multi-file layout (flame on, the CLI default)

```text
{out-dir}/
  index.html                     # summary; section.flame links
  source.html
  file-<fid>.html
  style.css
  all_stacks_by_time.svg         # native folded-based SVG (oracle-aligned basename)
  all_stacks_by_time.folded      # render_folded_stacks body (not oracle .calls dialect)
```

With `--no-flame` the layout is unchanged from pre-flame releases — **no** `all_stacks_by_time.*` files.

## Artifact requirements

| Artifact | Requirement |
|----------|-------------|
| `all_stacks_by_time.folded` | Same text as `render_folded_stacks`; default-calls1 includes `main::mid;main::leaf 15` |
| `all_stacks_by_time.svg` | Well-formed SVG; **call-tree** stacked `rect`s (roots at the bottom); same caller is **one** frame (not a column per outgoing edge); widths proportional to inclusive time when known, else count (not all equal when weights differ); labels include `main::leaf` / `main::mid`; count **15** visible for mid→leaf (`calls: 15` in `<title>`). Frames with a `sub_def` (or `CORE:` / `RUNTIME`) wrap in `<a class="flame-link" href="file-{fid}.html#L{line}">`. `<title>` lists name, calls, inclusive, exclusive. Sub-pixel frames omitted. |
| Index `section.flame` | `p.flame-links` with relative `href` to both files; **inlined** sibling SVG (same body) so hover + click work under `file://`; `#nytprof-flame-tip` vanilla tooltip; **not** `<img>` / `<object>` |

**Presentation (2026-08-15 refresh, not contract):** frames use rounded corners (`rx="3"`), a translucent white separator stroke, `pointer-events="none"` labels (hover/click always lands on the frame), and a CSS `transition` on hover. These attributes are visual polish only — tests must keep asserting structure (`<rect `, names, counts, `flame-link`, `<title>` fields), never stroke/radius values.
| Atomic publish | Flame files written in the same temp-then-rename path as `style.css` |
| stderr listing | CLI lists flame paths with other published files when flame is on |

## Single-file (flame on, the CLI default)

- One self-contained HTML document
- `section.flame` with **inlined** SVG (no separate files)
- `--no-flame` single-file path has **no** flame section

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

// Library defaults (flame off) remain — only the CLI flips to default-on:
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
- Graphviz packages/subs/per-file `.dot`
- Treemap / JIT assets
- Visual or byte identity with oracle SVG
- Full plan REPORT-008 integration suite

## Tests

| Test | Asserts |
|------|---------|
| `flame_svg_default_calls1_real_render` | Real model → SVG has leaf/mid, count 15, stacked `rect`s, **unequal** widths |
| `flame_svg_stacks_callers_once_not_per_edge_columns` | Equal-count scanner-shaped graph: each name once; root wider than children (not a barcode of identical columns) |
| `flame_svg_omits_subpixel_frames` | 1-count edge vs 1e6-count edge: tiny name not painted |
| `html_site_default_no_flame_artifacts` | Library default site/disk has no flame files or index links (CLI flips default on) |
| `flame_skipped_when_no_call_edges` | Flame requested but zero `call_edges`: no flame files, section, or links (oracle no-calls parity) |
| `html_site_optional_flame_default_calls1` | Opt-in render + disk publish; folded mid→leaf 15; index inlines SVG + hover tip; leaf `flame-link` to `sub_def` line |
| `html_summary_optional_flame_embed_default_calls1` | Single-file default vs embed |
| CLI `html_out_dir_default_writes_flame_files` | Real binary: default `--out-dir` writes flame (2026-08-15 default-on amendment) |
| CLI `html_out_dir_no_flame_writes_no_flame_files` | Real binary: `--no-flame` writes no flame files or links |
| CLI `html_out_dir_flame_writes_svg_and_folded_and_lists_on_stderr` | Real binary: explicit `--flame` writes + stderr + **15/3** |
| CLI `html_single_file_flame_embeds_svg_on_stdout` | Real binary: single-file embed |

The shared-CSS CLI suite (`html_shared_css.rs`) runs with `--no-flame`: its "no inline `<style>`" rule contracts the **shared page stylesheet** only; the flame SVG's SVG-scoped `<style>` is covered here, not there.

## Residual honesty

Closing this slice flips inventory **Flame graph SVG** and **Call-stack flame inputs** to native MVP **partial** (path **advertised**; CLI default-on parity since 2026-08-15; not `flamegraph.pl`, not multi-frame call stream). Graphviz and treemap remain residual **yes**. See residual inventory + [REPORT_SURFACE_CONTRACT_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_SURFACE_CONTRACT_v0.md).

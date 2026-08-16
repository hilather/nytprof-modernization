//! Native report rendering (text summary + CSV tabular + HTML MVP + exports).
//!
//! Multi-file HTML is **operator HTML v2** (ADR-0012): chrome, index IA,
//! six-column source, compact `fmt_time` units, vanilla sort. See
//! `docs/schemas/html-operator-v2-mvp-v0.md`. jquery / tablesorter stay WAIVE.
//!
//! Content requirements: `docs/schemas/aggregate-comparison-v0.md`,
//! `docs/schemas/html-report-mvp-v0.md`,
//! `docs/schemas/html-multifile-mvp-v0.md`,
//! `docs/schemas/html-per-file-mvp-v0.md` (A4b block_line_totals),
//! `docs/schemas/html-outdir-safety-mvp-v0.md`,
//! `docs/schemas/html-shared-css-structure-mvp-v0.md` (shared CSS + structure),
//! `docs/schemas/html-sort-js-mvp-v0.md` (vanilla sort JS; not jquery/tablesorter),
//! `docs/schemas/html-operator-v2-mvp-v0.md` (chrome / IA / compact time),
//! `docs/schemas/export-formats-mvp-v0.md`,
//! `docs/schemas/export-semantic-parity-mvp-v0.md`,
//! `docs/schemas/verify-cli-mvp-v0.md`,
//! `docs/schemas/report-semantic-parity-mvp-v0.md`,
//! `docs/schemas/blocks-semantic-parity-mvp-v0.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nytprof_model::ProfileModel;

/// Env opt-in salvage for incomplete (but record-aligned) streams.
///
/// When `NYTPROF_ALLOW_INCOMPLETE=1`, [`verify_profile`] and report-path
/// completeness checks accept incomplete streams instead of returning `Err`.
/// See `docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md`.
pub fn allow_incomplete_stream() -> bool {
    match std::env::var("NYTPROF_ALLOW_INCOMPLETE") {
        Ok(v) => v == "1",
        Err(_) => false,
    }
}

/// Fail closed unless the model stream is complete or salvage env is set.
///
/// Used by report / csv / html / export paths after a successful decode+model.
/// Dump remains lenient and does not call this.
pub fn require_complete_stream(model: &ProfileModel) -> Result<(), Box<dyn Error>> {
    let reasons = model.stream_incompleteness_reasons();
    if reasons.is_empty() || allow_incomplete_stream() {
        return Ok(());
    }
    Err(format!(
        "incomplete profile stream: {} (set NYTPROF_ALLOW_INCOMPLETE=1 to salvage)",
        reasons.join("; ")
    )
    .into())
}

/// Decode a v5 profile, build the compact model, and return a short verify summary.
///
/// On success the returned string contains `OK` (or `INCOMPLETE` under salvage)
/// and the path, plus event counters.
/// On failure (empty/truncated/corrupt/unsupported/incomplete-by-default),
/// returns an error.
///
/// **Completeness (default fail-closed):** after a successful decode/model,
/// streams that are incomplete per
/// [`ProfileModel::stream_incompleteness_reasons`] fail with `Err` unless
/// `NYTPROF_ALLOW_INCOMPLETE=1` is set (then success with an `INCOMPLETE` note).
///
/// See `docs/schemas/verify-cli-mvp-v0.md` and
/// `docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md`.
pub fn verify_profile(path: &Path) -> Result<String, Box<dyn Error>> {
    let model = ProfileModel::from_path(path)?;
    let path_display = path.display();
    let reasons = model.stream_incompleteness_reasons();

    if !reasons.is_empty() && !allow_incomplete_stream() {
        return Err(format!(
            "incomplete profile stream: {} ({path_display}; set NYTPROF_ALLOW_INCOMPLETE=1 to salvage)",
            reasons.join("; ")
        )
        .into());
    }

    let header = if reasons.is_empty() {
        format!("OK: {path_display}")
    } else {
        format!(
            "INCOMPLETE: {path_display}\n  \
             note: {}; allowed by NYTPROF_ALLOW_INCOMPLETE=1",
            reasons.join("; ")
        )
    };

    Ok(format!(
        "{header}\n  \
         events: {}\n  \
         TIME_LINE: {}\n  \
         TIME_BLOCK: {}\n  \
         files: {}\n  \
         subs: {}\n",
        model.total_events,
        model.time_line_events,
        model.time_block_events,
        model.files.len(),
        model.sub_return_totals.len(),
    ))
}

/// Render a user-visible text summary from a compact [`ProfileModel`].
///
/// Includes event counts, subroutine totals from `SUB_RETURN`, and top lines by calls.
pub fn render_summary_text(model: &ProfileModel, profile_path: &str) -> String {
    let mut out = String::with_capacity(4096);

    out.push_str("NYTProf summary report\n");
    out.push_str(&format!("profile: {profile_path}\n"));
    out.push_str(&format!("time_line_events: {}\n", model.time_line_events));
    out.push_str(&format!("time_block_events: {}\n", model.time_block_events));
    out.push_str(&format!("discount_events: {}\n", model.discount_events));
    out.push('\n');

    out.push_str("Subroutines (from SUB_RETURN)\n");
    if model.sub_return_totals.is_empty() {
        out.push_str("  (none)\n");
    } else {
        // Deterministic order by subname (model stores a HashMap).
        let mut subs: Vec<_> = model.sub_return_totals.iter().collect();
        subs.sort_by_key(|(a, _)| *a);
        for (name, t) in subs {
            out.push_str(&format!(
                "  {name}  returns={}  excl={}  incl={}\n",
                t.returns,
                format_ticks(t.excl), // exclusive ticks (A5)
                format_ticks(t.incl), // inclusive ticks (A5)
            ));
        }
    }
    out.push('\n');

    out.push_str("Top lines (by calls)\n");
    out.push_str("  fid  line  calls  ticks  file\n");

    let mut lines: Vec<_> = model.line_totals.iter().collect();
    lines.sort_by(|((f1, l1), a), ((f2, l2), b)| {
        b.calls
            .cmp(&a.calls)
            .then_with(|| f1.cmp(f2))
            .then_with(|| l1.cmp(l2))
    });

    for ((fid, line), t) in lines.into_iter().take(10) {
        let file = model.fid_basename(*fid).unwrap_or("?");
        out.push_str(&format!(
            "  {fid}  {line}  {}  {}  {file}\n",
            t.calls, t.ticks
        ));
    }

    out
}

/// Subroutine totals CSV (schema A5).
///
/// Header: `name,returns,incl,excl`  
/// Rows: all `sub_return_totals`, sorted by name. Names are RFC 4180-escaped
/// when they contain comma, quote, or newline.
pub fn render_subs_csv(model: &ProfileModel) -> String {
    let mut out = String::with_capacity(1024);
    out.push_str("name,returns,incl,excl\n");
    let mut rows: Vec<_> = model.sub_return_totals.iter().collect();
    rows.sort_by_key(|(a, _)| *a);
    for (name, t) in rows {
        out.push_str(&csv_escape(name));
        out.push(',');
        out.push_str(&t.returns.to_string());
        out.push(',');
        out.push_str(&format_ticks(t.incl));
        out.push(',');
        out.push_str(&format_ticks(t.excl));
        out.push('\n');
    }
    out
}

/// Call-edge totals CSV (schema A7).
///
/// Header: `caller,called,count,incl,excl`  
/// Rows from `call_edges`, sorted by `(caller, called)`. Name fields are
/// CSV-escaped when needed.
pub fn render_edges_csv(model: &ProfileModel) -> String {
    let mut out = String::with_capacity(1024);
    out.push_str("caller,called,count,incl,excl\n");
    let mut rows: Vec<_> = model.call_edges.iter().collect();
    rows.sort_by(|((c1, d1), _), ((c2, d2), _)| c1.cmp(c2).then_with(|| d1.cmp(d2)));
    for ((caller, called), e) in rows {
        out.push_str(&csv_escape(caller));
        out.push(',');
        out.push_str(&csv_escape(called));
        out.push(',');
        out.push_str(&e.count.to_string());
        out.push(',');
        out.push_str(&format_ticks(e.incl));
        out.push(',');
        out.push_str(&format_ticks(e.excl));
        out.push('\n');
    }
    out
}

/// Combined dual-section CSV used by `nytprof-cli csv` (default stdout form).
///
/// ```text
/// # subroutines
/// name,returns,incl,excl
/// ...
/// # call_edges
/// caller,called,count,incl,excl
/// ...
/// ```
pub fn render_csv_report(model: &ProfileModel) -> String {
    let mut out = String::with_capacity(2048);
    out.push_str("# subroutines\n");
    out.push_str(&render_subs_csv(model));
    out.push_str("# call_edges\n");
    out.push_str(&render_edges_csv(model));
    out
}

/// Escape text for inclusion in HTML text nodes / attributes.
///
/// Escapes `&`, `<`, `>`, `"` (and `'` as `&#39;` for attribute safety).
pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Options for native HTML rendering (single-file and multi-file).
///
/// Defaults keep sites lean: optional artifacts (flame) are **off** unless
/// requested. See `docs/schemas/html-optional-flame-mvp-v0.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HtmlRenderOptions {
    /// Opt-in flame path: publish folded stacks + native SVG (and index links /
    /// single-file embed). **Default `false`** — no default bloat.
    pub flame: bool,
}

/// Multi-file HTML report site (index + per-fid source pages + exclusive sub
/// index + shared CSS + optional flame).
///
/// See `docs/schemas/html-multifile-mvp-v0.md`,
/// `docs/schemas/html-per-file-mvp-v0.md`,
/// `docs/schemas/html-shared-css-structure-mvp-v0.md`,
/// `docs/schemas/html-subs-excl-index-mvp-v0.md`, and
/// `docs/schemas/html-optional-flame-mvp-v0.md` (optional flame fields).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlSite {
    /// Contents of `index.html` (summary with relative links to file pages).
    pub index_html: String,
    /// Contents of the primary workload source page (also written as `source.html`).
    pub source_html: String,
    /// Relative filename of the legacy primary alias (`"source.html"`).
    pub source_filename: String,
    /// Per-fid pages: `(filename, html)` e.g. `("file-1.html", "...")`, sorted by fid.
    pub file_pages: Vec<(String, String)>,
    /// Full exclusive-time sub ranking page body
    /// ([`HtmlSite::index_subs_excl_filename`] / `index-subs-excl.html`).
    pub index_subs_excl_html: String,
    /// Relative filename of the exclusive sub index (always
    /// [`INDEX_SUBS_EXCL_FILENAME`] / `"index-subs-excl.html"` from
    /// [`render_html_site`]).
    pub index_subs_excl_filename: String,
    /// Shared stylesheet body written as [`HtmlSite::style_filename`] (`style.css`).
    ///
    /// Same text as [`SHARED_STYLE_CSS`]. Multi-file pages link to
    /// [`HtmlSite::style_filename`] via `<link rel="stylesheet" href="…">`
    /// (no inline `<style>`).
    pub style_css: String,
    /// Relative filename of the shared stylesheet (must match page `<link href>`;
    /// always [`STYLE_CSS_FILENAME`] / `"style.css"` from [`render_html_site`]).
    pub style_filename: String,
    /// Vanilla sort script body written as [`HtmlSite::sort_js_filename`].
    ///
    /// Same text as [`SHARED_SORT_JS`]. Multi-file pages link it via
    /// `<script src="nytprof-sort.js" defer>`; single-file summaries inline
    /// the same source. **Not** jquery / tablesorter.
    pub sort_js: String,
    /// Relative filename of the sort script (must match page `<script src>`;
    /// always [`SORT_JS_FILENAME`] / `"nytprof-sort.js"` from [`render_html_site`]).
    pub sort_js_filename: String,
    /// Optional folded-stack body for flame tools (`None` when flame is off).
    ///
    /// Same text as [`render_folded_stacks`]. Published as
    /// [`HtmlSite::flame_folded_filename`] when present.
    pub flame_folded: Option<String>,
    /// Relative folded filename (e.g. [`FLAME_FOLDED_FILENAME`]) when flame is on.
    pub flame_folded_filename: Option<String>,
    /// Optional native flame SVG body (`None` when flame is off).
    ///
    /// From [`render_flame_svg`] (call-tree flame by inclusive time; **not** oracle
    /// `flamegraph.pl`). Published as [`HtmlSite::flame_svg_filename`].
    pub flame_svg: Option<String>,
    /// Relative SVG filename (e.g. [`FLAME_SVG_FILENAME`]) when flame is on.
    pub flame_svg_filename: Option<String>,
    /// Graphviz inter-package call graph (always published; not `dot` PNG).
    pub packages_callgraph_dot: String,
    /// Relative filename ([`PACKAGES_CALLGRAPH_FILENAME`]).
    pub packages_callgraph_filename: String,
    /// Graphviz inter-subroutine call graph (always published; not `dot` PNG).
    pub subs_callgraph_dot: String,
    /// Relative filename ([`SUBS_CALLGRAPH_FILENAME`]).
    pub subs_callgraph_filename: String,
}

/// How a document loads the shared MVP stylesheet.
///
/// See `docs/schemas/html-shared-css-structure-mvp-v0.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HtmlCssMode<'a> {
    /// Single-file / stdout summary: embed [`SHARED_STYLE_CSS`] in a `<style>` tag
    /// so the document is self-contained (no external asset).
    Inline,
    /// Multi-file site: link to sibling stylesheet written by [`write_html_site`].
    ///
    /// The filename must match [`HtmlSite::style_filename`] / the file published
    /// as `style.css` (normally [`STYLE_CSS_FILENAME`]).
    LinkedStyleSheet(&'a str),
}

/// How a document loads the vanilla sort script ([`SHARED_SORT_JS`]).
///
/// See `docs/schemas/html-sort-js-mvp-v0.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HtmlJsMode<'a> {
    /// Single-file / stdout summary: embed [`SHARED_SORT_JS`] in a `<script>`
    /// tag so the document is self-contained.
    Inline,
    /// Multi-file site: sibling [`SORT_JS_FILENAME`] with `defer`.
    LinkedScript(&'a str),
}

/// Shared operator HTML v2 stylesheet (ADR-0012).
///
/// **Policy:** multi-file sites write this as `style.css` and link to it;
/// single-file `render_html_summary` embeds the **same** text inline. This is
/// **not** oracle `get_css()` / tablesorter CSS parity — residual honesty for
/// full DOM/JS remains in
/// [`REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md).
/// Heat class names stay `heat-*`; CSS variables `--nyt-c0`…`--nyt-c3` hold
/// the oracle palette. Do **not** emit `.c0`–`.c3` class selectors.
/// A `prefers-color-scheme: dark` block re-tunes the `--nyt-*` variables
/// (including the heat hues) for dark displays; selectors are unchanged.
pub const SHARED_STYLE_CSS: &str = r#"/* nytprof-report operator HTML v2 CSS (ADR-0012; not oracle get_css) */
:root{
color-scheme:light;
--nyt-font:system-ui,-apple-system,"Segoe UI",Roboto,"Helvetica Neue",Arial,sans-serif;
--nyt-mono:ui-monospace,"SF Mono",Menlo,Consolas,"Liberation Mono",monospace;
--nyt-fg:#1f2328;
--nyt-bg:#f6f8fa;
--nyt-surface:#fff;
--nyt-link:#0563c1;
--nyt-link-visited:#6d00e6;
--nyt-link-hover:#c00;
--nyt-header-top:rgb(17,136,255);
--nyt-header-bot:rgb(0,68,187);
--nyt-header-fg:#fff;
--nyt-th:#eef1f4;
--nyt-th-fg:#57606a;
--nyt-th-border:#d0d7de;
--nyt-td-border:#d8dee4;
--nyt-caption:#eef1f4;
--nyt-footer:#d0d7de;
--nyt-calls:#57606a;
--nyt-row-hover:#eaf2fe;
--nyt-target:#fff3bf;
--nyt-radius:10px;
--nyt-shadow:0 1px 2px rgba(31,35,40,.07),0 1px 3px rgba(31,35,40,.05);
--nyt-c0:#ffb3b3;
--nyt-c1:#ffd9b4;
--nyt-c2:#ffffb4;
--nyt-c3:#b4ffb4;
}
body{font-family:var(--nyt-font);margin:0;line-height:1.45;color:var(--nyt-fg);background:var(--nyt-bg);-webkit-font-smoothing:antialiased}
a{color:var(--nyt-link);text-decoration:none}
a:visited{color:var(--nyt-link-visited)}
a:hover{color:var(--nyt-link-hover);text-decoration:underline}
a:focus-visible{outline:2px solid var(--nyt-header-top);outline-offset:2px;border-radius:3px}
.header{background:linear-gradient(135deg,var(--nyt-header-top),var(--nyt-header-bot));color:var(--nyt-header-fg);padding:1.15rem 1.5rem 1.35rem;box-shadow:0 2px 12px rgba(0,32,96,.3)}
.header_back{margin-bottom:.45rem}
.header_back a{color:#fff;border:1px solid rgba(255,255,255,.55);border-radius:999px;padding:.16rem .75rem;font-size:.8rem;letter-spacing:.03em;transition:background-color .15s ease,border-color .15s ease}
.header_back a:hover{background:rgba(255,255,255,.16);border-color:#fff;color:#fff;text-decoration:none}
.siteTitle{font-size:1.7rem;font-weight:700;letter-spacing:-.015em;margin:.1rem 0;text-shadow:0 1px 2px rgba(0,24,80,.25)}
.siteSubtitle{font-size:.95rem;opacity:.92}
.body_content{padding:1.1rem 1.5rem 2.5rem;max-width:80rem}
.footer{color:var(--nyt-calls);font-size:.82rem;padding:1rem 1.5rem;border-top:1px solid var(--nyt-footer)}
table{border-collapse:separate;border-spacing:0;margin:.4rem 0 1.1rem;width:auto;max-width:100%;background:var(--nyt-surface);border:1px solid var(--nyt-td-border);border-radius:var(--nyt-radius);box-shadow:var(--nyt-shadow)}
caption{background:transparent;text-align:left;padding:.3em .1em;font-weight:600;font-size:.85rem;color:var(--nyt-calls);letter-spacing:.01em}
th,td{border:0;border-bottom:1px solid var(--nyt-td-border);padding:.45em .75em;text-align:left;vertical-align:top}
th{background:var(--nyt-th);font-size:.8em;font-weight:600}
thead th{position:sticky;top:0;z-index:1;color:var(--nyt-th-fg);font-size:.72rem;text-transform:uppercase;letter-spacing:.05em;border-bottom:2px solid var(--nyt-th-border);white-space:nowrap}
table>thead:first-child th:first-child,table>caption:first-child+thead th:first-child{border-top-left-radius:var(--nyt-radius)}
table>thead:first-child th:last-child,table>caption:first-child+thead th:last-child{border-top-right-radius:var(--nyt-radius)}
table>tbody:first-child tr:first-child th:first-child,table>tbody:first-child tr:first-child td:first-child{border-top-left-radius:var(--nyt-radius)}
table>tbody:first-child tr:first-child th:last-child,table>tbody:first-child tr:first-child td:last-child{border-top-right-radius:var(--nyt-radius)}
table>tbody:last-child tr:last-child th:first-child,table>tbody:last-child tr:last-child td:first-child{border-bottom-left-radius:var(--nyt-radius)}
table>tbody:last-child tr:last-child th:last-child,table>tbody:last-child tr:last-child td:last-child{border-bottom-right-radius:var(--nyt-radius)}
table>tfoot:last-child tr:last-child td:first-child{border-bottom-left-radius:var(--nyt-radius)}
table>tfoot:last-child tr:last-child td:last-child{border-bottom-right-radius:var(--nyt-radius)}
tbody tr:last-child th,tbody tr:last-child td,tfoot tr:last-child td{border-bottom:0}
td.num{text-align:right;font-variant-numeric:tabular-nums;white-space:nowrap}
td.s,.s,pre,code,.sub_name{font-family:var(--nyt-mono)}
td.s,.src-line{white-space:pre}
td.s{font-size:.88rem;line-height:1.5}
code{background:var(--nyt-caption);border-radius:4px;padding:.05em .35em;font-size:.92em}
table.source thead th:first-child,table.source tbody td:first-child{background:var(--nyt-caption);color:var(--nyt-calls)}
.index_summary{margin:0 0 1.25rem;padding:.8rem 1.1rem;background:var(--nyt-surface);border:1px solid var(--nyt-td-border);border-left:4px solid var(--nyt-header-top);border-radius:8px;box-shadow:var(--nyt-shadow);font-size:1.02rem}
.table_footer{margin:0.9rem 0;font-size:.95rem}
tbody tr{transition:background-color .12s ease-in-out}
tbody tr:hover{background:var(--nyt-row-hover)}
tr:target{background:var(--nyt-target)}
tfoot td{font-weight:600;background:var(--nyt-caption)}
td.num.heat-hot,tr.heat-hot{background:var(--nyt-c0)}
td.num.heat-high,tr.heat-high{background:var(--nyt-c1)}
td.num.heat-mid,tr.heat-mid{background:var(--nyt-c2)}
td.num.heat-low,tr.heat-low{background:var(--nyt-c3)}
th[data-sort]{cursor:pointer;user-select:none}
th[data-sort]::after{content:" \21c5";font-size:.85em;font-weight:400;opacity:.4}
th[data-sort]:hover{background:var(--nyt-row-hover)}
th.sort-asc::after{content:" \25b2";font-size:0.7em;opacity:1}
th.sort-desc::after{content:" \25bc";font-size:0.7em;opacity:1}
section.flame,.flamegraph{margin:1.25rem 0}
p.flame-links{font-size:0.95rem}
.flame-svg-embed{max-width:100%;overflow:hidden auto;border:1px solid var(--nyt-td-border);border-radius:var(--nyt-radius);padding:0.5rem;background:var(--nyt-surface);box-shadow:var(--nyt-shadow);position:relative}
.flame-svg-embed svg,.flame-svg-embed img,img.flame-svg-embed{display:block;max-width:100%;width:100%;height:auto}
.flame-svg-embed a.flame-link{cursor:pointer}
#nytprof-flame-tip{position:fixed;z-index:40;display:none;background:#1b1b1b;color:#f4f4f4;font:12px var(--nyt-mono,ui-monospace,monospace);padding:0.4rem 0.55rem;border-radius:6px;white-space:pre;pointer-events:none;box-shadow:0 2px 8px rgba(0,0,0,.28);max-width:28rem}
p.profile-path code{word-break:break-all}
p.source-link{margin:0.5rem 0}
h1,h2{margin-top:1.6rem;font-weight:600;letter-spacing:-.01em}
h2{font-size:1.18rem;border-bottom:1px solid var(--nyt-td-border);padding-bottom:.3rem}
.calls,.calls_in,.calls_out{color:var(--nyt-calls);font-size:0.85em;margin:0.15em 0}
p.callgraph-links{margin:0.75rem 0}
@media (prefers-color-scheme:dark){
:root{
color-scheme:dark;
--nyt-fg:#dbe1e8;
--nyt-bg:#101318;
--nyt-surface:#1a1f26;
--nyt-link:#7cb3ff;
--nyt-link-visited:#c4a5ff;
--nyt-link-hover:#a8ccff;
--nyt-th:#242b34;
--nyt-th-fg:#9aa7b4;
--nyt-th-border:#38414c;
--nyt-td-border:#2c343e;
--nyt-caption:#21272f;
--nyt-footer:#2c343e;
--nyt-calls:#9aa7b4;
--nyt-row-hover:#1c2530;
--nyt-target:#3a3413;
--nyt-shadow:0 1px 2px rgba(0,0,0,.4),0 1px 3px rgba(0,0,0,.3);
--nyt-c0:#6e2b2b;
--nyt-c1:#6e5224;
--nyt-c2:#5f5f1e;
--nyt-c3:#2d5a2d;
}
}
"#;

/// Canonical multi-file stylesheet filename (`style.css`).
pub const STYLE_CSS_FILENAME: &str = "style.css";

/// Canonical multi-file vanilla sort script filename (`nytprof-sort.js`).
///
/// **Not** jquery / tablesorter. See `docs/schemas/html-sort-js-mvp-v0.md`.
pub const SORT_JS_FILENAME: &str = "nytprof-sort.js";

/// Vanilla table-sort script (reorder existing `tbody` rows only).
///
/// Multi-file sites write this as [`SORT_JS_FILENAME`]; single-file summaries
/// inline the same text. Never assigns `innerHTML` from profile data.
pub const SHARED_SORT_JS: &str = r#"/* nytprof-report vanilla column sort */
(function () {
  "use strict";
  function cellSortValue(cell) {
    if (!cell) return "";
    var raw = cell.getAttribute("data-sort-value");
    if (raw !== null && raw !== "") {
      var n = Number(raw);
      if (!isNaN(n)) return n;
      return raw;
    }
    return (cell.textContent || "").replace(/^\s+|\s+$/g, "");
  }
  function compareValues(a, b) {
    var an = typeof a === "number";
    var bn = typeof b === "number";
    if (an && bn) return a - b;
    if (an) return -1;
    if (bn) return 1;
    if (a < b) return -1;
    if (a > b) return 1;
    return 0;
  }
  function sortTable(table, col, th, forceDir) {
    var tbody = table.tBodies && table.tBodies[0];
    if (!tbody) return;
    var rows = [];
    var i;
    for (i = 0; i < tbody.rows.length; i++) {
      var tr = tbody.rows[i];
      if (typeof tr._nytprofOrig !== "number") tr._nytprofOrig = i;
      rows.push(tr);
    }
    var headers = table.querySelectorAll("thead th");
    var hasSort = th.classList.contains("sort-asc") || th.classList.contains("sort-desc");
    var desc;
    if (forceDir === "desc") {
      desc = true;
    } else if (forceDir === "asc") {
      desc = false;
    } else if (!hasSort) {
      desc = th.getAttribute("data-sort") === "num";
    } else {
      desc = th.classList.contains("sort-asc");
    }
    for (i = 0; i < headers.length; i++) {
      headers[i].classList.remove("sort-asc", "sort-desc");
      headers[i].setAttribute("aria-sort", "none");
    }
    th.classList.add(desc ? "sort-desc" : "sort-asc");
    th.setAttribute("aria-sort", desc ? "descending" : "ascending");
    rows.sort(function (ra, rb) {
      var cmp = compareValues(
        cellSortValue(ra.cells[col]),
        cellSortValue(rb.cells[col])
      );
      if (cmp === 0) return ra._nytprofOrig - rb._nytprofOrig;
      return desc ? -cmp : cmp;
    });
    for (i = 0; i < rows.length; i++) {
      tbody.appendChild(rows[i]);
    }
  }
  function bindTable(table) {
    var ths = table.querySelectorAll("thead th[data-sort]");
    if (!ths.length) return;
    if ((" " + table.className + " ").indexOf(" sortable ") === -1) {
      table.className = (table.className ? table.className + " " : "") + "sortable";
    }
    var c;
    for (c = 0; c < ths.length; c++) {
      (function (th, col) {
        th.addEventListener("click", function () {
          sortTable(table, col, th);
        });
      })(ths[c], ths[c].cellIndex);
    }
    var d;
    for (d = 0; d < ths.length; d++) {
      var dir = ths[d].getAttribute("data-sort-default");
      if (dir === "desc" || dir === "asc") {
        sortTable(table, ths[d].cellIndex, ths[d], dir);
        break;
      }
    }
  }
  function nytprofSortInit() {
    var tables = document.getElementsByTagName("table");
    var i;
    for (i = 0; i < tables.length; i++) {
      bindTable(tables[i]);
    }
  }
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", nytprofSortInit);
  } else {
    nytprofSortInit();
  }
})();
"#;

/// Canonical multi-file exclusive-time sub index filename (`index-subs-excl.html`).
///
/// Oracle `nytprofhtml` emits the same name for the full exclusive-sorted
/// subroutine index. Native page is MVP (stable structure classes, not oracle
/// DOM/tablesorter). See `docs/schemas/html-subs-excl-index-mvp-v0.md`.
pub const INDEX_SUBS_EXCL_FILENAME: &str = "index-subs-excl.html";

/// Index `#subs_table` top-N (oracle `$max_subs = 15`). Full list is
/// [`INDEX_SUBS_EXCL_FILENAME`].
pub const INDEX_TOP_SUBS: usize = 15;

/// Multi-file folded-stack filename when `--flame` / [`HtmlRenderOptions::flame`].
///
/// Folded text matches [`render_folded_stacks`] (not oracle `nytprofcalls`
/// `.calls` dialect). See `docs/schemas/html-optional-flame-mvp-v0.md`.
pub const FLAME_FOLDED_FILENAME: &str = "all_stacks_by_time.folded";

/// Multi-file native flame SVG filename when flame is on (oracle-aligned basename;
/// **not** `flamegraph.pl` output).
pub const FLAME_SVG_FILENAME: &str = "all_stacks_by_time.svg";

/// Multi-file inter-package Graphviz source (6.15 basename; native edges).
pub const PACKAGES_CALLGRAPH_FILENAME: &str = "packages-callgraph.dot";

/// Multi-file inter-subroutine Graphviz source (6.15 basename; native edges).
pub const SUBS_CALLGRAPH_FILENAME: &str = "subs-callgraph.dot";

/// Self-contained HTML summary report (MVP; see `docs/schemas/html-report-mvp-v0.md`).
///
/// Includes profile path, event counts, subroutine table from `sub_return_totals`,
/// call edges, exclusive-time ranking, and a source section for the primary workload fid.
///
/// **CSS policy:** embeds [`SHARED_STYLE_CSS`] inline so the single document needs
/// no external assets (see `docs/schemas/html-shared-css-structure-mvp-v0.md`).
///
/// **Flame:** off by default; use [`render_html_summary_with_options`] with
/// [`HtmlRenderOptions::flame`] for an embedded native SVG section.
pub fn render_html_summary(model: &ProfileModel, profile_path: &str) -> String {
    render_html_summary_with_options(model, profile_path, HtmlRenderOptions::default())
}


/// Single-file HTML summary with optional flame embed.
///
/// When `opts.flame` is true, inserts a `section.flame` with an inlined native
/// SVG from [`render_flame_svg`] after the exclusive ranking (before source).
/// Does **not** write separate flame files (single-file path stays one document).
pub fn render_html_summary_with_options(
    model: &ProfileModel,
    profile_path: &str,
    opts: HtmlRenderOptions,
) -> String {
    let title = html_report_title(profile_path);
    let primary_fid = primary_workload_fid(model);
    let mut out = String::with_capacity(if opts.flame { 16384 } else { 8192 });
    push_html_doc_start(&mut out, &title, HtmlCssMode::Inline, HtmlJsMode::Inline);
    let app = application_basename(model);
    push_page_chrome(
        &mut out,
        "NYTProf Performance Profile",
        &format!("For {}", escape_html(&app)),
        false,
    );
    out.push_str("<div class=\"body_content\">\n");
    out.push_str(&format!("<h1>{}</h1>\n", escape_html(&title)));
    push_profile_path(&mut out, profile_path);
    push_event_counts(&mut out, model);
    push_subs_table(&mut out, model, false);
    push_sub_defs_table(&mut out, model, false);
    push_call_edges_table(&mut out, model, false);
    push_top_exclusive_table(&mut out, model, false);
    // Skip the section entirely when the profile has no call edges (oracle
    // nytprofhtml does not flame a no-calls profile either).
    if opts.flame && !collect_nonzero_call_edges(model).is_empty() {
        push_flame_section_embedded(&mut out, model);
    }
    push_source_heading(&mut out, model, primary_fid);
    push_source_table(&mut out, model, primary_fid, true);
    // A4b: all block_line_totals when present (blocks fixtures).
    push_block_line_totals_table(&mut out, model, None);
    out.push_str("</div>\n");
    out.push_str("</body>\n</html>\n");
    out
}


/// Multi-file HTML site: summary index + exclusive sub index + per-fid pages +
/// shared CSS + optional flame.
///
/// Eligible fids are those in [`ProfileModel::files`] that have at least one
/// `source_lines`, `line_totals`, or `block_line_totals` entry.
///
/// Site contents (default, flame **off**):
/// - `index.html` — summary; relative links to every `file-<fid>.html`, to
///   [`HtmlSite::source_filename`] (`source.html`) as a primary alias, and to
///   [`HtmlSite::index_subs_excl_filename`] (`index-subs-excl.html`)
/// - `index-subs-excl.html` — full exclusive-time subroutine ranking page
/// - `file-<fid>.html` — source + A4 (and A4b when present) for that fid
/// - `source.html` — copy of the primary workload file page (back-compat)
/// - `style.css` — shared MVP stylesheet ([`SHARED_STYLE_CSS`]); pages link via
///   `<link rel="stylesheet" href="style.css">`
/// - `nytprof-sort.js` — vanilla sort ([`SHARED_SORT_JS`]); pages use
///   `<script src="nytprof-sort.js" defer>`
///
/// When [`HtmlRenderOptions::flame`] is set (via
/// [`render_html_site_with_options`] / [`write_html_site_with_options`]), also:
/// - `all_stacks_by_time.folded` — folded stacks ([`render_folded_stacks`])
/// - `all_stacks_by_time.svg` — native call-tree flame SVG ([`render_flame_svg`])
/// - index inlines the SVG (hover + click-to-source) and links the sibling files
pub fn render_html_site(model: &ProfileModel, profile_path: &str) -> HtmlSite {
    render_html_site_with_options(model, profile_path, HtmlRenderOptions::default())
}

/// Multi-file HTML site with optional flame artifacts.
pub fn render_html_site_with_options(
    model: &ProfileModel,
    profile_path: &str,
    opts: HtmlRenderOptions,
) -> HtmlSite {
    let source_filename = "source.html".to_owned();
    let style_filename = STYLE_CSS_FILENAME.to_owned();
    let style_css = SHARED_STYLE_CSS.to_owned();
    let sort_js_filename = SORT_JS_FILENAME.to_owned();
    let sort_js = SHARED_SORT_JS.to_owned();
    let index_subs_excl_filename = INDEX_SUBS_EXCL_FILENAME.to_owned();
    let title = html_report_title(profile_path);
    let primary_fid = primary_workload_fid(model);
    let eligible = eligible_source_fids(model);
    let profile_base = Path::new(profile_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(profile_path);
    let css = HtmlCssMode::LinkedStyleSheet(style_filename.as_str());
    let js = HtmlJsMode::LinkedScript(sort_js_filename.as_str());

    let mut file_pages: Vec<(String, String)> = Vec::with_capacity(eligible.len());
    for fid in &eligible {
        let filename = file_page_filename(*fid);
        let page = render_file_page(
            model,
            *fid,
            profile_base,
            style_filename.as_str(),
            sort_js_filename.as_str(),
        );
        file_pages.push((filename, page));
    }
    let primary_name = file_page_filename(primary_fid);
    let source_html = file_pages
        .iter()
        .find(|(name, _)| name == &primary_name)
        .map(|(_, html)| html.clone())
        .unwrap_or_else(|| {
            render_file_page(
                model,
                primary_fid,
                profile_base,
                style_filename.as_str(),
                sort_js_filename.as_str(),
            )
        });

    let index_subs_excl_html = render_index_subs_excl_page(
        model,
        profile_path,
        profile_base,
        style_filename.as_str(),
        sort_js_filename.as_str(),
    );

    // Oracle parity: nytprofhtml skips flame when the profile has no calls
    // data — do not ship an empty SVG / folded pair for edge-less profiles.
    let (flame_folded, flame_folded_filename, flame_svg, flame_svg_filename) = if opts.flame {
        let stacks = collect_nonzero_call_edges(model);
        if stacks.is_empty() {
            (None, None, None, None)
        } else {
            (
                Some(folded_from_stacks(&stacks)),
                Some(FLAME_FOLDED_FILENAME.to_owned()),
                Some(render_flame_svg(model)),
                Some(FLAME_SVG_FILENAME.to_owned()),
            )
        }
    } else {
        (None, None, None, None)
    };

    let mut index = String::with_capacity(if opts.flame { 8192 } else { 6144 });
    push_html_doc_start(&mut index, &title, css, js);
    let app = application_basename(model);
    let mut subtitle = format!("For {}", escape_html(&app));
    if let Some(run) = run_time_label(model) {
        subtitle.push_str("<br>Run on ");
        subtitle.push_str(&escape_html(&run));
    }
    subtitle.push_str("<br>Reported on ");
    subtitle.push_str(&escape_html(&reported_on_now()));
    push_page_chrome(&mut index, "Performance Profile Index", &subtitle, false);
    index.push_str("<div class=\"body_content\">\n");
    push_index_summary(&mut index, model);
    if let (Some(ref folded_name), Some(ref svg_name)) =
        (&flame_folded_filename, &flame_svg_filename)
    {
        index.push_str("<div class=\"flamegraph\">\n");
        push_flame_section_links(
            &mut index,
            folded_name,
            svg_name,
            flame_svg.as_deref().unwrap_or(""),
        );
        index.push_str("</div>\n");
    }
    push_subs_table_v2(
        &mut index,
        model,
        SubsTableKind::Index,
        None,
        true,
    );
    let n_subs = model.sub_return_totals.len();
    index.push_str("<div class=\"table_footer\"><p class=\"subs-excl-link\"><a href=\"");
    index.push_str(&escape_html(index_subs_excl_filename.as_str()));
    index.push_str(&format!("\">See all {n_subs} subroutines</a></p></div>\n"));
    push_files_table(
        &mut index,
        model,
        &eligible,
        primary_fid,
        source_filename.as_str(),
    );
    push_profile_path(&mut index, profile_path);
    push_event_counts(&mut index, model);
    push_call_edges_table(&mut index, model, true);
    push_sub_defs_table(&mut index, model, true);
    push_block_line_totals_table(&mut index, model, None);
    index.push_str("<p class=\"callgraph-links\">Call graphs: <a href=\"");
    index.push_str(PACKAGES_CALLGRAPH_FILENAME);
    index.push_str("\">packages-callgraph.dot</a> · <a href=\"");
    index.push_str(SUBS_CALLGRAPH_FILENAME);
    index.push_str("\">subs-callgraph.dot</a></p>\n");
    index.push_str("</div>\n");
    index.push_str("<div class=\"footer\">NYTProfM native operator HTML v2</div>\n");
    index.push_str("</body>\n</html>\n");

    HtmlSite {
        index_html: index,
        source_html,
        source_filename,
        file_pages,
        index_subs_excl_html,
        index_subs_excl_filename,
        style_css,
        style_filename,
        sort_js,
        sort_js_filename,
        flame_folded,
        flame_folded_filename,
        flame_svg,
        flame_svg_filename,
        packages_callgraph_dot: render_packages_callgraph_dot(model),
        packages_callgraph_filename: PACKAGES_CALLGRAPH_FILENAME.to_owned(),
        subs_callgraph_dot: render_subs_callgraph_dot(model),
        subs_callgraph_filename: SUBS_CALLGRAPH_FILENAME.to_owned(),
    }
}


/// Fail-closed path rules for multi-file HTML `--out-dir` / [`write_html_site`].
///
/// See `docs/schemas/html-outdir-safety-mvp-v0.md`.
///
/// Rejects (returns `Err` with [`io::ErrorKind::InvalidInput`]):
/// - empty path (`""`)
/// - any path component that is `..` ([`Component::ParentDir`])
/// - null byte (`\0`) anywhere in the path's OS representation
///
/// Absolute paths are allowed (CLI may pass absolute dirs). Relative paths
/// without `..` or `\0` are allowed. Existing non-directory at `out_dir` is
/// checked later by the publish path (not here).
pub fn validate_html_out_dir(out_dir: &Path) -> io::Result<()> {
    if out_dir.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "write_html_site: out_dir path is empty",
        ));
    }
    if path_os_contains_nul(out_dir) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "write_html_site: out_dir path contains a null byte",
        ));
    }
    for c in out_dir.components() {
        if matches!(c, Component::ParentDir) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "write_html_site: out_dir must not contain '..' path components: {}",
                    out_dir.display()
                ),
            ));
        }
    }
    Ok(())
}

/// True if the path's OS bytes contain a NUL (`\0`).
fn path_os_contains_nul(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().contains(&0)
    }
    #[cfg(not(unix))]
    {
        // Best-effort: lossy UTF-8 may not preserve embedded NULs on all hosts.
        path.to_string_lossy().contains('\0')
    }
}

/// Write a multi-file HTML site under `out_dir` (fail-closed publish).
///
/// Renders the site in memory, writes all files into a sibling temporary
/// directory under `out_dir`'s parent (same filesystem for `rename`), then
/// publishes with rename:
///
/// - If `out_dir` does not exist: `rename(temp, out_dir)`.
/// - If `out_dir` exists as a directory: `rename(out_dir, bak)` →
///   `rename(temp, out_dir)` → best-effort remove `bak`. On failure after the
///   first rename, attempts to restore the previous `out_dir` from `bak`.
///
/// On any failure before a successful final rename, an existing `out_dir` is
/// left unchanged (when restore succeeds) and the temp directory is removed.
/// If `out_dir` already exists and is not a directory, returns an error without
/// writing a partial site there.
///
/// Path safety ([`validate_html_out_dir`]) runs before any create/write.
///
/// Writes `index.html`, `index-subs-excl.html`, every `file-<fid>.html` from
/// [`HtmlSite::file_pages`], `source.html` (primary alias), shared
/// [`HtmlSite::style_filename`] (`style.css`), and [`HtmlSite::sort_js_filename`]
/// (`nytprof-sort.js`). Flame files are **not** written on the default options path.
///
/// Returns the rendered [`HtmlSite`] so callers can list filenames written.
pub fn write_html_site(
    model: &ProfileModel,
    profile_path: &str,
    out_dir: &Path,
) -> io::Result<HtmlSite> {
    write_html_site_with_options(model, profile_path, out_dir, HtmlRenderOptions::default())
}

/// Write multi-file HTML site with options (e.g. opt-in flame artifacts).
///
/// When `opts.flame` is true, also publishes [`FLAME_FOLDED_FILENAME`] and
/// [`FLAME_SVG_FILENAME`] atomically with the rest of the site.
pub fn write_html_site_with_options(
    model: &ProfileModel,
    profile_path: &str,
    out_dir: &Path,
    opts: HtmlRenderOptions,
) -> io::Result<HtmlSite> {
    validate_html_out_dir(out_dir)?;
    let site = render_html_site_with_options(model, profile_path, opts);
    publish_html_site_files(&site, out_dir)?;
    Ok(site)
}

/// Write rendered site files into `out_dir` via temp-then-rename (see [`write_html_site`]).
fn publish_html_site_files(site: &HtmlSite, out_dir: &Path) -> io::Result<()> {
    // Defense in depth if called without going through write_html_site.
    validate_html_out_dir(out_dir)?;

    if out_dir.exists() && !out_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "write_html_site: out_dir exists and is not a directory: {}",
                out_dir.display()
            ),
        ));
    }

    let parent = out_dir_parent(out_dir);
    fs::create_dir_all(&parent)?;

    let temp_dir = create_temp_site_dir(&parent)?;
    let write_result = (|| -> io::Result<()> {
        fs::write(temp_dir.join("index.html"), site.index_html.as_bytes())?;
        fs::write(
            temp_dir.join(&site.index_subs_excl_filename),
            site.index_subs_excl_html.as_bytes(),
        )?;
        for (filename, html) in &site.file_pages {
            fs::write(temp_dir.join(filename), html.as_bytes())?;
        }
        fs::write(
            temp_dir.join(&site.source_filename),
            site.source_html.as_bytes(),
        )?;
        fs::write(
            temp_dir.join(&site.style_filename),
            site.style_css.as_bytes(),
        )?;
        fs::write(
            temp_dir.join(&site.sort_js_filename),
            site.sort_js.as_bytes(),
        )?;
        if let (Some(body), Some(name)) = (&site.flame_folded, &site.flame_folded_filename) {
            fs::write(temp_dir.join(name), body.as_bytes())?;
        }
        if let (Some(body), Some(name)) = (&site.flame_svg, &site.flame_svg_filename) {
            fs::write(temp_dir.join(name), body.as_bytes())?;
        }
        fs::write(
            temp_dir.join(&site.packages_callgraph_filename),
            site.packages_callgraph_dot.as_bytes(),
        )?;
        fs::write(
            temp_dir.join(&site.subs_callgraph_filename),
            site.subs_callgraph_dot.as_bytes(),
        )?;
        atomic_replace_dir(&temp_dir, out_dir)
    })();

    if write_result.is_err() {
        let _ = fs::remove_dir_all(&temp_dir);
    }
    write_result
}


/// Parent directory of `out_dir` (`.` when the path has no parent component).
fn out_dir_parent(out_dir: &Path) -> PathBuf {
    match out_dir.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// Create a unique sibling temp directory (e.g. `.nytprof-html-<pid>-<nanos>`).
fn create_temp_site_dir(parent: &Path) -> io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    // A few attempts in case of collision under parallel tests.
    for attempt in 0u32..16 {
        let name = if attempt == 0 {
            format!(".nytprof-html-{pid}-{nanos}")
        } else {
            format!(".nytprof-html-{pid}-{nanos}-{attempt}")
        };
        let path = parent.join(name);
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "write_html_site: could not create unique temp directory",
    ))
}

/// Atomically publish `temp_dir` as `out_dir` via rename (Linux-friendly).
///
/// Preferred path when `out_dir` exists: rename to `*.bak`, rename temp in,
/// remove bak. Caller removes `temp_dir` only on failure before/without a
/// successful rename of temp.
fn atomic_replace_dir(temp_dir: &Path, out_dir: &Path) -> io::Result<()> {
    if !out_dir.exists() {
        return fs::rename(temp_dir, out_dir);
    }

    let bak = unique_bak_path(out_dir);
    // Clear a leftover bak from a prior crash if present.
    if bak.exists() {
        if bak.is_dir() {
            fs::remove_dir_all(&bak)?;
        } else {
            fs::remove_file(&bak)?;
        }
    }

    fs::rename(out_dir, &bak)?;
    match fs::rename(temp_dir, out_dir) {
        Ok(()) => {
            let _ = fs::remove_dir_all(&bak);
            Ok(())
        }
        Err(e) => {
            // Best-effort restore of the previous site.
            let _ = fs::rename(&bak, out_dir);
            Err(e)
        }
    }
}

fn unique_bak_path(out_dir: &Path) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let file_name = out_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("nytprof-html-out");
    let parent = out_dir_parent(out_dir);
    parent.join(format!(".{file_name}.bak-{nanos}"))
}

/// Relative filename for a per-fid source page.
pub fn file_page_filename(fid: u32) -> String {
    format!("file-{fid}.html")
}

fn dot_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn package_name_of(sub: &str) -> &str {
    match sub.rfind("::") {
        Some(i) if i > 0 => &sub[..i],
        _ => "main",
    }
}

/// Graphviz `digraph` of inter-package call edges from [`ProfileModel::call_edges`].
pub fn render_packages_callgraph_dot(model: &ProfileModel) -> String {
    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    for ((caller, called), e) in &model.call_edges {
        if e.count == 0 || caller.is_empty() || called.is_empty() {
            continue;
        }
        edges.insert((
            package_name_of(caller).to_owned(),
            package_name_of(called).to_owned(),
        ));
    }
    let mut out = String::from("digraph {\ngraph [overlap=false]\n");
    for (a, b) in &edges {
        out.push_str(&dot_quote(a));
        out.push_str(" -> ");
        out.push_str(&dot_quote(b));
        out.push_str(";\n");
    }
    out.push_str("}\n");
    out
}

/// Graphviz `digraph` of inter-subroutine call edges from [`ProfileModel::call_edges`].
pub fn render_subs_callgraph_dot(model: &ProfileModel) -> String {
    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    for ((caller, called), e) in &model.call_edges {
        if e.count == 0 || caller.is_empty() || called.is_empty() {
            continue;
        }
        edges.insert((caller.clone(), called.clone()));
    }
    let mut out = String::from("digraph {\ngraph [overlap=false]\n");
    for (a, b) in &edges {
        out.push_str(&dot_quote(a));
        out.push_str(" -> ");
        out.push_str(&dot_quote(b));
        out.push_str(";\n");
    }
    out.push_str("}\n");
    out
}

/// Fids in `model.files` that have source text and/or A4/A4b line data.
fn eligible_source_fids(model: &ProfileModel) -> Vec<u32> {
    let mut fids: Vec<u32> = model
        .files
        .keys()
        .copied()
        .filter(|&fid| {
            model.source_lines.keys().any(|(f, _)| *f == fid)
                || model.line_totals.keys().any(|(f, _)| *f == fid)
                || model.block_line_totals.keys().any(|(f, _)| *f == fid)
        })
        .collect();
    fids.sort_unstable();
    fids
}

/// Render one per-fid HTML page body (full document; links shared stylesheet).
///
/// `style_filename` is the same relative name written as
/// [`HtmlSite::style_filename`] (normally [`STYLE_CSS_FILENAME`]).
fn render_file_page(
    model: &ProfileModel,
    fid: u32,
    profile_base: &str,
    style_filename: &str,
    sort_filename: &str,
) -> String {
    let basename = model
        .fid_basename(fid)
        .map(|s| s.to_owned())
        .unwrap_or_else(|| source_file_label(model, fid));
    let src_title = format!("Source — {basename} (fid {fid}) — {profile_base}");
    let mut page = String::with_capacity(4096);
    push_html_doc_start(
        &mut page,
        &src_title,
        HtmlCssMode::LinkedStyleSheet(style_filename),
        HtmlJsMode::LinkedScript(sort_filename),
    );
    push_page_chrome(
        &mut page,
        "NYTProf Performance Profile",
        &format!("{} — « line view »", escape_html(&basename)),
        true,
    );
    page.push_str("<div class=\"body_content\">\n");
    push_file_summary(&mut page, model, fid);
    push_subs_table_v2(&mut page, model, SubsTableKind::File, Some(fid), true);
    push_source_heading(&mut page, model, fid);
    let emit_stubs = fid == primary_workload_fid(model);
    push_source_table(&mut page, model, fid, emit_stubs);
    // A4b: block_line_totals for this fid when present (blocks fixtures).
    push_block_line_totals_table(&mut page, model, Some(fid));
    page.push_str("</div>\n");
    page.push_str("<div class=\"footer\">NYTProfM native operator HTML v2</div>\n");
    page.push_str("</body>\n</html>\n");
    page
}

/// Index `#filestable`: Stmts / Exclusive Time / Reports / Source File.
fn push_files_table(
    out: &mut String,
    model: &ProfileModel,
    eligible: &[u32],
    primary_fid: u32,
    source_filename: &str,
) {
    out.push_str("<h2>Source Code Files</h2>\n");
    out.push_str("<table id=\"filestable\" class=\"sortable\">\n");
    out.push_str("<caption>Source Code Files</caption>\n<thead><tr>");
    push_th(out, "Stmts", "num");
    push_th_default(out, "Exclusive<br>Time", "num", "desc");
    out.push_str("<th>Reports</th>");
    push_th(out, "Source File", "text");
    out.push_str("</tr></thead>\n<tbody>\n");
    let tps = ticks_per_sec_attr(model);
    let mut total_stmts: u64 = 0;
    let mut total_ticks: i64 = 0;
    let heat_vals: Vec<f64> = eligible
        .iter()
        .map(|fid| fid_exclusive_ticks(model, *fid) as f64)
        .collect();
    let scale = HeatScale::from_values(&heat_vals);
    for fid in eligible {
        let stmts = fid_stmt_calls(model, *fid);
        let ticks = fid_exclusive_ticks(model, *fid);
        total_stmts = total_stmts.saturating_add(stmts);
        total_ticks = total_ticks.saturating_add(ticks);
        let href = file_page_filename(*fid);
        let label = model
            .fid_basename(*fid)
            .map(|s| s.to_owned())
            .unwrap_or_else(|| source_file_label(model, *fid));
        let heat = heat_class(ticks as f64, &scale);
        out.push_str("<tr>");
        push_count_td_heat(out, stmts, if stmts > 0 { heat } else { "" });
        push_time_td_heat(out, ticks as f64, tps, heat);
        out.push_str("<td><a href=\"");
        out.push_str(&escape_html(&href));
        out.push_str("\">line</a></td>");
        out.push_str("<td>");
        out.push_str(&escape_html(&label));
        out.push_str("</td></tr>\n");
    }
    out.push_str("</tbody>\n<tfoot><tr>");
    push_count_td(out, total_stmts);
    push_time_td(out, total_ticks as f64, tps);
    out.push_str("<td></td><td></td></tr></tfoot>\n</table>\n");
    out.push_str(&format!(
        "<p class=\"source-link\"><a href=\"{}\">Source — {}</a> (primary alias)</p>\n",
        escape_html(source_filename),
        escape_html(&source_file_label(model, primary_fid)),
    ));
}

fn html_report_title(profile_path: &str) -> String {
    let basename = Path::new(profile_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(profile_path);
    format!("NYTProf report — {}", basename)
}

fn source_file_label(model: &ProfileModel, primary_fid: u32) -> String {
    model
        .file_name(primary_fid)
        .map(|s| s.to_owned())
        .or_else(|| model.fid_basename(primary_fid).map(|s| s.to_owned()))
        .unwrap_or_else(|| format!("fid {primary_fid}"))
}

fn push_html_doc_start(out: &mut String, title: &str, css: HtmlCssMode<'_>, js: HtmlJsMode<'_>) {
    out.push_str("<!DOCTYPE html>\n");
    out.push_str("<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n");
    out.push_str(&format!("<title>{}</title>\n", escape_html(title)));
    match css {
        HtmlCssMode::Inline => {
            out.push_str("<style>\n");
            out.push_str(SHARED_STYLE_CSS);
            out.push_str("</style>\n");
        }
        HtmlCssMode::LinkedStyleSheet(style_filename) => {
            // Relative sibling asset: same name as HtmlSite::style_filename / disk write.
            out.push_str(&format!(
                "<link rel=\"stylesheet\" href=\"{}\">\n",
                escape_html(style_filename)
            ));
        }
    }
    match js {
        HtmlJsMode::Inline => {
            out.push_str("<script>\n");
            out.push_str(SHARED_SORT_JS);
            out.push_str("</script>\n");
        }
        HtmlJsMode::LinkedScript(sort_filename) => {
            out.push_str(&format!(
                "<script src=\"{}\" defer></script>\n",
                escape_html(sort_filename)
            ));
        }
    }
    out.push_str("</head>\n<body>\n");
}

/// Shared header chrome (`div.header` / `.siteTitle` / `.siteSubtitle` / `.header_back`).
///
/// `subtitle` may include pre-escaped HTML (e.g. `<br>`). `site_title` is escaped.
fn push_page_chrome(out: &mut String, site_title: &str, subtitle: &str, back_to_index: bool) {
    out.push_str("<div class=\"header\">\n");
    if back_to_index {
        out.push_str("<div class=\"header_back\"><a href=\"index.html\">← Index</a></div>\n");
    }
    out.push_str("<div class=\"siteTitle\">");
    out.push_str(&escape_html(site_title));
    out.push_str("</div>\n");
    if !subtitle.is_empty() {
        out.push_str("<div class=\"siteSubtitle\">");
        out.push_str(subtitle);
        out.push_str("</div>\n");
    }
    out.push_str("</div>\n");
}

fn parse_attr_f64(model: &ProfileModel, key: &str) -> Option<f64> {
    model
        .attributes
        .get(key)
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn run_time_label(model: &ProfileModel) -> Option<String> {
    for key in ["start_date", "run_date", "basetime_iso"] {
        if let Some(v) = model.attributes.get(key) {
            if !v.is_empty() && v.parse::<f64>().is_err() {
                return Some(v.clone());
            }
        }
    }
    None
}

fn reported_on_now() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => format!("unix {}", d.as_secs()),
        Err(_) => "unknown".to_owned(),
    }
}

fn push_index_summary(out: &mut String, model: &ProfileModel) {
    let app = application_basename(model);
    let stmt_secs = parse_attr_f64(model, "profiler_active").or_else(|| {
        let tps = ticks_per_sec_attr(model)?;
        let ticks: i64 = model.line_totals.values().map(|t| t.ticks).sum();
        Some(ticks as f64 / tps as f64)
    });
    // Wall only from a real duration attribute — never `application`.
    let wall_secs = parse_attr_f64(model, "profiler_duration")
        .or_else(|| parse_attr_f64(model, "duration"));
    let stmt_count: u64 = model.line_totals.values().map(|t| t.calls).sum();
    let sub_calls: u64 = model.sub_return_totals.values().map(|t| t.returns).sum();
    let file_count = model.files.len();
    let stmt_disp = stmt_secs
        .map(format_compact_secs)
        .unwrap_or_else(|| "0s".to_owned());
    out.push_str("<div class=\"index_summary\">Profile of ");
    out.push_str(&escape_html(&app));
    out.push_str(" for ");
    out.push_str(&escape_html(&stmt_disp));
    if let Some(wall) = wall_secs {
        out.push_str(" (of ");
        out.push_str(&escape_html(&format_compact_secs(wall)));
        out.push(')');
    }
    out.push_str(&format!(
        ", executing {stmt_count} statements and {sub_calls} subroutine calls in {file_count} source files.</div>\n"
    ));
}

fn push_file_summary(out: &mut String, model: &ProfileModel, fid: u32) {
    let label = source_file_label(model, fid);
    let stmts = fid_stmt_calls(model, fid);
    let ticks = fid_exclusive_ticks(model, fid);
    let tps = ticks_per_sec_attr(model);
    let secs = match tps {
        Some(n) if n > 0 => format_compact_secs(ticks as f64 / n as f64),
        _ => format_ticks(ticks as f64),
    };
    out.push_str("<table class=\"file_summary\">\n");
    out.push_str("<tr><th>Filename</th><td>");
    out.push_str(&escape_html(&label));
    out.push_str("</td></tr>\n<tr><th>Statements</th><td>");
    out.push_str(&escape_html(&format!(
        "Executed {stmts} statements in {secs}"
    )));
    out.push_str("</td></tr>\n</table>\n");
}

fn push_profile_path(out: &mut String, profile_path: &str) {
    out.push_str(&format!(
        "<p class=\"profile-path\">Profile: <code>{}</code></p>\n",
        escape_html(profile_path)
    ));
}

fn push_event_counts(out: &mut String, model: &ProfileModel) {
    out.push_str("<h2>Event counts</h2>\n<ul>\n");
    out.push_str(&format!(
        "<li>time_line_events: {}</li>\n",
        model.time_line_events
    ));
    out.push_str(&format!(
        "<li>time_block_events: {}</li>\n",
        model.time_block_events
    ));
    out.push_str(&format!(
        "<li>discount_events: {}</li>\n",
        model.discount_events
    ));
    if let Some(tps) = ticks_per_sec_attr(model) {
        out.push_str(&format!("<li>ticks_per_sec: {tps}</li>\n"));
    }
    out.push_str("</ul>\n");
}

/// Which 6-col `#subs_table` to emit (operator HTML v2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubsTableKind {
    /// Index: top [`INDEX_TOP_SUBS`], exclusive-desc, no `data-sort-default`.
    Index,
    /// Exclusive index page: all rows, `data-sort-default=desc`, class `subs-excl`.
    ExclIndex,
    /// Per-file: subs in this fid (+ CORE: names with edges), default Exclusive desc.
    File,
}

fn push_subs_table_v2(
    out: &mut String,
    model: &ProfileModel,
    kind: SubsTableKind,
    only_fid: Option<u32>,
    multi_file: bool,
) {
    let mut rows = sorted_subs_by_exclusive(model);
    if let Some(fid) = only_fid {
        rows.retain(|(name, _)| sub_belongs_on_file_page(model, name, fid));
    }
    let extra_core: Vec<(String, nytprof_model::SubTotal)> = if let Some(fid) = only_fid {
        core_names_for_fid(model, fid)
            .into_iter()
            .filter(|name| {
                !rows.iter().any(|(n, _)| *n == name) && !model.sub_return_totals.contains_key(name)
            })
            .map(|name| {
                (
                    name,
                    nytprof_model::SubTotal {
                        returns: 0,
                        incl: 0.0,
                        excl: 0.0,
                    },
                )
            })
            .collect()
    } else {
        Vec::new()
    };

    if matches!(kind, SubsTableKind::Index) {
        rows.truncate(INDEX_TOP_SUBS);
    }

    let class = match kind {
        SubsTableKind::ExclIndex => "subs-excl sortable",
        _ => "sortable",
    };
    let default_excl = !matches!(kind, SubsTableKind::Index);

    out.push_str("<h2>Subroutines</h2>\n");
    out.push_str("<table id=\"subs_table\" class=\"");
    out.push_str(class);
    out.push_str("\">\n<caption>Subroutines</caption>\n<thead><tr>");
    push_th(out, "Calls", "num");
    push_th(out, "P", "num");
    push_th(out, "F", "num");
    if default_excl {
        push_th_default(out, "Exclusive<br>Time", "num", "desc");
    } else {
        push_th_html(out, "Exclusive<br>Time", "num");
    }
    push_th_html(out, "Inclusive<br>Time", "num");
    push_th(out, "Subroutine", "text");
    out.push_str("</tr></thead>\n<tbody>\n");
    let tps = ticks_per_sec_attr(model);
    let heat_vals: Vec<f64> = rows
        .iter()
        .map(|(_, t)| t.excl)
        .chain(extra_core.iter().map(|(_, t)| t.excl))
        .collect();
    let scale = HeatScale::from_values(&heat_vals);
    for (name, t) in rows {
        push_subs_v2_row(out, model, name, t, tps, &scale, multi_file);
    }
    for (name, t) in &extra_core {
        push_subs_v2_row(out, model, name, t, tps, &scale, multi_file);
    }
    out.push_str("</tbody>\n</table>\n");
}

fn push_subs_v2_row(
    out: &mut String,
    model: &ProfileModel,
    name: &str,
    t: &nytprof_model::SubTotal,
    tps: Option<u64>,
    scale: &HeatScale,
    multi_file: bool,
) {
    let (p, f) = sub_places_and_files(model, name);
    let heat = heat_class(t.excl, scale);
    out.push_str("<tr");
    if !heat.is_empty() {
        out.push_str(" class=\"");
        out.push_str(heat);
        out.push('"');
    }
    out.push('>');
    push_count_td_heat(out, t.returns, if t.returns > 0 { heat } else { "" });
    push_count_td(out, p);
    push_count_td(out, f);
    push_time_td_heat(out, t.excl, tps, heat);
    push_time_td_heat(out, t.incl, tps, heat_class(t.incl, scale));
    push_sub_name_cell(out, model, name, multi_file);
    out.push_str("</tr>\n");
}

fn sub_belongs_on_file_page(model: &ProfileModel, name: &str, fid: u32) -> bool {
    if model.sub_defs.get(name).is_some_and(|d| d.fid == fid) {
        return true;
    }
    name.contains("CORE:") && sub_involves_fid(model, name, fid)
}

fn sub_involves_fid(model: &ProfileModel, name: &str, fid: u32) -> bool {
    model.call_edges.keys().any(|(caller, called)| {
        if caller != name && called != name {
            return false;
        }
        model
            .sub_defs
            .get(caller)
            .is_some_and(|d| d.fid == fid)
            || model.sub_defs.get(called).is_some_and(|d| d.fid == fid)
            || (primary_workload_fid(model) == fid)
    })
}

fn core_names_for_fid(model: &ProfileModel, fid: u32) -> Vec<String> {
    let mut names = BTreeSet::new();
    for (caller, called) in model.call_edges.keys() {
        for n in [caller, called] {
            if n.contains("CORE:") && sub_involves_fid(model, n, fid) {
                names.insert(n.clone());
            }
        }
    }
    names.into_iter().collect()
}

fn sub_places_and_files(model: &ProfileModel, name: &str) -> (u64, u64) {
    let mut callers: BTreeSet<&str> = BTreeSet::new();
    let mut fids: BTreeSet<u32> = BTreeSet::new();
    for (caller, called) in model.call_edges.keys() {
        if called == name {
            callers.insert(caller.as_str());
            if let Some(d) = model.sub_defs.get(caller) {
                fids.insert(d.fid);
            }
        }
    }
    let p = callers.len() as u64;
    let f = if fids.is_empty() { 1 } else { fids.len() as u64 };
    (p, f)
}

fn push_subs_table(out: &mut String, model: &ProfileModel, multi_file: bool) {
    out.push_str("<h2>Subroutines</h2>\n");
    out.push_str("<table class=\"subs sortable\">\n<thead><tr>");
    push_th(out, "name", "text");
    push_th(out, "returns", "num");
    push_th(out, "incl", "num");
    push_th(out, "excl", "num");
    out.push_str("</tr></thead>\n<tbody>\n");
    let tps = ticks_per_sec_attr(model);
    let mut subs: Vec<_> = model.sub_return_totals.iter().collect();
    subs.sort_by_key(|(a, _)| *a);
    let heat_vals: Vec<f64> = subs.iter().map(|(_, t)| t.excl).collect();
    let scale = HeatScale::from_values(&heat_vals);
    for (name, t) in subs {
        let heat = heat_class(t.excl, &scale);
        out.push_str("<tr class=\"");
        out.push_str(heat);
        out.push_str("\">");
        push_sub_name_cell(out, model, name, multi_file);
        push_count_td(out, t.returns);
        push_time_td(out, t.incl, tps);
        push_time_td(out, t.excl, tps);
        out.push_str("</tr>\n");
    }
    out.push_str("</tbody>\n</table>\n");
}

/// Optional A9 `sub_defs` table (skipped when empty).
fn push_sub_defs_table(out: &mut String, model: &ProfileModel, multi_file: bool) {
    if model.sub_defs.is_empty() {
        return;
    }
    out.push_str("<h2>Subroutine definitions</h2>\n");
    out.push_str("<table class=\"sub-defs sortable\">\n<thead><tr>");
    push_th(out, "name", "text");
    push_th(out, "fid", "num");
    push_th(out, "first", "num");
    push_th(out, "last", "num");
    out.push_str("</tr></thead>\n<tbody>\n");
    let mut defs: Vec<_> = model.sub_defs.iter().collect();
    defs.sort_by_key(|(a, _)| *a);
    for (name, d) in defs {
        out.push_str("<tr>");
        push_sub_name_cell(out, model, name, multi_file);
        push_count_td(out, u64::from(d.fid));
        push_count_td(out, u64::from(d.first_line));
        push_count_td(out, u64::from(d.last_line));
        out.push_str("</tr>\n");
    }
    out.push_str("</tbody>\n</table>\n");
}

fn push_call_edges_table(out: &mut String, model: &ProfileModel, multi_file: bool) {
    // Call edges (A7): count desc, then caller, then called.
    out.push_str("<h2>Call edges</h2>\n");
    out.push_str("<table class=\"call-edges sortable\">\n<thead><tr>");
    push_th(out, "caller", "text");
    push_th(out, "called", "text");
    push_th(out, "count", "num");
    push_th(out, "incl", "num");
    push_th(out, "excl", "num");
    out.push_str("</tr></thead>\n<tbody>\n");
    let tps = ticks_per_sec_attr(model);
    let mut edges: Vec<_> = model.call_edges.iter().collect();
    edges.sort_by(|((c1, d1), e1), ((c2, d2), e2)| {
        e2.count
            .cmp(&e1.count)
            .then_with(|| c1.cmp(c2))
            .then_with(|| d1.cmp(d2))
    });
    let heat_vals: Vec<f64> = edges.iter().map(|(_, e)| e.excl).collect();
    let scale = HeatScale::from_values(&heat_vals);
    for ((caller, called), e) in edges {
        let heat = heat_class(e.excl, &scale);
        out.push_str("<tr class=\"");
        out.push_str(heat);
        out.push_str("\">");
        push_sub_name_cell(out, model, caller, multi_file);
        push_sub_name_cell(out, model, called, multi_file);
        push_count_td(out, e.count);
        push_time_td(out, e.incl, tps);
        push_time_td(out, e.excl, tps);
        out.push_str("</tr>\n");
    }
    out.push_str("</tbody>\n</table>\n");
}

fn push_top_exclusive_table(out: &mut String, model: &ProfileModel, multi_file: bool) {
    // Exclusive-time ranking (summary): excl desc, then name for stability.
    // Full page lives at `index-subs-excl.html` (see `push_subs_excl_table`).
    out.push_str("<h2>Top exclusive</h2>\n");
    out.push_str("<table class=\"top-exclusive sortable\">\n<thead><tr>");
    push_th(out, "name", "text");
    push_th(out, "excl", "num");
    push_th(out, "returns", "num");
    out.push_str("</tr></thead>\n<tbody>\n");
    let tps = ticks_per_sec_attr(model);
    let rows = sorted_subs_by_exclusive(model);
    let heat_vals: Vec<f64> = rows.iter().map(|(_, t)| t.excl).collect();
    let scale = HeatScale::from_values(&heat_vals);
    for (name, t) in rows {
        let heat = heat_class(t.excl, &scale);
        out.push_str("<tr class=\"");
        out.push_str(heat);
        out.push_str("\">");
        push_sub_name_cell(out, model, name, multi_file);
        push_time_td(out, t.excl, tps);
        push_count_td(out, t.returns);
        out.push_str("</tr>\n");
    }
    out.push_str("</tbody>\n</table>\n");
}

/// All `sub_return_totals` sorted by exclusive time descending, then name.
fn sorted_subs_by_exclusive(model: &ProfileModel) -> Vec<(&String, &nytprof_model::SubTotal)> {
    let mut by_excl: Vec<_> = model.sub_return_totals.iter().collect();
    by_excl.sort_by(|(n1, t1), (n2, t2)| t2.excl.total_cmp(&t1.excl).then_with(|| n1.cmp(n2)));
    by_excl
}

/// Render the multi-file exclusive sub index page (`index-subs-excl.html`).
///
/// See `docs/schemas/html-subs-excl-index-mvp-v0.md`.
fn render_index_subs_excl_page(
    model: &ProfileModel,
    profile_path: &str,
    profile_base: &str,
    style_filename: &str,
    sort_filename: &str,
) -> String {
    let title = format!("Subroutine exclusive index — {profile_base}");
    let mut page = String::with_capacity(4096);
    push_html_doc_start(
        &mut page,
        &title,
        HtmlCssMode::LinkedStyleSheet(style_filename),
        HtmlJsMode::LinkedScript(sort_filename),
    );
    push_page_chrome(
        &mut page,
        "Performance Profile Subroutine Index",
        &format!("For {}", escape_html(&application_basename(model))),
        true,
    );
    page.push_str("<div class=\"body_content\">\n");
    push_profile_path(&mut page, profile_path);
    push_subs_table_v2(&mut page, model, SubsTableKind::ExclIndex, None, true);
    page.push_str("</div>\n");
    page.push_str("<div class=\"footer\">NYTProfM native operator HTML v2</div>\n");
    page.push_str("</body>\n</html>\n");
    page
}

fn push_source_heading(out: &mut String, model: &ProfileModel, primary_fid: u32) {
    let file_label = source_file_label(model, primary_fid);
    out.push_str(&format!(
        "<h2>Source — {} (fid {})</h2>\n",
        escape_html(&file_label),
        primary_fid
    ));
}

fn push_source_table(
    out: &mut String,
    model: &ProfileModel,
    primary_fid: u32,
    emit_opcode_stubs: bool,
) {
    out.push_str("<table class=\"source sortable\">\n<thead><tr>");
    push_th(out, "Line", "num");
    push_th(out, "Statements", "num");
    push_th_html(out, "Time on line", "num");
    push_th(out, "Calls", "num");
    push_th_html(out, "Time in subs", "num");
    push_th(out, "Code", "text");
    out.push_str("</tr></thead>\n<tbody>\n");

    // Sorted union of SRC_LINE text, A4 line_totals, and A4b block_line lines.
    let mut lines: BTreeSet<u32> = BTreeSet::new();
    for (fid, line) in model.source_lines.keys() {
        if *fid == primary_fid {
            lines.insert(*line);
        }
    }
    for (fid, line) in model.line_totals.keys() {
        if *fid == primary_fid {
            lines.insert(*line);
        }
    }
    for (fid, line) in model.block_line_totals.keys() {
        if *fid == primary_fid {
            lines.insert(*line);
        }
    }

    let tps = ticks_per_sec_attr(model);
    let heat_vals: Vec<f64> = lines
        .iter()
        .filter_map(|line| {
            model
                .line_totals
                .get(&(primary_fid, *line))
                .map(|t| t.ticks as f64)
                .filter(|v| *v > 0.0)
        })
        .collect();
    let scale = HeatScale::from_values(&heat_vals);
    let stmt_vals: Vec<f64> = lines
        .iter()
        .filter_map(|line| {
            model
                .line_totals
                .get(&(primary_fid, *line))
                .map(|t| t.calls as f64)
                .filter(|v| *v > 0.0)
        })
        .collect();
    let stmt_scale = HeatScale::from_values(&stmt_vals);
    let calls_by_line = line_call_aggs(model);
    let stub_ins = stub_call_ins(model);

    for line in lines {
        let totals = model.line_totals.get(&(primary_fid, line));
        let ticks_val = totals.map(|t| t.ticks as f64).unwrap_or(0.0);
        let agg = calls_by_line.get(&(primary_fid, line));
        out.push_str(&format!("<tr id=\"L{line}\">"));
        push_count_td(out, u64::from(line));
        match totals {
            Some(t) => {
                let stmt_heat = heat_class(t.calls as f64, &stmt_scale);
                let time_heat = heat_class(ticks_val, &scale);
                push_count_td_heat(out, t.calls, stmt_heat);
                push_time_td_heat(out, t.ticks as f64, tps, time_heat);
            }
            None => {
                push_placeholder_num_td(out);
                push_placeholder_num_td(out);
            }
        }
        match agg {
            Some(a) if a.out_count > 0 => {
                push_count_td(out, a.out_count);
                push_time_td(out, a.out_incl, tps);
            }
            _ => {
                push_placeholder_num_td(out);
                push_placeholder_num_td(out);
            }
        }
        let display = model
            .source_lines
            .get(&(primary_fid, line))
            .map(|text| text.trim_end_matches(['\n', '\r']).to_owned())
            .unwrap_or_else(|| "—".to_owned());
        out.push_str("<td class=\"s\">");
        out.push_str(&escape_html(&display));
        if let Some(a) = agg {
            push_call_annotations(out, model, a, true);
        }
        out.push_str("</td></tr>\n");
    }

    if emit_opcode_stubs {
        for name in opcode_stub_names(model) {
            let id = opcode_fragment_id(&name);
            out.push_str("<tr id=\"");
            out.push_str(&escape_html(&id));
            out.push_str("\">");
            push_placeholder_num_td(out);
            push_placeholder_num_td(out);
            push_placeholder_num_td(out);
            push_placeholder_num_td(out);
            push_placeholder_num_td(out);
            out.push_str("<td class=\"s\">");
            out.push_str(&escape_html(&name));
            if name.contains("CORE:") {
                out.push_str(" (opcode)");
            }
            if let Some(ins) = stub_ins.get(&name) {
                for (caller, cfid, cline, count) in ins {
                    out.push_str("<div class=\"calls calls_in\"># spent ");
                    out.push_str(&count.to_string());
                    out.push_str(" times called from ");
                    push_call_site_link(out, model, caller, *cfid, *cline, true);
                    out.push_str("</div>");
                }
            }
            out.push_str("</td></tr>\n");
        }
    }
    out.push_str("</tbody>\n</table>\n");
}

/// A4b — dedicated **Block line totals** table from `model.block_line_totals`.
///
/// Skipped when empty (or when `only_fid` filters out every entry). Rows sorted
/// by `(fid, block_line)`. Columns: fid, block_line, calls, ticks.
fn push_block_line_totals_table(out: &mut String, model: &ProfileModel, only_fid: Option<u32>) {
    let mut rows: Vec<_> = model
        .block_line_totals
        .iter()
        .filter(|((fid, _), _)| only_fid.map(|f| *fid == f).unwrap_or(true))
        .collect();
    if rows.is_empty() {
        return;
    }
    rows.sort_by_key(|((fid, block_line), _)| (*fid, *block_line));

    out.push_str("<h2>Block line totals</h2>\n");
    out.push_str(
        "<table class=\"block-line-totals\">\n<thead><tr>\
         <th>fid</th><th>block_line</th><th>calls</th><th>ticks</th>\
         </tr></thead>\n<tbody>\n",
    );
    for ((fid, block_line), t) in rows {
        out.push_str(&format!(
            "<tr><td class=\"num\">{fid}</td>\
             <td class=\"num\">{block_line}</td>\
             <td class=\"num\">{calls}</td>\
             <td class=\"num\">{ticks}</td></tr>\n",
            fid = fid,
            block_line = block_line,
            calls = t.calls,
            ticks = t.ticks,
        ));
    }
    out.push_str("</tbody>\n</table>\n");
}

/// Folded-stack export for flamegraph-style tools (export-formats MVP v0).
///
/// One line per `call_edges` entry with non-zero count:
///
/// ```text
/// {caller};{called} {count}
/// ```
///
/// Lines are sorted lexicographically for deterministic output. Empty callers
/// are kept as a leading `;` (i.e. `;called count`).
pub fn render_folded_stacks(model: &ProfileModel) -> String {
    folded_from_stacks(&collect_nonzero_call_edges(model))
}

/// Non-zero `call_edges` in folded-stack order (lexicographic caller, called).
fn collect_nonzero_call_edges(model: &ProfileModel) -> Vec<((&str, &str), u64)> {
    let mut rows: Vec<((&str, &str), u64)> = model
        .call_edges
        .iter()
        .filter(|(_, e)| e.count > 0)
        .map(|((caller, called), e)| ((caller.as_str(), called.as_str()), e.count))
        .collect();
    rows.sort_unstable_by(|((c1, d1), _), ((c2, d2), _)| c1.cmp(c2).then_with(|| d1.cmp(d2)));
    rows
}

fn folded_from_stacks(stacks: &[((&str, &str), u64)]) -> String {
    let mut out = String::with_capacity(stacks.len().saturating_mul(48));
    for ((caller, called), count) in stacks {
        out.push_str(caller);
        out.push(';');
        out.push_str(called);
        out.push(' ');
        let _ = write!(out, "{count}");
        out.push('\n');
    }
    out
}

/// Native flame SVG from the `call_edges` tree (optional HTML path).
///
/// **Not** oracle `flamegraph.pl` / multi-frame `nytprofcalls` / a sampled
/// timeline. Roots sit at the **bottom**; callees stack upward. Width is
/// inclusive time when `sub_return_totals` has it, otherwise call count.
/// Same caller is **one** frame (not a column per edge). Frames narrower
/// than [`FLAME_MIN_PAINT_PX`] are omitted.
///
/// Published sibling SVG uses multi-file `file-{fid}.html#L{line}` hrefs
/// (same directory as the site). See `docs/schemas/html-optional-flame-mvp-v0.md`.
pub fn render_flame_svg(model: &ProfileModel) -> String {
    flame_svg_from_model(model, true)
}

/// Minimum painted frame width (CSS px in the 1200-wide viewBox).
const FLAME_MIN_PAINT_PX: f64 = 1.0;
const FLAME_MIN_LABEL_PX: f64 = 48.0;
const FLAME_SVG_W: f64 = 1200.0;
const FLAME_ROW_H: f64 = 22.0;
const FLAME_PAD: f64 = 2.0;
const FLAME_MAX_DEPTH: usize = 16;

fn flame_node_incl(model: &ProfileModel, name: &str) -> f64 {
    match model.sub_return_totals.get(name) {
        Some(t) if t.incl.is_finite() && t.incl > 0.0 => t.incl,
        _ => 0.0,
    }
}

#[derive(Clone, Copy)]
struct FlameKid<'a> {
    name: &'a str,
    count: u64,
    weight: f64,
}

fn flame_children<'a>(model: &'a ProfileModel) -> BTreeMap<&'a str, Vec<FlameKid<'a>>> {
    let mut kids: BTreeMap<&str, Vec<FlameKid<'_>>> = BTreeMap::new();
    for ((caller, called), e) in &model.call_edges {
        if e.count == 0 || called.is_empty() || caller == called {
            continue;
        }
        let weight = {
            let incl = flame_node_incl(model, called);
            if incl > 0.0 {
                incl
            } else if e.incl.is_finite() && e.incl > 0.0 {
                e.incl
            } else {
                e.count as f64
            }
        };
        kids.entry(caller.as_str()).or_default().push(FlameKid {
            name: called.as_str(),
            count: e.count,
            weight,
        });
    }
    for v in kids.values_mut() {
        v.sort_unstable_by(|a, b| b.weight.total_cmp(&a.weight).then_with(|| a.name.cmp(b.name)));
    }
    kids
}

fn flame_svg_from_model(model: &ProfileModel, multi_file: bool) -> String {
    let kids = flame_children(model);
    let called: BTreeSet<&str> = kids
        .values()
        .flat_map(|v| v.iter().map(|k| k.name))
        .collect();
    let mut roots: Vec<&str> = kids
        .keys()
        .copied()
        .filter(|c| !c.is_empty() && !called.contains(c))
        .collect();
    if roots.is_empty() {
        roots = kids.keys().copied().filter(|c| !c.is_empty()).collect();
    }
    roots.sort_unstable();

    let mut root_nodes: Vec<(&str, u64, f64)> = roots
        .into_iter()
        .map(|name| {
            let child_sum: f64 = kids.get(name).map(|v| v.iter().map(|k| k.weight).sum()).unwrap_or(0.0);
            let incl = flame_node_incl(model, name);
            let weight = incl.max(child_sum).max(1.0);
            let count = kids.get(name).map(|v| v.iter().map(|k| k.count).sum()).unwrap_or(0);
            (name, count, weight)
        })
        .collect();
    let total_w: f64 = root_nodes.iter().map(|(_, _, w)| *w).sum();
    root_nodes.sort_unstable_by(|a, b| b.2.total_cmp(&a.2).then_with(|| a.0.cmp(b.0)));

    let mut max_depth = 1usize;
    let mut seen_depth = BTreeSet::new();
    for (name, _, _) in &root_nodes {
        seen_depth.clear();
        max_depth = max_depth.max(flame_depth(name, &kids, &mut seen_depth, 0));
    }
    max_depth = max_depth.min(FLAME_MAX_DEPTH);
    let svg_h = FLAME_PAD * 2.0 + (max_depth as f64) * FLAME_ROW_H;

    let mut out = String::with_capacity(kids.len().saturating_mul(200).max(512));
    let _ = write!(
        out,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" class=\"nytprof-flame\" role=\"img\" aria-label=\"NYTProf call-tree flame\">\n\
<title>NYTProf call-tree flame (native; not flamegraph.pl)</title>\n\
<desc>Roots at the bottom; callees stack up. Width is inclusive time when known, else calls. Hover a frame for details; click to open source.</desc>\n\
<style type=\"text/css\"><![CDATA[\n\
.flame-link{{cursor:pointer}}\n\
.flame-frame rect{{transition:filter .12s ease-out}}\n\
.flame-link:hover rect,.flame-frame:hover rect{{filter:brightness(1.06) saturate(1.2);stroke:#111;stroke-width:1.2px}}\n\
]]></style>\n",
        w = FLAME_SVG_W as u32,
        h = svg_h as u32,
    );

    if root_nodes.is_empty() || total_w <= 0.0 {
        out.push_str(
            "<text x=\"8\" y=\"16\" font-family=\"system-ui,sans-serif\" font-size=\"12\" fill=\"#444\">No call edges for flame</text>\n</svg>\n",
        );
        return out;
    }

    let mut x = 0.0_f64;
    let mut ctx = FlamePaint {
        out: &mut out,
        kids: &kids,
        model,
        multi_file,
        tps: ticks_per_sec_attr(model),
        max_depth,
        svg_h,
    };
    for (name, count, weight) in &root_nodes {
        let width = (*weight / total_w) * FLAME_SVG_W;
        let mut path = BTreeSet::new();
        paint_flame_node(
            &mut ctx,
            FlameFrame {
                name,
                count: *count,
                weight: *weight,
                x,
                width,
                depth: 0,
            },
            &mut path,
        );
        x += width;
    }
    out.push_str("</svg>\n");
    out
}

fn flame_depth(
    name: &str,
    kids: &BTreeMap<&str, Vec<FlameKid<'_>>>,
    path: &mut BTreeSet<String>,
    depth: usize,
) -> usize {
    if depth >= FLAME_MAX_DEPTH || !path.insert(name.to_owned()) {
        return depth + 1;
    }
    let mut d = depth + 1;
    if let Some(cs) = kids.get(name) {
        for k in cs {
            d = d.max(flame_depth(k.name, kids, path, depth + 1));
        }
    }
    path.remove(name);
    d
}

struct FlamePaint<'a, 'b> {
    out: &'a mut String,
    kids: &'b BTreeMap<&'b str, Vec<FlameKid<'b>>>,
    model: &'b ProfileModel,
    multi_file: bool,
    tps: Option<u64>,
    max_depth: usize,
    svg_h: f64,
}

struct FlameFrame<'a> {
    name: &'a str,
    count: u64,
    weight: f64,
    x: f64,
    width: f64,
    depth: usize,
}

fn paint_flame_node(ctx: &mut FlamePaint<'_, '_>, frame: FlameFrame<'_>, path: &mut BTreeSet<String>) {
    if frame.width < FLAME_MIN_PAINT_PX || frame.depth >= ctx.max_depth {
        return;
    }
    // Classic flame: depth 0 at the bottom, children grow upward.
    let y = ctx.svg_h - FLAME_PAD - ((frame.depth + 1) as f64) * FLAME_ROW_H;
    let (calls, mut incl, excl) = flame_frame_totals(ctx.model, frame.name, frame.count);
    if incl <= 0.0 && frame.weight.is_finite() && frame.weight > 0.0 {
        incl = frame.weight;
    }
    let incl_l = format_time_cell(incl, ctx.tps);
    let excl_l = format_time_cell(excl, ctx.tps);
    let href = flame_frame_href(ctx.model, frame.name, ctx.multi_file);
    push_flame_rect(
        ctx.out,
        FlameRect {
            x: frame.x,
            y,
            width: frame.width,
            name: frame.name,
            count: calls,
            incl_label: &incl_l,
            excl_label: &excl_l,
            href: href.as_deref(),
        },
    );
    if !path.insert(frame.name.to_owned()) {
        return;
    }
    let Some(cs) = ctx.kids.get(frame.name) else {
        path.remove(frame.name);
        return;
    };
    let child_sum: f64 = cs.iter().map(|k| k.weight).sum();
    if child_sum <= 0.0 {
        path.remove(frame.name);
        return;
    }
    // Scale children into the parent bar (leftover width is parent self-time).
    let fit = frame
        .width
        .min(frame.width * (child_sum / frame.weight.max(child_sum)));
    let mut cx = frame.x;
    let next_depth = frame.depth + 1;
    let kids: Vec<FlameKid<'_>> = cs.clone();
    for k in kids {
        let cw = (k.weight / child_sum) * fit;
        paint_flame_node(
            ctx,
            FlameFrame {
                name: k.name,
                count: k.count,
                weight: k.weight,
                x: cx,
                width: cw,
                depth: next_depth,
            },
            path,
        );
        cx += cw;
    }
    path.remove(frame.name);
}

fn flame_frame_totals(model: &ProfileModel, name: &str, fallback_count: u64) -> (u64, f64, f64) {
    match model.sub_return_totals.get(name) {
        Some(t) => {
            let calls = if t.returns > 0 {
                t.returns
            } else {
                fallback_count
            };
            let incl = if t.incl.is_finite() && t.incl > 0.0 {
                t.incl
            } else {
                0.0
            };
            let excl = if t.excl.is_finite() && t.excl > 0.0 {
                t.excl
            } else {
                0.0
            };
            (calls, incl, excl)
        }
        None => (fallback_count, 0.0, 0.0),
    }
}

fn flame_frame_href(model: &ProfileModel, name: &str, multi_file: bool) -> Option<String> {
    if let Some(h) = sub_href(model, name, multi_file) {
        return Some(h);
    }
    if name == "RUNTIME" || name.ends_with("::RUNTIME") {
        let fid = primary_workload_fid(model);
        return Some(if multi_file {
            format!("file-{fid}.html")
        } else {
            "#L1".to_owned()
        });
    }
    None
}

struct FlameRect<'a> {
    x: f64,
    y: f64,
    width: f64,
    name: &'a str,
    count: u64,
    incl_label: &'a str,
    excl_label: &'a str,
    href: Option<&'a str>,
}

fn push_flame_rect(out: &mut String, r: FlameRect<'_>) {
    let ix = r.x + 0.5;
    let iy = r.y + 0.5;
    let iw = (r.width - 1.0).max(0.5);
    let ih = FLAME_ROW_H - 1.0;
    let fill = flame_fill_color(r.name);
    let linked = r.href.filter(|h| !h.is_empty());
    if let Some(h) = linked {
        out.push_str("<a class=\"flame-link\" href=\"");
        escape_xml_into(out, h);
        out.push_str("\" xlink:href=\"");
        escape_xml_into(out, h);
        out.push_str("\">");
    }
    out.push_str("<g class=\"flame-frame\"><title>");
    escape_xml_into(out, r.name);
    let _ = write!(out, "\ncalls: {}\ninclusive: ", r.count);
    escape_xml_into(out, r.incl_label);
    out.push_str("\nexclusive: ");
    escape_xml_into(out, r.excl_label);
    if linked.is_some() {
        out.push_str("\nclick to open source");
    }
    out.push_str("</title>");
    let _ = write!(
        out,
        "<rect x=\"{ix:.2}\" y=\"{iy:.2}\" width=\"{iw:.2}\" height=\"{ih:.2}\" rx=\"3\" fill=\"{fill}\" stroke=\"rgba(255,255,255,.45)\" stroke-width=\"1\"/>"
    );
    if r.width >= FLAME_MIN_LABEL_PX {
        let label = if r.name.is_empty() { "(anon)" } else { r.name };
        let tx = r.x + 5.0;
        let ty = r.y + FLAME_ROW_H * 0.68;
        let _ = write!(
            out,
            "<text x=\"{tx:.2}\" y=\"{ty:.2}\" font-family=\"ui-monospace,monospace\" font-size=\"11\" fill=\"#111\" pointer-events=\"none\">"
        );
        escape_xml_into(out, label);
        out.push_str("</text>");
    }
    out.push_str("</g>");
    if linked.is_some() {
        out.push_str("</a>");
    }
    out.push('\n');
}

/// Deterministic pastel fill from subroutine name (not a visual-oracle palette).
fn flame_fill_color(name: &str) -> &'static str {
    let mut h: u32 = 2166136261;
    for b in name.as_bytes() {
        h ^= u32::from(*b);
        h = h.wrapping_mul(16777619);
    }
    const PALETTE: [&str; 12] = [
        "#f4a3a3", "#f4c3a3", "#f4e3a3", "#d4f4a3", "#a3f4b3", "#a3f4e3", "#a3d4f4", "#a3b3f4",
        "#c3a3f4", "#e3a3f4", "#f4a3d4", "#f4a3c3",
    ];
    PALETTE[((h % 360) / 30) as usize]
}


fn escape_xml_into(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
}


/// Multi-file index: sibling flame files plus an **inlined** SVG so hover and
/// click-to-source work under `file://` (`<img>` / `<object>` cannot).
fn push_flame_section_links(
    out: &mut String,
    folded_filename: &str,
    svg_filename: &str,
    svg_body: &str,
) {
    out.push_str("<section class=\"flame\">\n");
    out.push_str("<h2>Flame graph</h2>\n");
    out.push_str(
        "<p class=\"flame-note\">Hover a frame for inclusive/exclusive time. Click a frame to open its source. (Native call-tree; not oracle flamegraph.pl.)</p>\n",
    );
    out.push_str(&format!(
        "<p class=\"flame-links\"><a href=\"{svg}\">{svg}</a> · <a href=\"{folded}\">{folded}</a></p>\n",
        svg = escape_html(svg_filename),
        folded = escape_html(folded_filename),
    ));
    push_inlined_flame_svg(out, svg_body);
    out.push_str("</section>\n");
}

/// Single-file: embed native flame SVG inline (self-contained document).
fn push_flame_section_embedded(out: &mut String, model: &ProfileModel) {
    out.push_str("<section class=\"flame\">\n");
    out.push_str("<h2>Flame graph</h2>\n");
    out.push_str(
        "<p class=\"flame-note\">Hover a frame for inclusive/exclusive time. Click a frame to open its source. (Native call-tree; not oracle flamegraph.pl.)</p>\n",
    );
    let svg = flame_svg_from_model(model, false);
    push_inlined_flame_svg(out, &svg);
    out.push_str("</section>\n");
}

fn push_inlined_flame_svg(out: &mut String, svg_body: &str) {
    out.push_str("<div class=\"flame-svg-embed\">\n");
    let embed = svg_body
        .strip_prefix("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")
        .unwrap_or(svg_body);
    out.push_str(embed);
    if !embed.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("</div>\n");
    out.push_str("<div id=\"nytprof-flame-tip\" hidden></div>\n");
    // Vanilla; no jquery. Reads SVG <title> so the same details show immediately.
    out.push_str(
        "<script>\n\
(function(){\n\
var svg=document.querySelector(\".flame-svg-embed svg\");\n\
var tip=document.getElementById(\"nytprof-flame-tip\");\n\
if(!svg||!tip)return;\n\
function hide(){tip.style.display=\"none\";tip.hidden=true;}\n\
svg.addEventListener(\"mouseover\",function(e){\n\
var g=e.target.closest(\".flame-frame\");\n\
if(!g)return;\n\
var t=g.querySelector(\"title\");\n\
if(!t)return;\n\
tip.textContent=t.textContent;\n\
tip.hidden=false;\n\
tip.style.display=\"block\";\n\
});\n\
svg.addEventListener(\"mousemove\",function(e){\n\
if(tip.hidden)return;\n\
var x=e.clientX+12,y=e.clientY+12;\n\
if(x+tip.offsetWidth>window.innerWidth-8)x=e.clientX-tip.offsetWidth-12;\n\
if(y+tip.offsetHeight>window.innerHeight-8)y=e.clientY-tip.offsetHeight-12;\n\
tip.style.left=x+\"px\";tip.style.top=y+\"px\";\n\
});\n\
svg.addEventListener(\"mouseout\",function(e){\n\
if(!e.relatedTarget||!svg.contains(e.relatedTarget))hide();\n\
});\n\
})();\n\
</script>\n",
    );
}

/// Callgrind-inspired text export (export-formats MVP v0).
///
/// Not byte-identical to legacy `nytprofcg` / Valgrind Callgrind; provides a
/// minimal structure tools can grep or parse loosely:
///
/// - header: `# callgrind format`, `positions: line`, `events: Ticks`
/// - per function: `fn=name` and a self-cost line (`0 <cost>`)
/// - per outgoing edge with count &gt; 0: `cfn=called`, `calls={count} 0`, cost line
///
/// Functions are the sorted union of `sub_return_totals` names and call-edge
/// endpoints. Self cost prefers exclusive ticks (integer when exact), else
/// `returns`. Edge cost prefers edge exclusive ticks, else `count`.
pub fn render_callgrind(model: &ProfileModel) -> String {
    // Union of all names that appear as subs or edge endpoints.
    let mut names: BTreeSet<&str> = BTreeSet::new();
    for name in model.sub_return_totals.keys() {
        names.insert(name.as_str());
    }
    for ((caller, called), e) in &model.call_edges {
        if e.count == 0 {
            continue;
        }
        names.insert(caller.as_str());
        names.insert(called.as_str());
    }

    // Outgoing edges grouped later by linear scan of sorted edge list.
    let mut edges: Vec<((&str, &str), &nytprof_model::CallEdgeTotal)> = model
        .call_edges
        .iter()
        .filter(|(_, e)| e.count > 0)
        .map(|((c, d), e)| ((c.as_str(), d.as_str()), e))
        .collect();
    edges.sort_by(|((c1, d1), _), ((c2, d2), _)| c1.cmp(c2).then_with(|| d1.cmp(d2)));

    let mut out =
        String::with_capacity(names.len().saturating_mul(64) + edges.len().saturating_mul(48));
    out.push_str("# callgrind format\n");
    out.push_str("positions: line\n");
    out.push_str("events: Ticks\n");

    for name in names {
        out.push('\n');
        out.push_str("fn=");
        out.push_str(name);
        out.push('\n');

        let self_cost = match model.sub_return_totals.get(name) {
            Some(t) => callgrind_cost_from_f64(t.excl, t.returns),
            None => 0,
        };
        out.push_str("0 ");
        out.push_str(&self_cost.to_string());
        out.push('\n');

        // Outgoing call edges from this function (already sorted by called within caller).
        for ((caller, called), e) in &edges {
            if *caller != name {
                continue;
            }
            out.push_str("cfn=");
            out.push_str(called);
            out.push('\n');
            out.push_str("calls=");
            out.push_str(&e.count.to_string());
            out.push_str(" 0\n");
            let edge_cost = callgrind_cost_from_f64(e.excl, e.count);
            out.push_str("0 ");
            out.push_str(&edge_cost.to_string());
            out.push('\n');
        }
    }

    out
}

/// Integer event cost for Callgrind lines: exact non-negative integral f64, else fallback.
fn callgrind_cost_from_f64(v: f64, fallback: u64) -> u64 {
    if v.is_finite() && v >= 0.0 && v.fract() == 0.0 && v <= (u64::MAX as f64) {
        v as u64
    } else if fallback > 0 {
        fallback
    } else if v.is_finite() && v > 0.0 {
        // Non-integral exclusive time: still surface a positive cost when possible.
        v.round().clamp(0.0, u64::MAX as f64) as u64
    } else {
        0
    }
}

fn path_basename(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(path)
}

fn is_inc_ish_path(path: &str) -> bool {
    path.contains("/lib/perl")
        || path.contains("site_perl")
        || path.contains("vendor_perl")
        || path.contains("/usr/share/perl")
}

fn fid_exclusive_ticks(model: &ProfileModel, fid: u32) -> i64 {
    model
        .line_totals
        .iter()
        .filter(|((f, _), _)| *f == fid)
        .map(|(_, t)| t.ticks)
        .sum()
}

fn fid_stmt_calls(model: &ProfileModel, fid: u32) -> u64 {
    model
        .line_totals
        .iter()
        .filter(|((f, _), _)| *f == fid)
        .map(|(_, t)| t.calls)
        .sum()
}

fn application_basename(model: &ProfileModel) -> String {
    if let Some(app) = model.attributes.get("application") {
        if !app.is_empty() {
            return path_basename(app).to_owned();
        }
    }
    let fid = primary_workload_fid(model);
    model
        .fid_basename(fid)
        .unwrap_or("profile")
        .to_owned()
}

/// Choose the primary source file id for `source.html` (KD-PRIMARY).
///
/// 1. `attributes["application"]` basename matching a `files` entry
/// 2. non-@INC `.pl` with the most exclusive time
/// 3. existing `"workload"` / `workload.pl` heuristic
/// 4. min fid with source_lines / eligible data, else 1
fn primary_workload_fid(model: &ProfileModel) -> u32 {
    if let Some(app) = model.attributes.get("application") {
        if !app.is_empty() {
            let base = path_basename(app);
            let mut matches: Vec<u32> = model
                .files
                .iter()
                .filter(|(_, name)| path_basename(name) == base || name.as_str() == app.as_str())
                .map(|(fid, _)| *fid)
                .collect();
            if !matches.is_empty() {
                matches.sort_unstable();
                return matches[0];
            }
        }
    }

    let mut best: Option<(i64, u32)> = None;
    for (fid, name) in &model.files {
        let base = path_basename(name);
        if !base.ends_with(".pl") || is_inc_ish_path(name) {
            continue;
        }
        let ticks = fid_exclusive_ticks(model, *fid);
        match best {
            None => best = Some((ticks, *fid)),
            Some((bt, bf)) if ticks > bt || (ticks == bt && *fid < bf) => {
                best = Some((ticks, *fid));
            }
            _ => {}
        }
    }
    if let Some((_, fid)) = best {
        return fid;
    }

    let mut workload_fids: Vec<u32> = model
        .files
        .iter()
        .filter(|(_, name)| {
            let base = path_basename(name);
            name.contains("workload") || base == "workload.pl"
        })
        .map(|(fid, _)| *fid)
        .collect();
    if !workload_fids.is_empty() {
        workload_fids.sort_unstable();
        return workload_fids[0];
    }

    let mut source_fids: Vec<u32> = model.source_lines.keys().map(|(fid, _)| *fid).collect();
    source_fids.sort_unstable();
    source_fids.dedup();
    if let Some(&fid) = source_fids.first() {
        return fid;
    }
    eligible_source_fids(model)
        .first()
        .copied()
        .unwrap_or(1)
}

/// Parse `attributes["ticks_per_sec"]` as a positive unsigned integer.
fn ticks_per_sec_attr(model: &ProfileModel) -> Option<u64> {
    model
        .attributes
        .get("ticks_per_sec")
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&n| n > 0)
}

/// HTML-only time-cell display (seconds when `ticks_per_sec` is Some(n>0)).
///
/// Text / CSV / `report --json` stay on [`format_ticks`] integer ticks.
fn format_time_cell(ticks: f64, ticks_per_sec: Option<u64>) -> String {
    match ticks_per_sec {
        Some(tps) if tps > 0 => {
            let scaled = ticks / (tps as f64);
            // default-calls1 SUB_CALLERS NVs are already seconds-scale (OI-003-02);
            // dividing again would collapse them to 0s.
            let secs = if ticks.abs() > 0.0 && ticks.abs() < 1.0 && scaled.abs() < 1e-9 {
                ticks
            } else {
                scaled
            };
            format_compact_secs(secs)
        }
        _ => format_ticks(ticks),
    }
}

fn int_digit_len(v: f64) -> usize {
    let n = v.trunc().abs() as u64;
    if n == 0 { 1 } else { n.to_string().len() }
}

/// 6.15 `Util.pm` `fmt_time` (empty width) — HTML display only.
fn format_compact_secs(secs: f64) -> String {
    if !secs.is_finite() {
        return format_ticks(secs);
    }
    if secs < 0.0 {
        return format!("-{}", format_compact_secs(-secs));
    }
    if secs == 0.0 {
        return "0s".to_owned();
    }
    if secs < 1e-6 {
        return format!("{:.0}ns", secs * 1e9);
    }
    if secs < 1e-3 {
        return format!("{:.0}µs", secs * 1e6);
    }
    if secs < 1.0 {
        let val = secs * 1e3;
        let prec = 3usize.saturating_sub(int_digit_len(val));
        return format!("{val:.prec$}ms");
    }
    if secs < 100.0 {
        let prec = 3usize.saturating_sub(int_digit_len(secs));
        return format!("{secs:.prec$}s");
    }
    format!("{secs:.0}s")
}

/// Quartile thresholds for heat classes within one table.
struct HeatScale {
    q1: f64,
    q2: f64,
    q3: f64,
    spread: bool,
}

impl HeatScale {
    fn from_values(values: &[f64]) -> Self {
        let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
        if v.is_empty() {
            return Self {
                q1: 0.0,
                q2: 0.0,
                q3: 0.0,
                spread: false,
            };
        }
        v.sort_by(|a, b| a.total_cmp(b));
        let n = v.len();
        let q_at = |p: f64| -> f64 {
            if n == 1 {
                return v[0];
            }
            let idx = p * (n - 1) as f64;
            let lo = idx.floor() as usize;
            let hi = idx.ceil() as usize;
            let frac = idx - lo as f64;
            if lo == hi {
                v[lo]
            } else {
                v[lo].mul_add(1.0 - frac, v[hi] * frac)
            }
        };
        let min = v[0];
        let max = v[n - 1];
        Self {
            q1: q_at(0.25),
            q2: q_at(0.50),
            q3: q_at(0.75),
            spread: max > min,
        }
    }
}

/// Quartile rank class: `heat-hot` (highest) … `heat-low` (lowest).
///
/// Zero values and tables with no spread get **no** class (unused source rows
/// stay uncolored).
fn heat_class(value: f64, scale: &HeatScale) -> &'static str {
    if value == 0.0 || !scale.spread {
        return "";
    }
    if value >= scale.q3 {
        "heat-hot"
    } else if value >= scale.q2 {
        "heat-high"
    } else if value >= scale.q1 {
        "heat-mid"
    } else {
        "heat-low"
    }
}

/// Sub → source href from `model.sub_def(name)` (never a hard-coded line).
///
/// Multi-file: `file-{fid}.html#L{first_line}`. Single-file: `#L{first_line}`.
/// CORE: / xsub names without a def link to an opcode stub on the application
/// file (`#main__CORE_match`).
fn sub_href(model: &ProfileModel, name: &str, multi_file: bool) -> Option<String> {
    if let Some(d) = model.sub_def(name) {
        if d.first_line > 0 {
            return if multi_file {
                Some(format!("file-{}.html#L{}", d.fid, d.first_line))
            } else {
                Some(format!("#L{}", d.first_line))
            };
        }
    }
    if name.contains("CORE:") {
        let frag = opcode_fragment_id(name);
        return if multi_file {
            let fid = primary_workload_fid(model);
            Some(format!("file-{}.html#{frag}", fid))
        } else {
            Some(format!("#{frag}"))
        };
    }
    None
}

fn opcode_fragment_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn call_site_usable(model: &ProfileModel, caller: &str, fid: u32, line: u32) -> bool {
    if fid == 0 || line == 0 {
        return false;
    }
    if fid == 1 && line == 1 {
        return match model.sub_def(caller) {
            Some(d) => d.fid == 1 && d.first_line == 1,
            None => false,
        };
    }
    true
}

#[derive(Default)]
struct LineCallAgg {
    out_count: u64,
    out_incl: f64,
    outs: Vec<(String, u64, f64)>,
    ins: Vec<(String, u32, u32, u64)>,
}

fn line_call_aggs(model: &ProfileModel) -> BTreeMap<(u32, u32), LineCallAgg> {
    let mut map: BTreeMap<(u32, u32), LineCallAgg> = BTreeMap::new();
    for ((caller, called, fid, line), site) in &model.call_sites {
        if !call_site_usable(model, caller, *fid, *line) {
            continue;
        }
        let out = map.entry((*fid, *line)).or_default();
        out.out_count = out.out_count.saturating_add(site.count);
        out.out_incl += site.incl;
        out.outs.push((called.clone(), site.count, site.incl));
        let dest = model.sub_def(called).map(|d| (d.fid, d.first_line));
        if let Some((dfid, dline)) = dest {
            if dline > 0 {
                let incoming = map.entry((dfid, dline)).or_default();
                incoming
                    .ins
                    .push((caller.clone(), *fid, *line, site.count));
            }
        }
    }
    for agg in map.values_mut() {
        agg.outs.sort_by(|a, b| a.0.cmp(&b.0));
        agg.ins.sort_by(|a, b| a.0.cmp(&b.0).then(a.2.cmp(&b.2)));
    }
    map
}

fn stub_call_ins(model: &ProfileModel) -> BTreeMap<String, Vec<(String, u32, u32, u64)>> {
    let mut map: BTreeMap<String, Vec<(String, u32, u32, u64)>> = BTreeMap::new();
    for ((caller, called, fid, line), site) in &model.call_sites {
        if !called.contains("CORE:") || model.sub_defs.contains_key(called) {
            continue;
        }
        if !call_site_usable(model, caller, *fid, *line) {
            continue;
        }
        map.entry(called.clone())
            .or_default()
            .push((caller.clone(), *fid, *line, site.count));
    }
    for v in map.values_mut() {
        v.sort_by(|a, b| a.0.cmp(&b.0));
    }
    map
}

fn href_file_line(fid: u32, line: u32, multi_file: bool) -> String {
    if multi_file {
        format!("file-{fid}.html#L{line}")
    } else {
        format!("#L{line}")
    }
}

fn opcode_stub_names(model: &ProfileModel) -> Vec<String> {
    let mut names = BTreeSet::new();
    for (caller, called) in model.call_edges.keys() {
        for n in [caller, called] {
            if n.contains("CORE:") && !model.sub_defs.contains_key(n) {
                names.insert(n.clone());
            }
        }
    }
    names.into_iter().collect()
}

fn push_th(out: &mut String, label: &str, sort: &str) {
    push_th_inner(out, &escape_html(label), sort, None);
}

fn push_th_html(out: &mut String, label_html: &str, sort: &str) {
    push_th_inner(out, label_html, sort, None);
}

/// Header that may contain unescaped `<br>` plus `data-sort-default`.
fn push_th_default(out: &mut String, label: &str, sort: &str, default_dir: &str) {
    push_th_inner(out, label, sort, Some(default_dir));
}

fn push_th_inner(out: &mut String, label_html: &str, sort: &str, default_dir: Option<&str>) {
    out.push_str("<th data-sort=\"");
    out.push_str(&escape_html(sort));
    out.push('"');
    if let Some(dir) = default_dir {
        out.push_str(" data-sort-default=\"");
        out.push_str(&escape_html(dir));
        out.push('"');
    }
    out.push('>');
    out.push_str(label_html);
    out.push_str("</th>");
}

fn push_count_td(out: &mut String, n: u64) {
    push_count_td_heat(out, n, "");
}

fn push_count_td_heat(out: &mut String, n: u64, heat: &str) {
    let s = n.to_string();
    out.push_str("<td class=\"num");
    if !heat.is_empty() {
        out.push(' ');
        out.push_str(heat);
    }
    out.push_str("\" data-sort-value=\"");
    out.push_str(&s);
    out.push_str("\">");
    out.push_str(&s);
    out.push_str("</td>");
}

fn push_time_td(out: &mut String, ticks: f64, ticks_per_sec: Option<u64>) {
    push_time_td_heat(out, ticks, ticks_per_sec, "");
}

fn push_time_td_heat(out: &mut String, ticks: f64, ticks_per_sec: Option<u64>, heat: &str) {
    let display = format_time_cell(ticks, ticks_per_sec);
    let raw = format_ticks(ticks);
    let title = match ticks_per_sec {
        Some(n) if n > 0 => format!("{raw} ticks"),
        _ => raw.clone(),
    };
    out.push_str("<td class=\"num");
    if !heat.is_empty() {
        out.push(' ');
        out.push_str(heat);
    }
    out.push_str("\" data-sort-value=\"");
    out.push_str(&escape_html(&raw));
    out.push_str("\" title=\"");
    out.push_str(&escape_html(&title));
    out.push_str("\">");
    out.push_str(&escape_html(&display));
    out.push_str("</td>");
}

fn push_placeholder_num_td(out: &mut String) {
    out.push_str("<td class=\"num\" data-sort-value=\"\">—</td>");
}

fn push_call_site_link(
    out: &mut String,
    model: &ProfileModel,
    name: &str,
    fid: u32,
    line: u32,
    multi_file: bool,
) {
    let href = if line > 0 {
        href_file_line(fid, line, multi_file)
    } else {
        sub_href(model, name, multi_file).unwrap_or_else(|| "#".to_owned())
    };
    out.push_str("<a href=\"");
    out.push_str(&escape_html(&href));
    out.push_str("\">");
    out.push_str(&escape_html(name));
    out.push_str("</a>");
}

fn push_call_annotations(
    out: &mut String,
    model: &ProfileModel,
    agg: &LineCallAgg,
    multi_file: bool,
) {
    if !agg.ins.is_empty() {
        for (caller, cfid, cline, count) in &agg.ins {
            out.push_str("<div class=\"calls calls_in\"># spent ");
            out.push_str(&count.to_string());
            out.push_str(" times called from ");
            push_call_site_link(out, model, caller, *cfid, *cline, multi_file);
            out.push_str("</div>");
        }
    }
    if !agg.outs.is_empty() {
        for (called, count, _incl) in &agg.outs {
            out.push_str("<div class=\"calls calls_out\">");
            out.push_str(&count.to_string());
            out.push_str(" calls to ");
            if let Some(href) = sub_href(model, called, multi_file) {
                out.push_str("<a href=\"");
                out.push_str(&escape_html(&href));
                out.push_str("\">");
                out.push_str(&escape_html(called));
                out.push_str("</a>");
            } else {
                out.push_str(&escape_html(called));
            }
            out.push_str("</div>");
        }
    }
}

fn push_sub_name_cell(out: &mut String, model: &ProfileModel, name: &str, multi_file: bool) {
    out.push_str("<td class=\"sub_name\">");
    let escaped = escape_html(name);
    if let Some(href) = sub_href(model, name, multi_file) {
        out.push_str("<a href=\"");
        out.push_str(&escape_html(&href));
        out.push_str("\">");
        out.push_str(&escaped);
        out.push_str("</a>");
    } else {
        out.push_str(&escaped);
    }
    if name.contains("CORE:") {
        out.push_str(" <span class=\"hint\">(opcode)</span>");
    }
    out.push_str("</td>");
}

/// Format floating tick / time sums without noisy trailing zeros when integral.
fn format_ticks(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < (i64::MAX as f64) {
        format!("{}", v as i64)
    } else {
        // Keep enough precision for small wall-time NVs from SUB_CALLERS.
        format!("{v}")
    }
}

/// RFC 4180 field escape: quote when the value contains comma, quote, or CR/LF.
fn csv_escape(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        let mut s = String::with_capacity(field.len() + 2);
        s.push('"');
        for ch in field.chars() {
            if ch == '"' {
                s.push('"');
            }
            s.push(ch);
        }
        s.push('"');
        s
    } else {
        field.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nytprof_model::ProfileModel;
    use std::path::PathBuf;

    fn fixture_out(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/v5")
            .join(name)
            .join("nytprof.out")
    }

    #[test]
    fn summary_default_calls1_real_render_path() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();

        let model = ProfileModel::from_path(&path).expect("build model");
        let text = render_summary_text(&model, &path_str);

        assert!(!text.is_empty());
        assert!(
            text.contains("main::leaf"),
            "report must list main::leaf:\n{text}"
        );
        assert!(
            text.contains("main::mid"),
            "report must list main::mid:\n{text}"
        );
        // Return counts from real aggregation (mid → leaf × 5 × 3 = 15 leaf, 3 mid).
        assert!(
            text.contains("main::leaf  returns=15"),
            "leaf returns=15:\n{text}"
        );
        assert!(
            text.contains("main::mid  returns=3"),
            "mid returns=3:\n{text}"
        );

        let expected_line = format!("time_line_events: {}", model.time_line_events);
        assert!(
            text.contains(&expected_line),
            "time_line_events must match model ({expected_line}):\n{text}"
        );
        assert!(
            text.contains(&format!("discount_events: {}", model.discount_events)),
            "discount_events must match model"
        );
        assert!(text.contains("Top lines (by calls)"));
        assert!(text.contains("Subroutines (from SUB_RETURN)"));
        assert!(text.contains("workload.pl"));
    }

    #[test]
    fn subs_csv_default_calls1_real_render() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let model = ProfileModel::from_path(&path).expect("build model");
        let csv = render_subs_csv(&model);

        assert!(
            csv.starts_with("name,returns,incl,excl\n"),
            "header:\n{csv}"
        );
        // Exact row prefixes from real model (returns from A5).
        assert!(
            csv.contains("main::leaf,15,"),
            "leaf row with returns=15:\n{csv}"
        );
        assert!(
            csv.contains("main::mid,3,"),
            "mid row with returns=3:\n{csv}"
        );
        // Sorted by name — leaf before mid.
        let leaf_pos = csv.find("main::leaf,15,").expect("leaf");
        let mid_pos = csv.find("main::mid,3,").expect("mid");
        assert!(leaf_pos < mid_pos, "subs sorted by name");
    }

    #[test]
    fn edges_csv_default_calls1_real_render() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let model = ProfileModel::from_path(&path).expect("build model");
        let csv = render_edges_csv(&model);

        assert!(
            csv.starts_with("caller,called,count,incl,excl\n"),
            "header:\n{csv}"
        );
        // mid → leaf count 15 from real call_edges (A7 / SUB_CALLERS).
        assert!(
            csv.contains("main::mid,main::leaf,15,"),
            "mid→leaf count 15:\n{csv}"
        );
        // RUNTIME → mid count 3 (schema example).
        assert!(
            csv.contains("main::RUNTIME,main::mid,3,"),
            "RUNTIME→mid count 3:\n{csv}"
        );

        let edge = model
            .call_edge("main::mid", "main::leaf")
            .expect("model must have mid→leaf");
        assert_eq!(edge.count, 15);
    }

    #[test]
    fn csv_report_dual_section() {
        let path = fixture_out("default-calls1");
        let model = ProfileModel::from_path(&path).expect("build model");
        let text = render_csv_report(&model);
        assert!(text.contains("# subroutines\n"));
        assert!(text.contains("# call_edges\n"));
        assert!(text.contains("main::leaf,15,"));
        assert!(text.contains("main::mid,main::leaf,15,"));
    }

    /// CSV-SEMANTIC-PARITY: model + CSV for default-calls1.
    ///
    /// Loads the real fixture via `ProfileModel::from_path`, asserts oracle-aligned
    /// counts (leaf returns 15, mid returns 3, mid→leaf edge 15), and checks that
    /// the shipped `render_subs_csv` / `render_edges_csv` / `render_csv_report`
    /// paths surface those rows. See `docs/schemas/csv-semantic-parity-mvp-v0.md`.
    #[test]
    fn csv_semantic_parity_default_calls1() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());

        // 1) Real model from fixture (not synthetic totals).
        let model = ProfileModel::from_path(&path).expect("ProfileModel::from_path");

        let leaf = model
            .sub_total("main::leaf")
            .expect("model must have main::leaf");
        let mid = model
            .sub_total("main::mid")
            .expect("model must have main::mid");
        assert_eq!(
            leaf.returns, 15,
            "main::leaf returns must be 15 (oracle A5 / aggregates.oracle.json)"
        );
        assert_eq!(
            mid.returns, 3,
            "main::mid returns must be 3 (oracle A5 / aggregates.oracle.json)"
        );

        let edge = model
            .call_edge("main::mid", "main::leaf")
            .expect("model must have mid→leaf call_edge (A7)");
        assert_eq!(
            edge.count, 15,
            "main::mid → main::leaf count must be 15 (oracle A7)"
        );

        // 2) Subs CSV (A5).
        let subs = render_subs_csv(&model);
        assert!(
            subs.starts_with("name,returns,incl,excl\n"),
            "subs header:\n{subs}"
        );
        assert!(
            subs.contains("main::leaf,15,"),
            "subs must contain main::leaf,15,:\n{subs}"
        );
        assert!(
            subs.contains("main::mid,3,"),
            "subs must contain main::mid,3,:\n{subs}"
        );

        // 3) Edges CSV (A7).
        let edges = render_edges_csv(&model);
        assert!(
            edges.starts_with("caller,called,count,incl,excl\n"),
            "edges header:\n{edges}"
        );
        assert!(
            edges.contains("main::mid,main::leaf,15,"),
            "edges must contain mid→leaf count 15:\n{edges}"
        );

        // 4) Dual-section CSV (default CLI `csv` path).
        let dual = render_csv_report(&model);
        assert!(
            dual.contains("# subroutines\n"),
            "dual must have # subroutines:\n{dual}"
        );
        assert!(
            dual.contains("# call_edges\n"),
            "dual must have # call_edges:\n{dual}"
        );
        assert!(
            dual.contains("main::leaf,15,"),
            "dual must contain main::leaf,15,:\n{dual}"
        );
        assert!(
            dual.contains("main::mid,3,"),
            "dual must contain main::mid,3,:\n{dual}"
        );
        assert!(
            dual.contains("main::mid,main::leaf,15,"),
            "dual must contain mid→leaf count 15:\n{dual}"
        );
    }

    #[test]
    fn csv_escape_quotes_when_needed() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn escape_html_basic() {
        assert_eq!(escape_html("<"), "&lt;");
        assert_eq!(escape_html(">"), "&gt;");
        assert_eq!(escape_html("&"), "&amp;");
        assert_eq!(escape_html("\""), "&quot;");
        assert_eq!(
            escape_html("a < b & c > \"d\""),
            "a &lt; b &amp; c &gt; &quot;d&quot;"
        );
    }

    #[test]
    fn html_summary_default_calls1_real_render_path() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();

        let model = ProfileModel::from_path(&path).expect("build model");
        let html = render_html_summary(&model, &path_str);

        let lower = html.to_ascii_lowercase();
        assert!(
            lower.contains("<!doctype html"),
            "must start with doctype:\n{}",
            &html[..html.len().min(200)]
        );
        assert!(html.contains("<title>"), "must have title");
        assert!(html.contains("main::leaf"), "must list main::leaf:\n{html}");
        assert!(html.contains("main::mid"), "must list main::mid:\n{html}");

        // returns 15 and 3 as table cells (or obvious text next to names).
        assert!(
            html.contains(">15<") || html.contains("returns\">15") || html.contains("returns=15"),
            "leaf returns 15 must appear:\n{html}"
        );
        assert!(
            html.contains(">3<") || html.contains("returns\">3") || html.contains("returns=3"),
            "mid returns 3 must appear:\n{html}"
        );
        // Stronger: table rows include the names and return counts.
        assert!(
            html.contains("main::leaf") && html.contains(">15<"),
            "leaf row with returns cell 15"
        );
        assert!(
            html.contains("main::mid") && html.contains(">3<"),
            "mid row with returns cell 3"
        );

        // Single-file CSS policy: inline shared stylesheet (self-contained).
        assert!(
            html.contains("<style>") && html.contains(SHARED_STYLE_CSS.trim()),
            "single-file must inline SHARED_STYLE_CSS"
        );
        assert!(
            !html.contains("href=\"style.css\""),
            "single-file must not require external style.css"
        );

        assert!(
            html.contains(&format!("time_line_events: {}", model.time_line_events)),
            "time_line_events must match model"
        );

        // Source section: hot loop body from SRC_LINE.
        assert!(
            html.contains("$x++") || html.contains("for 1 .. 50"),
            "source must include loop body:\n{html}"
        );
        assert!(
            html.contains("$x++") && html.contains("for 1 .. 50"),
            "source must include hot loop `$x++` / `for 1 .. 50`:\n{html}"
        );

        // Call edges section: mid → leaf with count 15.
        let lower_full = html.to_ascii_lowercase();
        assert!(
            lower_full.contains("call edges") || lower_full.contains("call-edges"),
            "must have Call edges heading:\n{html}"
        );
        assert!(
            html.contains("main::mid") && html.contains("main::leaf"),
            "edges must list mid and leaf"
        );
        // Row cells for mid→leaf edge with count 15 (table class call-edges).
        let edges_idx = lower_full
            .find("call edges")
            .or_else(|| lower_full.find("call-edges"))
            .expect("call edges section");
        let edges_slice = &html[edges_idx..];
        assert!(
            edges_slice.contains("main::mid")
                && edges_slice.contains("main::leaf")
                && edges_slice.contains(">15<"),
            "call edges must include mid→leaf count 15:\n{edges_slice}"
        );

        // Exclusive ranking section.
        assert!(
            lower_full.contains("exclusive") || lower_full.contains("top exclusive"),
            "must have exclusive ranking heading:\n{html}"
        );
        let excl_idx = lower_full
            .find("top exclusive")
            .or_else(|| lower_full.find("exclusive"))
            .expect("exclusive section");
        let excl_slice = &html[excl_idx..];
        assert!(
            excl_slice.contains("main::leaf") && excl_slice.contains("main::mid"),
            "top exclusive must include leaf and mid:\n{excl_slice}"
        );

        // Escaped profile path appears.
        assert!(
            html.contains(&escape_html(&path_str)) || html.contains(path_str.as_ref()),
            "profile path in HTML"
        );
    }

    #[test]
    fn html_escapes_angle_brackets_in_source() {
        // Inject a synthetic model line with raw < so render must escape it.
        let path = fixture_out("default-calls1");
        let mut model = ProfileModel::from_path(&path).expect("build model");
        model
            .source_lines
            .insert((1, 999), "if ($a < $b && $c > 0) { }".to_owned());
        let html = render_html_summary(&model, "test.out");
        assert!(
            html.contains("&lt;") && html.contains("&gt;"),
            "source < and > must be escaped:\n{html}"
        );
        assert!(
            !html.contains("if ($a < $b"),
            "raw < must not appear unescaped in source cell"
        );
        assert_eq!(escape_html("<"), "&lt;");
    }

    #[test]
    fn html_site_default_calls1_render_html_site() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();

        let model = ProfileModel::from_path(&path).expect("build model");
        let site = render_html_site(&model, &path_str);

        assert_eq!(site.source_filename, "source.html");
        assert_eq!(site.style_filename, STYLE_CSS_FILENAME);
        assert_eq!(site.style_css, SHARED_STYLE_CSS);
        assert_eq!(site.sort_js_filename, SORT_JS_FILENAME);
        assert_eq!(site.sort_js, SHARED_SORT_JS);
        assert_eq!(
            site.packages_callgraph_filename,
            PACKAGES_CALLGRAPH_FILENAME
        );
        assert_eq!(site.subs_callgraph_filename, SUBS_CALLGRAPH_FILENAME);
        assert!(
            site.packages_callgraph_dot.starts_with("digraph"),
            "packages dot: {}",
            site.packages_callgraph_dot
        );
        assert!(
            site.subs_callgraph_dot.contains("->"),
            "subs dot must have edges:\n{}",
            site.subs_callgraph_dot
        );
        if model.call_edge("main::mid", "main::leaf").is_some() {
            assert!(
                site.subs_callgraph_dot.contains("main::mid")
                    && site.subs_callgraph_dot.contains("main::leaf")
                    && site.subs_callgraph_dot.contains("->"),
                "subs-callgraph.dot must include mid→leaf from call_edges:\n{}",
                site.subs_callgraph_dot
            );
        }
        assert!(!site.index_html.is_empty());
        assert!(!site.source_html.is_empty());
        assert!(
            site.file_pages.len() >= 2,
            "default-calls1 must emit ≥2 file pages, got {}: {:?}",
            site.file_pages.len(),
            site.file_pages
                .iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>()
        );
        assert!(
            site.file_pages.iter().any(|(n, _)| n == "file-1.html"),
            "must include file-1.html: {:?}",
            site.file_pages
                .iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>()
        );

        let index = &site.index_html;
        assert!(
            index.contains("packages-callgraph.dot") && index.contains("subs-callgraph.dot"),
            "index must link Graphviz files:\n{index}"
        );
        let lower = index.to_ascii_lowercase();
        assert!(
            lower.contains("<!doctype html"),
            "index must have doctype:\n{}",
            &index[..index.len().min(200)]
        );
        assert!(index.contains("<title>"), "index title");
        // Shared CSS structure: multi-file pages link style.css (no inline <style>).
        assert!(
            index.contains(&format!("href=\"{}\"", STYLE_CSS_FILENAME)),
            "index must link style.css:\n{index}"
        );
        assert!(
            !index.to_ascii_lowercase().contains("<style"),
            "multi-file index must not embed inline <style>:\n{index}"
        );
        assert!(
            index.contains("main::leaf"),
            "index must list main::leaf:\n{index}"
        );
        assert!(
            index.contains("main::mid"),
            "index must list main::mid:\n{index}"
        );
        assert!(
            index.contains("main::leaf") && index.contains(">15<"),
            "leaf returns 15 on index"
        );
        assert!(
            index.contains("main::mid") && index.contains(">3<"),
            "mid returns 3 on index"
        );
        assert!(
            index.contains(&format!("time_line_events: {}", model.time_line_events)),
            "time_line_events on index"
        );
        // Relative link to legacy source alias.
        assert!(
            index.contains(&format!("href=\"{}\"", site.source_filename))
                || index.contains(&format!("href='{}'", site.source_filename)),
            "index must link to {}:\n{index}",
            site.source_filename
        );
        // Per-file links: primary + at least one other fid.
        assert!(
            index.contains("href=\"file-1.html\""),
            "index must link to file-1.html:\n{index}"
        );
        let other_file_link = site
            .file_pages
            .iter()
            .map(|(n, _)| n.as_str())
            .find(|n| *n != "file-1.html")
            .expect("second file page");
        assert!(
            index.contains(&format!("href=\"{other_file_link}\"")),
            "index must link to {other_file_link}:\n{index}"
        );
        // A9 SUB_INFO defs when present (leaf/mid ranges).
        if model.sub_def("main::leaf").is_some() {
            assert!(
                index.contains("Subroutine definitions") || index.contains("sub-defs"),
                "index should list sub_defs when model has them:\n{index}"
            );
            assert!(
                index.contains("main::leaf") && index.contains("main::mid"),
                "sub_defs leaf/mid names"
            );
        }

        let source = &site.source_html;
        assert!(
            source.contains("$x++") || source.contains("for 1 .. 50"),
            "source must include loop body:\n{source}"
        );
        assert!(
            source.contains("$x++") && source.contains("for 1 .. 50"),
            "source must include hot loop `$x++` / `for 1 .. 50`:\n{source}"
        );
        assert!(
            source.to_ascii_lowercase().contains("workload") || source.contains("fid "),
            "source page should label workload file"
        );
        // Primary alias matches file-1.html body.
        let file1 = site
            .file_pages
            .iter()
            .find(|(n, _)| n == "file-1.html")
            .map(|(_, h)| h.as_str())
            .expect("file-1.html");
        assert_eq!(
            source, file1,
            "source.html must be a copy of primary file-1.html"
        );
        let mid_leaf_usable = model.call_sites.iter().any(|((c, d, f, l), s)| {
            c == "main::mid"
                && d == "main::leaf"
                && s.count > 0
                && call_site_usable(&model, c, *f, *l)
        });
        if mid_leaf_usable {
            assert!(
                file1.contains("calls_out") && file1.contains("main::leaf"),
                "file-1 must annotate mid→leaf call-out:\n{file1}"
            );
            assert!(
                file1.contains("calls_in") && file1.contains("main::mid"),
                "file-1 must annotate leaf call-in from mid:\n{file1}"
            );
        }
    }

    /// Private workspace under the process temp dir so staging/orphan checks
    /// do not race with parallel tests sharing `/tmp`.
    fn unique_html_workspace(label: &str) -> PathBuf {
        let ws = std::env::temp_dir().join(format!(
            "nytprof-html-ws-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&ws);
        fs::create_dir_all(&ws).expect("create test workspace");
        ws
    }

    fn assert_no_staging_leftovers(workspace: &Path, out_stem: &str) {
        for entry in fs::read_dir(workspace).expect("read workspace") {
            let entry = entry.expect("dirent");
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(".nytprof-html-") {
                panic!("orphan temp dir after publish: {name}");
            }
            if name.starts_with(&format!(".{out_stem}.bak-")) {
                panic!("orphan bak dir after publish: {name}");
            }
        }
    }

    /// ATOMIC-HTML-PUBLISH success path: real fixture → write_html_site → disk
    /// index shows leaf returns 15, mid returns 3, mid→leaf 15 (parity patterns).
    #[test]
    fn write_html_site_atomic_default_calls1() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();
        let model = ProfileModel::from_path(&path).expect("ProfileModel::from_path");

        let leaf = model.sub_total("main::leaf").expect("main::leaf");
        let mid = model.sub_total("main::mid").expect("main::mid");
        let edge = model
            .call_edge("main::mid", "main::leaf")
            .expect("mid→leaf");
        assert_eq!(leaf.returns, 15);
        assert_eq!(mid.returns, 3);
        assert_eq!(edge.count, 15);

        let ws = unique_html_workspace("atomic");
        let out = ws.join("site");
        write_html_site(&model, &path_str, &out).expect("write_html_site");
        assert_no_staging_leftovers(&ws, "site");

        let index = fs::read_to_string(out.join("index.html")).expect("index.html");
        let source = fs::read_to_string(out.join("source.html")).expect("source.html");
        assert!(
            out.join("file-1.html").is_file(),
            "file-1.html missing under {}",
            out.display()
        );
        // Complete site set includes shared CSS on the atomic publish path.
        let style_path = out.join(STYLE_CSS_FILENAME);
        assert!(
            style_path.is_file(),
            "style.css missing under {}",
            out.display()
        );
        let style = fs::read_to_string(&style_path).expect("read style.css");
        assert_eq!(style, SHARED_STYLE_CSS);
        assert!(
            index.contains(&format!("href=\"{}\"", STYLE_CSS_FILENAME)),
            "index must link style.css:\n{index}"
        );
        assert!(
            index.contains("main::leaf") && index.contains(&format!(">{}<", leaf.returns)),
            "leaf returns {}:\n{index}",
            leaf.returns
        );
        assert!(
            index.contains("main::mid") && index.contains(&format!(">{}<", mid.returns)),
            "mid returns {}:\n{index}",
            mid.returns
        );
        let lower = index.to_ascii_lowercase();
        let edges_idx = lower
            .find("call edges")
            .or_else(|| lower.find("call-edges"))
            .expect("call edges section");
        let edges_slice = &index[edges_idx..];
        assert!(
            edges_slice.contains("main::mid")
                && edges_slice.contains("main::leaf")
                && edges_slice.contains(&format!(">{}<", edge.count)),
            "mid→leaf count {}:\n{edges_slice}",
            edge.count
        );
        assert!(source.contains("$x++") && source.contains("for 1 .. 50"));

        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn write_html_site_default_calls1_tempdir() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();
        let model = ProfileModel::from_path(&path).expect("build model");

        let ws = unique_html_workspace("default");
        let tmp = ws.join("site");
        write_html_site(&model, &path_str, &tmp).expect("write_html_site");

        let index_path = tmp.join("index.html");
        let source_path = tmp.join("source.html");
        let file1_path = tmp.join("file-1.html");
        assert!(
            tmp.join(PACKAGES_CALLGRAPH_FILENAME).is_file(),
            "missing packages-callgraph.dot"
        );
        assert!(
            tmp.join(SUBS_CALLGRAPH_FILENAME).is_file(),
            "missing subs-callgraph.dot"
        );
        let subs_dot =
            fs::read_to_string(tmp.join(SUBS_CALLGRAPH_FILENAME)).expect("read subs dot");
        assert!(
            subs_dot.starts_with("digraph") && subs_dot.contains("->"),
            "subs-callgraph.dot must be a real digraph:\n{subs_dot}"
        );
        assert!(
            index_path.is_file(),
            "index.html missing at {}",
            index_path.display()
        );
        assert!(
            source_path.is_file(),
            "source.html missing at {}",
            source_path.display()
        );
        assert!(
            file1_path.is_file(),
            "file-1.html missing at {}",
            file1_path.display()
        );
        // At least one more fid page (warnings.pm is typically file-2.html).
        let extra_file = ["file-2.html", "file-3.html"]
            .iter()
            .map(|n| tmp.join(n))
            .find(|p| p.is_file());
        assert!(
            extra_file.is_some(),
            "expected file-2.html or file-3.html under {}",
            tmp.display()
        );

        let index = fs::read_to_string(&index_path).expect("read index");
        let source = fs::read_to_string(&source_path).expect("read source");
        let file1 = fs::read_to_string(&file1_path).expect("read file-1");
        assert!(!index.is_empty(), "index.html non-empty");
        assert!(!source.is_empty(), "source.html non-empty");
        assert!(index.contains("main::leaf") && index.contains(">15<"));
        assert!(index.contains("main::mid") && index.contains(">3<"));
        assert!(index.contains("href=\"source.html\""));
        assert!(index.contains("href=\"file-1.html\""));
        assert!(
            index.contains("href=\"file-2.html\"") || index.contains("href=\"file-3.html\""),
            "index must link to another file-N.html:\n{index}"
        );
        assert!(source.contains("$x++") && source.contains("for 1 .. 50"));
        assert!(file1.contains("$x++") && file1.contains("for 1 .. 50"));
        assert_eq!(source, file1, "source.html alias of file-1.html");

        // Shared CSS asset on disk + pages link it.
        let style_path = tmp.join(STYLE_CSS_FILENAME);
        assert!(
            style_path.is_file(),
            "style.css missing at {}",
            style_path.display()
        );
        let style = fs::read_to_string(&style_path).expect("read style.css");
        assert_eq!(style, SHARED_STYLE_CSS);
        assert!(index.contains(&format!("href=\"{}\"", STYLE_CSS_FILENAME)));
        assert!(file1.contains(&format!("href=\"{}\"", STYLE_CSS_FILENAME)));
        assert!(source.contains(&format!("href=\"{}\"", STYLE_CSS_FILENAME)));

        let _ = fs::remove_dir_all(&ws);
    }

    /// PR-A01: shared CSS + structure contract (default-calls1, 15/3/15).
    ///
    /// - multi-file: `style.css` on disk; pages use `<link rel="stylesheet">`
    /// - single-file: same CSS text inlined (self-contained policy)
    /// - structure: stable table classes + semantic counts from real model
    ///
    /// See `docs/schemas/html-shared-css-structure-mvp-v0.md`.
    #[test]
    fn html_shared_css_structure_contract_default_calls1() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();
        let model = ProfileModel::from_path(&path).expect("ProfileModel::from_path");

        let leaf = model.sub_total("main::leaf").expect("main::leaf");
        let mid = model.sub_total("main::mid").expect("main::mid");
        let edge = model
            .call_edge("main::mid", "main::leaf")
            .expect("mid→leaf");
        assert_eq!(leaf.returns, 15, "semantic leaf returns");
        assert_eq!(mid.returns, 3, "semantic mid returns");
        assert_eq!(edge.count, 15, "semantic mid→leaf count");

        // --- multi-file site ---
        let site = render_html_site(&model, &path_str);
        assert_eq!(site.style_filename, "style.css");
        assert_eq!(site.style_css, SHARED_STYLE_CSS);
        assert!(
            !site.style_css.is_empty() && site.style_css.contains("body{"),
            "shared CSS body non-empty"
        );

        for (label, html) in std::iter::once(("index", site.index_html.as_str()))
            .chain(std::iter::once(("source", site.source_html.as_str())))
            .chain(
                site.file_pages
                    .iter()
                    .map(|(n, h)| (n.as_str(), h.as_str())),
            )
        {
            assert!(
                html.contains(&format!("href=\"{}\"", STYLE_CSS_FILENAME)),
                "{label} must link style.css:\n{html}"
            );
            assert!(
                !html.to_ascii_lowercase().contains("<style"),
                "{label} must not use inline <style>:\n{html}"
            );
            assert!(
                html.to_ascii_lowercase().contains("<!doctype html"),
                "{label} doctype"
            );
            assert!(html.contains("<html lang=\"en\">"), "{label} lang");
            assert!(html.contains("<meta charset=\"utf-8\">"), "{label} charset");
        }

        // Structure contract on index: required section headings + table classes.
        let index = &site.index_html;
        for needle in [
            "class=\"profile-path\"",
            "id=\"subs_table\"",
            "class=\"call-edges",
            "id=\"filestable\"",
            "<h2>Event counts</h2>",
            "<h2>Subroutines</h2>",
            "<h2>Call edges</h2>",
            "<h2>Source Code Files</h2>",
            "Performance Profile Index",
        ] {
            assert!(
                index.contains(needle),
                "index structure missing {needle}:\n{index}"
            );
        }
        // Semantic counts 15/3/15 on index tables (name may be an <a href>).
        assert!(
            index.contains("main::leaf") && index.contains(&format!(">{}<", leaf.returns)),
            "leaf returns cell"
        );
        assert!(
            index.contains("main::mid") && index.contains(&format!(">{}<", mid.returns)),
            "mid returns cell"
        );
        let edges_idx = index
            .to_ascii_lowercase()
            .find("call edges")
            .expect("call edges");
        let edges = &index[edges_idx..];
        assert!(
            edges.contains("main::mid")
                && edges.contains("main::leaf")
                && edges.contains(&format!(">{}<", edge.count)),
            "mid→leaf {} on index",
            edge.count
        );

        // Source page structure.
        let source = &site.source_html;
        assert!(source.contains("class=\"source"));
        assert!(source.contains("$x++") && source.contains("for 1 .. 50"));

        // Disk publish includes style.css with exact shared text.
        let ws = unique_html_workspace("shared-css");
        let out = ws.join("site");
        write_html_site(&model, &path_str, &out).expect("write_html_site");
        let disk_css = fs::read_to_string(out.join(STYLE_CSS_FILENAME)).expect("style.css");
        assert_eq!(disk_css, SHARED_STYLE_CSS);
        let disk_index = fs::read_to_string(out.join("index.html")).expect("index");
        assert!(disk_index.contains("href=\"style.css\""));
        assert!(
            disk_index.contains("main::leaf") && disk_index.contains(">15<"),
            "disk index leaf 15"
        );
        assert!(
            disk_index.contains("main::mid") && disk_index.contains(">3<"),
            "disk index mid 3"
        );
        // Full 15/3/15 on published index (mirror in-memory call-edges check).
        let disk_lower = disk_index.to_ascii_lowercase();
        let disk_edges_idx = disk_lower
            .find("call edges")
            .or_else(|| disk_lower.find("call-edges"))
            .expect("disk index call edges");
        let disk_edges = &disk_index[disk_edges_idx..];
        assert!(
            disk_edges.contains("main::mid")
                && disk_edges.contains("main::leaf")
                && disk_edges.contains(&format!(">{}<", edge.count)),
            "disk index mid→leaf {}:\n{disk_edges}",
            edge.count
        );

        // --- single-file: inline same CSS, self-contained ---
        let single = render_html_summary(&model, &path_str);
        assert!(
            single.contains("<style>") && single.contains(SHARED_STYLE_CSS.trim()),
            "single-file must inline SHARED_STYLE_CSS"
        );
        assert!(
            !single.contains("href=\"style.css\""),
            "single-file must not depend on external style.css:\n{}",
            &single[..single.len().min(400)]
        );
        assert!(
            (single.contains("class=\"subs") || single.contains("id=\"subs_table\""))
                && single.contains("class=\"call-edges")
                && single.contains("class=\"top-exclusive")
                && single.contains("class=\"source"),
            "single-file structure classes"
        );
        assert!(
            single.contains("main::leaf")
                && single.contains(">15<")
                && single.contains("main::mid")
                && single.contains(">3<"),
            "single-file 15/3"
        );
        let s_edges_idx = single
            .to_ascii_lowercase()
            .find("call edges")
            .expect("single call edges");
        assert!(
            single[s_edges_idx..].contains(">15<"),
            "single-file mid→leaf 15"
        );

        let _ = fs::remove_dir_all(&ws);
    }

    /// ATOMIC-HTML-PUBLISH: second write to the same out_dir replaces atomically
    /// and still yields a complete, correct site (no partial mix of old/new).
    #[test]
    fn write_html_site_atomic_overwrite_same_outdir() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();
        let model = ProfileModel::from_path(&path).expect("build model");

        let ws = unique_html_workspace("overwrite");
        let out = ws.join("site");

        write_html_site(&model, &path_str, &out).expect("first write");
        // Poison marker must disappear after overwrite publish.
        fs::write(out.join("POISON.txt"), b"stale partial").expect("poison");
        assert!(out.join("POISON.txt").is_file());

        write_html_site(&model, &path_str, &out).expect("second write overwrite");
        assert_no_staging_leftovers(&ws, "site");

        assert!(
            !out.join("POISON.txt").exists(),
            "atomic replace must not leave files from previous out_dir"
        );
        // Complete site after overwrite still includes shared CSS.
        let style_path = out.join(STYLE_CSS_FILENAME);
        assert!(
            style_path.is_file(),
            "style.css missing after overwrite under {}",
            out.display()
        );
        assert_eq!(
            fs::read_to_string(&style_path).expect("read style.css"),
            SHARED_STYLE_CSS
        );
        let index = fs::read_to_string(out.join("index.html")).expect("index");
        assert!(
            index.contains(&format!("href=\"{}\"", STYLE_CSS_FILENAME)),
            "index must link style.css after overwrite:\n{index}"
        );
        assert!(index.contains("main::leaf") && index.contains(">15<"));
        assert!(index.contains("main::mid") && index.contains(">3<"));
        let lower = index.to_ascii_lowercase();
        let edges_idx = lower
            .find("call edges")
            .or_else(|| lower.find("call-edges"))
            .expect("call edges");
        assert!(
            index[edges_idx..].contains("main::mid")
                && index[edges_idx..].contains("main::leaf")
                && index[edges_idx..].contains(">15<"),
            "mid→leaf 15 after overwrite:\n{}",
            &index[edges_idx..]
        );
        assert!(out.join("source.html").is_file());
        assert!(out.join("file-1.html").is_file());

        let _ = fs::remove_dir_all(&ws);
    }

    /// ATOMIC-HTML-PUBLISH fail-closed: out_dir is a regular file → Err and no
    /// half-written final site directory replacing that path incorrectly.
    #[test]
    fn write_html_site_atomic_outdir_is_file_err() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();
        let model = ProfileModel::from_path(&path).expect("build model");

        let ws = unique_html_workspace("as-file");
        let out = ws.join("site");
        // Parent exists; out_dir itself is a file (not a directory).
        fs::write(&out, b"not a directory").expect("create file at out_dir");
        assert!(out.is_file());

        let err =
            write_html_site(&model, &path_str, &out).expect_err("must fail when out_dir is file");
        assert!(
            err.kind() == io::ErrorKind::AlreadyExists || err.kind() == io::ErrorKind::Other,
            "expected AlreadyExists-ish error, got {err:?}"
        );
        // Path remains a file with original content — not replaced by a site dir.
        assert!(out.is_file(), "out_dir must remain a file on failure");
        let body = fs::read_to_string(&out).expect("read out file");
        assert_eq!(body, "not a directory");
        assert_no_staging_leftovers(&ws, "site");

        let _ = fs::remove_dir_all(&ws);
    }

    /// ATOMIC-HTML-PUBLISH fail-closed: parent of out_dir is a file → Err without
    /// creating the final site under a wrong location.
    #[test]
    fn write_html_site_atomic_parent_is_file_err() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();
        let model = ProfileModel::from_path(&path).expect("build model");

        let ws = unique_html_workspace("parent-file");
        let parent_as_file = ws.join("blocked-parent");
        fs::write(&parent_as_file, b"blocks mkdir").expect("parent file");
        let out = parent_as_file.join("site");

        let err =
            write_html_site(&model, &path_str, &out).expect_err("must fail when parent is file");
        // Linux typically returns ENOTDIR (20) when mkdir under a file path.
        assert!(
            err.raw_os_error() == Some(20) // ENOTDIR
                || err.kind() == io::ErrorKind::AlreadyExists
                || err.kind() == io::ErrorKind::PermissionDenied
                || err.kind() == io::ErrorKind::Other,
            "expected create_dir failure when parent is a file, got {err:?}"
        );
        assert!(
            !out.exists(),
            "must not create final site when parent is unusable"
        );
        assert!(parent_as_file.is_file());

        let _ = fs::remove_dir_all(&ws);
    }

    /// HTML-OUTDIR-SAFETY: PathBuf with a `..` component is rejected before write.
    #[test]
    fn write_html_site_rejects_dotdot_component() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();
        let model = ProfileModel::from_path(&path).expect("build model");

        let ws = unique_html_workspace("dotdot");
        // Explicit `..` component under the workspace (traversal intent).
        let out = ws.join("a").join("..").join("evil-site");
        assert!(
            out.components().any(|c| matches!(c, Component::ParentDir)),
            "test path must contain ParentDir: {}",
            out.display()
        );

        let err = write_html_site(&model, &path_str, &out).expect_err("must reject '..' component");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput, "got {err:?}");
        let msg = err.to_string();
        assert!(
            msg.contains("..") || msg.to_ascii_lowercase().contains("parent"),
            "error should mention '..': {msg}"
        );
        // Must not have published a site at the resolved/evil location either.
        assert!(!ws.join("evil-site").exists());
        assert_no_staging_leftovers(&ws, "evil-site");

        let _ = fs::remove_dir_all(&ws);
    }

    /// HTML-OUTDIR-SAFETY: empty out_dir path is rejected.
    #[test]
    fn write_html_site_rejects_empty_path() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();
        let model = ProfileModel::from_path(&path).expect("build model");

        let err = write_html_site(&model, &path_str, Path::new(""))
            .expect_err("must reject empty out_dir");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput, "got {err:?}");
        assert!(
            err.to_string().to_ascii_lowercase().contains("empty"),
            "error should mention empty: {err}"
        );
    }

    /// HTML-OUTDIR-SAFETY: null byte in out_dir (Linux OsString) is rejected.
    #[cfg(unix)]
    #[test]
    fn write_html_site_rejects_null_byte() {
        use std::ffi::OsString;
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();
        let model = ProfileModel::from_path(&path).expect("build model");

        let ws = unique_html_workspace("nul");
        let mut bytes = ws.as_os_str().as_bytes().to_vec();
        bytes.push(b'/');
        bytes.extend_from_slice(b"site");
        bytes.push(0);
        bytes.extend_from_slice(b"evil");
        let out = PathBuf::from(OsString::from_vec(bytes));
        assert!(path_os_contains_nul(&out), "test path must contain NUL");

        let err =
            write_html_site(&model, &path_str, &out).expect_err("must reject null byte in out_dir");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput, "got {err:?}");
        let msg = err.to_string().to_ascii_lowercase();
        assert!(
            msg.contains("null") || msg.contains("nul"),
            "error should mention null: {err}"
        );
        assert_no_staging_leftovers(&ws, "site");

        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn html_site_blocks_calls1_source_line_calls() {
        let path = fixture_out("blocks-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let model = ProfileModel::from_path(&path).expect("build model");
        let lt = model
            .line_total(1, 5)
            .expect("line_total(1,5) from TIME_BLOCK");
        assert!(lt.calls > 0, "expected positive calls on line 5");

        let site = render_html_site(&model, "blocks-calls1.out");
        let calls_cell = format!(">{}<", lt.calls);
        assert!(
            site.source_html.contains(&calls_cell),
            "source page must show line_total(1,5).calls={}:\n{}",
            lt.calls,
            site.source_html
        );
        assert!(
            site.source_html.contains("$x++") || site.source_html.contains("for 1 .. 50"),
            "source hot loop:\n{}",
            site.source_html
        );
        assert!(
            site.index_html.contains("href=\"source.html\""),
            "index link:\n{}",
            site.index_html
        );
    }

    /// blocks-calls1: TIME_BLOCK only; A4 line_totals filled from TIME_BLOCK.
    /// HTML source must show model-derived calls (e.g. line 5) for the hot loop.
    #[test]
    fn html_summary_blocks_calls1_line_calls() {
        let path = fixture_out("blocks-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();

        let model = ProfileModel::from_path(&path).expect("build model");
        assert!(
            model.time_block_events > 0,
            "blocks-calls1 must have TIME_BLOCK"
        );
        assert_eq!(
            model.time_line_events, 0,
            "blocks-calls1 should have no TIME_LINE when blocks=1"
        );

        let leaf = model.sub_total("main::leaf").expect("main::leaf in model");
        let mid = model.sub_total("main::mid").expect("main::mid in model");
        assert_eq!(leaf.returns, 15, "main::leaf returns");
        assert_eq!(mid.returns, 3, "main::mid returns");

        let lt = model
            .line_total(1, 5)
            .expect("line_total(1,5) from TIME_BLOCK");
        let expected_calls = lt.calls;
        assert!(
            expected_calls > 0,
            "line_total(1,5).calls must be positive, got {expected_calls}"
        );

        let html = render_html_summary(&model, &path_str);

        assert!(
            html.contains("main::leaf") && html.contains(">15<"),
            "leaf returns 15 must appear:\n{html}"
        );
        assert!(
            html.contains("main::mid") && html.contains(">3<"),
            "mid returns 3 must appear:\n{html}"
        );

        // Hot loop body from SRC_LINE (workload.pl line 5).
        assert!(
            html.contains("$x++") || html.contains("for 1 .. 50"),
            "source must include loop body ($x++ / for 1 .. 50):\n{html}"
        );

        // Source table must surface A4 calls for the hot line (not invent a constant).
        let calls_cell = format!(">{expected_calls}<");
        assert!(
            html.contains(&calls_cell),
            "source must show line_total(1,5).calls={expected_calls} as table cell:\n{html}"
        );

        // Prefer a tight match near line 5 + source text when present.
        let src_idx = html
            .find("class=\"source\"")
            .or_else(|| html.find(">Source"))
            .expect("source section");
        let src_slice = &html[src_idx..];
        assert!(
            src_slice.contains(&calls_cell),
            "calls={expected_calls} must appear in source section:\n{src_slice}"
        );
        assert!(
            src_slice.contains("$x++") || src_slice.contains("for 1 .. 50"),
            "source section must include hot loop text:\n{src_slice}"
        );
        // Line 5 row: `id="L5"` plus A4 calls (data-sort-value cells).
        assert!(
            src_slice.contains("id=\"L5\"") && src_slice.contains(&calls_cell),
            "line 5 source row must include calls={expected_calls}:\n{src_slice}"
        );
    }

    /// blocks-calls1 A4b: HTML must surface model `block_line_totals` (not hard-coded).
    #[test]
    fn html_summary_blocks_calls1_block_line_totals() {
        let path = fixture_out("blocks-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();

        let model = ProfileModel::from_path(&path).expect("build model");
        assert!(
            !model.block_line_totals.is_empty(),
            "blocks-calls1 must have A4b block_line_totals"
        );

        let leaf = model.sub_total("main::leaf").expect("main::leaf in model");
        let mid = model.sub_total("main::mid").expect("main::mid in model");
        assert_eq!(leaf.returns, 15, "main::leaf returns");
        assert_eq!(mid.returns, 3, "main::mid returns");

        // A4 line totals still present for the hot line.
        let lt = model
            .line_total(1, 5)
            .expect("line_total(1,5) from TIME_BLOCK");
        assert!(lt.calls > 0, "line_total(1,5).calls must be positive");

        // Pick a deterministic key from the model (first after sort by fid, block_line).
        let mut block_keys: Vec<_> = model.block_line_totals.keys().copied().collect();
        block_keys.sort_unstable();
        let (bfid, bline) = block_keys[0];
        let bt = model
            .block_line_total(bfid, bline)
            .expect("block_line_totals entry");
        assert!(
            bt.calls > 0,
            "first block_line_totals entry must have positive calls"
        );

        let html = render_html_summary(&model, &path_str);

        assert!(
            html.contains("main::leaf") && html.contains(">15<"),
            "leaf returns 15 must appear:\n{html}"
        );
        assert!(
            html.contains("main::mid") && html.contains(">3<"),
            "mid returns 3 must appear:\n{html}"
        );

        // A4 line_total(1,5).calls still surfaces in HTML.
        let a4_calls_cell = format!(">{}<", lt.calls);
        assert!(
            html.contains(&a4_calls_cell),
            "line_total(1,5).calls={} must appear:\n{html}",
            lt.calls
        );

        // A4b heading.
        let lower = html.to_ascii_lowercase();
        assert!(
            lower.contains("block line totals") || lower.contains("block-line-totals"),
            "must have Block line totals heading:\n{html}"
        );

        // Model-derived block calls as a num cell (do not hard-code fixture values).
        let block_calls_cell = format!(">{}<", bt.calls);
        assert!(
            html.contains(&block_calls_cell),
            "block_line_totals({bfid},{bline}).calls={} must appear as num cell:\n{html}",
            bt.calls
        );

        let block_idx = lower
            .find("block line totals")
            .or_else(|| lower.find("block-line-totals"))
            .expect("block line totals section");
        let block_slice = &html[block_idx..];
        assert!(
            block_slice.contains(&block_calls_cell),
            "block calls cell in Block line totals section:\n{block_slice}"
        );
        assert!(
            block_slice.contains(&format!(">{}<", bfid))
                && block_slice.contains(&format!(">{}<", bline)),
            "block section should list fid={bfid} and block_line={bline}:\n{block_slice}"
        );
    }

    /// Multi-file source page also surfaces A4b for the primary workload fid.
    #[test]
    fn html_site_blocks_calls1_block_line_totals() {
        let path = fixture_out("blocks-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let model = ProfileModel::from_path(&path).expect("build model");
        assert!(
            !model.block_line_totals.is_empty(),
            "blocks-calls1 must have A4b block_line_totals"
        );

        let primary = primary_workload_fid(&model);
        let mut block_keys: Vec<_> = model
            .block_line_totals
            .keys()
            .copied()
            .filter(|(fid, _)| *fid == primary)
            .collect();
        // Fall back to any entry if primary has none (should not happen for this fixture).
        if block_keys.is_empty() {
            block_keys = model.block_line_totals.keys().copied().collect();
        }
        block_keys.sort_unstable();
        let (bfid, bline) = block_keys[0];
        let bt = model
            .block_line_total(bfid, bline)
            .expect("block_line_totals entry");
        assert!(bt.calls > 0, "positive block calls expected");

        let site = render_html_site(&model, "blocks-calls1.out");
        let source = &site.source_html;
        let lower = source.to_ascii_lowercase();
        assert!(
            lower.contains("block line totals") || lower.contains("block-line-totals"),
            "source page must have Block line totals:\n{source}"
        );
        let calls_cell = format!(">{}<", bt.calls);
        assert!(
            source.contains(&calls_cell),
            "source page must show block calls={}:\n{source}",
            bt.calls
        );
    }

    /// REPORT-SEMANTIC-PARITY: model + HTML for default-calls1.
    ///
    /// Loads the real fixture via `ProfileModel::from_path`, asserts oracle-aligned
    /// counts (leaf returns 15, mid returns 3, mid→leaf edge 15), and checks that
    /// the shipped `render_html_summary` / site path surfaces those numbers.
    /// See `docs/schemas/report-semantic-parity-mvp-v0.md`.
    #[test]
    fn report_semantic_parity_default_calls1() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();

        // 1) Real model from fixture (not synthetic totals).
        let model = ProfileModel::from_path(&path).expect("ProfileModel::from_path");

        let leaf = model
            .sub_total("main::leaf")
            .expect("model must have main::leaf");
        let mid = model
            .sub_total("main::mid")
            .expect("model must have main::mid");
        assert_eq!(
            leaf.returns, 15,
            "main::leaf returns must be 15 (oracle A5 / aggregates.oracle.json)"
        );
        assert_eq!(
            mid.returns, 3,
            "main::mid returns must be 3 (oracle A5 / aggregates.oracle.json)"
        );

        let edge = model
            .call_edge("main::mid", "main::leaf")
            .expect("model must have mid→leaf call_edge (A7)");
        assert_eq!(
            edge.count, 15,
            "main::mid → main::leaf count must be 15 (oracle A7)"
        );

        // 2) Single-file HTML render path (shipped library).
        let html = render_html_summary(&model, &path_str);
        assert!(
            html.to_ascii_lowercase().contains("<!doctype html"),
            "HTML must have doctype"
        );
        assert!(
            html.contains("main::leaf"),
            "HTML must list main::leaf:\n{html}"
        );
        assert!(
            html.contains("main::mid"),
            "HTML must list main::mid:\n{html}"
        );

        // Sub table cells: name (plain or linked) then returns num cell.
        assert!(
            html.contains("main::leaf") && html.contains(&format!(">{}<", leaf.returns)),
            "subs table must show leaf returns={}:\n{html}",
            leaf.returns
        );
        assert!(
            html.contains("main::mid") && html.contains(&format!(">{}<", mid.returns)),
            "subs table must show mid returns={}:\n{html}",
            mid.returns
        );

        // Call-edges section: mid → leaf with model-derived count.
        let lower = html.to_ascii_lowercase();
        let edges_idx = lower
            .find("call edges")
            .or_else(|| lower.find("call-edges"))
            .expect("call edges section");
        let edges_slice = &html[edges_idx..];
        let edge_count_cell = format!(">{}<", edge.count);
        assert!(
            edges_slice.contains("main::mid")
                && edges_slice.contains("main::leaf")
                && edges_slice.contains(&edge_count_cell),
            "call edges must include mid→leaf count {}:\n{edges_slice}",
            edge.count
        );
        assert!(
            edges_slice.contains("main::mid")
                && edges_slice.contains("main::leaf")
                && edges_slice.contains(&format!(">{}<", edge.count)),
            "call-edges row mid→leaf count={}:\n{edges_slice}",
            edge.count
        );

        // 3) Multi-file site index also surfaces the same semantics.
        let site = render_html_site(&model, &path_str);
        let index = &site.index_html;
        assert!(
            index.contains("main::leaf") && index.contains(&format!(">{}<", leaf.returns)),
            "site index leaf returns"
        );
        assert!(
            index.contains("main::mid") && index.contains(&format!(">{}<", mid.returns)),
            "site index mid returns"
        );
        let index_lower = index.to_ascii_lowercase();
        let idx_edges = index_lower
            .find("call edges")
            .or_else(|| index_lower.find("call-edges"))
            .expect("site index call edges");
        let idx_edges_slice = &index[idx_edges..];
        assert!(
            idx_edges_slice.contains("main::mid")
                && idx_edges_slice.contains("main::leaf")
                && idx_edges_slice.contains(&edge_count_cell),
            "site index mid→leaf count {}:\n{idx_edges_slice}",
            edge.count
        );
    }

    /// BLOCKS-SEMANTIC-PARITY: model + HTML for blocks-calls1.
    ///
    /// Loads the real fixture via `ProfileModel::from_path`, asserts oracle-aligned
    /// counts (A4 line 5 calls **780** from TIME_BLOCK; leaf returns **15**, mid **3**),
    /// and checks that the shipped `render_html_summary` / site path surfaces those numbers.
    /// See `docs/schemas/blocks-semantic-parity-mvp-v0.md`.
    #[test]
    fn blocks_semantic_parity_blocks_calls1() {
        let path = fixture_out("blocks-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let path_str = path.to_string_lossy();

        // 1) Real model from fixture (not synthetic totals).
        let model = ProfileModel::from_path(&path).expect("ProfileModel::from_path");

        // blocks=1 → statement timing is TIME_BLOCK only.
        assert!(
            model.time_block_events > 0,
            "blocks-calls1 must have TIME_BLOCK events, got {}",
            model.time_block_events
        );
        assert_eq!(
            model.time_line_events, 0,
            "blocks-calls1 should have no TIME_LINE when blocks=1"
        );

        // A4 from TIME_BLOCK: hot loop line in workload.pl.
        let lt = model
            .line_total(1, 5)
            .expect("line_total(1, 5) from TIME_BLOCK");
        assert_eq!(
            lt.calls, 780,
            "line_total(1,5).calls must be 780 (oracle A4 / aggregates.oracle.json)"
        );
        assert!(lt.ticks > 0, "line_total(1,5).ticks > 0, got {}", lt.ticks);

        let leaf = model
            .sub_total("main::leaf")
            .expect("model must have main::leaf");
        let mid = model
            .sub_total("main::mid")
            .expect("model must have main::mid");
        assert_eq!(
            leaf.returns, 15,
            "main::leaf returns must be 15 (oracle A5 / aggregates.oracle.json)"
        );
        assert_eq!(
            mid.returns, 3,
            "main::mid returns must be 3 (oracle A5 / aggregates.oracle.json)"
        );

        // Supporting: A4b present for blocks fixture.
        assert!(
            !model.block_line_totals.is_empty(),
            "A4b block_line_totals must be non-empty when TIME_BLOCK present"
        );

        // 2) Single-file HTML render path (shipped library).
        let html = render_html_summary(&model, &path_str);
        assert!(
            html.to_ascii_lowercase().contains("<!doctype html"),
            "HTML must have doctype"
        );
        assert!(
            html.contains("main::leaf"),
            "HTML must list main::leaf:\n{html}"
        );
        assert!(
            html.contains("main::mid"),
            "HTML must list main::mid:\n{html}"
        );

        // Sub table cells: name then returns num cell.
        assert!(
            html.contains(&format!(
                "main::leaf</td><td class=\"num\">{}</td>",
                leaf.returns
            )) || html.contains(&format!(">{}<", leaf.returns)),
            "subs table must show leaf returns={}:\n{html}",
            leaf.returns
        );
        assert!(
            html.contains(&format!(
                "main::mid</td><td class=\"num\">{}</td>",
                mid.returns
            )) || html.contains(&format!(">{}<", mid.returns)),
            "subs table must show mid returns={}:\n{html}",
            mid.returns
        );

        // A4 line calls 780 in source / line context.
        let calls_cell = format!(">{}<", lt.calls);
        assert!(
            html.contains(&calls_cell),
            "HTML must show line_total(1,5).calls={} as cell:\n{html}",
            lt.calls
        );
        let src_idx = html
            .find("class=\"source\"")
            .or_else(|| html.find(">Source"))
            .expect("source section");
        let src_slice = &html[src_idx..];
        assert!(
            src_slice.contains(&calls_cell),
            "source section must show calls={}:\n{src_slice}",
            lt.calls
        );
        assert!(
            src_slice.contains("$x++") || src_slice.contains("for 1 .. 50"),
            "source section must include hot loop text:\n{src_slice}"
        );
        assert!(
            src_slice.contains("id=\"L5\"") && src_slice.contains(&calls_cell),
            "line 5 source row must include calls={}:\n{src_slice}",
            lt.calls
        );

        // 3) Multi-file site: index leaf/mid + source page line calls.
        let site = render_html_site(&model, &path_str);
        let index = &site.index_html;
        assert!(
            index.contains("main::leaf") && index.contains(&format!(">{}<", leaf.returns)),
            "site index leaf returns"
        );
        assert!(
            index.contains("main::mid") && index.contains(&format!(">{}<", mid.returns)),
            "site index mid returns"
        );
        assert!(
            site.source_html.contains(&calls_cell),
            "site source page must show line_total(1,5).calls={}:\n{}",
            lt.calls,
            site.source_html
        );
        assert!(
            site.source_html.contains("$x++") || site.source_html.contains("for 1 .. 50"),
            "site source hot loop:\n{}",
            site.source_html
        );
    }

    #[test]
    fn verify_profile_default_calls1_ok() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let model = ProfileModel::from_path(&path).expect("build model for expected counts");
        let summary = verify_profile(&path).expect("verify_profile should Ok");

        assert!(
            summary.contains("OK") || summary.to_ascii_lowercase().contains("ok:"),
            "must contain OK:\n{summary}"
        );
        assert!(
            summary.contains(path.to_string_lossy().as_ref())
                || summary.contains("nytprof.out")
                || summary.contains("default-calls1"),
            "must mention path or basename:\n{summary}"
        );
        // Counters must match the model (TIME_LINE / event counts).
        assert!(
            summary.contains(&format!("TIME_LINE: {}", model.time_line_events))
                || summary.contains(&format!("time_line_events: {}", model.time_line_events)),
            "TIME_LINE count must match model ({}):\n{summary}",
            model.time_line_events
        );
        assert!(
            summary.contains(&format!("events: {}", model.total_events))
                || summary.contains(&format!("total_events: {}", model.total_events)),
            "events count must match model ({}):\n{summary}",
            model.total_events
        );
        assert!(
            summary.contains(&format!("files: {}", model.files.len())),
            "files count:\n{summary}"
        );
        assert!(
            summary.contains(&format!("subs: {}", model.sub_return_totals.len())),
            "subs count:\n{summary}"
        );
    }

    /// COMPAT-010-ERR: unique tempfile path for corrupt-input cases.
    fn fail_closed_temp(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nytprof-fail-closed-{}-{}-{}",
            label,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    /// COMPAT-010-ERR: shipped `verify_profile` and report-path model load must
    /// return `Err` (no panic, no silent Ok) on empty / truncated / bad magic.
    fn assert_fail_closed_verify_and_model(path: &Path, label: &str) {
        let verify = verify_profile(path);
        assert!(
            verify.is_err(),
            "{label}: verify_profile must Err, got Ok:\n{}",
            verify.as_ref().unwrap()
        );
        // report / summary path uses the same model load as verify.
        let model = ProfileModel::from_path(path);
        assert!(
            model.is_err(),
            "{label}: ProfileModel::from_path (report path) must Err, got Ok"
        );
    }

    #[test]
    fn verify_profile_truncated_default_calls1_err() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let bytes = fs::read(&path).expect("read golden profile");
        assert!(bytes.len() > 2, "fixture must be non-trivial");
        let half = bytes.len() / 2;
        assert!(half > 0);

        let tmp = fail_closed_temp("truncated");
        fs::write(&tmp, &bytes[..half]).expect("write truncated profile");
        let result = verify_profile(&tmp);
        let _ = fs::remove_file(&tmp);
        assert!(
            result.is_err(),
            "truncated profile must fail verify, got Ok:\n{}",
            result.as_ref().unwrap()
        );
    }

    /// COMPAT-010-ERR: empty file → verify + model load Err (no panic).
    #[test]
    fn fail_closed_empty_file_verify_and_model_err() {
        let tmp = fail_closed_temp("empty");
        fs::write(&tmp, b"").expect("write empty");
        assert_fail_closed_verify_and_model(&tmp, "empty file");
        let _ = fs::remove_file(&tmp);
    }

    /// COMPAT-010-ERR: half of default-calls1 → verify + model load Err.
    #[test]
    fn fail_closed_truncated_default_calls1_verify_and_model_err() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let bytes = fs::read(&path).expect("read golden");
        let half = bytes.len() / 2;
        assert!(half > 0, "fixture empty");

        let tmp = fail_closed_temp("trunc-half");
        fs::write(&tmp, &bytes[..half]).expect("write half");
        assert_fail_closed_verify_and_model(&tmp, "truncated half of default-calls1");
        let _ = fs::remove_file(&tmp);
    }

    /// COMPAT-010-ERR: bad header magic → verify + model load Err.
    #[test]
    fn fail_closed_bad_magic_verify_and_model_err() {
        let tmp = fail_closed_temp("bad-magic");
        fs::write(&tmp, b"NOTPROF 5 0\n").expect("write bad magic");
        assert_fail_closed_verify_and_model(&tmp, "bad magic NOTPROF");
        let _ = fs::remove_file(&tmp);
    }

    /// INCOMPLETE-STREAM: first 500 bytes of default-calls1 is a record-aligned
    /// short prefix (header + ATTRIBUTES/OPTIONS, no TIME_LINE, often no PID_END).
    /// Decode/model may succeed; verify must still fail closed by default.
    #[test]
    fn verify_profile_incomplete_prefix_default_calls1_err() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let bytes = fs::read(&path).expect("read golden");
        assert!(bytes.len() > 500, "fixture must be larger than 500 bytes");
        let prefix = &bytes[..500];

        let tmp = fail_closed_temp("incomplete-500");
        fs::write(&tmp, prefix).expect("write incomplete prefix");

        // Model load may succeed (lenient dump path); if it fails, that is also fine.
        let model_result = ProfileModel::from_path(&tmp);
        if let Ok(ref model) = model_result {
            assert!(
                !model.is_stream_complete(),
                "500-byte prefix of default-calls1 must be incomplete \
                 (TIME_LINE={}, TIME_BLOCK={}, pid_start={}, pid_end={})",
                model.time_line_events,
                model.time_block_events,
                model.pid_start_events,
                model.pid_end_events
            );
            assert!(
                model.time_line_events + model.time_block_events == 0,
                "expected no statement timing on short prefix"
            );
            // Report path must fail closed too.
            assert!(
                require_complete_stream(model).is_err(),
                "require_complete_stream must Err without salvage"
            );
        }

        // Ensure salvage env is off for this assertion (best-effort; tests may run parallel).
        // Prefer checking error message content over relying on env isolation.
        let verify = verify_profile(&tmp);
        let _ = fs::remove_file(&tmp);

        if allow_incomplete_stream() {
            // Parallel test may have set salvage; accept Ok(INCOMPLETE) only then.
            let summary = verify.expect("salvage env allows Ok");
            assert!(
                summary.contains("INCOMPLETE"),
                "under salvage, verify must note INCOMPLETE:\n{summary}"
            );
        } else {
            assert!(
                verify.is_err(),
                "incomplete prefix must fail verify by default, got Ok:\n{}",
                verify.as_ref().unwrap()
            );
            let err = verify.unwrap_err().to_string();
            assert!(
                err.contains("incomplete"),
                "error should mention incomplete: {err}"
            );
        }
    }

    /// INCOMPLETE-STREAM: golden default-calls1 remains complete (covered by
    /// `verify_profile_default_calls1_ok`; assert model completeness explicitly).
    #[test]
    fn default_calls1_model_is_stream_complete() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let model = ProfileModel::from_path(&path).expect("build model");
        assert!(
            model.is_stream_complete(),
            "golden default-calls1 must be complete: {:?}",
            model.stream_incompleteness_reasons()
        );
        assert!(model.pid_start_events > 0, "expected PID_START on golden");
        assert!(
            model.pid_end_events >= model.pid_start_events,
            "pid_end >= pid_start"
        );
        assert!(
            model.time_line_events + model.time_block_events > 0,
            "expected statement timing on golden"
        );
        require_complete_stream(&model).expect("complete stream ok");
    }

    #[test]
    fn folded_stacks_default_calls1_real_render() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let model = ProfileModel::from_path(&path).expect("build model");
        let folded = render_folded_stacks(&model);

        assert!(!folded.is_empty(), "folded export must be non-empty");
        assert!(
            folded.contains("main::mid;main::leaf 15"),
            "mid→leaf folded line:\n{folded}"
        );
        // RUNTIME→mid edge exists on default-calls1 (A7).
        if model.call_edge("main::RUNTIME", "main::mid").is_some() {
            assert!(
                folded.contains("main::RUNTIME;main::mid 3"),
                "RUNTIME→mid folded line:\n{folded}"
            );
        }

        // Deterministic sort: lines are sorted lexicographically.
        let lines: Vec<&str> = folded.lines().filter(|l| !l.is_empty()).collect();
        let mut sorted = lines.clone();
        sorted.sort_unstable();
        assert_eq!(lines, sorted, "folded lines must be sorted");

        let edge = model
            .call_edge("main::mid", "main::leaf")
            .expect("model must have mid→leaf");
        assert_eq!(edge.count, 15);
    }

    fn flame_rect_widths(svg: &str) -> Vec<f64> {
        let mut widths = Vec::new();
        let mut rest = svg;
        while let Some(i) = rest.find("<rect ") {
            rest = &rest[i + 6..];
            if let Some(wpos) = rest.find("width=\"") {
                let after = &rest[wpos + 7..];
                if let Some(end) = after.find('"') {
                    if let Ok(w) = after[..end].parse::<f64>() {
                        widths.push(w);
                    }
                }
            }
        }
        widths
    }

    #[test]
    fn flame_svg_default_calls1_real_render() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let model = ProfileModel::from_path(&path).expect("build model");
        let svg = render_flame_svg(&model);
        assert!(svg.contains("<svg"), "well-formed svg:\n{svg}");
        assert!(
            svg.contains("main::leaf") && svg.contains("main::mid"),
            "labels:\n{svg}"
        );
        let edge = model
            .call_edge("main::mid", "main::leaf")
            .expect("mid→leaf");
        assert_eq!(edge.count, 15);
        assert!(
            svg.contains("calls: 15") || svg.contains("main::leaf (15)") || svg.contains("(15)"),
            "count 15 visible:\n{svg}"
        );
        let leaf_href = sub_href(&model, "main::leaf", true).expect("leaf href");
        assert!(
            svg.contains(&format!("href=\"{leaf_href}\""))
                && svg.contains("class=\"flame-link\""),
            "leaf frame must link to source:\n{svg}"
        );
        assert!(
            svg.contains("inclusive:") && svg.contains("exclusive:"),
            "hover title must include incl/excl:\n{svg}"
        );
        let widths = flame_rect_widths(&svg);
        assert!(
            widths.len() >= 4,
            "stacked parent+child rects, got {}: {widths:?}",
            widths.len()
        );
        let min_w = widths.iter().copied().fold(f64::INFINITY, f64::min);
        let max_w = widths.iter().copied().fold(0.0, f64::max);
        assert!(
            max_w > min_w + 0.5,
            "widths must not all be equal when edge counts differ: {widths:?}"
        );
        assert!(
            !svg.contains("jquery") && !svg.contains("tablesorter"),
            "flame svg must not pull jquery"
        );
    }

    #[test]
    fn flame_svg_stacks_callers_once_not_per_edge_columns() {
        // Equal-count scanner-shaped graph. The abandoned per-edge painter
        // emitted five identical-width columns (RUNTIME painted twice).
        let mut model = ProfileModel::default();
        for (caller, called) in [
            ("main::RUNTIME", "main::merge_freq"),
            ("main::RUNTIME", "main::scan_file"),
            ("main::scan_file", "main::classify"),
            ("main::scan_file", "main::tokenize"),
            ("main::tokenize", "main::CORE:match"),
        ] {
            model.call_edges.insert(
                (caller.to_owned(), called.to_owned()),
                nytprof_model::CallEdgeTotal {
                    count: 7576,
                    ..Default::default()
                },
            );
        }
        let svg = render_flame_svg(&model);
        assert_eq!(
            svg.matches("<title>main::RUNTIME").count(),
            1,
            "RUNTIME must be one root frame, not a column per outgoing edge:\n{svg}"
        );
        assert_eq!(
            svg.matches("<title>main::scan_file").count(),
            1,
            "scan_file must nest once under RUNTIME:\n{svg}"
        );
        assert_eq!(svg.matches("<title>main::tokenize").count(), 1, "{svg}");
        assert_eq!(svg.matches("<title>main::CORE:match").count(), 1, "{svg}");
        assert_eq!(svg.matches("<title>main::merge_freq").count(), 1, "{svg}");
        assert_eq!(svg.matches("<title>main::classify").count(), 1, "{svg}");
        let widths = flame_rect_widths(&svg);
        assert_eq!(widths.len(), 6, "one rect per unique tree node: {widths:?}");
        let max_w = widths.iter().copied().fold(0.0_f64, f64::max);
        let min_w = widths.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(
            max_w > min_w + 50.0,
            "root must be wider than nested children, not a barcode: {widths:?}"
        );
        assert!(
            svg.contains("height=\"92\""),
            "four stacked rows (RUNTIME→scan_file→tokenize→match): {svg}"
        );
    }

    #[test]
    #[test]
    fn flame_skipped_when_no_call_edges() {
        // Oracle parity: no calls data ⇒ no flame artifacts or section, even
        // with flame requested (matters now that the CLI defaults flame on).
        let model = ProfileModel::default();
        let opts = HtmlRenderOptions { flame: true };
        let site = render_html_site_with_options(&model, "nytprof.out", opts);
        assert!(
            site.flame_svg_filename.is_none() && site.flame_folded_filename.is_none(),
            "edge-less profile must not name flame files"
        );
        assert!(
            !site.index_html.contains("all_stacks_by_time")
                && !site.index_html.contains("class=\"flame\""),
            "edge-less index must not reference flame:\n{}",
            site.index_html
        );
        let summary = render_html_summary_with_options(&model, "nytprof.out", opts);
        assert!(
            !summary.contains("class=\"flame\""),
            "edge-less single-file must not embed flame section:\n{summary}"
        );
    }

    #[test]
    fn flame_svg_omits_subpixel_frames() {
        let mut model = ProfileModel::default();
        model.call_edges.insert(
            ("big".to_owned(), "child".to_owned()),
            nytprof_model::CallEdgeTotal {
                count: 1_000_000,
                ..Default::default()
            },
        );
        model.call_edges.insert(
            ("tiny".to_owned(), "speck".to_owned()),
            nytprof_model::CallEdgeTotal {
                count: 1,
                ..Default::default()
            },
        );
        let svg = render_flame_svg(&model);
        assert!(svg.contains("big"), "big frame:\n{svg}");
        assert!(
            !svg.contains("speck"),
            "sub-pixel edge must not emit a labeled frame:\n{svg}"
        );
        let widths = flame_rect_widths(&svg);
        assert_eq!(
            widths.len(),
            2,
            "only the paintable edge's parent+child rects: {widths:?}"
        );
    }

    #[test]
    fn html_site_default_no_flame_artifacts() {
        let path = fixture_out("default-calls1");
        let model = ProfileModel::from_path(&path).expect("model");
        let site = render_html_site(&model, "nytprof.out");
        assert!(site.flame_svg.is_none() && site.flame_folded.is_none());
        assert!(!site.index_html.contains("all_stacks_by_time"));
        assert!(!site.index_html.to_ascii_lowercase().contains("jquery"));
    }

    #[test]
    fn html_site_optional_flame_default_calls1() {
        let path = fixture_out("default-calls1");
        let model = ProfileModel::from_path(&path).expect("model");
        let site = render_html_site_with_options(
            &model,
            "nytprof.out",
            HtmlRenderOptions { flame: true },
        );
        let folded = site.flame_folded.expect("folded");
        let svg = site.flame_svg.expect("svg");
        assert!(folded.contains("main::mid;main::leaf 15"), "{folded}");
        assert!(svg.contains("<svg") && svg.contains("main::leaf"));
        assert!(
            site.index_html.contains("href=\"all_stacks_by_time.svg\"")
                && site.index_html.contains("href=\"all_stacks_by_time.folded\"")
        );
        assert!(
            site.index_html.contains("<svg") && site.index_html.contains("class=\"nytprof-flame\""),
            "file:// hover/click requires inlined SVG, not <img>:\n{}",
            site.index_html
        );
        assert!(
            !site.index_html.contains("<img ") && !site.index_html.contains("<object"),
            "preview must not use <img> or <object>:\n{}",
            site.index_html
        );
        assert!(
            site.index_html.contains("id=\"nytprof-flame-tip\"")
                && site.index_html.contains("querySelector"),
            "index must publish the hover tip script:\n{}",
            site.index_html
        );
        let leaf_href = sub_href(&model, "main::leaf", true).expect("leaf href");
        assert!(
            site.index_html.contains(&format!("href=\"{leaf_href}\"")),
            "inlined flame must link leaf to source:\n{}",
            site.index_html
        );
        assert!(
            site.index_html.contains("main::leaf") && site.index_html.contains(">15<")
        );
        assert!(
            site.index_html.contains("main::mid") && site.index_html.contains(">3<")
        );
    }

    #[test]
    fn callgrind_default_calls1_real_render() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let model = ProfileModel::from_path(&path).expect("build model");
        let cg = render_callgrind(&model);

        assert!(!cg.is_empty(), "callgrind export must be non-empty");
        assert!(cg.contains("# callgrind format"), "header comment:\n{cg}");
        assert!(cg.contains("positions: line"), "positions header:\n{cg}");
        assert!(
            cg.contains("events: Ticks") || cg.contains("Events: Calls"),
            "events header:\n{cg}"
        );
        assert!(cg.contains("main::leaf"), "must mention main::leaf:\n{cg}");
        assert!(cg.contains("main::mid"), "must mention main::mid:\n{cg}");
        // mid→leaf call count 15 from call_edges.
        assert!(cg.contains("15"), "must include mid→leaf count 15:\n{cg}");
        assert!(
            cg.contains("fn=main::leaf") || cg.contains("cfn=main::leaf"),
            "fn/cfn for leaf:\n{cg}"
        );
        assert!(
            cg.contains("fn=main::mid") || cg.contains("cfn=main::mid"),
            "fn/cfn for mid:\n{cg}"
        );
        assert!(
            cg.contains("calls=15 0") || cg.contains("calls=15"),
            "calls=15 for mid→leaf:\n{cg}"
        );
    }

    /// EXPORT-SEMANTIC-PARITY: model + folded + callgrind for default-calls1.
    ///
    /// Named gate consolidating `folded_stacks_default_calls1_real_render` and
    /// `callgrind_default_calls1_real_render` with exact A5/A7 model counts and
    /// format-appropriate export evidence. See
    /// `docs/schemas/export-semantic-parity-mvp-v0.md`.
    #[test]
    fn export_semantic_parity_default_calls1() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());

        // 1) Real model from fixture (not synthetic totals).
        let model = ProfileModel::from_path(&path).expect("ProfileModel::from_path");

        let leaf = model
            .sub_total("main::leaf")
            .expect("model must have main::leaf");
        let mid = model
            .sub_total("main::mid")
            .expect("model must have main::mid");
        assert_eq!(
            leaf.returns, 15,
            "main::leaf returns must be 15 (oracle A5 / aggregates.oracle.json)"
        );
        assert_eq!(
            mid.returns, 3,
            "main::mid returns must be 3 (oracle A5 / aggregates.oracle.json)"
        );

        let edge = model
            .call_edge("main::mid", "main::leaf")
            .expect("model must have mid→leaf call_edge (A7)");
        assert_eq!(
            edge.count, 15,
            "main::mid → main::leaf count must be 15 (oracle A7)"
        );

        // RUNTIME→mid is the mid returns relationship (count 3) on this fixture.
        if let Some(rt_mid) = model.call_edge("main::RUNTIME", "main::mid") {
            assert_eq!(
                rt_mid.count, 3,
                "main::RUNTIME → main::mid count must be 3 (oracle A7)"
            );
        }

        // 2) Folded stacks: contracted call-edge lines.
        let folded = render_folded_stacks(&model);
        assert!(!folded.is_empty(), "folded export must be non-empty");
        assert!(
            folded.contains("main::mid;main::leaf 15"),
            "folded must contain mid→leaf 15:\n{folded}"
        );
        assert!(
            folded.contains("main::leaf") && folded.contains("main::mid"),
            "folded must show leaf/mid presence:\n{folded}"
        );
        if model.call_edge("main::RUNTIME", "main::mid").is_some() {
            assert!(
                folded.contains("main::RUNTIME;main::mid 3"),
                "folded must contain RUNTIME→mid 3:\n{folded}"
            );
        }

        // 3) Callgrind: leaf/mid presence + calls=15 under mid→leaf + mid calls=3.
        let cg = render_callgrind(&model);
        assert!(!cg.is_empty(), "callgrind export must be non-empty");
        assert!(cg.contains("# callgrind format"), "callgrind header:\n{cg}");
        assert!(cg.contains("positions: line"), "callgrind positions:\n{cg}");
        assert!(
            cg.contains("main::leaf"),
            "callgrind must mention main::leaf:\n{cg}"
        );
        assert!(
            cg.contains("main::mid"),
            "callgrind must mention main::mid:\n{cg}"
        );
        assert!(
            cg.contains("fn=main::leaf") || cg.contains("cfn=main::leaf"),
            "callgrind fn/cfn leaf:\n{cg}"
        );
        assert!(
            cg.contains("fn=main::mid") || cg.contains("cfn=main::mid"),
            "callgrind fn/cfn mid:\n{cg}"
        );

        // mid→leaf: cfn=main::leaf with calls=15 (prefer under fn=main::mid block).
        assert!(
            cg.contains("cfn=main::leaf"),
            "callgrind must have cfn=main::leaf:\n{cg}"
        );
        assert!(
            cg.contains("calls=15 0") || cg.contains("calls=15\n") || cg.contains("calls=15 "),
            "callgrind must have calls=15 for mid→leaf:\n{cg}"
        );
        // Stronger: under fn=main::mid, next cfn=main::leaf should carry calls=15.
        // Line-anchored scan so we never match the "fn=..." suffix of "cfn=main::mid".
        let mid_fn_block = callgrind_fn_block(&cg, "main::mid");
        assert!(
            mid_fn_block.is_some(),
            "callgrind must have a fn=main::mid block:\n{cg}"
        );
        if let Some(window) = mid_fn_block {
            assert!(
                window.contains("cfn=main::leaf")
                    && (window.contains("calls=15 0")
                        || window.contains("calls=15\n")
                        || window.contains("calls=15 ")),
                "under fn=main::mid, expect cfn=main::leaf with calls=15:\n{window}"
            );
        }

        // mid returns relationship 3: cfn=main::mid with calls=3 (RUNTIME→mid).
        if model.call_edge("main::RUNTIME", "main::mid").is_some() {
            assert!(
                cg.contains("cfn=main::mid"),
                "callgrind must have cfn=main::mid:\n{cg}"
            );
            assert!(
                cg.contains("calls=3 0") || cg.contains("calls=3\n") || cg.contains("calls=3 "),
                "callgrind must have calls=3 for mid relationship:\n{cg}"
            );
            if let Some(window) = callgrind_fn_block(&cg, "main::RUNTIME") {
                assert!(
                    window.contains("cfn=main::mid")
                        && (window.contains("calls=3 0")
                            || window.contains("calls=3\n")
                            || window.contains("calls=3 ")),
                    "under fn=main::RUNTIME, expect cfn=main::mid with calls=3:\n{window}"
                );
            }
        }
    }

    /// Extract the `fn=<name>` block from a callgrind body (line-anchored).
    ///
    /// Does not match `cfn=<name>` (which contains the substring `fn=`).
    fn callgrind_fn_block<'a>(cg: &'a str, name: &str) -> Option<&'a str> {
        let needle = format!("fn={name}");
        let mut search_from = 0;
        while let Some(rel) = cg[search_from..].find(&needle) {
            let abs = search_from + rel;
            // Line-start: beginning of string or previous char is '\n'.
            if abs == 0 || cg.as_bytes().get(abs - 1) == Some(&b'\n') {
                let rest = &cg[abs..];
                // Block ends at the next line-start `fn=` (not `cfn=`).
                let end = rest
                    .char_indices()
                    .skip(1)
                    .find(|&(i, _)| {
                        rest[i..].starts_with("fn=")
                            && (i == 0 || rest.as_bytes().get(i - 1) == Some(&b'\n'))
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(rest.len());
                return Some(&rest[..end]);
            }
            search_from = abs + needle.len();
        }
        None
    }

    #[test]
    fn format_time_cell_html_only_seconds() {
        assert_eq!(format_time_cell(12340.0, None), "12340");
        assert_eq!(format_time_cell(12340.0, Some(0)), "12340");
        let scaled = format_time_cell(12340.0, Some(10_000_000));
        assert!(
            scaled.contains('s') && (scaled.contains('.') || scaled == "0s"),
            "seconds display: {scaled}"
        );
        assert_ne!(scaled, "12340");
        // Already-seconds NVs (SUB_CALLERS on default-calls1) must not collapse to 0s.
        let already_secs = format_time_cell(0.0000524, Some(10_000_000));
        assert_ne!(already_secs, "0s", "already-seconds NV: {already_secs}");
        assert!(already_secs.contains('s'), "{already_secs}");
        // Text/CSV path is unchanged integer ticks.
        assert_eq!(format_ticks(12340.0), "12340");
    }

    #[test]
    fn heat_class_quartile_names_only() {
        let scale = HeatScale::from_values(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(heat_class(4.0, &scale), "heat-hot");
        assert_eq!(heat_class(1.0, &scale), "heat-low");
        for v in [1.0, 2.0, 3.0, 4.0] {
            let c = heat_class(v, &scale);
            assert!(
                matches!(c, "heat-hot" | "heat-high" | "heat-mid" | "heat-low"),
                "{c}"
            );
            assert!(!c.starts_with('c'), "must not use oracle c0–c3: {c}");
        }
        let flat = HeatScale::from_values(&[5.0, 5.0, 5.0]);
        assert_eq!(heat_class(5.0, &flat), "");
        assert_eq!(heat_class(0.0, &scale), "");
    }

    #[test]
    fn sub_href_from_model_sub_def_default_calls1() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let model = ProfileModel::from_path(&path).expect("model");
        let d = model.sub_def("main::leaf").expect("main::leaf sub_def");
        let href = sub_href(&model, "main::leaf", true).expect("multi-file href");
        assert_eq!(href, format!("file-{}.html#L{}", d.fid, d.first_line));
        let single = sub_href(&model, "main::leaf", false).expect("single-file href");
        assert_eq!(single, format!("#L{}", d.first_line));
        assert!(sub_href(&model, "no::such::sub", true).is_none());
        let html = render_html_site(&model, "default-calls1.out").index_html;
        assert!(
            html.contains(&format!("href=\"{href}\"")),
            "index must link leaf via model sub_def:\n{html}"
        );
        assert!(
            !html.contains("jquery") && !html.contains("tablesorter"),
            "must not vendor jquery/tablesorter"
        );
    }

    #[test]
    fn html_source_union_line_totals_without_source_text() {
        let mut model = ProfileModel::default();
        model.files.insert(1, "workload.pl".to_owned());
        model.line_totals.insert(
            (1, 42),
            nytprof_model::LineTotal {
                calls: 7,
                ticks: 100,
            },
        );
        let html = render_html_summary(&model, "union.out");
        assert!(
            html.contains("id=\"L42\""),
            "union must emit a source row for line_totals-only line:\n{html}"
        );
        assert!(
            html.contains("—"),
            "missing source text must be em dash:\n{html}"
        );
        assert!(
            html.contains(">7<"),
            "calls from line_totals must appear:\n{html}"
        );
        assert!(
            !html.contains("id=\"L42\" class=\"heat-"),
            "single unused/no-spread source row must not paint row heat:\n{html}"
        );
    }

    #[test]
    fn shared_style_and_sort_js_contract() {
        assert!(SHARED_STYLE_CSS.contains("heat-hot"));
        assert!(SHARED_STYLE_CSS.contains("heat-high"));
        assert!(SHARED_STYLE_CSS.contains("heat-mid"));
        assert!(SHARED_STYLE_CSS.contains("heat-low"));
        assert!(SHARED_STYLE_CSS.contains("th[data-sort]"));
        assert!(SHARED_STYLE_CSS.contains("th.sort-asc::after"));
        assert!(SHARED_STYLE_CSS.contains("th.sort-desc::after"));
        assert!(
            !SHARED_STYLE_CSS.contains(".c0") && !SHARED_STYLE_CSS.contains("c0{"),
            "must not emit oracle .c0 class selectors (CSS vars --nyt-c0 are ok)"
        );
        assert!(SHARED_SORT_JS.contains("nytprofSortInit"));
        assert!(SHARED_SORT_JS.contains("data-sort"));
        let lower = SHARED_SORT_JS.to_ascii_lowercase();
        assert!(!lower.contains("jquery") && !lower.contains("tablesorter"));
        assert!(!SHARED_SORT_JS.contains("innerHTML"));
        assert!(SHARED_SORT_JS.contains("data-sort-default"));
        assert_eq!(SORT_JS_FILENAME, "nytprof-sort.js");
    }

    #[test]
    fn format_compact_secs_fmt_time_branches() {
        assert_eq!(format_compact_secs(0.0), "0s");
        assert_eq!(format_compact_secs(500e-9), "500ns");
        assert_eq!(format_compact_secs(49e-6), "49µs");
        assert_eq!(format_compact_secs(0.129), "129ms");
        assert_eq!(format_compact_secs(4.72), "4.72s");
        assert_eq!(format_compact_secs(150.0), "150s");
        assert_eq!(format_compact_secs(-0.129), "-129ms");
    }

    #[test]
    fn primary_workload_fid_scanner_shaped_not_warnings() {
        let mut model = ProfileModel::default();
        model
            .files
            .insert(1, "/usr/share/perl5/warnings.pm".to_owned());
        model
            .files
            .insert(3, "/tmp/lab/minute_text_scanner.pl".to_owned());
        model
            .source_lines
            .insert((1, 1), "package warnings;".to_owned());
        model
            .source_lines
            .insert((3, 1), "sub tokenize {".to_owned());
        model.line_totals.insert(
            (1, 1),
            nytprof_model::LineTotal {
                calls: 2,
                ticks: 50,
            },
        );
        model.line_totals.insert(
            (3, 20),
            nytprof_model::LineTotal {
                calls: 400,
                ticks: 9_000,
            },
        );
        assert_eq!(
            primary_workload_fid(&model),
            3,
            "hottest non-@INC .pl must win over warnings.pm"
        );
        model.attributes.insert(
            "application".to_owned(),
            "/tmp/lab/minute_text_scanner.pl".to_owned(),
        );
        assert_eq!(
            primary_workload_fid(&model),
            3,
            "application basename must select the scanner"
        );
    }

    #[test]
    fn primary_workload_fid_default_calls1_still_workload() {
        let path = fixture_out("default-calls1");
        let model = ProfileModel::from_path(&path).expect("model");
        let fid = primary_workload_fid(&model);
        let base = model.fid_basename(fid).unwrap_or("");
        assert!(
            base.contains("workload"),
            "default-calls1 primary fid {fid} basename {base}"
        );
    }

    #[test]
    fn push_page_chrome_escapes_title_and_optional_back() {
        let mut with_back = String::new();
        push_page_chrome(
            &mut with_back,
            "A <title>",
            "For app",
            true,
        );
        assert!(with_back.contains("class=\"header\""));
        assert!(with_back.contains("class=\"siteTitle\""));
        assert!(with_back.contains("A &lt;title&gt;"));
        assert!(
            with_back.contains("← Index") || with_back.contains("&larr; Index"),
            "{with_back}"
        );
        assert!(with_back.contains("href=\"index.html\""));
        let mut index = String::new();
        push_page_chrome(&mut index, "Performance Profile Index", "For x", false);
        assert!(index.contains("Performance Profile Index"));
        assert!(!index.contains("header_back"));
    }
}

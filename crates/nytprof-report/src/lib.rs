//! Native report rendering (text summary + CSV tabular + HTML MVP + exports).
//!
//! Content requirements: `docs/schemas/aggregate-comparison-v0.md`,
//! `docs/schemas/html-report-mvp-v0.md`,
//! `docs/schemas/html-multifile-mvp-v0.md`,
//! `docs/schemas/html-per-file-mvp-v0.md` (A4b block_line_totals),
//! `docs/schemas/html-outdir-safety-mvp-v0.md`,
//! `docs/schemas/export-formats-mvp-v0.md`,
//! `docs/schemas/export-semantic-parity-mvp-v0.md`,
//! `docs/schemas/verify-cli-mvp-v0.md`,
//! `docs/schemas/report-semantic-parity-mvp-v0.md`,
//! `docs/schemas/blocks-semantic-parity-mvp-v0.md`.

use std::collections::BTreeSet;
use std::error::Error;
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
        subs.sort_by(|(a, _), (b, _)| a.cmp(b));
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
    rows.sort_by(|(a, _), (b, _)| a.cmp(b));
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

/// Multi-file HTML report site (index + per-fid source pages).
///
/// See `docs/schemas/html-multifile-mvp-v0.md` and
/// `docs/schemas/html-per-file-mvp-v0.md`.
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
}

const HTML_STYLE: &str = "\
body{font-family:system-ui,sans-serif;margin:1.5rem;line-height:1.4}\n\
table{border-collapse:collapse;margin:0.75rem 0}\n\
th,td{border:1px solid #ccc;padding:0.25rem 0.5rem;text-align:left}\n\
th{background:#f0f0f0}\n\
td.num{text-align:right;font-variant-numeric:tabular-nums}\n\
pre,code{font-family:ui-monospace,monospace}\n\
.src-line{white-space:pre}\n\
h1,h2{margin-top:1.25rem}\n\
";

/// Self-contained HTML summary report (MVP; see `docs/schemas/html-report-mvp-v0.md`).
///
/// Includes profile path, event counts, subroutine table from `sub_return_totals`,
/// call edges, exclusive-time ranking, and a source section for the primary workload fid.
pub fn render_html_summary(model: &ProfileModel, profile_path: &str) -> String {
    let title = html_report_title(profile_path);
    let primary_fid = primary_workload_fid(model);
    let mut out = String::with_capacity(8192);
    push_html_doc_start(&mut out, &title);
    out.push_str(&format!("<h1>{}</h1>\n", escape_html(&title)));
    push_profile_path(&mut out, profile_path);
    push_event_counts(&mut out, model);
    push_subs_table(&mut out, model);
    push_sub_defs_table(&mut out, model);
    push_call_edges_table(&mut out, model);
    push_top_exclusive_table(&mut out, model);
    push_source_heading(&mut out, model, primary_fid);
    push_source_table(&mut out, model, primary_fid);
    // A4b: all block_line_totals when present (blocks fixtures).
    push_block_line_totals_table(&mut out, model, None);
    out.push_str("</body>\n</html>\n");
    out
}

/// Multi-file HTML site: summary index + one page per eligible fid.
///
/// Eligible fids are those in [`ProfileModel::files`] that have at least one
/// `source_lines`, `line_totals`, or `block_line_totals` entry.
///
/// Site contents:
/// - `index.html` — summary; relative links to every `file-<fid>.html` and to
///   [`HtmlSite::source_filename`] (`source.html`) as a primary alias
/// - `file-<fid>.html` — source + A4 (and A4b when present) for that fid
/// - `source.html` — copy of the primary workload file page (back-compat)
pub fn render_html_site(model: &ProfileModel, profile_path: &str) -> HtmlSite {
    let source_filename = "source.html".to_owned();
    let title = html_report_title(profile_path);
    let primary_fid = primary_workload_fid(model);
    let eligible = eligible_source_fids(model);
    let profile_base = Path::new(profile_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(profile_path);

    // --- per-fid file pages ---
    let mut file_pages: Vec<(String, String)> = Vec::with_capacity(eligible.len());
    for fid in &eligible {
        let filename = file_page_filename(*fid);
        let page = render_file_page(model, *fid, profile_base);
        file_pages.push((filename, page));
    }
    // Primary alias (`source.html`): copy of the primary workload file page.
    // If primary is not eligible (unusual), still render a page for back-compat.
    let primary_name = file_page_filename(primary_fid);
    let source_html = file_pages
        .iter()
        .find(|(name, _)| name == &primary_name)
        .map(|(_, html)| html.clone())
        .unwrap_or_else(|| render_file_page(model, primary_fid, profile_base));

    // --- index.html ---
    let mut index = String::with_capacity(4096);
    push_html_doc_start(&mut index, &title);
    index.push_str(&format!("<h1>{}</h1>\n", escape_html(&title)));
    push_profile_path(&mut index, profile_path);
    push_event_counts(&mut index, model);
    push_subs_table(&mut index, model);
    push_sub_defs_table(&mut index, model);
    push_call_edges_table(&mut index, model);
    push_top_exclusive_table(&mut index, model);
    push_source_file_links(
        &mut index,
        model,
        &eligible,
        primary_fid,
        &source_filename,
    );
    index.push_str("</body>\n</html>\n");

    HtmlSite {
        index_html: index,
        source_html,
        source_filename,
        file_pages,
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
/// Writes `index.html`, every `file-<fid>.html` from [`HtmlSite::file_pages`],
/// and `source.html` (primary alias).
///
/// Returns the rendered [`HtmlSite`] so callers can list filenames written.
pub fn write_html_site(
    model: &ProfileModel,
    profile_path: &str,
    out_dir: &Path,
) -> io::Result<HtmlSite> {
    validate_html_out_dir(out_dir)?;
    let site = render_html_site(model, profile_path);
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
        for (filename, html) in &site.file_pages {
            fs::write(temp_dir.join(filename), html.as_bytes())?;
        }
        fs::write(
            temp_dir.join(&site.source_filename),
            site.source_html.as_bytes(),
        )?;
        atomic_replace_dir(&temp_dir, out_dir)
    })();

    if write_result.is_err() {
        // Temp may already have been renamed away on partial publish failure;
        // ignore cleanup errors.
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

/// Render one per-fid HTML page body (full document).
fn render_file_page(model: &ProfileModel, fid: u32, profile_base: &str) -> String {
    let basename = model
        .fid_basename(fid)
        .map(|s| s.to_owned())
        .unwrap_or_else(|| source_file_label(model, fid));
    let src_title = format!("Source — {basename} (fid {fid}) — {profile_base}");
    let mut page = String::with_capacity(4096);
    push_html_doc_start(&mut page, &src_title);
    page.push_str(&format!("<h1>{}</h1>\n", escape_html(&src_title)));
    page.push_str("<p><a href=\"index.html\">← Back to index</a></p>\n");
    push_source_heading(&mut page, model, fid);
    push_source_table(&mut page, model, fid);
    // A4b: block_line_totals for this fid when present (blocks fixtures).
    push_block_line_totals_table(&mut page, model, Some(fid));
    page.push_str("</body>\n</html>\n");
    page
}

/// Index section: relative links to every `file-*.html` plus primary `source.html` alias.
fn push_source_file_links(
    out: &mut String,
    model: &ProfileModel,
    eligible: &[u32],
    primary_fid: u32,
    source_filename: &str,
) {
    out.push_str("<h2>Source files</h2>\n");
    out.push_str("<ul class=\"source-files\">\n");
    for fid in eligible {
        let href = file_page_filename(*fid);
        let basename = model
            .fid_basename(*fid)
            .map(|s| s.to_owned())
            .unwrap_or_else(|| source_file_label(model, *fid));
        out.push_str(&format!(
            "<li><a href=\"{}\">{}</a> (fid {})</li>\n",
            escape_html(&href),
            escape_html(&basename),
            fid
        ));
    }
    out.push_str("</ul>\n");
    // Back-compat: keep an explicit href="source.html" to the primary page.
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

fn push_html_doc_start(out: &mut String, title: &str) {
    out.push_str("<!DOCTYPE html>\n");
    out.push_str("<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n");
    out.push_str(&format!("<title>{}</title>\n", escape_html(title)));
    out.push_str("<style>\n");
    out.push_str(HTML_STYLE);
    out.push_str("</style>\n");
    out.push_str("</head>\n<body>\n");
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
    out.push_str("</ul>\n");
}

fn push_subs_table(out: &mut String, model: &ProfileModel) {
    out.push_str("<h2>Subroutines</h2>\n");
    out.push_str(
        "<table class=\"subs\">\n<thead><tr>\
         <th>name</th><th>returns</th><th>incl</th><th>excl</th>\
         </tr></thead>\n<tbody>\n",
    );
    let mut subs: Vec<_> = model.sub_return_totals.iter().collect();
    subs.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (name, t) in subs {
        out.push_str(&format!(
            "<tr><td>{}</td><td class=\"num\">{}</td>\
             <td class=\"num\">{}</td><td class=\"num\">{}</td></tr>\n",
            escape_html(name),
            t.returns,
            format_ticks(t.incl),
            format_ticks(t.excl),
        ));
    }
    out.push_str("</tbody>\n</table>\n");
}

/// Optional A9 `sub_defs` table (skipped when empty).
fn push_sub_defs_table(out: &mut String, model: &ProfileModel) {
    if model.sub_defs.is_empty() {
        return;
    }
    out.push_str("<h2>Subroutine definitions</h2>\n");
    out.push_str(
        "<table class=\"sub-defs\">\n<thead><tr>\
         <th>name</th><th>fid</th><th>first</th><th>last</th>\
         </tr></thead>\n<tbody>\n",
    );
    let mut defs: Vec<_> = model.sub_defs.iter().collect();
    defs.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (name, d) in defs {
        out.push_str(&format!(
            "<tr><td>{}</td><td class=\"num\">{}</td>\
             <td class=\"num\">{}</td><td class=\"num\">{}</td></tr>\n",
            escape_html(name),
            d.fid,
            d.first_line,
            d.last_line,
        ));
    }
    out.push_str("</tbody>\n</table>\n");
}

fn push_call_edges_table(out: &mut String, model: &ProfileModel) {
    // Call edges (A7): count desc, then caller, then called.
    out.push_str("<h2>Call edges</h2>\n");
    out.push_str(
        "<table class=\"call-edges\">\n<thead><tr>\
         <th>caller</th><th>called</th><th>count</th><th>incl</th><th>excl</th>\
         </tr></thead>\n<tbody>\n",
    );
    let mut edges: Vec<_> = model.call_edges.iter().collect();
    edges.sort_by(|((c1, d1), e1), ((c2, d2), e2)| {
        e2.count
            .cmp(&e1.count)
            .then_with(|| c1.cmp(c2))
            .then_with(|| d1.cmp(d2))
    });
    for ((caller, called), e) in edges {
        out.push_str(&format!(
            "<tr><td>{}</td><td>{}</td>\
             <td class=\"num\">{}</td>\
             <td class=\"num\">{}</td><td class=\"num\">{}</td></tr>\n",
            escape_html(caller),
            escape_html(called),
            e.count,
            format_ticks(e.incl),
            format_ticks(e.excl),
        ));
    }
    out.push_str("</tbody>\n</table>\n");
}

fn push_top_exclusive_table(out: &mut String, model: &ProfileModel) {
    // Exclusive-time ranking: excl desc, then name for stability.
    out.push_str("<h2>Top exclusive</h2>\n");
    out.push_str(
        "<table class=\"top-exclusive\">\n<thead><tr>\
         <th>name</th><th>excl</th><th>returns</th>\
         </tr></thead>\n<tbody>\n",
    );
    let mut by_excl: Vec<_> = model.sub_return_totals.iter().collect();
    by_excl.sort_by(|(n1, t1), (n2, t2)| {
        t2.excl
            .total_cmp(&t1.excl)
            .then_with(|| n1.cmp(n2))
    });
    for (name, t) in by_excl {
        out.push_str(&format!(
            "<tr><td>{}</td>\
             <td class=\"num\">{}</td><td class=\"num\">{}</td></tr>\n",
            escape_html(name),
            format_ticks(t.excl),
            t.returns,
        ));
    }
    out.push_str("</tbody>\n</table>\n");
}

fn push_source_heading(out: &mut String, model: &ProfileModel, primary_fid: u32) {
    let file_label = source_file_label(model, primary_fid);
    out.push_str(&format!(
        "<h2>Source — {} (fid {})</h2>\n",
        escape_html(&file_label),
        primary_fid
    ));
}

fn push_source_table(out: &mut String, model: &ProfileModel, primary_fid: u32) {
    out.push_str(
        "<table class=\"source\">\n<thead><tr>\
         <th>line</th><th>calls</th><th>ticks</th><th>source</th>\
         </tr></thead>\n<tbody>\n",
    );

    let mut src_rows: Vec<(u32, &String)> = model
        .source_lines
        .iter()
        .filter(|((fid, _), _)| *fid == primary_fid)
        .map(|((_, line), text)| (*line, text))
        .collect();
    src_rows.sort_by_key(|(line, _)| *line);

    for (line, text) in src_rows {
        let (calls, ticks) = model
            .line_totals
            .get(&(primary_fid, line))
            .map(|t| (t.calls.to_string(), t.ticks.to_string()))
            .unwrap_or_else(|| ("—".to_owned(), "—".to_owned()));
        // SRC_LINE text often includes a trailing newline; strip for cell display.
        let display = text.trim_end_matches(['\n', '\r']);
        out.push_str(&format!(
            "<tr><td class=\"num\">{line}</td>\
             <td class=\"num\">{calls}</td>\
             <td class=\"num\">{ticks}</td>\
             <td class=\"src-line\"><code>{}</code></td></tr>\n",
            escape_html(display),
        ));
    }
    out.push_str("</tbody>\n</table>\n");
}

/// A4b — dedicated **Block line totals** table from `model.block_line_totals`.
///
/// Skipped when empty (or when `only_fid` filters out every entry). Rows sorted
/// by `(fid, block_line)`. Columns: fid, block_line, calls, ticks.
fn push_block_line_totals_table(
    out: &mut String,
    model: &ProfileModel,
    only_fid: Option<u32>,
) {
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
    let mut rows: Vec<((&str, &str), u64)> = model
        .call_edges
        .iter()
        .filter(|(_, e)| e.count > 0)
        .map(|((caller, called), e)| ((caller.as_str(), called.as_str()), e.count))
        .collect();
    rows.sort_by(|((c1, d1), _), ((c2, d2), _)| c1.cmp(c2).then_with(|| d1.cmp(d2)));

    let mut out = String::with_capacity(rows.len().saturating_mul(48));
    for ((caller, called), count) in rows {
        out.push_str(caller);
        out.push(';');
        out.push_str(called);
        out.push(' ');
        out.push_str(&count.to_string());
        out.push('\n');
    }
    out
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

    let mut out = String::with_capacity(names.len().saturating_mul(64) + edges.len().saturating_mul(48));
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

/// Choose the primary source file id for the HTML source section.
///
/// Prefers the lowest fid whose path/name contains `"workload"` (case-sensitive)
/// or whose basename is `workload.pl`. Falls back to the minimum fid that has
/// any `source_lines` entry; finally `1`.
fn primary_workload_fid(model: &ProfileModel) -> u32 {
    let mut workload_fids: Vec<u32> = model
        .files
        .iter()
        .filter(|(_, name)| {
            let base = name
                .rsplit(['/', '\\'])
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or(name);
            name.contains("workload") || base == "workload.pl"
        })
        .map(|(fid, _)| *fid)
        .collect();
    if !workload_fids.is_empty() {
        workload_fids.sort_unstable();
        return workload_fids[0];
    }

    let mut source_fids: Vec<u32> = model
        .source_lines
        .keys()
        .map(|(fid, _)| *fid)
        .collect();
    source_fids.sort_unstable();
    source_fids.dedup();
    source_fids.first().copied().unwrap_or(1)
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
        assert_eq!(escape_html("a < b & c > \"d\""), "a &lt; b &amp; c &gt; &quot;d&quot;");
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
        assert!(
            html.contains("main::leaf"),
            "must list main::leaf:\n{html}"
        );
        assert!(
            html.contains("main::mid"),
            "must list main::mid:\n{html}"
        );

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
        let lower = index.to_ascii_lowercase();
        assert!(
            lower.contains("<!doctype html"),
            "index must have doctype:\n{}",
            &index[..index.len().min(200)]
        );
        assert!(index.contains("<title>"), "index title");
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
            source.to_ascii_lowercase().contains("workload")
                || source.contains("fid "),
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
        assert!(index_path.is_file(), "index.html missing at {}", index_path.display());
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
        let index = fs::read_to_string(out.join("index.html")).expect("index");
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

        let err = write_html_site(&model, &path_str, &out).expect_err("must fail when out_dir is file");
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

        let err = write_html_site(&model, &path_str, &out).expect_err("must fail when parent is file");
        // Linux typically returns ENOTDIR (20) when mkdir under a file path.
        assert!(
            err.raw_os_error() == Some(20) // ENOTDIR
                || err.kind() == io::ErrorKind::AlreadyExists
                || err.kind() == io::ErrorKind::PermissionDenied
                || err.kind() == io::ErrorKind::Other,
            "expected create_dir failure when parent is a file, got {err:?}"
        );
        assert!(!out.exists(), "must not create final site when parent is unusable");
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

        let err =
            write_html_site(&model, &path_str, &out).expect_err("must reject '..' component");
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
        // Line 5 row: line-number cell then calls from line_totals (A4).
        // Prefer the full row shape so we do not match an earlier calls-cell "5".
        let line5_row_prefix = format!("<td class=\"num\">5</td><td class=\"num\">{expected_calls}</td>");
        assert!(
            src_slice.contains(&line5_row_prefix)
                || src_slice.contains(&format!(
                    "<td class=\"num\">5</td>\n             <td class=\"num\">{expected_calls}</td>"
                )),
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

        // Sub table cells: name then returns num cell.
        let leaf_returns_cell = format!(
            "<td>{}</td><td class=\"num\">{}</td>",
            "main::leaf", leaf.returns
        );
        let mid_returns_cell =
            format!("<td>{}</td><td class=\"num\">{}</td>", "main::mid", mid.returns);
        assert!(
            html.contains(&leaf_returns_cell) || html.contains(&format!("main::leaf</td><td class=\"num\">{}</td>", leaf.returns)),
            "subs table must show leaf returns={}:\n{html}",
            leaf.returns
        );
        assert!(
            html.contains(&mid_returns_cell)
                || html.contains(&format!("main::mid</td><td class=\"num\">{}</td>", mid.returns)),
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
        let edge_row = format!(
            "<td>main::mid</td><td>main::leaf</td>\
             <td class=\"num\">{}</td>",
            edge.count
        );
        assert!(
            edges_slice.contains(&edge_row)
                || edges_slice.contains(&format!(
                    "main::mid</td><td>main::leaf</td><td class=\"num\">{}</td>",
                    edge.count
                )),
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
        let line5_row_prefix =
            format!("<td class=\"num\">5</td><td class=\"num\">{}</td>", lt.calls);
        assert!(
            src_slice.contains(&line5_row_prefix)
                || src_slice.contains(&format!(
                    "<td class=\"num\">5</td>\n             <td class=\"num\">{}</td>",
                    lt.calls
                )),
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
        assert!(
            bytes.len() > 500,
            "fixture must be larger than 500 bytes"
        );
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

    #[test]
    fn callgrind_default_calls1_real_render() {
        let path = fixture_out("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let model = ProfileModel::from_path(&path).expect("build model");
        let cg = render_callgrind(&model);

        assert!(!cg.is_empty(), "callgrind export must be non-empty");
        assert!(
            cg.contains("# callgrind format"),
            "header comment:\n{cg}"
        );
        assert!(
            cg.contains("positions: line"),
            "positions header:\n{cg}"
        );
        assert!(
            cg.contains("events: Ticks") || cg.contains("Events: Calls"),
            "events header:\n{cg}"
        );
        assert!(
            cg.contains("main::leaf"),
            "must mention main::leaf:\n{cg}"
        );
        assert!(
            cg.contains("main::mid"),
            "must mention main::mid:\n{cg}"
        );
        // mid→leaf call count 15 from call_edges.
        assert!(
            cg.contains("15"),
            "must include mid→leaf count 15:\n{cg}"
        );
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
        assert!(
            cg.contains("# callgrind format"),
            "callgrind header:\n{cg}"
        );
        assert!(
            cg.contains("positions: line"),
            "callgrind positions:\n{cg}"
        );
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
}

//! CLI for NYTProf modernisation tools.
//!
//! Subcommands:
//! - `dump <file>`   — canonical JSONL event dump (default if a bare path is given)
//! - `report <file>` — text summary report from the compact model
//! - `report --json <file>` — structured JSON aggregates (NATIVE-AGG-JSON)
//! - `summary <file>` — alias for `report` (also accepts `--json`)
//! - `aggregates <file>` — alias for `report --json`
//! - `csv <file>`    — dual-section CSV (subs + call edges); optional `--subs` / `--edges`
//! - `html <file>`   — HTML summary (stdout, `-o path.html`, or `--out-dir DIR` multi-file)
//! - `folded <file>` — folded-stack lines (flamegraph input)
//! - `callgrind <file>` / `cg <file>` — Callgrind-style text export
//! - `verify <file>` / `inspect <file>` — decode + model load; short OK summary
//! - `capability` / `selftest` / `capabilities` — native offline capability self-test
//!
//! Global options:
//! - `--engine=native|legacy|auto` (or env `NYTPROF_ENGINE`)
//!
//! Schema (dump): `docs/schemas/canonical-event-dump-v0.md`
//! Aggregates: `docs/schemas/aggregate-comparison-v0.md`
//! Native aggregates JSON: `docs/schemas/native-aggregates-json-mvp-v0.md`
//! HTML MVP: `docs/schemas/html-report-mvp-v0.md`
//! HTML multi-file: `docs/schemas/html-multifile-mvp-v0.md`
//! HTML out-dir safety: `docs/schemas/html-outdir-safety-mvp-v0.md`
//! HTML per-file: `docs/schemas/html-per-file-mvp-v0.md`
//! Export MVP: `docs/schemas/export-formats-mvp-v0.md`
//! Verify MVP: `docs/schemas/verify-cli-mvp-v0.md`
//! Engine selection: `docs/schemas/engine-selection-mvp-v0.md`
//! Capability self-test: `docs/schemas/capability-selftest-mvp-v0.md`

mod engine;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use nytprof_format_v5::decode_path;
use nytprof_model::ProfileModel;
use nytprof_report::{
    render_callgrind, render_csv_report, render_edges_csv, render_folded_stacks,
    render_html_summary, render_subs_csv, render_summary_text, require_complete_stream,
    verify_profile, write_html_site,
};
use nytprof_types::{tags, Event};
use serde_json::{json, Value};

use engine::{legacy_not_wired_message, peel_engine_flag, resolve_engine, Engine};

fn main() {
    let raw_args: Vec<String> = env::args().skip(1).collect();

    let (engine_cli, args) = match peel_engine_flag(&raw_args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("nytprof-cli: {e}");
            process::exit(1);
        }
    };

    let engine_env = env::var("NYTPROF_ENGINE").ok();
    let engine = match resolve_engine(engine_cli.as_deref(), engine_env.as_deref()) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("nytprof-cli: {e}");
            process::exit(1);
        }
    };

    // Help / no-args: always print usage (even if engine=legacy).
    let first = args.first().map(|s| s.as_str());
    if first.is_none() || matches!(first, Some("-h" | "--help" | "help")) {
        print_usage();
        process::exit(0);
    }

    // Capability self-test reports *this* binary's native offline tools.
    // Runs regardless of --engine (including legacy) so operators can probe
    // the shipped native artifact without selecting a report backend.
    if matches!(first, Some("capability" | "selftest" | "capabilities")) {
        if let Err(e) = cmd_capability(&args[1..]) {
            eprintln!("nytprof-cli: {e}");
            process::exit(1);
        }
        return;
    }

    if engine == Engine::Legacy {
        eprintln!("nytprof-cli: {}", legacy_not_wired_message());
        process::exit(2);
    }

    // native / auto → existing Rust paths
    if let Err(e) = run(&args) {
        eprintln!("nytprof-cli: {e}");
        process::exit(1);
    }
}

fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = args.iter().cloned();
    let first = args
        .next()
        .ok_or("Usage: nytprof-cli [--engine=native|legacy|auto] <dump|report|summary|aggregates|csv|html|folded|callgrind|cg|verify|inspect|capability|selftest> ...\n       nytprof-cli <profile.out>   # dump (back-compat)")?;

    match first.as_str() {
        "dump" => {
            let path = args
                .next()
                .ok_or("Usage: nytprof-cli dump <profile.out>")?;
            cmd_dump(&path)
        }
        "report" | "summary" => {
            let rest: Vec<String> = args.collect();
            cmd_report(&rest)
        }
        "aggregates" | "agg" => {
            // Always JSON aggregates (NATIVE-AGG-JSON).
            let rest: Vec<String> = args.collect();
            cmd_aggregates(&rest)
        }
        "csv" => {
            let rest: Vec<String> = args.collect();
            cmd_csv(&rest)
        }
        "html" => {
            let rest: Vec<String> = args.collect();
            cmd_html(&rest)
        }
        "folded" => {
            let path = args
                .next()
                .ok_or("Usage: nytprof-cli folded <profile.out>")?;
            cmd_folded(&path)
        }
        "callgrind" | "cg" => {
            let path = args
                .next()
                .ok_or("Usage: nytprof-cli callgrind <profile.out>")?;
            cmd_callgrind(&path)
        }
        "verify" | "inspect" => {
            let path = args
                .next()
                .ok_or("Usage: nytprof-cli verify <profile.out>")?;
            cmd_verify(&path)
        }
        "capability" | "selftest" | "capabilities" => {
            let rest: Vec<String> = args.collect();
            cmd_capability(&rest)
        }
        "-h" | "--help" | "help" => {
            print_usage();
            Ok(())
        }
        // Bare path → dump (back-compat with nytprof-dump / earlier CLI).
        path if looks_like_path(&first) => cmd_dump(path),
        other => Err(format!(
            "unknown subcommand '{other}'\nUsage: nytprof-cli [--engine=native|legacy|auto] <dump|report|summary|aggregates|csv|html|folded|callgrind|cg|verify|inspect|capability|selftest> ..."
        )
        .into()),
    }
}

fn looks_like_path(s: &str) -> bool {
    s.contains('/')
        || s.contains('\\')
        || s.ends_with(".out")
        || s.starts_with('.')
        || std::path::Path::new(s).exists()
}

fn print_usage() {
    eprintln!(
        "Usage:\n  \
         nytprof-cli [--engine=native|legacy|auto] <subcommand> ...\n  \
         nytprof-cli dump <profile.out>          Canonical JSONL event dump\n  \
         nytprof-cli report <profile.out>        Text summary report\n  \
         nytprof-cli report --json <profile.out> Structured JSON aggregates\n  \
         nytprof-cli summary <profile.out>       Alias for report\n  \
         nytprof-cli aggregates <profile.out>    Alias for report --json\n  \
         nytprof-cli csv <profile.out>           Dual-section CSV (subs + edges)\n  \
         nytprof-cli csv --subs <profile.out>    Subroutines CSV only\n  \
         nytprof-cli csv --edges <profile.out>   Call-edges CSV only\n  \
         nytprof-cli html <profile.out>          HTML summary to stdout\n  \
         nytprof-cli html <profile.out> -o path  HTML summary to file\n  \
         nytprof-cli html <profile.out> --out-dir DIR  Multi-file HTML site\n  \
         nytprof-cli folded <profile.out>        Folded-stack lines (flamegraph)\n  \
         nytprof-cli callgrind <profile.out>     Callgrind-style text export\n  \
         nytprof-cli cg <profile.out>            Alias for callgrind\n  \
         nytprof-cli verify <profile.out>        Decode + model; short OK summary\n  \
         nytprof-cli inspect <profile.out>       Alias for verify\n  \
         nytprof-cli capability                  Native offline capability self-test\n  \
         nytprof-cli capability --json           Capability self-test as JSON object\n  \
         nytprof-cli selftest                    Alias for capability\n  \
         nytprof-cli capabilities                Alias for capability\n  \
         nytprof-cli <profile.out>               Dump (back-compat)\n\n\
         Global options:\n  \
         --engine=native|legacy|auto   Backend selection (default: native)\n  \
         NYTPROF_ENGINE                Same values when --engine is omitted\n\n\
         Report / aggregates options:\n  \
         --json / --format=json        Structured aggregates JSON (NATIVE-AGG-JSON)\n\n\
         Capability options:\n  \
         --json / --format=json        Machine-readable JSON (CAPABILITY-JSON-MVP)\n  \
         --profile PATH / -p PATH      Force golden profile probe\n\n\
         Engines:\n  \
         native   Rust decode/model/report path (default)\n  \
         auto     Same as native until a Perl facade exists\n  \
         legacy   Pinned oracle under baseline/6.15 (not wired yet; exits 2)"
    );
}

/// Default golden probe relative to repo root / CWD.
const DEFAULT_CAPABILITY_FIXTURE: &str = "fixtures/v5/default-calls1/nytprof.out";

/// Native offline capability self-test (CAPABILITY-SELFTEST / CAPABILITY-JSON-MVP).
///
/// Default (human) output — greppable lines:
/// ```text
/// OK: native capability self-test
/// decode: yes
/// report: yes
/// verify: yes
/// profile_ok: <path>   # when a golden fixture is found and verify succeeds
/// profile_ok: skip     # when no probe path is available
/// ```
///
/// JSON mode (`--json` / `--format=json` / `--format json`) — single object on stdout:
/// ```json
/// {"ok":true,"decode":true,"report":true,"verify":true,"profile_ok":"<path>|null"}
/// ```
///
/// Exit 0 when claimed tools work. Non-zero if a found probe fails verify
/// (fail closed — never claim present tools that cannot load a real profile).
///
/// Optional args: bare path or `--profile <path>` / `-p <path>` to force a probe.
fn cmd_capability(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_usage();
        return Ok(());
    }
    let opts = parse_capability_args(args)?;

    // This binary *is* the native offline CLI: decode / report / verify are linked.
    let profile_ok: Option<String> = match resolve_capability_probe(opts.profile.as_deref()) {
        Some(path) => {
            // Real exercise of decode+model+verify when a fixture is present.
            match verify_profile(&path) {
                Ok(summary) => {
                    if !summary.lines().any(|l| l.starts_with("OK:")) {
                        return Err(format!(
                            "capability self-test: verify of {} did not produce OK: summary",
                            path.display()
                        )
                        .into());
                    }
                    Some(path.display().to_string())
                }
                Err(e) => {
                    return Err(format!(
                        "capability self-test: verify failed for {}: {e}",
                        path.display()
                    )
                    .into());
                }
            }
        }
        None => None,
    };

    if opts.json {
        let obj = json!({
            "ok": true,
            "decode": true,
            "report": true,
            "verify": true,
            "profile_ok": profile_ok,
        });
        write_stdout_text(&serde_json::to_string(&obj)?)
    } else {
        let mut lines: Vec<String> = vec![
            "OK: native capability self-test".to_string(),
            "decode: yes".to_string(),
            "report: yes".to_string(),
            "verify: yes".to_string(),
        ];
        match profile_ok {
            Some(p) => lines.push(format!("profile_ok: {p}")),
            None => lines.push("profile_ok: skip".to_string()),
        }
        write_stdout_text(&lines.join("\n"))
    }
}

struct CapabilityOpts {
    profile: Option<String>,
    json: bool,
}

fn parse_capability_args(args: &[String]) -> Result<CapabilityOpts, Box<dyn std::error::Error>> {
    let usage = "Usage: nytprof-cli capability [--json | --format=json] [--profile PATH]";
    let mut profile: Option<String> = None;
    let mut json = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => {
                json = true;
            }
            "--format" => {
                i += 1;
                let fmt = args
                    .get(i)
                    .ok_or(format!("{usage} (--format requires a value)"))?;
                match fmt.to_ascii_lowercase().as_str() {
                    "json" => json = true,
                    other => {
                        return Err(format!(
                            "unknown capability format '{other}' (supported: json)\n{usage}"
                        )
                        .into());
                    }
                }
            }
            flag if flag.starts_with("--format=") => {
                let fmt = flag["--format=".len()..].to_ascii_lowercase();
                match fmt.as_str() {
                    "json" => json = true,
                    other => {
                        return Err(format!(
                            "unknown capability format '{other}' (supported: json)\n{usage}"
                        )
                        .into());
                    }
                }
            }
            "--profile" | "-p" => {
                i += 1;
                let p = args
                    .get(i)
                    .ok_or(format!("{usage} (--profile requires PATH)"))?;
                if profile.is_some() {
                    return Err(format!("{usage} (duplicate --profile)").into());
                }
                profile = Some(p.clone());
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown capability option '{flag}'\n{usage}").into());
            }
            p => {
                if profile.is_some() {
                    return Err(format!("{usage} (extra argument)").into());
                }
                profile = Some(p.to_string());
            }
        }
        i += 1;
    }
    Ok(CapabilityOpts { profile, json })
}

/// Resolve an optional golden profile to probe.
///
/// Order:
/// 1. Explicit path from CLI (`--profile` / bare arg) — must exist or error later via verify
/// 2. `NYTPROF_CAPABILITY_FIXTURE` env
/// 3. CWD-relative `fixtures/v5/default-calls1/nytprof.out`
/// 4. Repo-relative via `CARGO_MANIFEST_DIR` (crate → workspace root)
fn resolve_capability_probe(forced: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = forced {
        return Some(PathBuf::from(p));
    }
    if let Ok(p) = env::var("NYTPROF_CAPABILITY_FIXTURE") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    let cwd_rel = PathBuf::from(DEFAULT_CAPABILITY_FIXTURE);
    if cwd_rel.is_file() {
        // Prefer stable relative display when run from repo root.
        return Some(cwd_rel);
    }
    // crates/nytprof-cli → repo root (normalize `../..` for clean profile_ok: paths)
    let from_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DEFAULT_CAPABILITY_FIXTURE);
    if from_manifest.is_file() {
        return Some(
            from_manifest
                .canonicalize()
                .unwrap_or(from_manifest),
        );
    }
    None
}

fn cmd_dump(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let events = decode_path(path)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for ev in &events {
        write_record(&mut out, ev)?;
    }

    // Trailing synthetic _END (oracle dump_readstream.pl)
    let end = Event::new(events.len() as u64, tags::END, vec![]);
    write_record(&mut out, &end)?;

    Ok(())
}

fn load_model_for_report(path: &str) -> Result<ProfileModel, Box<dyn std::error::Error>> {
    let model = ProfileModel::from_path(path)?;
    // INCOMPLETE-STREAM: report/export fail closed by default; dump stays lenient.
    require_complete_stream(&model)?;
    Ok(model)
}

/// Parse `report` / `summary` args: optional `--json` / `--format=json` + profile path.
///
/// Accepted forms (path and flags in either order):
/// - `report <profile.out>`
/// - `report --json <profile.out>`
/// - `report <profile.out> --json`
/// - `report --format=json <profile.out>` / `report --format json <profile.out>`
fn cmd_report(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let usage = "Usage: nytprof-cli report [--json | --format=json] <profile.out>";
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_usage();
        return Ok(());
    }
    let opts = parse_report_args(args, usage)?;
    let model = load_model_for_report(&opts.path)?;
    if opts.json {
        let text = render_aggregates_json(&model, &opts.path)?;
        write_stdout_text(&text)
    } else {
        let text = render_summary_text(&model, &opts.path);
        write_stdout_text(&text)
    }
}

/// `aggregates` / `agg` — always emit structured JSON aggregates (NATIVE-AGG-JSON).
fn cmd_aggregates(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let usage = "Usage: nytprof-cli aggregates [--json] <profile.out>";
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_usage();
        return Ok(());
    }
    // Reuse report parser (accepts optional --json / --format=json); always emit JSON.
    let opts = parse_report_args(args, usage)?;
    let model = load_model_for_report(&opts.path)?;
    let text = render_aggregates_json(&model, &opts.path)?;
    write_stdout_text(&text)
}

struct ReportOpts {
    path: String,
    json: bool,
}

fn parse_report_args(
    args: &[String],
    usage: &str,
) -> Result<ReportOpts, Box<dyn std::error::Error>> {
    let mut path: Option<String> = None;
    let mut json = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => {
                json = true;
            }
            "--format" => {
                i += 1;
                let fmt = args
                    .get(i)
                    .ok_or(format!("{usage} (--format requires a value)"))?;
                match fmt.to_ascii_lowercase().as_str() {
                    "json" => json = true,
                    other => {
                        return Err(format!(
                            "unknown report format '{other}' (supported: json)\n{usage}"
                        )
                        .into());
                    }
                }
            }
            flag if flag.starts_with("--format=") => {
                let fmt = flag["--format=".len()..].to_ascii_lowercase();
                match fmt.as_str() {
                    "json" => json = true,
                    other => {
                        return Err(format!(
                            "unknown report format '{other}' (supported: json)\n{usage}"
                        )
                        .into());
                    }
                }
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown report option '{flag}'\n{usage}").into());
            }
            p => {
                if path.is_some() {
                    return Err(format!("{usage} (extra argument)").into());
                }
                path = Some(p.to_string());
            }
        }
        i += 1;
    }
    let path = path.ok_or(usage.to_string())?;
    Ok(ReportOpts { path, json })
}

/// Structured JSON aggregates from a real [`ProfileModel`] (NATIVE-AGG-JSON).
///
/// Schema: `docs/schemas/native-aggregates-json-mvp-v0.md`
///
/// Fields:
/// - `ok` (bool true)
/// - `profile` (path string)
/// - `leaf_returns` / `mid_returns` / `mid_leaf_edge` (convenience ints)
/// - `discount_events` (A3 count)
/// - `sub_entry_events` (SUB_ENTRY multiplicity; ProfileModel.sub_entry_events)
/// - `is_stream_complete` / `incompleteness_reasons` (COMPAT-010; JSON-NATIVE-STREAM-MVP)
/// - `time_line_events` / `time_block_events` / `pid_start_events` / `pid_end_events` (model counters; JSON-TIME-BLOCK-MVP for A2)
/// - `line_calls_1_5` / `block_line_calls_1_4` (A4 / A4b greppable ints; JSON-BLOCKS-MVP)
/// - `sub_def_leaf` / `sub_def_mid` / `source_line_1_5` (A9/A8 samples; JSON-SUBDEF-SOURCE-MVP)
/// - `attribute_ticks_per_sec` / `option_calls` / `file_1` (ATTRIBUTE/OPTION/NEW_FID; JSON-META-FILES-MVP)
/// - `attribute_basetime` (ATTRIBUTE basetime sample; JSON-ATTR-BASETIME-MVP)
/// - `file_1_basename` (stable basename of fid 1; JSON-FILE-BASENAME-MVP)
/// - `total_events` (canonical dump stream multiplicity; JSON-TOTAL-EVENTS-MVP;
///   default-calls1 **2474** = golden `readstream.jsonl` lines / `nytprof-cli dump`
///   lines / JsonlData `records_seen`; includes dump-only synthetic `_END`.
///   ProfileModel.total_events is decoded binary tags only = total_events−1)
/// - `sub_return_events` / `new_fid_events` / `sub_callers_events` /
///   `src_line_events` / `sub_info_events` (tag multiplicity; JSON-EVENT-COUNTS-MVP;
///   default-calls1 **27** / **3** / **13** / **632** / **31**)
/// - `subs` map: subname → returns (A5)
/// - `edges` map: `"caller\\tcalled"` → count (A7; TAB-joined keys)
fn render_aggregates_json(
    model: &ProfileModel,
    profile_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let leaf_returns = model
        .sub_total("main::leaf")
        .map(|t| t.returns)
        .unwrap_or(0);
    let mid_returns = model
        .sub_total("main::mid")
        .map(|t| t.returns)
        .unwrap_or(0);
    let mid_leaf_edge = model
        .call_edge("main::mid", "main::leaf")
        .map(|e| e.count)
        .unwrap_or(0);
    // JSON-BLOCKS-MVP: same greppable A4/A4b keys as Perl query --json.
    // blocks-calls1: line 1:5 calls = 780, block_line 1:4 calls = 810.
    // Absent locations → 0 (default-calls1 has no TIME_BLOCK → block 0).
    let line_calls_1_5 = model
        .line_total(1, 5)
        .map(|t| t.calls)
        .unwrap_or(0);
    let block_line_calls_1_4 = model
        .block_line_total(1, 4)
        .map(|t| t.calls)
        .unwrap_or(0);

    // JSON-NATIVE-STREAM-MVP: same keys as Perl query --json (QUERY-JSON-EXPAND).
    // Reasons come from ProfileModel::stream_incompleteness_reasons (COMPAT-010).
    let incompleteness_reasons: Vec<Value> = model
        .stream_incompleteness_reasons()
        .into_iter()
        .map(|s| json!(s))
        .collect();

    // JSON-SUBDEF-SOURCE-MVP: greppable A9 samples + A8 source text.
    // ProfileModel::sub_def / source_line only (null when absent).
    let sub_def_json = |name: &str| -> Value {
        match model.sub_def(name) {
            Some(d) => json!({
                "fid": d.fid,
                "first_line": d.first_line,
                "last_line": d.last_line,
            }),
            None => Value::Null,
        }
    };
    let source_line_1_5 = model
        .source_line(1, 5)
        .map(|s| json!(s))
        .unwrap_or(Value::Null);

    // JSON-META-FILES-MVP: greppable ATTRIBUTE / OPTION / NEW_FID samples.
    // ProfileModel attributes/options/files (or file_name) only; null when absent.
    let attr_str = |key: &str| -> Value {
        model
            .attributes
            .get(key)
            .map(|s| json!(s))
            .unwrap_or(Value::Null)
    };
    let opt_str = |key: &str| -> Value {
        model
            .options
            .get(key)
            .map(|s| json!(s))
            .unwrap_or(Value::Null)
    };
    let file_1 = model
        .file_name(1)
        .map(|s| json!(s))
        .unwrap_or(Value::Null);
    // JSON-FILE-BASENAME-MVP: stable basename for fid 1 (absolute path is volatile).
    // ProfileModel::fid_basename only; null when fid/path absent.
    let file_1_basename = model
        .fid_basename(1)
        .map(|s| json!(s))
        .unwrap_or(Value::Null);

    let mut subs = serde_json::Map::new();
    let mut sub_rows: Vec<_> = model.sub_return_totals.iter().collect();
    sub_rows.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (name, t) in sub_rows {
        subs.insert(name.clone(), json!(t.returns));
    }

    let mut edges = serde_json::Map::new();
    let mut edge_rows: Vec<_> = model.call_edges.iter().collect();
    edge_rows.sort_by(|(a, _), (b, _)| a.cmp(b));
    for ((caller, called), e) in edge_rows {
        // TAB-joined key — same convention as QUERY-JSON-MVP / JsonlData.
        let key = format!("{caller}\t{called}");
        edges.insert(key, json!(e.count));
    }

    // Stable field order for compact single-line JSON (serde_json Map insertion order).
    let mut obj = serde_json::Map::new();
    obj.insert("ok".into(), json!(true));
    obj.insert("profile".into(), json!(profile_path));
    obj.insert("leaf_returns".into(), json!(leaf_returns));
    obj.insert("mid_returns".into(), json!(mid_returns));
    obj.insert("mid_leaf_edge".into(), json!(mid_leaf_edge));
    obj.insert("discount_events".into(), json!(model.discount_events));
    // SUB_ENTRY multiplicity (JSON-SUB-ENTRY-MVP); same field name as JsonlData.
    obj.insert("sub_entry_events".into(), json!(model.sub_entry_events));
    // JSON-TOTAL-EVENTS-MVP: canonical dump stream multiplicity (shared key with
    // Perl query --json / JsonlData records_seen).
    // ProfileModel.total_events counts decoded binary tags only; `nytprof-cli dump`
    // always appends one synthetic `_END` (oracle dump_readstream.pl), so dump
    // line count = model.total_events + 1. default-calls1 golden = 2474.
    obj.insert(
        "total_events".into(),
        json!(model.total_events.saturating_add(1)),
    );
    // JSON-NATIVE-STREAM-MVP: stream completeness + dump/model-derived PID/timing counts.
    obj.insert("is_stream_complete".into(), json!(model.is_stream_complete()));
    obj.insert(
        "incompleteness_reasons".into(),
        Value::Array(incompleteness_reasons),
    );
    obj.insert("time_line_events".into(), json!(model.time_line_events));
    // JSON-TIME-BLOCK-MVP: A2 TIME_BLOCK multiplicity (same field as JsonlData / Perl query).
    // default-calls1 → 0; blocks-calls1 → 916 (model-matched).
    obj.insert("time_block_events".into(), json!(model.time_block_events));
    obj.insert("pid_start_events".into(), json!(model.pid_start_events));
    obj.insert("pid_end_events".into(), json!(model.pid_end_events));
    // JSON-BLOCKS-MVP: greppable A4 / A4b convenience integers.
    obj.insert("line_calls_1_5".into(), json!(line_calls_1_5));
    obj.insert("block_line_calls_1_4".into(), json!(block_line_calls_1_4));
    // JSON-SUBDEF-SOURCE-MVP: sample A9 ranges + A8 hot-loop source line.
    obj.insert("sub_def_leaf".into(), sub_def_json("main::leaf"));
    obj.insert("sub_def_mid".into(), sub_def_json("main::mid"));
    obj.insert("source_line_1_5".into(), source_line_1_5);
    // JSON-META-FILES-MVP: greppable ATTRIBUTE / OPTION / NEW_FID samples.
    obj.insert("attribute_ticks_per_sec".into(), attr_str("ticks_per_sec"));
    // JSON-ATTR-BASETIME-MVP: greppable ATTRIBUTE basetime sample (string-or-null).
    // default-calls1 golden often "1786111723"; not a wall-clock policy freeze.
    obj.insert("attribute_basetime".into(), attr_str("basetime"));
    obj.insert("option_calls".into(), opt_str("calls"));
    obj.insert("file_1".into(), file_1);
    // JSON-FILE-BASENAME-MVP: stable basename sample (not full files map).
    obj.insert("file_1_basename".into(), file_1_basename);
    // JSON-EVENT-COUNTS-MVP: dump/model tag multiplicity (match JsonlData / cross-smoke).
    // default-calls1: SUB_RETURN 27, NEW_FID 3, SUB_CALLERS 13, SRC_LINE 632, SUB_INFO 31.
    obj.insert("sub_return_events".into(), json!(model.sub_return_events));
    obj.insert("new_fid_events".into(), json!(model.new_fid_events));
    obj.insert("sub_callers_events".into(), json!(model.sub_callers_events));
    obj.insert("src_line_events".into(), json!(model.src_line_events));
    obj.insert("sub_info_events".into(), json!(model.sub_info_events));
    obj.insert("subs".into(), Value::Object(subs));
    obj.insert("edges".into(), Value::Object(edges));

    Ok(serde_json::to_string(&Value::Object(obj))?)
}

fn cmd_verify(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let text = verify_profile(Path::new(path))?;
    write_stdout_text(&text)
}

fn cmd_folded(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let model = load_model_for_report(path)?;
    let text = render_folded_stacks(&model);
    write_stdout_text(&text)
}

fn cmd_callgrind(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let model = load_model_for_report(path)?;
    let text = render_callgrind(&model);
    write_stdout_text(&text)
}

fn write_stdout_text(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    out.write_all(text.as_bytes())?;
    if !text.ends_with('\n') {
        out.write_all(b"\n")?;
    }
    Ok(())
}

/// Parse `html` args:
/// - `html <profile.out>` — single-file HTML to stdout
/// - `html <profile.out> -o path.html` — single-file to path
/// - `html <profile.out> --out-dir DIR` — multi-file site (`index.html` + `source.html`)
///
/// `-o` / `--output` and `--out-dir` / `--dir` are mutually exclusive.
fn cmd_html(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let usage =
        "Usage: nytprof-cli html <profile.out> [-o path.html | --out-dir DIR]";
    let mut path: Option<&str> = None;
    let mut out_path: Option<&str> = None;
    let mut out_dir: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                let p = args
                    .get(i)
                    .ok_or(format!("{usage} (-o needs a path)"))?;
                if out_path.is_some() {
                    return Err(format!("{usage} (duplicate -o)").into());
                }
                if out_dir.is_some() {
                    return Err(format!("{usage} (-o and --out-dir are mutually exclusive)").into());
                }
                out_path = Some(p.as_str());
            }
            "--out-dir" | "--dir" => {
                i += 1;
                let p = args
                    .get(i)
                    .ok_or(format!("{usage} (--out-dir needs a directory)"))?;
                if out_dir.is_some() {
                    return Err(format!("{usage} (duplicate --out-dir)").into());
                }
                if out_path.is_some() {
                    return Err(format!("{usage} (-o and --out-dir are mutually exclusive)").into());
                }
                out_dir = Some(p.as_str());
            }
            "-h" | "--help" => {
                print_usage();
                return Ok(());
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown html option '{flag}'\n{usage}").into());
            }
            p => {
                if path.is_some() {
                    return Err(format!("{usage} (extra argument)").into());
                }
                path = Some(p);
            }
        }
        i += 1;
    }

    let path = path.ok_or(usage)?;
    let model = load_model_for_report(path)?;

    if let Some(dir) = out_dir {
        let site = write_html_site(&model, path, Path::new(dir))?;
        // Paths written (stderr so stdout stays free for piping other modes).
        let base = dir.trim_end_matches('/');
        eprintln!("{base}/index.html");
        for (filename, _) in &site.file_pages {
            eprintln!("{base}/{filename}");
        }
        eprintln!("{base}/{}", site.source_filename);
        return Ok(());
    }

    let html = render_html_summary(&model, path);
    if let Some(dest) = out_path {
        fs::write(dest, html.as_bytes())?;
    } else {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        out.write_all(html.as_bytes())?;
        if !html.ends_with('\n') {
            out.write_all(b"\n")?;
        }
    }
    Ok(())
}

/// Parse `csv` flags and path.
///
/// Accepted forms:
/// - `csv <profile.out>`
/// - `csv --subs <profile.out>`
/// - `csv --edges <profile.out>`
/// - `csv --subs --edges <profile.out>` (same as default dual section)
fn cmd_csv(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut want_subs = false;
    let mut want_edges = false;
    let mut path: Option<&str> = None;

    for a in args {
        match a.as_str() {
            "--subs" | "-s" => want_subs = true,
            "--edges" | "-e" => want_edges = true,
            "-h" | "--help" => {
                print_usage();
                return Ok(());
            }
            flag if flag.starts_with('-') => {
                return Err(format!(
                    "unknown csv option '{flag}'\nUsage: nytprof-cli csv [--subs] [--edges] <profile.out>"
                )
                .into());
            }
            p => {
                if path.is_some() {
                    return Err(
                        "Usage: nytprof-cli csv [--subs] [--edges] <profile.out> (extra argument)"
                            .into(),
                    );
                }
                path = Some(p);
            }
        }
    }

    let path = path.ok_or("Usage: nytprof-cli csv [--subs] [--edges] <profile.out>")?;
    let model = load_model_for_report(path)?;

    // Default (no flags): dual-section. Single flag selects that section only.
    let text = match (want_subs, want_edges) {
        (false, false) | (true, true) => render_csv_report(&model),
        (true, false) => render_subs_csv(&model),
        (false, true) => render_edges_csv(&model),
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();
    out.write_all(text.as_bytes())?;
    if !text.ends_with('\n') {
        out.write_all(b"\n")?;
    }
    Ok(())
}

fn write_record(out: &mut impl Write, ev: &Event) -> io::Result<()> {
    // Key order args, seq, tag matches JSON::PP->canonical(1).
    let mut map = serde_json::Map::new();
    map.insert("args".into(), Value::Array(ev.args.clone()));
    map.insert("seq".into(), json!(ev.seq));
    map.insert("tag".into(), Value::String(ev.tag.clone()));
    serde_json::to_writer(&mut *out, &Value::Object(map))?;
    out.write_all(b"\n")?;
    Ok(())
}

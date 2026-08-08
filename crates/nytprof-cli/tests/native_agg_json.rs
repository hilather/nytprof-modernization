//! NATIVE-AGG-JSON / JSON-SUB-ENTRY-MVP / JSON-NATIVE-STREAM-MVP /
//! JSON-META-FILES-MVP / JSON-TIME-BLOCK-MVP / JSON-EVENT-COUNTS-MVP:
//! structured JSON aggregates from real ProfileModel.
//!
//! Schema: `docs/schemas/native-aggregates-json-mvp-v0.md`
//!
//! Drives the real `nytprof-dump` binary (`nytprof-cli` package). Asserts
//! default-calls1 leaf returns **15**, mid **3**, mid→leaf **15**,
//! `sub_entry_events` **0**; stream/PID fields (`is_stream_complete`,
//! `incompleteness_reasons`, `time_line_events`, `time_block_events`,
//! `pid_*_events`) matching the same fixture via ProfileModel; greppable
//! ATTRIBUTE/OPTION/NEW_FID samples (`attribute_ticks_per_sec` /
//! `option_calls` / `file_1`); `file_1_basename` (**JSON-FILE-BASENAME-MVP**);
//! JSON-EVENT-COUNTS-MVP tag multiplicities `sub_return_events` **27**,
//! `new_fid_events` **3**, `sub_callers_events` **13**, `src_line_events`
//! **632**, `sub_info_events` **31** (model-matched); calls2-default
//! `sub_entry_events` **27**; blocks-calls1 `time_block_events` **916** /
//! default-calls1 **0**.

use std::path::PathBuf;
use std::process::Command;

use nytprof_model::ProfileModel;
use serde_json::Value;

fn cli_bin() -> PathBuf {
    if let Some(p) = option_env!("CARGO_BIN_EXE_nytprof_dump") {
        return PathBuf::from(p);
    }
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_nytprof_dump") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "../../target/debug/nytprof-dump",
        "../../target/release/nytprof-dump",
        "../../prefix/bin/nytprof-cli",
    ] {
        let p = manifest.join(rel);
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "nytprof-dump binary not found (CARGO_BIN_EXE_nytprof_dump unset; no target/prefix binary)"
    );
}

fn fixture_default_calls1() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/v5/default-calls1/nytprof.out")
}

fn fixture_calls2_default() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/v5/calls2-default/nytprof.out")
}

fn fixture_blocks_calls1() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/v5/blocks-calls1/nytprof.out")
}

fn run_cli(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(cli_bin())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {:?}: {e}", args));
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

/// Parse and assert MVP aggregate fields for default-calls1.
fn assert_agg_json_default_calls1(stdout: &str, label: &str) -> Value {
    let v: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("{label}: stdout is not JSON: {e}\nstdout:\n{stdout}")
    });
    let obj = v
        .as_object()
        .unwrap_or_else(|| panic!("{label}: expected JSON object\nstdout:\n{stdout}"));

    assert_eq!(
        obj.get("ok"),
        Some(&Value::Bool(true)),
        "{label}: ok must be true\nstdout:\n{stdout}"
    );
    assert!(
        obj.get("profile")
            .and_then(|p| p.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "{label}: profile path required\nstdout:\n{stdout}"
    );

    // Convenience integers — exact default-calls1 contract.
    assert_eq!(
        obj.get("leaf_returns").and_then(|x| x.as_u64()),
        Some(15),
        "{label}: leaf_returns must be 15\nstdout:\n{stdout}"
    );
    assert_eq!(
        obj.get("mid_returns").and_then(|x| x.as_u64()),
        Some(3),
        "{label}: mid_returns must be 3\nstdout:\n{stdout}"
    );
    assert_eq!(
        obj.get("mid_leaf_edge").and_then(|x| x.as_u64()),
        Some(15),
        "{label}: mid_leaf_edge must be 15\nstdout:\n{stdout}"
    );

    // discount_events present and positive for this fixture (oracle 818).
    let disc = obj
        .get("discount_events")
        .and_then(|x| x.as_u64())
        .unwrap_or_else(|| panic!("{label}: discount_events missing\nstdout:\n{stdout}"));
    assert!(
        disc > 0,
        "{label}: discount_events must be > 0 (got {disc})\nstdout:\n{stdout}"
    );

    // JSON-SUB-ENTRY-MVP: calls=1 fixture has no SUB_ENTRY tags.
    assert_eq!(
        obj.get("sub_entry_events").and_then(|x| x.as_u64()),
        Some(0),
        "{label}: sub_entry_events must be 0 on default-calls1\nstdout:\n{stdout}"
    );

    // JSON-NATIVE-STREAM-MVP: match ProfileModel for the same fixture path.
    // default-calls1 is complete; time_line/pid counts ≥ 1 (model-derived).
    let path = fixture_default_calls1();
    let model = ProfileModel::from_path(&path).unwrap_or_else(|e| {
        panic!("{label}: ProfileModel::from_path({}): {e}", path.display())
    });

    // JSON-TOTAL-EVENTS-MVP: dump stream multiplicity including synthetic _END.
    // ProfileModel.total_events = decoded binary tags (2473); dump/JSONL lines = 2474.
    assert_eq!(
        model.total_events, 2473,
        "{label}: model.total_events (decoded, no _END) must be 2473 (got {})",
        model.total_events
    );
    let want_total = model.total_events.saturating_add(1);
    assert_eq!(
        want_total, 2474,
        "{label}: dump-equivalent total_events must be 2474 (model+1={want_total})"
    );
    assert_eq!(
        obj.get("total_events").and_then(|x| x.as_u64()),
        Some(2474),
        "{label}: total_events must be 2474 (dump stream incl. _END)\nstdout:\n{stdout}"
    );
    assert_eq!(
        obj.get("total_events").and_then(|x| x.as_u64()),
        Some(want_total),
        "{label}: total_events must equal model.total_events+1 ({want_total})\nstdout:\n{stdout}"
    );

    // JSON-EVENT-COUNTS-MVP: stream tag multiplicities (model-matched + golden).
    // Independent stream recount on default-calls1: 27 / 3 / 13 / 632 / 31.
    let assert_event_count = |key: &str, model_val: u64, want: u64| {
        assert_eq!(
            model_val, want,
            "{label}: model {key} must be {want} (got {model_val})"
        );
        assert_eq!(
            obj.get(key).and_then(|x| x.as_u64()),
            Some(want),
            "{label}: {key} must be {want} (model-matched)\nstdout:\n{stdout}"
        );
        assert_eq!(
            obj.get(key).and_then(|x| x.as_u64()),
            Some(model_val),
            "{label}: {key} must match model ({model_val})\nstdout:\n{stdout}"
        );
    };
    assert_event_count("sub_return_events", model.sub_return_events, 27);
    assert_event_count("new_fid_events", model.new_fid_events, 3);
    assert_event_count("sub_callers_events", model.sub_callers_events, 13);
    assert_event_count("src_line_events", model.src_line_events, 632);
    assert_event_count("sub_info_events", model.sub_info_events, 31);
    assert_eq!(
        obj.get("is_stream_complete"),
        Some(&Value::Bool(true)),
        "{label}: is_stream_complete must be true on default-calls1\nstdout:\n{stdout}"
    );
    assert!(
        model.is_stream_complete(),
        "{label}: model must report stream complete on default-calls1: {:?}",
        model.stream_incompleteness_reasons()
    );
    let reasons = obj
        .get("incompleteness_reasons")
        .and_then(|x| x.as_array())
        .unwrap_or_else(|| {
            panic!("{label}: incompleteness_reasons array required\nstdout:\n{stdout}")
        });
    assert!(
        reasons.is_empty(),
        "{label}: incompleteness_reasons must be [] when complete\nstdout:\n{stdout}"
    );
    let tl = obj
        .get("time_line_events")
        .and_then(|x| x.as_u64())
        .unwrap_or_else(|| panic!("{label}: time_line_events missing\nstdout:\n{stdout}"));
    let ps = obj
        .get("pid_start_events")
        .and_then(|x| x.as_u64())
        .unwrap_or_else(|| panic!("{label}: pid_start_events missing\nstdout:\n{stdout}"));
    let pe = obj
        .get("pid_end_events")
        .and_then(|x| x.as_u64())
        .unwrap_or_else(|| panic!("{label}: pid_end_events missing\nstdout:\n{stdout}"));
    assert_eq!(
        tl, model.time_line_events,
        "{label}: time_line_events must match model ({})\nstdout:\n{stdout}",
        model.time_line_events
    );
    let tb = obj
        .get("time_block_events")
        .and_then(|x| x.as_u64())
        .unwrap_or_else(|| panic!("{label}: time_block_events missing\nstdout:\n{stdout}"));
    assert_eq!(
        tb, model.time_block_events,
        "{label}: time_block_events must match model ({})\nstdout:\n{stdout}",
        model.time_block_events
    );
    // default-calls1 is TIME_LINE-only (no TIME_BLOCK tags).
    assert_eq!(
        tb, 0,
        "{label}: time_block_events must be 0 on default-calls1\nstdout:\n{stdout}"
    );
    assert_eq!(
        ps, model.pid_start_events,
        "{label}: pid_start_events must match model ({})\nstdout:\n{stdout}",
        model.pid_start_events
    );
    assert_eq!(
        pe, model.pid_end_events,
        "{label}: pid_end_events must match model ({})\nstdout:\n{stdout}",
        model.pid_end_events
    );
    assert!(
        tl >= 1,
        "{label}: time_line_events must be ≥ 1 (got {tl})\nstdout:\n{stdout}"
    );
    assert!(
        ps >= 1,
        "{label}: pid_start_events must be ≥ 1 (got {ps})\nstdout:\n{stdout}"
    );
    assert!(
        pe >= 1,
        "{label}: pid_end_events must be ≥ 1 (got {pe})\nstdout:\n{stdout}"
    );

    // JSON-SUBDEF-SOURCE-MVP: greppable A9 samples + A8 hot-loop source.
    let assert_sub_def = |key: &str, fid: u64, first: u64, last: u64| {
        let d = obj
            .get(key)
            .and_then(|x| x.as_object())
            .unwrap_or_else(|| panic!("{label}: {key} object required\nstdout:\n{stdout}"));
        assert_eq!(
            d.get("fid").and_then(|x| x.as_u64()),
            Some(fid),
            "{label}: {key}.fid must be {fid}\nstdout:\n{stdout}"
        );
        assert_eq!(
            d.get("first_line").and_then(|x| x.as_u64()),
            Some(first),
            "{label}: {key}.first_line must be {first}\nstdout:\n{stdout}"
        );
        assert_eq!(
            d.get("last_line").and_then(|x| x.as_u64()),
            Some(last),
            "{label}: {key}.last_line must be {last}\nstdout:\n{stdout}"
        );
    };
    assert_sub_def("sub_def_leaf", 1, 3, 7);
    assert_sub_def("sub_def_mid", 1, 8, 12);
    let src = obj
        .get("source_line_1_5")
        .and_then(|x| x.as_str())
        .unwrap_or_else(|| panic!("{label}: source_line_1_5 string required\nstdout:\n{stdout}"));
    assert!(
        src.contains("$x++") && src.contains("1 .. 50"),
        "{label}: source_line_1_5 must contain $x++ and 1 .. 50, got {src:?}\nstdout:\n{stdout}"
    );

    // JSON-META-FILES-MVP: greppable ATTRIBUTE / OPTION / NEW_FID samples
    // must match ProfileModel for the same fixture (dump-derived only).
    let want_ticks = model
        .attributes
        .get("ticks_per_sec")
        .map(String::as_str)
        .unwrap_or_else(|| {
            panic!("{label}: model missing attributes[ticks_per_sec] on default-calls1")
        });
    // JSON-ATTR-BASETIME-MVP: greppable ATTRIBUTE basetime sample (model-matched).
    let want_basetime = model
        .attributes
        .get("basetime")
        .map(String::as_str)
        .unwrap_or_else(|| {
            panic!("{label}: model missing attributes[basetime] on default-calls1")
        });
    let want_calls = model
        .options
        .get("calls")
        .map(String::as_str)
        .unwrap_or_else(|| panic!("{label}: model missing options[calls] on default-calls1"));
    let want_file_1 = model.file_name(1).unwrap_or_else(|| {
        panic!("{label}: model missing file_name(1) on default-calls1")
    });
    let want_file_1_basename = model.fid_basename(1).unwrap_or_else(|| {
        panic!("{label}: model missing fid_basename(1) on default-calls1")
    });
    assert_eq!(
        obj.get("attribute_ticks_per_sec").and_then(|x| x.as_str()),
        Some(want_ticks),
        "{label}: attribute_ticks_per_sec must match model ({want_ticks})\nstdout:\n{stdout}"
    );
    assert_eq!(
        obj.get("attribute_basetime").and_then(|x| x.as_str()),
        Some(want_basetime),
        "{label}: attribute_basetime must match model ({want_basetime})\nstdout:\n{stdout}"
    );
    assert_eq!(
        want_basetime, "1786111723",
        "{label}: default-calls1 basetime golden is 1786111723, got {want_basetime:?}"
    );
    assert_eq!(
        obj.get("option_calls").and_then(|x| x.as_str()),
        Some(want_calls),
        "{label}: option_calls must match model ({want_calls})\nstdout:\n{stdout}"
    );
    assert_eq!(
        obj.get("file_1").and_then(|x| x.as_str()),
        Some(want_file_1),
        "{label}: file_1 must match model ({want_file_1})\nstdout:\n{stdout}"
    );
    assert!(
        want_file_1.contains("workload.pl"),
        "{label}: model file_1 must contain workload.pl, got {want_file_1:?}"
    );
    // JSON-FILE-BASENAME-MVP: stable basename (not volatile absolute path).
    assert_eq!(
        obj.get("file_1_basename").and_then(|x| x.as_str()),
        Some(want_file_1_basename),
        "{label}: file_1_basename must match model ({want_file_1_basename})\nstdout:\n{stdout}"
    );
    assert_eq!(
        want_file_1_basename, "workload.pl",
        "{label}: default-calls1 fid_basename(1) golden is workload.pl, got {want_file_1_basename:?}"
    );
    assert!(
        want_file_1_basename.contains("workload.pl"),
        "{label}: file_1_basename must contain workload.pl, got {want_file_1_basename:?}"
    );
    // Golden string expectations for this fixture (also greppable in smoke).
    assert_eq!(
        want_ticks, "10000000",
        "{label}: default-calls1 ticks_per_sec golden is 10000000, got {want_ticks}"
    );
    assert_eq!(
        want_calls, "1",
        "{label}: default-calls1 option calls golden is 1, got {want_calls}"
    );

    let subs = obj
        .get("subs")
        .and_then(|x| x.as_object())
        .unwrap_or_else(|| panic!("{label}: subs object required\nstdout:\n{stdout}"));
    assert_eq!(
        subs.get("main::leaf").and_then(|x| x.as_u64()),
        Some(15),
        "{label}: subs[main::leaf] must be 15\nstdout:\n{stdout}"
    );
    assert_eq!(
        subs.get("main::mid").and_then(|x| x.as_u64()),
        Some(3),
        "{label}: subs[main::mid] must be 3\nstdout:\n{stdout}"
    );

    let edges = obj
        .get("edges")
        .and_then(|x| x.as_object())
        .unwrap_or_else(|| panic!("{label}: edges object required\nstdout:\n{stdout}"));
    let edge_key = "main::mid\tmain::leaf";
    assert_eq!(
        edges.get(edge_key).and_then(|x| x.as_u64()),
        Some(15),
        "{label}: edges[{edge_key:?}] must be 15\nstdout:\n{stdout}"
    );

    // Must not look like human text report.
    assert!(
        !stdout.contains("NYTProf summary report"),
        "{label}: JSON mode must not emit human text header\nstdout:\n{stdout}"
    );

    v
}

#[test]
fn report_json_default_calls1_15_3_15() {
    let path = fixture_default_calls1();
    assert!(path.is_file(), "missing fixture {}", path.display());
    let p = path.to_str().expect("utf-8 path");

    let (code, stdout, stderr) = run_cli(&["report", "--json", p]);
    assert_eq!(
        code, 0,
        "report --json must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_agg_json_default_calls1(&stdout, "report --json");
}

#[test]
fn report_json_path_then_flag() {
    let path = fixture_default_calls1();
    let p = path.to_str().expect("utf-8 path");
    let (code, stdout, stderr) = run_cli(&["report", p, "--json"]);
    assert_eq!(
        code, 0,
        "report PATH --json must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_agg_json_default_calls1(&stdout, "report PATH --json");
}

#[test]
fn report_format_json_aliases() {
    let path = fixture_default_calls1();
    let p = path.to_str().expect("utf-8 path");

    let (c1, o1, e1) = run_cli(&["report", "--json", p]);
    let (c2, o2, e2) = run_cli(&["report", "--format=json", p]);
    let (c3, o3, e3) = run_cli(&["report", "--format", "json", p]);
    assert_eq!(c1, 0, "--json: {e1}");
    assert_eq!(c2, 0, "--format=json: {e2}");
    assert_eq!(c3, 0, "--format json: {e3}");

    let v1 = assert_agg_json_default_calls1(&o1, "--json");
    let v2 = assert_agg_json_default_calls1(&o2, "--format=json");
    let v3 = assert_agg_json_default_calls1(&o3, "--format json");

    // Core semantic fingerprint must match (profile path string may match too).
    for key in [
        "leaf_returns",
        "mid_returns",
        "mid_leaf_edge",
        "discount_events",
        "sub_entry_events",
        "total_events",
        "sub_return_events",
        "new_fid_events",
        "sub_callers_events",
        "src_line_events",
        "sub_info_events",
        "is_stream_complete",
        "incompleteness_reasons",
        "time_line_events",
        "time_block_events",
        "pid_start_events",
        "pid_end_events",
        "attribute_ticks_per_sec",
        "attribute_basetime",
        "option_calls",
        "file_1",
        "file_1_basename",
        "subs",
        "edges",
        "ok",
    ] {
        assert_eq!(v1[key], v2[key], "{key} --json vs --format=json");
        assert_eq!(v1[key], v3[key], "{key} --json vs --format json");
    }
}

#[test]
fn report_json_calls2_default_sub_entry_27() {
    let path = fixture_calls2_default();
    assert!(path.is_file(), "missing fixture {}", path.display());
    let p = path.to_str().expect("utf-8 path");

    let (code, stdout, stderr) = run_cli(&["report", "--json", p]);
    assert_eq!(
        code, 0,
        "report --json calls2-default must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let v: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("calls2-default: stdout is not JSON: {e}\nstdout:\n{stdout}")
    });
    let obj = v
        .as_object()
        .unwrap_or_else(|| panic!("calls2-default: expected JSON object\nstdout:\n{stdout}"));
    assert_eq!(
        obj.get("ok"),
        Some(&Value::Bool(true)),
        "calls2-default: ok must be true\nstdout:\n{stdout}"
    );
    // Fixture-derived SUB_ENTRY multiplicity (stream recount contract: 27).
    assert_eq!(
        obj.get("sub_entry_events").and_then(|x| x.as_u64()),
        Some(27),
        "calls2-default: sub_entry_events must be 27 (from real model)\nstdout:\n{stdout}"
    );
}

/// JSON-TIME-BLOCK-MVP: blocks-calls1 statement timing is TIME_BLOCK only.
/// Golden / ProfileModel observe time_block_events == 916, time_line == 0.
#[test]
fn report_json_blocks_calls1_time_block_916() {
    let path = fixture_blocks_calls1();
    assert!(path.is_file(), "missing fixture {}", path.display());
    let p = path.to_str().expect("utf-8 path");

    let model = ProfileModel::from_path(&path).unwrap_or_else(|e| {
        panic!("blocks-calls1: ProfileModel::from_path({}): {e}", path.display())
    });
    assert_eq!(
        model.time_block_events, 916,
        "blocks-calls1 model time_block_events must be 916 (got {})",
        model.time_block_events
    );
    assert_eq!(
        model.time_line_events, 0,
        "blocks-calls1 model time_line_events must be 0 (got {})",
        model.time_line_events
    );

    let (code, stdout, stderr) = run_cli(&["report", "--json", p]);
    assert_eq!(
        code, 0,
        "report --json blocks-calls1 must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let v: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("blocks-calls1: stdout is not JSON: {e}\nstdout:\n{stdout}")
    });
    let obj = v
        .as_object()
        .unwrap_or_else(|| panic!("blocks-calls1: expected JSON object\nstdout:\n{stdout}"));
    assert_eq!(
        obj.get("ok"),
        Some(&Value::Bool(true)),
        "blocks-calls1: ok must be true\nstdout:\n{stdout}"
    );
    let tb = obj
        .get("time_block_events")
        .and_then(|x| x.as_u64())
        .unwrap_or_else(|| panic!("blocks-calls1: time_block_events missing\nstdout:\n{stdout}"));
    let tl = obj
        .get("time_line_events")
        .and_then(|x| x.as_u64())
        .unwrap_or_else(|| panic!("blocks-calls1: time_line_events missing\nstdout:\n{stdout}"));
    assert_eq!(
        tb, model.time_block_events,
        "blocks-calls1: time_block_events must match model ({})\nstdout:\n{stdout}",
        model.time_block_events
    );
    assert_eq!(
        tl, model.time_line_events,
        "blocks-calls1: time_line_events must match model ({})\nstdout:\n{stdout}",
        model.time_line_events
    );
    assert_eq!(
        tb, 916,
        "blocks-calls1: time_block_events must be 916 (golden/stream)\nstdout:\n{stdout}"
    );
    assert_eq!(
        tl, 0,
        "blocks-calls1: time_line_events must be 0 (blocks=1)\nstdout:\n{stdout}"
    );
}

#[test]
fn aggregates_subcommand_default_calls1() {
    let path = fixture_default_calls1();
    let p = path.to_str().expect("utf-8 path");
    for sub in ["aggregates", "agg"] {
        let (code, stdout, stderr) = run_cli(&[sub, p]);
        assert_eq!(
            code, 0,
            "{sub} must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert_agg_json_default_calls1(&stdout, sub);
    }
}

#[test]
fn report_json_twice_consistent() {
    let path = fixture_default_calls1();
    let p = path.to_str().expect("utf-8 path");
    let (c1, o1, e1) = run_cli(&["report", "--json", p]);
    let (c2, o2, e2) = run_cli(&["report", "--json", p]);
    assert_eq!(c1, 0, "run1: {e1}");
    assert_eq!(c2, 0, "run2: {e2}");
    let v1 = assert_agg_json_default_calls1(&o1, "json run1");
    let v2 = assert_agg_json_default_calls1(&o2, "json run2");
    assert_eq!(v1, v2, "aggregates JSON must be stable across two runs");
}

#[test]
fn report_human_default_unchanged() {
    let path = fixture_default_calls1();
    let p = path.to_str().expect("utf-8 path");
    let (code, stdout, stderr) = run_cli(&["report", p]);
    assert_eq!(
        code, 0,
        "report (human) must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("NYTProf summary report") || stdout.contains("main::leaf"),
        "human report must remain greppable\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("returns=15") || stdout.contains("main::leaf"),
        "human report should show leaf returns\nstdout:\n{stdout}"
    );
    // Not a JSON-only object (text summary starts with a banner line).
    assert!(
        !stdout.trim_start().starts_with('{'),
        "human report must not be JSON-only mode\nstdout:\n{stdout}"
    );
}

#[test]
fn report_json_corrupt_fails_closed() {
    let tmp = std::env::temp_dir().join(format!(
        "nytprof-agg-json-bad-{}-{}.out",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&tmp, b"NOTPROF 5 0\n").expect("write bad");
    let p = tmp.to_str().expect("utf-8");
    let (code, stdout, stderr) = run_cli(&["report", "--json", p]);
    let _ = std::fs::remove_file(&tmp);
    assert_ne!(
        code, 0,
        "report --json on corrupt profile must fail closed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    // Must not emit ok:true success object.
    if let Ok(v) = serde_json::from_str::<Value>(stdout.trim()) {
        assert_ne!(
            v.get("ok"),
            Some(&Value::Bool(true)),
            "must not emit ok:true on failure\nstdout:\n{stdout}"
        );
    }
}

/// JSON-REPORT-INCOMPLETE-FAILCLOSED: record-aligned short prefix of
/// default-calls1 must not yield a successful complete report --json object.
#[test]
fn report_json_incomplete_prefix_fails_closed() {
    let golden = fixture_default_calls1();
    assert!(golden.is_file(), "missing fixture {}", golden.display());
    let bytes = std::fs::read(&golden).expect("read golden");
    assert!(bytes.len() > 500, "fixture too small for 500-byte prefix");

    let tmp = std::env::temp_dir().join(format!(
        "nytprof-agg-json-inc-{}-{}.out",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&tmp, &bytes[..500]).expect("write prefix");
    let p = tmp.to_str().expect("utf-8");

    // Default policy: no salvage env.
    let output = Command::new(cli_bin())
        .args(["report", "--json", p])
        .env_remove("NYTPROF_ALLOW_INCOMPLETE")
        .output()
        .expect("spawn report --json incomplete");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let _ = std::fs::remove_file(&tmp);

    assert_ne!(
        code, 0,
        "report --json incomplete prefix must exit non-zero\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let trimmed = stdout.trim();
    if !trimmed.is_empty() {
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            if let Some(obj) = v.as_object() {
                let ok_true = obj.get("ok") == Some(&Value::Bool(true));
                let isc_true = obj.get("is_stream_complete") == Some(&Value::Bool(true));
                assert!(
                    !(ok_true && isc_true),
                    "must not emit ok:true + is_stream_complete:true on incomplete\nstdout:\n{stdout}"
                );
            }
        }
    }
}

#[test]
fn summary_json_alias() {
    let path = fixture_default_calls1();
    let p = path.to_str().expect("utf-8 path");
    let (code, stdout, stderr) = run_cli(&["summary", "--json", p]);
    assert_eq!(
        code, 0,
        "summary --json must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_agg_json_default_calls1(&stdout, "summary --json");
}

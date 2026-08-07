//! NATIVE-AGG-JSON: structured JSON aggregates from real ProfileModel.
//!
//! Schema: `docs/schemas/native-aggregates-json-mvp-v0.md`
//!
//! Drives the real `nytprof-dump` binary (`nytprof-cli` package). Asserts
//! default-calls1 leaf returns **15**, mid **3**, mid→leaf **15** after a
//! real profile load (not hard-coded theater without CLI decode/model).

use std::path::PathBuf;
use std::process::Command;

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
        "subs",
        "edges",
        "ok",
    ] {
        assert_eq!(v1[key], v2[key], "{key} --json vs --format=json");
        assert_eq!(v1[key], v3[key], "{key} --json vs --format json");
    }
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

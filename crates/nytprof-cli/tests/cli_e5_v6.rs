//! CLI E5 — product report surfaces on v6 (opt-in path; not collection default).
//! CLI E4 product — v5↔v6 semantic equality via real CLIs on dual-sink pairs.
//!
//! Schema: `docs/schemas/cli-e5-v6-opt-in-mvp-v0.md`
//!         `docs/schemas/e4-product-cli-smoke-mvp-v0.md`
//! Design: dual-dispatch `ProfileModel::from_path` (PR-B11a) + full CLI surfaces
//! (report / html / csv / folded / callgrind) on v6 EVENT profiles; E4 product
//! compares the same CLI surfaces on same-run v5 vs v6 dual-sink pairs.
//!
//! Honesty:
//! - Collection default remains v5 (`capability` → `collection_default: v5`).
//! - Capability does **not** claim convert/merge.
//! - Magic auto-detect (no extra `--format=v6` flag required for offline tools).
//! - Dual-sink pairs are scaled synthetic (not full oracle TEST-008 counts).

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

fn fixture_default_calls1_v6() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/e4/dual-sink/default_calls1_v6.nytprof")
}

fn fixture_default_calls1_v5() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/e4/dual-sink/default_calls1_v5.nytprof")
}

fn fixture_e4_dual(stem: &str, fmt: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../fixtures/e4/dual-sink/{stem}_{fmt}.nytprof"
    ))
}

fn fixture_v6_absolute() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/v6/from-c/absolute.nytprof")
}

/// Strip path-only `profile` and return the remaining JSON object for E4 compare.
fn report_json_semantic(path: &str) -> Value {
    let (code, stdout, stderr) = run_cli(&["report", "--json", path]);
    assert_eq!(
        code, 0,
        "report --json must exit 0 for {path}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let mut v: Value = serde_json::from_str(stdout.trim()).expect("json");
    assert_eq!(v["ok"], true, "ok true for {path}\n{stdout}");
    if let Some(obj) = v.as_object_mut() {
        obj.remove("profile");
    }
    v
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

/// report text on dual-sink default_calls1 v6: leaf 15 / mid 3.
#[test]
fn e5_report_text_default_calls1_v6() {
    let path = fixture_default_calls1_v6();
    assert!(path.is_file(), "missing {}", path.display());
    let p = path.to_str().unwrap();
    let (code, stdout, stderr) = run_cli(&["report", p]);
    assert_eq!(
        code, 0,
        "report v6 must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("NYTProf summary report"),
        "text report header\n{stdout}"
    );
    assert!(
        stdout.contains("main::leaf") && stdout.contains("returns=15"),
        "leaf 15 on v6 report\n{stdout}"
    );
    assert!(
        stdout.contains("main::mid") && stdout.contains("returns=3"),
        "mid 3 on v6 report\n{stdout}"
    );
}

/// report --json on v6: leaf/mid/edge greppable ints.
#[test]
fn e5_report_json_default_calls1_v6() {
    let path = fixture_default_calls1_v6();
    assert!(path.is_file(), "missing {}", path.display());
    let p = path.to_str().unwrap();
    let (code, stdout, stderr) = run_cli(&["report", "--json", p]);
    assert_eq!(
        code, 0,
        "report --json v6 must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let v: Value = serde_json::from_str(stdout.trim()).expect("json");
    assert_eq!(v["ok"], true);
    assert_eq!(v["leaf_returns"], 15);
    assert_eq!(v["mid_returns"], 3);
    assert_eq!(v["mid_leaf_edge"], 15);
    assert_eq!(v["is_stream_complete"], true);
}

/// html single-file on v6.
#[test]
fn e5_html_default_calls1_v6() {
    let path = fixture_default_calls1_v6();
    assert!(path.is_file(), "missing {}", path.display());
    let p = path.to_str().unwrap();
    let (code, stdout, stderr) = run_cli(&["html", p]);
    assert_eq!(
        code, 0,
        "html v6 must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("<!DOCTYPE html>") || stdout.contains("<html"),
        "html document\n{stdout}"
    );
    assert!(
        stdout.contains("main::leaf") && stdout.contains(">15<"),
        "leaf 15 in html\n{stdout}"
    );
}

/// csv dual-section on v6.
#[test]
fn e5_csv_default_calls1_v6() {
    let path = fixture_default_calls1_v6();
    assert!(path.is_file(), "missing {}", path.display());
    let p = path.to_str().unwrap();
    let (code, stdout, stderr) = run_cli(&["csv", p]);
    assert_eq!(
        code, 0,
        "csv v6 must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("# subroutines") && stdout.contains("# call_edges"),
        "csv sections\n{stdout}"
    );
    assert!(
        stdout.contains("main::leaf,15,"),
        "leaf returns in csv\n{stdout}"
    );
    assert!(
        stdout.contains("main::mid,main::leaf,15,"),
        "mid→leaf edge in csv\n{stdout}"
    );
}

/// folded stacks on v6: mid→leaf 15.
#[test]
fn e5_folded_default_calls1_v6() {
    let path = fixture_default_calls1_v6();
    assert!(path.is_file(), "missing {}", path.display());
    let p = path.to_str().unwrap();
    let (code, stdout, stderr) = run_cli(&["folded", p]);
    assert_eq!(
        code, 0,
        "folded v6 must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.lines().any(|l| l.contains("main::mid;main::leaf") && l.ends_with(" 15")),
        "folded mid;leaf 15\n{stdout}"
    );
}

/// callgrind on v6: leaf + calls=15.
#[test]
fn e5_callgrind_default_calls1_v6() {
    let path = fixture_default_calls1_v6();
    assert!(path.is_file(), "missing {}", path.display());
    let p = path.to_str().unwrap();
    let (code, stdout, stderr) = run_cli(&["callgrind", p]);
    assert_eq!(
        code, 0,
        "callgrind v6 must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("# callgrind format"),
        "callgrind header\n{stdout}"
    );
    assert!(
        stdout.contains("fn=main::leaf"),
        "leaf fn\n{stdout}"
    );
    assert!(
        stdout.contains("cfn=main::leaf") && stdout.contains("calls=15"),
        "leaf calls=15\n{stdout}"
    );
}

/// cg alias on v6 absolute C fixture (minimal EVENT).
#[test]
fn e5_cg_alias_v6_absolute() {
    let path = fixture_v6_absolute();
    assert!(path.is_file(), "missing {}", path.display());
    let p = path.to_str().unwrap();
    let (code, stdout, stderr) = run_cli(&["cg", p]);
    assert_eq!(
        code, 0,
        "cg v6 absolute must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("# callgrind format"),
        "callgrind header\n{stdout}"
    );
}

/// dump + verify still work on v6 (E5 includes dump/verify opt-in).
#[test]
fn e5_dump_verify_v6_absolute() {
    let path = fixture_v6_absolute();
    assert!(path.is_file(), "missing {}", path.display());
    let p = path.to_str().unwrap();
    let (c1, o1, e1) = run_cli(&["verify", p]);
    assert_eq!(c1, 0, "verify: {e1}\n{o1}");
    assert!(o1.lines().any(|l| l.starts_with("OK:")), "verify OK: {o1}");
    let (c2, o2, e2) = run_cli(&["dump", p]);
    assert_eq!(c2, 0, "dump: {e2}\n{o2}");
    assert!(
        o2.contains("TIME_LINE") || o2.contains("TIME_BLOCK"),
        "dump events\n{o2}"
    );
}

/// No default format flip: capability always advertises collection_default=v5
/// and never claims convert/merge.
#[test]
fn e5_no_default_flip_and_no_convert_merge_claims() {
    let (code, stdout, stderr) = run_cli(&["capability", "--json"]);
    assert_eq!(
        code, 0,
        "capability --json must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let v: Value = serde_json::from_str(stdout.trim()).expect("json");
    assert_eq!(
        v["collection_default"], "v5",
        "collection_default must remain v5 (no R4 flip)\n{stdout}"
    );
    assert_eq!(v["convert"], false, "must not claim convert\n{stdout}");
    assert_eq!(v["merge"], false, "must not claim merge\n{stdout}");
    assert_eq!(v["v6_decode"], true);
    assert_eq!(v["v6_report"], true);
}

/// E4 product: report --json equal on all dual-sink pairs (sans profile path).
#[test]
fn e4_product_report_json_equal_all_dual_pairs() {
    for stem in ["m4", "default_calls1", "blocks_calls1", "calls2_default"] {
        let v5 = fixture_e4_dual(stem, "v5");
        let v6 = fixture_e4_dual(stem, "v6");
        assert!(v5.is_file(), "missing {}", v5.display());
        assert!(v6.is_file(), "missing {}", v6.display());
        let a = report_json_semantic(v5.to_str().unwrap());
        let b = report_json_semantic(v6.to_str().unwrap());
        assert_eq!(
            a, b,
            "E4 product: report --json v5≠v6 for stem={stem}\nv5={a}\nv6={b}"
        );
    }
}

/// E4 product: default_calls1 greppable leaf/mid/edge equal on report text + csv.
#[test]
fn e4_product_default_calls1_surfaces_v5_v6() {
    let v5 = fixture_default_calls1_v5();
    let v6 = fixture_default_calls1_v6();
    assert!(v5.is_file(), "missing {}", v5.display());
    assert!(v6.is_file(), "missing {}", v6.display());
    let p5 = v5.to_str().unwrap();
    let p6 = v6.to_str().unwrap();

    for (label, path) in [("v5", p5), ("v6", p6)] {
        let (code, stdout, stderr) = run_cli(&["report", path]);
        assert_eq!(code, 0, "report {label}: {stderr}\n{stdout}");
        assert!(
            stdout.contains("main::leaf") && stdout.contains("returns=15"),
            "report {label} leaf 15\n{stdout}"
        );
        assert!(
            stdout.contains("main::mid") && stdout.contains("returns=3"),
            "report {label} mid 3\n{stdout}"
        );

        let (code, stdout, stderr) = run_cli(&["csv", path]);
        assert_eq!(code, 0, "csv {label}: {stderr}\n{stdout}");
        assert!(
            stdout.contains("main::leaf,15,"),
            "csv {label} leaf\n{stdout}"
        );
        assert!(
            stdout.contains("main::mid,main::leaf,15,"),
            "csv {label} edge\n{stdout}"
        );

        let (code, stdout, stderr) = run_cli(&["folded", path]);
        assert_eq!(code, 0, "folded {label}: {stderr}\n{stdout}");
        assert!(
            stdout
                .lines()
                .any(|l| l.contains("main::mid;main::leaf") && l.ends_with(" 15")),
            "folded {label} mid;leaf 15\n{stdout}"
        );

        let (code, stdout, stderr) = run_cli(&["callgrind", path]);
        assert_eq!(code, 0, "callgrind {label}: {stderr}\n{stdout}");
        assert!(
            stdout.contains("fn=main::leaf") && stdout.contains("calls=15"),
            "callgrind {label} leaf calls=15\n{stdout}"
        );

        let (code, stdout, stderr) = run_cli(&["verify", path]);
        assert_eq!(code, 0, "verify {label}: {stderr}\n{stdout}");
    }
}

/// html --out-dir multi-file site on v6.
#[test]
fn e5_html_outdir_default_calls1_v6() {
    let path = fixture_default_calls1_v6();
    assert!(path.is_file(), "missing {}", path.display());
    let p = path.to_str().unwrap();
    let tmp = std::env::temp_dir().join(format!(
        "nytprof-e5-html-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    let dir = tmp.to_str().unwrap();
    let (code, stdout, stderr) = run_cli(&["html", p, "--out-dir", dir]);
    assert_eq!(
        code, 0,
        "html --out-dir v6 must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let index = tmp.join("index.html");
    assert!(index.is_file(), "index.html missing under {}", tmp.display());
    let body = std::fs::read_to_string(&index).expect("read index");
    assert!(
        body.contains("main::leaf"),
        "index should list leaf\n{body}"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

//! CLI merge / repack / salvage (PR-C02).
//!
//! Schema: `docs/schemas/merge-repack-salvage-mvp-v0.md`

use std::path::PathBuf;
use std::process::Command;

fn cli_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nytprof-dump"))
}

fn dual(stem: &str, side: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/e4/dual-sink")
        .join(format!("{stem}_{side}.nytprof"))
}

fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(cli_bin())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {:?}: {e}", args));
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn json_u64(blob: &str, key: &str) -> u64 {
    let needle = format!("\"{key}\"");
    for part in blob.split(&needle).skip(1) {
        let rest = part.trim_start();
        let rest = rest.strip_prefix(':').unwrap_or(rest).trim_start();
        let num: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(v) = num.parse::<u64>() {
            return v;
        }
    }
    panic!("missing numeric {key} in JSON:\n{blob}");
}

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "nytprof-mrs-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).expect("tmpdir");
    dir
}

#[test]
fn repack_v6_m4_ok() {
    let input = dual("m4", "v6");
    assert!(input.is_file(), "missing {}", input.display());
    let dir = tmp_dir();
    let out = dir.join("out.v6");
    let (code, stdout, stderr) = run(&[
        "repack",
        input.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "repack\n{stdout}\n{stderr}");
    assert!(
        stdout.lines().any(|l| l.starts_with("OK: repack")),
        "missing OK line\n{stdout}"
    );
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"NYTPROF6"));
    let (vc, _, verr) = run(&["verify", out.to_str().unwrap()]);
    assert_eq!(vc, 0, "verify repack\n{verr}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn merge_two_v6_ok() {
    let input = dual("m4", "v6");
    assert!(input.is_file(), "missing {}", input.display());
    let dir = tmp_dir();
    let out = dir.join("merged.v6");
    let (code, stdout, stderr) = run(&[
        "merge",
        "--to=v6",
        "-o",
        out.to_str().unwrap(),
        input.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "merge\n{stdout}\n{stderr}");
    assert!(
        stdout.lines().any(|l| l.starts_with("OK: merge")),
        "missing OK\n{stdout}"
    );
    let (vc, vout, verr) = run(&["verify", out.to_str().unwrap()]);
    assert_eq!(vc, 0, "verify merge\n{vout}\n{verr}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn merge_mixed_v5_v6_to_v6() {
    let v5 = dual("m4", "v5");
    let v6 = dual("m4", "v6");
    assert!(v5.is_file() && v6.is_file());
    let dir = tmp_dir();
    let out = dir.join("mixed.v6");
    let (code, stdout, stderr) = run(&[
        "merge",
        "--to=v6",
        "-o",
        out.to_str().unwrap(),
        v5.to_str().unwrap(),
        v6.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "mixed merge\n{stdout}\n{stderr}");
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"NYTPROF6"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn merge_aggregate_sum_default_calls1_v6_sums_line_calls() {
    let input = dual("default_calls1", "v6");
    assert!(input.is_file(), "missing {}", input.display());
    let dir = tmp_dir();
    let concat_out = dir.join("concat.v6");
    let sum_out = dir.join("sum.v6");

    let (c0, one_json, one_err) = run(&["report", "--json", input.to_str().unwrap()]);
    assert_eq!(c0, 0, "report one\n{one_json}\n{one_err}");
    let one_line = json_u64(&one_json, "line_calls_1_5");
    let one_leaf = json_u64(&one_json, "leaf_returns");
    assert!(one_line >= 1, "expected line_calls_1_5 on default_calls1: {one_json}");

    let (code, stdout, stderr) = run(&[
        "merge",
        "--to=v6",
        "-o",
        concat_out.to_str().unwrap(),
        input.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "concat merge\n{stdout}\n{stderr}");
    assert!(
        stdout.lines().any(|l| l.starts_with("OK: merge")),
        "missing OK: merge\n{stdout}"
    );
    assert!(
        !stdout.contains("--aggregate-sum"),
        "default merge must stay stream-concat\n{stdout}"
    );

    let (cc, concat_json, cerr) = run(&["report", "--json", concat_out.to_str().unwrap()]);
    assert_eq!(cc, 0, "report concat\n{concat_json}\n{cerr}");
    assert_eq!(
        json_u64(&concat_json, "line_calls_1_5"),
        one_line,
        "concat must not sum fid 1 line 5"
    );

    let (scode, sstdout, sstderr) = run(&[
        "merge",
        "--to=v6",
        "--aggregate-sum",
        "-o",
        sum_out.to_str().unwrap(),
        input.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert_eq!(scode, 0, "aggregate-sum merge\n{sstdout}\n{sstderr}");
    assert!(
        sstdout.lines().any(|l| l.starts_with("OK: merge") && l.contains("--aggregate-sum")),
        "missing OK: merge --aggregate-sum\n{sstdout}"
    );
    let (sc, sum_json, serr) = run(&["report", "--json", sum_out.to_str().unwrap()]);
    assert_eq!(sc, 0, "report sum\n{sum_json}\n{serr}");
    assert_eq!(
        json_u64(&sum_json, "line_calls_1_5"),
        one_line.saturating_mul(2),
        "aggregate-sum must combine line_calls_1_5"
    );
    assert_eq!(
        json_u64(&sum_json, "leaf_returns"),
        one_leaf.saturating_mul(2)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn merge_aggregate_sum_refuses_corrupt_member() {
    let good = dual("default_calls1", "v6");
    assert!(good.is_file());
    let dir = tmp_dir();
    let bad = dir.join("bad.nytprof");
    std::fs::write(&bad, b"not-a-profile").unwrap();
    let out = dir.join("out.v6");
    let (code, stdout, _stderr) = run(&[
        "merge",
        "--to=v6",
        "--aggregate-sum",
        "-o",
        out.to_str().unwrap(),
        good.to_str().unwrap(),
        bad.to_str().unwrap(),
    ]);
    assert_ne!(code, 0, "corrupt member must fail\n{stdout}");
    assert!(!stdout.lines().any(|l| l.starts_with("OK: merge")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn merge_refuses_corrupt_member() {
    let good = dual("m4", "v6");
    assert!(good.is_file());
    let dir = tmp_dir();
    let bad = dir.join("bad.nytprof");
    std::fs::write(&bad, b"not-a-profile").unwrap();
    let out = dir.join("out.v6");
    let (code, stdout, _stderr) = run(&[
        "merge",
        "--to=v6",
        "-o",
        out.to_str().unwrap(),
        good.to_str().unwrap(),
        bad.to_str().unwrap(),
    ]);
    assert_ne!(code, 0, "corrupt member must fail\n{stdout}");
    assert!(!out.is_file() || std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0) == 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn salvage_truncated_v5_labels_incomplete() {
    let input = dual("m4", "v5");
    assert!(input.is_file());
    let full = std::fs::read(&input).unwrap();
    let dir = tmp_dir();
    let cut = dir.join("cut.v5");
    std::fs::write(&cut, &full[..full.len() / 2]).unwrap();
    let out = dir.join("salvaged.v5");
    let (code, stdout, stderr) = run(&[
        "salvage",
        "--to=v5",
        cut.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "salvage\n{stdout}\n{stderr}");
    assert!(
        stdout.lines().any(|l| l.starts_with("OK: salvage incomplete=yes")),
        "missing salvage OK line\n{stdout}"
    );
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"NYTProf 5 0\n"));
    // Dump must show salvage attribute (lenient dump surface).
    let (dc, dout, derr) = run(&["dump", out.to_str().unwrap()]);
    assert_eq!(dc, 0, "dump salvaged\n{derr}");
    assert!(
        dout.contains("nytprof.salvage"),
        "dump must include salvage attr\n{dout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn salvage_v6_trailing_garbage() {
    let input = dual("m4", "v6");
    assert!(input.is_file());
    let mut bytes = std::fs::read(&input).unwrap();
    bytes.extend_from_slice(b"TRAILING_GARBAGE!!!!");
    let dir = tmp_dir();
    let dirty = dir.join("dirty.v6");
    std::fs::write(&dirty, &bytes).unwrap();
    let out = dir.join("clean.v6");
    let (code, stdout, stderr) = run(&[
        "salvage",
        dirty.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "salvage trailing\n{stdout}\n{stderr}");
    assert!(stdout.contains("discarded_tail="));
    let out_b = std::fs::read(&out).unwrap();
    assert!(out_b.starts_with(b"NYTPROF6"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn repack_refuses_truncated() {
    let input = dual("m4", "v5");
    assert!(input.is_file());
    let full = std::fs::read(&input).unwrap();
    let dir = tmp_dir();
    let cut = dir.join("cut.v5");
    std::fs::write(&cut, &full[..full.len() / 2]).unwrap();
    let out = dir.join("out.v5");
    let (code, stdout, _stderr) = run(&[
        "repack",
        cut.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_ne!(code, 0, "repack truncated must fail\n{stdout}");
    assert!(!stdout.lines().any(|l| l.starts_with("OK: repack")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_mentions_merge_repack_salvage() {
    let (code, _stdout, stderr) = run(&["--help"]);
    assert_eq!(code, 0);
    let text = format!("{_stdout}{stderr}");
    assert!(text.contains("merge"), "help missing merge");
    assert!(text.contains("repack"), "help missing repack");
    assert!(text.contains("salvage"), "help missing salvage");
    assert!(
        text.contains("incomplete") || text.contains("verified"),
        "help should mention recovery semantics"
    );
    assert!(
        text.contains("--aggregate-sum"),
        "help missing --aggregate-sum"
    );
}

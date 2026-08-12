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
}

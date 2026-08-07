//! COMPAT-010-ERR: corrupt inputs must fail closed on shipped CLI paths.
//!
//! Drives the real `nytprof-dump` binary (`nytprof-cli` package) for
//! `verify`, `dump`, and `report` — not reimplemented decode logic.
//!
//! Contract: `docs/contracts/COMPAT-010_ERROR_FAIL_CLOSED.md`

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn cli_bin() -> PathBuf {
    // Package bin name is nytprof-dump (see crates/nytprof-cli/Cargo.toml).
    // Prefer Cargo's CARGO_BIN_EXE_* (hyphen → underscore); fall back for hosts
    // where the compile-time env is not injected.
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

fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "nytprof-cli-fail-closed-{}-{}-{}",
        label,
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

/// Run `nytprof-cli <subcmd> <path>`; expect non-success exit (no panic → process exits).
fn assert_cli_fails(subcmd: &str, path: &Path, label: &str) {
    let output = Command::new(cli_bin())
        .args([subcmd, path.to_str().expect("utf-8 path")])
        .output()
        .unwrap_or_else(|e| panic!("spawn {subcmd}: {e}"));

    assert!(
        !output.status.success(),
        "{label}: `{subcmd}` must exit non-zero on corrupt input, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    // Must not look like a successful verify summary on stdout.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.lines().any(|l| l.starts_with("OK:")),
        "{label}: `{subcmd}` must not print OK: on corrupt input:\n{stdout}"
    );
}

fn assert_all_shipped_cmds_fail(path: &Path, label: &str) {
    for subcmd in ["verify", "dump", "report"] {
        assert_cli_fails(subcmd, path, &format!("{label} / {subcmd}"));
    }
}

#[test]
fn fail_closed_cli_empty_file() {
    let tmp = temp_path("empty");
    fs::write(&tmp, b"").expect("write empty");
    assert_all_shipped_cmds_fail(&tmp, "empty file");
    let _ = fs::remove_file(&tmp);
}

#[test]
fn fail_closed_cli_truncated_default_calls1() {
    let golden = fixture_default_calls1();
    assert!(golden.is_file(), "missing fixture {}", golden.display());
    let bytes = fs::read(&golden).expect("read golden");
    let half = bytes.len() / 2;
    assert!(half > 0);

    let tmp = temp_path("trunc");
    fs::write(&tmp, &bytes[..half]).expect("write half");
    assert_all_shipped_cmds_fail(&tmp, "truncated half of default-calls1");
    let _ = fs::remove_file(&tmp);
}

#[test]
fn fail_closed_cli_bad_magic() {
    let tmp = temp_path("bad-magic");
    fs::write(&tmp, b"NOTPROF 5 0\n").expect("write bad magic");
    assert_all_shipped_cmds_fail(&tmp, "bad magic NOTPROF");
    let _ = fs::remove_file(&tmp);
}

/// INCOMPLETE-STREAM: record-aligned short prefix → verify/report fail; dump may succeed.
#[test]
fn incomplete_stream_cli_prefix_default_calls1() {
    let golden = fixture_default_calls1();
    assert!(golden.is_file(), "missing fixture {}", golden.display());
    let bytes = fs::read(&golden).expect("read golden");
    assert!(bytes.len() > 500);

    let tmp = temp_path("incomplete-500");
    fs::write(&tmp, &bytes[..500]).expect("write 500-byte prefix");

    // verify must fail closed (non-zero, no OK:).
    assert_cli_fails("verify", &tmp, "incomplete prefix / verify");
    // report must fail closed.
    assert_cli_fails("report", &tmp, "incomplete prefix / report");

    // dump may remain lenient: either Err (mid-record) or Ok with JSONL — both acceptable.
    // Do not require dump failure for incomplete record-aligned prefixes.
    let _ = fs::remove_file(&tmp);
}

/// Golden default-calls1 still verifies OK (exit 0).
#[test]
fn verify_cli_default_calls1_ok() {
    let path = fixture_default_calls1();
    assert!(path.is_file(), "missing fixture {}", path.display());
    let output = Command::new(cli_bin())
        .args(["verify", path.to_str().expect("utf-8 path")])
        .output()
        .expect("spawn verify");
    assert!(
        output.status.success(),
        "verify default-calls1 must exit 0\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.lines().any(|l| l.starts_with("OK:")),
        "verify must print OK:\n{stdout}"
    );
}

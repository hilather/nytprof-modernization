//! CAPABILITY-SELFTEST / CAPABILITY-JSON-MVP: native offline capability CLI.
//!
//! Schema: `docs/schemas/capability-selftest-mvp-v0.md`

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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/v5/default-calls1/nytprof.out")
}

fn run_capability(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(cli_bin())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn capability {:?}: {e}", args));
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn assert_stable_markers(stdout: &str, label: &str) {
    for marker in [
        "OK: native capability self-test",
        "decode: yes",
        "report: yes",
        "verify: yes",
    ] {
        assert!(
            stdout.lines().any(|l| l == marker),
            "{label}: missing exact line {marker:?}\nstdout:\n{stdout}"
        );
    }
    assert!(
        stdout.lines().any(|l| l.starts_with("profile_ok: ")),
        "{label}: missing profile_ok: line\nstdout:\n{stdout}"
    );
}

#[test]
fn capability_selftest_default_ok() {
    let (code, stdout, stderr) = run_capability(&["capability"]);
    assert_eq!(
        code, 0,
        "capability must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_stable_markers(&stdout, "capability");

    let fixture = fixture_default_calls1();
    if fixture.is_file() {
        let line = stdout
            .lines()
            .find(|l| l.starts_with("profile_ok: "))
            .expect("profile_ok");
        assert_ne!(
            line, "profile_ok: skip",
            "golden fixture present → must not skip probe:\n{stdout}"
        );
        assert!(
            line.contains("nytprof.out") || line.contains("default-calls1"),
            "profile_ok path unexpected: {line}"
        );
    }
}

#[test]
fn capability_aliases_selftest_and_capabilities() {
    for sub in ["selftest", "capabilities"] {
        let (code, stdout, stderr) = run_capability(&[sub]);
        assert_eq!(
            code, 0,
            "{sub} must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert_stable_markers(&stdout, sub);
    }
}

#[test]
fn capability_forced_profile_ok() {
    let path = fixture_default_calls1();
    assert!(path.is_file(), "missing fixture {}", path.display());
    let p = path.to_str().expect("utf-8 path");
    let (code, stdout, stderr) = run_capability(&["capability", "--profile", p]);
    assert_eq!(
        code, 0,
        "capability --profile must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_stable_markers(&stdout, "capability --profile");
    let line = stdout
        .lines()
        .find(|l| l.starts_with("profile_ok: "))
        .expect("profile_ok");
    assert!(
        line.contains("nytprof.out"),
        "forced profile_ok unexpected: {line}"
    );
}

#[test]
fn capability_forced_bad_profile_fails() {
    let tmp = std::env::temp_dir().join(format!(
        "nytprof-capability-bad-{}-{}.out",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&tmp, b"NOTPROF 5 0\n").expect("write bad");
    let p = tmp.to_str().expect("utf-8");
    let (code, stdout, stderr) = run_capability(&["capability", "--profile", p]);
    let _ = std::fs::remove_file(&tmp);
    assert_ne!(
        code, 0,
        "capability on corrupt profile must fail closed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !stdout
            .lines()
            .any(|l| l == "OK: native capability self-test"),
        "must not print success OK block on failure\nstdout:\n{stdout}"
    );
}

#[test]
fn capability_twice_consistent_markers() {
    let (c1, o1, e1) = run_capability(&["capability"]);
    let (c2, o2, e2) = run_capability(&["capability"]);
    assert_eq!(c1, 0, "run1: {e1}");
    assert_eq!(c2, 0, "run2: {e2}");
    assert_stable_markers(&o1, "run1");
    assert_stable_markers(&o2, "run2");

    fn core(s: &str) -> Vec<&str> {
        let mut v: Vec<&str> = s
            .lines()
            .filter(|l| {
                l.starts_with("OK: native capability self-test")
                    || l.starts_with("decode: ")
                    || l.starts_with("report: ")
                    || l.starts_with("verify: ")
                    || l.starts_with("profile_ok: ")
            })
            .collect();
        v.sort_unstable();
        v
    }
    assert_eq!(
        core(&o1),
        core(&o2),
        "capability markers must match across runs"
    );
}

/// Engine flag must not block capability (reports this binary, not a backend).
#[test]
fn capability_works_under_engine_legacy() {
    let (code, stdout, stderr) = run_capability(&["--engine=legacy", "capability"]);
    assert_eq!(
        code, 0,
        "capability under --engine=legacy must still exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_stable_markers(&stdout, "--engine=legacy capability");
}

fn assert_json_capability_ok(stdout: &str, label: &str) -> Value {
    let v: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("{label}: stdout is not JSON: {e}\nstdout:\n{stdout}"));
    let obj = v
        .as_object()
        .unwrap_or_else(|| panic!("{label}: expected JSON object\nstdout:\n{stdout}"));
    for key in ["ok", "decode", "report", "verify"] {
        assert_eq!(
            obj.get(key),
            Some(&Value::Bool(true)),
            "{label}: field {key} must be true\nstdout:\n{stdout}"
        );
    }
    assert!(
        obj.contains_key("profile_ok"),
        "{label}: missing profile_ok\nstdout:\n{stdout}"
    );
    v
}

#[test]
fn capability_json_mode_ok() {
    let (code, stdout, stderr) = run_capability(&["capability", "--json"]);
    assert_eq!(
        code, 0,
        "capability --json must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    // Human markers must NOT be required in JSON mode (structured only).
    assert!(
        !stdout.contains("OK: native capability self-test"),
        "JSON mode should not emit greppable OK block\nstdout:\n{stdout}"
    );
    let v = assert_json_capability_ok(&stdout, "capability --json");
    let profile_ok = &v["profile_ok"];
    let fixture = fixture_default_calls1();
    if fixture.is_file() {
        let s = profile_ok
            .as_str()
            .unwrap_or_else(|| panic!("golden present → profile_ok string, got {profile_ok}"));
        assert!(
            s.contains("nytprof.out") || s.contains("default-calls1"),
            "profile_ok path unexpected: {s}"
        );
    } else {
        assert!(
            profile_ok.is_null(),
            "no golden → profile_ok null, got {profile_ok}"
        );
    }
}

#[test]
fn capability_format_json_equivalent() {
    let (c1, o1, e1) = run_capability(&["capability", "--json"]);
    let (c2, o2, e2) = run_capability(&["capability", "--format=json"]);
    let (c3, o3, e3) = run_capability(&["capability", "--format", "json"]);
    assert_eq!(c1, 0, "--json: {e1}");
    assert_eq!(c2, 0, "--format=json: {e2}");
    assert_eq!(c3, 0, "--format json: {e3}");
    let v1 = assert_json_capability_ok(&o1, "--json");
    let v2 = assert_json_capability_ok(&o2, "--format=json");
    let v3 = assert_json_capability_ok(&o3, "--format json");
    assert_eq!(v1, v2, "--json and --format=json must match");
    assert_eq!(v1, v3, "--json and --format json must match");
}

#[test]
fn capability_json_twice_consistent() {
    let (c1, o1, e1) = run_capability(&["capability", "--json"]);
    let (c2, o2, e2) = run_capability(&["capability", "--json"]);
    assert_eq!(c1, 0, "run1: {e1}");
    assert_eq!(c2, 0, "run2: {e2}");
    let v1 = assert_json_capability_ok(&o1, "json run1");
    let v2 = assert_json_capability_ok(&o2, "json run2");
    assert_eq!(v1, v2, "JSON capability must be stable across two runs");
}

#[test]
fn capability_json_forced_profile_ok() {
    let path = fixture_default_calls1();
    assert!(path.is_file(), "missing fixture {}", path.display());
    let p = path.to_str().expect("utf-8 path");
    let (code, stdout, stderr) = run_capability(&["capability", "--json", "--profile", p]);
    assert_eq!(
        code, 0,
        "capability --json --profile must exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let v = assert_json_capability_ok(&stdout, "capability --json --profile");
    let s = v["profile_ok"]
        .as_str()
        .expect("forced profile_ok must be a string");
    assert!(
        s.contains("nytprof.out"),
        "forced profile_ok unexpected: {s}"
    );
}

#[test]
fn capability_json_bad_profile_fails() {
    let tmp = std::env::temp_dir().join(format!(
        "nytprof-capability-json-bad-{}-{}.out",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&tmp, b"NOTPROF 5 0\n").expect("write bad");
    let p = tmp.to_str().expect("utf-8");
    let (code, stdout, stderr) = run_capability(&["capability", "--json", "--profile", p]);
    let _ = std::fs::remove_file(&tmp);
    assert_ne!(
        code, 0,
        "capability --json on corrupt profile must fail closed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    // No success object claiming ok:true on failure.
    if let Ok(v) = serde_json::from_str::<Value>(stdout.trim()) {
        assert_ne!(
            v.get("ok"),
            Some(&Value::Bool(true)),
            "must not emit ok:true on failure\nstdout:\n{stdout}"
        );
    }
}

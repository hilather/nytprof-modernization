//! PR-A02 / REPORT-HTML-SUBS-EXCL: real CLI `html --out-dir` publishes
//! `index-subs-excl.html` and lists it on stderr.
//!
//! Schema: `docs/schemas/html-subs-excl-index-mvp-v0.md`,
//! `docs/schemas/html-multifile-mvp-v0.md`.
//!
//! Drives the real `nytprof-dump` binary (`nytprof-cli` package). Asserts
//! default-calls1 multi-file site has exclusive sub index, pages link CSS,
//! stderr path list includes `index-subs-excl.html`, and leaf/mid **15/3**.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

fn unique_out_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "nytprof-cli-html-excl-{label}-{}-{}",
        std::process::id(),
        nanos
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn html_out_dir_writes_index_subs_excl_and_lists_on_stderr() {
    let fixture = fixture_default_calls1();
    assert!(fixture.is_file(), "missing fixture {}", fixture.display());
    let out = unique_out_dir("out-dir");
    let out_str = out.to_string_lossy();
    let fixture_str = fixture.to_string_lossy();

    let output = Command::new(cli_bin())
        .args(["html", fixture_str.as_ref(), "--out-dir", out_str.as_ref()])
        .output()
        .expect("spawn html --out-dir");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        code, 0,
        "html --out-dir must succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let excl_path = out.join("index-subs-excl.html");
    assert!(
        excl_path.is_file(),
        "CLI multi-file site must write index-subs-excl.html under {}",
        out.display()
    );
    let excl = fs::read_to_string(&excl_path).expect("read index-subs-excl.html");
    assert!(
        excl.contains("class=\"subs-excl\""),
        "excl page structure:\n{excl}"
    );
    assert!(
        excl.contains("href=\"style.css\""),
        "excl page must link style.css:\n{excl}"
    );
    assert!(
        !excl.to_ascii_lowercase().contains("<style"),
        "excl page must not inline <style>:\n{excl}"
    );
    assert!(
        excl.contains("main::leaf") && excl.contains(">15<"),
        "CLI excl leaf 15:\n{excl}"
    );
    assert!(
        excl.contains("main::mid") && excl.contains(">3<"),
        "CLI excl mid 3:\n{excl}"
    );

    assert!(
        stderr.contains("index-subs-excl.html"),
        "stderr must list index-subs-excl.html path:\n{stderr}"
    );

    let index = fs::read_to_string(out.join("index.html")).expect("index");
    assert!(
        index.contains("href=\"index-subs-excl.html\""),
        "index must link exclusive page:\n{index}"
    );

    let _ = fs::remove_dir_all(&out);
}

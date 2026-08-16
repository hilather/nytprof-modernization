//! PR-A01 / REPORT-HTML-SHARED-CSS: real CLI `html --out-dir` publishes shared
//! `style.css` and lists it on stderr.
//!
//! Schema: `docs/schemas/html-shared-css-structure-mvp-v0.md`,
//! `docs/schemas/html-multifile-mvp-v0.md`.
//!
//! Drives the real `nytprof-dump` binary (`nytprof-cli` package). Asserts
//! default-calls1 multi-file site has `style.css`, pages link it, stderr path
//! list includes `style.css`, and index still surfaces leaf/mid **15/3**.

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
        "nytprof-cli-html-css-{label}-{}-{}",
        std::process::id(),
        nanos
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn html_out_dir_writes_style_css_and_lists_on_stderr() {
    let fixture = fixture_default_calls1();
    assert!(fixture.is_file(), "missing fixture {}", fixture.display());
    let out = unique_out_dir("out-dir");
    let out_str = out.to_string_lossy();
    let fixture_str = fixture.to_string_lossy();

    // --no-flame: this suite contracts the shared page stylesheet
    // (external style.css, never inlined). The default-on flame SVG carries
    // its own SVG-scoped <style>, which is a separate contract
    // (html-optional-flame-mvp-v0.md) and would trip the no-inline assertion.
    let output = Command::new(cli_bin())
        .args([
            "html",
            fixture_str.as_ref(),
            "--out-dir",
            out_str.as_ref(),
            "--no-flame",
        ])
        .output()
        .expect("spawn html --out-dir");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        code, 0,
        "html --out-dir must succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Disk: shared CSS + index present.
    let style_path = out.join("style.css");
    assert!(
        style_path.is_file(),
        "CLI multi-file site must write style.css under {}",
        out.display()
    );
    let style = fs::read_to_string(&style_path).expect("read style.css");
    assert!(
        style.contains("body{") && !style.is_empty(),
        "style.css body non-empty"
    );
    assert!(out.join("index.html").is_file());
    assert!(out.join("source.html").is_file());
    assert!(out.join("file-1.html").is_file());

    // stderr path list includes style.css (CLI lists published paths).
    assert!(
        stderr.contains("style.css"),
        "stderr must list style.css path:\n{stderr}"
    );

    let index = fs::read_to_string(out.join("index.html")).expect("index");
    assert!(
        index.contains("href=\"style.css\""),
        "index must link style.css:\n{index}"
    );
    assert!(
        !index.to_ascii_lowercase().contains("<style"),
        "multi-file index must not inline <style>:\n{index}"
    );
    // Semantic counts still greppable on CLI-published index (15/3).
    assert!(
        index.contains("main::leaf") && index.contains(">15<"),
        "CLI index leaf 15:\n{index}"
    );
    assert!(
        index.contains("main::mid") && index.contains(">3<"),
        "CLI index mid 3:\n{index}"
    );

    let _ = fs::remove_dir_all(&out);
}

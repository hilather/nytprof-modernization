//! PR-A03 / REPORT-HTML-OPTIONAL-FLAME: real CLI `html` publishes folded +
//! native SVG flame artifacts under `--out-dir`. **Default on** (oracle
//! `nytprofhtml` parity: its `flame!` option defaults to 1); `--no-flame`
//! opts out (2026-08-15 contract amendment).
//!
//! Schema: `docs/schemas/html-optional-flame-mvp-v0.md`.
//!
//! Drives the real `nytprof-dump` binary (`nytprof-cli` package). Asserts
//! default-calls1: default path writes `all_stacks_by_time.{svg,folded}`,
//! lists them on stderr, and index links them; `--no-flame` has no flame
//! files or links; leaf/mid **15/3** remain greppable either way.

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
        "nytprof-cli-html-flame-{label}-{}-{}",
        std::process::id(),
        nanos
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn html_out_dir_default_writes_flame_files() {
    let fixture = fixture_default_calls1();
    assert!(fixture.is_file(), "missing fixture {}", fixture.display());
    let out = unique_out_dir("default");
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

    // Default on (oracle nytprofhtml parity): no flags needed for flame.
    assert!(
        out.join("all_stacks_by_time.svg").is_file(),
        "default path must write flame SVG"
    );
    assert!(
        out.join("all_stacks_by_time.folded").is_file(),
        "default path must write flame folded"
    );
    assert!(
        stderr.contains("all_stacks_by_time.svg"),
        "stderr must list flame files by default:\n{stderr}"
    );
    let index = fs::read_to_string(out.join("index.html")).expect("index");
    assert!(
        index.contains("href=\"all_stacks_by_time.svg\"") && index.contains("<svg"),
        "default index must link + inline flame SVG:\n{index}"
    );

    let _ = fs::remove_dir_all(&out);
}

#[test]
fn html_out_dir_no_flame_writes_no_flame_files() {
    let fixture = fixture_default_calls1();
    assert!(fixture.is_file(), "missing fixture {}", fixture.display());
    let out = unique_out_dir("noflame");
    let out_str = out.to_string_lossy();
    let fixture_str = fixture.to_string_lossy();

    let output = Command::new(cli_bin())
        .args([
            "html",
            fixture_str.as_ref(),
            "--out-dir",
            out_str.as_ref(),
            "--no-flame",
        ])
        .output()
        .expect("spawn html --out-dir --no-flame");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        code, 0,
        "html --out-dir --no-flame must succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    assert!(
        !out.join("all_stacks_by_time.svg").exists(),
        "--no-flame must not write flame SVG"
    );
    assert!(
        !out.join("all_stacks_by_time.folded").exists(),
        "--no-flame must not write flame folded"
    );
    assert!(
        !stderr.contains("all_stacks_by_time"),
        "stderr must not list flame files with --no-flame:\n{stderr}"
    );
    let index = fs::read_to_string(out.join("index.html")).expect("index");
    assert!(
        !index.contains("all_stacks_by_time"),
        "--no-flame index must not link flame:\n{index}"
    );

    let _ = fs::remove_dir_all(&out);
}

#[test]
fn html_out_dir_flame_writes_svg_and_folded_and_lists_on_stderr() {
    let fixture = fixture_default_calls1();
    assert!(fixture.is_file(), "missing fixture {}", fixture.display());
    let out = unique_out_dir("flame");
    let out_str = out.to_string_lossy();
    let fixture_str = fixture.to_string_lossy();

    let output = Command::new(cli_bin())
        .args([
            "html",
            fixture_str.as_ref(),
            "--out-dir",
            out_str.as_ref(),
            "--flame",
        ])
        .output()
        .expect("spawn html --out-dir --flame");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        code, 0,
        "html --out-dir --flame must succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let svg_path = out.join("all_stacks_by_time.svg");
    let folded_path = out.join("all_stacks_by_time.folded");
    assert!(
        svg_path.is_file(),
        "CLI --flame must write all_stacks_by_time.svg under {}",
        out.display()
    );
    assert!(
        folded_path.is_file(),
        "CLI --flame must write all_stacks_by_time.folded under {}",
        out.display()
    );

    let svg = fs::read_to_string(&svg_path).expect("svg");
    assert!(
        svg.contains("<svg") && svg.contains("main::leaf") && svg.contains("main::mid"),
        "SVG content:\n{svg}"
    );
    assert!(
        svg.contains("<rect ")
            && (svg.contains("calls: 15")
                || svg.contains("main::leaf (15)")
                || svg.contains("(15)")),
        "stacked rects + count 15:\n{svg}"
    );
    assert!(
        svg.contains("class=\"flame-link\"") && svg.contains("file-") && svg.contains("#L"),
        "SVG frames must link to source:\n{svg}"
    );
    assert!(
        svg.contains("inclusive:") && svg.contains("exclusive:"),
        "SVG title must include incl/excl:\n{svg}"
    );
    let mut rect_w = Vec::new();
    let mut rest = svg.as_str();
    while let Some(i) = rest.find("<rect ") {
        rest = &rest[i + 6..];
        if let Some(wpos) = rest.find("width=\"") {
            let after = &rest[wpos + 7..];
            if let Some(end) = after.find('"') {
                if let Ok(w) = after[..end].parse::<f64>() {
                    rect_w.push(w);
                }
            }
        }
    }
    let min_w = rect_w.iter().copied().fold(f64::INFINITY, f64::min);
    let max_w = rect_w.iter().copied().fold(0.0_f64, f64::max);
    assert!(
        rect_w.len() >= 4 && max_w > min_w + 0.5,
        "proportional widths (not all equal): {rect_w:?}"
    );

    let folded = fs::read_to_string(&folded_path).expect("folded");
    assert!(
        folded.contains("main::mid;main::leaf 15"),
        "folded mid→leaf 15:\n{folded}"
    );

    assert!(
        stderr.contains("all_stacks_by_time.svg"),
        "stderr must list SVG:\n{stderr}"
    );
    assert!(
        stderr.contains("all_stacks_by_time.folded"),
        "stderr must list folded:\n{stderr}"
    );

    let index = fs::read_to_string(out.join("index.html")).expect("index");
    assert!(
        index.contains("href=\"all_stacks_by_time.svg\""),
        "index must link SVG:\n{index}"
    );
    assert!(
        index.contains("href=\"all_stacks_by_time.folded\""),
        "index must link folded:\n{index}"
    );
    assert!(
        index.contains("<svg") && index.contains("class=\"flame-link\""),
        "index must inline SVG so hover/click work under file://:\n{index}"
    );
    assert!(
        !index.contains("<img ") && !index.contains("<object"),
        "preview must not use <img> or <object>:\n{index}"
    );
    assert!(
        index.contains("id=\"nytprof-flame-tip\""),
        "index must include hover tip:\n{index}"
    );
    assert!(
        index.contains("main::leaf") && index.contains(">15<"),
        "CLI index leaf 15:\n{index}"
    );
    assert!(
        index.contains("main::mid") && index.contains(">3<"),
        "CLI index mid 3:\n{index}"
    );

    // Shared CSS still published on flame path.
    assert!(out.join("style.css").is_file());

    let _ = fs::remove_dir_all(&out);
}

#[test]
fn html_single_file_flame_embeds_svg_on_stdout() {
    let fixture = fixture_default_calls1();
    assert!(fixture.is_file(), "missing fixture {}", fixture.display());
    let fixture_str = fixture.to_string_lossy();

    let output = Command::new(cli_bin())
        .args(["html", fixture_str.as_ref(), "--flame"])
        .output()
        .expect("spawn html --flame");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        code, 0,
        "html --flame must succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("<svg") || stdout.contains("nytprof-flame"),
        "single-file --flame must embed SVG:\n{stdout}"
    );
    assert!(
        stdout.contains("main::leaf") && stdout.contains(">15<"),
        "leaf 15 on single-file flame HTML:\n{stdout}"
    );
    assert!(
        stdout.contains("class=\"flame-link\"") && stdout.contains("href=\"#L"),
        "single-file flame must use in-page source anchors:\n{stdout}"
    );
    assert!(
        stdout.contains("id=\"nytprof-flame-tip\""),
        "single-file must include hover tip:\n{stdout}"
    );
}

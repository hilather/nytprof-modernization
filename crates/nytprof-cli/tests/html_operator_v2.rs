//! Operator HTML v2 (ADR-0012): chrome, index IA, files table, compact time.
//!
//! Schema: `docs/schemas/html-operator-v2-mvp-v0.md`.
//! Drives the real `nytprof-dump` binary (`html --out-dir`).

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
        "nytprof-cli-html-opv2-{label}-{}-{}",
        std::process::id(),
        nanos
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn html_operator_v2_cli_default_calls1() {
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

    let index = fs::read_to_string(out.join("index.html")).expect("index");
    let file1 = fs::read_to_string(out.join("file-1.html")).expect("file-1");
    let excl = fs::read_to_string(out.join("index-subs-excl.html")).expect("excl");
    let source = fs::read_to_string(out.join("source.html")).expect("source");
    let style = fs::read_to_string(out.join("style.css")).expect("style.css");
    let sort_js = fs::read_to_string(out.join("nytprof-sort.js")).expect("nytprof-sort.js");

    assert!(
        index.contains("Performance Profile Index") && index.contains("siteTitle"),
        "index siteTitle:\n{index}"
    );
    assert!(
        index.contains("id=\"subs_table\"") && index.contains("id=\"filestable\""),
        "index tables:\n{index}"
    );
    assert!(
        index.contains("See all") && index.contains("href=\"index-subs-excl.html\""),
        "See all N:\n{index}"
    );
    assert!(
        index.contains("href=\"source.html\""),
        "must-link source.html:\n{index}"
    );
    assert!(
        index.contains("time_line_events"),
        "event counts stay on index:\n{index}"
    );

    let lower_pages = format!("{index}{file1}{excl}{sort_js}").to_ascii_lowercase();
    assert!(
        !lower_pages.contains("jquery") && !lower_pages.contains("tablesorter"),
        "must not vendor jquery/tablesorter"
    );
    assert!(
        !style.to_ascii_lowercase().contains("jquery-")
            && !style.contains(".tablesorter")
            && !out.join("js").exists(),
        "must not ship jquery/tablesorter assets"
    );
    assert!(
        !style.contains(".c0") && !style.contains("c0{"),
        "no .c0 class selectors"
    );
    assert!(
        style.contains("--nyt-header-top") || style.contains("#ffb3b3"),
        "v2 tokens: {style}"
    );
    assert!(
        sort_js.contains("data-sort-default"),
        "sort JS must honor data-sort-default"
    );

    assert!(
        source.to_ascii_lowercase().contains("workload"),
        "source.html must be the application (workload):\n{source}"
    );

    let compact = index.contains("ms")
        || index.contains("µs")
        || index.contains("us")
        || index.contains('s');
    assert!(compact, "compact time units on index:\n{index}");

    let has_back = file1.contains("← Index") || file1.contains("&larr; Index");
    assert!(has_back, "file page back link:\n{file1}");
    assert!(
        excl.contains("← Index") || excl.contains("&larr; Index"),
        "excl back link:\n{excl}"
    );
    assert!(
        file1.contains("Statements")
            && file1.contains("Time on line")
            && file1.contains("Time in subs"),
        "file page six-column headers:\n{file1}"
    );

    let pkgs = fs::read_to_string(out.join("packages-callgraph.dot")).expect("packages dot");
    let subs_dot = fs::read_to_string(out.join("subs-callgraph.dot")).expect("subs dot");
    assert!(
        pkgs.starts_with("digraph") && pkgs.contains("->"),
        "packages-callgraph.dot:\n{pkgs}"
    );
    assert!(
        subs_dot.starts_with("digraph")
            && subs_dot.contains("main::mid")
            && subs_dot.contains("main::leaf")
            && subs_dot.contains("->"),
        "subs-callgraph.dot must include mid→leaf:\n{subs_dot}"
    );
    assert!(
        index.contains("packages-callgraph.dot") && index.contains("subs-callgraph.dot"),
        "index must link .dot files:\n{index}"
    );
    assert!(
        !index.to_ascii_lowercase().contains("jquery")
            && !subs_dot.to_ascii_lowercase().contains("jquery"),
        "graphviz path must not pull in jquery"
    );
    let lower_file = file1.to_ascii_lowercase();
    assert!(
        !lower_file.contains("warnings.pm#l1") && !file1.contains("file-1.html#L1\">main::mid"),
        "must not link stub (1,1) Perl callers at warnings L1:\n{file1}"
    );
    if file1.contains("calls_out") {
        assert!(
            file1.contains("main::leaf") || file1.contains("class=\"calls"),
            "call-out markup should name a callee:\n{file1}"
        );
    }

    let _ = fs::remove_dir_all(&out);
}

//! Operator HTML v1 (PR-2 / PR-4 / PR-5): seconds cells, source union, heat,
//! sub→source links, vanilla sort JS.
//!
//! Schema: `docs/schemas/html-shared-css-structure-mvp-v0.md`,
//! `docs/schemas/html-per-file-mvp-v0.md`,
//! `docs/schemas/html-sort-js-mvp-v0.md`.
//!
//! Drives the real `nytprof-dump` binary (`nytprof-cli` package) plus the
//! shipped `ProfileModel` for href construction. Default-calls1 still
//! surfaces leaf/mid **15/3**.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use nytprof_model::ProfileModel;

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
        "nytprof-cli-html-opv1-{label}-{}-{}",
        std::process::id(),
        nanos
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn html_operator_v1_cli_default_calls1() {
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
    let style = fs::read_to_string(out.join("style.css")).expect("style.css");
    let sort_js = fs::read_to_string(out.join("nytprof-sort.js")).expect("nytprof-sort.js");

    // Existing 15/3 leaf/mid asserts (returns / counts stay integer ticks).
    assert!(
        index.contains("main::leaf") && index.contains(">15<"),
        "CLI index leaf 15:\n{index}"
    );
    assert!(
        index.contains("main::mid") && index.contains(">3<"),
        "CLI index mid 3:\n{index}"
    );

    // HTML-only seconds: incl/excl cells have title= and seconds-ish display.
    let has_time_title = index.contains("title=")
        && (index.contains(" ticks\"") || index.contains("title=\""));
    assert!(has_time_title, "time cells must have title=:\n{index}");
    let subs_idx = index
        .find("id=\"subs_table\"")
        .or_else(|| index.find("subs_table"))
        .expect("subs_table");
    let subs_slice = &index[subs_idx..];
    let time_cells_look_like_seconds = subs_slice.contains('s') || subs_slice.contains('.');
    assert!(
        time_cells_look_like_seconds,
        "incl/excl cells should be seconds-ish (contain s or .), not bare ticks only:\n{subs_slice}"
    );

    // Source union / #Ln anchors.
    assert!(
        file1.contains("<tr") && file1.contains("id=\"L"),
        "file-1.html must have source <tr> rows with id=L:\n{file1}"
    );

    // Heat class names (not oracle c0–c3).
    assert!(
        style.contains("heat-hot"),
        "style.css must contain heat-hot:\n{style}"
    );
    assert!(style.contains("heat-high") && style.contains("heat-mid") && style.contains("heat-low"));
    assert!(
        index.contains("heat-hot")
            || index.contains("heat-high")
            || index.contains("heat-mid")
            || index.contains("heat-low"),
        "index rows must use heat-* classes:\n{index}"
    );

    // Vanilla sort JS published + referenced; no jquery/tablesorter.
    assert!(
        out.join("nytprof-sort.js").is_file(),
        "CLI multi-file site must write nytprof-sort.js"
    );
    assert!(
        index.contains("nytprof-sort.js") && index.contains("defer"),
        "index must reference nytprof-sort.js with defer:\n{index}"
    );
    let sort_l = sort_js.to_ascii_lowercase();
    assert!(
        !sort_l.contains("jquery") && !sort_l.contains("tablesorter"),
        "sort JS must not mention jquery/tablesorter"
    );
    assert!(
        sort_js.contains("nytprofSortInit") && sort_js.contains("data-sort"),
        "sort JS contract"
    );
    assert!(
        stderr.contains("nytprof-sort.js"),
        "stderr must list nytprof-sort.js:\n{stderr}"
    );

    // Sub → source href from model.sub_def (never hard-coded L3).
    let model = ProfileModel::from_path(&fixture).expect("ProfileModel::from_path");
    let d = model.sub_def("main::leaf").expect("sub_def(main::leaf)");
    let href = format!("file-{}.html#L{}", d.fid, d.first_line);
    assert!(
        index.contains(&format!("href=\"{href}\"")),
        "index must link main::leaf via model sub_def {href}:\n{index}"
    );

    let _ = fs::remove_dir_all(&out);
}

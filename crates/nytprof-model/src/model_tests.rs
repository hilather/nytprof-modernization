//! Unit tests for ProfileModel aggregation (A1–A9).
//!
//! Lives in a separate file so `lib.rs` stays under the 1k-line maintainability bar.

use super::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/v5")
        .join(name)
}

fn load_oracle_jsonl(path: &Path) -> Vec<Event> {
    let file = File::open(path).unwrap_or_else(|e| {
        panic!("open oracle jsonl {}: {e}", path.display());
    });
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    for (lineno, line) in reader.lines().enumerate() {
        let line = line.expect("read line");
        if line.trim().is_empty() {
            continue;
        }
        let event: Event = serde_json::from_str(&line).unwrap_or_else(|e| {
            panic!("parse jsonl {}:{}: {e}", path.display(), lineno + 1);
        });
        if event.tag == tags::END {
            continue;
        }
        events.push(event);
    }
    events
}

/// Expected returns for a subname by scanning oracle events (not hard-coded alone).
fn expected_returns_from_events(events: &[Event], subname: &str) -> u64 {
    events
        .iter()
        .filter(|e| e.tag == tags::SUB_RETURN)
        .filter(|e| e.args.get(3).and_then(|v| v.as_str()) == Some(subname))
        .count() as u64
}

/// Sum of SUB_CALLERS `count` for a (caller, called) edge by scanning oracle events.
fn expected_edge_count_from_events(events: &[Event], caller: &str, called: &str) -> u64 {
    events
        .iter()
        .filter(|e| e.tag == tags::SUB_CALLERS)
        .filter(|e| {
            e.args.get(7).and_then(|v| v.as_str()) == Some(called)
                && e.args.get(8).and_then(|v| v.as_str()) == Some(caller)
        })
        .map(|e| e.args.get(2).and_then(|v| v.as_u64()).unwrap_or(0))
        .sum()
}

fn count_tag(events: &[Event], tag: &str) -> u64 {
    events.iter().filter(|e| e.tag == tag).count() as u64
}

fn assert_call_edges_match(binary: &ProfileModel, oracle: &ProfileModel) {
    assert_eq!(
        binary.sub_callers_events, oracle.sub_callers_events,
        "sub_callers_events binary vs oracle"
    );
    assert_eq!(
        binary.call_edges.len(),
        oracle.call_edges.len(),
        "A7 call_edges map size"
    );
    for (key, o) in &oracle.call_edges {
        let b = binary
            .call_edges
            .get(key)
            .unwrap_or_else(|| panic!("binary missing edge {:?} -> {:?}", key.0, key.1));
        assert_eq!(b.count, o.count, "A7 {:?} count", key);
        assert_eq!(b.sites, o.sites, "A7 {:?} sites", key);
        assert_eq!(
            b.max_rec_depth, o.max_rec_depth,
            "A7 {:?} max_rec_depth",
            key
        );
        assert!(f64_close(b.incl, o.incl), "A7 {:?} incl", key);
        assert!(f64_close(b.excl, o.excl), "A7 {:?} excl", key);
        assert!(f64_close(b.reci, o.reci), "A7 {:?} reci", key);
    }
}

fn assert_source_lines_match(binary: &ProfileModel, oracle: &ProfileModel) {
    assert_eq!(
        binary.src_line_events, oracle.src_line_events,
        "src_line_events binary vs oracle"
    );
    assert_eq!(
        binary.source_lines, oracle.source_lines,
        "A8 source_lines binary vs oracle"
    );
}

fn assert_sub_defs_match(binary: &ProfileModel, oracle: &ProfileModel) {
    assert_eq!(
        binary.sub_info_events, oracle.sub_info_events,
        "sub_info_events binary vs oracle"
    );
    assert_eq!(
        binary.sub_defs, oracle.sub_defs,
        "A9 sub_defs binary vs oracle"
    );
}

/// Expected SubDef by scanning the last SUB_INFO for `subname` (last write wins).
fn expected_sub_def_from_events(events: &[Event], subname: &str) -> Option<SubDef> {
    events
        .iter()
        .filter(|e| e.tag == tags::SUB_INFO)
        .filter(|e| e.args.get(3).and_then(|v| v.as_str()) == Some(subname))
        .map(|e| SubDef {
            fid: e.args[0].as_u64().unwrap() as u32,
            first_line: e.args[1].as_u64().unwrap() as u32,
            last_line: e.args[2].as_u64().unwrap() as u32,
        })
        .last()
}

fn assert_models_a1_and_workload(binary: &ProfileModel, oracle: &ProfileModel) {
    assert_eq!(
        binary.time_line_events, oracle.time_line_events,
        "A1 time_line_events binary vs oracle jsonl"
    );
    assert_eq!(
        binary.time_block_events, oracle.time_block_events,
        "A2 time_block_events"
    );
    assert_eq!(
        binary.discount_events, oracle.discount_events,
        "A3 discount_events"
    );

    for name in ["main::leaf", "main::mid"] {
        let b = binary
            .sub_total(name)
            .unwrap_or_else(|| panic!("binary missing {name}"));
        let o = oracle
            .sub_total(name)
            .unwrap_or_else(|| panic!("oracle missing {name}"));
        assert_eq!(b.returns, o.returns, "{name} returns");
        assert!(
            f64_close(b.incl, o.incl),
            "{name} incl binary={} oracle={}",
            b.incl,
            o.incl
        );
        assert!(
            f64_close(b.excl, o.excl),
            "{name} excl binary={} oracle={}",
            b.excl,
            o.excl
        );
    }
}

fn check_fixture(name: &str) {
    let dir = fixture_dir(name);
    let out = dir.join("nytprof.out");
    let jsonl = dir.join("readstream.jsonl");
    assert!(out.is_file(), "missing {}", out.display());
    assert!(jsonl.is_file(), "missing {}", jsonl.display());

    let binary_events = nytprof_format_v5::decode_path(&out).expect("decode nytprof.out");
    let binary = ProfileModel::from_events(&binary_events).expect("aggregate binary");
    // Also exercise from_path.
    let from_path = ProfileModel::from_path(&out).expect("from_path");
    assert_eq!(binary.time_line_events, from_path.time_line_events);

    // Self-consistency: A1 equals TIME_LINE count in the same decode.
    let tl_count = count_tag(&binary_events, tags::TIME_LINE);
    assert_eq!(
        binary.time_line_events, tl_count,
        "time_line_events must equal TIME_LINE count in decode"
    );
    assert_eq!(
        binary.time_block_events,
        count_tag(&binary_events, tags::TIME_BLOCK)
    );
    assert_eq!(
        binary.discount_events,
        count_tag(&binary_events, tags::DISCOUNT)
    );
    assert_eq!(
        binary.sub_entry_events,
        count_tag(&binary_events, tags::SUB_ENTRY)
    );
    assert_eq!(
        binary.sub_callers_events,
        count_tag(&binary_events, tags::SUB_CALLERS)
    );
    assert_eq!(
        binary.src_line_events,
        count_tag(&binary_events, tags::SRC_LINE)
    );
    assert_eq!(
        binary.sub_info_events,
        count_tag(&binary_events, tags::SUB_INFO)
    );

    let oracle_events = load_oracle_jsonl(&jsonl);
    let oracle = ProfileModel::from_events(&oracle_events).expect("aggregate oracle jsonl");

    // Expected returns derived by scanning oracle jsonl events.
    let leaf_expected = expected_returns_from_events(&oracle_events, "main::leaf");
    let mid_expected = expected_returns_from_events(&oracle_events, "main::mid");
    assert_eq!(
        leaf_expected, 15,
        "workload: 3 mids × 5 leaves → main::leaf returns (from oracle jsonl scan)"
    );
    assert_eq!(
        mid_expected, 3,
        "workload: 3 mid calls → main::mid returns (from oracle jsonl scan)"
    );
    assert_eq!(
        oracle.sub_total("main::leaf").map(|t| t.returns),
        Some(leaf_expected)
    );
    assert_eq!(
        oracle.sub_total("main::mid").map(|t| t.returns),
        Some(mid_expected)
    );
    assert_eq!(
        binary.sub_total("main::leaf").map(|t| t.returns),
        Some(leaf_expected)
    );
    assert_eq!(
        binary.sub_total("main::mid").map(|t| t.returns),
        Some(mid_expected)
    );

    // A7 workload edges: counts scanned from oracle SUB_CALLERS (not invented).
    let mid_to_leaf =
        expected_edge_count_from_events(&oracle_events, "main::mid", "main::leaf");
    let runtime_to_mid =
        expected_edge_count_from_events(&oracle_events, "main::RUNTIME", "main::mid");
    assert_eq!(
        mid_to_leaf, 15,
        "workload edge main::mid -> main::leaf count (oracle scan)"
    );
    assert_eq!(
        runtime_to_mid, 3,
        "workload edge main::RUNTIME -> main::mid count (oracle scan)"
    );
    assert_eq!(
        binary
            .call_edge("main::mid", "main::leaf")
            .map(|e| e.count),
        Some(mid_to_leaf)
    );
    assert_eq!(
        from_path
            .call_edge("main::mid", "main::leaf")
            .map(|e| e.count),
        Some(mid_to_leaf)
    );
    assert_eq!(
        binary
            .call_edge("main::RUNTIME", "main::mid")
            .map(|e| e.count),
        Some(runtime_to_mid)
    );
    assert_eq!(
        oracle
            .call_edge("main::mid", "main::leaf")
            .map(|e| e.count),
        Some(mid_to_leaf)
    );

    // A8 source lines present for workload.
    assert!(binary.src_line_events > 0, "src_line_events > 0");
    assert!(!binary.source_lines.is_empty(), "source_lines non-empty");
    let src5 = binary
        .source_line(1, 5)
        .expect("source_line(1,5) from workload.pl");
    assert!(
        src5.contains("x++") || src5.contains("for 1 .. 50"),
        "source_line(1,5) should be leaf loop body, got {src5:?}"
    );
    assert!(binary.has_source(1, 5));

    assert_models_a1_and_workload(&binary, &oracle);
    assert_call_edges_match(&binary, &oracle);
    assert_source_lines_match(&binary, &oracle);
    assert_sub_defs_match(&binary, &oracle);

    // Full line_totals, block_line_totals, and sub maps must match for these fixtures.
    assert_eq!(
        binary.line_totals, oracle.line_totals,
        "A4 line_totals binary vs oracle"
    );
    assert_eq!(
        binary.block_line_totals, oracle.block_line_totals,
        "A4b block_line_totals binary vs oracle"
    );
    assert_eq!(
        binary.sub_return_totals.len(),
        oracle.sub_return_totals.len(),
        "A5 sub map size"
    );
    for (name, o) in &oracle.sub_return_totals {
        let b = binary
            .sub_return_totals
            .get(name)
            .unwrap_or_else(|| panic!("binary missing sub {name}"));
        assert_eq!(b.returns, o.returns, "A5 {name} returns");
        assert!(f64_close(b.incl, o.incl), "A5 {name} incl");
        assert!(f64_close(b.excl, o.excl), "A5 {name} excl");
    }

    // A9 — workload sub defs (main::leaf / main::mid) present and match oracle dump.
    assert!(binary.sub_info_events > 0, "sub_info_events > 0");
    assert!(!binary.sub_defs.is_empty(), "sub_defs non-empty");
    let leaf_def = binary
        .sub_def("main::leaf")
        .expect("sub_def(main::leaf)");
    let mid_def = binary.sub_def("main::mid").expect("sub_def(main::mid)");
    let leaf_expected = expected_sub_def_from_events(&oracle_events, "main::leaf")
        .expect("oracle SUB_INFO main::leaf");
    let mid_expected = expected_sub_def_from_events(&oracle_events, "main::mid")
        .expect("oracle SUB_INFO main::mid");
    assert_eq!(*leaf_def, leaf_expected, "main::leaf SubDef");
    assert_eq!(*mid_def, mid_expected, "main::mid SubDef");
    assert_eq!(from_path.sub_def("main::leaf"), Some(&leaf_expected));
    assert_eq!(from_path.sub_def("main::mid"), Some(&mid_expected));

    let wl = binary.workload_sub_names();
    assert!(wl.iter().any(|n| n == "main::leaf"), "{wl:?}");
    assert!(wl.iter().any(|n| n == "main::mid"), "{wl:?}");
    assert_eq!(binary.fid_basename(1), Some("workload.pl"));

    eprintln!("=== {name} ===\n{}", binary.debug_summary());
}

#[test]
fn default_calls1_binary_matches_oracle_jsonl() {
    check_fixture("default-calls1");
}

#[test]
fn default_calls2_binary_matches_oracle_jsonl() {
    // calls2 has SUB_ENTRY (calls=1).
    check_fixture("default-calls2");
    let dir = fixture_dir("default-calls2");
    let model = ProfileModel::from_path(dir.join("nytprof.out")).unwrap();
    assert!(
        model.sub_entry_events > 0,
        "default-calls2 must record SUB_ENTRY"
    );
}

#[test]
fn blocks_calls1_binary_matches_oracle_jsonl() {
    // blocks=1 emits TIME_BLOCK instead of TIME_LINE; same workload leaf/mid returns.
    check_fixture("blocks-calls1");
    let dir = fixture_dir("blocks-calls1");
    let model = ProfileModel::from_path(dir.join("nytprof.out")).expect("from_path");
    assert!(
        model.time_block_events > 0,
        "blocks-calls1 must record TIME_BLOCK (got {})",
        model.time_block_events
    );
    assert_eq!(
        model.time_line_events, 0,
        "blocks-calls1 should have no TIME_LINE when blocks=1"
    );
}

#[test]
fn default_calls1_workload_subs() {
    let path = fixture_dir("default-calls1").join("nytprof.out");
    assert!(path.is_file(), "missing {}", path.display());
    let model = ProfileModel::from_path(&path).expect("model");

    assert!(model.time_line_events > 0);
    assert_eq!(model.time_block_events, 0);
    // TIME_LINE still fills line_totals; no TIME_BLOCK → empty A4b.
    assert!(!model.line_totals.is_empty(), "A4 line_totals from TIME_LINE");
    assert!(
        model.block_line_totals.is_empty(),
        "A4b empty when no TIME_BLOCK"
    );

    let leaf = model.sub_returns("main::leaf").expect("main::leaf");
    assert_eq!(leaf.returns, 15);
    let mid = model.sub_returns("main::mid").expect("main::mid");
    assert_eq!(mid.returns, 3);

    // mid exclusive should be less than inclusive (leaf work nested)
    assert!(mid.excl < mid.incl);
    assert_eq!(leaf.incl, leaf.excl);

    assert!(model.files.contains_key(&1));
    assert_eq!(model.fid_basename(1), Some("workload.pl"));

    // A7: mid → leaf count 15; RUNTIME → mid count 3 (from SUB_CALLERS).
    let mid_leaf = model
        .call_edge("main::mid", "main::leaf")
        .expect("mid→leaf edge");
    assert_eq!(mid_leaf.count, 15);
    let rt_mid = model
        .call_edge("main::RUNTIME", "main::mid")
        .expect("RUNTIME→mid edge");
    assert_eq!(rt_mid.count, 3);

    // A9: SUB_INFO defs for workload subs (oracle dump: leaf 1/3–7, mid 1/8–12).
    assert!(model.sub_info_events > 0);
    let leaf_def = model.sub_def("main::leaf").expect("main::leaf SubDef");
    assert_eq!(leaf_def.fid, 1);
    assert_eq!(leaf_def.first_line, 3);
    assert_eq!(leaf_def.last_line, 7);
    let mid_def = model.sub_def("main::mid").expect("main::mid SubDef");
    assert_eq!(mid_def.fid, 1);
    assert_eq!(mid_def.first_line, 8);
    assert_eq!(mid_def.last_line, 12);
}

#[test]
fn blocks_calls1_workload_subs() {
    let path = fixture_dir("blocks-calls1").join("nytprof.out");
    assert!(path.is_file(), "missing {}", path.display());
    let model = ProfileModel::from_path(&path).expect("from_path");

    // blocks=1: statement timing is TIME_BLOCK; A4 line_totals filled from TIME_BLOCK.
    assert!(
        model.time_block_events > 0,
        "time_block_events > 0, got {}",
        model.time_block_events
    );
    assert_eq!(model.time_line_events, 0);
    assert!(
        !model.line_totals.is_empty(),
        "A4 line_totals must be non-empty from TIME_BLOCK"
    );
    // Hot loop line in workload.pl: 780 TIME_BLOCK hits (oracle dump aggregation).
    let lt = model
        .line_total(1, 5)
        .expect("line_total(1,5) from TIME_BLOCK");
    assert_eq!(lt.calls, 780, "line_total(1,5).calls from TIME_BLOCK scan");
    assert!(lt.ticks > 0, "line_total(1,5).ticks > 0, got {}", lt.ticks);

    assert!(
        !model.block_line_totals.is_empty(),
        "A4b block_line_totals non-empty when TIME_BLOCK present"
    );
    let any_block_ticks = model
        .block_line_totals
        .values()
        .any(|t| t.ticks > 0 && t.calls > 0);
    assert!(any_block_ticks, "block_line_totals must have positive ticks");

    let leaf = model.sub_returns("main::leaf").expect("main::leaf");
    assert_eq!(leaf.returns, 15);
    let mid = model.sub_returns("main::mid").expect("main::mid");
    assert_eq!(mid.returns, 3);

    assert!(mid.excl < mid.incl);
    assert_eq!(leaf.incl, leaf.excl);

    assert_eq!(model.fid_basename(1), Some("workload.pl"));

    let mid_leaf = model
        .call_edge("main::mid", "main::leaf")
        .expect("mid→leaf edge");
    assert_eq!(mid_leaf.count, 15);
    let rt_mid = model
        .call_edge("main::RUNTIME", "main::mid")
        .expect("RUNTIME→mid edge");
    assert_eq!(rt_mid.count, 3);

    // A9: same leaf/mid defs as default-calls1.
    assert!(model.sub_info_events > 0);
    let leaf_def = model.sub_def("main::leaf").expect("main::leaf SubDef");
    assert_eq!(
        *leaf_def,
        SubDef {
            fid: 1,
            first_line: 3,
            last_line: 7
        }
    );
    let mid_def = model.sub_def("main::mid").expect("main::mid SubDef");
    assert_eq!(
        *mid_def,
        SubDef {
            fid: 1,
            first_line: 8,
            last_line: 12
        }
    );
}

#[test]
fn accumulate_single_time_line() {
    let mut m = ProfileModel::new();
    m.accumulate(&Event::new(
        0,
        tags::TIME_LINE,
        vec![Value::from(10), Value::from(1u64), Value::from(5u64)],
    ))
    .unwrap();
    m.accumulate(&Event::new(
        1,
        tags::TIME_LINE,
        vec![Value::from(3), Value::from(1u64), Value::from(5u64)],
    ))
    .unwrap();
    assert_eq!(m.time_line_events, 2);
    assert_eq!(
        m.line_total(1, 5),
        Some(LineTotal {
            calls: 2,
            ticks: 13
        })
    );
    assert!(m.block_line_totals.is_empty());
}

#[test]
fn accumulate_time_block_fills_line_and_block_totals() {
    // TIME_BLOCK args: ticks, fid, line, block_line, sub_line
    let mut m = ProfileModel::new();
    m.accumulate(&Event::new(
        0,
        tags::TIME_BLOCK,
        vec![
            Value::from(10),
            Value::from(1u64),
            Value::from(5u64),
            Value::from(4u64),
            Value::from(3u64),
        ],
    ))
    .unwrap();
    m.accumulate(&Event::new(
        1,
        tags::TIME_BLOCK,
        vec![
            Value::from(7),
            Value::from(1u64),
            Value::from(5u64),
            Value::from(4u64),
            Value::from(3u64),
        ],
    ))
    .unwrap();
    assert_eq!(m.time_block_events, 2);
    assert_eq!(m.time_line_events, 0);
    assert_eq!(
        m.line_total(1, 5),
        Some(LineTotal {
            calls: 2,
            ticks: 17
        })
    );
    assert_eq!(
        m.block_line_total(1, 4),
        Some(LineTotal {
            calls: 2,
            ticks: 17
        })
    );
    // block_line key is distinct from statement line when they differ.
    assert!(m.line_total(1, 4).is_none());
}

#[test]
fn accumulate_sub_return() {
    let mut m = ProfileModel::new();
    m.accumulate(&Event::new(
        0,
        tags::SUB_RETURN,
        vec![
            Value::from(1u64),
            Value::from(10.5),
            Value::from(4.25),
            Value::String("main::leaf".into()),
        ],
    ))
    .unwrap();
    m.accumulate(&Event::new(
        1,
        tags::SUB_RETURN,
        vec![
            Value::from(1u64),
            Value::from(1.5),
            Value::from(0.75),
            Value::String("main::leaf".into()),
        ],
    ))
    .unwrap();
    let t = m.sub_total("main::leaf").unwrap();
    assert_eq!(t.returns, 2);
    assert!(f64_close(t.incl, 12.0));
    assert!(f64_close(t.excl, 5.0));
    assert!(f64_close(t.incl_ticks(), 12.0));
    assert!(f64_close(t.excl_ticks(), 5.0));
}

#[test]
fn accumulate_sub_callers_merges_sites() {
    let mut m = ProfileModel::new();
    // Same (caller, called) from two sites: sum count/times, max rec_depth, sites=2.
    m.accumulate(&Event::new(
        0,
        tags::SUB_CALLERS,
        vec![
            Value::from(1u64),
            Value::from(10u64),
            Value::from(5u64),
            Value::from(1.0),
            Value::from(0.5),
            Value::from(0.0),
            Value::from(1u64),
            Value::String("main::leaf".into()),
            Value::String("main::mid".into()),
        ],
    ))
    .unwrap();
    m.accumulate(&Event::new(
        1,
        tags::SUB_CALLERS,
        vec![
            Value::from(1u64),
            Value::from(11u64),
            Value::from(10u64),
            Value::from(2.0),
            Value::from(1.5),
            Value::from(0.25),
            Value::from(3u64),
            Value::String("main::leaf".into()),
            Value::String("main::mid".into()),
        ],
    ))
    .unwrap();
    assert_eq!(m.sub_callers_events, 2);
    let e = m.call_edge("main::mid", "main::leaf").expect("edge");
    assert_eq!(e.count, 15);
    assert_eq!(e.sites, 2);
    assert_eq!(e.max_rec_depth, 3);
    assert!(f64_close(e.incl, 3.0));
    assert!(f64_close(e.excl, 2.0));
    assert!(f64_close(e.reci, 0.25));
}

#[test]
fn accumulate_src_line_last_write_wins() {
    let mut m = ProfileModel::new();
    m.accumulate(&Event::new(
        0,
        tags::SRC_LINE,
        vec![
            Value::from(1u64),
            Value::from(5u64),
            Value::String("first\n".into()),
        ],
    ))
    .unwrap();
    m.accumulate(&Event::new(
        1,
        tags::SRC_LINE,
        vec![
            Value::from(1u64),
            Value::from(5u64),
            Value::String("    $x++ for 1 .. 50;\n".into()),
        ],
    ))
    .unwrap();
    assert_eq!(m.src_line_events, 2);
    assert_eq!(m.source_lines.len(), 1);
    assert!(m.has_source(1, 5));
    assert_eq!(m.source_line(1, 5), Some("    $x++ for 1 .. 50;\n"));
    assert!(!m.has_source(1, 99));
}

#[test]
fn accumulate_sub_info_last_write_wins() {
    // Args: fid, first_line, last_line, name
    let mut m = ProfileModel::new();
    m.accumulate(&Event::new(
        0,
        tags::SUB_INFO,
        vec![
            Value::from(1u64),
            Value::from(1u64),
            Value::from(2u64),
            Value::String("main::leaf".into()),
        ],
    ))
    .unwrap();
    m.accumulate(&Event::new(
        1,
        tags::SUB_INFO,
        vec![
            Value::from(1u64),
            Value::from(3u64),
            Value::from(7u64),
            Value::String("main::leaf".into()),
        ],
    ))
    .unwrap();
    m.accumulate(&Event::new(
        2,
        tags::SUB_INFO,
        vec![
            Value::from(1u64),
            Value::from(8u64),
            Value::from(12u64),
            Value::String("main::mid".into()),
        ],
    ))
    .unwrap();
    assert_eq!(m.sub_info_events, 3);
    assert_eq!(m.sub_defs.len(), 2);
    assert_eq!(
        m.sub_def("main::leaf"),
        Some(&SubDef {
            fid: 1,
            first_line: 3,
            last_line: 7
        })
    );
    assert_eq!(
        m.sub_def("main::mid"),
        Some(&SubDef {
            fid: 1,
            first_line: 8,
            last_line: 12
        })
    );
    assert!(m.sub_def("missing").is_none());
}

#[test]
fn default_calls1_call_edges_and_source() {
    let path = fixture_dir("default-calls1").join("nytprof.out");
    let jsonl = fixture_dir("default-calls1").join("readstream.jsonl");
    let model = ProfileModel::from_path(&path).expect("model");
    let oracle_events = load_oracle_jsonl(&jsonl);
    let oracle = ProfileModel::from_events(&oracle_events).expect("oracle model");

    let mid_leaf =
        expected_edge_count_from_events(&oracle_events, "main::mid", "main::leaf");
    let runtime_mid =
        expected_edge_count_from_events(&oracle_events, "main::RUNTIME", "main::mid");
    assert_eq!(mid_leaf, 15);
    assert_eq!(runtime_mid, 3);

    assert_eq!(
        model.call_edge("main::mid", "main::leaf").map(|e| e.count),
        Some(15)
    );
    assert_eq!(
        model
            .call_edge("main::RUNTIME", "main::mid")
            .map(|e| e.count),
        Some(3)
    );
    assert_call_edges_match(&model, &oracle);
    assert_source_lines_match(&model, &oracle);

    let text = model.source_line(1, 5).expect("src");
    assert!(text.contains("x++") || text.contains("for 1 .. 50"), "{text:?}");
    assert!(model.src_line_events > 0);
    assert!(!model.source_lines.is_empty());

    // A9 — binary matches oracle for leaf/mid defs.
    assert_sub_defs_match(&model, &oracle);
    assert!(model.sub_info_events > 0);
    let leaf_def = model.sub_def("main::leaf").expect("leaf def");
    assert_eq!(leaf_def.fid, 1);
    assert_eq!(leaf_def.first_line, 3);
    assert_eq!(leaf_def.last_line, 7);
    let mid_def = model.sub_def("main::mid").expect("mid def");
    assert_eq!(mid_def.fid, 1);
    assert_eq!(mid_def.first_line, 8);
    assert_eq!(mid_def.last_line, 12);

    let summary = model.debug_summary();
    assert!(summary.contains("SUB_CALLERS="), "{summary}");
    assert!(summary.contains("source_lines="), "{summary}");
    assert!(summary.contains("SUB_INFO="), "{summary}");
    assert!(summary.contains("main::mid -> main::leaf"), "{summary}");
}

#[test]
fn default_calls1_sub_defs() {
    let path = fixture_dir("default-calls1").join("nytprof.out");
    let jsonl = fixture_dir("default-calls1").join("readstream.jsonl");
    let model = ProfileModel::from_path(&path).expect("from_path");
    let oracle_events = load_oracle_jsonl(&jsonl);
    let oracle = ProfileModel::from_events(&oracle_events).expect("oracle model");

    assert!(model.sub_info_events > 0, "sub_info_events > 0");
    assert_sub_defs_match(&model, &oracle);

    let leaf = model.sub_def("main::leaf").expect("main::leaf");
    assert_eq!(leaf.fid, 1);
    assert_eq!(leaf.first_line, 3);
    assert_eq!(leaf.last_line, 7);

    let mid = model.sub_def("main::mid").expect("main::mid");
    assert_eq!(mid.fid, 1);
    assert_eq!(mid.first_line, 8);
    assert_eq!(mid.last_line, 12);

    // Oracle dump scan must agree (not hard-coded alone).
    assert_eq!(
        *leaf,
        expected_sub_def_from_events(&oracle_events, "main::leaf").unwrap()
    );
    assert_eq!(
        *mid,
        expected_sub_def_from_events(&oracle_events, "main::mid").unwrap()
    );
}

#[test]
fn blocks_calls1_sub_defs() {
    let path = fixture_dir("blocks-calls1").join("nytprof.out");
    let model = ProfileModel::from_path(&path).expect("from_path");
    assert!(model.sub_info_events > 0);
    assert_eq!(
        model.sub_def("main::leaf"),
        Some(&SubDef {
            fid: 1,
            first_line: 3,
            last_line: 7
        })
    );
    assert_eq!(
        model.sub_def("main::mid"),
        Some(&SubDef {
            fid: 1,
            first_line: 8,
            last_line: 12
        })
    );
}

/// Load committed `aggregates.oracle.json` and compare A5 returns + A7 edges
/// against a native `ProfileModel` built from `nytprof.out`.
///
/// This is the cargo-test gate that fails if native edge counts drift from
/// the Python oracle aggregator baseline.
fn check_native_vs_aggregates_oracle_json(name: &str) {
    let dir = fixture_dir(name);
    let out = dir.join("nytprof.out");
    let oracle_path = dir.join("aggregates.oracle.json");
    assert!(out.is_file(), "missing {}", out.display());
    assert!(
        oracle_path.is_file(),
        "missing {} (regenerate with tools/oracle/aggregate_from_jsonl.py)",
        oracle_path.display()
    );

    let model = ProfileModel::from_path(&out).expect("from_path nytprof.out");
    let raw = std::fs::read_to_string(&oracle_path).expect("read aggregates.oracle.json");
    let oracle: Value = serde_json::from_str(&raw).expect("parse aggregates.oracle.json");

    assert_eq!(
        oracle.get("schema").and_then(|v| v.as_str()),
        Some("aggregate-comparison-v0"),
        "unexpected schema in {}",
        oracle_path.display()
    );

    // A5 — leaf / mid returns (and incl/excl) from oracle JSON.
    let subs = oracle
        .get("sub_return_totals")
        .and_then(|v| v.as_object())
        .expect("sub_return_totals object");
    for sub in ["main::leaf", "main::mid"] {
        let o = subs
            .get(sub)
            .unwrap_or_else(|| panic!("oracle missing sub {sub}"));
        let o_returns = o
            .get("returns")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| panic!("oracle {sub}.returns"));
        let native = model
            .sub_returns(sub)
            .unwrap_or_else(|| panic!("native missing {sub}"));
        assert_eq!(
            native.returns, o_returns,
            "A5 {sub} returns: native={} oracle={}",
            native.returns, o_returns
        );
        let o_incl = o.get("incl").and_then(|v| v.as_f64()).unwrap();
        let o_excl = o.get("excl").and_then(|v| v.as_f64()).unwrap();
        assert!(
            f64_close(native.incl, o_incl),
            "A5 {sub} incl: native={} oracle={}",
            native.incl, o_incl
        );
        assert!(
            f64_close(native.excl, o_excl),
            "A5 {sub} excl: native={} oracle={}",
            native.excl, o_excl
        );
    }

    // A7 — call_edges; gate on main::mid → main::leaf count.
    let edges = oracle
        .get("call_edges")
        .and_then(|v| v.as_object())
        .expect("call_edges object in aggregates.oracle.json");
    let mid_leaf_key = ProfileModel::call_edge_key("main::mid", "main::leaf");
    let o_edge = edges.get(&mid_leaf_key).unwrap_or_else(|| {
        panic!(
            "oracle missing edge {mid_leaf_key}; keys={:?}",
            edges.keys().take(8).collect::<Vec<_>>()
        )
    });
    let o_count = o_edge
        .get("count")
        .and_then(|v| v.as_u64())
        .expect("oracle edge count");
    let native_edge = model
        .call_edge("main::mid", "main::leaf")
        .expect("native mid→leaf edge");
    assert_eq!(
        native_edge.count, o_count,
        "A7 main::mid -> main::leaf count: native={} oracle={}",
        native_edge.count, o_count
    );
    if let Some(o_incl) = o_edge.get("incl").and_then(|v| v.as_f64()) {
        assert!(
            f64_close(native_edge.incl, o_incl),
            "A7 mid→leaf incl native={} oracle={}",
            native_edge.incl, o_incl
        );
    }
    if let Some(o_excl) = o_edge.get("excl").and_then(|v| v.as_f64()) {
        assert!(
            f64_close(native_edge.excl, o_excl),
            "A7 mid→leaf excl native={} oracle={}",
            native_edge.excl, o_excl
        );
    }

    // Workload RUNTIME → mid (count 3).
    let rt_key = ProfileModel::call_edge_key("main::RUNTIME", "main::mid");
    if let Some(o_rt) = edges.get(&rt_key) {
        let o_rt_count = o_rt.get("count").and_then(|v| v.as_u64()).unwrap();
        let n_rt = model
            .call_edge("main::RUNTIME", "main::mid")
            .expect("native RUNTIME→mid");
        assert_eq!(
            n_rt.count, o_rt_count,
            "A7 main::RUNTIME -> main::mid count: native={} oracle={}",
            n_rt.count, o_rt_count
        );
    }

    // A8 — source_line_count when present in baseline.
    if let Some(o_src) = oracle.get("source_line_count").and_then(|v| v.as_u64()) {
        assert_eq!(
            model.source_line_count(),
            o_src,
            "A8 source_line_count native vs oracle"
        );
    }
    // Workload sample: oracle stores rstrip'd (and optionally truncated) text.
    if let Some(sample) = oracle.get("source_sample").and_then(|v| v.as_object()) {
        if let Some(o_text) = sample.get("1:5").and_then(|v| v.as_str()) {
            let n_text = model.source_line(1, 5).expect("native source 1:5");
            let n_prefix = n_text.trim_end_matches('\n');
            assert!(
                n_prefix == o_text || n_prefix.starts_with(o_text),
                "source_sample 1:5 mismatch native={n_prefix:?} oracle={o_text:?}"
            );
        }
    }

    // A9 — sub_defs when present in regenerated baseline.
    if let Some(subs) = oracle.get("sub_defs").and_then(|v| v.as_object()) {
        for name in ["main::leaf", "main::mid"] {
            let o = subs
                .get(name)
                .unwrap_or_else(|| panic!("oracle sub_defs missing {name}"));
            let n = model
                .sub_def(name)
                .unwrap_or_else(|| panic!("native missing sub_def {name}"));
            assert_eq!(
                n.fid,
                o.get("fid").and_then(|v| v.as_u64()).unwrap() as u32,
                "A9 {name} fid"
            );
            assert_eq!(
                n.first_line,
                o.get("first_line").and_then(|v| v.as_u64()).unwrap() as u32,
                "A9 {name} first_line"
            );
            assert_eq!(
                n.last_line,
                o.get("last_line").and_then(|v| v.as_u64()).unwrap() as u32,
                "A9 {name} last_line"
            );
        }
        if let Some(o_count) = oracle.get("sub_info_events").and_then(|v| v.as_u64()) {
            assert_eq!(
                model.sub_info_events, o_count,
                "A9 sub_info_events native vs oracle"
            );
        }
    }

    eprintln!(
        "=== {name} native vs aggregates.oracle.json OK ===\n{}",
        model.debug_summary()
    );
}

#[test]
fn default_calls1_native_matches_aggregates_oracle_json() {
    check_native_vs_aggregates_oracle_json("default-calls1");
}

#[test]
fn default_calls2_native_matches_aggregates_oracle_json() {
    check_native_vs_aggregates_oracle_json("default-calls2");
}

#[test]
fn calls2_default_binary_matches_oracle_jsonl() {
    // calls=2: SUB_ENTRY present; same mid×3 → leaf×5 workload (FIXTURE-EXPAND-2).
    check_fixture("calls2-default");
    let dir = fixture_dir("calls2-default");
    let model = ProfileModel::from_path(dir.join("nytprof.out")).expect("from_path");
    assert!(
        model.sub_entry_events > 0,
        "calls2-default must record SUB_ENTRY (calls=2)"
    );
    assert_eq!(model.time_block_events, 0, "calls2-default is non-blocks");
}

#[test]
fn calls2_default_workload_subs() {
    let path = fixture_dir("calls2-default").join("nytprof.out");
    assert!(path.is_file(), "missing {}", path.display());
    let model = ProfileModel::from_path(&path).expect("from_path");

    // Contracted returns for mid×3 → leaf×5 with calls=2 (oracle aggregates).
    let leaf = model.sub_returns("main::leaf").expect("main::leaf");
    assert_eq!(leaf.returns, 15, "main::leaf returns");
    let mid = model.sub_returns("main::mid").expect("main::mid");
    assert_eq!(mid.returns, 3, "main::mid returns");

    let mid_leaf = model
        .call_edge("main::mid", "main::leaf")
        .expect("mid→leaf edge");
    assert_eq!(mid_leaf.count, 15);
    let rt_mid = model
        .call_edge("main::RUNTIME", "main::mid")
        .expect("RUNTIME→mid edge");
    assert_eq!(rt_mid.count, 3);

    assert!(model.sub_entry_events > 0);
    assert_eq!(model.time_block_events, 0);
}

#[test]
fn calls2_default_native_matches_aggregates_oracle_json() {
    check_native_vs_aggregates_oracle_json("calls2-default");
}

#[test]
fn blocks_calls1_native_matches_aggregates_oracle_json() {
    check_native_vs_aggregates_oracle_json("blocks-calls1");
}

/// `ProfileModel::from_path` must surface decoder failures for truncated profiles.
#[test]
fn from_path_truncated_profile_errors() {
    let fixture = fixture_dir("default-calls1").join("nytprof.out");
    assert!(fixture.is_file(), "missing {}", fixture.display());
    let bytes = std::fs::read(&fixture).expect("read fixture");
    let half = bytes.len() / 2;
    assert!(half > 0);

    let tmp = std::env::temp_dir().join(format!(
        "nytprof-model-trunc-{}.out",
        std::process::id()
    ));
    std::fs::write(&tmp, &bytes[..half]).expect("write truncated temp");
    let err = ProfileModel::from_path(&tmp).expect_err("truncated from_path must Err");
    assert!(
        matches!(err, ModelError::Decode(_)),
        "expected ModelError::Decode, got {err:?}"
    );
    let _ = std::fs::remove_file(&tmp);
}

/// INCOMPLETE-STREAM: track PID_START/PID_END and completeness rules.
#[test]
fn stream_completeness_pid_and_timing_rules() {
    use serde_json::json;

    // Empty model: no timing → incomplete.
    let empty = ProfileModel::new();
    assert!(!empty.is_stream_complete());
    let reasons = empty.stream_incompleteness_reasons();
    assert!(
        reasons.iter().any(|r| r.contains("TIME_LINE")),
        "empty should flag missing timing: {reasons:?}"
    );

    // PID_START without PID_END, still no timing.
    let events = vec![Event::new(
        0,
        tags::PID_START,
        vec![json!(1), json!(0), json!(0.0)],
    )];
    let m = ProfileModel::from_events(&events).expect("accumulate");
    assert_eq!(m.pid_start_events, 1);
    assert_eq!(m.pid_end_events, 0);
    assert!(!m.is_stream_complete());
    let reasons = m.stream_incompleteness_reasons();
    assert!(
        reasons.iter().any(|r| r.contains("PID_END")),
        "missing PID_END: {reasons:?}"
    );
    assert!(
        reasons.iter().any(|r| r.contains("TIME_LINE")),
        "missing timing: {reasons:?}"
    );

    // Balanced PIDs + TIME_LINE → complete.
    let events = vec![
        Event::new(0, tags::PID_START, vec![json!(1), json!(0), json!(0.0)]),
        Event::new(
            1,
            tags::TIME_LINE,
            vec![json!(10), json!(1), json!(1)],
        ),
        Event::new(2, tags::PID_END, vec![json!(1), json!(1.0)]),
    ];
    let m = ProfileModel::from_events(&events).expect("accumulate complete");
    assert_eq!(m.pid_start_events, 1);
    assert_eq!(m.pid_end_events, 1);
    assert_eq!(m.time_line_events, 1);
    assert!(
        m.is_stream_complete(),
        "balanced pid + timing should be complete: {:?}",
        m.stream_incompleteness_reasons()
    );
}

/// INCOMPLETE-STREAM: short prefix of default-calls1 may decode but is incomplete.
#[test]
fn incomplete_prefix_default_calls1_model_incomplete() {
    let fixture = fixture_dir("default-calls1").join("nytprof.out");
    assert!(fixture.is_file(), "missing {}", fixture.display());
    let bytes = std::fs::read(&fixture).expect("read fixture");
    assert!(bytes.len() > 500);
    let tmp = std::env::temp_dir().join(format!(
        "nytprof-model-incomplete-{}-{}.out",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&tmp, &bytes[..500]).expect("write prefix");
    match ProfileModel::from_path(&tmp) {
        Ok(model) => {
            assert!(
                !model.is_stream_complete(),
                "500-byte prefix must be incomplete: TL={} TB={} ps={} pe={}",
                model.time_line_events,
                model.time_block_events,
                model.pid_start_events,
                model.pid_end_events
            );
        }
        Err(_) => {
            // Mid-record truncate is also acceptable for this length; not required to decode.
        }
    }
    let _ = std::fs::remove_file(&tmp);
}

/// Golden default-calls1 tracks matching PID lifecycle and is complete.
#[test]
fn default_calls1_pid_lifecycle_complete() {
    let model = ProfileModel::from_path(fixture_dir("default-calls1").join("nytprof.out"))
        .expect("from_path");
    assert!(model.pid_start_events > 0);
    assert!(model.pid_end_events >= model.pid_start_events);
    assert!(model.is_stream_complete());
}

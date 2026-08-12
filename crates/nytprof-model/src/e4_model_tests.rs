//! E4-v0 model-level semantic equality tests (PR-B10).
//!
//! Compare ProfileModel aggregates on same-workload v5 vs v6 dual-sink pairs.
//! No CLI E5 report path required — product `from_path` dual dispatch only.
//!
//! Policy: `docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md`
//! Fixtures: `fixtures/e4/dual-sink/*` (C dual-sink COL-014, test/dev-only OQ-4)
//!
//! Residuals: full oracle `fixtures/v5/*` dual pairs (TEST-003/TEST-008);
//! E4 product smoke in offline_gate (PR-B12b); wire freeze; CLI v6 default.

use super::*;
use std::path::PathBuf;

fn e4_dual_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/e4/dual-sink")
        .join(name)
}

fn load_pair(stem: &str) -> (ProfileModel, ProfileModel) {
    let v5_path = e4_dual_fixture(&format!("{stem}_v5.nytprof"));
    let v6_path = e4_dual_fixture(&format!("{stem}_v6.nytprof"));
    assert!(
        v5_path.is_file(),
        "missing E4 dual v5 fixture {} (regen: make -C collector test && cp collector/build/dual_* fixtures/e4/dual-sink/)",
        v5_path.display()
    );
    assert!(
        v6_path.is_file(),
        "missing E4 dual v6 fixture {}",
        v6_path.display()
    );
    let v5 = ProfileModel::from_path(&v5_path)
        .unwrap_or_else(|e| panic!("v5 load {}: {e}", v5_path.display()));
    let v6 = ProfileModel::from_path(&v6_path)
        .unwrap_or_else(|e| panic!("v6 load {}: {e}", v6_path.display()));
    (v5, v6)
}

fn assert_e4_pair(stem: &str) {
    let (v5, v6) = load_pair(stem);
    e4_v0_aggregates_equal(&v5, &v6, true).unwrap_or_else(|m| {
        panic!("E4-v0 model equality failed for dual-sink pair '{stem}': {m}")
    });
    // Both sides must be stream-complete for E4 required surfaces.
    assert!(
        v5.is_stream_complete(),
        "{stem} v5 incomplete: {:?}",
        v5.stream_incompleteness_reasons()
    );
    assert!(
        v6.is_stream_complete(),
        "{stem} v6 incomplete: {:?}",
        v6.stream_incompleteness_reasons()
    );
}

/// M4 mini dual-sink same-run pair (COL-014).
#[test]
fn e4_v0_dual_m4_model_equal() {
    assert_e4_pair("m4");
    let (v5, _) = load_pair("m4");
    assert_eq!(v5.time_line_events, 3);
    assert_eq!(v5.discount_events, 1);
    assert_eq!(v5.sub_return_events, 1);
    assert_eq!(v5.sub_total("main::leaf").map(|t| t.returns), Some(1));
}

/// Primary-fixture-shaped default-calls1 (scaled synthetic — not full oracle 818 discounts).
#[test]
fn e4_v0_dual_default_calls1_model_equal() {
    assert_e4_pair("default_calls1");
    let (v5, _) = load_pair("default_calls1");
    // Pattern matches E4 policy sample shapes (leaf 15 / mid 3 / edge 15).
    assert_eq!(v5.sub_total("main::leaf").map(|t| t.returns), Some(15));
    assert_eq!(v5.sub_total("main::mid").map(|t| t.returns), Some(3));
    assert_eq!(
        v5.call_edge("main::mid", "main::leaf").map(|e| e.count),
        Some(15)
    );
    assert_eq!(v5.sub_entry_events, 0);
    assert_eq!(v5.discount_events, 15); // scaled mini, not oracle 818
    assert_eq!(v5.line_total(1, 5).map(|t| t.calls), Some(15));
    assert_eq!(
        v5.sub_def("main::leaf")
            .map(|d| (d.fid, d.first_line, d.last_line)),
        Some((1, 3, 7))
    );
    assert_eq!(
        v5.sub_def("main::mid")
            .map(|d| (d.fid, d.first_line, d.last_line)),
        Some((1, 8, 12))
    );
    assert!(v5
        .source_line(1, 5)
        .unwrap_or("")
        .contains("x++"));
    assert_eq!(
        v5.attributes.get("ticks_per_sec").map(String::as_str),
        Some("10000000")
    );
    assert_eq!(v5.options.get("calls").map(String::as_str), Some("1"));
}

/// Primary-fixture-shaped blocks-calls1 (TIME_BLOCK path; scaled mini).
#[test]
fn e4_v0_dual_blocks_calls1_model_equal() {
    assert_e4_pair("blocks_calls1");
    let (v5, _) = load_pair("blocks_calls1");
    assert_eq!(v5.time_line_events, 0);
    assert_eq!(v5.time_block_events, 12);
    assert_eq!(v5.line_total(1, 5).map(|t| t.calls), Some(12)); // A4 from TIME_BLOCK
    assert_eq!(v5.block_line_total(1, 4).map(|t| t.calls), Some(12)); // A4b
    assert_eq!(v5.sub_total("main::leaf").map(|t| t.returns), Some(1));
}

/// Primary-fixture-shaped calls2-default (SUB_ENTRY multiplicity pattern).
#[test]
fn e4_v0_dual_calls2_default_model_equal() {
    assert_e4_pair("calls2_default");
    let (v5, _) = load_pair("calls2_default");
    assert_eq!(v5.sub_entry_events, 9);
    assert_eq!(v5.time_line_events, 9);
    assert_eq!(v5.sub_return_events, 9);
    assert_eq!(v5.options.get("calls").map(String::as_str), Some("2"));
}

/// All dual-sink pairs in one table-driven pass (smoke-friendly filter: e4_v0_).
#[test]
fn e4_v0_all_dual_sink_pairs_model_equal() {
    for stem in [
        "m4",
        "default_calls1",
        "blocks_calls1",
        "calls2_default",
    ] {
        assert_e4_pair(stem);
    }
}

/// Absolute vs packing C E3 fixtures of the same logical sample remain E4-equal
/// at model level (v6↔v6 packing policy interaction).
#[test]
fn e4_v0_v6_absolute_vs_packing_model_equal() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/v6/from-c");
    let abs = ProfileModel::from_path(base.join("absolute.nytprof")).expect("abs");
    let pack = ProfileModel::from_path(base.join("packing.nytprof")).expect("pack");
    let lz4 = ProfileModel::from_path(base.join("packing_lz4.nytprof")).expect("lz4");
    // auto-VERSION inject is shared on product path; totals should match.
    e4_v0_aggregates_equal(&abs, &pack, true).expect("abs vs packing");
    e4_v0_aggregates_equal(&abs, &lz4, true).expect("abs vs packing_lz4");
}

/// Stand-in: logical from_events vs v6 absolute encode (engineering pair path).
#[test]
fn e4_v0_standin_logical_vs_v6_model_equal() {
    use nytprof_format_v6::chunk::codec;
    use nytprof_format_v6::event_body::EventRecordSpec;
    use nytprof_format_v6::{attribute_kv, e3_standin_write_absolute, option_kv};
    use nytprof_types::{tags, Event};
    use serde_json::json;

    let logical = vec![
        Event::new(0, tags::VERSION, vec![json!(6), json!(0)]),
        Event::new(
            1,
            tags::ATTRIBUTE,
            vec![json!("ticks_per_sec"), json!("10000000")],
        ),
        Event::new(2, tags::OPTION, vec![json!("calls"), json!("1")]),
        Event::new(
            3,
            tags::NEW_FID,
            vec![
                json!(1u64),
                json!(0u64),
                json!(0u64),
                json!(0u64),
                json!(0u64),
                json!(0u64),
                json!("workload.pl"),
            ],
        ),
        Event::new(4, tags::TIME_LINE, vec![json!(5i64), json!(1u64), json!(5u64)]),
        Event::new(
            5,
            tags::SUB_RETURN,
            vec![json!(1u64), json!(100.0), json!(40.0), json!("main::leaf")],
        ),
        Event::new(
            6,
            tags::SUB_RETURN,
            vec![json!(1u64), json!(300.0), json!(50.0), json!("main::mid")],
        ),
        Event::new(
            7,
            tags::SUB_CALLERS,
            vec![
                json!(1u64),
                json!(9u64),
                json!(15u64),
                json!(200.0),
                json!(100.0),
                json!(0.0),
                json!(0u64),
                json!("main::leaf"),
                json!("main::mid"),
            ],
        ),
        Event::new(
            8,
            tags::SUB_INFO,
            vec![json!(1u64), json!(3u64), json!(7u64), json!("main::leaf")],
        ),
        Event::new(
            9,
            tags::SRC_LINE,
            vec![json!(1u64), json!(5u64), json!("    $x++ for 1 .. 50;\n")],
        ),
        Event::new(10, tags::PID_START, vec![json!(1u64), json!(0u64), json!(0u64)]),
        Event::new(11, tags::PID_END, vec![json!(1u64), json!(1u64)]),
    ];
    let from_logical = ProfileModel::from_events(&logical).expect("logical");
    let specs = [
        attribute_kv(b"ticks_per_sec", b"10000000"),
        option_kv(b"calls", b"1"),
        EventRecordSpec::NewFid {
            fid: 1,
            string_id: 0,
            string_flags: 0,
            filename: b"workload.pl",
        },
        EventRecordSpec::TimeLine {
            fid: 1,
            line: 5,
            ticks: 5,
        },
        EventRecordSpec::SubReturn {
            depth: 1,
            incl: 100,
            excl: 40,
            string_id: 0,
            string_flags: 0,
            subname: b"main::leaf",
        },
        EventRecordSpec::SubReturn {
            depth: 1,
            incl: 300,
            excl: 50,
            string_id: 0,
            string_flags: 0,
            subname: b"main::mid",
        },
        EventRecordSpec::SubCallers {
            fid: 1,
            line: 9,
            count: 15,
            incl: 200,
            excl: 100,
            reci: 0,
            rec_depth: 0,
            called_string_id: 0,
            called_string_flags: 0,
            called: b"main::leaf",
            caller_string_id: 0,
            caller_string_flags: 0,
            caller: b"main::mid",
        },
        EventRecordSpec::SubInfo {
            fid: 1,
            first_line: 3,
            last_line: 7,
            string_id: 0,
            string_flags: 0,
            name: b"main::leaf",
        },
        EventRecordSpec::SrcLine {
            fid: 1,
            line: 5,
            string_id: 0,
            string_flags: 0,
            text: b"    $x++ for 1 .. 50;\n",
        },
        EventRecordSpec::PidStart {
            pid: 1,
            ppid: 0,
            start_time: 0,
        },
        EventRecordSpec::PidEnd {
            pid: 1,
            end_time: 1,
        },
    ];
    let wire = e3_standin_write_absolute(&specs, codec::NONE).expect("encode");
    let from_v6 = ProfileModel::from_bytes(&wire).expect("v6 model");
    // VERSION present on both; total_events should match.
    e4_v0_aggregates_equal(&from_logical, &from_v6, true)
        .expect("logical vs v6 stand-in E4-v0");
}

/// e4_v0_aggregates_equal reports mismatches (regression: must not silently pass).
#[test]
fn e4_v0_aggregates_equal_detects_mismatch() {
    let (v5, mut v6) = load_pair("m4");
    v6.time_line_events = v6.time_line_events.saturating_add(99);
    let err = e4_v0_aggregates_equal(&v5, &v6, true).expect_err("must mismatch");
    assert!(
        err.contains("TIME_LINE"),
        "expected TIME_LINE in error, got {err}"
    );
}

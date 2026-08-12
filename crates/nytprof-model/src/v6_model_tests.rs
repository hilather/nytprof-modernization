//! Product v6 → ProfileModel ingest tests (PR-B11a).
//!
//! Lives separately so `model_tests.rs` stays focused on v5/A1–A9 fixtures.

use super::*;
use std::path::PathBuf;
use nytprof_types::{tags, Event};

// ---------------------------------------------------------------------------
// PR-B11a — product v6 → ProfileModel ingest + dump-aligned pair aggregates
// ---------------------------------------------------------------------------

fn v6_from_c_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/v6/from-c")
        .join(name)
}

/// Shared aggregate fields compared for v5-logical vs v6-encoded pairs.
fn assert_aggregate_parity(a: &ProfileModel, b: &ProfileModel, label: &str) {
    assert_eq!(a.time_line_events, b.time_line_events, "{label}: TIME_LINE");
    assert_eq!(a.time_block_events, b.time_block_events, "{label}: TIME_BLOCK");
    assert_eq!(a.discount_events, b.discount_events, "{label}: DISCOUNT");
    assert_eq!(a.sub_entry_events, b.sub_entry_events, "{label}: SUB_ENTRY");
    assert_eq!(a.sub_return_events, b.sub_return_events, "{label}: SUB_RETURN");
    assert_eq!(a.sub_callers_events, b.sub_callers_events, "{label}: SUB_CALLERS");
    assert_eq!(a.new_fid_events, b.new_fid_events, "{label}: NEW_FID");
    assert_eq!(a.src_line_events, b.src_line_events, "{label}: SRC_LINE");
    assert_eq!(a.sub_info_events, b.sub_info_events, "{label}: SUB_INFO");
    assert_eq!(a.line_totals, b.line_totals, "{label}: A4 line_totals");
    assert_eq!(
        a.block_line_totals, b.block_line_totals,
        "{label}: A4b block_line_totals"
    );
    assert_eq!(
        a.sub_return_totals.len(),
        b.sub_return_totals.len(),
        "{label}: sub_return_totals size"
    );
    for (name, t) in &a.sub_return_totals {
        let o = b
            .sub_return_totals
            .get(name)
            .unwrap_or_else(|| panic!("{label}: missing sub {name}"));
        assert_eq!(t.returns, o.returns, "{label}: {name} returns");
        assert!(f64_close(t.incl, o.incl), "{label}: {name} incl");
        assert!(f64_close(t.excl, o.excl), "{label}: {name} excl");
    }
    assert_eq!(a.call_edges.len(), b.call_edges.len(), "{label}: call_edges");
    for (key, e) in &a.call_edges {
        let o = b
            .call_edges
            .get(key)
            .unwrap_or_else(|| panic!("{label}: missing edge {:?}", key));
        assert_eq!(e.count, o.count, "{label}: edge {:?} count", key);
        assert_eq!(e.sites, o.sites, "{label}: edge {:?} sites", key);
    }
    assert_eq!(a.sub_defs, b.sub_defs, "{label}: A9 sub_defs");
    assert_eq!(a.source_lines, b.source_lines, "{label}: A8 source_lines");
    assert_eq!(a.files, b.files, "{label}: files");
    assert_eq!(a.attributes, b.attributes, "{label}: attributes");
    assert_eq!(a.options, b.options, "{label}: options");
}

/// C absolute E3 fixture loads via product from_path and matches expected counts.
#[test]
fn v6_c_absolute_from_path_aggregates() {
    let path = v6_from_c_fixture("absolute.nytprof");
    assert!(path.is_file(), "missing {}", path.display());
    let model = ProfileModel::from_path(&path).expect("v6 absolute from_path");
    // Sample: TL×2 + TB + SUB_ENTRY (+ auto VERSION not counted in A1–A9 counters beyond total)
    assert_eq!(model.time_line_events, 2);
    assert_eq!(model.time_block_events, 1);
    assert_eq!(model.sub_entry_events, 1);
    assert_eq!(model.line_total(1, 10).map(|t| t.calls), Some(1));
    assert_eq!(model.line_total(1, 11).map(|t| t.calls), Some(1));
    assert_eq!(model.line_total(1, 12).map(|t| t.calls), Some(1)); // A4 from TIME_BLOCK
    assert_eq!(model.block_line_total(1, 4).map(|t| t.calls), Some(1));
    assert_eq!(model.line_total(1, 10).map(|t| t.ticks), Some(5));
    assert!(model.total_events >= 4);
}

/// Absolute vs packing C fixtures of the same logical sample → equal aggregates.
#[test]
fn v6_c_absolute_vs_packing_aggregate_parity() {
    let abs = ProfileModel::from_path(v6_from_c_fixture("absolute.nytprof")).expect("abs");
    let pack = ProfileModel::from_path(v6_from_c_fixture("packing.nytprof")).expect("pack");
    let lz4 = ProfileModel::from_path(v6_from_c_fixture("packing_lz4.nytprof")).expect("lz4");
    assert_aggregate_parity(&abs, &pack, "absolute vs packing");
    assert_aggregate_parity(&abs, &lz4, "absolute vs packing_lz4");
}

/// Dict FOOTER C fixture resolves ATTRIBUTE into the model attributes map.
#[test]
fn v6_c_dict_from_path_attribute() {
    let model = ProfileModel::from_path(v6_from_c_fixture("dict.nytprof")).expect("dict");
    assert_eq!(model.time_line_events, 1);
    assert_eq!(
        model.attributes.get("basetime").map(String::as_str),
        Some("1786111723")
    );
}

/// Fail closed on unknown magic.
#[test]
fn from_path_unknown_magic_errors() {
    let tmp = std::env::temp_dir().join(format!(
        "nytprof-model-badmagic-{}.out",
        std::process::id()
    ));
    std::fs::write(&tmp, b"NOTAPROFILE\n").expect("write");
    let err = ProfileModel::from_path(&tmp).expect_err("must Err");
    assert!(
        matches!(err, ModelError::UnsupportedProfile { .. }),
        "got {err:?}"
    );
    let _ = std::fs::remove_file(&tmp);
}

/// Pair parity: same logical stream via from_events (v5-style) vs v6 stand-in encode → from_path.
#[test]
fn v6_standin_pair_matches_from_events_aggregates() {
    use nytprof_format_v6::chunk::codec;
    use nytprof_format_v6::event_body::EventRecordSpec;
    use nytprof_format_v6::{e3_standin_write_absolute, e3_standin_write_packing};
    use serde_json::json;

    // Build a richer logical stream as dump Events (v5 argument shapes).
    let logical = vec![
        Event::new(0, tags::VERSION, vec![json!(6), json!(0)]),
        Event::new(
            1,
            tags::ATTRIBUTE,
            vec![json!("ticks_per_sec"), json!("10000000")],
        ),
        Event::new(
            2,
            tags::OPTION,
            vec![json!("calls"), json!("1")],
        ),
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
        Event::new(4, tags::TIME_LINE, vec![json!(5i64), json!(1u64), json!(10u64)]),
        Event::new(5, tags::TIME_LINE, vec![json!(6i64), json!(1u64), json!(11u64)]),
        Event::new(
            6,
            tags::TIME_BLOCK,
            vec![
                json!(20i64),
                json!(1u64),
                json!(12u64),
                json!(4u64),
                json!(0u64),
            ],
        ),
        Event::new(7, tags::SUB_ENTRY, vec![json!(1u64), json!(10u64)]),
        Event::new(
            8,
            tags::SUB_RETURN,
            vec![json!(1u64), json!(100.0), json!(40.0), json!("main::leaf")],
        ),
        Event::new(
            9,
            tags::SUB_RETURN,
            vec![json!(1u64), json!(300.0), json!(50.0), json!("main::mid")],
        ),
        Event::new(
            10,
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
            11,
            tags::SUB_INFO,
            vec![json!(1u64), json!(3u64), json!(7u64), json!("main::leaf")],
        ),
        Event::new(
            12,
            tags::SRC_LINE,
            vec![json!(1u64), json!(5u64), json!("    $x++ for 1 .. 50;\n")],
        ),
        Event::new(13, tags::DISCOUNT, vec![]),
        Event::new(14, tags::PID_START, vec![json!(1u64), json!(0u64), json!(0u64)]),
        Event::new(15, tags::PID_END, vec![json!(1u64), json!(1u64)]),
    ];
    let from_logical = ProfileModel::from_events(&logical).expect("from_events");

    // Encode isomorphic v6 EVENT body (stand-in — engineering pair path, not C E3 evidence).
    use nytprof_format_v6::{attribute_kv, option_kv};
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
            line: 10,
            ticks: 5,
        },
        EventRecordSpec::TimeLine {
            fid: 1,
            line: 11,
            ticks: 6,
        },
        EventRecordSpec::TimeBlock {
            fid: 1,
            line: 12,
            block_line: 4,
            ticks: 20,
        },
        EventRecordSpec::SubEntry {
            caller_fid: 1,
            caller_line: 10,
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
        EventRecordSpec::Discount,
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

    let abs_bytes = e3_standin_write_absolute(&specs, codec::NONE).expect("abs encode");
    let pack_bytes =
        e3_standin_write_packing(&specs, codec::ZLIB, 4).expect("pack encode");

    let from_v6_abs = ProfileModel::from_bytes(&abs_bytes).expect("v6 abs model");
    let from_v6_pack = ProfileModel::from_bytes(&pack_bytes).expect("v6 pack model");

    assert_aggregate_parity(&from_logical, &from_v6_abs, "logical vs v6 absolute");
    assert_aggregate_parity(&from_logical, &from_v6_pack, "logical vs v6 packing");
    assert_aggregate_parity(&from_v6_abs, &from_v6_pack, "v6 abs vs packing");

    // Workload samples
    assert_eq!(from_v6_abs.sub_total("main::leaf").map(|t| t.returns), Some(1));
    assert_eq!(
        from_v6_abs.call_edge("main::mid", "main::leaf").map(|e| e.count),
        Some(15)
    );
    assert_eq!(
        from_v6_abs.sub_def("main::leaf").map(|d| (d.fid, d.first_line, d.last_line)),
        Some((1, 3, 7))
    );
    assert!(from_v6_abs
        .source_line(1, 5)
        .unwrap_or("")
        .contains("x++"));
    assert!(from_v6_abs.is_stream_complete());
}

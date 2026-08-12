//! Dual-equality **E3 harness** — writer bytes → Rust always-inflate decode.
//!
//! Contract: [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`]
//!
//! **E3** is: external writer-produced v6 profile bytes (COL-007 C encoder, or any
//! independent encoder) must always-inflate decode via shipped consumers to a known
//! logical event stream (absolute sites + sequences + optional FOOTER string-dict resolve).
//!
//! This module provides the **check harness**. Product E3 evidence is **`e3_c_*` tests**
//! that load **C-produced only** fixtures under `fixtures/v6/from-c/**` (never Rust stand-in
//! encode). Stand-in writers (`e3_standin_*`) remain for harness unit coverage only.
//!
//! # Residual honesty (non-claims)
//!
//! - Stand-in writer tests are **NOT product dual-equality evidence**.
//! - E3-EVENT with C is **ready** for the absolute/packing/dict/mid-stream matrix;
//!   **E3-mixed** (SOURCE/INDEX/SUMMARY multi-kind product C fixtures) remains residual.
//! - Not wire freeze; not CLI v6 default; not E4 enforcement; COL-008 still deferred.

use crate::compressed_profile::OwnedEventRecord;
use crate::decoded_event::{
    decode_decoded_event_profile, decode_decoded_event_profile_auto_version,
    decode_decoded_event_profile_auto_version_with_string_dict,
    decode_decoded_event_profile_with_string_dict,
    encode_decoded_event_mid_stream_codec_switch_with_site_deltas_and_seq,
    encode_decoded_event_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq,
    encode_decoded_event_profile, encode_decoded_event_profile_with_site_deltas_and_seq,
    encode_decoded_event_profile_with_string_dict,
    encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq, DecodedEventError,
    DecodedEventProfile,
};
use crate::event_body::EventRecordSpec;
use crate::string_dict::StringDictionary;
use crate::{MAGIC, SUPPORTED_MAJOR};

/// Profile wire family detected from file-prefix magic / text header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileWireKind {
    /// Fixed-header `NYTPROF6` (provisional format v6).
    V6,
    /// Text header `NYTProf <major> <minor>` (format v5; major checked by v5 decoder).
    V5,
    /// Neither recognized product magic/header.
    Unknown,
}

/// Max bytes scanned for a v5 text header newline when classifying non-v6 input.
///
/// Caps full-file scans on huge non-profile blobs (Issue: header probe O(n)).
pub const V5_HEADER_PROBE_MAX: usize = 4096;

/// Detect product wire family from the leading bytes of a profile file.
///
/// - First 8 bytes equal [`MAGIC`] (`NYTPROF6`) → [`ProfileWireKind::V6`]
/// - Leading text line starts with `NYTProf ` (v5 header form) within
///   [`V5_HEADER_PROBE_MAX`] → [`ProfileWireKind::V5`]
/// - Otherwise [`ProfileWireKind::Unknown`] (callers fail closed)
///
/// Does **not** validate full headers or major version.
pub fn detect_profile_wire_kind(buf: &[u8]) -> ProfileWireKind {
    if buf.len() >= MAGIC.len() && &buf[..MAGIC.len()] == MAGIC.as_slice() {
        return ProfileWireKind::V6;
    }
    // v5: first line `NYTProf <major> <minor>\n` (mixed case + space).
    // Probe only a bounded prefix so huge garbage blobs stay O(1).
    let probe = &buf[..buf.len().min(V5_HEADER_PROBE_MAX)];
    if let Some(nl) = probe.iter().position(|&b| b == b'\n') {
        let line = &probe[..nl];
        if line.starts_with(b"NYTProf ") {
            return ProfileWireKind::V5;
        }
    } else if probe.starts_with(b"NYTProf ") {
        // Incomplete single-line header still classifies as v5 for fail-closed decode.
        return ProfileWireKind::V5;
    }
    ProfileWireKind::Unknown
}

/// Product always-inflate decode of a v6 EVENT profile for model/CLI ingest.
///
/// - Always-inflate EVENT chunks (optional CRC verify)
/// - When FOOTER is present, **require** a well-formed string dictionary and resolve
///   string_ids (ADR-0002) — fail closed on missing ids / corrupt table (no empty-string soft fallback)
/// - Auto-align body VERSION with fixed-header (inject when omitted)
///
/// Not a wire freeze; not full multi-kind SOURCE/INDEX/SUMMARY product path
/// (E3-mixed residual). Default `parse_chunk_frame` remains non-inflating.
pub fn product_decode_v6_event_profile(
    wire: &[u8],
    verify_crc: bool,
) -> Result<E3Decoded, DecodedEventError> {
    // Plain always-inflate first (fail-closed on corrupt EVENT body / CRC).
    let (plain, n) = decode_decoded_event_profile_auto_version(wire, verify_crc)?;
    if plain.has_footer {
        // COL-007 product FOOTER is the string-dict table. Resolve fail-closed:
        // never soft-fallback to unresolved inline empties (blank keys/names).
        let (profile, dict, n2) =
            decode_decoded_event_profile_auto_version_with_string_dict(wire, verify_crc)?;
        return Ok(E3Decoded {
            profile,
            dict: Some(dict),
            bytes_consumed: n2,
        });
    }
    Ok(E3Decoded {
        profile: plain,
        dict: None,
        bytes_consumed: n,
    })
}

/// Result of an E3 always-inflate decode of writer-produced bytes.
#[derive(Debug)]
pub struct E3Decoded {
    pub profile: DecodedEventProfile,
    pub dict: Option<StringDictionary>,
    pub bytes_consumed: usize,
}

/// Always-inflate decode writer-produced EVENT profile bytes (optional FOOTER dict).
///
/// When `expect_string_dict` is true, uses
/// [`decode_decoded_event_profile_with_string_dict`]; otherwise plain
/// [`decode_decoded_event_profile`]. Both drive the **shipped** always-inflate path.
///
/// Product E3: feed COL-007 C bytes here. Stand-in tests prove the harness API only.
pub fn e3_decode_writer_bytes(
    wire: &[u8],
    verify_crc: bool,
    expect_string_dict: bool,
) -> Result<E3Decoded, DecodedEventError> {
    if expect_string_dict {
        let (profile, dict, n) = decode_decoded_event_profile_with_string_dict(wire, verify_crc)?;
        Ok(E3Decoded {
            profile,
            dict: Some(dict),
            bytes_consumed: n,
        })
    } else {
        let (profile, n) = decode_decoded_event_profile(wire, verify_crc)?;
        Ok(E3Decoded {
            profile,
            dict: None,
            bytes_consumed: n,
        })
    }
}

/// Fail-closed E3 equality: decoded records and sequences must match expected.
pub fn e3_assert_logical_equal(
    got: &DecodedEventProfile,
    expected_records: &[OwnedEventRecord],
    expected_sequences: &[Option<u64>],
) -> Result<(), String> {
    if got.records != expected_records {
        return Err(format!(
            "E3 record mismatch: got {} records, expected {}",
            got.records.len(),
            expected_records.len()
        ));
    }
    if got.sequences != expected_sequences {
        return Err(format!(
            "E3 sequence mismatch: got {:?}, expected {:?}",
            got.sequences, expected_sequences
        ));
    }
    Ok(())
}

/// Stand-in writer: absolute EVENT encode via shipped helper (simulates external encoder).
///
/// **Not product dual-equality evidence** — Rust encode stand-in only.
pub fn e3_standin_write_absolute(
    events: &[EventRecordSpec<'_>],
    event_codec: u8,
) -> Result<Vec<u8>, DecodedEventError> {
    encode_decoded_event_profile(
        SUPPORTED_MAJOR,
        0,
        0,
        0,
        0,
        &[],
        event_codec,
        events,
        0,
        None,
    )
}

/// Stand-in writer: packing EVENT encode via shipped helper (ADR-0001 candidate forms).
///
/// **Not product dual-equality evidence** — Rust encode stand-in only.
pub fn e3_standin_write_packing(
    events: &[EventRecordSpec<'_>],
    event_codec: u8,
    max_events_per_chunk: usize,
) -> Result<Vec<u8>, DecodedEventError> {
    encode_decoded_event_profile_with_site_deltas_and_seq(
        SUPPORTED_MAJOR,
        0,
        0,
        0,
        0,
        &[],
        event_codec,
        events,
        max_events_per_chunk,
        None,
    )
}

/// Stand-in writer: absolute EVENT + FOOTER string-dictionary (ADR-0002 candidate form).
///
/// Decode with [`e3_decode_writer_bytes`] and `expect_string_dict = true`.
/// **Not product dual-equality evidence** — Rust encode stand-in only.
pub fn e3_standin_write_string_dict(
    events: &[EventRecordSpec<'_>],
    event_codec: u8,
    dict_entries: &[(u64, u8, &[u8])],
) -> Result<Vec<u8>, DecodedEventError> {
    encode_decoded_event_profile_with_string_dict(
        SUPPORTED_MAJOR,
        0,
        0,
        0,
        0,
        &[],
        event_codec,
        events,
        0,
        dict_entries,
    )
}

/// Stand-in writer: packing EVENT + FOOTER string-dictionary (compose path).
///
/// Decode with [`e3_decode_writer_bytes`] and `expect_string_dict = true`.
/// **Not product dual-equality evidence** — Rust encode stand-in only.
pub fn e3_standin_write_string_dict_packing(
    events: &[EventRecordSpec<'_>],
    event_codec: u8,
    max_events_per_chunk: usize,
    dict_entries: &[(u64, u8, &[u8])],
) -> Result<Vec<u8>, DecodedEventError> {
    encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq(
        SUPPORTED_MAJOR,
        0,
        0,
        0,
        0,
        &[],
        event_codec,
        events,
        max_events_per_chunk,
        dict_entries,
    )
}

/// Stand-in writer: mid-stream START_DEFLATE packing continuity (ADR-0001 continuity intent).
///
/// Pre and post EVENT plains share packing state (site bases + seq continue across switch).
/// Decode with [`e3_decode_writer_bytes`] and `expect_string_dict = false`.
/// **Not product dual-equality evidence** — Rust encode stand-in only.
pub fn e3_standin_write_mid_stream_packing(
    pre_events: &[EventRecordSpec<'_>],
    pre_codec: u8,
    post_events: &[EventRecordSpec<'_>],
    post_codec: u8,
) -> Result<Vec<u8>, DecodedEventError> {
    encode_decoded_event_mid_stream_codec_switch_with_site_deltas_and_seq(
        SUPPORTED_MAJOR,
        0,
        0,
        0,
        0,
        &[],
        pre_codec,
        pre_events,
        post_codec,
        post_events,
        None,
    )
}

/// Stand-in writer: mid-stream packing continuity + FOOTER string-dictionary.
///
/// Decode with [`e3_decode_writer_bytes`] and `expect_string_dict = true`.
/// **Not product dual-equality evidence** — Rust encode stand-in only.
pub fn e3_standin_write_mid_stream_string_dict_packing(
    pre_events: &[EventRecordSpec<'_>],
    pre_codec: u8,
    post_events: &[EventRecordSpec<'_>],
    post_codec: u8,
    dict_entries: &[(u64, u8, &[u8])],
) -> Result<Vec<u8>, DecodedEventError> {
    encode_decoded_event_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq(
        SUPPORTED_MAJOR,
        0,
        0,
        0,
        0,
        &[],
        pre_codec,
        pre_events,
        post_codec,
        post_events,
        dict_entries,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::codec;
    use crate::string::FLAG_UTF8;

    #[test]
    fn detect_wire_kind_v6_and_v5() {
        assert_eq!(
            detect_profile_wire_kind(b"NYTPROF6\x06\0\0\0"),
            ProfileWireKind::V6
        );
        assert_eq!(
            detect_profile_wire_kind(b"NYTProf 5 0\n#Perl"),
            ProfileWireKind::V5
        );
        assert_eq!(
            detect_profile_wire_kind(b"garbage"),
            ProfileWireKind::Unknown
        );
        assert_eq!(detect_profile_wire_kind(b""), ProfileWireKind::Unknown);
    }

    #[test]
    fn product_decode_absolute_standin_matches_e3() {
        let specs = sample_specs();
        let wire = e3_standin_write_absolute(&specs, codec::NONE).expect("encode");
        let e3 = product_decode_v6_event_profile(&wire, true).expect("product decode");
        assert_eq!(e3.bytes_consumed, wire.len());
        // Auto-VERSION inject prepends one Version record.
        assert!(e3.profile.records.len() >= specs.len());
        assert!(matches!(
            e3.profile.records.first(),
            Some(OwnedEventRecord::Version { .. })
        ));
        let timelines: Vec<_> = e3
            .profile
            .records
            .iter()
            .filter(|r| matches!(r, OwnedEventRecord::TimeLine { .. }))
            .collect();
        assert_eq!(timelines.len(), 2);
    }

    /// FOOTER present but missing string_id → product path must Err (no empty-key soft OK).
    #[test]
    fn product_decode_footer_missing_string_id_fail_closed() {
        let events = [EventRecordSpec::Attribute {
            key_string_id: 99,
            key_string_flags: 0,
            key: b"",
            value_string_id: 0,
            value_string_flags: 0,
            value: b"1786111723",
        }];
        // Dict table has id 1 only — not 99.
        let dict_entries: &[(u64, u8, &[u8])] = &[(1, FLAG_UTF8, b"other")];
        let wire = e3_standin_write_string_dict(&events, codec::NONE, dict_entries)
            .expect("encode with FOOTER");
        let err = product_decode_v6_event_profile(&wire, true)
            .expect_err("missing dict id must fail closed");
        // Must not soft-succeed with empty Attribute key.
        let _ = err;
        // Sanity: e3 expect_string_dict=true also fails.
        assert!(
            e3_decode_writer_bytes(&wire, true, true).is_err(),
            "E3 expect_string_dict path must also fail"
        );
    }

    #[test]
    fn detect_wire_kind_header_probe_capped() {
        // Huge blob without early newline and without NYTPROF6 → Unknown without
        // needing full-buffer semantics beyond the probe window.
        let mut big = vec![0u8; V5_HEADER_PROBE_MAX + 10_000];
        big[0] = b'X';
        assert_eq!(detect_profile_wire_kind(&big), ProfileWireKind::Unknown);
        // v5 header only after probe window must not be classified as V5.
        let mut late = vec![b'Z'; V5_HEADER_PROBE_MAX + 32];
        let hdr = b"NYTProf 5 0\n";
        late[V5_HEADER_PROBE_MAX..V5_HEADER_PROBE_MAX + hdr.len()].copy_from_slice(hdr);
        assert_eq!(
            detect_profile_wire_kind(&late),
            ProfileWireKind::Unknown,
            "header past probe max must not be V5"
        );
    }

    fn sample_specs() -> [EventRecordSpec<'static>; 4] {
        [
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
        ]
    }

    fn sample_dict_events() -> [EventRecordSpec<'static>; 4] {
        [
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"",
            },
            EventRecordSpec::Attribute {
                key_string_id: 2,
                key_string_flags: 0,
                key: b"",
                value_string_id: 0,
                value_string_flags: 0,
                value: b"1786111723",
            },
            EventRecordSpec::Comment {
                string_id: 3,
                string_flags: 0,
                text: b"",
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"inline-mark",
            },
        ]
    }

    fn sample_dict_entries() -> &'static [(u64, u8, &'static [u8])] {
        &[
            (1, FLAG_UTF8, b"e3-dict-label"),
            (2, 0, b"basetime"),
            (3, 0, b"# e3 dict comment"),
        ]
    }

    /// Mid-stream packing pre: TIME_LINE + TIME_LINE_RUN + START_DEFLATE.
    fn mid_stream_packing_pre() -> [EventRecordSpec<'static>; 3] {
        const TL: &[u64] = &[7, 8];
        [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 50,
                ticks: TL,
            },
            EventRecordSpec::StartDeflate,
        ]
    }

    /// Mid-stream packing post: site-delta after run must land on (2,51).
    fn mid_stream_packing_post() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 51,
                ticks: 9,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 2,
                caller_line: 50,
            },
            EventRecordSpec::TimeBlock {
                fid: 3,
                line: 9,
                block_line: 7,
                ticks: 3,
            },
        ]
    }

    fn mid_stream_dict_packing_post() -> [EventRecordSpec<'static>; 4] {
        [
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 51,
                ticks: 9,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 2,
                caller_line: 50,
            },
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"",
            },
            EventRecordSpec::Comment {
                string_id: 2,
                string_flags: 0,
                text: b"",
            },
        ]
    }

    #[test]
    fn e3_harness_absolute_standin_writer_roundtrip() {
        let specs = sample_specs();
        // Stand-in writer = shipped absolute encode (future: replace with COL-007 C bytes).
        let wire = e3_standin_write_absolute(&specs, codec::NONE).expect("stand-in write");
        let e3 = e3_decode_writer_bytes(&wire, true, false).expect("E3 decode");
        assert_eq!(e3.bytes_consumed, wire.len());
        assert!(e3.dict.is_none());
        assert_eq!(e3.profile.records.len(), 4);
        match &e3.profile.records[0] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("{other:?}"),
        }
        // Baseline expected from a second independent decode of the same wire.
        let (expected, n2) = decode_decoded_event_profile(&wire, true).unwrap();
        assert_eq!(n2, wire.len());
        e3_assert_logical_equal(&e3.profile, &expected.records, &expected.sequences)
            .expect("E3 equality");
    }

    #[test]
    fn e3_harness_packing_standin_writer_multi_chunk_roundtrip() {
        let specs = sample_specs();
        // Packing multi-chunk stand-in writer (ADR-0001 candidate).
        let wire = e3_standin_write_packing(&specs, codec::ZLIB, 2).expect("packing write");
        let e3 = e3_decode_writer_bytes(&wire, true, false).expect("E3 decode");
        assert_eq!(e3.bytes_consumed, wire.len());
        assert!(e3.profile.event_chunk_count >= 2);
        // Single-chunk packing of same specs must decode to same logical stream.
        let single = e3_standin_write_packing(&specs, codec::NONE, 0).unwrap();
        let (single_prof, _) = decode_decoded_event_profile(&single, true).unwrap();
        e3_assert_logical_equal(
            &e3.profile,
            &single_prof.records,
            &single_prof.sequences,
        )
        .expect("multi-chunk packing E3 equals single-chunk packing");
        match &e3.profile.records[0] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("{other:?}"),
        }
        // Sequences present under packing path.
        assert_eq!(e3.profile.sequences.len(), e3.profile.records.len());
        for (i, s) in e3.profile.sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
    }

    /// Stand-in FOOTER string-dict via `expect_string_dict=true`.
    /// **Not product dual-equality evidence** (Rust stand-in, not COL-007 C bytes).
    #[test]
    fn e3_harness_string_dict_standin_writer_expect_string_dict_true() {
        let events = sample_dict_events();
        let dict_entries = sample_dict_entries();
        let wire = e3_standin_write_string_dict(&events, codec::NONE, dict_entries)
            .expect("dict stand-in write");
        let e3 = e3_decode_writer_bytes(&wire, true, /* expect_string_dict */ true)
            .expect("E3 decode with string dict");
        assert_eq!(e3.bytes_consumed, wire.len());
        let dict = e3.dict.as_ref().expect("dict present when expect_string_dict");
        assert_eq!(dict.len(), 3);
        assert_eq!(dict.get(1).unwrap().data, b"e3-dict-label");
        assert_eq!(dict.get(2).unwrap().data, b"basetime");
        assert_eq!(dict.get(3).unwrap().data, b"# e3 dict comment");
        assert_eq!(e3.profile.records.len(), 4);
        match &e3.profile.records[0] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"e3-dict-label"),
            other => panic!("{other:?}"),
        }
        match &e3.profile.records[1] {
            OwnedEventRecord::Attribute { key, value } => {
                assert_eq!(key, b"basetime");
                assert_eq!(value, b"1786111723");
            }
            other => panic!("{other:?}"),
        }
        match &e3.profile.records[2] {
            OwnedEventRecord::Comment { text } => assert_eq!(text, b"# e3 dict comment"),
            other => panic!("{other:?}"),
        }
        match &e3.profile.records[3] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"inline-mark"),
            other => panic!("{other:?}"),
        }
        // Independent baseline via shipped with_string_dict path.
        let (expected, expected_dict, n2) =
            decode_decoded_event_profile_with_string_dict(&wire, true).unwrap();
        assert_eq!(n2, wire.len());
        assert_eq!(dict.len(), expected_dict.len());
        e3_assert_logical_equal(&e3.profile, &expected.records, &expected.sequences)
            .expect("E3 string-dict equality");
    }

    /// Stand-in packing + FOOTER dict (multi-chunk) via `expect_string_dict=true`.
    /// **Not product dual-equality evidence**.
    #[test]
    fn e3_harness_string_dict_packing_standin_multi_chunk() {
        // Compose packing + dict: mark(id=1) + two TIME_LINE (site-delta) + comment(id=3).
        let events = [
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"",
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
            EventRecordSpec::Comment {
                string_id: 3,
                string_flags: 0,
                text: b"",
            },
        ];
        let dict_entries: &[(u64, u8, &[u8])] = &[
            (1, FLAG_UTF8, b"pack-dict-mark"),
            (3, 0, b"# pack-dict-end"),
        ];
        let wire =
            e3_standin_write_string_dict_packing(&events, codec::ZLIB, 2, dict_entries)
                .expect("dict packing write");
        let e3 = e3_decode_writer_bytes(&wire, true, true).expect("E3 dict packing decode");
        assert_eq!(e3.bytes_consumed, wire.len());
        assert!(e3.profile.event_chunk_count >= 2);
        let dict = e3.dict.as_ref().unwrap();
        assert_eq!(dict.get(1).unwrap().data, b"pack-dict-mark");
        // Single-chunk packing+dict of same specs → same logical stream.
        let single =
            e3_standin_write_string_dict_packing(&events, codec::NONE, 0, dict_entries).unwrap();
        let (single_prof, _, _) =
            decode_decoded_event_profile_with_string_dict(&single, true).unwrap();
        e3_assert_logical_equal(
            &e3.profile,
            &single_prof.records,
            &single_prof.sequences,
        )
        .expect("multi-chunk dict packing E3 equals single-chunk");
        match &e3.profile.records[0] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"pack-dict-mark"),
            other => panic!("{other:?}"),
        }
        assert_eq!(e3.profile.sequences.len(), e3.profile.records.len());
        for (i, s) in e3.profile.sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
    }

    /// Stand-in mid-stream packing continuity (site bases + seq across START_DEFLATE switch).
    /// **Not product dual-equality evidence**.
    #[test]
    fn e3_harness_mid_stream_packing_standin_continuity() {
        let pre = mid_stream_packing_pre();
        let post = mid_stream_packing_post();
        let wire = e3_standin_write_mid_stream_packing(
            &pre,
            codec::NONE,
            &post,
            codec::ZLIB,
        )
        .expect("mid-stream packing write");
        let e3 = e3_decode_writer_bytes(&wire, true, false).expect("E3 mid-stream decode");
        assert_eq!(e3.bytes_consumed, wire.len());
        assert_eq!(e3.profile.event_chunk_count, 2);
        assert_eq!(
            e3.profile.event_chunk_codecs,
            vec![codec::NONE, codec::ZLIB]
        );
        // Continuous packing of pre||post as logical baseline (single absolute decode of
        // single-chunk packing wire of the joined stream is not mid-stream; compare via
        // continuous packing body expansion through shipped encode path).
        let mut all: Vec<EventRecordSpec<'static>> = pre.to_vec();
        all.extend_from_slice(&post);
        let single_plain =
            crate::event_body::encode_event_body_with_site_deltas_and_seq(&all).unwrap();
        let (single_body, _) =
            crate::event_body::decode_event_body_full(&single_plain).unwrap();
        let owned: Vec<_> = single_body
            .records
            .iter()
            .map(OwnedEventRecord::from_borrowed)
            .collect();
        e3_assert_logical_equal(&e3.profile, &owned, &single_body.sequences)
            .expect("mid-stream packing E3 equals continuous packing");
        // Post-run site-delta: TL + 2 expanded run + StartDeflate = index 4 → (2,51,9).
        match &e3.profile.records[4] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("[4] post-run site-delta expected, got {other:?}"),
        }
        // Seq continuous across switch.
        assert_eq!(e3.profile.sequences.len(), e3.profile.records.len());
        for (i, s) in e3.profile.sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
    }

    /// Mid-stream packing + FOOTER dict through `expect_string_dict=true`.
    /// **Not product dual-equality evidence**.
    #[test]
    fn e3_harness_mid_stream_string_dict_packing_standin() {
        let pre = mid_stream_packing_pre();
        let post = mid_stream_dict_packing_post();
        let dict_entries: &[(u64, u8, &[u8])] = &[
            (1, FLAG_UTF8, b"ms-e3-dict-mark"),
            (2, 0, b"# ms-e3-dict-end"),
        ];
        let wire = e3_standin_write_mid_stream_string_dict_packing(
            &pre,
            codec::NONE,
            &post,
            codec::ZSTD,
            dict_entries,
        )
        .expect("mid-stream dict packing write");
        let e3 = e3_decode_writer_bytes(&wire, true, true).expect("E3 mid-stream dict decode");
        assert_eq!(e3.bytes_consumed, wire.len());
        assert_eq!(e3.profile.event_chunk_count, 2);
        assert_eq!(
            e3.profile.event_chunk_codecs,
            vec![codec::NONE, codec::ZSTD]
        );
        let dict = e3.dict.as_ref().unwrap();
        assert_eq!(dict.get(1).unwrap().data, b"ms-e3-dict-mark");
        assert_eq!(dict.get(2).unwrap().data, b"# ms-e3-dict-end");
        // Post-run site-delta continuity still holds.
        match &e3.profile.records[4] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("[4] {other:?}"),
        }
        let n = e3.profile.records.len();
        match &e3.profile.records[n - 2] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"ms-e3-dict-mark"),
            other => panic!("mark {other:?}"),
        }
        match &e3.profile.records[n - 1] {
            OwnedEventRecord::Comment { text } => assert_eq!(text, b"# ms-e3-dict-end"),
            other => panic!("comment {other:?}"),
        }
        // Cross-check: independent with_string_dict decode of same wire.
        let (expected, _, _) =
            decode_decoded_event_profile_with_string_dict(&wire, true).unwrap();
        e3_assert_logical_equal(&e3.profile, &expected.records, &expected.sequences)
            .expect("E3 mid-stream dict packing equality");
    }

    #[test]
    fn e3_harness_rejects_truncated_writer_bytes() {
        let specs = sample_specs();
        let wire = e3_standin_write_absolute(&specs, codec::NONE).unwrap();
        let truncated = &wire[..wire.len().saturating_sub(8).max(8)];
        match e3_decode_writer_bytes(truncated, true, false) {
            Err(_) => {}
            Ok(_) => panic!("truncated writer bytes must fail closed"),
        }
    }

    /// `expect_string_dict=true` on a profile without FOOTER dict must fail closed.
    #[test]
    fn e3_harness_expect_string_dict_without_footer_fail_closed() {
        let specs = sample_specs();
        let wire = e3_standin_write_absolute(&specs, codec::NONE).unwrap();
        match e3_decode_writer_bytes(&wire, true, /* expect_string_dict */ true) {
            Err(_) => {}
            Ok(_) => panic!("expect_string_dict without FOOTER dict must fail closed"),
        }
    }
}

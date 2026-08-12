//! Golden wire vectors — ADR-0006 / FMT-012 class (PR-B11).
//!
//! Loads immutable bytes from `fixtures/v6/vectors/` and asserts decode +
//! frozen ID alignment. Does not re-encode product C fixtures.

use nytprof_format_v6::chunk::{
    codec, kind, parse_chunk_frame, CHUNK_HEADER_LEN, CHUNK_SYNC, FLAG_KIND_REQUIRED,
};
use nytprof_format_v6::event_body::{
    decode_event_body, decode_event_body_full, is_known_opcode, opcode, EventRecord, FLAG_HAS_SEQ,
    FLAG_SITE_DELTA, MAX_TIME_BLOCK_RUN_LEN, MAX_TIME_LINE_RUN_LEN,
};
use nytprof_format_v6::mini_profile::decode_mini_profile;
use nytprof_format_v6::string_dict::decode_string_dictionary;
use nytprof_format_v6::tlv::{decode_tlv_region, type_id};
use nytprof_format_v6::varint::{decode_i64, decode_u64};
use nytprof_format_v6::{parse_fixed_header, HEADER_LEN_FULL, MAGIC, SUPPORTED_MAJOR};
use std::path::{Path, PathBuf};

fn vectors_root() -> PathBuf {
    // tests run with CARGO_MANIFEST_DIR = crates/nytprof-format-v6
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates
    p.pop(); // repo root
    p.join("fixtures/v6/vectors")
}

fn read_vec(rel: &str) -> Vec<u8> {
    let p = vectors_root().join(rel);
    std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn assert_sha256sum_file() {
    let sums = vectors_root().join("SHA256SUMS");
    let text = std::fs::read_to_string(&sums).expect("SHA256SUMS");
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let hex = parts.next().expect("hash");
        let path = parts.next().expect("path").trim_start_matches('*');
        let path = path.trim_start_matches("./");
        let bytes = read_vec(path);
        let got = sha256_hex(&bytes);
        assert_eq!(got, hex, "sha256 mismatch for {path}");
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    // Minimal SHA-256 (no extra deps) via `sha2` is not in crate deps — use
    // a pure verification against precomputed expected from file read + re-hash
    // with std only is hard. Prefer comparing to encode recompute for primitives
    // and shell-independent check: re-read SHA256SUMS format verified by
    // comparing file length + content decode instead when sha2 absent.
    //
    // Use a tiny public-domain style SHA256 implementation inline for the gate.
    sha256::digest(bytes)
}

mod sha256 {
    //! Compact SHA-256 for golden checksum verification only.
    pub fn digest(message: &[u8]) -> String {
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let mut msg = message.to_vec();
        let bit_len = (msg.len() as u64) * 8;
        msg.push(0x80);
        while (msg.len() % 64) != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bit_len.to_be_bytes());
        for chunk in msg.chunks_exact(64) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let mut a = h[0];
            let mut b = h[1];
            let mut c = h[2];
            let mut d = h[3];
            let mut e = h[4];
            let mut f = h[5];
            let mut g = h[6];
            let mut hh = h[7];
            const K: [u32; 64] = [
                0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
                0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
                0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
                0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
                0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
                0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
                0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
                0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
                0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
                0xc67178f2,
            ];
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let t1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(t1);
                d = c;
                c = b;
                b = a;
                a = t1.wrapping_add(t2);
            }
            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
            h[5] = h[5].wrapping_add(f);
            h[6] = h[6].wrapping_add(g);
            h[7] = h[7].wrapping_add(hh);
        }
        h.iter().map(|x| format!("{x:08x}")).collect()
    }
}

#[test]
fn golden_vector_sha256sums_match() {
    assert!(vectors_root().is_dir(), "missing {}", vectors_root().display());
    assert_sha256sum_file();
}

#[test]
fn golden_vector_primitives_uleb_zigzag() {
    let cases: &[(&str, u64)] = &[
        ("primitives/uleb_0.bin", 0),
        ("primitives/uleb_1.bin", 1),
        ("primitives/uleb_127.bin", 127),
        ("primitives/uleb_128.bin", 128),
        ("primitives/uleb_300.bin", 300),
        ("primitives/uleb_max_u64.bin", u64::MAX),
    ];
    for (path, expect) in cases {
        let b = read_vec(path);
        let (v, n) = decode_u64(&b, 0).expect(path);
        assert_eq!(n, b.len(), "{path} trailing");
        assert_eq!(v, *expect, "{path}");
    }
    let z: &[(&str, i64)] = &[
        ("primitives/zigzag_0.bin", 0),
        ("primitives/zigzag_neg1.bin", -1),
        ("primitives/zigzag_1.bin", 1),
        ("primitives/zigzag_neg2.bin", -2),
    ];
    for (path, expect) in z {
        let b = read_vec(path);
        let (v, n) = decode_i64(&b, 0).expect(path);
        assert_eq!(n, b.len());
        assert_eq!(v, *expect, "{path}");
    }
}

#[test]
fn golden_vector_fixed_header_and_chunk() {
    let hdr = read_vec("primitives/fixed_header_full.bin");
    assert_eq!(hdr.len(), HEADER_LEN_FULL as usize);
    assert_eq!(&hdr[..8], MAGIC.as_slice());
    let h = parse_fixed_header(&hdr).expect("header");
    assert_eq!(h.major, SUPPORTED_MAJOR);
    assert_eq!(h.header_len, HEADER_LEN_FULL);

    let ch = read_vec("primitives/chunk_event_none_empty.bin");
    assert_eq!(ch.len(), CHUNK_HEADER_LEN);
    let frame = parse_chunk_frame(&ch).expect("chunk");
    assert_eq!(frame.kind, kind::EVENT);
    assert_eq!(frame.codec, codec::NONE);
    assert_eq!(frame.flags, FLAG_KIND_REQUIRED);
    assert!(frame.payload.is_empty());
    assert_eq!(CHUNK_SYNC, u32::from_le_bytes(*b"NYT6"));
}

#[test]
fn golden_vector_tlv_and_string_dict() {
    let tlv = read_vec("primitives/tlv_region_producer_tps.bin");
    let (items, n) = decode_tlv_region(&tlv, 0).expect("tlv");
    assert_eq!(n, tlv.len());
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].type_id, type_id::PRODUCER);
    assert_eq!(items[0].value, b"nytprof-format-v6");
    assert_eq!(items[1].type_id, type_id::TICKS_PER_SEC);

    let dict = read_vec("primitives/string_dict_one_entry.bin");
    let (table, n) = decode_string_dictionary(&dict).expect("dict");
    assert_eq!(n, dict.len());
    let entry = table.get(1).expect("id 1");
    assert_eq!(entry.data.as_slice(), b"hello");
}

#[test]
fn golden_vector_event_bodies() {
    let tl = read_vec("event/time_line_1_2_3.bin");
    let (recs, n) = decode_event_body(&tl).expect("tl");
    assert_eq!(n, tl.len());
    assert_eq!(recs.len(), 1);
    match &recs[0] {
        EventRecord::TimeLine { fid, line, ticks } => {
            assert_eq!((*fid, *line, *ticks), (1, 2, 3));
        }
        other => panic!("{other:?}"),
    }

    let d = read_vec("event/discount.bin");
    let (recs, n) = decode_event_body(&d).expect("discount");
    assert_eq!(n, d.len());
    assert!(matches!(recs[0], EventRecord::Discount));

    let run = read_vec("event/time_line_run_n2.bin");
    let (full, n) = decode_event_body_full(&run).expect("run");
    assert_eq!(n, run.len());
    assert_eq!(full.records.len(), 2);
    match &full.records[0] {
        EventRecord::TimeLine { fid, line, ticks } => assert_eq!((*fid, *line, *ticks), (1, 5, 10)),
        other => panic!("{other:?}"),
    }
    match &full.records[1] {
        EventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 20),
        other => panic!("{other:?}"),
    }

    let seq = read_vec("event/site_delta_seq_tl_tl_se.bin");
    let (full, n) = decode_event_body_full(&seq).expect("seq");
    assert_eq!(n, seq.len());
    assert_eq!(full.records.len(), 3);
    assert!(seq.iter().any(|&b| b == FLAG_SITE_DELTA || b == (FLAG_SITE_DELTA | FLAG_HAS_SEQ)));
    for (i, s) in full.sequences.iter().enumerate() {
        assert_eq!(*s, Some(i as u64), "seq[{i}]");
    }

    let dual = read_vec("event/dual_output_sequence.bin");
    let (recs, n) = decode_event_body(&dual).expect("dual");
    assert_eq!(n, dual.len());
    assert!(matches!(recs[0], EventRecord::Version { major: 6, minor: 0 }));
    assert!(matches!(recs[2], EventRecord::StartDeflate));
    assert!(matches!(recs.last(), Some(EventRecord::PidEnd { .. })));
}

#[test]
fn golden_vector_mini_absolute_profile() {
    let mini = read_vec("profiles/mini_absolute_none.bin");
    let (prof, n) = decode_mini_profile(&mini).expect("mini");
    assert_eq!(n, mini.len());
    assert_eq!(prof.prefix.header.major, SUPPORTED_MAJOR);
    assert_eq!(prof.records.len(), 2);
    assert!(matches!(prof.records[0], EventRecord::TimeLine { ticks: 5, .. }));
    assert!(matches!(prof.records[1], EventRecord::Discount));
}

#[test]
fn wire_freeze_id_catalog_matches_c_header_values() {
    // Mirrors collector/include/nytprof_v6_ids.h (ADR-0006 frozen).
    assert_eq!(MAGIC, b"NYTPROF6");
    assert_eq!(SUPPORTED_MAJOR, 6);
    assert_eq!(HEADER_LEN_FULL, 36);
    assert_eq!(CHUNK_SYNC, 0x3654_594E);
    assert_eq!(kind::EVENT, 1);
    assert_eq!(kind::FOOTER, 5);
    assert_eq!(codec::NONE, 0);
    assert_eq!(codec::ZLIB, 1);
    assert_eq!(codec::ZSTD, 2);
    assert_eq!(codec::LZ4, 3);
    assert_eq!(opcode::TIME_LINE, 2);
    assert_eq!(opcode::TIME_LINE_RUN, 18);
    assert_eq!(opcode::TIME_BLOCK_RUN, 19);
    assert_eq!(FLAG_SITE_DELTA, 0x04);
    assert_eq!(FLAG_HAS_SEQ, 0x08);
    assert_eq!(type_id::END, 0x7e);
    assert_eq!(MAX_TIME_LINE_RUN_LEN, 1_048_576);
    assert_eq!(MAX_TIME_BLOCK_RUN_LEN, 1_048_576);
    assert!(is_known_opcode(opcode::TIME_LINE_RUN));
    assert!(is_known_opcode(opcode::TIME_BLOCK_RUN));
    // Ensure fixture tree present (regression: freeze without vectors).
    assert!(Path::new(&vectors_root().join("manifest.json")).is_file());
}

//! One-shot generator for `fixtures/v6/vectors` (PR-B11 / ADR-0006).
//!
//! ```sh
//! cargo run -p nytprof-format-v6 --example gen_wire_vectors -- fixtures/v6/vectors
//! cargo build -p nytprof-format-v6 --example gen_wire_vectors   # compile smoke
//! (cd fixtures/v6/vectors && sha256sum -c SHA256SUMS)
//! ```
//!
//! Not a product CLI. Vectors are immutable golden bytes under FMT-012 class.
//! Writes `.bin` files, `SHA256SUMS`, and `manifest.json` in one pass.

use nytprof_format_v6::chunk::{codec, encode_chunk_frame, kind, FLAG_KIND_REQUIRED};
use nytprof_format_v6::event_body::{
    encode_event_body, encode_event_body_with_seq, encode_event_body_with_site_deltas_and_seq,
    EventRecordSpec,
};
use nytprof_format_v6::mini_profile::{decode_mini_profile, encode_mini_profile};
use nytprof_format_v6::string_dict::encode_string_dictionary;
use nytprof_format_v6::tlv::{encode_tlv_region, type_id, FLAG_TYPE_REQUIRED};
use nytprof_format_v6::varint::{encode_i64, encode_u64};
use nytprof_format_v6::{encode_fixed_header_full, MAGIC, SUPPORTED_MAJOR};
use std::fs;
use std::path::{Path, PathBuf};

fn write_bin(dir: &Path, name: &str, bytes: &[u8], registry: &mut Vec<(String, Vec<u8>)>) {
    let p = dir.join(name);
    fs::write(&p, bytes).unwrap_or_else(|e| panic!("write {}: {e}", p.display()));
    println!("wrote {} ({} bytes)", p.display(), bytes.len());
    let rel = format!(
        "{}/{}",
        dir.file_name().and_then(|s| s.to_str()).unwrap_or("."),
        name
    );
    registry.push((rel, bytes.to_vec()));
}

fn sha256_hex(message: &[u8]) -> String {
    // Compact SHA-256 (same algorithm as golden_vectors test) — no extra deps.
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

fn main() {
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("fixtures/v6/vectors"));
    let prim = out.join("primitives");
    let event = out.join("event");
    let profiles = out.join("profiles");
    for d in [&out, &prim, &event, &profiles] {
        fs::create_dir_all(d).unwrap();
    }

    let mut registry: Vec<(String, Vec<u8>)> = Vec::new();

    // --- primitives ---
    write_bin(&prim, "uleb_0.bin", &encode_u64(0), &mut registry);
    write_bin(&prim, "uleb_1.bin", &encode_u64(1), &mut registry);
    write_bin(&prim, "uleb_127.bin", &encode_u64(127), &mut registry);
    write_bin(&prim, "uleb_128.bin", &encode_u64(128), &mut registry);
    write_bin(&prim, "uleb_300.bin", &encode_u64(300), &mut registry);
    write_bin(&prim, "uleb_max_u64.bin", &encode_u64(u64::MAX), &mut registry);
    write_bin(&prim, "zigzag_0.bin", &encode_i64(0), &mut registry);
    write_bin(&prim, "zigzag_neg1.bin", &encode_i64(-1), &mut registry);
    write_bin(&prim, "zigzag_1.bin", &encode_i64(1), &mut registry);
    write_bin(&prim, "zigzag_neg2.bin", &encode_i64(-2), &mut registry);

    let hdr = encode_fixed_header_full(SUPPORTED_MAJOR, 0, 0, 0, 0);
    assert_eq!(&hdr[..8], MAGIC.as_slice());
    write_bin(&prim, "fixed_header_full.bin", &hdr, &mut registry);

    let chunk = encode_chunk_frame(
        kind::EVENT,
        codec::NONE,
        FLAG_KIND_REQUIRED,
        0,
        0,
        0,
        0,
        &[],
        0,
    );
    write_bin(&prim, "chunk_event_none_empty.bin", &chunk, &mut registry);

    let tps = encode_u64(1_000_000);
    let tlv = encode_tlv_region(&[
        (type_id::PRODUCER, 0, b"nytprof-format-v6".as_slice()),
        (type_id::TICKS_PER_SEC, FLAG_TYPE_REQUIRED, tps.as_slice()),
    ]);
    write_bin(&prim, "tlv_region_producer_tps.bin", &tlv, &mut registry);

    let dict = encode_string_dictionary(&[(1u64, 0u8, b"hello".as_slice())]).expect("dict");
    write_bin(&prim, "string_dict_one_entry.bin", &dict, &mut registry);

    // --- event bodies ---
    write_bin(
        &event,
        "time_line_1_2_3.bin",
        &encode_event_body(&[EventRecordSpec::TimeLine {
            fid: 1,
            line: 2,
            ticks: 3,
        }]),
        &mut registry,
    );
    write_bin(
        &event,
        "discount.bin",
        &encode_event_body(&[EventRecordSpec::Discount]),
        &mut registry,
    );
    write_bin(
        &event,
        "time_line_run_n2.bin",
        &encode_event_body(&[EventRecordSpec::TimeLineRun {
            fid: 1,
            line: 5,
            ticks: &[10, 20],
        }]),
        &mut registry,
    );
    let seq = encode_event_body_with_site_deltas_and_seq(&[
        EventRecordSpec::TimeLine {
            fid: 1,
            line: 10,
            ticks: 1,
        },
        EventRecordSpec::TimeLine {
            fid: 1,
            line: 11,
            ticks: 2,
        },
        EventRecordSpec::SubEntry {
            caller_fid: 1,
            caller_line: 11,
        },
    ])
    .expect("site-delta+seq");
    write_bin(&event, "site_delta_seq_tl_tl_se.bin", &seq, &mut registry);

    // Order-only dual-output stream (no FLAG_HAS_SEQ) — dump-aligned shape.
    write_bin(
        &event,
        "dual_output_sequence.bin",
        &encode_event_body(&[
            EventRecordSpec::Version { major: 6, minor: 0 },
            EventRecordSpec::Comment {
                string_id: 0,
                string_flags: 0,
                text: b"c",
            },
            EventRecordSpec::StartDeflate,
            EventRecordSpec::PidStart {
                pid: 1,
                ppid: 0,
                start_time: 0,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 1,
            },
            EventRecordSpec::PidEnd {
                pid: 1,
                end_time: 1,
            },
        ]),
        &mut registry,
    );

    // OQ-5 / ADR-0006 §3: VERSION + START_DEFLATE may carry FLAG_HAS_SEQ in one
    // monotonic space with TIME_LINE (encode_event_body_with_seq).
    write_bin(
        &event,
        "dual_output_seq_oq5.bin",
        &encode_event_body_with_seq(&[
            EventRecordSpec::Version { major: 6, minor: 0 },
            EventRecordSpec::StartDeflate,
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 7,
            },
            EventRecordSpec::Discount,
        ]),
        &mut registry,
    );

    // --- mini profile ---
    let mini = encode_mini_profile(
        SUPPORTED_MAJOR,
        0,
        0,
        0,
        0,
        &[],
        &[
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 5,
            },
            EventRecordSpec::Discount,
        ],
        None,
    );
    let (decoded, n) = decode_mini_profile(&mini).expect("decode mini");
    assert_eq!(n, mini.len());
    assert_eq!(decoded.records.len(), 2);
    write_bin(&profiles, "mini_absolute_none.bin", &mini, &mut registry);

    registry.sort_by(|a, b| a.0.cmp(&b.0));

    // SHA256SUMS (paths relative to vectors dir: ./event/foo.bin)
    let mut sums = String::new();
    for (rel, bytes) in &registry {
        sums.push_str(&format!("{}  ./{}\n", sha256_hex(bytes), rel));
    }
    let sums_path = out.join("SHA256SUMS");
    fs::write(&sums_path, &sums).expect("write SHA256SUMS");
    println!("wrote {}", sums_path.display());

    // manifest.json
    let mut files_json = String::from("[\n");
    for (i, (rel, bytes)) in registry.iter().enumerate() {
        if i > 0 {
            files_json.push_str(",\n");
        }
        files_json.push_str(&format!(
            "    {{\n      \"path\": \"{}\",\n      \"bytes\": {},\n      \"sha256\": \"{}\"\n    }}",
            rel,
            bytes.len(),
            sha256_hex(bytes)
        ));
    }
    files_json.push_str("\n  ]");
    let manifest = format!(
        r#"{{
  "schema": "v6-wire-vectors-v1",
  "adr": "docs/adrs/0006-v6-wire-freeze.md",
  "catalog": "docs/schemas/v6-wire-ids-frozen-v1.md",
  "supported_major": 6,
  "generator": "cargo run -p nytprof-format-v6 --example gen_wire_vectors",
  "files": {files_json}
}}
"#
    );
    let man_path = out.join("manifest.json");
    fs::write(&man_path, manifest).expect("write manifest.json");
    println!("wrote {}", man_path.display());

    println!("OK vectors under {} ({} files)", out.display(), registry.len());
}

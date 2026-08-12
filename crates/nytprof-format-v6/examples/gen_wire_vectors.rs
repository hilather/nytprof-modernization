//! One-shot generator for `fixtures/v6/vectors` (PR-B11 / ADR-0006).
//!
//! ```sh
//! cargo run -p nytprof-format-v6 --example gen_wire_vectors -- fixtures/v6/vectors
//! ```
//!
//! Not a product CLI. Vectors are immutable golden bytes under FMT-012 class.

use nytprof_format_v6::chunk::{codec, encode_chunk_frame, kind, FLAG_KIND_REQUIRED};
use nytprof_format_v6::event_body::{
    encode_event_body, encode_event_body_with_site_deltas_and_seq, EventRecordSpec,
};
use nytprof_format_v6::mini_profile::{decode_mini_profile, encode_mini_profile};
use nytprof_format_v6::string_dict::encode_string_dictionary;
use nytprof_format_v6::tlv::{encode_tlv_region, type_id, FLAG_TYPE_REQUIRED};
use nytprof_format_v6::varint::{encode_i64, encode_u64};
use nytprof_format_v6::{encode_fixed_header_full, MAGIC, SUPPORTED_MAJOR};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn write(dir: &Path, name: &str, bytes: &[u8]) {
    let p = dir.join(name);
    fs::write(&p, bytes).unwrap_or_else(|e| panic!("write {}: {e}", p.display()));
    println!("wrote {} ({} bytes)", p.display(), bytes.len());
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

    // --- primitives ---
    write(&prim, "uleb_0.bin", &encode_u64(0));
    write(&prim, "uleb_1.bin", &encode_u64(1));
    write(&prim, "uleb_127.bin", &encode_u64(127));
    write(&prim, "uleb_128.bin", &encode_u64(128));
    write(&prim, "uleb_300.bin", &encode_u64(300));
    write(&prim, "uleb_max_u64.bin", &encode_u64(u64::MAX));
    write(&prim, "zigzag_0.bin", &encode_i64(0));
    write(&prim, "zigzag_neg1.bin", &encode_i64(-1));
    write(&prim, "zigzag_1.bin", &encode_i64(1));
    write(&prim, "zigzag_neg2.bin", &encode_i64(-2));

    let hdr = encode_fixed_header_full(SUPPORTED_MAJOR, 0, 0, 0, 0);
    assert_eq!(&hdr[..8], MAGIC.as_slice());
    write(&prim, "fixed_header_full.bin", &hdr);

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
    write(&prim, "chunk_event_none_empty.bin", &chunk);

    let tps = encode_u64(1_000_000);
    let tlv = encode_tlv_region(&[
        (type_id::PRODUCER, 0, b"nytprof-format-v6".as_slice()),
        (type_id::TICKS_PER_SEC, FLAG_TYPE_REQUIRED, tps.as_slice()),
    ]);
    write(&prim, "tlv_region_producer_tps.bin", &tlv);

    let dict = encode_string_dictionary(&[(1u64, 0u8, b"hello".as_slice())]).expect("dict");
    write(&prim, "string_dict_one_entry.bin", &dict);

    // --- event bodies ---
    write(
        &event,
        "time_line_1_2_3.bin",
        &encode_event_body(&[EventRecordSpec::TimeLine {
            fid: 1,
            line: 2,
            ticks: 3,
        }]),
    );
    write(&event, "discount.bin", &encode_event_body(&[EventRecordSpec::Discount]));
    write(
        &event,
        "time_line_run_n2.bin",
        &encode_event_body(&[EventRecordSpec::TimeLineRun {
            fid: 1,
            line: 5,
            ticks: &[10, 20],
        }]),
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
    write(&event, "site_delta_seq_tl_tl_se.bin", &seq);

    write(
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
    write(&profiles, "mini_absolute_none.bin", &mini);

    // Refresh SHA256SUMS via sha256sum when available
    if Command::new("sha256sum")
        .current_dir(&out)
        .arg("-b")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        // rewritten below with find
    }
    let status = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "cd {} && find . -type f -name "*.bin" -print0 | sort -z | xargs -0 sha256sum > SHA256SUMS",
            out.display()
        ))
        .status();
    match status {
        Ok(s) if s.success() => println!("wrote {}/SHA256SUMS", out.display()),
        _ => eprintln!("warning: could not write SHA256SUMS (sha256sum missing?)"),
    }

    println!("OK vectors under {}", out.display());
}

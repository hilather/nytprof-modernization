//! Engineering dual-path: decode C absolute v6 wire (PR-B06/B07).
//!
//! - codec NONE single-chunk: `decode_mini_profile` + always-inflate path
//! - ZLIB/ZSTD/LZ4 and multi-chunk: `decode_decoded_event_profile` (always-inflate)
//! - `--require-crc`: payload CRC verify (always-inflate) **and** header CRC
//!
//! Not a product CLI surface.
use std::env;
use std::fs;
use std::process;

use nytprof_format_v6::chunk::codec;
use nytprof_format_v6::compressed_profile::OwnedEventRecord;
use nytprof_format_v6::crc::verify_header_crc;
use nytprof_format_v6::decoded_event::decode_decoded_event_profile;
use nytprof_format_v6::mini_profile::decode_mini_profile;
use nytprof_format_v6::HEADER_LEN_FULL;

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: decode_abs_c_mini <path.nytprof> [--require-crc]");
            process::exit(2);
        }
    };
    let verify_crc = env::args().any(|a| a == "--require-crc");
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read {path}: {e}");
            process::exit(1);
        }
    };

    if verify_crc {
        if bytes.len() < HEADER_LEN_FULL as usize {
            eprintln!("truncated for header CRC verify");
            process::exit(1);
        }
        if let Err(e) = verify_header_crc(&bytes[..HEADER_LEN_FULL as usize]) {
            eprintln!("verify_header_crc: {e}");
            process::exit(1);
        }
    }

    // Always-inflate consumer (handles NONE/ZLIB/ZSTD/LZ4 + multi-chunk).
    // verify_crc here is **payload** CRC per chunk (decode_chunk_frame_plain).
    let (prof, n) = match decode_decoded_event_profile(&bytes, verify_crc) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("decode_decoded_event_profile: {e}");
            process::exit(1);
        }
    };
    if n != bytes.len() {
        eprintln!("truncated: consumed {n} of {}", bytes.len());
        process::exit(1);
    }
    if prof.header.major != 6 {
        eprintln!("bad major {}", prof.header.major);
        process::exit(1);
    }
    let tl = prof
        .records
        .iter()
        .filter(|r| matches!(r, OwnedEventRecord::TimeLine { .. }))
        .count();
    if prof.records.is_empty() || tl == 0 {
        eprintln!(
            "expected records with TIME_LINE; got records={} time_line={tl}",
            prof.records.len()
        );
        process::exit(1);
    }

    // Codec-NONE single-chunk also accepted by legacy mini decoder (compat).
    if prof.event_codec == codec::NONE && prof.event_chunk_count <= 1 {
        match decode_mini_profile(&bytes) {
            Ok((mp, mn)) if mn == bytes.len() && mp.records.len() == prof.records.len() => {}
            Ok((mp, mn)) => {
                eprintln!(
                    "mini path mismatch: mini_n={mn} mini_recs={} always_recs={}",
                    mp.records.len(),
                    prof.records.len()
                );
                process::exit(1);
            }
            Err(e) => {
                eprintln!("decode_mini_profile (NONE path): {e}");
                process::exit(1);
            }
        }
    }

    println!(
        "OK: v6 decode path={path} bytes={} records={} time_line={tl} codec={} chunks={} crc_verify={verify_crc} (header+payload)",
        bytes.len(),
        prof.records.len(),
        prof.event_codec,
        prof.event_chunk_count,
    );
}

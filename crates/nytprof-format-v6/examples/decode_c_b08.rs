//! Engineering dual-path: decode C v6 packing / FOOTER dict wire (PR-B08).
//!
//! ```text
//! cargo run -p nytprof-format-v6 --example decode_c_b08 -- <path> [--dict]
//! ```
//! Not a product CLI surface.
use std::env;
use std::fs;
use std::process;

use nytprof_format_v6::compressed_profile::OwnedEventRecord;
use nytprof_format_v6::crc::verify_header_crc;
use nytprof_format_v6::decoded_event::{
    decode_decoded_event_profile, decode_decoded_event_profile_with_string_dict,
};
use nytprof_format_v6::HEADER_LEN_FULL;

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: decode_c_b08 <path.nytprof> [--dict]");
            process::exit(2);
        }
    };
    let dict = env::args().any(|a| a == "--dict");
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read {path}: {e}");
            process::exit(1);
        }
    };
    if bytes.len() < HEADER_LEN_FULL as usize {
        eprintln!("truncated for header CRC");
        process::exit(1);
    }
    if let Err(e) = verify_header_crc(&bytes[..HEADER_LEN_FULL as usize]) {
        eprintln!("verify_header_crc: {e}");
        process::exit(1);
    }

    if dict {
        let (p, _d, n) = match decode_decoded_event_profile_with_string_dict(&bytes, true) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("decode_with_string_dict: {e}");
                process::exit(1);
            }
        };
        if n != bytes.len() {
            eprintln!("truncated: consumed {n} of {}", bytes.len());
            process::exit(1);
        }
        println!(
            "OK dict path={path} records={} chunks={} has_footer={}",
            p.records.len(),
            p.event_chunk_count,
            p.has_footer
        );
    } else {
        let (p, n) = match decode_decoded_event_profile(&bytes, true) {
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
        let tl = p
            .records
            .iter()
            .filter(|r| matches!(r, OwnedEventRecord::TimeLine { .. }))
            .count();
        println!(
            "OK path={path} records={} time_line={tl} chunks={}",
            p.records.len(),
            p.event_chunk_count
        );
    }
}

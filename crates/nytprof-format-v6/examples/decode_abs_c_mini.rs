//! Engineering dual-path: decode C absolute v6 mini (PR-B06 / COL-007-ABS).
//! Not a product CLI surface.
use std::env;
use std::fs;
use std::process;
use nytprof_format_v6::{decode_mini_profile, EventRecord};

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: decode_abs_c_mini <path.nytprof>");
            process::exit(2);
        }
    };
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read {path}: {e}");
            process::exit(1);
        }
    };
    let (mp, n) = match decode_mini_profile(&bytes) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("decode_mini_profile: {e}");
            process::exit(1);
        }
    };
    if n != bytes.len() {
        eprintln!("truncated: consumed {n} of {}", bytes.len());
        process::exit(1);
    }
    if mp.prefix.header.major != 6 {
        eprintln!("bad major {}", mp.prefix.header.major);
        process::exit(1);
    }
    let tl = mp
        .records
        .iter()
        .filter(|r| matches!(r, EventRecord::TimeLine { .. }))
        .count();
    if mp.records.is_empty() || tl == 0 {
        eprintln!(
            "expected records with TIME_LINE; got records={} time_line={tl}",
            mp.records.len()
        );
        process::exit(1);
    }
    println!(
        "OK: v6 decode path={path} bytes={} records={} time_line={tl}",
        bytes.len(),
        mp.records.len()
    );
}

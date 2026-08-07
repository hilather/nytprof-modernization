//! DECODE-FUZZ-MVP: shipped `decode_all` / `decode_path` never panic on corrupt
//! / truncated / single-byte-mutated inputs (Ok or Err only).
//!
//! Schema: docs/schemas/decode-fuzz-mvp-v0.md
//! Does **not** reimplement the decoder — calls the public crate API only.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nytprof_format_v5::{decode_all, decode_path};

fn fixture_default_calls1() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/v5/default-calls1/nytprof.out")
}

fn read_golden() -> Vec<u8> {
    let path = fixture_default_calls1();
    assert!(path.is_file(), "missing fixture {}", path.display());
    std::fs::read(&path).expect("read golden default-calls1")
}

/// DECODE-FUZZ-MVP (a): empty input → Err (Result path; panic fails the test).
#[test]
fn decode_fuzz_no_panic_empty() {
    let r = decode_all(b"");
    assert!(r.is_err(), "empty must Err, got Ok({})", r.unwrap().len());
}

/// DECODE-FUZZ-MVP (b): bad magic → Err.
#[test]
fn decode_fuzz_no_panic_bad_magic() {
    let r = decode_all(b"NOTPROF 5 0\n");
    assert!(r.is_err(), "bad magic must Err, got Ok");
    let r2 = decode_all(b"\x00\x01\x02garbage");
    assert!(r2.is_err(), "garbage header must Err, got Ok");
}

/// DECODE-FUZZ-MVP (c): mid-file half of default-calls1 → Err via decode_all
/// and decode_path.
#[test]
fn decode_fuzz_no_panic_mid_file_half() {
    let bytes = read_golden();
    let half = bytes.len() / 2;
    assert!(half > 0, "fixture empty");

    let r = decode_all(&bytes[..half]);
    assert!(
        r.is_err(),
        "half of default-calls1 must Err decode_all, got Ok({})",
        r.as_ref().map(|v| v.len()).unwrap_or(0)
    );

    let tmp = std::env::temp_dir().join(format!(
        "nytprof-decode-fuzz-half-{}-{}.out",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&tmp, &bytes[..half]).expect("write temp half");
    let path_r = decode_path(&tmp);
    let _ = std::fs::remove_file(&tmp);
    assert!(
        path_r.is_err(),
        "half of default-calls1 must Err decode_path, got Ok"
    );
}

/// DECODE-FUZZ-MVP (d): stepped prefixes of default-calls1 — every cut is
/// Ok or Err only (no panic). Full-length prefix may Ok; shorter cuts almost
/// always Err (EOF / format / zlib).
#[test]
fn fuzz_truncated_mutations() {
    let bytes = read_golden();
    let n = bytes.len();
    assert!(n > 64, "fixture too small for prefix battery");

    // Stepped prefixes: ~32–64 cuts covering the whole file.
    let step = (n / 48).max(1);
    let mut cuts: Vec<usize> = (0..=n).step_by(step).collect();
    // Fixed interesting cuts (empty / tiny / half / near-end / full).
    for extra in [0usize, 1, 12, 20, 64, 128, 256, 500, n / 4, n / 2, n.saturating_sub(1), n] {
        let extra = extra.min(n);
        if !cuts.contains(&extra) {
            cuts.push(extra);
        }
    }
    cuts.sort_unstable();
    cuts.dedup();

    let mut ok_count = 0usize;
    let mut err_count = 0usize;
    for &len in &cuts {
        // Result-only API: any panic aborts this test as failure.
        match decode_all(&bytes[..len]) {
            Ok(_) => ok_count += 1,
            Err(_) => err_count += 1,
        }
    }
    assert_eq!(
        ok_count + err_count,
        cuts.len(),
        "every cut must yield Ok or Err"
    );
    // Empty and mid-file must be among Err outcomes.
    assert!(decode_all(&bytes[..0]).is_err(), "prefix len 0 must Err");
    assert!(
        decode_all(&bytes[..n / 2]).is_err(),
        "prefix half must Err"
    );
    // Full golden must still Ok (sanity).
    assert!(
        decode_all(&bytes).is_ok(),
        "full default-calls1 must still decode Ok"
    );
    assert!(
        err_count > 0,
        "expected some truncated prefixes to Err (got ok={ok_count} err={err_count})"
    );
    let _ = ok_count;
}

/// DECODE-FUZZ-MVP: single-byte XOR mutations across default-calls1 — never
/// panic. Ok is allowed if the flip is effectively inert; Err is expected
/// for most offsets. Does not reimplement the decoder.
#[test]
fn decode_fuzz_no_panic_byte_xor_mutations() {
    let golden = read_golden();
    let n = golden.len();
    assert!(n > 64, "fixture too small");

    // 48 deterministic offsets spanning the file + header-sensitive spots.
    let mut offsets: Vec<usize> = Vec::with_capacity(64);
    offsets.extend([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 20, 32, 64]);
    let stride = (n / 40).max(1);
    let mut o = 0usize;
    while o < n && offsets.len() < 56 {
        if !offsets.contains(&o) {
            offsets.push(o);
        }
        o = o.saturating_add(stride);
    }
    // A few mid/end spots.
    for extra in [n / 3, n / 2, (2 * n) / 3, n.saturating_sub(8), n.saturating_sub(1)] {
        if extra < n && !offsets.contains(&extra) {
            offsets.push(extra);
        }
    }
    offsets.sort_unstable();
    offsets.dedup();
    assert!(
        offsets.len() >= 32,
        "expected ≥32 mutation offsets, got {}",
        offsets.len()
    );
    assert!(
        offsets.len() <= 64,
        "keep mutation battery modest (≤64), got {}",
        offsets.len()
    );

    let mut err_count = 0usize;
    let mut ok_count = 0usize;
    for &off in &offsets {
        let mut mutated = golden.clone();
        mutated[off] ^= 0xFF;
        // Prefer Result-only: panic in decode_all fails the suite.
        match decode_all(&mutated) {
            Ok(_) => ok_count += 1,
            Err(_) => err_count += 1,
        }
    }
    assert_eq!(ok_count + err_count, offsets.len());
    // Magic / header flips must Err.
    let mut magic_flip = golden.clone();
    magic_flip[0] ^= 0xFF; // 'N' → not NYTProf
    assert!(
        decode_all(&magic_flip).is_err(),
        "XOR first magic byte must Err"
    );
    assert!(
        err_count > 0,
        "expected some XOR mutations to Err (ok={ok_count} err={err_count})"
    );
    let _ = ok_count;
}

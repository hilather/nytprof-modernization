//! SEC-FUZZ-HARDENING-MVP / V6-DECODE-FUZZ: shipped always-inflate EVENT decode
//! never panics on corrupt / truncated / single-byte-mutated **C-produced**
//! fixtures (Ok or Err only).
//!
//! Schema: docs/schemas/security-fuzz-hardening-mvp-v0.md
//! Contract: docs/contracts/SECURITY_FUZZ_HARDENING_PACKAGE_v0.md
//!
//! Calls `e3_decode_writer_bytes` / `decode_decoded_event_profile` only —
//! does **not** reimplement the decoder or re-encode via `e3_standin_*`.

use std::path::PathBuf;

use nytprof_format_v6::{
    decode_decoded_event_profile, e3_decode_writer_bytes,
};

fn from_c_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/v6/from-c")
        .join(name)
}

fn read_c_fixture(name: &str) -> Vec<u8> {
    let path = from_c_fixture(name);
    assert!(
        path.is_file(),
        "missing C fixture {} — regenerate: make -C collector gen-e3-fixtures OUTDIR=fixtures/v6/from-c",
        path.display()
    );
    std::fs::read(&path).expect("read C fixture")
}

/// (a) empty → Err (Result path; panic fails the test).
#[test]
fn v6_decode_fuzz_no_panic_empty() {
    let r = e3_decode_writer_bytes(b"", true, false);
    assert!(r.is_err(), "empty must Err, got Ok");
    let r2 = decode_decoded_event_profile(b"", true);
    assert!(r2.is_err(), "empty decode_decoded_event_profile must Err");
}

/// (b) bad magic → Err.
#[test]
fn v6_decode_fuzz_no_panic_bad_magic() {
    let r = e3_decode_writer_bytes(b"NOTPROF 6 0\n", true, false);
    assert!(r.is_err(), "bad magic must Err, got Ok");
    let r2 = e3_decode_writer_bytes(b"\x00\x01\x02garbage", false, false);
    assert!(r2.is_err(), "garbage header must Err, got Ok");
}

/// (c) mid-file half of absolute C fixture → Err.
#[test]
fn v6_decode_fuzz_no_panic_mid_file_half() {
    let bytes = read_c_fixture("absolute.nytprof");
    let half = bytes.len() / 2;
    assert!(half > 0, "fixture empty");

    let r = e3_decode_writer_bytes(&bytes[..half], true, false);
    assert!(
        r.is_err(),
        "half of absolute.nytprof must Err e3_decode, got Ok"
    );
    let r2 = decode_decoded_event_profile(&bytes[..half], true);
    assert!(
        r2.is_err(),
        "half of absolute.nytprof must Err decode_decoded_event_profile"
    );
}

/// (d) stepped prefixes of absolute.nytprof — every cut is Ok or Err only.
#[test]
fn fuzz_truncated_mutations_v6() {
    let bytes = read_c_fixture("absolute.nytprof");
    let n = bytes.len();
    assert!(n > 16, "fixture too small for prefix battery ({n})");

    let step = (n / 24).max(1);
    let mut cuts: Vec<usize> = (0..=n).step_by(step).collect();
    for extra in [
        0usize,
        1,
        4,
        8,
        12,
        16,
        32,
        40,
        n / 4,
        n / 2,
        n.saturating_sub(1),
        n,
    ] {
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
        match e3_decode_writer_bytes(&bytes[..len], true, false) {
            Ok(_) => ok_count += 1,
            Err(_) => err_count += 1,
        }
    }
    assert_eq!(
        ok_count + err_count,
        cuts.len(),
        "every cut must yield Ok or Err"
    );
    assert!(
        e3_decode_writer_bytes(&bytes[..0], true, false).is_err(),
        "prefix len 0 must Err"
    );
    assert!(
        e3_decode_writer_bytes(&bytes[..n / 2], true, false).is_err(),
        "prefix half must Err"
    );
    assert!(
        e3_decode_writer_bytes(&bytes, true, false).is_ok(),
        "full absolute.nytprof must still decode Ok"
    );
    assert!(
        err_count > 0,
        "expected some truncated prefixes to Err (got ok={ok_count} err={err_count})"
    );
    let _ = ok_count;
}

/// Single-byte XOR mutations across absolute.nytprof — never panic.
#[test]
fn v6_decode_fuzz_no_panic_byte_xor_mutations() {
    let golden = read_c_fixture("absolute.nytprof");
    let n = golden.len();
    assert!(n > 16, "fixture too small");

    let mut offsets: Vec<usize> = Vec::with_capacity(48);
    offsets.extend([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 15]);
    let stride = (n / 16).max(1);
    let mut o = 0usize;
    while o < n && offsets.len() < 40 {
        if !offsets.contains(&o) {
            offsets.push(o);
        }
        o = o.saturating_add(stride);
    }
    for extra in [n / 3, n / 2, (2 * n) / 3, n.saturating_sub(4), n.saturating_sub(1)] {
        if extra < n && !offsets.contains(&extra) {
            offsets.push(extra);
        }
    }
    offsets.sort_unstable();
    offsets.dedup();
    assert!(
        offsets.len() >= 16,
        "expected ≥16 mutation offsets, got {}",
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
        match e3_decode_writer_bytes(&mutated, true, false) {
            Ok(_) => ok_count += 1,
            Err(_) => err_count += 1,
        }
    }
    assert_eq!(ok_count + err_count, offsets.len());

    let mut magic_flip = golden.clone();
    magic_flip[0] ^= 0xFF;
    assert!(
        e3_decode_writer_bytes(&magic_flip, true, false).is_err(),
        "XOR first magic byte must Err"
    );
    assert!(
        err_count > 0,
        "expected some XOR mutations to Err (ok={ok_count} err={err_count})"
    );
    let _ = ok_count;
}

/// Packing + dict C fixtures: full Ok; half Err; modest XOR never panic.
#[test]
fn v6_decode_fuzz_no_panic_packing_and_dict_fixtures() {
    // packing: no FOOTER dict
    let packing = read_c_fixture("packing.nytprof");
    assert!(
        e3_decode_writer_bytes(&packing, true, false).is_ok(),
        "full packing.nytprof must Ok"
    );
    let half_p = packing.len() / 2;
    assert!(
        e3_decode_writer_bytes(&packing[..half_p], true, false).is_err(),
        "half packing must Err"
    );

    // dict: FOOTER string-dict path
    let dict = read_c_fixture("dict.nytprof");
    assert!(
        e3_decode_writer_bytes(&dict, true, true).is_ok(),
        "full dict.nytprof must Ok with expect_string_dict"
    );
    let half_d = dict.len() / 2;
    assert!(
        e3_decode_writer_bytes(&dict[..half_d], true, true).is_err(),
        "half dict must Err"
    );

    // modest XOR battery on packing (span file without explosion)
    let n = packing.len();
    let stride = (n / 12).max(1);
    let mut o = 0usize;
    let mut trials = 0usize;
    while o < n && trials < 24 {
        let mut m = packing.clone();
        m[o] ^= 0xFF;
        let _ = e3_decode_writer_bytes(&m, true, false); // Ok or Err only
        o = o.saturating_add(stride);
        trials += 1;
    }
    assert!(trials >= 8, "expected several packing XOR trials");
}

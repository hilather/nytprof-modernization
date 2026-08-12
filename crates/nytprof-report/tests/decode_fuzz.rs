//! DECODE-FUZZ-MVP: shipped `verify_profile` never panics on corrupt /
//! truncated / single-byte-mutated inputs (Ok or Err only). Known fail-closed
//! classes (empty / bad magic / mid-file half) must Err.
//!
//! Schema: docs/schemas/decode-fuzz-mvp-v0.md
//! Does **not** reimplement the decoder — calls `nytprof_report::verify_profile` only.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nytprof_report::verify_profile;

fn fixture_default_calls1() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/v5/default-calls1/nytprof.out")
}

fn read_golden() -> Vec<u8> {
    let path = fixture_default_calls1();
    assert!(path.is_file(), "missing fixture {}", path.display());
    fs::read(&path).expect("read golden default-calls1")
}

fn fuzz_temp(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "nytprof-decode-fuzz-{}-{}-{}",
        label,
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

fn assert_verify_err(path: &Path, label: &str) {
    let r = verify_profile(path);
    assert!(
        r.is_err(),
        "{label}: verify_profile must Err, got Ok:\n{}",
        r.as_ref().unwrap()
    );
}

/// DECODE-FUZZ-MVP: battery — empty / bad magic / half default-calls1 must
/// Err on `verify_profile` (and not panic).
#[test]
fn decode_fuzz_no_panic_verify_empty_magic_half() {
    // (a) empty
    let empty = fuzz_temp("empty");
    fs::write(&empty, b"").expect("write empty");
    assert_verify_err(&empty, "empty");
    let _ = fs::remove_file(&empty);

    // (b) bad magic
    let bad = fuzz_temp("bad-magic");
    fs::write(&bad, b"NOTPROF 5 0\n").expect("write bad magic");
    assert_verify_err(&bad, "bad magic");
    let _ = fs::remove_file(&bad);

    // (c) half of default-calls1
    let bytes = read_golden();
    let half = bytes.len() / 2;
    assert!(half > 0);
    let trunc = fuzz_temp("half");
    fs::write(&trunc, &bytes[..half]).expect("write half");
    assert_verify_err(&trunc, "half default-calls1");
    let _ = fs::remove_file(&trunc);
}

/// DECODE-FUZZ-MVP (d): stepped prefixes of default-calls1 via
/// `verify_profile` — Ok or Err only (no panic). Full golden remains Ok;
/// empty / half must Err (fail-closed).
#[test]
fn fuzz_truncated_mutations_verify() {
    let path = fixture_default_calls1();
    let bytes = read_golden();
    let n = bytes.len();
    assert!(n > 64, "fixture too small");

    let step = (n / 32).max(1);
    let mut cuts: Vec<usize> = (0..=n).step_by(step).collect();
    for extra in [0usize, 1, 12, 64, 128, 500, n / 2, n.saturating_sub(1), n] {
        let extra = extra.min(n);
        if !cuts.contains(&extra) {
            cuts.push(extra);
        }
    }
    cuts.sort_unstable();
    cuts.dedup();

    // Keep the battery bounded: at most ~40 prefix cuts for verify (path I/O).
    if cuts.len() > 40 {
        let keep_step = (cuts.len() / 36).max(1);
        let mut thinned = Vec::with_capacity(40);
        for (i, &c) in cuts.iter().enumerate() {
            if i % keep_step == 0 || c == 0 || c == n / 2 || c == n {
                thinned.push(c);
            }
        }
        cuts = thinned;
        cuts.sort_unstable();
        cuts.dedup();
    }

    let mut ok_count = 0usize;
    let mut err_count = 0usize;
    for &len in &cuts {
        let tmp = fuzz_temp(&format!("prefix-{len}"));
        fs::write(&tmp, &bytes[..len]).expect("write prefix");
        // Result-only: any panic fails this test.
        match verify_profile(&tmp) {
            Ok(_) => ok_count += 1,
            Err(_) => err_count += 1,
        }
        let _ = fs::remove_file(&tmp);
    }
    assert_eq!(ok_count + err_count, cuts.len());

    // Fail-closed assertions for known corrupt classes.
    let empty = fuzz_temp("prefix-empty-assert");
    fs::write(&empty, b"").expect("write");
    assert_verify_err(&empty, "empty prefix");
    let _ = fs::remove_file(&empty);

    let half = fuzz_temp("prefix-half-assert");
    fs::write(&half, &bytes[..n / 2]).expect("write");
    assert_verify_err(&half, "half prefix");
    let _ = fs::remove_file(&half);

    // Full golden Ok (sanity; may already be covered elsewhere).
    assert!(
        verify_profile(&path).is_ok(),
        "full default-calls1 must verify Ok"
    );
    assert!(
        err_count > 0,
        "expected some prefixes to Err (ok={ok_count} err={err_count})"
    );
    let _ = ok_count;
}

/// DECODE-FUZZ-MVP: single-byte XOR mutations of default-calls1 through
/// `verify_profile` — never panic. Ok allowed only if the mutation is
/// still a complete valid profile (rare); most flips → Err.
#[test]
fn decode_fuzz_no_panic_verify_byte_xor_mutations() {
    let golden = read_golden();
    let n = golden.len();
    assert!(n > 64, "fixture too small");

    let mut offsets: Vec<usize> = vec![0, 1, 2, 4, 8, 11, 12, 20, 32, 64];
    let stride = (n / 28).max(1);
    let mut o = 0usize;
    while o < n && offsets.len() < 40 {
        if !offsets.contains(&o) {
            offsets.push(o);
        }
        o = o.saturating_add(stride);
    }
    for extra in [n / 2, n.saturating_sub(1)] {
        if extra < n && !offsets.contains(&extra) {
            offsets.push(extra);
        }
    }
    offsets.sort_unstable();
    offsets.dedup();
    // Cap at 48 path writes for CI time.
    if offsets.len() > 48 {
        offsets.truncate(48);
    }
    assert!(
        offsets.len() >= 32,
        "expected ≥32 mutation offsets, got {}",
        offsets.len()
    );

    let mut err_count = 0usize;
    let mut ok_count = 0usize;
    for &off in &offsets {
        let mut mutated = golden.clone();
        mutated[off] ^= 0xFF;
        let tmp = fuzz_temp(&format!("xor-{off}"));
        fs::write(&tmp, &mutated).expect("write mutated");
        match verify_profile(&tmp) {
            Ok(_) => ok_count += 1,
            Err(_) => err_count += 1,
        }
        let _ = fs::remove_file(&tmp);
    }
    assert_eq!(ok_count + err_count, offsets.len());

    // Magic flip must fail closed.
    let mut magic = golden.clone();
    magic[0] ^= 0xFF;
    let tmp = fuzz_temp("xor-magic");
    fs::write(&tmp, &magic).expect("write");
    assert_verify_err(&tmp, "XOR magic byte");
    let _ = fs::remove_file(&tmp);

    assert!(
        err_count > 0,
        "expected some XOR mutations to Err verify (ok={ok_count} err={err_count})"
    );
    let _ = ok_count;
}

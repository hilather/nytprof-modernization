//! Provisional **format v6** unsigned LEB128 / ULEB128 and signed ZigZag+ULEB128
//! (COL-007 runway).
//!
//! Schemas:
//! - `docs/schemas/v6-varint-uleb128-provisional-v0.md`
//! - `docs/schemas/v6-svarint-zigzag-provisional-v0.md`
//!
//! Independent of v5 packed integers in `nytprof-format-v5`.

use thiserror::Error;

/// Maximum bytes for a canonical or accepted `u64` ULEB128 (\(\lceil 64/7 \rceil\)).
pub const MAX_ULEB128_BYTES: usize = 10;

/// Fail-closed varint errors (never panic on crafted input).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum VarintError {
    #[error("truncated ULEB128: need more bytes at offset {offset}")]
    Truncated { offset: usize },

    #[error("ULEB128 too long (>{MAX_ULEB128_BYTES} bytes) at offset {offset}")]
    TooLong { offset: usize },

    #[error("ULEB128 overflow (exceeds u64) at offset {offset}")]
    Overflow { offset: usize },

    #[error("non-canonical overlong ULEB128 at offset {offset}")]
    NonCanonical { offset: usize },
}

pub type VarintResult<T> = std::result::Result<T, VarintError>;

/// Canonical encode of `value` as ULEB128 (minimum length).
pub fn encode_u64(mut value: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(MAX_ULEB128_BYTES);
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
            out.push(byte);
        } else {
            out.push(byte);
            break;
        }
    }
    debug_assert!(out.len() <= MAX_ULEB128_BYTES);
    out
}

/// Decode a **strict** canonical ULEB128 `u64` starting at `pos`.
///
/// Returns `(value, bytes_consumed)`. Fail-closed on truncated input, too-long
/// encodings, overflow, and non-canonical overlong forms.
pub fn decode_u64(data: &[u8], pos: usize) -> VarintResult<(u64, usize)> {
    decode_u64_inner(data, pos, true)
}

/// Decode ULEB128 without rejecting overlong forms (still fail-closed on
/// truncated / too long / overflow). Exposed for diagnostics; tests and the
/// provisional contract default to [`decode_u64`] (strict).
pub fn decode_u64_permissive(data: &[u8], pos: usize) -> VarintResult<(u64, usize)> {
    decode_u64_inner(data, pos, false)
}

fn decode_u64_inner(data: &[u8], pos: usize, strict: bool) -> VarintResult<(u64, usize)> {
    if pos >= data.len() {
        return Err(VarintError::Truncated { offset: pos });
    }

    let start = pos;
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    let mut p = pos;

    for _ in 0..MAX_ULEB128_BYTES {
        if p >= data.len() {
            return Err(VarintError::Truncated { offset: p });
        }
        let byte = data[p];
        p += 1;

        let payload = (byte & 0x7f) as u64;
        // Fail closed if shifting payload into result would exceed u64.
        if shift >= 64 {
            return Err(VarintError::Overflow { offset: start });
        }
        if payload > (u64::MAX >> shift) {
            return Err(VarintError::Overflow { offset: start });
        }
        result |= payload << shift;

        if byte & 0x80 == 0 {
            let consumed = p - start;
            if strict {
                let canon = encode_u64(result);
                if canon.as_slice() != &data[start..p] {
                    return Err(VarintError::NonCanonical { offset: start });
                }
            }
            return Ok((result, consumed));
        }

        shift = shift.saturating_add(7);
    }

    // Exhausted MAX_ULEB128_BYTES with continuation still set on last byte.
    Err(VarintError::TooLong { offset: start })
}

/// Write canonical ULEB128 into `out`, returning bytes written.
pub fn encode_u64_into(value: u64, out: &mut Vec<u8>) -> usize {
    let enc = encode_u64(value);
    let n = enc.len();
    out.extend_from_slice(&enc);
    n
}

// ---------------------------------------------------------------------------
// Signed: ZigZag + ULEB128 (provisional; SLEB128 residual alternative)
// ---------------------------------------------------------------------------

/// ZigZag-encode a signed `i64` to unsigned for ULEB128.
pub fn zigzag_encode_i64(n: i64) -> u64 {
    ((n << 1) ^ (n >> 63)) as u64
}

/// ZigZag-decode an unsigned value back to signed `i64`.
pub fn zigzag_decode_i64(u: u64) -> i64 {
    ((u >> 1) as i64) ^ (-((u & 1) as i64))
}

/// Canonical encode of signed `value` as ZigZag + ULEB128.
pub fn encode_i64(value: i64) -> Vec<u8> {
    encode_u64(zigzag_encode_i64(value))
}

/// Decode a **strict** signed value (ZigZag of strict ULEB128) at `pos`.
///
/// Returns `(value, bytes_consumed)`. Fail-closed behavior is inherited from
/// [`decode_u64`] (truncated / too long / overflow / non-canonical overlong).
pub fn decode_i64(data: &[u8], pos: usize) -> VarintResult<(i64, usize)> {
    let (u, n) = decode_u64(data, pos)?;
    Ok((zigzag_decode_i64(u), n))
}

/// Write canonical ZigZag+ULEB128 into `out`, returning bytes written.
pub fn encode_i64_into(value: i64, out: &mut Vec<u8>) -> usize {
    encode_u64_into(zigzag_encode_i64(value), out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_magnitudes() {
        let samples: &[u64] = &[
            0,
            1,
            127,
            128,
            255,
            300,
            16_383,
            16_384,
            1_000_000,
            u32::MAX as u64,
            (u32::MAX as u64) + 1,
            u64::MAX,
            0xDEAD_BEEF,
            0x0123_4567_89AB_CDEF,
        ];
        for &v in samples {
            let enc = encode_u64(v);
            assert!(
                enc.len() <= MAX_ULEB128_BYTES,
                "encode len for {v:#x}: {}",
                enc.len()
            );
            let (dec, n) = decode_u64(&enc, 0).unwrap_or_else(|e| {
                panic!("decode {v:#x} enc={enc:02x?}: {e}");
            });
            assert_eq!(n, enc.len(), "consumed all for {v:#x}");
            assert_eq!(dec, v, "roundtrip {v:#x}");
        }
    }

    #[test]
    fn encode_documented_examples() {
        assert_eq!(encode_u64(0), vec![0x00]);
        assert_eq!(encode_u64(1), vec![0x01]);
        assert_eq!(encode_u64(127), vec![0x7f]);
        assert_eq!(encode_u64(128), vec![0x80, 0x01]);
        assert_eq!(encode_u64(255), vec![0xff, 0x01]);
        assert_eq!(encode_u64(300), vec![0xac, 0x02]);
        assert_eq!(
            encode_u64(u64::MAX),
            vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01]
        );
    }

    #[test]
    fn truncated_empty_err() {
        assert_eq!(
            decode_u64(&[], 0),
            Err(VarintError::Truncated { offset: 0 })
        );
    }

    #[test]
    fn truncated_mid_continuation_err() {
        // 0x80 alone: continuation set, no following byte.
        assert_eq!(
            decode_u64(&[0x80], 0),
            Err(VarintError::Truncated { offset: 1 })
        );
        assert_eq!(
            decode_u64(&[0x80, 0x80], 0),
            Err(VarintError::Truncated { offset: 2 })
        );
    }

    #[test]
    fn non_canonical_overlong_strict_err() {
        // Value 0 encoded as overlong: 0x80 0x00 (continuation then zero terminator).
        let overlong = [0x80u8, 0x00];
        assert_eq!(
            decode_u64(&overlong, 0),
            Err(VarintError::NonCanonical { offset: 0 })
        );
        // Value 1 as 0x81 0x00 is also overlong (canonical is 0x01).
        assert_eq!(
            decode_u64(&[0x81, 0x00], 0),
            Err(VarintError::NonCanonical { offset: 0 })
        );
    }

    #[test]
    fn permissive_accepts_simple_overlong_zero() {
        let overlong = [0x80u8, 0x00];
        let (v, n) = decode_u64_permissive(&overlong, 0).expect("permissive");
        assert_eq!(v, 0);
        assert_eq!(n, 2);
    }

    #[test]
    fn too_long_eleven_continuations_err() {
        // 10 continuation bytes without terminator → TooLong (or Truncated if shorter).
        let mut buf = vec![0x80u8; 10];
        // 10th byte still has continuation; no 11th → after loop TooLong
        // Actually: for i in 0..10 we read 10 bytes all with cont bit; never return Ok; TooLong.
        assert_eq!(decode_u64(&buf, 0), Err(VarintError::TooLong { offset: 0 }));
        // 11 bytes of 0x80: still TooLong at max width (strict stops at 10).
        buf.push(0x80);
        assert_eq!(decode_u64(&buf, 0), Err(VarintError::TooLong { offset: 0 }));
    }

    #[test]
    fn decode_with_trailing_bytes() {
        let mut buf = encode_u64(128);
        buf.push(0xAA);
        let (v, n) = decode_u64(&buf, 0).unwrap();
        assert_eq!(v, 128);
        assert_eq!(n, 2);
        assert_eq!(buf[n], 0xAA);
    }

    #[test]
    fn encode_into_matches_encode() {
        let mut out = Vec::new();
        let n = encode_u64_into(300, &mut out);
        assert_eq!(n, out.len());
        assert_eq!(out, encode_u64(300));
    }

    // --- signed ZigZag + ULEB128 ---

    #[test]
    fn zigzag_table_small() {
        assert_eq!(zigzag_encode_i64(0), 0);
        assert_eq!(zigzag_encode_i64(-1), 1);
        assert_eq!(zigzag_encode_i64(1), 2);
        assert_eq!(zigzag_encode_i64(-2), 3);
        assert_eq!(zigzag_encode_i64(2), 4);
        for n in [0i64, -1, 1, -2, 2, 127, -128, i64::MAX, i64::MIN] {
            assert_eq!(zigzag_decode_i64(zigzag_encode_i64(n)), n, "zigzag {n}");
        }
    }

    #[test]
    fn signed_roundtrip_magnitudes() {
        let samples: &[i64] = &[
            0,
            -1,
            1,
            -2,
            2,
            127,
            -128,
            128,
            -129,
            1_000_000,
            -1_000_000,
            i32::MAX as i64,
            i32::MIN as i64,
            i64::MAX,
            i64::MIN,
            -42,
            42,
        ];
        for &v in samples {
            let enc = encode_i64(v);
            assert!(enc.len() <= MAX_ULEB128_BYTES, "len for {v}");
            let (dec, n) = decode_i64(&enc, 0).unwrap_or_else(|e| {
                panic!("decode_i64 {v} enc={enc:02x?}: {e}");
            });
            assert_eq!(n, enc.len(), "consumed all for {v}");
            assert_eq!(dec, v, "roundtrip {v}");
        }
    }

    #[test]
    fn signed_documented_examples() {
        assert_eq!(encode_i64(0), vec![0x00]);
        assert_eq!(encode_i64(-1), vec![0x01]);
        assert_eq!(encode_i64(1), vec![0x02]);
        assert_eq!(encode_i64(-2), vec![0x03]);
        assert_eq!(encode_i64(127), vec![0xfe, 0x01]);
        assert_eq!(encode_i64(-128), vec![0xff, 0x01]);
    }

    #[test]
    fn signed_truncated_err() {
        assert_eq!(
            decode_i64(&[], 0),
            Err(VarintError::Truncated { offset: 0 })
        );
        // ZigZag(-1)=1 canonical is 0x01; 0x80 alone is truncated ULEB128.
        assert_eq!(
            decode_i64(&[0x80], 0),
            Err(VarintError::Truncated { offset: 1 })
        );
    }

    #[test]
    fn signed_overlong_strict_err() {
        // Overlong encoding of unsigned 0 (ZigZag 0 → signed 0): 0x80 0x00.
        assert_eq!(
            decode_i64(&[0x80, 0x00], 0),
            Err(VarintError::NonCanonical { offset: 0 })
        );
        // Overlong for unsigned 1 (ZigZag -1): 0x81 0x00 instead of 0x01.
        assert_eq!(
            decode_i64(&[0x81, 0x00], 0),
            Err(VarintError::NonCanonical { offset: 0 })
        );
    }

    #[test]
    fn signed_encode_into_matches() {
        let mut out = Vec::new();
        let n = encode_i64_into(-2, &mut out);
        assert_eq!(n, out.len());
        assert_eq!(out, encode_i64(-2));
    }
}

//! Provisional **format v6** CRC32 (IEEE / ISO-HDLC) for header + chunk payload.
//!
//! Schema: `docs/schemas/v6-crc-provisional-v0.md`
//!
//! CRC32 polynomial `0xEDB88320` (reflected), init/xorout `0xFFFFFFFF`.
//! Header CRC covers fixed-header bytes `[0, HEADER_CRC)` (excludes CRC field).
//! Chunk payload CRC covers payload bytes only (not the 40-byte chunk header).
//! Optional verify is fail-closed on mismatch; default parse still does not verify.
//! Not a permanent algorithm freeze; not COL-007 C writer.

use crate::chunk::ChunkFrame;
use crate::offsets;
use crate::HEADER_LEN_FULL;

/// IEEE / ISO-HDLC reflected CRC-32 polynomial.
pub const CRC32_IEEE_POLY: u32 = 0xEDB_88320;

/// Fail-closed CRC errors.
#[derive(Debug, PartialEq, Eq)]
pub enum CrcError {
    Truncated {
        need: usize,
        got: usize,
    },
    /// Stored CRC does not match recomputed value.
    Mismatch {
        expected: u32,
        got: u32,
    },
    /// Header too short for CRC field / covered range.
    HeaderTooShort {
        len: usize,
    },
}

impl std::fmt::Display for CrcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrcError::Truncated { need, got } => {
                write!(f, "truncated crc input: need {need} bytes, got {got}")
            }
            CrcError::Mismatch { expected, got } => {
                write!(
                    f,
                    "crc mismatch: expected 0x{expected:08X}, got 0x{got:08X}"
                )
            }
            CrcError::HeaderTooShort { len } => {
                write!(
                    f,
                    "header too short for crc ({len} bytes; need {HEADER_LEN_FULL})"
                )
            }
        }
    }
}

impl std::error::Error for CrcError {}

pub type CrcResult<T> = std::result::Result<T, CrcError>;

/// Compute CRC-32/IEEE (ISO-HDLC) over `data`.
///
/// Pure function over a byte slice. Empty input → `0` (after init/xorout).
pub fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ CRC32_IEEE_POLY;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Byte range of the full fixed header covered by `header_crc` (excludes CRC field).
///
/// Provisional rule: bytes `[0, offsets::HEADER_CRC)` = first 32 bytes of a 36-byte
/// full header (`magic … optional_features`).
pub const HEADER_CRC_COVERED_LEN: usize = offsets::HEADER_CRC; // 32

/// Compute header CRC over the covered prefix of a full fixed header buffer.
///
/// Requires at least `HEADER_CRC_COVERED_LEN` bytes; uses only that prefix.
pub fn compute_header_crc(header: &[u8]) -> CrcResult<u32> {
    if header.len() < HEADER_CRC_COVERED_LEN {
        return Err(CrcError::HeaderTooShort { len: header.len() });
    }
    Ok(crc32_ieee(&header[..HEADER_CRC_COVERED_LEN]))
}

/// Write computed header CRC into `header[HEADER_CRC..HEADER_CRC+4]` (LE).
///
/// `header` must be a full 36-byte fixed header. Other fields are left unchanged.
pub fn fill_header_crc(header: &mut [u8]) -> CrcResult<u32> {
    if header.len() < HEADER_LEN_FULL as usize {
        return Err(CrcError::HeaderTooShort { len: header.len() });
    }
    let c = compute_header_crc(header)?;
    header[offsets::HEADER_CRC..offsets::HEADER_CRC + 4].copy_from_slice(&c.to_le_bytes());
    Ok(c)
}

/// Verify `header_crc` field against recomputed CRC over covered prefix.
///
/// Fail-closed on too-short header or mismatch. Does **not** re-check magic/major.
pub fn verify_header_crc(header: &[u8]) -> CrcResult<()> {
    if header.len() < HEADER_LEN_FULL as usize {
        return Err(CrcError::Truncated {
            need: HEADER_LEN_FULL as usize,
            got: header.len(),
        });
    }
    let stored = u32::from_le_bytes(
        header[offsets::HEADER_CRC..offsets::HEADER_CRC + 4]
            .try_into()
            .expect("4 bytes"),
    );
    let got = compute_header_crc(header)?;
    if stored != got {
        return Err(CrcError::Mismatch {
            expected: stored,
            got,
        });
    }
    Ok(())
}

/// Compute chunk payload CRC over payload bytes only.
pub fn compute_payload_crc(payload: &[u8]) -> u32 {
    crc32_ieee(payload)
}

/// Verify a stored payload checksum against `payload` bytes.
pub fn verify_payload_crc(payload: &[u8], expected: u32) -> CrcResult<()> {
    let got = compute_payload_crc(payload);
    if got != expected {
        return Err(CrcError::Mismatch { expected, got });
    }
    Ok(())
}

/// Verify `frame.payload_checksum` against `frame.payload`.
pub fn verify_chunk_payload_crc(frame: &ChunkFrame<'_>) -> CrcResult<()> {
    verify_payload_crc(frame.payload, frame.payload_checksum)
}

/// Encode a full fixed header and seal `header_crc` with CRC32 over the covered range.
///
/// The `header_crc` argument is ignored (overwritten by the sealed value).
pub fn encode_fixed_header_full_sealed(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
) -> [u8; HEADER_LEN_FULL as usize] {
    let mut h =
        crate::encode_fixed_header_full(major, minor, required_features, optional_features, 0);
    // Covered range is always present in full header; fill cannot fail.
    let _ = fill_header_crc(&mut h);
    h
}

/// Encode a chunk frame with `payload_checksum` sealed via CRC32 over the payload.
pub fn encode_chunk_frame_sealed(
    kind: u8,
    codec: u8,
    flags: u16,
    sequence: u64,
    first_logical_seq: u64,
    logical_event_count: u32,
    uncompressed_len: u32,
    payload: &[u8],
) -> Vec<u8> {
    let checksum = compute_payload_crc(payload);
    crate::chunk::encode_chunk_frame(
        kind,
        codec,
        flags,
        sequence,
        first_logical_seq,
        logical_event_count,
        uncompressed_len,
        payload,
        checksum,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{codec, kind, parse_chunk_frame};
    use crate::{encode_fixed_header_full, parse_fixed_header, MAGIC, SUPPORTED_MAJOR};

    #[test]
    fn crc32_ieee_empty_and_standard_vector() {
        // ISO-HDLC / zlib check value for "123456789".
        assert_eq!(crc32_ieee(b""), 0);
        assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
        // Dual compute stability.
        assert_eq!(crc32_ieee(b"abc"), crc32_ieee(b"abc"));
    }

    #[test]
    fn header_fill_and_verify_roundtrip() {
        let mut hdr = encode_fixed_header_full(SUPPORTED_MAJOR, 1, 0x11, 0x22, 0);
        assert_eq!(&hdr[..8], MAGIC.as_slice());
        let c = fill_header_crc(&mut hdr).expect("fill");
        assert_eq!(c, compute_header_crc(&hdr).unwrap());
        verify_header_crc(&hdr).expect("verify");
        // Parse still succeeds; CRC field is the sealed value.
        let h = parse_fixed_header(&hdr).expect("parse");
        assert_eq!(h.header_crc, Some(c));
        // Dual seal stability.
        let sealed_a = encode_fixed_header_full_sealed(SUPPORTED_MAJOR, 1, 0x11, 0x22);
        let sealed_b = encode_fixed_header_full_sealed(SUPPORTED_MAJOR, 1, 0x11, 0x22);
        assert_eq!(sealed_a, sealed_b);
        verify_header_crc(&sealed_a).unwrap();
    }

    #[test]
    fn header_mismatch_fail_closed() {
        let mut hdr = encode_fixed_header_full_sealed(SUPPORTED_MAJOR, 0, 0, 0);
        verify_header_crc(&hdr).unwrap();
        hdr[offsets::HEADER_CRC] ^= 0x01;
        match verify_header_crc(&hdr) {
            Err(CrcError::Mismatch { .. }) => {}
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    #[test]
    fn header_too_short_err() {
        assert!(matches!(
            verify_header_crc(&[0u8; 20]),
            Err(CrcError::Truncated { .. })
        ));
        assert!(matches!(
            compute_header_crc(&[0u8; 8]),
            Err(CrcError::HeaderTooShort { .. })
        ));
    }

    #[test]
    fn payload_crc_and_chunk_composition() {
        let payload = b"opaque-event-body";
        let frame = encode_chunk_frame_sealed(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            0,
            payload.len() as u32,
            payload,
        );
        let parsed = parse_chunk_frame(&frame).expect("parse");
        assert_eq!(parsed.payload, payload);
        assert_eq!(parsed.payload_checksum, compute_payload_crc(payload));
        verify_chunk_payload_crc(&parsed).expect("payload crc");
        // Dual seal stability.
        let f2 = encode_chunk_frame_sealed(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            0,
            payload.len() as u32,
            payload,
        );
        assert_eq!(frame, f2);
    }

    #[test]
    fn payload_mismatch_fail_closed() {
        let payload = b"data";
        assert!(verify_payload_crc(payload, compute_payload_crc(payload)).is_ok());
        match verify_payload_crc(payload, 0xDEAD_BEEF) {
            Err(CrcError::Mismatch { expected, got }) => {
                assert_eq!(expected, 0xDEAD_BEEF);
                assert_eq!(got, compute_payload_crc(payload));
            }
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    #[test]
    fn never_panic_on_empty() {
        assert_eq!(crc32_ieee(&[]), 0);
        assert!(verify_payload_crc(&[], 0).is_ok());
        assert!(verify_header_crc(&[]).is_err());
    }
}

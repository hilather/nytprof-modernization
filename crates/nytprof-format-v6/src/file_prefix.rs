//! Provisional **format v6** file-prefix composition (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-file-prefix-provisional-v0.md`
//!
//! Composes shipped [`crate::parse_fixed_header`] / [`crate::encode_fixed_header_full`]
//! with [`crate::tlv::encode_tlv_region`] / [`crate::tlv::decode_tlv_region`].

use crate::tlv::{decode_tlv_region, encode_tlv_region, Tlv, TlvError};
use crate::{
    encode_fixed_header_full, parse_fixed_header, Error as HeaderError, FixedHeader,
};

/// Fail-closed file-prefix errors (compose header + multi-TLV region).
#[derive(Debug, PartialEq, Eq)]
pub enum FilePrefixError {
    Header(HeaderError),
    Tlv(TlvError),
}

impl std::fmt::Display for FilePrefixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilePrefixError::Header(e) => write!(f, "v6 file prefix header: {e}"),
            FilePrefixError::Tlv(e) => write!(f, "v6 file prefix tlv region: {e}"),
        }
    }
}

impl std::error::Error for FilePrefixError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FilePrefixError::Header(e) => Some(e),
            FilePrefixError::Tlv(e) => Some(e),
        }
    }
}

impl From<HeaderError> for FilePrefixError {
    fn from(e: HeaderError) -> Self {
        FilePrefixError::Header(e)
    }
}

impl From<TlvError> for FilePrefixError {
    fn from(e: TlvError) -> Self {
        FilePrefixError::Tlv(e)
    }
}

pub type FilePrefixResult<T> = std::result::Result<T, FilePrefixError>;

/// Parsed provisional v6 file prefix (fixed header + payload TLVs, END excluded).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePrefix<'a> {
    pub header: FixedHeader,
    /// Payload TLVs only (END terminator consumed, not listed).
    pub tlvs: Vec<Tlv<'a>>,
}

/// Encode a provisional v6 file prefix: full fixed header then multi-TLV region.
///
/// Uses `encode_fixed_header_full` (36-byte header; `header_len = 36`). TLV region
/// starts immediately after the fixed header at offset `header_len`.
///
/// `tlv_items` are payload TLVs only (see `encode_tlv_region`); END is appended.
pub fn encode_file_prefix(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
) -> Vec<u8> {
    let mut out = encode_fixed_header_full(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
    )
    .to_vec();
    out.extend_from_slice(&encode_tlv_region(tlv_items));
    out
}

/// Decode a provisional v6 file prefix from `buf` starting at offset 0.
///
/// 1. Parse fixed header (`parse_fixed_header`).
/// 2. Decode multi-TLV region starting at `header.header_len` (provisional rule:
///    TLV region begins immediately after the declared fixed-header length).
///
/// Returns `(FilePrefix, total_bytes_consumed)` including the END terminator.
pub fn decode_file_prefix(buf: &[u8]) -> FilePrefixResult<(FilePrefix<'_>, usize)> {
    let header = parse_fixed_header(buf)?;
    let tlv_start = header.header_len as usize;
    if tlv_start > buf.len() {
        return Err(FilePrefixError::Header(HeaderError::Truncated {
            need: tlv_start,
            got: buf.len(),
        }));
    }
    let (tlvs, tlv_n) = decode_tlv_region(buf, tlv_start)?;
    let total = tlv_start
        .checked_add(tlv_n)
        .ok_or(FilePrefixError::Header(HeaderError::Truncated {
            need: tlv_start,
            got: buf.len(),
        }))?;
    Ok((FilePrefix { header, tlvs }, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tlv::{type_id, TlvError};
    use crate::{MAGIC, SUPPORTED_MAJOR, HEADER_LEN_FULL};

    #[test]
    fn roundtrip_empty_tlv_region() {
        let enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let (fp, n) = decode_file_prefix(&enc).expect("empty region");
        assert_eq!(n, enc.len());
        assert_eq!(fp.header.major, SUPPORTED_MAJOR);
        assert_eq!(fp.header.header_len, HEADER_LEN_FULL);
        assert!(fp.tlvs.is_empty());
        assert_eq!(&enc[..8], MAGIC.as_slice());
    }

    #[test]
    fn roundtrip_with_payload_tlvs() {
        let items: [(u64, u8, &[u8]); 2] = [
            (type_id::PRODUCER, 0, b"nytprof-rust"),
            (type_id::TICKS_PER_SEC, 0, b"10000000"),
        ];
        let enc = encode_file_prefix(SUPPORTED_MAJOR, 1, 1, 2, 0xABCD, &items);
        let (fp, n) = decode_file_prefix(&enc).expect("with tlvs");
        assert_eq!(n, enc.len());
        assert_eq!(fp.header.minor, 1);
        assert_eq!(fp.header.required_features, Some(1));
        assert_eq!(fp.header.optional_features, Some(2));
        assert_eq!(fp.header.header_crc, Some(0xABCD));
        assert_eq!(fp.tlvs.len(), 2);
        assert_eq!(fp.tlvs[0].type_id, type_id::PRODUCER);
        assert_eq!(fp.tlvs[0].value, b"nytprof-rust");
        assert_eq!(fp.tlvs[1].type_id, type_id::TICKS_PER_SEC);
        assert_eq!(fp.tlvs[1].value, b"10000000");
    }

    #[test]
    fn bad_magic_err() {
        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        enc[0] = b'X';
        assert_eq!(
            decode_file_prefix(&enc),
            Err(FilePrefixError::Header(HeaderError::BadMagic))
        );
    }

    #[test]
    fn truncated_after_header_before_end_err() {
        // Full header only — no END terminator.
        let enc = encode_fixed_header_full(SUPPORTED_MAJOR, 0, 0, 0, 0).to_vec();
        match decode_file_prefix(&enc) {
            Err(FilePrefixError::Tlv(TlvError::MissingTerminator)) => {}
            other => panic!("expected MissingTerminator, got {other:?}"),
        }
    }

    #[test]
    fn truncated_mid_tlv_err() {
        let mut enc = encode_file_prefix(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[(type_id::PRODUCER, 0, b"abcd")],
        );
        // Chop trailing terminator + part of producer.
        enc.truncate(HEADER_LEN_FULL as usize + 6);
        match decode_file_prefix(&enc) {
            Err(FilePrefixError::Tlv(_)) => {}
            other => panic!("expected Tlv error, got {other:?}"),
        }
    }

    #[test]
    fn trailing_bytes_after_prefix_ok() {
        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        enc.push(0xCC);
        let (fp, n) = decode_file_prefix(&enc).unwrap();
        assert!(fp.tlvs.is_empty());
        assert_eq!(enc[n], 0xCC);
    }
}

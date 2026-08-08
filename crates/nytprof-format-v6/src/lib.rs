//! Provisional **format v6** fixed-header + chunk-frame + ULEB128 + ZigZag signed
//! + length-prefixed string/blob + header TLV + file-prefix composition (COL-007 runway).
//!
//! Schemas:
//! - `docs/schemas/v6-fixed-header-provisional-v0.md`
//! - `docs/schemas/v6-chunk-frame-provisional-v0.md`
//! - `docs/schemas/v6-varint-uleb128-provisional-v0.md`
//! - `docs/schemas/v6-svarint-zigzag-provisional-v0.md`
//! - `docs/schemas/v6-string-blob-provisional-v0.md`
//! - `docs/schemas/v6-header-tlv-provisional-v0.md`
//! - `docs/schemas/v6-tlv-region-provisional-v0.md`
//! - `docs/schemas/v6-file-prefix-provisional-v0.md`
//!
//! This is **not** a wire freeze and does **not** implement the C v6 writer
//! (COL-007), payload codecs, event streams, or dictionaries. Layout may change under ADR.

pub mod chunk;
pub mod file_prefix;
pub mod string;
pub mod tlv;
pub mod varint;

pub use chunk::{
    parse_chunk_frame, ChunkError, ChunkFrame, ChunkResult, CHUNK_HEADER_LEN, CHUNK_SYNC,
    FLAG_KIND_REQUIRED, MAX_CHUNK_PAYLOAD,
};
pub use file_prefix::{
    decode_file_prefix, encode_file_prefix, FilePrefix, FilePrefixError, FilePrefixResult,
};
pub use string::{
    decode_string_blob, encode_string_blob, StringBlob, StringError, StringResult, FLAG_UTF8,
    MAX_STRING_BYTES,
};
pub use tlv::{
    decode_tlv, decode_tlv_region, encode_tlv, encode_tlv_region, is_known_type, Tlv, TlvError,
    TlvResult, FLAG_TYPE_REQUIRED, MAX_TLV_REGION_BYTES, MAX_TLV_VALUE_BYTES,
};
pub use varint::{
    decode_i64, decode_u64, decode_u64_permissive, encode_i64, encode_i64_into, encode_u64,
    encode_u64_into, zigzag_decode_i64, zigzag_encode_i64, VarintError, VarintResult,
    MAX_ULEB128_BYTES,
};

use thiserror::Error;

/// Provisional 8-byte magic: ASCII `NYTPROF6`.
pub const MAGIC: &[u8; 8] = b"NYTPROF6";

/// Only major accepted by this provisional MVP.
pub const SUPPORTED_MAJOR: u16 = 6;

/// Minimum bytes needed to read magic + major + minor + header_len.
pub const HEADER_LEN_MIN: usize = 16;

/// Provisional full fixed-header length (magic…header_crc), when present.
pub const HEADER_LEN_FULL: u32 = 36;

/// Fail-closed upper bound on declared `header_len` (1 MiB).
pub const MAX_HEADER_LEN: u32 = 1024 * 1024;

/// Offsets into the fixed header (little-endian multi-byte fields).
pub mod offsets {
    pub const MAGIC: usize = 0;
    pub const MAJOR: usize = 8;
    pub const MINOR: usize = 10;
    pub const HEADER_LEN: usize = 12;
    pub const REQUIRED_FEATURES: usize = 16;
    pub const OPTIONAL_FEATURES: usize = 24;
    pub const HEADER_CRC: usize = 32;
}

/// Parsed provisional fixed header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedHeader {
    pub major: u16,
    pub minor: u16,
    /// Declared total fixed-header length in bytes (includes this field).
    pub header_len: u32,
    /// Present when `header_len` covers the provisional full layout (≥ 36).
    pub required_features: Option<u64>,
    pub optional_features: Option<u64>,
    /// CRC placeholder — **not** validated by this MVP.
    pub header_crc: Option<u32>,
}

/// Fail-closed header parse errors (never panic on crafted input).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("truncated v6 header: need at least {need} bytes, got {got}")]
    Truncated { need: usize, got: usize },

    #[error("bad v6 magic (expected NYTPROF6)")]
    BadMagic,

    #[error("unsupported v6 major {major} (only {SUPPORTED_MAJOR} accepted provisionally)")]
    UnsupportedMajor { major: u16 },

    #[error("invalid v6 header_len {header_len} (min {HEADER_LEN_MIN}, max {MAX_HEADER_LEN})")]
    InvalidHeaderLen { header_len: u32 },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Parse a provisional v6 fixed header from `buf`.
///
/// Fail-closed on truncated input, bad magic, unsupported major, or invalid
/// `header_len`. Does **not** validate the header CRC placeholder.
///
/// Pure byte-slice API — no I/O.
pub fn parse_fixed_header(buf: &[u8]) -> Result<FixedHeader> {
    if buf.len() < HEADER_LEN_MIN {
        return Err(Error::Truncated {
            need: HEADER_LEN_MIN,
            got: buf.len(),
        });
    }

    let magic = &buf[offsets::MAGIC..offsets::MAGIC + 8];
    if magic != MAGIC.as_slice() {
        return Err(Error::BadMagic);
    }

    let major = u16::from_le_bytes([buf[offsets::MAJOR], buf[offsets::MAJOR + 1]]);
    let minor = u16::from_le_bytes([buf[offsets::MINOR], buf[offsets::MINOR + 1]]);
    if major != SUPPORTED_MAJOR {
        return Err(Error::UnsupportedMajor { major });
    }

    let header_len = u32::from_le_bytes([
        buf[offsets::HEADER_LEN],
        buf[offsets::HEADER_LEN + 1],
        buf[offsets::HEADER_LEN + 2],
        buf[offsets::HEADER_LEN + 3],
    ]);

    if header_len < HEADER_LEN_MIN as u32 || header_len > MAX_HEADER_LEN {
        return Err(Error::InvalidHeaderLen { header_len });
    }

    let need = header_len as usize;
    if buf.len() < need {
        return Err(Error::Truncated {
            need,
            got: buf.len(),
        });
    }

    let mut required_features = None;
    let mut optional_features = None;
    let mut header_crc = None;

    if header_len >= HEADER_LEN_FULL {
        required_features = Some(u64::from_le_bytes(
            buf[offsets::REQUIRED_FEATURES..offsets::REQUIRED_FEATURES + 8]
                .try_into()
                .expect("slice len 8"),
        ));
        optional_features = Some(u64::from_le_bytes(
            buf[offsets::OPTIONAL_FEATURES..offsets::OPTIONAL_FEATURES + 8]
                .try_into()
                .expect("slice len 8"),
        ));
        header_crc = Some(u32::from_le_bytes(
            buf[offsets::HEADER_CRC..offsets::HEADER_CRC + 4]
                .try_into()
                .expect("slice len 4"),
        ));
    }

    Ok(FixedHeader {
        major,
        minor,
        header_len,
        required_features,
        optional_features,
        header_crc,
    })
}

/// Build a provisional full fixed header (36 bytes) for tests / future writers.
///
/// CRC is stored as provided; this MVP does not compute or check it.
pub fn encode_fixed_header_full(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
) -> [u8; HEADER_LEN_FULL as usize] {
    let mut out = [0u8; HEADER_LEN_FULL as usize];
    out[offsets::MAGIC..offsets::MAGIC + 8].copy_from_slice(MAGIC);
    out[offsets::MAJOR..offsets::MAJOR + 2].copy_from_slice(&major.to_le_bytes());
    out[offsets::MINOR..offsets::MINOR + 2].copy_from_slice(&minor.to_le_bytes());
    out[offsets::HEADER_LEN..offsets::HEADER_LEN + 4]
        .copy_from_slice(&HEADER_LEN_FULL.to_le_bytes());
    out[offsets::REQUIRED_FEATURES..offsets::REQUIRED_FEATURES + 8]
        .copy_from_slice(&required_features.to_le_bytes());
    out[offsets::OPTIONAL_FEATURES..offsets::OPTIONAL_FEATURES + 8]
        .copy_from_slice(&optional_features.to_le_bytes());
    out[offsets::HEADER_CRC..offsets::HEADER_CRC + 4].copy_from_slice(&header_crc.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_full_header_ok() {
        let bytes = encode_fixed_header_full(6, 0, 0, 0, 0xDEAD_BEEF);
        let h = parse_fixed_header(&bytes).expect("valid header");
        assert_eq!(h.major, 6);
        assert_eq!(h.minor, 0);
        assert_eq!(h.header_len, HEADER_LEN_FULL);
        assert_eq!(h.required_features, Some(0));
        assert_eq!(h.optional_features, Some(0));
        assert_eq!(h.header_crc, Some(0xDEAD_BEEF));
    }

    #[test]
    fn valid_header_with_extra_tail_ok() {
        let mut bytes = encode_fixed_header_full(6, 1, 1, 2, 3).to_vec();
        bytes.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        let h = parse_fixed_header(&bytes).expect("valid with tail");
        assert_eq!(h.major, 6);
        assert_eq!(h.minor, 1);
        assert_eq!(h.required_features, Some(1));
        assert_eq!(h.optional_features, Some(2));
        assert_eq!(h.header_crc, Some(3));
    }

    #[test]
    fn bad_magic_err() {
        let mut bytes = encode_fixed_header_full(6, 0, 0, 0, 0);
        bytes[0] = b'X';
        assert_eq!(parse_fixed_header(&bytes), Err(Error::BadMagic));
    }

    #[test]
    fn bad_magic_v5_text_prelude_err() {
        // v5 starts with text "NYTProf …" — must not parse as v6.
        let mut bytes = encode_fixed_header_full(6, 0, 0, 0, 0);
        bytes[..7].copy_from_slice(b"NYTProf");
        bytes[7] = b' ';
        assert_eq!(parse_fixed_header(&bytes), Err(Error::BadMagic));
    }

    #[test]
    fn truncated_before_min_err() {
        let bytes = encode_fixed_header_full(6, 0, 0, 0, 0);
        let got = parse_fixed_header(&bytes[..15]);
        assert_eq!(
            got,
            Err(Error::Truncated {
                need: HEADER_LEN_MIN,
                got: 15
            })
        );
    }

    #[test]
    fn truncated_after_len_field_err() {
        let mut bytes = encode_fixed_header_full(6, 0, 0, 0, 0).to_vec();
        // Declare full length but provide only 20 bytes.
        bytes.truncate(20);
        let got = parse_fixed_header(&bytes);
        assert_eq!(
            got,
            Err(Error::Truncated {
                need: HEADER_LEN_FULL as usize,
                got: 20
            })
        );
    }

    #[test]
    fn unsupported_major_err() {
        let bytes = encode_fixed_header_full(7, 0, 0, 0, 0);
        assert_eq!(
            parse_fixed_header(&bytes),
            Err(Error::UnsupportedMajor { major: 7 })
        );
    }

    #[test]
    fn invalid_header_len_too_small_err() {
        let mut bytes = encode_fixed_header_full(6, 0, 0, 0, 0);
        // header_len = 8 (< 16)
        bytes[offsets::HEADER_LEN..offsets::HEADER_LEN + 4]
            .copy_from_slice(&8u32.to_le_bytes());
        assert_eq!(
            parse_fixed_header(&bytes),
            Err(Error::InvalidHeaderLen { header_len: 8 })
        );
    }

    #[test]
    fn empty_buf_truncated() {
        assert_eq!(
            parse_fixed_header(&[]),
            Err(Error::Truncated {
                need: HEADER_LEN_MIN,
                got: 0
            })
        );
    }

    #[test]
    fn magic_constant_is_nytprof6() {
        assert_eq!(MAGIC, b"NYTPROF6");
        assert_eq!(MAGIC.len(), 8);
    }
}

//! Provisional **format v6** chunk-frame parse (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-chunk-frame-provisional-v0.md`
//!
//! Does **not** inflate payloads, decode events, or implement the C v6 writer.

use thiserror::Error;

/// Provisional chunk sync word: ASCII bytes `N Y T 6` as `u32 LE`.
pub const CHUNK_SYNC: u32 = u32::from_le_bytes(*b"NYT6");

/// Fixed chunk-header length in bytes.
pub const CHUNK_HEADER_LEN: usize = 40;

/// Fail-closed upper bound on declared compressed/uncompressed payload (64 MiB).
pub const MAX_CHUNK_PAYLOAD: u32 = 64 * 1024 * 1024;

/// Flag: unknown chunk kind must fail closed (required kind).
pub const FLAG_KIND_REQUIRED: u16 = 0x0001;

/// Provisional chunk kind values.
pub mod kind {
    pub const RESERVED: u8 = 0;
    pub const EVENT: u8 = 1;
    pub const SOURCE: u8 = 2;
    pub const INDEX: u8 = 3;
    pub const SUMMARY: u8 = 4;
    pub const FOOTER: u8 = 5;
}

/// Provisional codec ids.
///
/// Payload inflate/deflate helpers: `crate::payload_codec` (ZLIB/ZSTD/LZ4;
/// default parse stays non-inflating).
pub mod codec {
    pub const NONE: u8 = 0;
    pub const ZLIB: u8 = 1;
    pub const ZSTD: u8 = 2;
    pub const LZ4: u8 = 3;
}

mod offsets {
    pub const SYNC: usize = 0;
    pub const KIND: usize = 4;
    pub const CODEC: usize = 5;
    pub const FLAGS: usize = 6;
    pub const SEQUENCE: usize = 8;
    pub const FIRST_LOGICAL: usize = 16;
    pub const LOGICAL_COUNT: usize = 24;
    pub const UNCOMPRESSED_LEN: usize = 28;
    pub const COMPRESSED_LEN: usize = 32;
    pub const CHECKSUM: usize = 36;
}

/// Parsed provisional chunk frame (header fields + payload subslice).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkFrame<'a> {
    pub kind: u8,
    pub codec: u8,
    pub flags: u16,
    pub sequence: u64,
    pub first_logical_seq: u64,
    pub logical_event_count: u32,
    pub uncompressed_len: u32,
    pub compressed_len: u32,
    /// Placeholder checksum — **not** verified by this MVP.
    pub payload_checksum: u32,
    /// True when `kind` is one of the provisional known kinds (1..=5).
    pub known_kind: bool,
    /// Payload bytes (`compressed_len` long); borrowed from input.
    pub payload: &'a [u8],
}

/// Fail-closed chunk-frame parse errors (never panic on crafted input).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ChunkError {
    #[error("truncated v6 chunk frame: need at least {need} bytes, got {got}")]
    Truncated { need: usize, got: usize },

    #[error("bad v6 chunk sync (expected NYT6 / 0x{expected:08X}, got 0x{got:08X})")]
    BadSync { expected: u32, got: u32 },

    #[error("invalid v6 chunk kind 0 (reserved)")]
    InvalidKind,

    #[error("oversize v6 chunk compressed_len {len} (max {MAX_CHUNK_PAYLOAD})")]
    OversizeCompressed { len: u32 },

    #[error("oversize v6 chunk uncompressed_len {len} (max {MAX_CHUNK_PAYLOAD})")]
    OversizeUncompressed { len: u32 },

    #[error("unknown required v6 chunk kind {kind}")]
    UnknownRequiredKind { kind: u8 },
}

pub type ChunkResult<T> = std::result::Result<T, ChunkError>;

/// True if `kind` is a known provisional kind (EVENT..=FOOTER).
pub fn is_known_kind(kind: u8) -> bool {
    matches!(
        kind,
        kind::EVENT | kind::SOURCE | kind::INDEX | kind::SUMMARY | kind::FOOTER
    )
}

/// Parse one provisional v6 chunk frame from `buf` (header + payload).
///
/// Fail-closed on truncated input, bad sync, reserved/invalid kind 0, oversize
/// lengths, and unknown kind when `FLAG_KIND_REQUIRED` is set. Does **not**
/// verify the payload checksum or inflate the body.
///
/// Pure byte-slice API — no I/O.
pub fn parse_chunk_frame(buf: &[u8]) -> ChunkResult<ChunkFrame<'_>> {
    if buf.len() < CHUNK_HEADER_LEN {
        return Err(ChunkError::Truncated {
            need: CHUNK_HEADER_LEN,
            got: buf.len(),
        });
    }

    let sync = u32::from_le_bytes(
        buf[offsets::SYNC..offsets::SYNC + 4]
            .try_into()
            .expect("4 bytes"),
    );
    if sync != CHUNK_SYNC {
        return Err(ChunkError::BadSync {
            expected: CHUNK_SYNC,
            got: sync,
        });
    }

    let kind = buf[offsets::KIND];
    let codec = buf[offsets::CODEC];
    let flags = u16::from_le_bytes(
        buf[offsets::FLAGS..offsets::FLAGS + 2]
            .try_into()
            .expect("2 bytes"),
    );
    let sequence = u64::from_le_bytes(
        buf[offsets::SEQUENCE..offsets::SEQUENCE + 8]
            .try_into()
            .expect("8 bytes"),
    );
    let first_logical_seq = u64::from_le_bytes(
        buf[offsets::FIRST_LOGICAL..offsets::FIRST_LOGICAL + 8]
            .try_into()
            .expect("8 bytes"),
    );
    let logical_event_count = u32::from_le_bytes(
        buf[offsets::LOGICAL_COUNT..offsets::LOGICAL_COUNT + 4]
            .try_into()
            .expect("4 bytes"),
    );
    let uncompressed_len = u32::from_le_bytes(
        buf[offsets::UNCOMPRESSED_LEN..offsets::UNCOMPRESSED_LEN + 4]
            .try_into()
            .expect("4 bytes"),
    );
    let compressed_len = u32::from_le_bytes(
        buf[offsets::COMPRESSED_LEN..offsets::COMPRESSED_LEN + 4]
            .try_into()
            .expect("4 bytes"),
    );
    let payload_checksum = u32::from_le_bytes(
        buf[offsets::CHECKSUM..offsets::CHECKSUM + 4]
            .try_into()
            .expect("4 bytes"),
    );

    if kind == kind::RESERVED {
        return Err(ChunkError::InvalidKind);
    }

    if compressed_len > MAX_CHUNK_PAYLOAD {
        return Err(ChunkError::OversizeCompressed {
            len: compressed_len,
        });
    }
    if uncompressed_len > MAX_CHUNK_PAYLOAD {
        return Err(ChunkError::OversizeUncompressed {
            len: uncompressed_len,
        });
    }

    let known = is_known_kind(kind);
    if !known && (flags & FLAG_KIND_REQUIRED) != 0 {
        return Err(ChunkError::UnknownRequiredKind { kind });
    }

    let need = CHUNK_HEADER_LEN
        .checked_add(compressed_len as usize)
        .ok_or(ChunkError::OversizeCompressed {
            len: compressed_len,
        })?;
    if buf.len() < need {
        return Err(ChunkError::Truncated {
            need,
            got: buf.len(),
        });
    }

    let payload = &buf[CHUNK_HEADER_LEN..need];

    Ok(ChunkFrame {
        kind,
        codec,
        flags,
        sequence,
        first_logical_seq,
        logical_event_count,
        uncompressed_len,
        compressed_len,
        payload_checksum,
        known_kind: known,
        payload,
    })
}

/// Encode a provisional chunk frame (header + payload) into a new `Vec<u8>`.
///
/// For tests and future writers. Does not compute a real checksum.
pub fn encode_chunk_frame(
    kind: u8,
    codec: u8,
    flags: u16,
    sequence: u64,
    first_logical_seq: u64,
    logical_event_count: u32,
    uncompressed_len: u32,
    payload: &[u8],
    payload_checksum: u32,
) -> Vec<u8> {
    let compressed_len = payload.len() as u32;
    let mut out = Vec::with_capacity(CHUNK_HEADER_LEN + payload.len());
    out.extend_from_slice(&CHUNK_SYNC.to_le_bytes());
    out.push(kind);
    out.push(codec);
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&sequence.to_le_bytes());
    out.extend_from_slice(&first_logical_seq.to_le_bytes());
    out.extend_from_slice(&logical_event_count.to_le_bytes());
    out.extend_from_slice(&uncompressed_len.to_le_bytes());
    out.extend_from_slice(&compressed_len.to_le_bytes());
    out.extend_from_slice(&payload_checksum.to_le_bytes());
    out.extend_from_slice(payload);
    debug_assert_eq!(out.len(), CHUNK_HEADER_LEN + payload.len());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_event_frame() -> Vec<u8> {
        encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            0,
            0,
            &[],
            0,
        )
    }

    #[test]
    fn valid_empty_payload_ok() {
        let bytes = minimal_event_frame();
        let f = parse_chunk_frame(&bytes).expect("valid frame");
        assert_eq!(f.kind, kind::EVENT);
        assert_eq!(f.codec, codec::NONE);
        assert_eq!(f.compressed_len, 0);
        assert_eq!(f.payload, &[][..]);
        assert!(f.known_kind);
    }

    #[test]
    fn valid_with_payload_ok() {
        let payload = b"hello-v6";
        let bytes = encode_chunk_frame(
            kind::SOURCE,
            codec::NONE,
            0,
            7,
            3,
            1,
            payload.len() as u32,
            payload,
            0xA5A5_A5A5,
        );
        let f = parse_chunk_frame(&bytes).expect("valid");
        assert_eq!(f.kind, kind::SOURCE);
        assert_eq!(f.sequence, 7);
        assert_eq!(f.first_logical_seq, 3);
        assert_eq!(f.logical_event_count, 1);
        assert_eq!(f.uncompressed_len, payload.len() as u32);
        assert_eq!(f.compressed_len, payload.len() as u32);
        assert_eq!(f.payload, payload);
        assert_eq!(f.payload_checksum, 0xA5A5_A5A5);
    }

    #[test]
    fn bad_sync_err() {
        let mut bytes = minimal_event_frame();
        bytes[0] = 0x00;
        match parse_chunk_frame(&bytes) {
            Err(ChunkError::BadSync { expected, got }) => {
                assert_eq!(expected, CHUNK_SYNC);
                assert_ne!(got, CHUNK_SYNC);
            }
            other => panic!("expected BadSync, got {other:?}"),
        }
    }

    #[test]
    fn truncated_header_err() {
        let bytes = minimal_event_frame();
        assert_eq!(
            parse_chunk_frame(&bytes[..CHUNK_HEADER_LEN - 1]),
            Err(ChunkError::Truncated {
                need: CHUNK_HEADER_LEN,
                got: CHUNK_HEADER_LEN - 1
            })
        );
    }

    #[test]
    fn truncated_payload_err() {
        let mut bytes = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            0,
            4,
            b"abcd",
            0,
        );
        // Claim compressed_len=4 but drop payload bytes.
        bytes.truncate(CHUNK_HEADER_LEN + 2);
        // Fix compressed_len field to still say 4.
        bytes[offsets::COMPRESSED_LEN..offsets::COMPRESSED_LEN + 4]
            .copy_from_slice(&4u32.to_le_bytes());
        assert_eq!(
            parse_chunk_frame(&bytes),
            Err(ChunkError::Truncated {
                need: CHUNK_HEADER_LEN + 4,
                got: CHUNK_HEADER_LEN + 2
            })
        );
    }

    #[test]
    fn oversize_compressed_err() {
        let mut bytes = minimal_event_frame();
        let too_big = MAX_CHUNK_PAYLOAD + 1;
        bytes[offsets::COMPRESSED_LEN..offsets::COMPRESSED_LEN + 4]
            .copy_from_slice(&too_big.to_le_bytes());
        assert_eq!(
            parse_chunk_frame(&bytes),
            Err(ChunkError::OversizeCompressed { len: too_big })
        );
    }

    #[test]
    fn oversize_uncompressed_err() {
        let mut bytes = minimal_event_frame();
        let too_big = MAX_CHUNK_PAYLOAD + 1;
        bytes[offsets::UNCOMPRESSED_LEN..offsets::UNCOMPRESSED_LEN + 4]
            .copy_from_slice(&too_big.to_le_bytes());
        assert_eq!(
            parse_chunk_frame(&bytes),
            Err(ChunkError::OversizeUncompressed { len: too_big })
        );
    }

    #[test]
    fn unknown_required_kind_err() {
        let bytes = encode_chunk_frame(
            0x42, // unknown
            codec::NONE,
            FLAG_KIND_REQUIRED,
            0,
            0,
            0,
            0,
            &[],
            0,
        );
        assert_eq!(
            parse_chunk_frame(&bytes),
            Err(ChunkError::UnknownRequiredKind { kind: 0x42 })
        );
    }

    #[test]
    fn unknown_optional_kind_ok_not_known() {
        let bytes = encode_chunk_frame(
            0x42,
            codec::NONE,
            0, // not required
            1,
            0,
            0,
            0,
            &[],
            0,
        );
        let f = parse_chunk_frame(&bytes).expect("optional unknown kind ok");
        assert!(!f.known_kind);
        assert_eq!(f.kind, 0x42);
    }

    #[test]
    fn reserved_kind_zero_err() {
        let bytes = encode_chunk_frame(0, codec::NONE, 0, 0, 0, 0, 0, &[], 0);
        assert_eq!(parse_chunk_frame(&bytes), Err(ChunkError::InvalidKind));
    }

    #[test]
    fn sync_constant_is_nyt6_le() {
        assert_eq!(CHUNK_SYNC.to_le_bytes(), *b"NYT6");
    }

    #[test]
    fn empty_buf_truncated() {
        assert_eq!(
            parse_chunk_frame(&[]),
            Err(ChunkError::Truncated {
                need: CHUNK_HEADER_LEN,
                got: 0
            })
        );
    }
}

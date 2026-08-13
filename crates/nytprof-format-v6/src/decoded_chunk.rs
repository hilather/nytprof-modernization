//! Provisional **format v6** decoded-chunk consumer path (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-decoded-chunk-provisional-v0.md`
//!
//! Always-inflate consumer: wire-level `parse_chunk_frame` (non-inflating) →
//! optional payload CRC verify → `decode_chunk_payload` for plain body bytes.
//! Does **not** change default `parse_chunk_frame` semantics.
//! Not dictionaries, not COL-007 C writer, not CLI v6 default.

use crate::chunk::{parse_chunk_frame, ChunkError, ChunkFrame, CHUNK_HEADER_LEN};
use crate::crc::{verify_chunk_payload_crc, CrcError};
use crate::payload_codec::{decode_chunk_payload, PayloadCodecError};

/// Fail-closed decoded-chunk errors.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodedChunkError {
    Chunk(ChunkError),
    Crc(CrcError),
    Payload(PayloadCodecError),
}

impl std::fmt::Display for DecodedChunkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodedChunkError::Chunk(e) => write!(f, "decoded-chunk frame: {e}"),
            DecodedChunkError::Crc(e) => write!(f, "decoded-chunk crc: {e}"),
            DecodedChunkError::Payload(e) => write!(f, "decoded-chunk payload: {e}"),
        }
    }
}

impl std::error::Error for DecodedChunkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodedChunkError::Chunk(e) => Some(e),
            DecodedChunkError::Crc(e) => Some(e),
            DecodedChunkError::Payload(e) => Some(e),
        }
    }
}

impl From<ChunkError> for DecodedChunkError {
    fn from(e: ChunkError) -> Self {
        DecodedChunkError::Chunk(e)
    }
}

impl From<CrcError> for DecodedChunkError {
    fn from(e: CrcError) -> Self {
        DecodedChunkError::Crc(e)
    }
}

impl From<PayloadCodecError> for DecodedChunkError {
    fn from(e: PayloadCodecError) -> Self {
        DecodedChunkError::Payload(e)
    }
}

pub type DecodedChunkResult<T> = std::result::Result<T, DecodedChunkError>;

/// One decoded chunk: header metadata + always-inflated plain body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedChunk {
    pub kind: u8,
    pub codec: u8,
    pub flags: u16,
    pub sequence: u64,
    pub first_logical_seq: u64,
    pub logical_event_count: u32,
    pub uncompressed_len: u32,
    pub compressed_len: u32,
    pub payload_checksum: u32,
    pub known_kind: bool,
    /// Plain body after identity (NONE) or inflate (ZLIB/ZSTD/LZ4).
    pub plain: Vec<u8>,
}

impl DecodedChunk {
    fn from_frame(frame: &ChunkFrame<'_>, plain: Vec<u8>) -> Self {
        DecodedChunk {
            kind: frame.kind,
            codec: frame.codec,
            flags: frame.flags,
            sequence: frame.sequence,
            first_logical_seq: frame.first_logical_seq,
            logical_event_count: frame.logical_event_count,
            uncompressed_len: frame.uncompressed_len,
            compressed_len: frame.compressed_len,
            payload_checksum: frame.payload_checksum,
            known_kind: frame.known_kind,
            plain,
        }
    }
}

/// Decode plain body from an already-parsed frame (always inflate).
///
/// When `verify_crc` is true, verifies on-wire payload CRC **before** inflate.
/// Default `parse_chunk_frame` remains non-inflating / non-CRC; this is the
/// consumer path that always recovers plain bytes.
pub fn decode_chunk_frame_plain(
    frame: &ChunkFrame<'_>,
    verify_crc: bool,
) -> DecodedChunkResult<Vec<u8>> {
    if verify_crc {
        verify_chunk_payload_crc(frame)?;
    }
    Ok(decode_chunk_payload(frame)?)
}

/// Parse one sealed chunk from `buf`, optionally verify CRC, always inflate.
///
/// Returns `(DecodedChunk, bytes_consumed)` where `bytes_consumed` is
/// `CHUNK_HEADER_LEN + compressed_len` for the first frame in `buf`.
///
/// Pure byte-slice API — no I/O. Does not mutate default parse policy.
pub fn decode_chunk(buf: &[u8], verify_crc: bool) -> DecodedChunkResult<(DecodedChunk, usize)> {
    let frame = parse_chunk_frame(buf)?;
    let n = CHUNK_HEADER_LEN + frame.payload.len();
    let plain = decode_chunk_frame_plain(&frame, verify_crc)?;
    Ok((DecodedChunk::from_frame(&frame, plain), n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{codec, encode_chunk_frame, kind, CHUNK_SYNC};
    use crate::crc::{compute_payload_crc, encode_chunk_frame_sealed};
    use crate::payload_codec::{
        deflate_zlib, encode_chunk_frame_lz4, encode_chunk_frame_zlib, encode_chunk_frame_zstd,
    };

    const SAMPLE_PLAIN: &[u8] = b"decoded-chunk-plain-body-v6-preflight";

    fn seal_none(plain: &[u8]) -> Vec<u8> {
        encode_chunk_frame_sealed(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            1,
            plain.len() as u32,
            plain,
        )
    }

    #[test]
    fn none_zlib_zstd_lz4_roundtrip_plain_with_crc() {
        // NONE
        let wire_none = seal_none(SAMPLE_PLAIN);
        let (d0, n0) = decode_chunk(&wire_none, true).expect("none");
        assert_eq!(n0, wire_none.len());
        assert_eq!(d0.codec, codec::NONE);
        assert_eq!(d0.plain, SAMPLE_PLAIN);
        assert_eq!(d0.kind, kind::EVENT);

        // ZLIB / ZSTD / LZ4 via shipped encode helpers
        for (label, enc) in [
            (
                "zlib",
                encode_chunk_frame_zlib(kind::EVENT, 0, 0, 0, 1, SAMPLE_PLAIN).unwrap(),
            ),
            (
                "zstd",
                encode_chunk_frame_zstd(kind::EVENT, 0, 0, 0, 1, SAMPLE_PLAIN).unwrap(),
            ),
            (
                "lz4",
                encode_chunk_frame_lz4(kind::EVENT, 0, 0, 0, 1, SAMPLE_PLAIN).unwrap(),
            ),
        ] {
            let (d, n) = decode_chunk(&enc, true).unwrap_or_else(|e| panic!("{label}: {e}"));
            assert_eq!(n, enc.len(), "{label}");
            assert_eq!(d.plain, SAMPLE_PLAIN, "{label}");
            assert_eq!(d.uncompressed_len as usize, SAMPLE_PLAIN.len(), "{label}");
            // On-wire payload is not the plain body for compressed codecs.
            let frame = parse_chunk_frame(&enc).unwrap();
            assert_ne!(frame.payload, SAMPLE_PLAIN, "{label} wire != plain");
            // Frame-level API matches.
            let plain2 = decode_chunk_frame_plain(&frame, true).unwrap();
            assert_eq!(plain2, SAMPLE_PLAIN, "{label} frame api");
        }
    }

    #[test]
    fn none_without_crc_verify_still_inflates() {
        let wire = seal_none(SAMPLE_PLAIN);
        let (d, _) = decode_chunk(&wire, false).expect("no crc");
        assert_eq!(d.plain, SAMPLE_PLAIN);
    }

    #[test]
    fn corrupt_zlib_payload_err() {
        let mut enc =
            encode_chunk_frame_zlib(kind::SOURCE, 0, 1, 0, 1, SAMPLE_PLAIN).expect("encode");
        // Flip on-wire compressed bytes after header; re-seal CRC so we isolate inflate fail
        // when verify is off, and still fail when verify is on if we don't re-seal.
        let payload_off = CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        if enc.len() > payload_off + 1 {
            enc[payload_off + 1] ^= 0xAA;
        }
        // Without CRC verify: corrupt stream → payload error.
        match decode_chunk(&enc, false) {
            Err(DecodedChunkError::Payload(_)) => {}
            other => panic!("expected payload err without crc, got {other:?}"),
        }
        // With CRC verify: corrupted bytes should fail CRC first (checksum still original).
        match decode_chunk(&enc, true) {
            Err(DecodedChunkError::Crc(_)) => {}
            other => panic!("expected crc err with verify, got {other:?}"),
        }
    }

    #[test]
    fn crc_mismatch_when_verify_on_err() {
        let plain = SAMPLE_PLAIN;
        let compressed = deflate_zlib(plain).unwrap();
        // Wrong checksum on purpose.
        let wire = encode_chunk_frame(
            kind::EVENT,
            codec::ZLIB,
            0,
            0,
            0,
            1,
            plain.len() as u32,
            &compressed,
            compute_payload_crc(&compressed) ^ 0xDEAD_BEEF,
        );
        match decode_chunk(&wire, true) {
            Err(DecodedChunkError::Crc(CrcError::Mismatch { .. })) => {}
            other => panic!("expected crc mismatch, got {other:?}"),
        }
        // Same wire without verify still inflates successfully.
        let (d, n) = decode_chunk(&wire, false).expect("inflate without crc");
        assert_eq!(n, wire.len());
        assert_eq!(d.plain, plain);
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_chunk(&[], true).is_err());
        assert!(decode_chunk(b"nope", false).is_err());
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        match decode_chunk(&bad, true) {
            Err(DecodedChunkError::Chunk(ChunkError::BadSync { expected, got })) => {
                assert_eq!(expected, CHUNK_SYNC);
                assert_eq!(got, 0xDEAD_BEEF);
            }
            other => panic!("expected bad sync, got {other:?}"),
        }
    }

    #[test]
    fn parse_chunk_frame_stays_non_inflating() {
        // Structural honesty: default parse returns compressed wire bytes, not plain.
        let enc =
            encode_chunk_frame_zlib(kind::EVENT, 0, 0, 0, 1, SAMPLE_PLAIN).expect("encode zlib");
        let frame = parse_chunk_frame(&enc).expect("parse");
        assert_eq!(frame.codec, codec::ZLIB);
        assert_ne!(frame.payload, SAMPLE_PLAIN);
        // Consumer path recovers plain.
        let plain = decode_chunk_frame_plain(&frame, true).unwrap();
        assert_eq!(plain, SAMPLE_PLAIN);
    }
}

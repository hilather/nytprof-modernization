//! Provisional **format v6** decoded prefix+chunk stream consumer path
//! (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-decoded-stream-provisional-v0.md`
//!
//! Always-inflate multi-chunk consumer:
//! non-inflating `decode_prefix_chunk_stream` → per-chunk optional CRC +
//! `decode_chunk_payload` via shipped `decode_chunk_frame_plain`.
//! Does **not** change default `parse_chunk_frame` semantics.
//! Not dictionaries, not COL-007 C writer, not CLI v6 default.

use crate::decoded_chunk::{decode_chunk_frame_plain, DecodedChunk, DecodedChunkError};
use crate::file_prefix::encode_file_prefix;
use crate::stream::{decode_prefix_chunk_stream, StreamError};
use crate::FixedHeader;

/// Fail-closed decoded-stream errors.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodedStreamError {
    Stream(StreamError),
    Chunk(DecodedChunkError),
}

impl std::fmt::Display for DecodedStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodedStreamError::Stream(e) => write!(f, "decoded-stream: {e}"),
            DecodedStreamError::Chunk(e) => write!(f, "decoded-stream chunk: {e}"),
        }
    }
}

impl std::error::Error for DecodedStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodedStreamError::Stream(e) => Some(e),
            DecodedStreamError::Chunk(e) => Some(e),
        }
    }
}

impl From<StreamError> for DecodedStreamError {
    fn from(e: StreamError) -> Self {
        DecodedStreamError::Stream(e)
    }
}

impl From<DecodedChunkError> for DecodedStreamError {
    fn from(e: DecodedChunkError) -> Self {
        DecodedStreamError::Chunk(e)
    }
}

pub type DecodedStreamResult<T> = std::result::Result<T, DecodedStreamError>;

/// Decoded prefix+chunk stream: fixed header + ordered always-inflated chunks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedStream {
    pub header: FixedHeader,
    /// Number of TLVs in the file prefix (END not counted).
    pub tlv_count: usize,
    pub chunks: Vec<DecodedChunk>,
}

/// Decode a file-prefix + multi-chunk stream, always inflating each chunk.
///
/// 1. Shipped non-inflating `decode_prefix_chunk_stream`
/// 2. Per chunk: shipped `decode_chunk_frame_plain(..., verify_crc)`
///
/// Returns `(DecodedStream, bytes_consumed)` (`bytes_consumed == buf.len()` on success).
/// Default `parse_chunk_frame` remains non-inflating; this is the consumer path.
pub fn decode_prefix_chunk_stream_plain(
    buf: &[u8],
    verify_crc: bool,
) -> DecodedStreamResult<(DecodedStream, usize)> {
    let (stream, n) = decode_prefix_chunk_stream(buf)?;
    let mut chunks = Vec::with_capacity(stream.chunks.len());
    for frame in &stream.chunks {
        let plain = decode_chunk_frame_plain(frame, verify_crc)?;
        chunks.push(DecodedChunk {
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
        });
    }
    Ok((
        DecodedStream {
            header: stream.prefix.header,
            tlv_count: stream.prefix.tlvs.len(),
            chunks,
        },
        n,
    ))
}

/// Encode a provisional decoded-stream wire: file prefix + sealed chunk frames.
///
/// `sealed_chunk_frames` are already-encoded full chunk frames (header+payload),
/// typically from `encode_chunk_frame_sealed` / `encode_chunk_frame_zlib` / etc.
/// Pure composition — no reimplementation of codecs.
pub fn encode_prefix_sealed_chunks(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    sealed_chunk_frames: &[&[u8]],
) -> Vec<u8> {
    let mut out = encode_file_prefix(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
    );
    for frame in sealed_chunk_frames {
        out.extend_from_slice(frame);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{
        codec, encode_chunk_frame, kind, parse_chunk_frame, ChunkError, CHUNK_HEADER_LEN,
        CHUNK_SYNC,
    };
    use crate::crc::{compute_payload_crc, encode_chunk_frame_sealed};
    use crate::decoded_chunk::DecodedChunkError;
    use crate::payload_codec::{
        deflate_zlib, encode_chunk_frame_lz4, encode_chunk_frame_zlib, encode_chunk_frame_zstd,
    };
    use crate::stream::StreamError;
    use crate::{MAGIC, SUPPORTED_MAJOR};

    const PLAIN_A: &[u8] = b"stream-plain-chunk-A-body";
    const PLAIN_B: &[u8] = b"stream-plain-chunk-B-longer-payload";
    const PLAIN_C: &[u8] = b"C";

    fn seal_none(kind_id: u8, seq: u64, plain: &[u8]) -> Vec<u8> {
        encode_chunk_frame_sealed(
            kind_id,
            codec::NONE,
            0,
            seq,
            0,
            1,
            plain.len() as u32,
            plain,
        )
    }

    #[test]
    fn multi_chunk_none_and_compressed_ordered_plain() {
        // ≥2 chunks: NONE + ZLIB + ZSTD + LZ4 under one prefix.
        let f_none = seal_none(kind::EVENT, 0, PLAIN_A);
        let f_zlib = encode_chunk_frame_zlib(kind::SOURCE, 0, 1, 0, 1, PLAIN_B).unwrap();
        let f_zstd = encode_chunk_frame_zstd(kind::INDEX, 0, 2, 0, 1, PLAIN_C).unwrap();
        let f_lz4 = encode_chunk_frame_lz4(kind::SUMMARY, 0, 3, 0, 1, PLAIN_A).unwrap();

        let wire = encode_prefix_sealed_chunks(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            &[&f_none, &f_zlib, &f_zstd, &f_lz4],
        );
        assert_eq!(&wire[..8], MAGIC.as_slice());

        let (dec, n) = decode_prefix_chunk_stream_plain(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert_eq!(dec.header.major, SUPPORTED_MAJOR);
        assert_eq!(dec.chunks.len(), 4);
        assert_eq!(dec.chunks[0].codec, codec::NONE);
        assert_eq!(dec.chunks[0].plain, PLAIN_A);
        assert_eq!(dec.chunks[0].kind, kind::EVENT);
        assert_eq!(dec.chunks[1].codec, codec::ZLIB);
        assert_eq!(dec.chunks[1].plain, PLAIN_B);
        assert_eq!(dec.chunks[1].kind, kind::SOURCE);
        assert_eq!(dec.chunks[2].codec, codec::ZSTD);
        assert_eq!(dec.chunks[2].plain, PLAIN_C);
        assert_eq!(dec.chunks[3].codec, codec::LZ4);
        assert_eq!(dec.chunks[3].plain, PLAIN_A);

        // Structural: non-inflating stream parse still sees compressed wire on ZLIB.
        let (raw, _) = decode_prefix_chunk_stream(&wire).unwrap();
        assert_eq!(raw.chunks.len(), 4);
        assert_ne!(raw.chunks[1].payload, PLAIN_B);
        assert_eq!(
            decode_chunk_frame_plain(&raw.chunks[1], true).unwrap(),
            PLAIN_B
        );
    }

    #[test]
    fn empty_chunks_after_prefix_ok() {
        let wire = encode_prefix_sealed_chunks(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[]);
        let (dec, n) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
        assert_eq!(n, wire.len());
        assert!(dec.chunks.is_empty());
    }

    #[test]
    fn truncated_mid_stream_err() {
        let f0 = seal_none(kind::EVENT, 0, PLAIN_A);
        let f1 = encode_chunk_frame_zlib(kind::EVENT, 0, 1, 0, 1, PLAIN_B).unwrap();
        let mut wire = encode_prefix_sealed_chunks(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[&f0, &f1]);
        // Drop last bytes of second chunk.
        wire.truncate(wire.len() - 4);
        match decode_prefix_chunk_stream_plain(&wire, false) {
            Err(DecodedStreamError::Stream(StreamError::Chunk(ChunkError::Truncated {
                ..
            }))) => {}
            other => panic!("expected truncated mid-stream, got {other:?}"),
        }
    }

    #[test]
    fn corrupt_zlib_second_chunk_err() {
        let f0 = seal_none(kind::EVENT, 0, PLAIN_A);
        let mut f1 = encode_chunk_frame_zlib(kind::SOURCE, 0, 1, 0, 1, PLAIN_B).unwrap();
        f1[CHUNK_HEADER_LEN] ^= 0xFF;
        if f1.len() > CHUNK_HEADER_LEN + 1 {
            f1[CHUNK_HEADER_LEN + 1] ^= 0xAA;
        }
        let wire = encode_prefix_sealed_chunks(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[&f0, &f1]);
        // Without CRC: payload inflate fails.
        match decode_prefix_chunk_stream_plain(&wire, false) {
            Err(DecodedStreamError::Chunk(DecodedChunkError::Payload(_))) => {}
            other => panic!("expected payload err, got {other:?}"),
        }
        // With CRC: fail at CRC first.
        match decode_prefix_chunk_stream_plain(&wire, true) {
            Err(DecodedStreamError::Chunk(DecodedChunkError::Crc(_))) => {}
            other => panic!("expected crc err, got {other:?}"),
        }
    }

    #[test]
    fn crc_mismatch_when_verify_on_err() {
        let plain = PLAIN_B;
        let compressed = deflate_zlib(plain).unwrap();
        let bad = encode_chunk_frame(
            kind::EVENT,
            codec::ZLIB,
            0,
            0,
            0,
            1,
            plain.len() as u32,
            &compressed,
            compute_payload_crc(&compressed) ^ 0x1111_2222,
        );
        let good = seal_none(kind::SOURCE, 1, PLAIN_A);
        let wire = encode_prefix_sealed_chunks(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[&bad, &good]);
        match decode_prefix_chunk_stream_plain(&wire, true) {
            Err(DecodedStreamError::Chunk(DecodedChunkError::Crc(_))) => {}
            other => panic!("expected crc mismatch, got {other:?}"),
        }
        // Without verify still recovers ordered plains.
        let (dec, n) = decode_prefix_chunk_stream_plain(&wire, false).expect("no crc");
        assert_eq!(n, wire.len());
        assert_eq!(dec.chunks.len(), 2);
        assert_eq!(dec.chunks[0].plain, plain);
        assert_eq!(dec.chunks[1].plain, PLAIN_A);
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_prefix_chunk_stream_plain(&[], true).is_err());
        assert!(decode_prefix_chunk_stream_plain(b"nope", false).is_err());
        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_prefix_chunk_stream_plain(&enc, true) {
            Err(DecodedStreamError::Stream(StreamError::Chunk(ChunkError::BadSync {
                expected,
                got,
            }))) => {
                assert_eq!(expected, CHUNK_SYNC);
                assert_eq!(got, 0xDEAD_BEEF);
            }
            other => panic!("expected bad sync, got {other:?}"),
        }
    }

    #[test]
    fn default_parse_stays_non_inflating_on_stream() {
        let f_zlib = encode_chunk_frame_zlib(kind::EVENT, 0, 0, 0, 1, PLAIN_A).unwrap();
        let wire = encode_prefix_sealed_chunks(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[&f_zlib]);
        let (raw, _) = decode_prefix_chunk_stream(&wire).unwrap();
        assert_eq!(raw.chunks[0].codec, codec::ZLIB);
        assert_ne!(raw.chunks[0].payload, PLAIN_A);
        // parse_chunk_frame on sealed frame alone also non-inflating.
        let frame = parse_chunk_frame(&f_zlib).unwrap();
        assert_ne!(frame.payload, PLAIN_A);
        let (dec, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
        assert_eq!(dec.chunks[0].plain, PLAIN_A);
    }
}

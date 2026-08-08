//! Provisional **format v6** file-prefix + chunk stream composition (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-prefix-chunk-stream-provisional-v0.md`
//!
//! Composes shipped [`crate::encode_file_prefix`] / [`crate::decode_file_prefix`]
//! with [`crate::chunk::encode_chunk_frame`] / [`crate::chunk::parse_chunk_frame`].
//! Codec **NONE** only for MVP; no payload inflate, no event opcodes.

use crate::chunk::{
    encode_chunk_frame, parse_chunk_frame, ChunkError, ChunkFrame, CHUNK_HEADER_LEN,
};
use crate::file_prefix::{
    decode_file_prefix, encode_file_prefix, FilePrefix, FilePrefixError,
};

/// Fail-closed stream composition errors (prefix + chunk frames).
#[derive(Debug, PartialEq, Eq)]
pub enum StreamError {
    Prefix(FilePrefixError),
    Chunk(ChunkError),
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::Prefix(e) => write!(f, "v6 prefix+chunk stream prefix: {e}"),
            StreamError::Chunk(e) => write!(f, "v6 prefix+chunk stream chunk: {e}"),
        }
    }
}

impl std::error::Error for StreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StreamError::Prefix(e) => Some(e),
            StreamError::Chunk(e) => Some(e),
        }
    }
}

impl From<FilePrefixError> for StreamError {
    fn from(e: FilePrefixError) -> Self {
        StreamError::Prefix(e)
    }
}

impl From<ChunkError> for StreamError {
    fn from(e: ChunkError) -> Self {
        StreamError::Chunk(e)
    }
}

pub type StreamResult<T> = std::result::Result<T, StreamError>;

/// One chunk to encode into a prefix+chunk stream (payload opaque).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkSpec<'a> {
    pub kind: u8,
    pub codec: u8,
    pub flags: u16,
    pub sequence: u64,
    pub first_logical_seq: u64,
    pub logical_event_count: u32,
    pub uncompressed_len: u32,
    pub payload: &'a [u8],
    pub payload_checksum: u32,
}

/// Parsed provisional v6 stream: file prefix + zero or more chunk frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixChunkStream<'a> {
    pub prefix: FilePrefix<'a>,
    pub chunks: Vec<ChunkFrame<'a>>,
}

/// Encode a provisional v6 file: `[file prefix][chunk frame…]`.
///
/// Composes shipped `encode_file_prefix` then zero or more `encode_chunk_frame`.
/// MVP payloads are opaque; typical codec is `codec::NONE`. No inflate / CRC verify.
pub fn encode_prefix_chunk_stream(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    chunks: &[ChunkSpec<'_>],
) -> Vec<u8> {
    let mut out = encode_file_prefix(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
    );
    for c in chunks {
        out.extend_from_slice(&encode_chunk_frame(
            c.kind,
            c.codec,
            c.flags,
            c.sequence,
            c.first_logical_seq,
            c.logical_event_count,
            c.uncompressed_len,
            c.payload,
            c.payload_checksum,
        ));
    }
    out
}

/// Decode a provisional v6 file stream: prefix then zero or more chunk frames.
///
/// 1. `decode_file_prefix` (fail-closed bad magic / truncated TLV / missing END).
/// 2. Walk remaining bytes with `parse_chunk_frame` until the buffer is exhausted.
///
/// Zero chunks after a valid prefix is **Ok**. Any leftover that is not a complete
/// frame fails closed via existing chunk errors (truncated mid-chunk, bad sync, …).
/// Returns `(stream, total_bytes_consumed)` (equals `buf.len()` on success).
pub fn decode_prefix_chunk_stream(buf: &[u8]) -> StreamResult<(PrefixChunkStream<'_>, usize)> {
    let (prefix, mut pos) = decode_file_prefix(buf)?;
    let mut chunks = Vec::new();
    while pos < buf.len() {
        let frame = parse_chunk_frame(&buf[pos..])?;
        let frame_len = CHUNK_HEADER_LEN
            .checked_add(frame.payload.len())
            .ok_or(StreamError::Chunk(ChunkError::OversizeCompressed {
                len: frame.compressed_len,
            }))?;
        pos = pos.checked_add(frame_len).ok_or(StreamError::Chunk(ChunkError::Truncated {
            need: frame_len,
            got: buf.len().saturating_sub(pos),
        }))?;
        chunks.push(frame);
    }
    Ok((PrefixChunkStream { prefix, chunks }, pos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{codec, kind, CHUNK_SYNC};
    use crate::tlv::type_id;
    use crate::{FilePrefixError, HEADER_LEN_FULL, MAGIC, SUPPORTED_MAJOR};
    use crate::Error as HeaderError;

    fn empty_prefix_args() -> (u16, u16, u64, u64, u32) {
        (SUPPORTED_MAJOR, 0, 0, 0, 0)
    }

    #[test]
    fn roundtrip_zero_chunks() {
        let (maj, min, req, opt, crc) = empty_prefix_args();
        let enc_a = encode_prefix_chunk_stream(maj, min, req, opt, crc, &[], &[]);
        let enc_b = encode_prefix_chunk_stream(maj, min, req, opt, crc, &[], &[]);
        // Dual-assert encode stability (deterministic).
        assert_eq!(enc_a, enc_b);
        assert_eq!(&enc_a[..8], MAGIC.as_slice());

        let (stream, n) = decode_prefix_chunk_stream(&enc_a).expect("0 chunks");
        assert_eq!(n, enc_a.len());
        assert_eq!(stream.prefix.header.major, SUPPORTED_MAJOR);
        assert_eq!(stream.prefix.header.header_len, HEADER_LEN_FULL);
        assert!(stream.prefix.tlvs.is_empty());
        assert!(stream.chunks.is_empty());

        // Second decode of same bytes is identical (stability).
        let (stream2, n2) = decode_prefix_chunk_stream(&enc_a).expect("0 chunks again");
        assert_eq!(n2, n);
        assert_eq!(stream2.chunks.len(), 0);
        assert_eq!(stream2.prefix.header, stream.prefix.header);
    }

    #[test]
    fn roundtrip_one_or_more_chunks_codec_none() {
        let items: [(u64, u8, &[u8]); 1] = [(type_id::PRODUCER, 0, b"nytprof-rust")];
        let payload = b"opaque-event-body";
        let specs = [ChunkSpec {
            kind: kind::EVENT,
            codec: codec::NONE,
            flags: 0,
            sequence: 1,
            first_logical_seq: 0,
            logical_event_count: 0,
            uncompressed_len: payload.len() as u32,
            payload,
            payload_checksum: 0,
        }];
        let enc = encode_prefix_chunk_stream(SUPPORTED_MAJOR, 1, 0, 0, 0, &items, &specs);
        // Length must be prefix + one frame (no hardcoded golden detached from encode).
        let prefix_only =
            encode_file_prefix(SUPPORTED_MAJOR, 1, 0, 0, 0, &items);
        let one_frame = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            1,
            0,
            0,
            payload.len() as u32,
            payload,
            0,
        );
        assert_eq!(enc.len(), prefix_only.len() + one_frame.len());
        assert_eq!(&enc[prefix_only.len()..], one_frame.as_slice());

        let (stream, n) = decode_prefix_chunk_stream(&enc).expect("≥1 chunk");
        assert_eq!(n, enc.len());
        assert_eq!(stream.prefix.tlvs.len(), 1);
        assert_eq!(stream.prefix.tlvs[0].value, b"nytprof-rust");
        assert_eq!(stream.chunks.len(), 1);
        assert_eq!(stream.chunks[0].kind, kind::EVENT);
        assert_eq!(stream.chunks[0].codec, codec::NONE);
        assert_eq!(stream.chunks[0].sequence, 1);
        assert_eq!(stream.chunks[0].payload, payload);
        assert_eq!(
            stream.chunks[0].compressed_len,
            payload.len() as u32
        );

        // Two-chunk stream.
        let specs2 = [
            specs[0],
            ChunkSpec {
                kind: kind::SOURCE,
                codec: codec::NONE,
                flags: 0,
                sequence: 2,
                first_logical_seq: 0,
                logical_event_count: 0,
                uncompressed_len: 3,
                payload: b"src",
                payload_checksum: 0x11,
            },
        ];
        let enc2 = encode_prefix_chunk_stream(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &specs2);
        let (s2, n2) = decode_prefix_chunk_stream(&enc2).expect("2 chunks");
        assert_eq!(n2, enc2.len());
        assert_eq!(s2.chunks.len(), 2);
        assert_eq!(s2.chunks[0].payload, payload);
        assert_eq!(s2.chunks[1].kind, kind::SOURCE);
        assert_eq!(s2.chunks[1].payload, b"src");
    }

    #[test]
    fn bad_magic_err() {
        let mut enc = encode_prefix_chunk_stream(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[]);
        enc[0] = b'X';
        assert_eq!(
            decode_prefix_chunk_stream(&enc),
            Err(StreamError::Prefix(FilePrefixError::Header(
                HeaderError::BadMagic
            )))
        );
    }

    #[test]
    fn truncated_mid_chunk_after_prefix_err() {
        let payload = b"0123456789abcdef";
        let specs = [ChunkSpec {
            kind: kind::EVENT,
            codec: codec::NONE,
            flags: 0,
            sequence: 0,
            first_logical_seq: 0,
            logical_event_count: 0,
            uncompressed_len: payload.len() as u32,
            payload,
            payload_checksum: 0,
        }];
        let mut enc =
            encode_prefix_chunk_stream(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &specs);
        // Chop into the middle of the chunk payload (keep full prefix).
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        assert!(enc.len() > prefix_n + CHUNK_HEADER_LEN + 4);
        enc.truncate(prefix_n + CHUNK_HEADER_LEN + 4);
        match decode_prefix_chunk_stream(&enc) {
            Err(StreamError::Chunk(ChunkError::Truncated { .. })) => {}
            other => panic!("expected truncated mid-chunk, got {other:?}"),
        }
    }

    #[test]
    fn bad_chunk_sync_after_prefix_err() {
        let mut enc = encode_prefix_chunk_stream(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[]);
        // Append a 40-byte pseudo-header with wrong sync.
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_prefix_chunk_stream(&enc) {
            Err(StreamError::Chunk(ChunkError::BadSync { expected, got })) => {
                assert_eq!(expected, CHUNK_SYNC);
                assert_eq!(got, 0xDEAD_BEEF);
            }
            other => panic!("expected bad sync, got {other:?}"),
        }
    }

    #[test]
    fn never_panic_on_empty_and_garbage() {
        assert!(decode_prefix_chunk_stream(&[]).is_err());
        assert!(decode_prefix_chunk_stream(b"not-a-v6-file").is_err());
        let mut almost = encode_prefix_chunk_stream(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[]);
        almost.push(0x01); // incomplete chunk start
        assert!(decode_prefix_chunk_stream(&almost).is_err());
    }
}

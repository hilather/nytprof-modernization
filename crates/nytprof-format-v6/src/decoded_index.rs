//! Provisional **format v6** decoded INDEX profile consumer path (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-decoded-index-provisional-v0.md`
//!
//! Stream → always-inflate INDEX payloads (optional CRC) → join plain INDEX
//! bytes → `decode_index_body`. Composes shipped decoded-stream + index-body +
//! payload seal helpers. Does **not** change default `parse_chunk_frame`.
//! Not full index catalog freeze, not COL-007 C writer, not CLI v6 default.
//! SUMMARY decoded profile remains residual.

use crate::chunk::{codec, encode_chunk_frame, kind};
use crate::compressed_mixed::{partition_index_records, OwnedIndexRecord};
use crate::compressed_profile::{
    encode_kind_chunk, is_supported_event_codec, CompressedProfileError,
};
use crate::crc::compute_payload_crc;
use crate::decoded_stream::{
    decode_prefix_chunk_stream_plain, encode_prefix_sealed_chunks, DecodedStreamError,
};
use crate::index_body::{
    decode_index_body, encode_index_body, IndexBodyError, IndexRecordSpec,
};
use crate::FixedHeader;

/// Fail-closed decoded-INDEX profile errors.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodedIndexError {
    Stream(DecodedStreamError),
    IndexBody(IndexBodyError),
    Encode(CompressedProfileError),
    /// Non-INDEX/FOOTER kind on this MVP path.
    UnexpectedKind { kind: u8 },
    /// FOOTER not last / more than one FOOTER.
    InvalidFooter,
    /// FOOTER must use codec NONE.
    UnexpectedFooterCodec { codec: u8 },
    /// INDEX codec not in {NONE, ZLIB, ZSTD, LZ4} or mixed across INDEX chunks.
    UnsupportedIndexCodec { codec: u8 },
    /// No INDEX chunks when a non-empty body path is required.
    MissingIndexChunks,
}

impl std::fmt::Display for DecodedIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodedIndexError::Stream(e) => write!(f, "decoded-index stream: {e}"),
            DecodedIndexError::IndexBody(e) => write!(f, "decoded-index body: {e}"),
            DecodedIndexError::Encode(e) => write!(f, "decoded-index encode: {e}"),
            DecodedIndexError::UnexpectedKind { kind } => {
                write!(f, "decoded-index unexpected kind {kind}")
            }
            DecodedIndexError::InvalidFooter => {
                write!(f, "decoded-index invalid FOOTER placement")
            }
            DecodedIndexError::UnexpectedFooterCodec { codec } => {
                write!(f, "decoded-index FOOTER codec {codec} (NONE required)")
            }
            DecodedIndexError::UnsupportedIndexCodec { codec } => {
                write!(f, "decoded-index unsupported INDEX codec {codec}")
            }
            DecodedIndexError::MissingIndexChunks => {
                write!(f, "decoded-index missing INDEX chunks")
            }
        }
    }
}

impl std::error::Error for DecodedIndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodedIndexError::Stream(e) => Some(e),
            DecodedIndexError::IndexBody(e) => Some(e),
            DecodedIndexError::Encode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<DecodedStreamError> for DecodedIndexError {
    fn from(e: DecodedStreamError) -> Self {
        DecodedIndexError::Stream(e)
    }
}

impl From<IndexBodyError> for DecodedIndexError {
    fn from(e: IndexBodyError) -> Self {
        DecodedIndexError::IndexBody(e)
    }
}

impl From<CompressedProfileError> for DecodedIndexError {
    fn from(e: CompressedProfileError) -> Self {
        DecodedIndexError::Encode(e)
    }
}

pub type DecodedIndexResult<T> = std::result::Result<T, DecodedIndexError>;

/// Decoded INDEX profile: header + ordered logical indexes after always-inflate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedIndexProfile {
    pub header: FixedHeader,
    pub index_codec: u8,
    pub index_chunk_count: usize,
    pub records: Vec<OwnedIndexRecord>,
    pub has_footer: bool,
    pub footer_payload: Option<Vec<u8>>,
}

/// Encode a provisional decoded-INDEX profile.
///
/// - File prefix
/// - One or more INDEX chunks: `encode_index_body` sealed under `index_codec`
///   (optionally record-partitioned when `max_indexes_per_chunk` is set and &gt; 0)
/// - Optional FOOTER codec NONE last
///
/// Pure byte-slice / `Vec` API. Does not change default parse inflate policy.
pub fn encode_decoded_index_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    index_codec: u8,
    indexes: &[IndexRecordSpec<'_>],
    max_indexes_per_chunk: usize,
    footer: Option<&[u8]>,
) -> DecodedIndexResult<Vec<u8>> {
    if !indexes.is_empty() && !is_supported_event_codec(index_codec) {
        return Err(DecodedIndexError::UnsupportedIndexCodec {
            codec: index_codec,
        });
    }

    let parts = if indexes.is_empty() {
        Vec::new()
    } else {
        partition_index_records(indexes, max_indexes_per_chunk)
    };

    let mut sealed: Vec<Vec<u8>> = Vec::with_capacity(parts.len() + usize::from(footer.is_some()));
    for (i, part) in parts.iter().enumerate() {
        let plain = encode_index_body(part);
        let frame = encode_kind_chunk(
            kind::INDEX,
            index_codec,
            i as u64,
            part.len() as u32,
            &plain,
        )?;
        sealed.push(frame);
    }

    if let Some(fp) = footer {
        let checksum = compute_payload_crc(fp);
        sealed.push(encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            sealed.len() as u64,
            0,
            0,
            fp.len() as u32,
            fp,
            checksum,
        ));
    }

    let refs: Vec<&[u8]> = sealed.iter().map(|v| v.as_slice()).collect();
    Ok(encode_prefix_sealed_chunks(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        &refs,
    ))
}

/// Decode a provisional decoded-INDEX profile.
///
/// 1. Shipped `decode_prefix_chunk_stream_plain` (always inflate + optional CRC)
/// 2. Collect INDEX plains in order; optional trailing FOOTER codec NONE
/// 3. Join INDEX plains → single `decode_index_body`
///
/// Default `parse_chunk_frame` remains non-inflating.
pub fn decode_decoded_index_profile(
    buf: &[u8],
    verify_crc: bool,
) -> DecodedIndexResult<(DecodedIndexProfile, usize)> {
    let (stream, n) = decode_prefix_chunk_stream_plain(buf, verify_crc)?;
    let mut plain = Vec::new();
    let mut index_chunk_count = 0usize;
    let mut has_footer = false;
    let mut footer_payload: Option<Vec<u8>> = None;
    let mut saw_footer = false;
    let mut index_codec = codec::NONE;
    let mut saw_index = false;

    for chunk in &stream.chunks {
        if saw_footer {
            return Err(DecodedIndexError::InvalidFooter);
        }
        match chunk.kind {
            k if k == kind::INDEX => {
                if !is_supported_event_codec(chunk.codec) {
                    return Err(DecodedIndexError::UnsupportedIndexCodec {
                        codec: chunk.codec,
                    });
                }
                if !saw_index {
                    index_codec = chunk.codec;
                    saw_index = true;
                } else if chunk.codec != index_codec {
                    return Err(DecodedIndexError::UnsupportedIndexCodec {
                        codec: chunk.codec,
                    });
                }
                plain.extend_from_slice(&chunk.plain);
                index_chunk_count += 1;
            }
            k if k == kind::FOOTER => {
                if chunk.codec != codec::NONE {
                    return Err(DecodedIndexError::UnexpectedFooterCodec {
                        codec: chunk.codec,
                    });
                }
                has_footer = true;
                footer_payload = Some(chunk.plain.clone());
                saw_footer = true;
            }
            other => {
                return Err(DecodedIndexError::UnexpectedKind { kind: other });
            }
        }
    }

    if plain.is_empty() && index_chunk_count == 0 {
        return Ok((
            DecodedIndexProfile {
                header: stream.header,
                index_codec: codec::NONE,
                index_chunk_count: 0,
                records: Vec::new(),
                has_footer,
                footer_payload,
            },
            n,
        ));
    }

    if index_chunk_count == 0 {
        return Err(DecodedIndexError::MissingIndexChunks);
    }

    let (body_recs, body_n) = decode_index_body(&plain)?;
    if body_n != plain.len() {
        return Err(DecodedIndexError::IndexBody(IndexBodyError::Truncated {
            need: plain.len(),
            got: body_n,
        }));
    }

    let mut records = Vec::with_capacity(body_recs.len());
    for r in &body_recs {
        records.push(OwnedIndexRecord {
            key_id: r.key_id,
            file_offset: r.file_offset,
            length: r.length,
            label: r.label.data.to_vec(),
        });
    }

    Ok((
        DecodedIndexProfile {
            header: stream.header,
            index_codec,
            index_chunk_count,
            records,
            has_footer,
            footer_payload,
        },
        n,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{parse_chunk_frame, CHUNK_HEADER_LEN, CHUNK_SYNC};
    use crate::decoded_chunk::DecodedChunkError;
    use crate::decoded_stream::DecodedStreamError;
    use crate::payload_codec::deflate_zlib;
    use crate::stream::{decode_prefix_chunk_stream, StreamError};
    use crate::{MAGIC, SUPPORTED_MAJOR};

    fn sample_indexes() -> [IndexRecordSpec<'static>; 3] {
        [
            IndexRecordSpec {
                key_id: 1,
                file_offset: 100,
                length: 50,
                string_id: 0,
                string_flags: 0,
                label: b"decoded-index-entry-one",
            },
            IndexRecordSpec {
                key_id: 2,
                file_offset: 200,
                length: 80,
                string_id: 1,
                string_flags: 0,
                label: b"decoded-index-entry-two-longer",
            },
            IndexRecordSpec {
                key_id: 3,
                file_offset: 300,
                length: 10,
                string_id: 2,
                string_flags: 0,
                label: b"third",
            },
        ]
    }

    fn assert_sample(recs: &[OwnedIndexRecord]) {
        assert_eq!(recs.len(), 3);
        assert_eq!(recs[0].key_id, 1);
        assert_eq!(recs[0].file_offset, 100);
        assert_eq!(recs[0].length, 50);
        assert_eq!(recs[0].label, b"decoded-index-entry-one");
        assert_eq!(recs[1].key_id, 2);
        assert_eq!(recs[1].file_offset, 200);
        assert_eq!(recs[1].length, 80);
        assert_eq!(recs[1].label, b"decoded-index-entry-two-longer");
        assert_eq!(recs[2].key_id, 3);
        assert_eq!(recs[2].file_offset, 300);
        assert_eq!(recs[2].length, 10);
        assert_eq!(recs[2].label, b"third");
    }

    #[test]
    fn none_zlib_zstd_lz4_single_chunk_roundtrip() {
        let indexes = sample_indexes();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_index_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &indexes,
                0,
                Some(b"idx-end"),
            )
            .expect("encode");
            assert_eq!(&wire[..8], MAGIC.as_slice());

            let (prof, n) = decode_decoded_index_profile(&wire, true).expect("decode");
            assert_eq!(n, wire.len());
            assert_eq!(prof.index_codec, c);
            assert_eq!(prof.index_chunk_count, 1);
            assert_sample(&prof.records);
            assert!(prof.has_footer);
            assert_eq!(prof.footer_payload.as_deref(), Some(&b"idx-end"[..]));

            if c != codec::NONE {
                let (raw, _) = decode_prefix_chunk_stream(&wire).unwrap();
                let idx = raw.chunks.iter().find(|f| f.kind == kind::INDEX).unwrap();
                let body = encode_index_body(&indexes);
                assert_ne!(idx.payload, body.as_slice());
            }
        }
    }

    #[test]
    fn multi_chunk_record_aligned_zlib_roundtrip() {
        let indexes = sample_indexes();
        let wire = encode_decoded_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &indexes,
            1,
            None,
        )
        .expect("encode");
        let (prof, n) = decode_decoded_index_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert!(prof.index_chunk_count >= 2);
        assert_eq!(prof.index_codec, codec::ZLIB);
        assert_sample(&prof.records);

        let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
        let mut joined = Vec::new();
        for c in &stream.chunks {
            if c.kind == kind::INDEX {
                joined.extend_from_slice(&c.plain);
            }
        }
        assert_eq!(joined, encode_index_body(&indexes));
    }

    #[test]
    fn truncated_index_body_join_err() {
        let indexes = sample_indexes();
        let mut wire = encode_decoded_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &indexes,
            0,
            None,
        )
        .unwrap();
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        let payload_off = prefix_n + CHUNK_HEADER_LEN;
        let keep = f0.payload.len() / 2;
        assert!(keep > 0 && keep < f0.payload.len());
        wire.truncate(payload_off + keep);
        match decode_decoded_index_profile(&wire, false) {
            Err(DecodedIndexError::Stream(DecodedStreamError::Stream(StreamError::Chunk(
                crate::chunk::ChunkError::Truncated { .. },
            )))) => {}
            Err(DecodedIndexError::IndexBody(_)) => {}
            other => panic!("expected truncated err, got {other:?}"),
        }
    }

    #[test]
    fn corrupt_zlib_index_payload_err() {
        let indexes = sample_indexes();
        let mut wire = encode_decoded_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &indexes,
            0,
            None,
        )
        .unwrap();
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        assert_eq!(f0.codec, codec::ZLIB);
        let payload_len = f0.payload.len();
        let payload_off = prefix_n + CHUNK_HEADER_LEN;
        wire[payload_off] ^= 0xFF;
        if payload_len > 1 {
            wire[payload_off + 1] ^= 0xAA;
        }
        match decode_decoded_index_profile(&wire, false) {
            Err(DecodedIndexError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Payload(_),
            ))) => {}
            other => panic!("expected payload err, got {other:?}"),
        }
        match decode_decoded_index_profile(&wire, true) {
            Err(DecodedIndexError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc err, got {other:?}"),
        }
    }

    #[test]
    fn crc_mismatch_when_verify_on_err() {
        let indexes = sample_indexes();
        let plain = encode_index_body(&indexes);
        let compressed = deflate_zlib(&plain).unwrap();
        let bad = encode_chunk_frame(
            kind::INDEX,
            codec::ZLIB,
            0,
            0,
            0,
            indexes.len() as u32,
            plain.len() as u32,
            &compressed,
            compute_payload_crc(&compressed) ^ 0xABCD_EF01,
        );
        let wire = encode_prefix_sealed_chunks(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[&bad]);
        match decode_decoded_index_profile(&wire, true) {
            Err(DecodedIndexError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc mismatch, got {other:?}"),
        }
        let (prof, n) = decode_decoded_index_profile(&wire, false).expect("no crc");
        assert_eq!(n, wire.len());
        assert_sample(&prof.records);
    }

    #[test]
    fn empty_indexes_prefix_only() {
        let wire = encode_decoded_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &[],
            0,
            None,
        )
        .unwrap();
        let (prof, n) = decode_decoded_index_profile(&wire, true).unwrap();
        assert_eq!(n, wire.len());
        assert_eq!(prof.index_chunk_count, 0);
        assert!(prof.records.is_empty());
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_decoded_index_profile(&[], true).is_err());
        assert!(decode_decoded_index_profile(b"nope", false).is_err());
        let mut enc = crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_decoded_index_profile(&enc, true) {
            Err(DecodedIndexError::Stream(DecodedStreamError::Stream(StreamError::Chunk(
                crate::chunk::ChunkError::BadSync { expected, got },
            )))) => {
                assert_eq!(expected, CHUNK_SYNC);
                assert_eq!(got, 0xDEAD_BEEF);
            }
            other => panic!("expected bad sync, got {other:?}"),
        }
    }

    #[test]
    fn zlib_wire_not_plain_index_body() {
        let indexes = sample_indexes();
        let plain = encode_index_body(&indexes);
        let expected = deflate_zlib(&plain).unwrap();
        let wire = encode_decoded_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &indexes,
            0,
            None,
        )
        .unwrap();
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::INDEX);
        assert_eq!(f0.payload, expected.as_slice());
    }
}

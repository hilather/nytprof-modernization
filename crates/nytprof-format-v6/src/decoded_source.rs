//! Provisional **format v6** decoded SOURCE profile consumer path (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-decoded-source-provisional-v0.md`
//!
//! Stream → always-inflate SOURCE payloads (optional CRC) → join plain SOURCE
//! bytes → `decode_source_body`. Composes shipped decoded-stream + source-body +
//! payload seal helpers. Does **not** change default `parse_chunk_frame`.
//! Not full SRC_LINE catalog freeze, not COL-007 C writer, not CLI v6 default.

use crate::chunk::{codec, encode_chunk_frame, kind};
use crate::compressed_mixed::{partition_source_records, OwnedSourceRecord};
use crate::compressed_profile::{
    encode_kind_chunk, is_supported_event_codec, CompressedProfileError,
};
use crate::crc::compute_payload_crc;
use crate::decoded_stream::{
    decode_prefix_chunk_stream_plain, encode_prefix_sealed_chunks, DecodedStreamError,
};
use crate::source_body::{
    decode_source_body, encode_source_body, SourceBodyError, SourceRecordSpec,
};
use crate::FixedHeader;

/// Fail-closed decoded-SOURCE profile errors.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodedSourceError {
    Stream(DecodedStreamError),
    SourceBody(SourceBodyError),
    Encode(CompressedProfileError),
    /// Non-SOURCE/FOOTER kind on this MVP path.
    UnexpectedKind { kind: u8 },
    /// FOOTER not last / more than one FOOTER.
    InvalidFooter,
    /// FOOTER must use codec NONE.
    UnexpectedFooterCodec { codec: u8 },
    /// SOURCE codec not in {NONE, ZLIB, ZSTD, LZ4} or mixed across SOURCE chunks.
    UnsupportedSourceCodec { codec: u8 },
    /// No SOURCE chunks when a non-empty body path is required.
    MissingSourceChunks,
}

impl std::fmt::Display for DecodedSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodedSourceError::Stream(e) => write!(f, "decoded-source stream: {e}"),
            DecodedSourceError::SourceBody(e) => write!(f, "decoded-source body: {e}"),
            DecodedSourceError::Encode(e) => write!(f, "decoded-source encode: {e}"),
            DecodedSourceError::UnexpectedKind { kind } => {
                write!(f, "decoded-source unexpected kind {kind}")
            }
            DecodedSourceError::InvalidFooter => {
                write!(f, "decoded-source invalid FOOTER placement")
            }
            DecodedSourceError::UnexpectedFooterCodec { codec } => {
                write!(f, "decoded-source FOOTER codec {codec} (NONE required)")
            }
            DecodedSourceError::UnsupportedSourceCodec { codec } => {
                write!(f, "decoded-source unsupported SOURCE codec {codec}")
            }
            DecodedSourceError::MissingSourceChunks => {
                write!(f, "decoded-source missing SOURCE chunks")
            }
        }
    }
}

impl std::error::Error for DecodedSourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodedSourceError::Stream(e) => Some(e),
            DecodedSourceError::SourceBody(e) => Some(e),
            DecodedSourceError::Encode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<DecodedStreamError> for DecodedSourceError {
    fn from(e: DecodedStreamError) -> Self {
        DecodedSourceError::Stream(e)
    }
}

impl From<SourceBodyError> for DecodedSourceError {
    fn from(e: SourceBodyError) -> Self {
        DecodedSourceError::SourceBody(e)
    }
}

impl From<CompressedProfileError> for DecodedSourceError {
    fn from(e: CompressedProfileError) -> Self {
        DecodedSourceError::Encode(e)
    }
}

pub type DecodedSourceResult<T> = std::result::Result<T, DecodedSourceError>;

/// Decoded SOURCE profile: header + ordered logical sources after always-inflate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedSourceProfile {
    pub header: FixedHeader,
    pub source_codec: u8,
    pub source_chunk_count: usize,
    pub records: Vec<OwnedSourceRecord>,
    pub has_footer: bool,
    pub footer_payload: Option<Vec<u8>>,
}

/// Encode a provisional decoded-SOURCE profile.
///
/// - File prefix
/// - One or more SOURCE chunks: `encode_source_body` sealed under `source_codec`
///   (optionally record-partitioned when `max_sources_per_chunk` is set and &gt; 0)
/// - Optional FOOTER codec NONE last
///
/// Pure byte-slice / `Vec` API. Does not change default parse inflate policy.
pub fn encode_decoded_source_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    source_codec: u8,
    sources: &[SourceRecordSpec<'_>],
    max_sources_per_chunk: usize,
    footer: Option<&[u8]>,
) -> DecodedSourceResult<Vec<u8>> {
    if !sources.is_empty() && !is_supported_event_codec(source_codec) {
        return Err(DecodedSourceError::UnsupportedSourceCodec {
            codec: source_codec,
        });
    }

    let parts = if sources.is_empty() {
        Vec::new()
    } else {
        partition_source_records(sources, max_sources_per_chunk)
    };

    let mut sealed: Vec<Vec<u8>> = Vec::with_capacity(parts.len() + usize::from(footer.is_some()));
    for (i, part) in parts.iter().enumerate() {
        let plain = encode_source_body(part);
        let frame = encode_kind_chunk(
            kind::SOURCE,
            source_codec,
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

/// Decode a provisional decoded-SOURCE profile.
///
/// 1. Shipped `decode_prefix_chunk_stream_plain` (always inflate + optional CRC)
/// 2. Collect SOURCE plains in order; optional trailing FOOTER codec NONE
/// 3. Join SOURCE plains → single `decode_source_body`
///
/// Default `parse_chunk_frame` remains non-inflating.
pub fn decode_decoded_source_profile(
    buf: &[u8],
    verify_crc: bool,
) -> DecodedSourceResult<(DecodedSourceProfile, usize)> {
    let (stream, n) = decode_prefix_chunk_stream_plain(buf, verify_crc)?;
    let mut plain = Vec::new();
    let mut source_chunk_count = 0usize;
    let mut has_footer = false;
    let mut footer_payload: Option<Vec<u8>> = None;
    let mut saw_footer = false;
    let mut source_codec = codec::NONE;
    let mut saw_source = false;

    for chunk in &stream.chunks {
        if saw_footer {
            return Err(DecodedSourceError::InvalidFooter);
        }
        match chunk.kind {
            k if k == kind::SOURCE => {
                if !is_supported_event_codec(chunk.codec) {
                    return Err(DecodedSourceError::UnsupportedSourceCodec {
                        codec: chunk.codec,
                    });
                }
                if !saw_source {
                    source_codec = chunk.codec;
                    saw_source = true;
                } else if chunk.codec != source_codec {
                    return Err(DecodedSourceError::UnsupportedSourceCodec {
                        codec: chunk.codec,
                    });
                }
                plain.extend_from_slice(&chunk.plain);
                source_chunk_count += 1;
            }
            k if k == kind::FOOTER => {
                if chunk.codec != codec::NONE {
                    return Err(DecodedSourceError::UnexpectedFooterCodec {
                        codec: chunk.codec,
                    });
                }
                has_footer = true;
                footer_payload = Some(chunk.plain.clone());
                saw_footer = true;
            }
            other => {
                return Err(DecodedSourceError::UnexpectedKind { kind: other });
            }
        }
    }

    if plain.is_empty() && source_chunk_count == 0 {
        return Ok((
            DecodedSourceProfile {
                header: stream.header,
                source_codec: codec::NONE,
                source_chunk_count: 0,
                records: Vec::new(),
                has_footer,
                footer_payload,
            },
            n,
        ));
    }

    if source_chunk_count == 0 {
        return Err(DecodedSourceError::MissingSourceChunks);
    }

    let (body_recs, body_n) = decode_source_body(&plain)?;
    if body_n != plain.len() {
        return Err(DecodedSourceError::SourceBody(SourceBodyError::Truncated {
            need: plain.len(),
            got: body_n,
        }));
    }

    let mut records = Vec::with_capacity(body_recs.len());
    for r in &body_recs {
        records.push(OwnedSourceRecord {
            fid: r.fid,
            line: r.line,
            text: r.text.data.to_vec(),
        });
    }

    Ok((
        DecodedSourceProfile {
            header: stream.header,
            source_codec,
            source_chunk_count,
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

    fn sample_sources() -> [SourceRecordSpec<'static>; 3] {
        [
            SourceRecordSpec {
                fid: 1,
                line: 10,
                string_id: 0,
                string_flags: 0,
                text: b"decoded-source-line-one",
            },
            SourceRecordSpec {
                fid: 1,
                line: 11,
                string_id: 1,
                string_flags: 0,
                text: b"decoded-source-line-two-longer",
            },
            SourceRecordSpec {
                fid: 2,
                line: 1,
                string_id: 2,
                string_flags: 0,
                text: b"third",
            },
        ]
    }

    fn assert_sample(recs: &[OwnedSourceRecord]) {
        assert_eq!(recs.len(), 3);
        assert_eq!(recs[0].fid, 1);
        assert_eq!(recs[0].line, 10);
        assert_eq!(recs[0].text, b"decoded-source-line-one");
        assert_eq!(recs[1].fid, 1);
        assert_eq!(recs[1].line, 11);
        assert_eq!(recs[1].text, b"decoded-source-line-two-longer");
        assert_eq!(recs[2].fid, 2);
        assert_eq!(recs[2].line, 1);
        assert_eq!(recs[2].text, b"third");
    }

    #[test]
    fn none_zlib_zstd_lz4_single_chunk_roundtrip() {
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_source_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &sources,
                0,
                Some(b"src-end"),
            )
            .expect("encode");
            assert_eq!(&wire[..8], MAGIC.as_slice());

            let (prof, n) = decode_decoded_source_profile(&wire, true).expect("decode");
            assert_eq!(n, wire.len());
            assert_eq!(prof.source_codec, c);
            assert_eq!(prof.source_chunk_count, 1);
            assert_sample(&prof.records);
            assert!(prof.has_footer);
            assert_eq!(prof.footer_payload.as_deref(), Some(&b"src-end"[..]));

            if c != codec::NONE {
                let (raw, _) = decode_prefix_chunk_stream(&wire).unwrap();
                let src = raw.chunks.iter().find(|f| f.kind == kind::SOURCE).unwrap();
                let body = encode_source_body(&sources);
                assert_ne!(src.payload, body.as_slice());
            }
        }
    }

    #[test]
    fn multi_chunk_record_aligned_zlib_roundtrip() {
        let sources = sample_sources();
        let wire = encode_decoded_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &sources,
            1,
            None,
        )
        .expect("encode");
        let (prof, n) = decode_decoded_source_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert!(prof.source_chunk_count >= 2);
        assert_eq!(prof.source_codec, codec::ZLIB);
        assert_sample(&prof.records);

        let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
        let mut joined = Vec::new();
        for c in &stream.chunks {
            if c.kind == kind::SOURCE {
                joined.extend_from_slice(&c.plain);
            }
        }
        assert_eq!(joined, encode_source_body(&sources));
    }

    #[test]
    fn truncated_source_body_join_err() {
        let sources = sample_sources();
        let mut wire = encode_decoded_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &sources,
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
        match decode_decoded_source_profile(&wire, false) {
            Err(DecodedSourceError::Stream(DecodedStreamError::Stream(StreamError::Chunk(
                crate::chunk::ChunkError::Truncated { .. },
            )))) => {}
            Err(DecodedSourceError::SourceBody(_)) => {}
            other => panic!("expected truncated err, got {other:?}"),
        }
    }

    #[test]
    fn corrupt_zlib_source_payload_err() {
        let sources = sample_sources();
        let mut wire = encode_decoded_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &sources,
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
        match decode_decoded_source_profile(&wire, false) {
            Err(DecodedSourceError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Payload(_),
            ))) => {}
            other => panic!("expected payload err, got {other:?}"),
        }
        match decode_decoded_source_profile(&wire, true) {
            Err(DecodedSourceError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc err, got {other:?}"),
        }
    }

    #[test]
    fn crc_mismatch_when_verify_on_err() {
        let sources = sample_sources();
        let plain = encode_source_body(&sources);
        let compressed = deflate_zlib(&plain).unwrap();
        let bad = encode_chunk_frame(
            kind::SOURCE,
            codec::ZLIB,
            0,
            0,
            0,
            sources.len() as u32,
            plain.len() as u32,
            &compressed,
            compute_payload_crc(&compressed) ^ 0xABCD_EF01,
        );
        let wire = encode_prefix_sealed_chunks(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[&bad]);
        match decode_decoded_source_profile(&wire, true) {
            Err(DecodedSourceError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc mismatch, got {other:?}"),
        }
        let (prof, n) = decode_decoded_source_profile(&wire, false).expect("no crc");
        assert_eq!(n, wire.len());
        assert_sample(&prof.records);
    }

    #[test]
    fn empty_sources_prefix_only() {
        let wire = encode_decoded_source_profile(
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
        let (prof, n) = decode_decoded_source_profile(&wire, true).unwrap();
        assert_eq!(n, wire.len());
        assert_eq!(prof.source_chunk_count, 0);
        assert!(prof.records.is_empty());
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_decoded_source_profile(&[], true).is_err());
        assert!(decode_decoded_source_profile(b"nope", false).is_err());
        let mut enc = crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_decoded_source_profile(&enc, true) {
            Err(DecodedSourceError::Stream(DecodedStreamError::Stream(StreamError::Chunk(
                crate::chunk::ChunkError::BadSync { expected, got },
            )))) => {
                assert_eq!(expected, CHUNK_SYNC);
                assert_eq!(got, 0xDEAD_BEEF);
            }
            other => panic!("expected bad sync, got {other:?}"),
        }
    }

    #[test]
    fn zlib_wire_not_plain_source_body() {
        let sources = sample_sources();
        let plain = encode_source_body(&sources);
        let expected = deflate_zlib(&plain).unwrap();
        let wire = encode_decoded_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &sources,
            0,
            None,
        )
        .unwrap();
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::SOURCE);
        assert_eq!(f0.payload, expected.as_slice());
    }
}

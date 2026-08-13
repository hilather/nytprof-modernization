//! Provisional **format v6** decoded SUMMARY profile consumer path (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-decoded-summary-provisional-v0.md`
//!
//! Stream → always-inflate SUMMARY payloads (optional CRC) → join plain SUMMARY
//! bytes → `decode_summary_body`. Composes shipped decoded-stream + summary-body +
//! payload seal helpers. Does **not** change default `parse_chunk_frame`.
//! Not full summary catalog freeze, not COL-007 C writer, not CLI v6 default.

use crate::chunk::{codec, encode_chunk_frame, kind};
use crate::compressed_mixed::{partition_summary_records, OwnedSummaryRecord};
use crate::compressed_profile::{
    encode_kind_chunk, is_supported_event_codec, CompressedProfileError,
};
use crate::crc::compute_payload_crc;
use crate::decoded_stream::{
    decode_prefix_chunk_stream_plain, encode_prefix_sealed_chunks, DecodedStreamError,
};
use crate::summary_body::{
    decode_summary_body, encode_summary_body, SummaryBodyError, SummaryRecordSpec,
};
use crate::FixedHeader;

/// Fail-closed decoded-SUMMARY profile errors.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodedSummaryError {
    Stream(DecodedStreamError),
    SummaryBody(SummaryBodyError),
    Encode(CompressedProfileError),
    /// Non-SUMMARY/FOOTER kind on this MVP path.
    UnexpectedKind {
        kind: u8,
    },
    /// FOOTER not last / more than one FOOTER.
    InvalidFooter,
    /// FOOTER must use codec NONE.
    UnexpectedFooterCodec {
        codec: u8,
    },
    /// SUMMARY codec not in {NONE, ZLIB, ZSTD, LZ4} or mixed across SUMMARY chunks.
    UnsupportedSummaryCodec {
        codec: u8,
    },
    /// No SUMMARY chunks when a non-empty body path is required.
    MissingSummaryChunks,
}

impl std::fmt::Display for DecodedSummaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodedSummaryError::Stream(e) => write!(f, "decoded-summary stream: {e}"),
            DecodedSummaryError::SummaryBody(e) => write!(f, "decoded-summary body: {e}"),
            DecodedSummaryError::Encode(e) => write!(f, "decoded-summary encode: {e}"),
            DecodedSummaryError::UnexpectedKind { kind } => {
                write!(f, "decoded-summary unexpected kind {kind}")
            }
            DecodedSummaryError::InvalidFooter => {
                write!(f, "decoded-summary invalid FOOTER placement")
            }
            DecodedSummaryError::UnexpectedFooterCodec { codec } => {
                write!(f, "decoded-summary FOOTER codec {codec} (NONE required)")
            }
            DecodedSummaryError::UnsupportedSummaryCodec { codec } => {
                write!(f, "decoded-summary unsupported SUMMARY codec {codec}")
            }
            DecodedSummaryError::MissingSummaryChunks => {
                write!(f, "decoded-summary missing SUMMARY chunks")
            }
        }
    }
}

impl std::error::Error for DecodedSummaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodedSummaryError::Stream(e) => Some(e),
            DecodedSummaryError::SummaryBody(e) => Some(e),
            DecodedSummaryError::Encode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<DecodedStreamError> for DecodedSummaryError {
    fn from(e: DecodedStreamError) -> Self {
        DecodedSummaryError::Stream(e)
    }
}

impl From<SummaryBodyError> for DecodedSummaryError {
    fn from(e: SummaryBodyError) -> Self {
        DecodedSummaryError::SummaryBody(e)
    }
}

impl From<CompressedProfileError> for DecodedSummaryError {
    fn from(e: CompressedProfileError) -> Self {
        DecodedSummaryError::Encode(e)
    }
}

pub type DecodedSummaryResult<T> = std::result::Result<T, DecodedSummaryError>;

/// Decoded SUMMARY profile: header + ordered logical summaries after always-inflate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedSummaryProfile {
    pub header: FixedHeader,
    pub summary_codec: u8,
    pub summary_chunk_count: usize,
    pub records: Vec<OwnedSummaryRecord>,
    pub has_footer: bool,
    pub footer_payload: Option<Vec<u8>>,
}

/// Encode a provisional decoded-SUMMARY profile.
///
/// - File prefix
/// - One or more SUMMARY chunks: `encode_summary_body` sealed under `summary_codec`
///   (optionally record-partitioned when `max_summaries_per_chunk` is set and &gt; 0)
/// - Optional FOOTER codec NONE last
///
/// Pure byte-slice / `Vec` API. Does not change default parse inflate policy.
pub fn encode_decoded_summary_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    summary_codec: u8,
    summaries: &[SummaryRecordSpec<'_>],
    max_summaries_per_chunk: usize,
    footer: Option<&[u8]>,
) -> DecodedSummaryResult<Vec<u8>> {
    if !summaries.is_empty() && !is_supported_event_codec(summary_codec) {
        return Err(DecodedSummaryError::UnsupportedSummaryCodec {
            codec: summary_codec,
        });
    }

    let parts = if summaries.is_empty() {
        Vec::new()
    } else {
        partition_summary_records(summaries, max_summaries_per_chunk)
    };

    let mut sealed: Vec<Vec<u8>> = Vec::with_capacity(parts.len() + usize::from(footer.is_some()));
    for (i, part) in parts.iter().enumerate() {
        let plain = encode_summary_body(part);
        let frame = encode_kind_chunk(
            kind::SUMMARY,
            summary_codec,
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

/// Decode a provisional decoded-SUMMARY profile.
///
/// 1. Shipped `decode_prefix_chunk_stream_plain` (always inflate + optional CRC)
/// 2. Collect SUMMARY plains in order; optional trailing FOOTER codec NONE
/// 3. Join SUMMARY plains → single `decode_summary_body`
///
/// Default `parse_chunk_frame` remains non-inflating.
pub fn decode_decoded_summary_profile(
    buf: &[u8],
    verify_crc: bool,
) -> DecodedSummaryResult<(DecodedSummaryProfile, usize)> {
    let (stream, n) = decode_prefix_chunk_stream_plain(buf, verify_crc)?;
    let mut plain = Vec::new();
    let mut summary_chunk_count = 0usize;
    let mut has_footer = false;
    let mut footer_payload: Option<Vec<u8>> = None;
    let mut saw_footer = false;
    let mut summary_codec = codec::NONE;
    let mut saw_summary = false;

    for chunk in &stream.chunks {
        if saw_footer {
            return Err(DecodedSummaryError::InvalidFooter);
        }
        match chunk.kind {
            k if k == kind::SUMMARY => {
                if !is_supported_event_codec(chunk.codec) {
                    return Err(DecodedSummaryError::UnsupportedSummaryCodec {
                        codec: chunk.codec,
                    });
                }
                if !saw_summary {
                    summary_codec = chunk.codec;
                    saw_summary = true;
                } else if chunk.codec != summary_codec {
                    return Err(DecodedSummaryError::UnsupportedSummaryCodec {
                        codec: chunk.codec,
                    });
                }
                plain.extend_from_slice(&chunk.plain);
                summary_chunk_count += 1;
            }
            k if k == kind::FOOTER => {
                if chunk.codec != codec::NONE {
                    return Err(DecodedSummaryError::UnexpectedFooterCodec { codec: chunk.codec });
                }
                has_footer = true;
                footer_payload = Some(chunk.plain.clone());
                saw_footer = true;
            }
            other => {
                return Err(DecodedSummaryError::UnexpectedKind { kind: other });
            }
        }
    }

    if plain.is_empty() && summary_chunk_count == 0 {
        return Ok((
            DecodedSummaryProfile {
                header: stream.header,
                summary_codec: codec::NONE,
                summary_chunk_count: 0,
                records: Vec::new(),
                has_footer,
                footer_payload,
            },
            n,
        ));
    }

    if summary_chunk_count == 0 {
        return Err(DecodedSummaryError::MissingSummaryChunks);
    }

    let (body_recs, body_n) = decode_summary_body(&plain)?;
    if body_n != plain.len() {
        return Err(DecodedSummaryError::SummaryBody(
            SummaryBodyError::Truncated {
                need: plain.len(),
                got: body_n,
            },
        ));
    }

    let mut records = Vec::with_capacity(body_recs.len());
    for r in &body_recs {
        records.push(OwnedSummaryRecord {
            key_id: r.key_id,
            count: r.count,
            value: r.value,
            label: r.label.data.to_vec(),
        });
    }

    Ok((
        DecodedSummaryProfile {
            header: stream.header,
            summary_codec,
            summary_chunk_count,
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

    fn sample_summaries() -> [SummaryRecordSpec<'static>; 3] {
        [
            SummaryRecordSpec {
                key_id: 1,
                count: 10,
                value: 100,
                string_id: 0,
                string_flags: 0,
                label: b"decoded-summary-entry-one",
            },
            SummaryRecordSpec {
                key_id: 2,
                count: 20,
                value: 200,
                string_id: 1,
                string_flags: 0,
                label: b"decoded-summary-entry-two-longer",
            },
            SummaryRecordSpec {
                key_id: 3,
                count: 5,
                value: 50,
                string_id: 2,
                string_flags: 0,
                label: b"third",
            },
        ]
    }

    fn assert_sample(recs: &[OwnedSummaryRecord]) {
        assert_eq!(recs.len(), 3);
        assert_eq!(recs[0].key_id, 1);
        assert_eq!(recs[0].count, 10);
        assert_eq!(recs[0].value, 100);
        assert_eq!(recs[0].label, b"decoded-summary-entry-one");
        assert_eq!(recs[1].key_id, 2);
        assert_eq!(recs[1].count, 20);
        assert_eq!(recs[1].value, 200);
        assert_eq!(recs[1].label, b"decoded-summary-entry-two-longer");
        assert_eq!(recs[2].key_id, 3);
        assert_eq!(recs[2].count, 5);
        assert_eq!(recs[2].value, 50);
        assert_eq!(recs[2].label, b"third");
    }

    #[test]
    fn none_zlib_zstd_lz4_single_chunk_roundtrip() {
        let summaries = sample_summaries();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_summary_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &summaries,
                0,
                Some(b"sum-end"),
            )
            .expect("encode");
            assert_eq!(&wire[..8], MAGIC.as_slice());

            let (prof, n) = decode_decoded_summary_profile(&wire, true).expect("decode");
            assert_eq!(n, wire.len());
            assert_eq!(prof.summary_codec, c);
            assert_eq!(prof.summary_chunk_count, 1);
            assert_sample(&prof.records);
            assert!(prof.has_footer);
            assert_eq!(prof.footer_payload.as_deref(), Some(&b"sum-end"[..]));

            if c != codec::NONE {
                let (raw, _) = decode_prefix_chunk_stream(&wire).unwrap();
                let s = raw.chunks.iter().find(|f| f.kind == kind::SUMMARY).unwrap();
                let body = encode_summary_body(&summaries);
                assert_ne!(s.payload, body.as_slice());
            }
        }
    }

    #[test]
    fn multi_chunk_record_aligned_zlib_roundtrip() {
        let summaries = sample_summaries();
        let wire = encode_decoded_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &summaries,
            1,
            None,
        )
        .expect("encode");
        let (prof, n) = decode_decoded_summary_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert!(prof.summary_chunk_count >= 2);
        assert_eq!(prof.summary_codec, codec::ZLIB);
        assert_sample(&prof.records);

        let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
        let mut joined = Vec::new();
        for c in &stream.chunks {
            if c.kind == kind::SUMMARY {
                joined.extend_from_slice(&c.plain);
            }
        }
        assert_eq!(joined, encode_summary_body(&summaries));
    }

    #[test]
    fn truncated_summary_body_join_err() {
        let summaries = sample_summaries();
        let mut wire = encode_decoded_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &summaries,
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
        match decode_decoded_summary_profile(&wire, false) {
            Err(DecodedSummaryError::Stream(DecodedStreamError::Stream(StreamError::Chunk(
                crate::chunk::ChunkError::Truncated { .. },
            )))) => {}
            Err(DecodedSummaryError::SummaryBody(_)) => {}
            other => panic!("expected truncated err, got {other:?}"),
        }
    }

    #[test]
    fn corrupt_zlib_summary_payload_err() {
        let summaries = sample_summaries();
        let mut wire = encode_decoded_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &summaries,
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
        match decode_decoded_summary_profile(&wire, false) {
            Err(DecodedSummaryError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Payload(_),
            ))) => {}
            other => panic!("expected payload err, got {other:?}"),
        }
        match decode_decoded_summary_profile(&wire, true) {
            Err(DecodedSummaryError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc err, got {other:?}"),
        }
    }

    #[test]
    fn crc_mismatch_when_verify_on_err() {
        let summaries = sample_summaries();
        let plain = encode_summary_body(&summaries);
        let compressed = deflate_zlib(&plain).unwrap();
        let bad = encode_chunk_frame(
            kind::SUMMARY,
            codec::ZLIB,
            0,
            0,
            0,
            summaries.len() as u32,
            plain.len() as u32,
            &compressed,
            compute_payload_crc(&compressed) ^ 0xABCD_EF01,
        );
        let wire = encode_prefix_sealed_chunks(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[&bad]);
        match decode_decoded_summary_profile(&wire, true) {
            Err(DecodedSummaryError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc mismatch, got {other:?}"),
        }
        let (prof, n) = decode_decoded_summary_profile(&wire, false).expect("no crc");
        assert_eq!(n, wire.len());
        assert_sample(&prof.records);
    }

    #[test]
    fn empty_summaries_prefix_only() {
        let wire = encode_decoded_summary_profile(
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
        let (prof, n) = decode_decoded_summary_profile(&wire, true).unwrap();
        assert_eq!(n, wire.len());
        assert_eq!(prof.summary_chunk_count, 0);
        assert!(prof.records.is_empty());
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_decoded_summary_profile(&[], true).is_err());
        assert!(decode_decoded_summary_profile(b"nope", false).is_err());
        let mut enc = crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_decoded_summary_profile(&enc, true) {
            Err(DecodedSummaryError::Stream(DecodedStreamError::Stream(StreamError::Chunk(
                crate::chunk::ChunkError::BadSync { expected, got },
            )))) => {
                assert_eq!(expected, CHUNK_SYNC);
                assert_eq!(got, 0xDEAD_BEEF);
            }
            other => panic!("expected bad sync, got {other:?}"),
        }
    }

    #[test]
    fn zlib_wire_not_plain_summary_body() {
        let summaries = sample_summaries();
        let plain = encode_summary_body(&summaries);
        let expected = deflate_zlib(&plain).unwrap();
        let wire = encode_decoded_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &summaries,
            0,
            None,
        )
        .unwrap();
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::SUMMARY);
        assert_eq!(f0.payload, expected.as_slice());
    }
}

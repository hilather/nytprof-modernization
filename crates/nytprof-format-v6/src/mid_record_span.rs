//! Provisional **format v6** mid-record spanning across EVENT, SOURCE, INDEX,
//! and SUMMARY chunks (COL-007 runway).
//!
//! Schemas:
//! - EVENT: `docs/schemas/v6-mid-record-span-provisional-v0.md`
//! - SOURCE: `docs/schemas/v6-mid-record-source-provisional-v0.md`
//! - INDEX: `docs/schemas/v6-mid-record-index-provisional-v0.md`
//! - SUMMARY: `docs/schemas/v6-mid-record-summary-provisional-v0.md`
//!
//! Encodes a full body, splits bytes mid-record into ≥2 same-kind payloads,
//! seals with NONE/ZLIB/ZSTD/LZ4, and decodes by concatenating inflated payloads
//! then a single body decode. Default `parse_chunk_frame` stays non-inflating.
//! Not COL-007 C writer. Always-on inflate remains residual.

use crate::chunk::{codec, encode_chunk_frame, kind};
use crate::compressed_mixed::{OwnedIndexRecord, OwnedSourceRecord, OwnedSummaryRecord};
use crate::compressed_profile::{
    encode_event_chunk, encode_kind_chunk, is_supported_event_codec, CompressedProfileError,
    OwnedEventRecord,
};
use crate::crc::compute_payload_crc;
use crate::event_body::{decode_event_body, encode_event_body, EventBodyError, EventRecordSpec};
use crate::file_prefix::encode_file_prefix;
use crate::index_body::{
    decode_index_body, encode_index_body, IndexBodyError, IndexRecordSpec,
};
use crate::payload_codec::decode_chunk_payload;
use crate::source_body::{
    decode_source_body, encode_source_body, SourceBodyError, SourceRecordSpec,
};
use crate::stream::decode_prefix_chunk_stream;
use crate::summary_body::{
    decode_summary_body, encode_summary_body, SummaryBodyError, SummaryRecordSpec,
};
use crate::FixedHeader;

/// Fail-closed mid-record span errors (composition layer).
pub type MidRecordSpanError = CompressedProfileError;
pub type MidRecordSpanResult<T> = std::result::Result<T, MidRecordSpanError>;

/// Decoded mid-record-span EVENT profile (owned logical events).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidRecordSpanProfile {
    pub header: FixedHeader,
    pub event_codec: u8,
    /// Number of EVENT chunks that contributed body bytes.
    pub event_chunk_count: usize,
    /// Byte offset where the body was split (first chunk length in plain bytes).
    pub split_at: usize,
    pub records: Vec<OwnedEventRecord>,
    pub has_footer: bool,
    pub footer_payload: Option<Vec<u8>>,
}

/// Split `body` at `split_at` into `(prefix, suffix)`.
///
/// Returns `None` if `split_at == 0` or `split_at >= body.len()` (no real span).
pub fn split_event_body_bytes(body: &[u8], split_at: usize) -> Option<(&[u8], &[u8])> {
    if split_at == 0 || split_at >= body.len() {
        return None;
    }
    Some((&body[..split_at], &body[split_at..]))
}

/// Pick a provisional mid-body split: halfway through the full body, clamped
/// so both pieces are non-empty. Prefer an interior byte that is not a record
/// boundary when the body has ≥2 records (caller should still verify with tests).
pub fn default_mid_body_split(body: &[u8]) -> Option<usize> {
    if body.len() < 2 {
        return None;
    }
    Some(body.len() / 2)
}

/// Encode a multi-chunk EVENT profile where body bytes span a mid-record boundary.
///
/// 1. `plain = encode_event_body(events)`
/// 2. Split at `split_at` (must be interior: `0 < split_at < plain.len()`)
/// 3. Each piece → EVENT frame under `event_codec` (sequence 0, 1, …)
/// 4. Optional FOOTER codec NONE last
///
/// First EVENT's `logical_event_count` is `events.len()`; continuations use `0`.
pub fn encode_mid_record_span_event_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    event_codec: u8,
    events: &[EventRecordSpec<'_>],
    split_at: usize,
    footer: Option<&[u8]>,
) -> MidRecordSpanResult<Vec<u8>> {
    if events.is_empty() {
        return Err(CompressedProfileError::EventBody(EventBodyError::Truncated {
            need: 1,
            got: 0,
        }));
    }
    if !is_supported_event_codec(event_codec) {
        return Err(CompressedProfileError::UnsupportedEventCodec {
            codec: event_codec,
        });
    }

    let plain = encode_event_body(events);
    let (head, tail) = split_event_body_bytes(&plain, split_at).ok_or_else(|| {
        CompressedProfileError::EventBody(EventBodyError::Truncated {
            need: split_at.saturating_add(1),
            got: plain.len(),
        })
    })?;

    let mut out = encode_file_prefix(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
    );

    // First piece: full logical count (provisional).
    let frame0 = encode_event_chunk(event_codec, 0, events.len() as u32, head)?;
    out.extend_from_slice(&frame0);
    // Continuation: zero logical count (provisional).
    let frame1 = encode_event_chunk(event_codec, 1, 0, tail)?;
    out.extend_from_slice(&frame1);

    if let Some(fp) = footer {
        let checksum = compute_payload_crc(fp);
        out.extend_from_slice(&encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            2,
            0,
            0,
            fp.len() as u32,
            fp,
            checksum,
        ));
    }

    Ok(out)
}

/// Decode a mid-record-span EVENT profile.
///
/// Non-inflating stream parse → inflate each EVENT payload → concatenate plain
/// bytes → single `decode_event_body` on the joined buffer.
pub fn decode_mid_record_span_event_profile(
    buf: &[u8],
) -> MidRecordSpanResult<(MidRecordSpanProfile, usize)> {
    let (stream, n) = decode_prefix_chunk_stream(buf)?;
    let mut plain = Vec::new();
    let mut event_chunk_count = 0usize;
    let mut has_footer = false;
    let mut footer_payload: Option<Vec<u8>> = None;
    let mut saw_footer = false;
    let mut event_codec = codec::NONE;
    let mut saw_event = false;
    let mut first_piece_len = 0usize;

    for frame in &stream.chunks {
        if saw_footer {
            return Err(CompressedProfileError::InvalidFooter);
        }
        match frame.kind {
            k if k == kind::EVENT => {
                if !is_supported_event_codec(frame.codec) {
                    return Err(CompressedProfileError::UnsupportedEventCodec {
                        codec: frame.codec,
                    });
                }
                if !saw_event {
                    event_codec = frame.codec;
                    saw_event = true;
                } else if frame.codec != event_codec {
                    return Err(CompressedProfileError::UnsupportedEventCodec {
                        codec: frame.codec,
                    });
                }
                let piece = decode_chunk_payload(frame)?;
                if event_chunk_count == 0 {
                    first_piece_len = piece.len();
                }
                plain.extend_from_slice(&piece);
                event_chunk_count += 1;
            }
            k if k == kind::FOOTER => {
                if frame.codec != codec::NONE {
                    return Err(CompressedProfileError::UnexpectedFooterCodec {
                        codec: frame.codec,
                    });
                }
                has_footer = true;
                footer_payload = Some(frame.payload.to_vec());
                saw_footer = true;
            }
            other => {
                return Err(CompressedProfileError::UnexpectedKind { kind: other });
            }
        }
    }

    if event_chunk_count < 2 {
        // Mid-record span MVP requires ≥2 EVENT pieces.
        return Err(CompressedProfileError::EventBody(EventBodyError::Truncated {
            need: 2,
            got: event_chunk_count,
        }));
    }

    let (body_recs, body_n) = decode_event_body(&plain)?;
    if body_n != plain.len() {
        return Err(CompressedProfileError::EventBody(EventBodyError::Truncated {
            need: plain.len(),
            got: body_n,
        }));
    }

    let mut records = Vec::with_capacity(body_recs.len());
    for r in &body_recs {
        records.push(OwnedEventRecord::from_borrowed(r));
    }

    Ok((
        MidRecordSpanProfile {
            header: stream.prefix.header,
            event_codec,
            event_chunk_count,
            split_at: first_piece_len,
            records,
            has_footer,
            footer_payload,
        },
        n,
    ))
}

// --- SOURCE mid-record span (mirror EVENT) ---

/// Split SOURCE body bytes at `split_at` into `(prefix, suffix)`.
///
/// Same rules as [`split_event_body_bytes`]: interior only.
pub fn split_source_body_bytes(body: &[u8], split_at: usize) -> Option<(&[u8], &[u8])> {
    split_event_body_bytes(body, split_at)
}

/// Decoded mid-record-span SOURCE profile (owned logical sources).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidRecordSourceProfile {
    pub header: FixedHeader,
    pub source_codec: u8,
    /// Number of SOURCE chunks that contributed body bytes.
    pub source_chunk_count: usize,
    /// Byte offset where the body was split (first chunk length in plain bytes).
    pub split_at: usize,
    pub records: Vec<OwnedSourceRecord>,
    pub has_footer: bool,
    pub footer_payload: Option<Vec<u8>>,
}

/// Encode a multi-chunk SOURCE profile where body bytes span a mid-record boundary.
///
/// 1. `plain = encode_source_body(sources)`
/// 2. Split at `split_at` (must be interior: `0 < split_at < plain.len()`)
/// 3. Each piece → SOURCE frame under `source_codec` (sequence 0, 1, …)
/// 4. Optional FOOTER codec NONE last
///
/// First SOURCE's `logical_event_count` is `sources.len()`; continuations use `0`.
pub fn encode_mid_record_span_source_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    source_codec: u8,
    sources: &[SourceRecordSpec<'_>],
    split_at: usize,
    footer: Option<&[u8]>,
) -> MidRecordSpanResult<Vec<u8>> {
    if sources.is_empty() {
        return Err(CompressedProfileError::SourceBody(
            SourceBodyError::Truncated { need: 1, got: 0 },
        ));
    }
    if !is_supported_event_codec(source_codec) {
        return Err(CompressedProfileError::UnsupportedEventCodec {
            codec: source_codec,
        });
    }

    let plain = encode_source_body(sources);
    let (head, tail) = split_source_body_bytes(&plain, split_at).ok_or_else(|| {
        CompressedProfileError::SourceBody(SourceBodyError::Truncated {
            need: split_at.saturating_add(1),
            got: plain.len(),
        })
    })?;

    let mut out = encode_file_prefix(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
    );

    let frame0 = encode_kind_chunk(kind::SOURCE, source_codec, 0, sources.len() as u32, head)?;
    out.extend_from_slice(&frame0);
    let frame1 = encode_kind_chunk(kind::SOURCE, source_codec, 1, 0, tail)?;
    out.extend_from_slice(&frame1);

    if let Some(fp) = footer {
        let checksum = compute_payload_crc(fp);
        out.extend_from_slice(&encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            2,
            0,
            0,
            fp.len() as u32,
            fp,
            checksum,
        ));
    }

    Ok(out)
}

/// Decode a mid-record-span SOURCE profile.
///
/// Non-inflating stream parse → inflate each SOURCE payload → concatenate plain
/// bytes → single `decode_source_body` on the joined buffer.
pub fn decode_mid_record_span_source_profile(
    buf: &[u8],
) -> MidRecordSpanResult<(MidRecordSourceProfile, usize)> {
    let (stream, n) = decode_prefix_chunk_stream(buf)?;
    let mut plain = Vec::new();
    let mut source_chunk_count = 0usize;
    let mut has_footer = false;
    let mut footer_payload: Option<Vec<u8>> = None;
    let mut saw_footer = false;
    let mut source_codec = codec::NONE;
    let mut saw_source = false;
    let mut first_piece_len = 0usize;

    for frame in &stream.chunks {
        if saw_footer {
            return Err(CompressedProfileError::InvalidFooter);
        }
        match frame.kind {
            k if k == kind::SOURCE => {
                if !is_supported_event_codec(frame.codec) {
                    return Err(CompressedProfileError::UnsupportedEventCodec {
                        codec: frame.codec,
                    });
                }
                if !saw_source {
                    source_codec = frame.codec;
                    saw_source = true;
                } else if frame.codec != source_codec {
                    return Err(CompressedProfileError::UnsupportedEventCodec {
                        codec: frame.codec,
                    });
                }
                let piece = decode_chunk_payload(frame)?;
                if source_chunk_count == 0 {
                    first_piece_len = piece.len();
                }
                plain.extend_from_slice(&piece);
                source_chunk_count += 1;
            }
            k if k == kind::FOOTER => {
                if frame.codec != codec::NONE {
                    return Err(CompressedProfileError::UnexpectedFooterCodec {
                        codec: frame.codec,
                    });
                }
                has_footer = true;
                footer_payload = Some(frame.payload.to_vec());
                saw_footer = true;
            }
            other => {
                return Err(CompressedProfileError::UnexpectedKind { kind: other });
            }
        }
    }

    if source_chunk_count < 2 {
        return Err(CompressedProfileError::SourceBody(
            SourceBodyError::Truncated {
                need: 2,
                got: source_chunk_count,
            },
        ));
    }

    let (body_recs, body_n) = decode_source_body(&plain)?;
    if body_n != plain.len() {
        return Err(CompressedProfileError::SourceBody(
            SourceBodyError::Truncated {
                need: plain.len(),
                got: body_n,
            },
        ));
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
        MidRecordSourceProfile {
            header: stream.prefix.header,
            source_codec,
            source_chunk_count,
            split_at: first_piece_len,
            records,
            has_footer,
            footer_payload,
        },
        n,
    ))
}

// --- INDEX mid-record span (mirror SOURCE) ---

/// Split INDEX body bytes at `split_at` into `(prefix, suffix)`.
///
/// Same rules as [`split_event_body_bytes`]: interior only.
pub fn split_index_body_bytes(body: &[u8], split_at: usize) -> Option<(&[u8], &[u8])> {
    split_event_body_bytes(body, split_at)
}

/// Decoded mid-record-span INDEX profile (owned logical indexes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidRecordIndexProfile {
    pub header: FixedHeader,
    pub index_codec: u8,
    /// Number of INDEX chunks that contributed body bytes.
    pub index_chunk_count: usize,
    /// Byte offset where the body was split (first chunk length in plain bytes).
    pub split_at: usize,
    pub records: Vec<OwnedIndexRecord>,
    pub has_footer: bool,
    pub footer_payload: Option<Vec<u8>>,
}

/// Encode a multi-chunk INDEX profile where body bytes span a mid-record boundary.
///
/// 1. `plain = encode_index_body(indexes)`
/// 2. Split at `split_at` (must be interior: `0 < split_at < plain.len()`)
/// 3. Each piece → INDEX frame under `index_codec` (sequence 0, 1, …)
/// 4. Optional FOOTER codec NONE last
///
/// First INDEX's `logical_event_count` is `indexes.len()`; continuations use `0`.
pub fn encode_mid_record_span_index_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    index_codec: u8,
    indexes: &[IndexRecordSpec<'_>],
    split_at: usize,
    footer: Option<&[u8]>,
) -> MidRecordSpanResult<Vec<u8>> {
    if indexes.is_empty() {
        return Err(CompressedProfileError::IndexBody(
            IndexBodyError::Truncated { need: 1, got: 0 },
        ));
    }
    if !is_supported_event_codec(index_codec) {
        return Err(CompressedProfileError::UnsupportedEventCodec {
            codec: index_codec,
        });
    }

    let plain = encode_index_body(indexes);
    let (head, tail) = split_index_body_bytes(&plain, split_at).ok_or_else(|| {
        CompressedProfileError::IndexBody(IndexBodyError::Truncated {
            need: split_at.saturating_add(1),
            got: plain.len(),
        })
    })?;

    let mut out = encode_file_prefix(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
    );

    let frame0 = encode_kind_chunk(kind::INDEX, index_codec, 0, indexes.len() as u32, head)?;
    out.extend_from_slice(&frame0);
    let frame1 = encode_kind_chunk(kind::INDEX, index_codec, 1, 0, tail)?;
    out.extend_from_slice(&frame1);

    if let Some(fp) = footer {
        let checksum = compute_payload_crc(fp);
        out.extend_from_slice(&encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            2,
            0,
            0,
            fp.len() as u32,
            fp,
            checksum,
        ));
    }

    Ok(out)
}

/// Decode a mid-record-span INDEX profile.
///
/// Non-inflating stream parse → inflate each INDEX payload → concatenate plain
/// bytes → single `decode_index_body` on the joined buffer.
pub fn decode_mid_record_span_index_profile(
    buf: &[u8],
) -> MidRecordSpanResult<(MidRecordIndexProfile, usize)> {
    let (stream, n) = decode_prefix_chunk_stream(buf)?;
    let mut plain = Vec::new();
    let mut index_chunk_count = 0usize;
    let mut has_footer = false;
    let mut footer_payload: Option<Vec<u8>> = None;
    let mut saw_footer = false;
    let mut index_codec = codec::NONE;
    let mut saw_index = false;
    let mut first_piece_len = 0usize;

    for frame in &stream.chunks {
        if saw_footer {
            return Err(CompressedProfileError::InvalidFooter);
        }
        match frame.kind {
            k if k == kind::INDEX => {
                if !is_supported_event_codec(frame.codec) {
                    return Err(CompressedProfileError::UnsupportedEventCodec {
                        codec: frame.codec,
                    });
                }
                if !saw_index {
                    index_codec = frame.codec;
                    saw_index = true;
                } else if frame.codec != index_codec {
                    return Err(CompressedProfileError::UnsupportedEventCodec {
                        codec: frame.codec,
                    });
                }
                let piece = decode_chunk_payload(frame)?;
                if index_chunk_count == 0 {
                    first_piece_len = piece.len();
                }
                plain.extend_from_slice(&piece);
                index_chunk_count += 1;
            }
            k if k == kind::FOOTER => {
                if frame.codec != codec::NONE {
                    return Err(CompressedProfileError::UnexpectedFooterCodec {
                        codec: frame.codec,
                    });
                }
                has_footer = true;
                footer_payload = Some(frame.payload.to_vec());
                saw_footer = true;
            }
            other => {
                return Err(CompressedProfileError::UnexpectedKind { kind: other });
            }
        }
    }

    if index_chunk_count < 2 {
        return Err(CompressedProfileError::IndexBody(
            IndexBodyError::Truncated {
                need: 2,
                got: index_chunk_count,
            },
        ));
    }

    let (body_recs, body_n) = decode_index_body(&plain)?;
    if body_n != plain.len() {
        return Err(CompressedProfileError::IndexBody(
            IndexBodyError::Truncated {
                need: plain.len(),
                got: body_n,
            },
        ));
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
        MidRecordIndexProfile {
            header: stream.prefix.header,
            index_codec,
            index_chunk_count,
            split_at: first_piece_len,
            records,
            has_footer,
            footer_payload,
        },
        n,
    ))
}

// --- SUMMARY mid-record span (mirror INDEX) ---

/// Split SUMMARY body bytes at `split_at` into `(prefix, suffix)`.
///
/// Same rules as [`split_event_body_bytes`]: interior only.
pub fn split_summary_body_bytes(body: &[u8], split_at: usize) -> Option<(&[u8], &[u8])> {
    split_event_body_bytes(body, split_at)
}

/// Decoded mid-record-span SUMMARY profile (owned logical summaries).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidRecordSummaryProfile {
    pub header: FixedHeader,
    pub summary_codec: u8,
    /// Number of SUMMARY chunks that contributed body bytes.
    pub summary_chunk_count: usize,
    /// Byte offset where the body was split (first chunk length in plain bytes).
    pub split_at: usize,
    pub records: Vec<OwnedSummaryRecord>,
    pub has_footer: bool,
    pub footer_payload: Option<Vec<u8>>,
}

/// Encode a multi-chunk SUMMARY profile where body bytes span a mid-record boundary.
///
/// 1. `plain = encode_summary_body(summaries)`
/// 2. Split at `split_at` (must be interior: `0 < split_at < plain.len()`)
/// 3. Each piece → SUMMARY frame under `summary_codec` (sequence 0, 1, …)
/// 4. Optional FOOTER codec NONE last
///
/// First SUMMARY's `logical_event_count` is `summaries.len()`; continuations use `0`.
pub fn encode_mid_record_span_summary_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    summary_codec: u8,
    summaries: &[SummaryRecordSpec<'_>],
    split_at: usize,
    footer: Option<&[u8]>,
) -> MidRecordSpanResult<Vec<u8>> {
    if summaries.is_empty() {
        return Err(CompressedProfileError::SummaryBody(
            SummaryBodyError::Truncated { need: 1, got: 0 },
        ));
    }
    if !is_supported_event_codec(summary_codec) {
        return Err(CompressedProfileError::UnsupportedEventCodec {
            codec: summary_codec,
        });
    }

    let plain = encode_summary_body(summaries);
    let (head, tail) = split_summary_body_bytes(&plain, split_at).ok_or_else(|| {
        CompressedProfileError::SummaryBody(SummaryBodyError::Truncated {
            need: split_at.saturating_add(1),
            got: plain.len(),
        })
    })?;

    let mut out = encode_file_prefix(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
    );

    let frame0 =
        encode_kind_chunk(kind::SUMMARY, summary_codec, 0, summaries.len() as u32, head)?;
    out.extend_from_slice(&frame0);
    let frame1 = encode_kind_chunk(kind::SUMMARY, summary_codec, 1, 0, tail)?;
    out.extend_from_slice(&frame1);

    if let Some(fp) = footer {
        let checksum = compute_payload_crc(fp);
        out.extend_from_slice(&encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            2,
            0,
            0,
            fp.len() as u32,
            fp,
            checksum,
        ));
    }

    Ok(out)
}

/// Decode a mid-record-span SUMMARY profile.
///
/// Non-inflating stream parse → inflate each SUMMARY payload → concatenate plain
/// bytes → single `decode_summary_body` on the joined buffer.
pub fn decode_mid_record_span_summary_profile(
    buf: &[u8],
) -> MidRecordSpanResult<(MidRecordSummaryProfile, usize)> {
    let (stream, n) = decode_prefix_chunk_stream(buf)?;
    let mut plain = Vec::new();
    let mut summary_chunk_count = 0usize;
    let mut has_footer = false;
    let mut footer_payload: Option<Vec<u8>> = None;
    let mut saw_footer = false;
    let mut summary_codec = codec::NONE;
    let mut saw_summary = false;
    let mut first_piece_len = 0usize;

    for frame in &stream.chunks {
        if saw_footer {
            return Err(CompressedProfileError::InvalidFooter);
        }
        match frame.kind {
            k if k == kind::SUMMARY => {
                if !is_supported_event_codec(frame.codec) {
                    return Err(CompressedProfileError::UnsupportedEventCodec {
                        codec: frame.codec,
                    });
                }
                if !saw_summary {
                    summary_codec = frame.codec;
                    saw_summary = true;
                } else if frame.codec != summary_codec {
                    return Err(CompressedProfileError::UnsupportedEventCodec {
                        codec: frame.codec,
                    });
                }
                let piece = decode_chunk_payload(frame)?;
                if summary_chunk_count == 0 {
                    first_piece_len = piece.len();
                }
                plain.extend_from_slice(&piece);
                summary_chunk_count += 1;
            }
            k if k == kind::FOOTER => {
                if frame.codec != codec::NONE {
                    return Err(CompressedProfileError::UnexpectedFooterCodec {
                        codec: frame.codec,
                    });
                }
                has_footer = true;
                footer_payload = Some(frame.payload.to_vec());
                saw_footer = true;
            }
            other => {
                return Err(CompressedProfileError::UnexpectedKind { kind: other });
            }
        }
    }

    if summary_chunk_count < 2 {
        return Err(CompressedProfileError::SummaryBody(
            SummaryBodyError::Truncated {
                need: 2,
                got: summary_chunk_count,
            },
        ));
    }

    let (body_recs, body_n) = decode_summary_body(&plain)?;
    if body_n != plain.len() {
        return Err(CompressedProfileError::SummaryBody(
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
        MidRecordSummaryProfile {
            header: stream.prefix.header,
            summary_codec,
            summary_chunk_count,
            split_at: first_piece_len,
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
    use crate::chunk::{CHUNK_HEADER_LEN, CHUNK_SYNC};
    use crate::{MAGIC, SUPPORTED_MAJOR};

    fn sample_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 10,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"mid-span-label",
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 2,
                ticks: 20,
            },
        ]
    }

    fn assert_sample_records(recs: &[OwnedEventRecord]) {
        assert_eq!(recs.len(), 3);
        match &recs[0] {
            OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
            other => panic!("{other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"mid-span-label"),
            other => panic!("{other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 20),
            other => panic!("{other:?}"),
        }
    }

    /// Interior split that is not a whole-record boundary for the sample body.
    fn sample_split_at() -> usize {
        let body = encode_event_body(&sample_events());
        // First record is opcode+flags+3 uleb fields — small; pick mid of full body.
        let split = default_mid_body_split(&body).expect("body large enough");
        // Ensure neither piece alone is a complete valid event-body of all 3 records.
        assert!(split > 0 && split < body.len());
        // First piece alone must fail full decode of 3 records (truncated).
        assert!(decode_event_body(&body[..split]).is_err());
        split
    }

    #[test]
    fn split_event_body_bytes_rejects_edges() {
        let body = encode_event_body(&sample_events());
        assert!(split_event_body_bytes(&body, 0).is_none());
        assert!(split_event_body_bytes(&body, body.len()).is_none());
        assert!(split_event_body_bytes(&body, body.len() + 1).is_none());
        let (a, b) = split_event_body_bytes(&body, 1).unwrap();
        assert_eq!(a.len() + b.len(), body.len());
    }

    #[test]
    fn mid_record_span_none_zlib_zstd_lz4_roundtrip() {
        let events = sample_events();
        let split = sample_split_at();
        let body = encode_event_body(&events);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let enc_a = encode_mid_record_span_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                split,
                Some(b"end"),
            )
            .expect("encode");
            let enc_b = encode_mid_record_span_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                split,
                Some(b"end"),
            )
            .expect("encode 2");
            assert_eq!(enc_a, enc_b, "deterministic codec {c}");
            assert_eq!(&enc_a[..8], MAGIC.as_slice());

            let (prof, n) = decode_mid_record_span_event_profile(&enc_a).expect("decode");
            assert_eq!(n, enc_a.len());
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.event_chunk_count, 2);
            assert_eq!(prof.split_at, split);
            assert_sample_records(&prof.records);
            assert!(prof.has_footer);
            assert_eq!(prof.footer_payload.as_deref(), Some(&b"end"[..]));

            // Wire: each piece is not the full plain body; pieces reassemble.
            let (stream, _) = decode_prefix_chunk_stream(&enc_a).unwrap();
            let events_f: Vec<_> = stream
                .chunks
                .iter()
                .filter(|f| f.kind == kind::EVENT)
                .collect();
            assert_eq!(events_f.len(), 2);
            let p0 = decode_chunk_payload(events_f[0]).unwrap();
            let p1 = decode_chunk_payload(events_f[1]).unwrap();
            assert_eq!(p0.len(), split);
            assert_eq!([p0.as_slice(), p1.as_slice()].concat(), body);
            if c != codec::NONE {
                assert_ne!(events_f[0].payload, p0.as_slice());
            }
            // Each piece alone fails full 3-record decode.
            assert!(decode_event_body(&p0).is_err());
            assert!(decode_event_body(&p1).is_err());
        }
    }

    #[test]
    fn mid_record_span_invalid_split_err() {
        let events = sample_events();
        let body = encode_event_body(&events);
        match encode_mid_record_span_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &events,
            0,
            None,
        ) {
            Err(CompressedProfileError::EventBody(_)) => {}
            other => panic!("expected invalid split, got {other:?}"),
        }
        match encode_mid_record_span_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &events,
            body.len(),
            None,
        ) {
            Err(CompressedProfileError::EventBody(_)) => {}
            other => panic!("expected invalid split end, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_span_truncated_joined_body_err() {
        // Build a valid span, then drop the second EVENT payload bytes so join is truncated.
        let events = sample_events();
        let split = sample_split_at();
        let mut enc = encode_mid_record_span_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &events,
            split,
            None,
        )
        .unwrap();
        // Truncate after first EVENT only.
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        let only_first = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        enc.truncate(only_first);
        match decode_mid_record_span_event_profile(&enc) {
            Err(CompressedProfileError::EventBody(_)) => {}
            other => panic!("expected truncated mid-record / need ≥2 chunks, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_span_corrupt_zlib_second_chunk_err() {
        let events = sample_events();
        let split = sample_split_at();
        let mut enc = encode_mid_record_span_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &events,
            split,
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        let second_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let f1 = parse_chunk_frame(&enc[second_off..]).unwrap();
        assert_eq!(f1.codec, codec::ZLIB);
        let payload_len = f1.payload.len();
        let payload_off = second_off + CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        if payload_len > 1 {
            enc[payload_off + 1] ^= 0xAA;
        }
        match decode_mid_record_span_event_profile(&enc) {
            Err(CompressedProfileError::Payload(_)) => {}
            other => panic!("expected payload error on corrupt continuation, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_span_zlib_wire_uses_deflate() {
        let events = sample_events();
        let split = sample_split_at();
        let body = encode_event_body(&events);
        let head = &body[..split];
        let expected = deflate_zlib(head).unwrap();
        let enc = encode_mid_record_span_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &events,
            split,
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        assert_eq!(f0.payload, expected.as_slice());
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_mid_record_span_event_profile(&[]).is_err());
        assert!(decode_mid_record_span_event_profile(b"nope").is_err());
        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_mid_record_span_event_profile(&enc) {
            Err(CompressedProfileError::Stream(StreamError::Chunk(ChunkError::BadSync {
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
    fn empty_events_encode_err() {
        match encode_mid_record_span_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &[],
            1,
            None,
        ) {
            Err(CompressedProfileError::EventBody(_)) => {}
            other => panic!("expected empty events err, got {other:?}"),
        }
    }

    // --- SOURCE mid-record span ---

    fn sample_sources() -> [SourceRecordSpec<'static>; 3] {
        [
            SourceRecordSpec {
                fid: 1,
                line: 10,
                string_id: 0,
                string_flags: 0,
                text: b"first-source-line",
            },
            SourceRecordSpec {
                fid: 1,
                line: 11,
                string_id: 1,
                string_flags: 0,
                text: b"second-source-line-longer",
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

    fn assert_sample_sources(recs: &[OwnedSourceRecord]) {
        assert_eq!(recs.len(), 3);
        assert_eq!(recs[0].fid, 1);
        assert_eq!(recs[0].line, 10);
        assert_eq!(recs[0].text, b"first-source-line");
        assert_eq!(recs[1].fid, 1);
        assert_eq!(recs[1].line, 11);
        assert_eq!(recs[1].text, b"second-source-line-longer");
        assert_eq!(recs[2].fid, 2);
        assert_eq!(recs[2].line, 1);
        assert_eq!(recs[2].text, b"third");
    }

    fn sample_source_split_at() -> usize {
        let body = encode_source_body(&sample_sources());
        let split = default_mid_body_split(&body).expect("body large enough");
        assert!(split > 0 && split < body.len());
        assert!(decode_source_body(&body[..split]).is_err());
        split
    }

    #[test]
    fn split_source_body_bytes_rejects_edges() {
        let body = encode_source_body(&sample_sources());
        assert!(split_source_body_bytes(&body, 0).is_none());
        assert!(split_source_body_bytes(&body, body.len()).is_none());
        let (a, b) = split_source_body_bytes(&body, 1).unwrap();
        assert_eq!(a.len() + b.len(), body.len());
    }

    #[test]
    fn mid_record_source_none_zlib_zstd_lz4_roundtrip() {
        let sources = sample_sources();
        let split = sample_source_split_at();
        let body = encode_source_body(&sources);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let enc_a = encode_mid_record_span_source_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &sources,
                split,
                Some(b"src-end"),
            )
            .expect("encode");
            let enc_b = encode_mid_record_span_source_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &sources,
                split,
                Some(b"src-end"),
            )
            .expect("encode 2");
            assert_eq!(enc_a, enc_b, "deterministic codec {c}");
            assert_eq!(&enc_a[..8], MAGIC.as_slice());

            let (prof, n) = decode_mid_record_span_source_profile(&enc_a).expect("decode");
            assert_eq!(n, enc_a.len());
            assert_eq!(prof.source_codec, c);
            assert_eq!(prof.source_chunk_count, 2);
            assert_eq!(prof.split_at, split);
            assert_sample_sources(&prof.records);
            assert!(prof.has_footer);
            assert_eq!(prof.footer_payload.as_deref(), Some(&b"src-end"[..]));

            let (stream, _) = decode_prefix_chunk_stream(&enc_a).unwrap();
            let src_f: Vec<_> = stream
                .chunks
                .iter()
                .filter(|f| f.kind == kind::SOURCE)
                .collect();
            assert_eq!(src_f.len(), 2);
            let p0 = decode_chunk_payload(src_f[0]).unwrap();
            let p1 = decode_chunk_payload(src_f[1]).unwrap();
            assert_eq!(p0.len(), split);
            assert_eq!([p0.as_slice(), p1.as_slice()].concat(), body);
            if c != codec::NONE {
                assert_ne!(src_f[0].payload, p0.as_slice());
            }
            assert!(decode_source_body(&p0).is_err());
            assert!(decode_source_body(&p1).is_err());
        }
    }

    #[test]
    fn mid_record_source_invalid_split_err() {
        let sources = sample_sources();
        let body = encode_source_body(&sources);
        match encode_mid_record_span_source_profile(
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
        ) {
            Err(CompressedProfileError::SourceBody(_)) => {}
            other => panic!("expected invalid split, got {other:?}"),
        }
        match encode_mid_record_span_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &sources,
            body.len(),
            None,
        ) {
            Err(CompressedProfileError::SourceBody(_)) => {}
            other => panic!("expected invalid split end, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_source_truncated_joined_body_err() {
        let sources = sample_sources();
        let split = sample_source_split_at();
        let mut enc = encode_mid_record_span_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &sources,
            split,
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        let only_first = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        enc.truncate(only_first);
        match decode_mid_record_span_source_profile(&enc) {
            Err(CompressedProfileError::SourceBody(_)) => {}
            other => panic!("expected truncated mid-record / need ≥2 chunks, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_source_corrupt_zlib_second_chunk_err() {
        let sources = sample_sources();
        let split = sample_source_split_at();
        let mut enc = encode_mid_record_span_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &sources,
            split,
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        let second_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let f1 = parse_chunk_frame(&enc[second_off..]).unwrap();
        assert_eq!(f1.codec, codec::ZLIB);
        assert_eq!(f1.kind, kind::SOURCE);
        let payload_len = f1.payload.len();
        let payload_off = second_off + CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        if payload_len > 1 {
            enc[payload_off + 1] ^= 0xAA;
        }
        match decode_mid_record_span_source_profile(&enc) {
            Err(CompressedProfileError::Payload(_)) => {}
            other => panic!("expected payload error on corrupt continuation, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_source_zlib_wire_uses_deflate() {
        let sources = sample_sources();
        let split = sample_source_split_at();
        let body = encode_source_body(&sources);
        let head = &body[..split];
        let expected = deflate_zlib(head).unwrap();
        let enc = encode_mid_record_span_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &sources,
            split,
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::SOURCE);
        assert_eq!(f0.payload, expected.as_slice());
    }

    #[test]
    fn mid_record_source_never_panic_garbage() {
        assert!(decode_mid_record_span_source_profile(&[]).is_err());
        assert!(decode_mid_record_span_source_profile(b"nope").is_err());
        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_mid_record_span_source_profile(&enc) {
            Err(CompressedProfileError::Stream(StreamError::Chunk(ChunkError::BadSync {
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
    fn empty_sources_encode_err() {
        match encode_mid_record_span_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &[],
            1,
            None,
        ) {
            Err(CompressedProfileError::SourceBody(_)) => {}
            other => panic!("expected empty sources err, got {other:?}"),
        }
    }

    // --- INDEX mid-record span ---

    fn sample_indexes() -> [IndexRecordSpec<'static>; 3] {
        [
            IndexRecordSpec {
                key_id: 1,
                file_offset: 100,
                length: 50,
                string_id: 0,
                string_flags: 0,
                label: b"first-index-entry",
            },
            IndexRecordSpec {
                key_id: 2,
                file_offset: 200,
                length: 80,
                string_id: 1,
                string_flags: 0,
                label: b"second-index-entry-longer",
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

    fn assert_sample_indexes(recs: &[OwnedIndexRecord]) {
        assert_eq!(recs.len(), 3);
        assert_eq!(recs[0].key_id, 1);
        assert_eq!(recs[0].file_offset, 100);
        assert_eq!(recs[0].length, 50);
        assert_eq!(recs[0].label, b"first-index-entry");
        assert_eq!(recs[1].key_id, 2);
        assert_eq!(recs[1].file_offset, 200);
        assert_eq!(recs[1].length, 80);
        assert_eq!(recs[1].label, b"second-index-entry-longer");
        assert_eq!(recs[2].key_id, 3);
        assert_eq!(recs[2].file_offset, 300);
        assert_eq!(recs[2].length, 10);
        assert_eq!(recs[2].label, b"third");
    }

    fn sample_index_split_at() -> usize {
        let body = encode_index_body(&sample_indexes());
        let split = default_mid_body_split(&body).expect("body large enough");
        assert!(split > 0 && split < body.len());
        assert!(decode_index_body(&body[..split]).is_err());
        split
    }

    #[test]
    fn split_index_body_bytes_rejects_edges() {
        let body = encode_index_body(&sample_indexes());
        assert!(split_index_body_bytes(&body, 0).is_none());
        assert!(split_index_body_bytes(&body, body.len()).is_none());
        let (a, b) = split_index_body_bytes(&body, 1).unwrap();
        assert_eq!(a.len() + b.len(), body.len());
    }

    #[test]
    fn mid_record_index_none_zlib_zstd_lz4_roundtrip() {
        let indexes = sample_indexes();
        let split = sample_index_split_at();
        let body = encode_index_body(&indexes);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let enc_a = encode_mid_record_span_index_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &indexes,
                split,
                Some(b"idx-end"),
            )
            .expect("encode");
            let enc_b = encode_mid_record_span_index_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &indexes,
                split,
                Some(b"idx-end"),
            )
            .expect("encode 2");
            assert_eq!(enc_a, enc_b, "deterministic codec {c}");
            assert_eq!(&enc_a[..8], MAGIC.as_slice());

            let (prof, n) = decode_mid_record_span_index_profile(&enc_a).expect("decode");
            assert_eq!(n, enc_a.len());
            assert_eq!(prof.index_codec, c);
            assert_eq!(prof.index_chunk_count, 2);
            assert_eq!(prof.split_at, split);
            assert_sample_indexes(&prof.records);
            assert!(prof.has_footer);
            assert_eq!(prof.footer_payload.as_deref(), Some(&b"idx-end"[..]));

            let (stream, _) = decode_prefix_chunk_stream(&enc_a).unwrap();
            let idx_f: Vec<_> = stream
                .chunks
                .iter()
                .filter(|f| f.kind == kind::INDEX)
                .collect();
            assert_eq!(idx_f.len(), 2);
            let p0 = decode_chunk_payload(idx_f[0]).unwrap();
            let p1 = decode_chunk_payload(idx_f[1]).unwrap();
            assert_eq!(p0.len(), split);
            assert_eq!([p0.as_slice(), p1.as_slice()].concat(), body);
            if c != codec::NONE {
                assert_ne!(idx_f[0].payload, p0.as_slice());
            }
            assert!(decode_index_body(&p0).is_err());
            assert!(decode_index_body(&p1).is_err());
        }
    }

    #[test]
    fn mid_record_index_invalid_split_err() {
        let indexes = sample_indexes();
        let body = encode_index_body(&indexes);
        match encode_mid_record_span_index_profile(
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
        ) {
            Err(CompressedProfileError::IndexBody(_)) => {}
            other => panic!("expected invalid split, got {other:?}"),
        }
        match encode_mid_record_span_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &indexes,
            body.len(),
            None,
        ) {
            Err(CompressedProfileError::IndexBody(_)) => {}
            other => panic!("expected invalid split end, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_index_truncated_joined_body_err() {
        let indexes = sample_indexes();
        let split = sample_index_split_at();
        let mut enc = encode_mid_record_span_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &indexes,
            split,
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        let only_first = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        enc.truncate(only_first);
        match decode_mid_record_span_index_profile(&enc) {
            Err(CompressedProfileError::IndexBody(_)) => {}
            other => panic!("expected truncated mid-record / need ≥2 chunks, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_index_corrupt_zlib_second_chunk_err() {
        let indexes = sample_indexes();
        let split = sample_index_split_at();
        let mut enc = encode_mid_record_span_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &indexes,
            split,
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        let second_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let f1 = parse_chunk_frame(&enc[second_off..]).unwrap();
        assert_eq!(f1.codec, codec::ZLIB);
        assert_eq!(f1.kind, kind::INDEX);
        let payload_len = f1.payload.len();
        let payload_off = second_off + CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        if payload_len > 1 {
            enc[payload_off + 1] ^= 0xAA;
        }
        match decode_mid_record_span_index_profile(&enc) {
            Err(CompressedProfileError::Payload(_)) => {}
            other => panic!("expected payload error on corrupt continuation, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_index_zlib_wire_uses_deflate() {
        let indexes = sample_indexes();
        let split = sample_index_split_at();
        let body = encode_index_body(&indexes);
        let head = &body[..split];
        let expected = deflate_zlib(head).unwrap();
        let enc = encode_mid_record_span_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &indexes,
            split,
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::INDEX);
        assert_eq!(f0.payload, expected.as_slice());
    }

    #[test]
    fn mid_record_index_never_panic_garbage() {
        assert!(decode_mid_record_span_index_profile(&[]).is_err());
        assert!(decode_mid_record_span_index_profile(b"nope").is_err());
        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_mid_record_span_index_profile(&enc) {
            Err(CompressedProfileError::Stream(StreamError::Chunk(ChunkError::BadSync {
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
    fn empty_indexes_encode_err() {
        match encode_mid_record_span_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &[],
            1,
            None,
        ) {
            Err(CompressedProfileError::IndexBody(_)) => {}
            other => panic!("expected empty indexes err, got {other:?}"),
        }
    }

    // --- SUMMARY mid-record span ---

    fn sample_summaries() -> [SummaryRecordSpec<'static>; 3] {
        [
            SummaryRecordSpec {
                key_id: 1,
                count: 10,
                value: 100,
                string_id: 0,
                string_flags: 0,
                label: b"first-summary-entry",
            },
            SummaryRecordSpec {
                key_id: 2,
                count: 20,
                value: 200,
                string_id: 1,
                string_flags: 0,
                label: b"second-summary-entry-longer",
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

    fn assert_sample_summaries(recs: &[OwnedSummaryRecord]) {
        assert_eq!(recs.len(), 3);
        assert_eq!(recs[0].key_id, 1);
        assert_eq!(recs[0].count, 10);
        assert_eq!(recs[0].value, 100);
        assert_eq!(recs[0].label, b"first-summary-entry");
        assert_eq!(recs[1].key_id, 2);
        assert_eq!(recs[1].count, 20);
        assert_eq!(recs[1].value, 200);
        assert_eq!(recs[1].label, b"second-summary-entry-longer");
        assert_eq!(recs[2].key_id, 3);
        assert_eq!(recs[2].count, 5);
        assert_eq!(recs[2].value, 50);
        assert_eq!(recs[2].label, b"third");
    }

    fn sample_summary_split_at() -> usize {
        let body = encode_summary_body(&sample_summaries());
        let split = default_mid_body_split(&body).expect("body large enough");
        assert!(split > 0 && split < body.len());
        assert!(decode_summary_body(&body[..split]).is_err());
        split
    }

    #[test]
    fn split_summary_body_bytes_rejects_edges() {
        let body = encode_summary_body(&sample_summaries());
        assert!(split_summary_body_bytes(&body, 0).is_none());
        assert!(split_summary_body_bytes(&body, body.len()).is_none());
        let (a, b) = split_summary_body_bytes(&body, 1).unwrap();
        assert_eq!(a.len() + b.len(), body.len());
    }

    #[test]
    fn mid_record_summary_none_zlib_zstd_lz4_roundtrip() {
        let summaries = sample_summaries();
        let split = sample_summary_split_at();
        let body = encode_summary_body(&summaries);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let enc_a = encode_mid_record_span_summary_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &summaries,
                split,
                Some(b"sum-end"),
            )
            .expect("encode");
            let enc_b = encode_mid_record_span_summary_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &summaries,
                split,
                Some(b"sum-end"),
            )
            .expect("encode 2");
            assert_eq!(enc_a, enc_b, "deterministic codec {c}");
            assert_eq!(&enc_a[..8], MAGIC.as_slice());

            let (prof, n) = decode_mid_record_span_summary_profile(&enc_a).expect("decode");
            assert_eq!(n, enc_a.len());
            assert_eq!(prof.summary_codec, c);
            assert_eq!(prof.summary_chunk_count, 2);
            assert_eq!(prof.split_at, split);
            assert_sample_summaries(&prof.records);
            assert!(prof.has_footer);
            assert_eq!(prof.footer_payload.as_deref(), Some(&b"sum-end"[..]));

            let (stream, _) = decode_prefix_chunk_stream(&enc_a).unwrap();
            let sum_f: Vec<_> = stream
                .chunks
                .iter()
                .filter(|f| f.kind == kind::SUMMARY)
                .collect();
            assert_eq!(sum_f.len(), 2);
            let p0 = decode_chunk_payload(sum_f[0]).unwrap();
            let p1 = decode_chunk_payload(sum_f[1]).unwrap();
            assert_eq!(p0.len(), split);
            assert_eq!([p0.as_slice(), p1.as_slice()].concat(), body);
            if c != codec::NONE {
                assert_ne!(sum_f[0].payload, p0.as_slice());
            }
            assert!(decode_summary_body(&p0).is_err());
            assert!(decode_summary_body(&p1).is_err());
        }
    }

    #[test]
    fn mid_record_summary_invalid_split_err() {
        let summaries = sample_summaries();
        let body = encode_summary_body(&summaries);
        match encode_mid_record_span_summary_profile(
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
        ) {
            Err(CompressedProfileError::SummaryBody(_)) => {}
            other => panic!("expected invalid split, got {other:?}"),
        }
        match encode_mid_record_span_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &summaries,
            body.len(),
            None,
        ) {
            Err(CompressedProfileError::SummaryBody(_)) => {}
            other => panic!("expected invalid split end, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_summary_truncated_joined_body_err() {
        let summaries = sample_summaries();
        let split = sample_summary_split_at();
        let mut enc = encode_mid_record_span_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &summaries,
            split,
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        let only_first = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        enc.truncate(only_first);
        match decode_mid_record_span_summary_profile(&enc) {
            Err(CompressedProfileError::SummaryBody(_)) => {}
            other => panic!("expected truncated mid-record / need ≥2 chunks, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_summary_corrupt_zlib_second_chunk_err() {
        let summaries = sample_summaries();
        let split = sample_summary_split_at();
        let mut enc = encode_mid_record_span_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &summaries,
            split,
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        let second_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let f1 = parse_chunk_frame(&enc[second_off..]).unwrap();
        assert_eq!(f1.codec, codec::ZLIB);
        assert_eq!(f1.kind, kind::SUMMARY);
        let payload_len = f1.payload.len();
        let payload_off = second_off + CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        if payload_len > 1 {
            enc[payload_off + 1] ^= 0xAA;
        }
        match decode_mid_record_span_summary_profile(&enc) {
            Err(CompressedProfileError::Payload(_)) => {}
            other => panic!("expected payload error on corrupt continuation, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_summary_zlib_wire_uses_deflate() {
        let summaries = sample_summaries();
        let split = sample_summary_split_at();
        let body = encode_summary_body(&summaries);
        let head = &body[..split];
        let expected = deflate_zlib(head).unwrap();
        let enc = encode_mid_record_span_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &summaries,
            split,
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::SUMMARY);
        assert_eq!(f0.payload, expected.as_slice());
    }

    #[test]
    fn mid_record_summary_never_panic_garbage() {
        assert!(decode_mid_record_span_summary_profile(&[]).is_err());
        assert!(decode_mid_record_span_summary_profile(b"nope").is_err());
        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_mid_record_span_summary_profile(&enc) {
            Err(CompressedProfileError::Stream(StreamError::Chunk(ChunkError::BadSync {
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
    fn empty_summaries_encode_err() {
        match encode_mid_record_span_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &[],
            1,
            None,
        ) {
            Err(CompressedProfileError::SummaryBody(_)) => {}
            other => panic!("expected empty summaries err, got {other:?}"),
        }
    }
}

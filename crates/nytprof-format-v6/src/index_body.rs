//! Provisional **format v6** INDEX chunk body (codec NONE) (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-index-body-provisional-v0.md`
//!
//! Ordered index records: ULEB128 key_id + file_offset + length + optional
//! length-prefixed string-blob label. Composes shipped varint + string-blob.
//! Optional mixed profile with EVENT + SOURCE + INDEX + SUMMARY + FOOTER.
//! No inflate, no full index catalog, no C writer.

use crate::chunk::{codec, kind};
use crate::event_body::{
    decode_event_body, encode_event_body, EventBodyError, EventRecord, EventRecordSpec,
};
use crate::file_prefix::FilePrefix;
use crate::source_body::{
    decode_source_body, encode_source_body, SourceBodyError, SourceRecord, SourceRecordSpec,
};
use crate::stream::{
    decode_prefix_chunk_stream, encode_prefix_chunk_stream, ChunkSpec, StreamError,
};
use crate::string::{decode_string_blob, encode_string_blob, StringBlob, StringError};
use crate::footer_body::{
    decode_footer_body, encode_footer_body, FooterBodyError, FooterRecord, FooterRecordSpec,
};
use crate::summary_body::{
    decode_summary_body, encode_summary_body, SummaryBodyError, SummaryRecord, SummaryRecordSpec,
};
use crate::varint::{decode_u64, encode_u64, VarintError};

/// Fail-closed upper bound on total INDEX body size (64 MiB).
pub const MAX_INDEX_BODY_BYTES: usize = 64 * 1024 * 1024;

/// One decoded INDEX record (label borrowed from input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRecord<'a> {
    /// Provisional key (e.g. fid or sub id).
    pub key_id: u64,
    /// Byte offset into the profile file (provisional semantic).
    pub file_offset: u64,
    /// Byte length or count of the referenced span (provisional).
    pub length: u64,
    /// Optional human label (may be empty).
    pub label: StringBlob<'a>,
}

/// Spec for encoding one INDEX record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexRecordSpec<'a> {
    pub key_id: u64,
    pub file_offset: u64,
    pub length: u64,
    pub string_id: u64,
    pub string_flags: u8,
    pub label: &'a [u8],
}

/// Fail-closed INDEX-body errors.
#[derive(Debug, PartialEq, Eq)]
pub enum IndexBodyError {
    Varint(VarintError),
    String(StringError),
    Truncated { need: usize, got: usize },
    Oversize { len: usize },
}

impl std::fmt::Display for IndexBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexBodyError::Varint(e) => write!(f, "index-body varint: {e}"),
            IndexBodyError::String(e) => write!(f, "index-body string: {e}"),
            IndexBodyError::Truncated { need, got } => {
                write!(f, "truncated index-body: need {need} bytes, got {got}")
            }
            IndexBodyError::Oversize { len } => {
                write!(
                    f,
                    "oversize index-body {len} bytes (max {MAX_INDEX_BODY_BYTES})"
                )
            }
        }
    }
}

impl std::error::Error for IndexBodyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IndexBodyError::Varint(e) => Some(e),
            IndexBodyError::String(e) => Some(e),
            _ => None,
        }
    }
}

impl From<VarintError> for IndexBodyError {
    fn from(e: VarintError) -> Self {
        IndexBodyError::Varint(e)
    }
}

impl From<StringError> for IndexBodyError {
    fn from(e: StringError) -> Self {
        IndexBodyError::String(e)
    }
}

pub type IndexBodyResult<T> = std::result::Result<T, IndexBodyError>;

/// Encode a provisional INDEX-body (codec NONE chunk payload).
///
/// Each record:
/// `ULEB128 key_id || ULEB128 file_offset || ULEB128 length || string_blob(id, flags, label)`.
/// Empty `records` → empty body. Pure byte-slice / `Vec` API.
pub fn encode_index_body(records: &[IndexRecordSpec<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    for rec in records {
        out.extend_from_slice(&encode_u64(rec.key_id));
        out.extend_from_slice(&encode_u64(rec.file_offset));
        out.extend_from_slice(&encode_u64(rec.length));
        out.extend_from_slice(&encode_string_blob(
            rec.string_id,
            rec.string_flags,
            rec.label,
        ));
    }
    out
}

/// Decode a provisional INDEX-body until the buffer is exhausted.
///
/// Empty input → empty list. Fail-closed on truncated mid-record or oversize.
/// Returns `(records, bytes_consumed)` (`bytes_consumed == data.len()` on success).
pub fn decode_index_body(data: &[u8]) -> IndexBodyResult<(Vec<IndexRecord<'_>>, usize)> {
    if data.len() > MAX_INDEX_BODY_BYTES {
        return Err(IndexBodyError::Oversize { len: data.len() });
    }
    let mut pos = 0;
    let mut out = Vec::new();
    while pos < data.len() {
        if pos > MAX_INDEX_BODY_BYTES {
            return Err(IndexBodyError::Oversize { len: pos });
        }
        let (key_id, n1) = decode_u64(data, pos)?;
        pos += n1;
        let (file_offset, n2) = decode_u64(data, pos)?;
        pos += n2;
        let (length, n3) = decode_u64(data, pos)?;
        pos += n3;
        let (label, n4) = decode_string_blob(data, pos)?;
        pos += n4;
        out.push(IndexRecord {
            key_id,
            file_offset,
            length,
            label,
        });
    }
    Ok((out, pos))
}

// --- Mixed EVENT + SOURCE + INDEX + SUMMARY + FOOTER-body composition (codec NONE) ---

/// Fail-closed mixed profile errors (EVENT + SOURCE + INDEX + SUMMARY + FOOTER-body).
#[derive(Debug, PartialEq, Eq)]
pub enum MixedProfileError {
    Stream(StreamError),
    EventBody(EventBodyError),
    SourceBody(SourceBodyError),
    IndexBody(IndexBodyError),
    SummaryBody(SummaryBodyError),
    FooterBody(FooterBodyError),
    UnexpectedCodec { codec: u8 },
    UnexpectedKind { kind: u8 },
    InvalidFooter,
}

impl std::fmt::Display for MixedProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MixedProfileError::Stream(e) => write!(f, "mixed profile stream: {e}"),
            MixedProfileError::EventBody(e) => write!(f, "mixed profile event-body: {e}"),
            MixedProfileError::SourceBody(e) => write!(f, "mixed profile source-body: {e}"),
            MixedProfileError::IndexBody(e) => write!(f, "mixed profile index-body: {e}"),
            MixedProfileError::SummaryBody(e) => write!(f, "mixed profile summary-body: {e}"),
            MixedProfileError::FooterBody(e) => write!(f, "mixed profile footer-body: {e}"),
            MixedProfileError::UnexpectedCodec { codec } => {
                write!(f, "mixed profile unexpected codec {codec} (NONE required)")
            }
            MixedProfileError::UnexpectedKind { kind } => {
                write!(f, "mixed profile unexpected chunk kind {kind}")
            }
            MixedProfileError::InvalidFooter => write!(f, "mixed profile invalid FOOTER placement"),
        }
    }
}

impl std::error::Error for MixedProfileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MixedProfileError::Stream(e) => Some(e),
            MixedProfileError::EventBody(e) => Some(e),
            MixedProfileError::SourceBody(e) => Some(e),
            MixedProfileError::IndexBody(e) => Some(e),
            MixedProfileError::SummaryBody(e) => Some(e),
            MixedProfileError::FooterBody(e) => Some(e),
            _ => None,
        }
    }
}

impl From<StreamError> for MixedProfileError {
    fn from(e: StreamError) -> Self {
        MixedProfileError::Stream(e)
    }
}

impl From<EventBodyError> for MixedProfileError {
    fn from(e: EventBodyError) -> Self {
        MixedProfileError::EventBody(e)
    }
}

impl From<SourceBodyError> for MixedProfileError {
    fn from(e: SourceBodyError) -> Self {
        MixedProfileError::SourceBody(e)
    }
}

impl From<IndexBodyError> for MixedProfileError {
    fn from(e: IndexBodyError) -> Self {
        MixedProfileError::IndexBody(e)
    }
}

impl From<SummaryBodyError> for MixedProfileError {
    fn from(e: SummaryBodyError) -> Self {
        MixedProfileError::SummaryBody(e)
    }
}

impl From<FooterBodyError> for MixedProfileError {
    fn from(e: FooterBodyError) -> Self {
        MixedProfileError::FooterBody(e)
    }
}

pub type MixedProfileResult<T> = std::result::Result<T, MixedProfileError>;

/// Decoded mixed profile: prefix + EVENT + SOURCE + INDEX + SUMMARY + optional FOOTER-body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixedKindProfile<'a> {
    pub prefix: FilePrefix<'a>,
    pub event_records: Vec<EventRecord<'a>>,
    pub source_records: Vec<SourceRecord<'a>>,
    pub index_records: Vec<IndexRecord<'a>>,
    pub summary_records: Vec<SummaryRecord<'a>>,
    pub footer_records: Vec<FooterRecord<'a>>,
    pub event_chunk_count: usize,
    pub source_chunk_count: usize,
    pub index_chunk_count: usize,
    pub summary_chunk_count: usize,
    pub has_footer: bool,
    /// Raw FOOTER payload bytes when present (empty body is valid).
    pub footer_payload: Option<&'a [u8]>,
}

/// Encode prefix + optional EVENT + SOURCE + INDEX + SUMMARY + optional FOOTER-body.
///
/// Wire order when non-empty: EVENT, SOURCE, INDEX, SUMMARY, then optional FOOTER last.
/// `footer`: `None` = no FOOTER chunk; `Some(records)` = FOOTER with `encode_footer_body`
/// (empty slice → empty FOOTER payload, still last).
/// Composes shipped body encode APIs + `encode_prefix_chunk_stream`.
pub fn encode_mixed_kind_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> Vec<u8> {
    let event_body = if events.is_empty() {
        None
    } else {
        Some(encode_event_body(events))
    };
    let source_body = if sources.is_empty() {
        None
    } else {
        Some(encode_source_body(sources))
    };
    let index_body = if indexes.is_empty() {
        None
    } else {
        Some(encode_index_body(indexes))
    };
    let summary_body = if summaries.is_empty() {
        None
    } else {
        Some(encode_summary_body(summaries))
    };

    let mut chunks: Vec<ChunkSpec<'_>> = Vec::new();
    if let Some(ref body) = event_body {
        chunks.push(ChunkSpec {
            kind: kind::EVENT,
            codec: codec::NONE,
            flags: 0,
            sequence: 0,
            first_logical_seq: 0,
            logical_event_count: events.len() as u32,
            uncompressed_len: body.len() as u32,
            payload: body.as_slice(),
            payload_checksum: 0,
        });
    }
    if let Some(ref body) = source_body {
        let seq = chunks.len() as u64;
        chunks.push(ChunkSpec {
            kind: kind::SOURCE,
            codec: codec::NONE,
            flags: 0,
            sequence: seq,
            first_logical_seq: 0,
            logical_event_count: sources.len() as u32,
            uncompressed_len: body.len() as u32,
            payload: body.as_slice(),
            payload_checksum: 0,
        });
    }
    if let Some(ref body) = index_body {
        let seq = chunks.len() as u64;
        chunks.push(ChunkSpec {
            kind: kind::INDEX,
            codec: codec::NONE,
            flags: 0,
            sequence: seq,
            first_logical_seq: 0,
            logical_event_count: indexes.len() as u32,
            uncompressed_len: body.len() as u32,
            payload: body.as_slice(),
            payload_checksum: 0,
        });
    }
    if let Some(ref body) = summary_body {
        let seq = chunks.len() as u64;
        chunks.push(ChunkSpec {
            kind: kind::SUMMARY,
            codec: codec::NONE,
            flags: 0,
            sequence: seq,
            first_logical_seq: 0,
            logical_event_count: summaries.len() as u32,
            uncompressed_len: body.len() as u32,
            payload: body.as_slice(),
            payload_checksum: 0,
        });
    }

    let footer_owned = footer.map(|recs| encode_footer_body(recs));
    if let Some(ref fp) = footer_owned {
        let seq = chunks.len() as u64;
        let footer_rec_count = footer.map(|r| r.len() as u32).unwrap_or(0);
        chunks.push(ChunkSpec {
            kind: kind::FOOTER,
            codec: codec::NONE,
            flags: 0,
            sequence: seq,
            first_logical_seq: 0,
            logical_event_count: footer_rec_count,
            uncompressed_len: fp.len() as u32,
            payload: fp.as_slice(),
            payload_checksum: 0,
        });
    }

    encode_prefix_chunk_stream(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        &chunks,
    )
}

/// Decode a mixed EVENT + SOURCE + INDEX + SUMMARY + FOOTER-body profile (codec NONE only).
///
/// Allows EVENT / SOURCE / INDEX / SUMMARY in any order before a trailing FOOTER.
/// FOOTER payload is decoded via `decode_footer_body` (empty body → empty records).
/// Fail-closed on bad magic, truncated mid-chunk, bad sync, truncated body,
/// unexpected kind/codec, or FOOTER not last.
pub fn decode_mixed_kind_profile(
    buf: &[u8],
) -> MixedProfileResult<(MixedKindProfile<'_>, usize)> {
    let (stream, n) = decode_prefix_chunk_stream(buf)?;
    let mut event_records = Vec::new();
    let mut source_records = Vec::new();
    let mut index_records = Vec::new();
    let mut summary_records = Vec::new();
    let mut footer_records = Vec::new();
    let mut event_chunk_count = 0usize;
    let mut source_chunk_count = 0usize;
    let mut index_chunk_count = 0usize;
    let mut summary_chunk_count = 0usize;
    let mut has_footer = false;
    let mut footer_payload: Option<&[u8]> = None;
    let mut saw_footer = false;

    for frame in &stream.chunks {
        if saw_footer {
            return Err(MixedProfileError::InvalidFooter);
        }
        if frame.codec != codec::NONE {
            return Err(MixedProfileError::UnexpectedCodec {
                codec: frame.codec,
            });
        }
        match frame.kind {
            k if k == kind::EVENT => {
                let (recs, body_n) = decode_event_body(frame.payload)?;
                if body_n != frame.payload.len() {
                    return Err(MixedProfileError::EventBody(EventBodyError::Truncated {
                        need: frame.payload.len(),
                        got: body_n,
                    }));
                }
                event_records.extend(recs);
                event_chunk_count += 1;
            }
            k if k == kind::SOURCE => {
                let (recs, body_n) = decode_source_body(frame.payload)?;
                if body_n != frame.payload.len() {
                    return Err(MixedProfileError::SourceBody(SourceBodyError::Truncated {
                        need: frame.payload.len(),
                        got: body_n,
                    }));
                }
                source_records.extend(recs);
                source_chunk_count += 1;
            }
            k if k == kind::INDEX => {
                let (recs, body_n) = decode_index_body(frame.payload)?;
                if body_n != frame.payload.len() {
                    return Err(MixedProfileError::IndexBody(IndexBodyError::Truncated {
                        need: frame.payload.len(),
                        got: body_n,
                    }));
                }
                index_records.extend(recs);
                index_chunk_count += 1;
            }
            k if k == kind::SUMMARY => {
                let (recs, body_n) = decode_summary_body(frame.payload)?;
                if body_n != frame.payload.len() {
                    return Err(MixedProfileError::SummaryBody(SummaryBodyError::Truncated {
                        need: frame.payload.len(),
                        got: body_n,
                    }));
                }
                summary_records.extend(recs);
                summary_chunk_count += 1;
            }
            k if k == kind::FOOTER => {
                let (recs, body_n) = decode_footer_body(frame.payload)?;
                if body_n != frame.payload.len() {
                    return Err(MixedProfileError::FooterBody(FooterBodyError::Truncated {
                        need: frame.payload.len(),
                        got: body_n,
                    }));
                }
                footer_records = recs;
                has_footer = true;
                footer_payload = Some(frame.payload);
                saw_footer = true;
            }
            other => {
                return Err(MixedProfileError::UnexpectedKind { kind: other });
            }
        }
    }

    Ok((
        MixedKindProfile {
            prefix: stream.prefix,
            event_records,
            source_records,
            index_records,
            summary_records,
            footer_records,
            event_chunk_count,
            source_chunk_count,
            index_chunk_count,
            summary_chunk_count,
            has_footer,
            footer_payload,
        },
        n,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{encode_chunk_frame, CHUNK_HEADER_LEN};
    use crate::encode_file_prefix;
    use crate::stream::StreamError;
    use crate::string::FLAG_UTF8;
    use crate::footer_body::{encode_footer_body, FooterRecordSpec};
    use crate::summary_body::{encode_summary_body, SummaryRecordSpec};
    use crate::{FilePrefixError, MAGIC, SUPPORTED_MAJOR};
    use crate::Error as HeaderError;

    #[test]
    fn empty_index_body_roundtrip() {
        let enc_a = encode_index_body(&[]);
        let enc_b = encode_index_body(&[]);
        assert_eq!(enc_a, enc_b);
        assert!(enc_a.is_empty());
        let (recs, n) = decode_index_body(&enc_a).expect("empty");
        assert_eq!(n, 0);
        assert!(recs.is_empty());
    }

    #[test]
    fn index_records_roundtrip() {
        let specs = [
            IndexRecordSpec {
                key_id: 1,
                file_offset: 100,
                length: 50,
                string_id: 0,
                string_flags: FLAG_UTF8,
                label: b"main::leaf",
            },
            IndexRecordSpec {
                key_id: 2,
                file_offset: 200,
                length: 0,
                string_id: 0,
                string_flags: 0,
                label: b"",
            },
        ];
        let enc = encode_index_body(&specs);
        let mut expect = Vec::new();
        expect.extend_from_slice(&encode_u64(1));
        expect.extend_from_slice(&encode_u64(100));
        expect.extend_from_slice(&encode_u64(50));
        expect.extend_from_slice(&encode_string_blob(0, FLAG_UTF8, b"main::leaf"));
        expect.extend_from_slice(&encode_u64(2));
        expect.extend_from_slice(&encode_u64(200));
        expect.extend_from_slice(&encode_u64(0));
        expect.extend_from_slice(&encode_string_blob(0, 0, b""));
        assert_eq!(enc, expect);

        let (recs, n) = decode_index_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].key_id, 1);
        assert_eq!(recs[0].file_offset, 100);
        assert_eq!(recs[0].length, 50);
        assert_eq!(recs[0].label.data, b"main::leaf");
        assert_eq!(recs[1].key_id, 2);
        assert!(recs[1].label.data.is_empty());
    }

    #[test]
    fn truncated_mid_record_err() {
        let full = encode_index_body(&[IndexRecordSpec {
            key_id: 1,
            file_offset: 10,
            length: 5,
            string_id: 0,
            string_flags: 0,
            label: b"hello",
        }]);
        let trunc = &full[..full.len() - 1];
        match decode_index_body(trunc) {
            Err(IndexBodyError::Varint(_))
            | Err(IndexBodyError::String(_))
            | Err(IndexBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid-record, got {other:?}"),
        }
    }

    #[test]
    fn truncated_after_key_err() {
        let partial = encode_u64(1);
        match decode_index_body(&partial) {
            Err(IndexBodyError::Varint(_)) | Err(IndexBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated after key, got {other:?}"),
        }
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_index_body(&[]).is_ok());
        let _ = decode_index_body(&[0xFF; 6]);
        let _ = decode_index_body(b"\x01\x02");
    }

    #[test]
    fn index_as_codec_none_chunk_payload() {
        let body = encode_index_body(&[IndexRecordSpec {
            key_id: 7,
            file_offset: 36,
            length: 8,
            string_id: 0,
            string_flags: 0,
            label: b"x",
        }]);
        let frame = encode_chunk_frame(
            kind::INDEX,
            codec::NONE,
            0,
            0,
            0,
            1,
            body.len() as u32,
            &body,
            0,
        );
        let parsed = crate::parse_chunk_frame(&frame).expect("chunk");
        assert_eq!(parsed.kind, kind::INDEX);
        assert_eq!(parsed.codec, codec::NONE);
        let (recs, n) = decode_index_body(parsed.payload).expect("body");
        assert_eq!(n, body.len());
        assert_eq!(recs[0].key_id, 7);
        assert_eq!(recs[0].label.data, b"x");
    }

    #[test]
    fn mixed_event_source_index_roundtrip() {
        let events = [EventRecordSpec::TimeLine {
            fid: 1,
            line: 5,
            ticks: 42,
        }];
        let sources = [SourceRecordSpec {
            fid: 1,
            line: 5,
            string_id: 0,
            string_flags: FLAG_UTF8,
            text: b"$x++",
        }];
        let indexes = [IndexRecordSpec {
            key_id: 1,
            file_offset: 80,
            length: 40,
            string_id: 0,
            string_flags: 0,
            label: b"fid1",
        }];
        let summaries = [SummaryRecordSpec {
            key_id: 1,
            count: 15,
            value: 1000,
            string_id: 0,
            string_flags: 0,
            label: b"leaf",
        }];
        let footers = [FooterRecordSpec {
            key_id: 1,
            value: 2474,
            string_id: 0,
            string_flags: 0,
            label: b"total_events",
        }];
        let enc_a = encode_mixed_kind_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            &events,
            &sources,
            &indexes,
            &summaries,
            Some(&footers),
        );
        let enc_b = encode_mixed_kind_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            &events,
            &sources,
            &indexes,
            &summaries,
            Some(&footers),
        );
        assert_eq!(enc_a, enc_b);
        assert_eq!(&enc_a[..8], MAGIC.as_slice());

        let ebody = encode_event_body(&events);
        let sbody = encode_source_body(&sources);
        let ibody = encode_index_body(&indexes);
        let sbody_sum = encode_summary_body(&summaries);
        let fbody = encode_footer_body(&footers);
        let eframe = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            1,
            ebody.len() as u32,
            &ebody,
            0,
        );
        let sframe = encode_chunk_frame(
            kind::SOURCE,
            codec::NONE,
            0,
            1,
            0,
            1,
            sbody.len() as u32,
            &sbody,
            0,
        );
        let iframe = encode_chunk_frame(
            kind::INDEX,
            codec::NONE,
            0,
            2,
            0,
            1,
            ibody.len() as u32,
            &ibody,
            0,
        );
        let sumframe = encode_chunk_frame(
            kind::SUMMARY,
            codec::NONE,
            0,
            3,
            0,
            1,
            sbody_sum.len() as u32,
            &sbody_sum,
            0,
        );
        let fframe = encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            4,
            0,
            1,
            fbody.len() as u32,
            &fbody,
            0,
        );
        let prefix = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        assert_eq!(
            enc_a.len(),
            prefix.len()
                + eframe.len()
                + sframe.len()
                + iframe.len()
                + sumframe.len()
                + fframe.len()
        );

        let (prof, n) = decode_mixed_kind_profile(&enc_a).expect("mixed");
        assert_eq!(n, enc_a.len());
        assert_eq!(
            (
                prof.event_chunk_count,
                prof.source_chunk_count,
                prof.index_chunk_count,
                prof.summary_chunk_count
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(prof.event_records.len(), 1);
        assert_eq!(prof.source_records[0].text.data, b"$x++");
        assert_eq!(prof.index_records[0].file_offset, 80);
        assert_eq!(prof.index_records[0].label.data, b"fid1");
        assert_eq!(prof.summary_records[0].count, 15);
        assert_eq!(prof.summary_records[0].label.data, b"leaf");
        assert!(prof.has_footer);
        assert_eq!(prof.footer_records.len(), 1);
        assert_eq!(prof.footer_records[0].value, 2474);
        assert_eq!(prof.footer_records[0].label.data, b"total_events");
        assert_eq!(prof.footer_payload, Some(fbody.as_slice()));
    }

    #[test]
    fn mixed_bad_magic_err() {
        let mut enc =
            encode_mixed_kind_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[], &[], &[], &[], None);
        enc[0] = b'X';
        assert_eq!(
            decode_mixed_kind_profile(&enc),
            Err(MixedProfileError::Stream(StreamError::Prefix(
                FilePrefixError::Header(HeaderError::BadMagic)
            )))
        );
    }

    #[test]
    fn mixed_truncated_mid_chunk_err() {
        let indexes = [IndexRecordSpec {
            key_id: 1,
            file_offset: 1,
            length: 1,
            string_id: 0,
            string_flags: 0,
            label: b"abcdefgh",
        }];
        let mut enc = encode_mixed_kind_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            &[],
            &[],
            &indexes,
            &[],
            None,
        );
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        enc.truncate(prefix_n + CHUNK_HEADER_LEN + 2);
        match decode_mixed_kind_profile(&enc) {
            Err(MixedProfileError::Stream(_)) => {}
            other => panic!("expected stream truncated, got {other:?}"),
        }
    }
}

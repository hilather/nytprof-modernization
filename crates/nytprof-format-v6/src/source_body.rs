//! Provisional **format v6** SOURCE chunk body (codec NONE) (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-source-body-provisional-v0.md`
//!
//! Ordered source records: ULEB128 fid + line + length-prefixed string-blob text.
//! Composes shipped varint + string-blob. Optional mixed profile with EVENT + SOURCE.
//! No inflate, no full source catalog, no C writer.

use crate::chunk::{codec, kind};
use crate::event_body::{
    decode_event_body, encode_event_body, EventBodyError, EventRecord, EventRecordSpec,
};
use crate::file_prefix::FilePrefix;
use crate::stream::{
    decode_prefix_chunk_stream, encode_prefix_chunk_stream, ChunkSpec, StreamError,
};
use crate::string::{decode_string_blob, encode_string_blob, StringBlob, StringError};
use crate::varint::{decode_u64, encode_u64, VarintError};

/// Fail-closed upper bound on total SOURCE body size (64 MiB).
pub const MAX_SOURCE_BODY_BYTES: usize = 64 * 1024 * 1024;

/// One decoded SOURCE record (text borrowed from input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRecord<'a> {
    pub fid: u64,
    pub line: u64,
    pub text: StringBlob<'a>,
}

/// Spec for encoding one SOURCE record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRecordSpec<'a> {
    pub fid: u64,
    pub line: u64,
    pub string_id: u64,
    pub string_flags: u8,
    pub text: &'a [u8],
}

/// Fail-closed SOURCE-body errors.
#[derive(Debug, PartialEq, Eq)]
pub enum SourceBodyError {
    Varint(VarintError),
    String(StringError),
    Truncated { need: usize, got: usize },
    Oversize { len: usize },
}

impl std::fmt::Display for SourceBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceBodyError::Varint(e) => write!(f, "source-body varint: {e}"),
            SourceBodyError::String(e) => write!(f, "source-body string: {e}"),
            SourceBodyError::Truncated { need, got } => {
                write!(f, "truncated source-body: need {need} bytes, got {got}")
            }
            SourceBodyError::Oversize { len } => {
                write!(
                    f,
                    "oversize source-body {len} bytes (max {MAX_SOURCE_BODY_BYTES})"
                )
            }
        }
    }
}

impl std::error::Error for SourceBodyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SourceBodyError::Varint(e) => Some(e),
            SourceBodyError::String(e) => Some(e),
            _ => None,
        }
    }
}

impl From<VarintError> for SourceBodyError {
    fn from(e: VarintError) -> Self {
        SourceBodyError::Varint(e)
    }
}

impl From<StringError> for SourceBodyError {
    fn from(e: StringError) -> Self {
        SourceBodyError::String(e)
    }
}

pub type SourceBodyResult<T> = std::result::Result<T, SourceBodyError>;

/// Encode a provisional SOURCE-body (codec NONE chunk payload).
///
/// Each record: `ULEB128 fid || ULEB128 line || string_blob(id, flags, text)`.
/// Empty `records` → empty body. Pure byte-slice / `Vec` API.
pub fn encode_source_body(records: &[SourceRecordSpec<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    for rec in records {
        out.extend_from_slice(&encode_u64(rec.fid));
        out.extend_from_slice(&encode_u64(rec.line));
        out.extend_from_slice(&encode_string_blob(
            rec.string_id,
            rec.string_flags,
            rec.text,
        ));
    }
    out
}

/// Decode a provisional SOURCE-body until the buffer is exhausted.
///
/// Empty input → empty list. Fail-closed on truncated mid-record or oversize.
/// Returns `(records, bytes_consumed)` (`bytes_consumed == data.len()` on success).
pub fn decode_source_body(data: &[u8]) -> SourceBodyResult<(Vec<SourceRecord<'_>>, usize)> {
    if data.len() > MAX_SOURCE_BODY_BYTES {
        return Err(SourceBodyError::Oversize { len: data.len() });
    }
    let mut pos = 0;
    let mut out = Vec::new();
    while pos < data.len() {
        if pos > MAX_SOURCE_BODY_BYTES {
            return Err(SourceBodyError::Oversize { len: pos });
        }
        let (fid, n1) = decode_u64(data, pos)?;
        pos += n1;
        let (line, n2) = decode_u64(data, pos)?;
        pos += n2;
        let (text, n3) = decode_string_blob(data, pos)?;
        pos += n3;
        out.push(SourceRecord { fid, line, text });
    }
    Ok((out, pos))
}

// --- Mixed EVENT + SOURCE profile composition (codec NONE) ---

/// Fail-closed mixed profile errors (EVENT + SOURCE + optional FOOTER).
#[derive(Debug, PartialEq, Eq)]
pub enum EventSourceProfileError {
    Stream(StreamError),
    EventBody(EventBodyError),
    SourceBody(SourceBodyError),
    UnexpectedCodec { codec: u8 },
    UnexpectedKind { kind: u8 },
    InvalidFooter,
}

impl std::fmt::Display for EventSourceProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventSourceProfileError::Stream(e) => write!(f, "event+source profile stream: {e}"),
            EventSourceProfileError::EventBody(e) => {
                write!(f, "event+source profile event-body: {e}")
            }
            EventSourceProfileError::SourceBody(e) => {
                write!(f, "event+source profile source-body: {e}")
            }
            EventSourceProfileError::UnexpectedCodec { codec } => {
                write!(f, "event+source profile unexpected codec {codec} (NONE required)")
            }
            EventSourceProfileError::UnexpectedKind { kind } => {
                write!(f, "event+source profile unexpected chunk kind {kind}")
            }
            EventSourceProfileError::InvalidFooter => {
                write!(f, "event+source profile invalid FOOTER placement")
            }
        }
    }
}

impl std::error::Error for EventSourceProfileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EventSourceProfileError::Stream(e) => Some(e),
            EventSourceProfileError::EventBody(e) => Some(e),
            EventSourceProfileError::SourceBody(e) => Some(e),
            _ => None,
        }
    }
}

impl From<StreamError> for EventSourceProfileError {
    fn from(e: StreamError) -> Self {
        EventSourceProfileError::Stream(e)
    }
}

impl From<EventBodyError> for EventSourceProfileError {
    fn from(e: EventBodyError) -> Self {
        EventSourceProfileError::EventBody(e)
    }
}

impl From<SourceBodyError> for EventSourceProfileError {
    fn from(e: SourceBodyError) -> Self {
        EventSourceProfileError::SourceBody(e)
    }
}

pub type EventSourceProfileResult<T> = std::result::Result<T, EventSourceProfileError>;

/// Decoded mixed profile: prefix + EVENT records + SOURCE records + optional FOOTER.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSourceProfile<'a> {
    pub prefix: FilePrefix<'a>,
    pub event_records: Vec<EventRecord<'a>>,
    pub source_records: Vec<SourceRecord<'a>>,
    pub event_chunk_count: usize,
    pub source_chunk_count: usize,
    pub has_footer: bool,
    pub footer_payload: Option<&'a [u8]>,
}

/// Encode prefix + optional EVENT (codec NONE event-body) + optional SOURCE
/// (codec NONE source-body) + optional FOOTER.
///
/// Order on the wire: all EVENT chunks first (one chunk when non-empty), then
/// one SOURCE chunk when non-empty, then optional FOOTER. Composes shipped
/// `encode_event_body` / `encode_source_body` + `encode_prefix_chunk_stream`.
pub fn encode_event_source_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    footer: Option<&[u8]>,
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

    let footer_owned = footer.map(|p| p.to_vec());
    if let Some(ref fp) = footer_owned {
        let seq = chunks.len() as u64;
        chunks.push(ChunkSpec {
            kind: kind::FOOTER,
            codec: codec::NONE,
            flags: 0,
            sequence: seq,
            first_logical_seq: 0,
            logical_event_count: 0,
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

/// Decode a mixed EVENT + SOURCE profile (codec NONE only).
///
/// Allows EVENT and SOURCE in any order before a trailing FOOTER; appends
/// records in chunk order. Fail-closed on bad magic, truncated mid-chunk,
/// bad sync, truncated body, unexpected kind/codec, or FOOTER not last.
pub fn decode_event_source_profile(
    buf: &[u8],
) -> EventSourceProfileResult<(EventSourceProfile<'_>, usize)> {
    let (stream, n) = decode_prefix_chunk_stream(buf)?;
    let mut event_records = Vec::new();
    let mut source_records = Vec::new();
    let mut event_chunk_count = 0usize;
    let mut source_chunk_count = 0usize;
    let mut has_footer = false;
    let mut footer_payload: Option<&[u8]> = None;
    let mut saw_footer = false;

    for frame in &stream.chunks {
        if saw_footer {
            return Err(EventSourceProfileError::InvalidFooter);
        }
        if frame.codec != codec::NONE {
            return Err(EventSourceProfileError::UnexpectedCodec {
                codec: frame.codec,
            });
        }
        match frame.kind {
            k if k == kind::EVENT => {
                let (recs, body_n) = decode_event_body(frame.payload)?;
                if body_n != frame.payload.len() {
                    return Err(EventSourceProfileError::EventBody(
                        EventBodyError::Truncated {
                            need: frame.payload.len(),
                            got: body_n,
                        },
                    ));
                }
                event_records.extend(recs);
                event_chunk_count += 1;
            }
            k if k == kind::SOURCE => {
                let (recs, body_n) = decode_source_body(frame.payload)?;
                if body_n != frame.payload.len() {
                    return Err(EventSourceProfileError::SourceBody(
                        SourceBodyError::Truncated {
                            need: frame.payload.len(),
                            got: body_n,
                        },
                    ));
                }
                source_records.extend(recs);
                source_chunk_count += 1;
            }
            k if k == kind::FOOTER => {
                has_footer = true;
                footer_payload = Some(frame.payload);
                saw_footer = true;
            }
            other => {
                return Err(EventSourceProfileError::UnexpectedKind { kind: other });
            }
        }
    }

    Ok((
        EventSourceProfile {
            prefix: stream.prefix,
            event_records,
            source_records,
            event_chunk_count,
            source_chunk_count,
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
    use crate::string::FLAG_UTF8;
    use crate::{MAGIC, SUPPORTED_MAJOR};
    use crate::FilePrefixError;
    use crate::Error as HeaderError;
    use crate::stream::StreamError;

    #[test]
    fn empty_source_body_roundtrip() {
        let enc_a = encode_source_body(&[]);
        let enc_b = encode_source_body(&[]);
        assert_eq!(enc_a, enc_b);
        assert!(enc_a.is_empty());
        let (recs, n) = decode_source_body(&enc_a).expect("empty");
        assert_eq!(n, 0);
        assert!(recs.is_empty());
        let (recs2, n2) = decode_source_body(&enc_a).unwrap();
        assert_eq!((n2, recs2.len()), (n, 0));
    }

    #[test]
    fn source_records_roundtrip() {
        let specs = [
            SourceRecordSpec {
                fid: 1,
                line: 5,
                string_id: 0,
                string_flags: FLAG_UTF8,
                text: b"    $x++ for 1 .. 50;\n",
            },
            SourceRecordSpec {
                fid: 1,
                line: 8,
                string_id: 0,
                string_flags: 0,
                text: b"sub mid {",
            },
        ];
        let enc = encode_source_body(&specs);
        // Length from composing primitives (no detached golden).
        let mut expect = Vec::new();
        expect.extend_from_slice(&encode_u64(1));
        expect.extend_from_slice(&encode_u64(5));
        expect.extend_from_slice(&encode_string_blob(0, FLAG_UTF8, specs[0].text));
        expect.extend_from_slice(&encode_u64(1));
        expect.extend_from_slice(&encode_u64(8));
        expect.extend_from_slice(&encode_string_blob(0, 0, specs[1].text));
        assert_eq!(enc, expect);

        let (recs, n) = decode_source_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].fid, 1);
        assert_eq!(recs[0].line, 5);
        assert_eq!(recs[0].text.data, b"    $x++ for 1 .. 50;\n");
        assert_eq!(recs[0].text.flags, FLAG_UTF8);
        assert_eq!(recs[1].line, 8);
        assert_eq!(recs[1].text.data, b"sub mid {");
    }

    #[test]
    fn truncated_mid_record_err() {
        let full = encode_source_body(&[SourceRecordSpec {
            fid: 1,
            line: 2,
            string_id: 0,
            string_flags: 0,
            text: b"hello",
        }]);
        assert!(full.len() > 2);
        let trunc = &full[..full.len() - 1];
        match decode_source_body(trunc) {
            Err(SourceBodyError::Varint(_))
            | Err(SourceBodyError::String(_))
            | Err(SourceBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid-record, got {other:?}"),
        }
    }

    #[test]
    fn truncated_after_fid_before_line_err() {
        // Only a fid varint, missing line + text.
        let partial = encode_u64(1);
        match decode_source_body(&partial) {
            Err(SourceBodyError::Varint(_)) | Err(SourceBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated after fid, got {other:?}"),
        }
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_source_body(&[]).is_ok());
        let _ = decode_source_body(&[0xFF; 4]);
        let _ = decode_source_body(b"\x01");
    }

    #[test]
    fn source_as_codec_none_chunk_payload() {
        let body = encode_source_body(&[SourceRecordSpec {
            fid: 2,
            line: 10,
            string_id: 0,
            string_flags: 0,
            text: b"x",
        }]);
        let frame = encode_chunk_frame(
            kind::SOURCE,
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
        assert_eq!(parsed.kind, kind::SOURCE);
        assert_eq!(parsed.codec, codec::NONE);
        let (recs, n) = decode_source_body(parsed.payload).expect("body");
        assert_eq!(n, body.len());
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].text.data, b"x");
    }

    #[test]
    fn event_source_profile_composition_roundtrip() {
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
        let enc_a = encode_event_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            &events,
            &sources,
            Some(b""),
        );
        let enc_b = encode_event_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            &events,
            &sources,
            Some(b""),
        );
        assert_eq!(enc_a, enc_b);
        assert_eq!(&enc_a[..8], MAGIC.as_slice());

        // Length = prefix + EVENT frame + SOURCE frame + FOOTER frame.
        let ebody = encode_event_body(&events);
        let sbody = encode_source_body(&sources);
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
        let fframe = encode_chunk_frame(kind::FOOTER, codec::NONE, 0, 2, 0, 0, 0, b"", 0);
        let prefix = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        assert_eq!(
            enc_a.len(),
            prefix.len() + eframe.len() + sframe.len() + fframe.len()
        );

        let (prof, n) = decode_event_source_profile(&enc_a).expect("mixed");
        assert_eq!(n, enc_a.len());
        assert_eq!(prof.event_chunk_count, 1);
        assert_eq!(prof.source_chunk_count, 1);
        assert_eq!(prof.event_records.len(), 1);
        assert_eq!(prof.source_records.len(), 1);
        match &prof.event_records[0] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 5, 42));
            }
            other => panic!("expected TimeLine, got {other:?}"),
        }
        assert_eq!(prof.source_records[0].text.data, b"$x++");
        assert!(prof.has_footer);
    }

    #[test]
    fn event_source_bad_magic_err() {
        let mut enc =
            encode_event_source_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[], &[], None);
        enc[0] = b'X';
        assert_eq!(
            decode_event_source_profile(&enc),
            Err(EventSourceProfileError::Stream(StreamError::Prefix(
                FilePrefixError::Header(HeaderError::BadMagic)
            )))
        );
    }

    #[test]
    fn event_source_truncated_mid_chunk_err() {
        let sources = [SourceRecordSpec {
            fid: 1,
            line: 1,
            string_id: 0,
            string_flags: 0,
            text: b"abcdefgh",
        }];
        let mut enc =
            encode_event_source_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[], &sources, None);
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        enc.truncate(prefix_n + CHUNK_HEADER_LEN + 2);
        match decode_event_source_profile(&enc) {
            Err(EventSourceProfileError::Stream(_)) => {}
            other => panic!("expected stream truncated, got {other:?}"),
        }
    }
}

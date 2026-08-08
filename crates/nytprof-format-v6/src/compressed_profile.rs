//! Provisional **format v6** compressed multi-codec mini-profile composition
//! (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-compressed-profile-provisional-v0.md`
//!
//! Composes file-prefix + event-body + payload codecs (NONE/ZLIB/ZSTD/LZ4) into:
//! `[file prefix][EVENT with chosen codec…][optional FOOTER codec NONE]`.
//! Default `parse_chunk_frame` stays non-inflating; decode uses
//! `decode_chunk_payload` explicitly. Not dictionaries, not COL-007 C writer.

use crate::chunk::{codec, encode_chunk_frame, kind, parse_chunk_frame, ChunkError};
use crate::crc::compute_payload_crc;
use crate::event_body::{
    decode_event_body, encode_event_body, EventBodyError, EventRecord, EventRecordSpec,
};
use crate::file_prefix::{encode_file_prefix, FilePrefixError};
use crate::payload_codec::{
    decode_chunk_payload, encode_chunk_frame_lz4, encode_chunk_frame_zlib,
    encode_chunk_frame_zstd, PayloadCodecError,
};
use crate::index_body::IndexBodyError;
use crate::source_body::SourceBodyError;
use crate::stream::{decode_prefix_chunk_stream, StreamError};
use crate::summary_body::SummaryBodyError;
use crate::FixedHeader;

/// Fail-closed compressed mini-profile errors.
#[derive(Debug, PartialEq, Eq)]
pub enum CompressedProfileError {
    Stream(StreamError),
    EventBody(EventBodyError),
    /// SOURCE-body fail-closed (mid-record SOURCE span and related paths).
    SourceBody(SourceBodyError),
    /// INDEX-body fail-closed (mid-record INDEX span and related paths).
    IndexBody(IndexBodyError),
    /// SUMMARY-body fail-closed (mid-record SUMMARY span and related paths).
    SummaryBody(SummaryBodyError),
    Payload(PayloadCodecError),
    /// EVENT codec not in {NONE, ZLIB, ZSTD, LZ4}.
    UnsupportedEventCodec { codec: u8 },
    /// Chunk kind not EVENT or FOOTER on this MVP path.
    UnexpectedKind { kind: u8 },
    /// FOOTER not last / more than one FOOTER.
    InvalidFooter,
    /// FOOTER must use codec NONE on this MVP.
    UnexpectedFooterCodec { codec: u8 },
}

impl std::fmt::Display for CompressedProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressedProfileError::Stream(e) => write!(f, "compressed profile stream: {e}"),
            CompressedProfileError::EventBody(e) => {
                write!(f, "compressed profile event-body: {e}")
            }
            CompressedProfileError::SourceBody(e) => {
                write!(f, "compressed profile source-body: {e}")
            }
            CompressedProfileError::IndexBody(e) => {
                write!(f, "compressed profile index-body: {e}")
            }
            CompressedProfileError::SummaryBody(e) => {
                write!(f, "compressed profile summary-body: {e}")
            }
            CompressedProfileError::Payload(e) => write!(f, "compressed profile payload: {e}"),
            CompressedProfileError::UnsupportedEventCodec { codec } => {
                write!(f, "compressed profile unsupported EVENT codec {codec}")
            }
            CompressedProfileError::UnexpectedKind { kind } => {
                write!(f, "compressed profile unexpected chunk kind {kind}")
            }
            CompressedProfileError::InvalidFooter => {
                write!(f, "compressed profile invalid FOOTER placement")
            }
            CompressedProfileError::UnexpectedFooterCodec { codec } => {
                write!(f, "compressed profile FOOTER codec {codec} (NONE required)")
            }
        }
    }
}

impl std::error::Error for CompressedProfileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CompressedProfileError::Stream(e) => Some(e),
            CompressedProfileError::EventBody(e) => Some(e),
            CompressedProfileError::SourceBody(e) => Some(e),
            CompressedProfileError::IndexBody(e) => Some(e),
            CompressedProfileError::SummaryBody(e) => Some(e),
            CompressedProfileError::Payload(e) => Some(e),
            _ => None,
        }
    }
}

impl From<StreamError> for CompressedProfileError {
    fn from(e: StreamError) -> Self {
        CompressedProfileError::Stream(e)
    }
}

impl From<EventBodyError> for CompressedProfileError {
    fn from(e: EventBodyError) -> Self {
        CompressedProfileError::EventBody(e)
    }
}

impl From<SourceBodyError> for CompressedProfileError {
    fn from(e: SourceBodyError) -> Self {
        CompressedProfileError::SourceBody(e)
    }
}

impl From<IndexBodyError> for CompressedProfileError {
    fn from(e: IndexBodyError) -> Self {
        CompressedProfileError::IndexBody(e)
    }
}

impl From<SummaryBodyError> for CompressedProfileError {
    fn from(e: SummaryBodyError) -> Self {
        CompressedProfileError::SummaryBody(e)
    }
}

impl From<PayloadCodecError> for CompressedProfileError {
    fn from(e: PayloadCodecError) -> Self {
        CompressedProfileError::Payload(e)
    }
}

impl From<FilePrefixError> for CompressedProfileError {
    fn from(e: FilePrefixError) -> Self {
        CompressedProfileError::Stream(StreamError::Prefix(e))
    }
}

impl From<ChunkError> for CompressedProfileError {
    fn from(e: ChunkError) -> Self {
        CompressedProfileError::Stream(StreamError::Chunk(e))
    }
}

pub type CompressedProfileResult<T> = std::result::Result<T, CompressedProfileError>;

/// Owned logical event recovered after inflate + `decode_event_body`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedEventRecord {
    Mark { label: Vec<u8> },
    TimeLine { fid: u64, line: u64, ticks: u64 },
    TimeBlock {
        fid: u64,
        line: u64,
        block_line: u64,
        ticks: u64,
    },
    SubEntry { caller_fid: u64, caller_line: u64 },
    SubReturn {
        depth: u64,
        incl: u64,
        excl: u64,
        subname: Vec<u8>,
    },
    SubInfo {
        fid: u64,
        first_line: u64,
        last_line: u64,
        name: Vec<u8>,
    },
    SrcLine {
        fid: u64,
        line: u64,
        text: Vec<u8>,
    },
    NewFid {
        fid: u64,
        filename: Vec<u8>,
    },
    PidStart {
        pid: u64,
        ppid: u64,
        start_time: u64,
    },
    PidEnd { pid: u64, end_time: u64 },
    SubCallers {
        fid: u64,
        line: u64,
        count: u64,
        incl: u64,
        excl: u64,
        reci: u64,
        rec_depth: u64,
        called: Vec<u8>,
        caller: Vec<u8>,
    },
    Discount,
    Attribute {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Option {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Comment { text: Vec<u8> },
    StartDeflate,
    Version { major: u64, minor: u64 },
}

impl OwnedEventRecord {
    /// Copy a borrowed event-body record into an owned form (after inflate).
    pub fn from_borrowed(r: &EventRecord<'_>) -> Self {
        match r {
            EventRecord::Mark { label } => OwnedEventRecord::Mark {
                label: label.data.to_vec(),
            },
            EventRecord::TimeLine { fid, line, ticks } => OwnedEventRecord::TimeLine {
                fid: *fid,
                line: *line,
                ticks: *ticks,
            },
            EventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => OwnedEventRecord::TimeBlock {
                fid: *fid,
                line: *line,
                block_line: *block_line,
                ticks: *ticks,
            },
            EventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => OwnedEventRecord::SubEntry {
                caller_fid: *caller_fid,
                caller_line: *caller_line,
            },
            EventRecord::SubReturn {
                depth,
                incl,
                excl,
                subname,
            } => OwnedEventRecord::SubReturn {
                depth: *depth,
                incl: *incl,
                excl: *excl,
                subname: subname.data.to_vec(),
            },
            EventRecord::SubInfo {
                fid,
                first_line,
                last_line,
                name,
            } => OwnedEventRecord::SubInfo {
                fid: *fid,
                first_line: *first_line,
                last_line: *last_line,
                name: name.data.to_vec(),
            },
            EventRecord::SrcLine { fid, line, text } => OwnedEventRecord::SrcLine {
                fid: *fid,
                line: *line,
                text: text.data.to_vec(),
            },
            EventRecord::NewFid { fid, filename } => OwnedEventRecord::NewFid {
                fid: *fid,
                filename: filename.data.to_vec(),
            },
            EventRecord::PidStart {
                pid,
                ppid,
                start_time,
            } => OwnedEventRecord::PidStart {
                pid: *pid,
                ppid: *ppid,
                start_time: *start_time,
            },
            EventRecord::PidEnd { pid, end_time } => OwnedEventRecord::PidEnd {
                pid: *pid,
                end_time: *end_time,
            },
            EventRecord::SubCallers {
                fid,
                line,
                count,
                incl,
                excl,
                reci,
                rec_depth,
                called,
                caller,
            } => OwnedEventRecord::SubCallers {
                fid: *fid,
                line: *line,
                count: *count,
                incl: *incl,
                excl: *excl,
                reci: *reci,
                rec_depth: *rec_depth,
                called: called.data.to_vec(),
                caller: caller.data.to_vec(),
            },
            EventRecord::Discount => OwnedEventRecord::Discount,
            EventRecord::Attribute { key, value } => OwnedEventRecord::Attribute {
                key: key.data.to_vec(),
                value: value.data.to_vec(),
            },
            EventRecord::Option { key, value } => OwnedEventRecord::Option {
                key: key.data.to_vec(),
                value: value.data.to_vec(),
            },
            EventRecord::Comment { text } => OwnedEventRecord::Comment {
                text: text.data.to_vec(),
            },
            EventRecord::StartDeflate => OwnedEventRecord::StartDeflate,
            EventRecord::Version { major, minor } => OwnedEventRecord::Version {
                major: *major,
                minor: *minor,
            },
        }
    }
}

/// Decoded compressed mini-profile (owned logical events).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressedMiniProfile {
    pub header: FixedHeader,
    /// Codec of the (first) EVENT chunk when present; `NONE` if no EVENT chunks.
    pub event_codec: u8,
    pub records: Vec<OwnedEventRecord>,
    pub has_footer: bool,
    pub footer_payload: Option<Vec<u8>>,
}

pub(crate) fn is_supported_event_codec(c: u8) -> bool {
    c == codec::NONE || c == codec::ZLIB || c == codec::ZSTD || c == codec::LZ4
}

/// Encode a chunk frame for `plain` body bytes under the chosen payload codec.
///
/// `chunk_kind` is the v6 kind (EVENT/SOURCE/INDEX/SUMMARY/…). FOOTER compression
/// is not used by current composition paths (FOOTER stays codec NONE).
pub(crate) fn encode_kind_chunk(
    chunk_kind: u8,
    payload_codec: u8,
    sequence: u64,
    logical_event_count: u32,
    plain: &[u8],
) -> CompressedProfileResult<Vec<u8>> {
    match payload_codec {
        c if c == codec::NONE => {
            let checksum = compute_payload_crc(plain);
            Ok(encode_chunk_frame(
                chunk_kind,
                codec::NONE,
                0,
                sequence,
                0,
                logical_event_count,
                plain.len() as u32,
                plain,
                checksum,
            ))
        }
        c if c == codec::ZLIB => Ok(encode_chunk_frame_zlib(
            chunk_kind,
            0,
            sequence,
            0,
            logical_event_count,
            plain,
        )?),
        c if c == codec::ZSTD => Ok(encode_chunk_frame_zstd(
            chunk_kind,
            0,
            sequence,
            0,
            logical_event_count,
            plain,
        )?),
        c if c == codec::LZ4 => Ok(encode_chunk_frame_lz4(
            chunk_kind,
            0,
            sequence,
            0,
            logical_event_count,
            plain,
        )?),
        other => Err(CompressedProfileError::UnsupportedEventCodec { codec: other }),
    }
}

/// Encode EVENT frame for `plain` event-body under the chosen payload codec.
pub(crate) fn encode_event_chunk(
    event_codec: u8,
    sequence: u64,
    logical_event_count: u32,
    plain: &[u8],
) -> CompressedProfileResult<Vec<u8>> {
    encode_kind_chunk(kind::EVENT, event_codec, sequence, logical_event_count, plain)
}

/// Encode a provisional compressed mini-profile.
///
/// - File prefix (fixed header + multi-TLV … END)
/// - If `events` non-empty: one EVENT chunk with `event_codec` payload of
///   `encode_event_body(events)`
/// - If `footer` is `Some`: trailing FOOTER chunk, codec NONE
///
/// Pure byte-slice / `Vec` API — no I/O. Does not change default
/// `parse_chunk_frame` inflate policy.
pub fn encode_compressed_mini_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    event_codec: u8,
    events: &[EventRecordSpec<'_>],
    footer: Option<&[u8]>,
) -> CompressedProfileResult<Vec<u8>> {
    if !events.is_empty() && !is_supported_event_codec(event_codec) {
        return Err(CompressedProfileError::UnsupportedEventCodec {
            codec: event_codec,
        });
    }

    let mut out = encode_file_prefix(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
    );

    let mut seq = 0u64;
    if !events.is_empty() {
        let plain = encode_event_body(events);
        let frame = encode_event_chunk(event_codec, seq, events.len() as u32, &plain)?;
        out.extend_from_slice(&frame);
        seq += 1;
    }

    if let Some(fp) = footer {
        let checksum = compute_payload_crc(fp);
        out.extend_from_slice(&encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            seq,
            0,
            0,
            fp.len() as u32,
            fp,
            checksum,
        ));
    }

    Ok(out)
}

/// Decode a provisional compressed mini-profile.
///
/// 1. `decode_prefix_chunk_stream` (non-inflating chunk parse).
/// 2. For each EVENT: `decode_chunk_payload` then `decode_event_body` on plain bytes.
/// 3. Optional trailing FOOTER (codec NONE).
///
/// Returns owned logical records (labels copied after inflate).
pub fn decode_compressed_mini_profile(
    buf: &[u8],
) -> CompressedProfileResult<(CompressedMiniProfile, usize)> {
    let (stream, n) = decode_prefix_chunk_stream(buf)?;
    let mut records = Vec::new();
    let mut has_footer = false;
    let mut footer_payload: Option<Vec<u8>> = None;
    let mut saw_footer = false;
    let mut event_codec = codec::NONE;
    let mut saw_event = false;

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
                }
                // Explicit inflate path — default parse left wire payload compressed.
                let plain = decode_chunk_payload(frame)?;
                let (body_recs, body_n) = decode_event_body(&plain)?;
                if body_n != plain.len() {
                    return Err(CompressedProfileError::EventBody(EventBodyError::Truncated {
                        need: plain.len(),
                        got: body_n,
                    }));
                }
                for r in &body_recs {
                    records.push(OwnedEventRecord::from_borrowed(r));
                }
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

    Ok((
        CompressedMiniProfile {
            header: stream.prefix.header,
            event_codec,
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
    use crate::event_body::opcode;
    use crate::payload_codec::{compress_lz4, compress_zstd, deflate_zlib};
    use crate::varint::encode_u64;
    use crate::{MAGIC, SUPPORTED_MAJOR};

    fn sample_events() -> [EventRecordSpec<'static>; 2] {
        [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 42,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"main::leaf",
            },
        ]
    }

    fn assert_sample_records(recs: &[OwnedEventRecord]) {
        assert_eq!(recs.len(), 2);
        match &recs[0] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 5, 42));
            }
            other => panic!("expected TimeLine, got {other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"main::leaf"),
            other => panic!("expected Mark, got {other:?}"),
        }
    }

    #[test]
    fn roundtrip_none_zlib_zstd_lz4() {
        let events = sample_events();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let enc_a = encode_compressed_mini_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                Some(b"end"),
            )
            .expect("encode");
            let enc_b = encode_compressed_mini_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                Some(b"end"),
            )
            .expect("encode 2");
            assert_eq!(enc_a, enc_b, "deterministic encode for codec {c}");
            assert_eq!(&enc_a[..8], MAGIC.as_slice());

            let (prof, n) = decode_compressed_mini_profile(&enc_a).expect("decode");
            assert_eq!(n, enc_a.len());
            assert_eq!(prof.event_codec, c);
            assert_sample_records(&prof.records);
            assert!(prof.has_footer);
            assert_eq!(prof.footer_payload.as_deref(), Some(&b"end"[..]));
            assert_eq!(prof.header.major, SUPPORTED_MAJOR);

            // Default parse_chunk_frame path: first EVENT payload is not plain for compressed codecs.
            let (stream, _) = decode_prefix_chunk_stream(&enc_a).unwrap();
            let event = stream
                .chunks
                .iter()
                .find(|f| f.kind == kind::EVENT)
                .expect("event chunk");
            assert_eq!(event.codec, c);
            if c != codec::NONE {
                let plain = encode_event_body(&events);
                assert_ne!(
                    event.payload, plain.as_slice(),
                    "wire payload must stay compressed for codec {c}"
                );
            }
        }
    }

    #[test]
    fn empty_events_prefix_only() {
        let enc = encode_compressed_mini_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZSTD, // ignored when no events
            &[],
            None,
        )
        .unwrap();
        let prefix_only = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        assert_eq!(enc, prefix_only);
        let (prof, n) = decode_compressed_mini_profile(&enc).unwrap();
        assert_eq!(n, enc.len());
        assert!(prof.records.is_empty());
        assert_eq!(prof.event_codec, codec::NONE);
    }

    #[test]
    fn corrupt_zlib_event_payload_err() {
        let events = sample_events();
        let mut enc = encode_compressed_mini_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &events,
            None,
        )
        .unwrap();
        // Locate EVENT payload after prefix and header; flip wire bytes.
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let frame = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        assert_eq!(frame.codec, codec::ZLIB);
        assert!(frame.payload.len() > 4);
        let payload_off = prefix_n + CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        enc[payload_off + 1] ^= 0xAA;
        match decode_compressed_mini_profile(&enc) {
            Err(CompressedProfileError::Payload(_)) => {}
            other => panic!("expected payload error, got {other:?}"),
        }
    }

    #[test]
    fn size_mismatch_zstd_event_err() {
        // Build a well-formed prefix + EVENT frame whose uncompressed_len is wrong.
        let plain = encode_event_body(&sample_events());
        let compressed = compress_zstd(&plain).unwrap();
        let prefix = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        // Lie: declare half the plain length.
        let wrong_len = (plain.len() as u32) / 2;
        assert!(wrong_len > 0 && wrong_len != plain.len() as u32);
        let frame = encode_chunk_frame(
            kind::EVENT,
            codec::ZSTD,
            0,
            0,
            0,
            2,
            wrong_len,
            &compressed,
            compute_payload_crc(&compressed),
        );
        let mut enc = prefix;
        enc.extend_from_slice(&frame);
        match decode_compressed_mini_profile(&enc) {
            Err(CompressedProfileError::Payload(
                PayloadCodecError::SizeMismatch { .. } | PayloadCodecError::Zstd { .. },
            )) => {}
            other => panic!("expected size/zstd payload error, got {other:?}"),
        }
    }

    #[test]
    fn size_mismatch_lz4_event_err() {
        let plain = encode_event_body(&sample_events());
        let compressed = compress_lz4(&plain).unwrap();
        let prefix = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let wrong_len = plain.len() as u32 + 10;
        let frame = encode_chunk_frame(
            kind::EVENT,
            codec::LZ4,
            0,
            0,
            0,
            2,
            wrong_len,
            &compressed,
            compute_payload_crc(&compressed),
        );
        let mut enc = prefix;
        enc.extend_from_slice(&frame);
        match decode_compressed_mini_profile(&enc) {
            Err(CompressedProfileError::Payload(
                PayloadCodecError::SizeMismatch { .. } | PayloadCodecError::Lz4 { .. },
            )) => {}
            other => panic!("expected size/lz4 payload error, got {other:?}"),
        }
    }

    #[test]
    fn unsupported_codec_err() {
        match encode_compressed_mini_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            0xFE,
            &sample_events(),
            None,
        ) {
            Err(CompressedProfileError::UnsupportedEventCodec { codec: 0xFE }) => {}
            other => panic!("expected unsupported codec, got {other:?}"),
        }
    }

    #[test]
    fn never_panic_on_garbage() {
        assert!(decode_compressed_mini_profile(&[]).is_err());
        assert!(decode_compressed_mini_profile(b"not-v6").is_err());
        let mut almost =
            encode_compressed_mini_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], codec::NONE, &[], None)
                .unwrap();
        almost.push(0x01);
        assert!(decode_compressed_mini_profile(&almost).is_err());
        // reserved opcode inside NONE event body
        let mut bad_body = encode_u64(opcode::RESERVED);
        bad_body.push(0);
        let prefix = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let frame = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            1,
            bad_body.len() as u32,
            &bad_body,
            0,
        );
        let mut enc = prefix;
        enc.extend_from_slice(&frame);
        assert!(decode_compressed_mini_profile(&enc).is_err());
    }

    #[test]
    fn zlib_wire_uses_deflate_helper_not_plain() {
        let events = sample_events();
        let plain = encode_event_body(&events);
        let expected_wire = deflate_zlib(&plain).unwrap();
        let enc = encode_compressed_mini_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &events,
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let frame = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        assert_eq!(frame.payload, expected_wire.as_slice());
    }

    #[test]
    fn bad_sync_err() {
        let mut enc =
            encode_compressed_mini_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], codec::NONE, &[], None)
                .unwrap();
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_compressed_mini_profile(&enc) {
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
}

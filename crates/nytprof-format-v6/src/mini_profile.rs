//! Provisional **format v6** mini-profile composition (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-mini-profile-provisional-v0.md`
//!
//! Composes shipped file-prefix + chunk stream + event-body APIs into a minimal
//! complete profile: `[file prefix][EVENT codec-NONE event-body…][optional FOOTER]`.
//! No payload inflate, no full opcode catalog, no C writer.

use crate::chunk::{codec, kind};
use crate::event_body::{decode_event_body, EventBodyError, EventRecord, EventRecordSpec};
use crate::file_prefix::FilePrefix;
use crate::stream::{decode_prefix_chunk_stream, StreamError};

/// Fail-closed mini-profile composition errors.
#[derive(Debug, PartialEq, Eq)]
pub enum MiniProfileError {
    Stream(StreamError),
    EventBody(EventBodyError),
    /// EVENT chunk must use codec NONE in this MVP.
    UnexpectedCodec {
        codec: u8,
    },
    /// Chunk kind not EVENT or FOOTER (mini-profile MVP).
    UnexpectedKind {
        kind: u8,
    },
    /// FOOTER appeared before EVENT stream end more than once, or mid-stream FOOTER
    /// followed by another non-footer (MVP: at most one trailing FOOTER).
    InvalidFooter,
}

impl std::fmt::Display for MiniProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MiniProfileError::Stream(e) => write!(f, "v6 mini-profile stream: {e}"),
            MiniProfileError::EventBody(e) => write!(f, "v6 mini-profile event-body: {e}"),
            MiniProfileError::UnexpectedCodec { codec } => {
                write!(
                    f,
                    "v6 mini-profile unexpected chunk codec {codec} (NONE required)"
                )
            }
            MiniProfileError::UnexpectedKind { kind } => {
                write!(f, "v6 mini-profile unexpected chunk kind {kind}")
            }
            MiniProfileError::InvalidFooter => {
                write!(f, "v6 mini-profile invalid FOOTER placement")
            }
        }
    }
}

impl std::error::Error for MiniProfileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MiniProfileError::Stream(e) => Some(e),
            MiniProfileError::EventBody(e) => Some(e),
            _ => None,
        }
    }
}

impl From<StreamError> for MiniProfileError {
    fn from(e: StreamError) -> Self {
        MiniProfileError::Stream(e)
    }
}

impl From<EventBodyError> for MiniProfileError {
    fn from(e: EventBodyError) -> Self {
        MiniProfileError::EventBody(e)
    }
}

pub type MiniProfileResult<T> = std::result::Result<T, MiniProfileError>;

/// Decoded provisional mini-profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniProfile<'a> {
    pub prefix: FilePrefix<'a>,
    /// Flattened event records from all EVENT/codec-NONE chunk payloads (order preserved).
    pub records: Vec<EventRecord<'a>>,
    /// True if a trailing FOOTER chunk was present.
    pub has_footer: bool,
    /// FOOTER payload when present (may be empty).
    pub footer_payload: Option<&'a [u8]>,
}

/// Encode a provisional mini-profile (single-chunk EVENT when non-empty).
///
/// Layout:
/// - file prefix (fixed header + multi-TLV … END)
/// - if `events` is non-empty: one EVENT chunk, codec NONE, payload = `encode_event_body(events)`
/// - if `events` is empty: no EVENT chunk (prefix-only event section)
/// - if `footer` is `Some(payload)`: one FOOTER chunk (codec NONE, opaque payload)
///
/// Implemented via multi-chunk EVENT encode with `max_records_per_chunk = 0`
/// (unlimited → at most one EVENT). Multi-chunk split: `encode_multi_chunk_event_profile`.
/// Pure byte-slice / `Vec` API — no I/O.
pub fn encode_mini_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    events: &[EventRecordSpec<'_>],
    footer: Option<&[u8]>,
) -> Vec<u8> {
    crate::multi_chunk_event::encode_multi_chunk_event_profile(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        events,
        0, // unlimited → single EVENT chunk
        footer,
    )
}

/// Decode a provisional mini-profile.
///
/// 1. `decode_prefix_chunk_stream` (fail-closed bad magic / truncated / bad sync).
/// 2. Walk chunks: EVENT + codec NONE → `decode_event_body` (append records);
///    FOOTER (codec NONE) must be last and at most one; other kinds / codecs → Err.
///
/// Empty event section (no EVENT chunks) yields empty `records`. Returns
/// `(MiniProfile, total_bytes_consumed)`.
pub fn decode_mini_profile(buf: &[u8]) -> MiniProfileResult<(MiniProfile<'_>, usize)> {
    let (stream, n) = decode_prefix_chunk_stream(buf)?;
    let mut records = Vec::new();
    let mut has_footer = false;
    let mut footer_payload: Option<&[u8]> = None;
    let mut saw_footer = false;

    for frame in &stream.chunks {
        if saw_footer {
            // Nothing may follow FOOTER.
            return Err(MiniProfileError::InvalidFooter);
        }
        match frame.kind {
            k if k == kind::EVENT => {
                if frame.codec != codec::NONE {
                    return Err(MiniProfileError::UnexpectedCodec { codec: frame.codec });
                }
                let (body_recs, body_n) = decode_event_body(frame.payload)?;
                if body_n != frame.payload.len() {
                    // decode_event_body consumes all on success; belt-and-suspenders.
                    return Err(MiniProfileError::EventBody(EventBodyError::Truncated {
                        need: frame.payload.len(),
                        got: body_n,
                    }));
                }
                records.extend(body_recs);
            }
            k if k == kind::FOOTER => {
                if frame.codec != codec::NONE {
                    return Err(MiniProfileError::UnexpectedCodec { codec: frame.codec });
                }
                has_footer = true;
                footer_payload = Some(frame.payload);
                saw_footer = true;
            }
            other => {
                return Err(MiniProfileError::UnexpectedKind { kind: other });
            }
        }
    }

    Ok((
        MiniProfile {
            prefix: stream.prefix,
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
    use crate::chunk::{
        encode_chunk_frame, parse_chunk_frame, ChunkError, CHUNK_HEADER_LEN, CHUNK_SYNC,
    };
    use crate::encode_event_body;
    use crate::encode_file_prefix;
    use crate::event_body::opcode;
    use crate::stream::StreamError;
    use crate::varint::encode_u64;
    use crate::Error as HeaderError;
    use crate::{FilePrefixError, MAGIC, SUPPORTED_MAJOR};

    #[test]
    fn roundtrip_empty_events_prefix_only() {
        let enc_a = encode_mini_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[], None);
        let enc_b = encode_mini_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[], None);
        assert_eq!(enc_a, enc_b);
        assert_eq!(&enc_a[..8], MAGIC.as_slice());
        // Equals bare file-prefix encode (no chunks).
        let prefix_only = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        assert_eq!(enc_a, prefix_only);

        let (mp, n) = decode_mini_profile(&enc_a).expect("empty");
        assert_eq!(n, enc_a.len());
        assert!(mp.records.is_empty());
        assert!(!mp.has_footer);
        assert!(mp.footer_payload.is_none());
        assert_eq!(mp.prefix.header.major, SUPPORTED_MAJOR);

        let (mp2, n2) = decode_mini_profile(&enc_a).unwrap();
        assert_eq!(n2, n);
        assert_eq!(mp2.records.len(), 0);
    }

    #[test]
    fn roundtrip_events_and_optional_footer() {
        let events = [
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
        ];
        let footer = b"end";
        let enc = encode_mini_profile(SUPPORTED_MAJOR, 1, 0, 0, 0, &[], &events, Some(footer));

        // Length = prefix + EVENT frame + FOOTER frame (derived from shipped encodes).
        let body = encode_event_body(&events);
        let event_frame = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            events.len() as u32,
            body.len() as u32,
            &body,
            0,
        );
        let footer_frame = encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            1,
            0,
            0,
            footer.len() as u32,
            footer,
            0,
        );
        let prefix = encode_file_prefix(SUPPORTED_MAJOR, 1, 0, 0, 0, &[]);
        assert_eq!(
            enc.len(),
            prefix.len() + event_frame.len() + footer_frame.len()
        );

        let (mp, n) = decode_mini_profile(&enc).expect("with events");
        assert_eq!(n, enc.len());
        assert_eq!(mp.records.len(), 2);
        match &mp.records[0] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 5, 42));
            }
            other => panic!("expected TimeLine, got {other:?}"),
        }
        match &mp.records[1] {
            EventRecord::Mark { label } => {
                assert_eq!(label.data, b"main::leaf");
            }
            other => panic!("expected Mark, got {other:?}"),
        }
        assert!(mp.has_footer);
        assert_eq!(mp.footer_payload, Some(footer.as_slice()));
    }

    #[test]
    fn empty_events_with_footer_ok() {
        let enc = encode_mini_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[], Some(b""));
        let (mp, n) = decode_mini_profile(&enc).expect("footer only");
        assert_eq!(n, enc.len());
        assert!(mp.records.is_empty());
        assert!(mp.has_footer);
        assert_eq!(mp.footer_payload, Some(&b""[..]));
    }

    #[test]
    fn bad_magic_err() {
        let mut enc = encode_mini_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[], None);
        enc[0] = b'X';
        assert_eq!(
            decode_mini_profile(&enc),
            Err(MiniProfileError::Stream(StreamError::Prefix(
                FilePrefixError::Header(HeaderError::BadMagic)
            )))
        );
    }

    #[test]
    fn bad_chunk_sync_err() {
        let mut enc = encode_mini_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[], None);
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_mini_profile(&enc) {
            Err(MiniProfileError::Stream(StreamError::Chunk(ChunkError::BadSync {
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
    fn truncated_mid_chunk_err() {
        let events = [EventRecordSpec::TimeLine {
            fid: 1,
            line: 2,
            ticks: 3,
        }];
        let mut enc = encode_mini_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &events, None);
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        assert!(enc.len() > prefix_n + CHUNK_HEADER_LEN + 2);
        enc.truncate(prefix_n + CHUNK_HEADER_LEN + 2);
        match decode_mini_profile(&enc) {
            Err(MiniProfileError::Stream(StreamError::Chunk(ChunkError::Truncated { .. }))) => {}
            other => panic!("expected truncated mid-chunk, got {other:?}"),
        }
    }

    #[test]
    fn truncated_mid_event_body_err() {
        // Full chunk framing but event-body truncated inside payload.
        let body = encode_event_body(&[EventRecordSpec::TimeLine {
            fid: 1,
            line: 2,
            ticks: 3,
        }]);
        assert!(body.len() > 2);
        let trunc_body = &body[..body.len() - 1];
        // Manually build prefix + EVENT chunk declaring full body length but with short payload
        // would fail at chunk layer. Instead: frame with truncated body as the full payload
        // so chunk parses, then event-body fails mid-record.
        let prefix = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let frame = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            1,
            trunc_body.len() as u32,
            trunc_body,
            0,
        );
        let mut enc = prefix;
        enc.extend_from_slice(&frame);
        // Sanity: chunk parses
        let _ =
            parse_chunk_frame(&enc[encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len()..])
                .expect("chunk ok");
        match decode_mini_profile(&enc) {
            Err(MiniProfileError::EventBody(_)) => {}
            other => panic!("expected event-body error, got {other:?}"),
        }
    }

    #[test]
    fn never_panic_on_garbage() {
        assert!(decode_mini_profile(&[]).is_err());
        assert!(decode_mini_profile(b"not-v6").is_err());
        let mut almost = encode_mini_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[], None);
        almost.push(0x01);
        assert!(decode_mini_profile(&almost).is_err());
        // reserved opcode inside a well-framed EVENT body
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
        assert!(decode_mini_profile(&enc).is_err());
    }
}

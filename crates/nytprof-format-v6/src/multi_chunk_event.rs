//! Provisional **format v6** multi-chunk EVENT body framing (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-multi-chunk-event-provisional-v0.md`
//!
//! Splits an ordered event-body record stream across **one or more** codec-NONE
//! EVENT chunks and reassembles on decode in sequence order.
//! Composes shipped event-body + chunk stream APIs. No inflate, no full catalog.

use crate::chunk::{codec, kind};
use crate::event_body::{
    decode_event_body, encode_event_body, EventBodyError, EventRecord, EventRecordSpec,
};
use crate::file_prefix::FilePrefix;
use crate::stream::{
    decode_prefix_chunk_stream, encode_prefix_chunk_stream, ChunkSpec, StreamError,
};

/// Fail-closed multi-chunk EVENT errors.
#[derive(Debug, PartialEq, Eq)]
pub enum MultiChunkEventError {
    Stream(StreamError),
    EventBody(EventBodyError),
    UnexpectedCodec { codec: u8 },
    UnexpectedKind { kind: u8 },
    InvalidFooter,
}

impl std::fmt::Display for MultiChunkEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultiChunkEventError::Stream(e) => write!(f, "v6 multi-chunk EVENT stream: {e}"),
            MultiChunkEventError::EventBody(e) => {
                write!(f, "v6 multi-chunk EVENT event-body: {e}")
            }
            MultiChunkEventError::UnexpectedCodec { codec } => {
                write!(
                    f,
                    "v6 multi-chunk EVENT unexpected chunk codec {codec} (NONE required)"
                )
            }
            MultiChunkEventError::UnexpectedKind { kind } => {
                write!(f, "v6 multi-chunk EVENT unexpected chunk kind {kind}")
            }
            MultiChunkEventError::InvalidFooter => {
                write!(f, "v6 multi-chunk EVENT invalid FOOTER placement")
            }
        }
    }
}

impl std::error::Error for MultiChunkEventError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MultiChunkEventError::Stream(e) => Some(e),
            MultiChunkEventError::EventBody(e) => Some(e),
            _ => None,
        }
    }
}

impl From<StreamError> for MultiChunkEventError {
    fn from(e: StreamError) -> Self {
        MultiChunkEventError::Stream(e)
    }
}

impl From<EventBodyError> for MultiChunkEventError {
    fn from(e: EventBodyError) -> Self {
        MultiChunkEventError::EventBody(e)
    }
}

pub type MultiChunkEventResult<T> = std::result::Result<T, MultiChunkEventError>;

/// Decoded multi-chunk EVENT profile (same shape as mini-profile).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiChunkEventProfile<'a> {
    pub prefix: FilePrefix<'a>,
    /// Flattened records from all EVENT/codec-NONE chunks in file order.
    pub records: Vec<EventRecord<'a>>,
    /// Number of EVENT chunks seen (0 if empty event section).
    pub event_chunk_count: usize,
    pub has_footer: bool,
    pub footer_payload: Option<&'a [u8]>,
}

/// Partition `events` into slices of at most `max_records_per_chunk` records.
///
/// Provisional split rule (records-per-chunk):
/// - `max_records_per_chunk == 0` → one partition containing all records (or empty).
/// - `max_records_per_chunk >= 1` → consecutive windows of that size (last may be shorter).
///
/// Empty `events` yields an empty partition list (encode emits zero EVENT chunks).
pub fn partition_event_records<'a>(
    events: &'a [EventRecordSpec<'a>],
    max_records_per_chunk: usize,
) -> Vec<&'a [EventRecordSpec<'a>]> {
    if events.is_empty() {
        return Vec::new();
    }
    if max_records_per_chunk == 0 {
        return vec![events];
    }
    events.chunks(max_records_per_chunk).collect()
}

/// Encode a provisional profile with multi-chunk EVENT framing.
///
/// Layout: `[file prefix][EVENT codec NONE…][optional FOOTER]`.
///
/// Each EVENT payload is an independent `encode_event_body` of one partition
/// from [`partition_event_records`]. Chunk `sequence` is 0..n-1 for EVENT chunks.
///
/// `max_records_per_chunk`:
/// - `0` = unlimited → at most one EVENT chunk (single-chunk / mini-profile compat)
/// - `n >= 1` = split into EVENT chunks of at most `n` records each
///
/// Composes shipped `encode_event_body` + `encode_prefix_chunk_stream`.
pub fn encode_multi_chunk_event_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    events: &[EventRecordSpec<'_>],
    max_records_per_chunk: usize,
    footer: Option<&[u8]>,
) -> Vec<u8> {
    let partitions = partition_event_records(events, max_records_per_chunk);
    let bodies: Vec<Vec<u8>> = partitions
        .iter()
        .map(|part| encode_event_body(part))
        .collect();

    let mut chunks: Vec<ChunkSpec<'_>> = Vec::with_capacity(bodies.len() + 1);
    for (i, body) in bodies.iter().enumerate() {
        let part_len = partitions[i].len() as u32;
        chunks.push(ChunkSpec {
            kind: kind::EVENT,
            codec: codec::NONE,
            flags: 0,
            sequence: i as u64,
            first_logical_seq: 0,
            logical_event_count: part_len,
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

/// Decode a multi-chunk EVENT profile and reassemble records in chunk order.
///
/// Walks EVENT/codec-NONE chunks via shipped `decode_prefix_chunk_stream` +
/// `decode_event_body`, appending records. Fail-closed on bad magic, truncated
/// mid-chunk, bad sync, truncated mid-event-body, unexpected kind/codec, or
/// invalid FOOTER placement.
pub fn decode_multi_chunk_event_profile(
    buf: &[u8],
) -> MultiChunkEventResult<(MultiChunkEventProfile<'_>, usize)> {
    let (stream, n) = decode_prefix_chunk_stream(buf)?;
    let mut records = Vec::new();
    let mut event_chunk_count = 0usize;
    let mut has_footer = false;
    let mut footer_payload: Option<&[u8]> = None;
    let mut saw_footer = false;

    for frame in &stream.chunks {
        if saw_footer {
            return Err(MultiChunkEventError::InvalidFooter);
        }
        match frame.kind {
            k if k == kind::EVENT => {
                if frame.codec != codec::NONE {
                    return Err(MultiChunkEventError::UnexpectedCodec {
                        codec: frame.codec,
                    });
                }
                let (body_recs, body_n) = decode_event_body(frame.payload)?;
                if body_n != frame.payload.len() {
                    return Err(MultiChunkEventError::EventBody(EventBodyError::Truncated {
                        need: frame.payload.len(),
                        got: body_n,
                    }));
                }
                records.extend(body_recs);
                event_chunk_count += 1;
            }
            k if k == kind::FOOTER => {
                if frame.codec != codec::NONE {
                    return Err(MultiChunkEventError::UnexpectedCodec {
                        codec: frame.codec,
                    });
                }
                has_footer = true;
                footer_payload = Some(frame.payload);
                saw_footer = true;
            }
            other => {
                return Err(MultiChunkEventError::UnexpectedKind { kind: other });
            }
        }
    }

    Ok((
        MultiChunkEventProfile {
            prefix: stream.prefix,
            records,
            event_chunk_count,
            has_footer,
            footer_payload,
        },
        n,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{encode_chunk_frame, CHUNK_HEADER_LEN, CHUNK_SYNC};
    use crate::encode_file_prefix;
    use crate::event_body::opcode;
    use crate::varint::encode_u64;
    use crate::{FilePrefixError, MAGIC, SUPPORTED_MAJOR};
    use crate::Error as HeaderError;
    use crate::stream::StreamError;
    use crate::chunk::ChunkError;

    fn sample_events() -> [EventRecordSpec<'static>; 4] {
        [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 10,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"a",
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 2,
                ticks: 20,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"b",
            },
        ]
    }

    #[test]
    fn partition_unlimited_and_empty() {
        let events = sample_events();
        let p0 = partition_event_records(&events, 0);
        assert_eq!(p0.len(), 1);
        assert_eq!(p0[0].len(), 4);
        assert!(partition_event_records(&[], 2).is_empty());
        let p2 = partition_event_records(&events, 2);
        assert_eq!(p2.len(), 2);
        assert_eq!(p2[0].len(), 2);
        assert_eq!(p2[1].len(), 2);
        let p3 = partition_event_records(&events, 3);
        assert_eq!(p3.len(), 2);
        assert_eq!(p3[0].len(), 3);
        assert_eq!(p3[1].len(), 1);
    }

    #[test]
    fn single_chunk_compat_roundtrip() {
        let events = sample_events();
        let enc_a =
            encode_multi_chunk_event_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &events, 0, None);
        let enc_b =
            encode_multi_chunk_event_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &events, 0, None);
        assert_eq!(enc_a, enc_b);
        assert_eq!(&enc_a[..8], MAGIC.as_slice());

        // Single EVENT: length = prefix + one body frame.
        let body = encode_event_body(&events);
        let frame = encode_chunk_frame(
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
        let prefix = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        assert_eq!(enc_a.len(), prefix.len() + frame.len());

        let (prof, n) = decode_multi_chunk_event_profile(&enc_a).expect("single");
        assert_eq!(n, enc_a.len());
        assert_eq!(prof.event_chunk_count, 1);
        assert_eq!(prof.records.len(), 4);
        match &prof.records[0] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 1, 10));
            }
            other => panic!("expected TimeLine, got {other:?}"),
        }
        match &prof.records[3] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"b"),
            other => panic!("expected Mark, got {other:?}"),
        }
    }

    #[test]
    fn multi_chunk_roundtrip_ordered_fields() {
        let events = sample_events();
        // Force ≥2 EVENT chunks: max 1 record per chunk → 4 chunks.
        let enc =
            encode_multi_chunk_event_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &events, 1, None);
        let parts = partition_event_records(&events, 1);
        assert_eq!(parts.len(), 4);
        let prefix = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let mut expect_len = prefix.len();
        for (i, part) in parts.iter().enumerate() {
            let body = encode_event_body(part);
            let frame = encode_chunk_frame(
                kind::EVENT,
                codec::NONE,
                0,
                i as u64,
                0,
                part.len() as u32,
                body.len() as u32,
                &body,
                0,
            );
            expect_len += frame.len();
        }
        assert_eq!(enc.len(), expect_len);

        let (prof, n) = decode_multi_chunk_event_profile(&enc).expect("multi");
        assert_eq!(n, enc.len());
        assert_eq!(prof.event_chunk_count, 4);
        assert_eq!(prof.records.len(), 4);
        // Ordered reassembly across chunk boundaries.
        match &prof.records[0] {
            EventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
            other => panic!("{other:?}"),
        }
        match &prof.records[1] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"a"),
            other => panic!("{other:?}"),
        }
        match &prof.records[2] {
            EventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 20),
            other => panic!("{other:?}"),
        }
        match &prof.records[3] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"b"),
            other => panic!("{other:?}"),
        }

        // max=2 → two EVENT chunks, same ordered fields.
        let enc2 =
            encode_multi_chunk_event_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &events, 2, Some(b"f"));
        let (p2, n2) = decode_multi_chunk_event_profile(&enc2).expect("max2");
        assert_eq!(n2, enc2.len());
        assert_eq!(p2.event_chunk_count, 2);
        assert_eq!(p2.records.len(), 4);
        assert!(p2.has_footer);
        assert_eq!(p2.footer_payload, Some(&b"f"[..]));
    }

    #[test]
    fn empty_events_zero_event_chunks() {
        let enc =
            encode_multi_chunk_event_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[], 2, None);
        let (prof, n) = decode_multi_chunk_event_profile(&enc).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(prof.event_chunk_count, 0);
        assert!(prof.records.is_empty());
        assert_eq!(enc, encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]));
    }

    #[test]
    fn bad_magic_err() {
        let mut enc =
            encode_multi_chunk_event_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[], 1, None);
        enc[0] = b'X';
        assert_eq!(
            decode_multi_chunk_event_profile(&enc),
            Err(MultiChunkEventError::Stream(StreamError::Prefix(
                FilePrefixError::Header(HeaderError::BadMagic)
            )))
        );
    }

    #[test]
    fn bad_sync_err() {
        let mut enc =
            encode_multi_chunk_event_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[], 1, None);
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xCAFE_BABEu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_multi_chunk_event_profile(&enc) {
            Err(MultiChunkEventError::Stream(StreamError::Chunk(ChunkError::BadSync {
                expected,
                got,
            }))) => {
                assert_eq!(expected, CHUNK_SYNC);
                assert_eq!(got, 0xCAFE_BABE);
            }
            other => panic!("expected bad sync, got {other:?}"),
        }
    }

    #[test]
    fn truncated_mid_chunk_err() {
        let events = [EventRecordSpec::TimeLine {
            fid: 1,
            line: 1,
            ticks: 1,
        }];
        let mut enc =
            encode_multi_chunk_event_profile(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &events, 1, None);
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        enc.truncate(prefix_n + CHUNK_HEADER_LEN + 1);
        match decode_multi_chunk_event_profile(&enc) {
            Err(MultiChunkEventError::Stream(StreamError::Chunk(ChunkError::Truncated {
                ..
            }))) => {}
            other => panic!("expected truncated mid-chunk, got {other:?}"),
        }
    }

    #[test]
    fn truncated_mid_record_err() {
        let body = encode_event_body(&[EventRecordSpec::TimeLine {
            fid: 1,
            line: 2,
            ticks: 3,
        }]);
        let trunc = &body[..body.len() - 1];
        let prefix = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let frame = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            1,
            trunc.len() as u32,
            trunc,
            0,
        );
        let mut enc = prefix;
        enc.extend_from_slice(&frame);
        match decode_multi_chunk_event_profile(&enc) {
            Err(MultiChunkEventError::EventBody(_)) => {}
            other => panic!("expected mid-record event-body err, got {other:?}"),
        }
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_multi_chunk_event_profile(&[]).is_err());
        assert!(decode_multi_chunk_event_profile(b"nope").is_err());
        let mut bad_body = encode_u64(opcode::RESERVED);
        bad_body.push(0);
        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        enc.extend_from_slice(&encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            1,
            bad_body.len() as u32,
            &bad_body,
            0,
        ));
        assert!(decode_multi_chunk_event_profile(&enc).is_err());
    }
}

//! Provisional **format v6** multi-chunk EVENT with compressed payloads
//! (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-multi-chunk-compressed-provisional-v0.md`
//!
//! Splits event-body records via shipped `partition_event_records`, seals each
//! partition with NONE/ZLIB/ZSTD/LZ4, and reassembles in order after explicit
//! `decode_chunk_payload`. Default `parse_chunk_frame` stays non-inflating.
//! Not mid-record span, not COL-007 C writer.

use crate::chunk::{codec, encode_chunk_frame, kind};
use crate::compressed_profile::{
    encode_event_chunk, is_supported_event_codec, CompressedProfileError, OwnedEventRecord,
};
use crate::crc::compute_payload_crc;
use crate::event_body::{decode_event_body, encode_event_body, EventBodyError, EventRecordSpec};
use crate::file_prefix::encode_file_prefix;
use crate::multi_chunk_event::partition_event_records;
use crate::payload_codec::decode_chunk_payload;
use crate::stream::decode_prefix_chunk_stream;
use crate::FixedHeader;

/// Fail-closed multi-chunk compressed profile errors (aliases composition errors).
pub type MultiChunkCompressedError = CompressedProfileError;
pub type MultiChunkCompressedResult<T> = std::result::Result<T, MultiChunkCompressedError>;

/// Decoded multi-chunk compressed EVENT profile (owned logical events).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiChunkCompressedProfile {
    pub header: FixedHeader,
    /// Codec of EVENT chunks when present; `NONE` if no EVENT chunks.
    pub event_codec: u8,
    /// Number of EVENT chunks (0 if empty event section).
    pub event_chunk_count: usize,
    /// Flattened records from all EVENT chunks in file order.
    pub records: Vec<OwnedEventRecord>,
    pub has_footer: bool,
    pub footer_payload: Option<Vec<u8>>,
}

/// Encode a multi-chunk EVENT profile with compressed (or NONE) payloads.
///
/// - Partitions `events` with shipped [`partition_event_records`].
/// - Each partition → `encode_event_body` → EVENT frame under `event_codec`.
/// - Optional trailing FOOTER, codec NONE.
///
/// `max_records_per_chunk`: `0` = unlimited (≤1 EVENT); `n >= 1` splits into
/// windows of size `n` (use `1` to force ≥2 EVENT chunks when `events.len() ≥ 2`).
pub fn encode_multi_chunk_compressed_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    event_codec: u8,
    events: &[EventRecordSpec<'_>],
    max_records_per_chunk: usize,
    footer: Option<&[u8]>,
) -> MultiChunkCompressedResult<Vec<u8>> {
    if !events.is_empty() && !is_supported_event_codec(event_codec) {
        return Err(CompressedProfileError::UnsupportedEventCodec {
            codec: event_codec,
        });
    }

    let partitions = partition_event_records(events, max_records_per_chunk);
    let mut out = encode_file_prefix(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
    );

    for (i, part) in partitions.iter().enumerate() {
        let plain = encode_event_body(part);
        let frame = encode_event_chunk(event_codec, i as u64, part.len() as u32, &plain)?;
        out.extend_from_slice(&frame);
    }

    if let Some(fp) = footer {
        let seq = partitions.len() as u64;
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

/// Decode a multi-chunk compressed EVENT profile.
///
/// Non-inflating stream parse → per EVENT `decode_chunk_payload` →
/// `decode_event_body` → ordered record append. Owned labels after inflate.
pub fn decode_multi_chunk_compressed_profile(
    buf: &[u8],
) -> MultiChunkCompressedResult<(MultiChunkCompressedProfile, usize)> {
    let (stream, n) = decode_prefix_chunk_stream(buf)?;
    let mut records = Vec::new();
    let mut event_chunk_count = 0usize;
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
                } else if frame.codec != event_codec {
                    // MVP: all EVENT chunks share one codec.
                    return Err(CompressedProfileError::UnsupportedEventCodec {
                        codec: frame.codec,
                    });
                }
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

    Ok((
        MultiChunkCompressedProfile {
            header: stream.prefix.header,
            event_codec,
            event_chunk_count,
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
    use crate::varint::encode_u64;
    use crate::{MAGIC, SUPPORTED_MAJOR};

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

    fn assert_ordered_four(recs: &[OwnedEventRecord]) {
        assert_eq!(recs.len(), 4);
        match &recs[0] {
            OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
            other => panic!("{other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"a"),
            other => panic!("{other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 20),
            other => panic!("{other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"b"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn multi_chunk_none_zlib_zstd_lz4_ordered() {
        let events = sample_events();
        // max_records_per_chunk=1 → 4 EVENT chunks (≥2).
        assert_eq!(partition_event_records(&events, 1).len(), 4);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let enc_a = encode_multi_chunk_compressed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                1,
                Some(b"end"),
            )
            .expect("encode");
            let enc_b = encode_multi_chunk_compressed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                1,
                Some(b"end"),
            )
            .expect("encode 2");
            assert_eq!(enc_a, enc_b, "deterministic for codec {c}");
            assert_eq!(&enc_a[..8], MAGIC.as_slice());

            let (prof, n) = decode_multi_chunk_compressed_profile(&enc_a).expect("decode");
            assert_eq!(n, enc_a.len());
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.event_chunk_count, 4);
            assert_ordered_four(&prof.records);
            assert!(prof.has_footer);
            assert_eq!(prof.footer_payload.as_deref(), Some(&b"end"[..]));

            // Wire payloads stay compressed under default non-inflating parse.
            let (stream, _) = decode_prefix_chunk_stream(&enc_a).unwrap();
            let event_frames: Vec<_> = stream
                .chunks
                .iter()
                .filter(|f| f.kind == kind::EVENT)
                .collect();
            assert_eq!(event_frames.len(), 4);
            if c != codec::NONE {
                for (i, frame) in event_frames.iter().enumerate() {
                    assert_eq!(frame.codec, c);
                    let part = partition_event_records(&events, 1)[i];
                    let plain = encode_event_body(part);
                    assert_ne!(
                        frame.payload,
                        plain.as_slice(),
                        "chunk {i} must stay compressed for codec {c}"
                    );
                }
            }
        }
    }

    #[test]
    fn multi_chunk_max2_zstd_two_event_chunks() {
        let events = sample_events();
        let enc = encode_multi_chunk_compressed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZSTD,
            &events,
            2,
            None,
        )
        .unwrap();
        let (prof, n) = decode_multi_chunk_compressed_profile(&enc).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(prof.event_chunk_count, 2);
        assert_eq!(prof.event_codec, codec::ZSTD);
        assert_ordered_four(&prof.records);
    }

    #[test]
    fn empty_events_zero_chunks() {
        let enc = encode_multi_chunk_compressed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::LZ4,
            &[],
            1,
            None,
        )
        .unwrap();
        assert_eq!(enc, encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]));
        let (prof, n) = decode_multi_chunk_compressed_profile(&enc).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(prof.event_chunk_count, 0);
        assert!(prof.records.is_empty());
    }

    #[test]
    fn corrupt_zlib_on_second_chunk_err() {
        let events = sample_events();
        let mut enc = encode_multi_chunk_compressed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &events,
            2, // 2 EVENT chunks
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        // Parse first EVENT to find start of second.
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        let f0_len = CHUNK_HEADER_LEN + f0.payload.len();
        let second_off = prefix_n + f0_len;
        let f1 = parse_chunk_frame(&enc[second_off..]).unwrap();
        assert_eq!(f1.codec, codec::ZLIB);
        assert!(f1.payload.len() > 2);
        let payload_off = second_off + CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        enc[payload_off + 1] ^= 0x55;
        match decode_multi_chunk_compressed_profile(&enc) {
            Err(CompressedProfileError::Payload(_)) => {}
            other => panic!("expected payload error on corrupt second chunk, got {other:?}"),
        }
    }

    #[test]
    fn size_mismatch_zstd_first_chunk_err() {
        let events = sample_events();
        let parts = partition_event_records(&events, 2);
        assert_eq!(parts.len(), 2);
        let plain0 = encode_event_body(parts[0]);
        let compressed = compress_zstd(&plain0).unwrap();
        let wrong_len = (plain0.len() as u32).saturating_sub(1).max(1);
        assert_ne!(wrong_len, plain0.len() as u32);

        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        enc.extend_from_slice(&encode_chunk_frame(
            kind::EVENT,
            codec::ZSTD,
            0,
            0,
            0,
            parts[0].len() as u32,
            wrong_len,
            &compressed,
            compute_payload_crc(&compressed),
        ));
        // Second chunk valid for completeness.
        let plain1 = encode_event_body(parts[1]);
        enc.extend_from_slice(
            &encode_event_chunk(codec::ZSTD, 1, parts[1].len() as u32, &plain1).unwrap(),
        );

        match decode_multi_chunk_compressed_profile(&enc) {
            Err(CompressedProfileError::Payload(
                PayloadCodecError::SizeMismatch { .. } | PayloadCodecError::Zstd { .. },
            )) => {}
            other => panic!("expected size/zstd error, got {other:?}"),
        }
    }

    #[test]
    fn uses_shipped_partition_for_split_count() {
        let events = sample_events();
        let parts = partition_event_records(&events, 1);
        let enc = encode_multi_chunk_compressed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::LZ4,
            &events,
            1,
            None,
        )
        .unwrap();
        let (prof, _) = decode_multi_chunk_compressed_profile(&enc).unwrap();
        assert_eq!(prof.event_chunk_count, parts.len());
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_multi_chunk_compressed_profile(&[]).is_err());
        assert!(decode_multi_chunk_compressed_profile(b"nope").is_err());
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
        assert!(decode_multi_chunk_compressed_profile(&enc).is_err());
    }

    #[test]
    fn bad_sync_err() {
        let mut enc = encode_multi_chunk_compressed_profile(
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
        )
        .unwrap();
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xCAFE_BABEu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_multi_chunk_compressed_profile(&enc) {
            Err(CompressedProfileError::Stream(StreamError::Chunk(ChunkError::BadSync {
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
    fn zlib_second_chunk_wire_matches_deflate() {
        let events = sample_events();
        let enc = encode_multi_chunk_compressed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &events,
            2,
            None,
        )
        .unwrap();
        let parts = partition_event_records(&events, 2);
        let plain1 = encode_event_body(parts[1]);
        let expected = deflate_zlib(&plain1).unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        let second_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let f1 = parse_chunk_frame(&enc[second_off..]).unwrap();
        assert_eq!(f1.payload, expected.as_slice());
    }
}

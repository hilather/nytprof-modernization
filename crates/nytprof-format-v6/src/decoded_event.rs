//! Provisional **format v6** decoded EVENT profile consumer path (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-decoded-event-provisional-v0.md`
//!
//! Stream → always-inflate EVENT payloads (optional CRC) → join plain EVENT
//! bytes → `decode_event_body`. Composes shipped decoded-stream + event-body +
//! payload seal helpers. Does **not** change default `parse_chunk_frame`.
//! Not full opcode catalog freeze, not COL-007 C writer, not CLI v6 default.

use crate::chunk::{codec, encode_chunk_frame, kind};
use crate::compressed_profile::{
    encode_event_chunk, is_supported_event_codec, CompressedProfileError, OwnedEventRecord,
};
use crate::crc::compute_payload_crc;
use crate::decoded_stream::{
    decode_prefix_chunk_stream_plain, encode_prefix_sealed_chunks, DecodedStreamError,
};
use crate::event_body::{
    decode_event_body, encode_event_body, EventBodyError, EventRecordSpec,
};
use crate::multi_chunk_event::partition_event_records;
use crate::FixedHeader;

/// Fail-closed decoded-EVENT profile errors.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodedEventError {
    Stream(DecodedStreamError),
    EventBody(EventBodyError),
    Encode(CompressedProfileError),
    /// Non-EVENT/FOOTER kind on this MVP path.
    UnexpectedKind { kind: u8 },
    /// FOOTER not last / more than one FOOTER.
    InvalidFooter,
    /// FOOTER must use codec NONE.
    UnexpectedFooterCodec { codec: u8 },
    /// EVENT codec not in {NONE, ZLIB, ZSTD, LZ4}.
    UnsupportedEventCodec { codec: u8 },
    /// No EVENT chunks present when events were expected on decode of non-empty body path.
    MissingEventChunks,
    /// Mid-stream codec-switch preflight requires START_DEFLATE in the pre-switch EVENT body.
    MissingStartDeflateMarker,
    /// Mid-stream codec-switch preflight requires distinct pre/post payload codecs.
    MidStreamCodecsMustDiffer { codec: u8 },
    /// Body VERSION major/minor disagree with fixed-header / file-prefix version fields.
    VersionHeaderMismatch {
        header_major: u16,
        header_minor: u16,
        body_major: u64,
        body_minor: u64,
    },
}

impl std::fmt::Display for DecodedEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodedEventError::Stream(e) => write!(f, "decoded-event stream: {e}"),
            DecodedEventError::EventBody(e) => write!(f, "decoded-event body: {e}"),
            DecodedEventError::Encode(e) => write!(f, "decoded-event encode: {e}"),
            DecodedEventError::UnexpectedKind { kind } => {
                write!(f, "decoded-event unexpected kind {kind}")
            }
            DecodedEventError::InvalidFooter => write!(f, "decoded-event invalid FOOTER placement"),
            DecodedEventError::UnexpectedFooterCodec { codec } => {
                write!(f, "decoded-event FOOTER codec {codec} (NONE required)")
            }
            DecodedEventError::UnsupportedEventCodec { codec } => {
                write!(f, "decoded-event unsupported EVENT codec {codec}")
            }
            DecodedEventError::MissingEventChunks => {
                write!(f, "decoded-event missing EVENT chunks")
            }
            DecodedEventError::MissingStartDeflateMarker => {
                write!(
                    f,
                    "decoded-event mid-stream codec-switch missing START_DEFLATE in pre region"
                )
            }
            DecodedEventError::MidStreamCodecsMustDiffer { codec } => {
                write!(
                    f,
                    "decoded-event mid-stream codec-switch pre/post codecs must differ (got {codec})"
                )
            }
            DecodedEventError::VersionHeaderMismatch {
                header_major,
                header_minor,
                body_major,
                body_minor,
            } => write!(
                f,
                "decoded-event VERSION body {body_major}.{body_minor} mismatches header {header_major}.{header_minor}"
            ),
        }
    }
}

impl std::error::Error for DecodedEventError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodedEventError::Stream(e) => Some(e),
            DecodedEventError::EventBody(e) => Some(e),
            DecodedEventError::Encode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<DecodedStreamError> for DecodedEventError {
    fn from(e: DecodedStreamError) -> Self {
        DecodedEventError::Stream(e)
    }
}

impl From<EventBodyError> for DecodedEventError {
    fn from(e: EventBodyError) -> Self {
        DecodedEventError::EventBody(e)
    }
}

impl From<CompressedProfileError> for DecodedEventError {
    fn from(e: CompressedProfileError) -> Self {
        DecodedEventError::Encode(e)
    }
}

pub type DecodedEventResult<T> = std::result::Result<T, DecodedEventError>;

/// Decoded EVENT profile: header + ordered logical events after always-inflate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedEventProfile {
    pub header: FixedHeader,
    /// First EVENT chunk codec (compat); see [`Self::event_chunk_codecs`] for per-chunk list.
    pub event_codec: u8,
    /// Payload codec of each EVENT chunk in file order (may differ after START_DEFLATE mid-stream switch preflight).
    pub event_chunk_codecs: Vec<u8>,
    pub event_chunk_count: usize,
    pub records: Vec<OwnedEventRecord>,
    pub has_footer: bool,
    pub footer_payload: Option<Vec<u8>>,
}

/// Build a dump-aligned [`OwnedEventRecord::Version`] from fixed-header major/minor.
#[inline]
pub fn version_record_from_header(header: &FixedHeader) -> OwnedEventRecord {
    OwnedEventRecord::Version {
        major: u64::from(header.major),
        minor: u64::from(header.minor),
    }
}

/// Ensure recovered EVENT records are header-aligned for VERSION (auto-emit preflight).
///
/// - If any body `VERSION` is present, every such record must match
///   `header.major` / `header.minor` (as u64); otherwise [`DecodedEventError::VersionHeaderMismatch`].
/// - If no body `VERSION` is present, **prepend** one synthesized from the header.
///
/// Does not reorder non-VERSION records. Not OI-001-03 sequence-number freeze.
pub fn align_event_records_version_with_header(
    header: &FixedHeader,
    records: &mut Vec<OwnedEventRecord>,
) -> DecodedEventResult<()> {
    let hm = u64::from(header.major);
    let hn = u64::from(header.minor);
    let mut saw_version = false;
    for r in records.iter() {
        if let OwnedEventRecord::Version { major, minor } = r {
            if *major != hm || *minor != hn {
                return Err(DecodedEventError::VersionHeaderMismatch {
                    header_major: header.major,
                    header_minor: header.minor,
                    body_major: *major,
                    body_minor: *minor,
                });
            }
            saw_version = true;
        }
    }
    if !saw_version {
        records.insert(0, version_record_from_header(header));
    }
    Ok(())
}

/// Encode helper: ensure a body VERSION matching `major`/`minor` is present, then seal.
///
/// If `events` already contains any `VERSION`, each must match header fields or encode fails.
/// If none, a VERSION record is prepended before encode. Decode with
/// [`decode_decoded_event_profile_auto_version`] or plain decode (body carries VERSION).
pub fn encode_decoded_event_profile_auto_version(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    event_codec: u8,
    events: &[EventRecordSpec<'_>],
    max_events_per_chunk: usize,
    footer: Option<&[u8]>,
) -> DecodedEventResult<Vec<u8>> {
    let hm = u64::from(major);
    let hn = u64::from(minor);
    let mut saw = false;
    for e in events {
        if let EventRecordSpec::Version {
            major: bm,
            minor: bn,
        } = e
        {
            if *bm != hm || *bn != hn {
                return Err(DecodedEventError::VersionHeaderMismatch {
                    header_major: major,
                    header_minor: minor,
                    body_major: *bm,
                    body_minor: *bn,
                });
            }
            saw = true;
        }
    }
    if saw {
        return encode_decoded_event_profile(
            major,
            minor,
            required_features,
            optional_features,
            header_crc,
            tlv_items,
            event_codec,
            events,
            max_events_per_chunk,
            footer,
        );
    }
    let mut with_ver: Vec<EventRecordSpec<'_>> = Vec::with_capacity(events.len() + 1);
    with_ver.push(EventRecordSpec::Version {
        major: hm,
        minor: hn,
    });
    with_ver.extend_from_slice(events);
    encode_decoded_event_profile(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        event_codec,
        &with_ver,
        max_events_per_chunk,
        footer,
    )
}

/// Encode a provisional decoded-EVENT profile.
///
/// - File prefix
/// - One or more EVENT chunks: `encode_event_body` sealed under `event_codec`
///   (optionally record-partitioned when `max_events_per_chunk` is set and &gt; 0)
/// - Optional FOOTER codec NONE last
///
/// Pure byte-slice / `Vec` API. Does not change default parse inflate policy.
/// Does **not** auto-emit VERSION; use [`encode_decoded_event_profile_auto_version`].
pub fn encode_decoded_event_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    event_codec: u8,
    events: &[EventRecordSpec<'_>],
    max_events_per_chunk: usize,
    footer: Option<&[u8]>,
) -> DecodedEventResult<Vec<u8>> {
    if !events.is_empty() && !is_supported_event_codec(event_codec) {
        return Err(DecodedEventError::UnsupportedEventCodec {
            codec: event_codec,
        });
    }

    let parts = if events.is_empty() {
        Vec::new()
    } else {
        partition_event_records(events, max_events_per_chunk)
    };

    let mut sealed: Vec<Vec<u8>> = Vec::with_capacity(parts.len() + usize::from(footer.is_some()));
    for (i, part) in parts.iter().enumerate() {
        let plain = encode_event_body(part);
        let frame = encode_event_chunk(event_codec, i as u64, part.len() as u32, &plain)?;
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

/// Encode a provisional EVENT profile that **switches payload codec after a START_DEFLATE marker**.
///
/// Wire model (chunk-framed preflight, not v5 mid-payload byte-stream deflate):
/// 1. One EVENT chunk under `pre_codec` (typically NONE) whose body includes
///    `START_DEFLATE` (usually last pre-switch record).
/// 2. One EVENT chunk under `post_codec` (≠ pre) carrying post-switch workload records.
/// 3. Optional FOOTER codec NONE last.
///
/// Decode with [`decode_decoded_event_profile`] (always-inflate join). Does **not**
/// mutate default `parse_chunk_frame`. Not OI-001-03 sequence-number freeze.
pub fn encode_decoded_event_mid_stream_codec_switch_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    pre_codec: u8,
    pre_events: &[EventRecordSpec<'_>],
    post_codec: u8,
    post_events: &[EventRecordSpec<'_>],
    footer: Option<&[u8]>,
) -> DecodedEventResult<Vec<u8>> {
    if pre_events.is_empty() || post_events.is_empty() {
        return Err(DecodedEventError::MissingEventChunks);
    }
    if !is_supported_event_codec(pre_codec) {
        return Err(DecodedEventError::UnsupportedEventCodec { codec: pre_codec });
    }
    if !is_supported_event_codec(post_codec) {
        return Err(DecodedEventError::UnsupportedEventCodec { codec: post_codec });
    }
    if pre_codec == post_codec {
        return Err(DecodedEventError::MidStreamCodecsMustDiffer { codec: post_codec });
    }
    if !pre_events
        .iter()
        .any(|e| matches!(e, EventRecordSpec::StartDeflate))
    {
        return Err(DecodedEventError::MissingStartDeflateMarker);
    }

    let pre_plain = encode_event_body(pre_events);
    let post_plain = encode_event_body(post_events);
    let pre_frame = encode_event_chunk(
        pre_codec,
        0,
        pre_events.len() as u32,
        &pre_plain,
    )?;
    let post_frame = encode_event_chunk(
        post_codec,
        1,
        post_events.len() as u32,
        &post_plain,
    )?;

    let mut sealed: Vec<Vec<u8>> = vec![pre_frame, post_frame];
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

/// Decode via [`decode_decoded_event_profile`], then align VERSION with fixed-header.
///
/// Auto-emits a leading VERSION from `header.major`/`header.minor` when the body
/// has none; fail-closed when a body VERSION mismatches the header.
/// Default `parse_chunk_frame` remains non-inflating.
pub fn decode_decoded_event_profile_auto_version(
    buf: &[u8],
    verify_crc: bool,
) -> DecodedEventResult<(DecodedEventProfile, usize)> {
    let (mut prof, n) = decode_decoded_event_profile(buf, verify_crc)?;
    align_event_records_version_with_header(&prof.header, &mut prof.records)?;
    Ok((prof, n))
}

/// Decode a provisional decoded-EVENT profile.
///
/// 1. Shipped `decode_prefix_chunk_stream_plain` (always inflate + optional CRC)
/// 2. Collect EVENT plains in order; optional trailing FOOTER codec NONE
/// 3. Join EVENT plains → single `decode_event_body`
///
/// EVENT chunks may use **different** supported payload codecs (mid-stream
/// START_DEFLATE codec-switch preflight). Default `parse_chunk_frame` remains
/// non-inflating. Does **not** auto-emit VERSION; use
/// [`decode_decoded_event_profile_auto_version`].
pub fn decode_decoded_event_profile(
    buf: &[u8],
    verify_crc: bool,
) -> DecodedEventResult<(DecodedEventProfile, usize)> {
    let (stream, n) = decode_prefix_chunk_stream_plain(buf, verify_crc)?;
    let mut plain = Vec::new();
    let mut event_chunk_count = 0usize;
    let mut event_chunk_codecs = Vec::new();
    let mut has_footer = false;
    let mut footer_payload: Option<Vec<u8>> = None;
    let mut saw_footer = false;
    let mut event_codec = codec::NONE;

    for chunk in &stream.chunks {
        if saw_footer {
            return Err(DecodedEventError::InvalidFooter);
        }
        match chunk.kind {
            k if k == kind::EVENT => {
                if !is_supported_event_codec(chunk.codec) {
                    return Err(DecodedEventError::UnsupportedEventCodec {
                        codec: chunk.codec,
                    });
                }
                if event_chunk_count == 0 {
                    event_codec = chunk.codec;
                }
                event_chunk_codecs.push(chunk.codec);
                plain.extend_from_slice(&chunk.plain);
                event_chunk_count += 1;
            }
            k if k == kind::FOOTER => {
                if chunk.codec != codec::NONE {
                    return Err(DecodedEventError::UnexpectedFooterCodec {
                        codec: chunk.codec,
                    });
                }
                has_footer = true;
                footer_payload = Some(chunk.plain.clone());
                saw_footer = true;
            }
            other => {
                return Err(DecodedEventError::UnexpectedKind { kind: other });
            }
        }
    }

    if plain.is_empty() && event_chunk_count == 0 {
        return Ok((
            DecodedEventProfile {
                header: stream.header,
                event_codec: codec::NONE,
                event_chunk_codecs: Vec::new(),
                event_chunk_count: 0,
                records: Vec::new(),
                has_footer,
                footer_payload,
            },
            n,
        ));
    }

    if event_chunk_count == 0 {
        return Err(DecodedEventError::MissingEventChunks);
    }

    let (body_recs, body_n) = decode_event_body(&plain)?;
    if body_n != plain.len() {
        return Err(DecodedEventError::EventBody(EventBodyError::Truncated {
            need: plain.len(),
            got: body_n,
        }));
    }

    let mut records = Vec::with_capacity(body_recs.len());
    for r in &body_recs {
        records.push(OwnedEventRecord::from_borrowed(r));
    }

    Ok((
        DecodedEventProfile {
            header: stream.header,
            event_codec,
            event_chunk_codecs,
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
    use crate::chunk::{parse_chunk_frame, CHUNK_HEADER_LEN, CHUNK_SYNC};
    use crate::decoded_chunk::DecodedChunkError;
    use crate::decoded_stream::DecodedStreamError;
    use crate::event_body::known_key_attr_option_sample_specs;
    use crate::payload_codec::deflate_zlib;
    use crate::stream::{decode_prefix_chunk_stream, StreamError};
    use crate::{MAGIC, SUPPORTED_MAJOR};

    fn sample_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 100,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"decoded-event-mark",
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 20,
                ticks: 200,
            },
        ]
    }

    fn assert_sample(recs: &[OwnedEventRecord]) {
        assert_eq!(recs.len(), 3);
        match &recs[0] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 100));
            }
            other => panic!("{other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"decoded-event-mark"),
            other => panic!("{other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 20, 200));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn none_zlib_zstd_lz4_single_chunk_roundtrip() {
        let events = sample_events();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0, // unlimited → one partition
                Some(b"evt-end"),
            )
            .expect("encode");
            assert_eq!(&wire[..8], MAGIC.as_slice());

            let (prof, n) = decode_decoded_event_profile(&wire, true).expect("decode");
            assert_eq!(n, wire.len());
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.event_chunk_count, 1);
            assert_sample(&prof.records);
            assert!(prof.has_footer);
            assert_eq!(prof.footer_payload.as_deref(), Some(&b"evt-end"[..]));

            // Default stream parse stays non-inflating for compressed codecs.
            if c != codec::NONE {
                let (raw, _) = decode_prefix_chunk_stream(&wire).unwrap();
                let ev = raw.chunks.iter().find(|f| f.kind == kind::EVENT).unwrap();
                let body = encode_event_body(&events);
                assert_ne!(ev.payload, body.as_slice());
            }
        }
    }

    #[test]
    fn multi_chunk_record_aligned_zlib_roundtrip() {
        let events = sample_events();
        let wire = encode_decoded_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &events,
            1, // one event per chunk → ≥2 EVENT chunks
            None,
        )
        .expect("encode");
        let (prof, n) = decode_decoded_event_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert!(prof.event_chunk_count >= 2);
        assert_eq!(prof.event_codec, codec::ZLIB);
        assert_sample(&prof.records);

        // Joined plains equal encode_event_body of full set.
        let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
        let mut joined = Vec::new();
        for c in &stream.chunks {
            if c.kind == kind::EVENT {
                joined.extend_from_slice(&c.plain);
            }
        }
        assert_eq!(joined, encode_event_body(&events));
    }

    #[test]
    fn truncated_event_body_join_err() {
        let events = sample_events();
        let mut wire = encode_decoded_event_profile(
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
        )
        .unwrap();
        // Truncate payload of the single EVENT chunk so joined body is incomplete.
        let prefix_n = crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        let payload_off = prefix_n + CHUNK_HEADER_LEN;
        // Keep header but drop half the plain payload bytes and shrink buffer.
        let keep = f0.payload.len() / 2;
        assert!(keep > 0 && keep < f0.payload.len());
        wire.truncate(payload_off + keep);
        // compressed_len in header still claims full length → truncated frame parse.
        match decode_decoded_event_profile(&wire, false) {
            Err(DecodedEventError::Stream(DecodedStreamError::Stream(StreamError::Chunk(
                crate::chunk::ChunkError::Truncated { .. },
            )))) => {}
            // If truncation lands such that frame parses but body is short:
            Err(DecodedEventError::EventBody(_)) => {}
            other => panic!("expected truncated err, got {other:?}"),
        }
    }

    #[test]
    fn corrupt_zlib_event_payload_err() {
        let events = sample_events();
        let mut wire = encode_decoded_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &events,
            0,
            None,
        )
        .unwrap();
        let prefix_n = crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        assert_eq!(f0.codec, codec::ZLIB);
        let payload_len = f0.payload.len();
        let payload_off = prefix_n + CHUNK_HEADER_LEN;
        wire[payload_off] ^= 0xFF;
        if payload_len > 1 {
            wire[payload_off + 1] ^= 0xAA;
        }
        match decode_decoded_event_profile(&wire, false) {
            Err(DecodedEventError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Payload(_),
            ))) => {}
            other => panic!("expected payload err, got {other:?}"),
        }
        match decode_decoded_event_profile(&wire, true) {
            Err(DecodedEventError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc err, got {other:?}"),
        }
    }

    #[test]
    fn crc_mismatch_when_verify_on_err() {
        let events = sample_events();
        let plain = encode_event_body(&events);
        let compressed = deflate_zlib(&plain).unwrap();
        let bad = encode_chunk_frame(
            kind::EVENT,
            codec::ZLIB,
            0,
            0,
            0,
            events.len() as u32,
            plain.len() as u32,
            &compressed,
            compute_payload_crc(&compressed) ^ 0xABCD_EF01,
        );
        let wire = encode_prefix_sealed_chunks(SUPPORTED_MAJOR, 0, 0, 0, 0, &[], &[&bad]);
        match decode_decoded_event_profile(&wire, true) {
            Err(DecodedEventError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc mismatch, got {other:?}"),
        }
        let (prof, n) = decode_decoded_event_profile(&wire, false).expect("no crc");
        assert_eq!(n, wire.len());
        assert_sample(&prof.records);
    }

    #[test]
    fn empty_events_prefix_only() {
        let wire = encode_decoded_event_profile(
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
        let (prof, n) = decode_decoded_event_profile(&wire, true).unwrap();
        assert_eq!(n, wire.len());
        assert_eq!(prof.event_chunk_count, 0);
        assert!(prof.records.is_empty());
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_decoded_event_profile(&[], true).is_err());
        assert!(decode_decoded_event_profile(b"nope", false).is_err());
        let mut enc = crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_decoded_event_profile(&enc, true) {
            Err(DecodedEventError::Stream(DecodedStreamError::Stream(StreamError::Chunk(
                crate::chunk::ChunkError::BadSync { expected, got },
            )))) => {
                assert_eq!(expected, CHUNK_SYNC);
                assert_eq!(got, 0xDEAD_BEEF);
            }
            other => panic!("expected bad sync, got {other:?}"),
        }
    }

    #[test]
    fn zlib_wire_not_plain_event_body() {
        let events = sample_events();
        let plain = encode_event_body(&events);
        let expected = deflate_zlib(&plain).unwrap();
        let wire = encode_decoded_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &events,
            0,
            None,
        )
        .unwrap();
        let prefix_n = crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::EVENT);
        assert_eq!(f0.payload, expected.as_slice());
    }

    fn time_block_sub_entry_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 5,
                block_line: 4,
                ticks: 780,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 1,
                caller_line: 12,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 6,
                ticks: 3,
            },
        ]
    }

    #[test]
    fn time_block_sub_entry_none_zlib_zstd_lz4_roundtrip() {
        let events = time_block_sub_entry_events();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.records.len(), 3, "codec {c}");
            match &prof.records[0] {
                OwnedEventRecord::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                } => assert_eq!((*fid, *line, *block_line, *ticks), (1, 5, 4, 780)),
                other => panic!("codec {c}: expected TimeBlock, got {other:?}"),
            }
            match &prof.records[1] {
                OwnedEventRecord::SubEntry {
                    caller_fid,
                    caller_line,
                } => assert_eq!((*caller_fid, *caller_line), (1, 12)),
                other => panic!("codec {c}: expected SubEntry, got {other:?}"),
            }
            match &prof.records[2] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (1, 6, 3));
                }
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            // Body layer agreement: joined plains match encode_event_body.
            let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, encode_event_body(&events), "codec {c}");
        }
    }

    fn sub_return_sub_info_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::SubReturn {
                depth: 1,
                incl: 900,
                excl: 50,
                string_id: 0,
                string_flags: 0,
                subname: b"main::leaf",
            },
            EventRecordSpec::SubInfo {
                fid: 1,
                first_line: 3,
                last_line: 7,
                string_id: 1,
                string_flags: 0,
                name: b"main::leaf",
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 10,
            },
        ]
    }

    #[test]
    fn sub_return_sub_info_none_zlib_zstd_lz4_roundtrip() {
        let events = sub_return_sub_info_events();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.records.len(), 3, "codec {c}");
            match &prof.records[0] {
                OwnedEventRecord::SubReturn {
                    depth,
                    incl,
                    excl,
                    subname,
                } => {
                    assert_eq!((*depth, *incl, *excl), (1, 900, 50));
                    assert_eq!(subname, b"main::leaf");
                }
                other => panic!("codec {c}: expected SubReturn, got {other:?}"),
            }
            match &prof.records[1] {
                OwnedEventRecord::SubInfo {
                    fid,
                    first_line,
                    last_line,
                    name,
                } => {
                    assert_eq!((*fid, *first_line, *last_line), (1, 3, 7));
                    assert_eq!(name, b"main::leaf");
                }
                other => panic!("codec {c}: expected SubInfo, got {other:?}"),
            }
            match &prof.records[2] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, encode_event_body(&events), "codec {c}");
        }
    }

    fn src_line_new_fid_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::NewFid {
                fid: 1,
                string_id: 0,
                string_flags: 0,
                filename: b"workload.pl",
            },
            EventRecordSpec::SrcLine {
                fid: 1,
                line: 5,
                string_id: 1,
                string_flags: 0,
                text: b"  my $x = 1;",
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 10,
            },
        ]
    }

    #[test]
    fn src_line_new_fid_none_zlib_zstd_lz4_roundtrip() {
        let events = src_line_new_fid_events();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.records.len(), 3, "codec {c}");
            match &prof.records[0] {
                OwnedEventRecord::NewFid { fid, filename } => {
                    assert_eq!(*fid, 1);
                    assert_eq!(filename, b"workload.pl");
                }
                other => panic!("codec {c}: expected NewFid, got {other:?}"),
            }
            match &prof.records[1] {
                OwnedEventRecord::SrcLine { fid, line, text } => {
                    assert_eq!((*fid, *line), (1, 5));
                    assert_eq!(text, b"  my $x = 1;");
                }
                other => panic!("codec {c}: expected SrcLine, got {other:?}"),
            }
            match &prof.records[2] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, encode_event_body(&events), "codec {c}");
        }
    }

    fn pid_start_end_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::PidStart {
                pid: 1001,
                ppid: 1,
                start_time: 1_700_000_000,
            },
            EventRecordSpec::PidEnd {
                pid: 1001,
                end_time: 1_700_000_042,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 10,
            },
        ]
    }

    #[test]
    fn pid_start_end_none_zlib_zstd_lz4_roundtrip() {
        let events = pid_start_end_events();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.records.len(), 3, "codec {c}");
            match &prof.records[0] {
                OwnedEventRecord::PidStart {
                    pid,
                    ppid,
                    start_time,
                } => assert_eq!((*pid, *ppid, *start_time), (1001, 1, 1_700_000_000)),
                other => panic!("codec {c}: expected PidStart, got {other:?}"),
            }
            match &prof.records[1] {
                OwnedEventRecord::PidEnd { pid, end_time } => {
                    assert_eq!((*pid, *end_time), (1001, 1_700_000_042));
                }
                other => panic!("codec {c}: expected PidEnd, got {other:?}"),
            }
            match &prof.records[2] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, encode_event_body(&events), "codec {c}");
        }
    }

    fn sub_callers_discount_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::SubCallers {
                fid: 1,
                line: 10,
                count: 15,
                incl: 900,
                excl: 50,
                reci: 0,
                rec_depth: 0,
                called_string_id: 0,
                called_string_flags: 0,
                called: b"main::leaf",
                caller_string_id: 1,
                caller_string_flags: 0,
                caller: b"main::mid",
            },
            EventRecordSpec::Discount,
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 10,
            },
        ]
    }

    #[test]
    fn sub_callers_discount_none_zlib_zstd_lz4_roundtrip() {
        let events = sub_callers_discount_events();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.records.len(), 3, "codec {c}");
            match &prof.records[0] {
                OwnedEventRecord::SubCallers {
                    fid,
                    line,
                    count,
                    incl,
                    excl,
                    reci,
                    rec_depth,
                    called,
                    caller,
                } => {
                    assert_eq!(
                        (*fid, *line, *count, *incl, *excl, *reci, *rec_depth),
                        (1, 10, 15, 900, 50, 0, 0)
                    );
                    assert_eq!(called, b"main::leaf");
                    assert_eq!(caller, b"main::mid");
                }
                other => panic!("codec {c}: expected SubCallers, got {other:?}"),
            }
            match &prof.records[1] {
                OwnedEventRecord::Discount => {}
                other => panic!("codec {c}: expected Discount, got {other:?}"),
            }
            match &prof.records[2] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, encode_event_body(&events), "codec {c}");
        }
    }

    fn attribute_option_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::Attribute {
                key_string_id: 0,
                key_string_flags: 0,
                key: b"basetime",
                value_string_id: 1,
                value_string_flags: 0,
                value: b"1700000000",
            },
            EventRecordSpec::Option {
                key_string_id: 2,
                key_string_flags: 0,
                key: b"calls",
                value_string_id: 3,
                value_string_flags: 0,
                value: b"1",
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 10,
            },
        ]
    }

    #[test]
    fn attribute_option_none_zlib_zstd_lz4_roundtrip() {
        let events = attribute_option_events();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.records.len(), 3, "codec {c}");
            match &prof.records[0] {
                OwnedEventRecord::Attribute { key, value } => {
                    assert_eq!(key, b"basetime");
                    assert_eq!(value, b"1700000000");
                }
                other => panic!("codec {c}: expected Attribute, got {other:?}"),
            }
            match &prof.records[1] {
                OwnedEventRecord::Option { key, value } => {
                    assert_eq!(key, b"calls");
                    assert_eq!(value, b"1");
                }
                other => panic!("codec {c}: expected Option, got {other:?}"),
            }
            match &prof.records[2] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, encode_event_body(&events), "codec {c}");
        }
    }

    fn comment_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::Comment {
                string_id: 0,
                string_flags: 0,
                text: b"# profiler note",
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 10,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"cmt",
            },
        ]
    }

    #[test]
    fn comment_none_zlib_zstd_lz4_roundtrip() {
        let events = comment_events();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.records.len(), 3, "codec {c}");
            match &prof.records[0] {
                OwnedEventRecord::Comment { text } => assert_eq!(text, b"# profiler note"),
                other => panic!("codec {c}: expected Comment, got {other:?}"),
            }
            match &prof.records[1] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            match &prof.records[2] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"cmt"),
                other => panic!("codec {c}: expected Mark, got {other:?}"),
            }
            let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, encode_event_body(&events), "codec {c}");
        }
    }

    fn start_deflate_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::StartDeflate,
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 10,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"sd",
            },
        ]
    }

    #[test]
    fn start_deflate_none_zlib_zstd_lz4_roundtrip() {
        let events = start_deflate_events();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.records.len(), 3, "codec {c}");
            match &prof.records[0] {
                OwnedEventRecord::StartDeflate => {}
                other => panic!("codec {c}: expected StartDeflate, got {other:?}"),
            }
            match &prof.records[1] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            match &prof.records[2] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"sd"),
                other => panic!("codec {c}: expected Mark, got {other:?}"),
            }
            let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, encode_event_body(&events), "codec {c}");
        }
    }

    fn version_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::Version {
                major: 5,
                minor: 0,
            },
            EventRecordSpec::StartDeflate,
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 10,
            },
        ]
    }

    #[test]
    fn version_none_zlib_zstd_lz4_roundtrip() {
        let events = version_events();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.records.len(), 3, "codec {c}");
            match &prof.records[0] {
                OwnedEventRecord::Version { major, minor } => {
                    assert_eq!((*major, *minor), (5, 0));
                }
                other => panic!("codec {c}: expected Version, got {other:?}"),
            }
            match &prof.records[1] {
                OwnedEventRecord::StartDeflate => {}
                other => panic!("codec {c}: expected StartDeflate, got {other:?}"),
            }
            match &prof.records[2] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, encode_event_body(&events), "codec {c}");
        }
    }

    /// VERSION → meta → START_DEFLATE → PID_START … interior … PID_END (provisional dual-output).
    fn dual_output_sequence_events() -> [EventRecordSpec<'static>; 9] {
        [
            EventRecordSpec::Version {
                major: 5,
                minor: 0,
            },
            EventRecordSpec::Comment {
                string_id: 0,
                string_flags: 0,
                text: b"# dual-output prelude",
            },
            EventRecordSpec::Attribute {
                key_string_id: 0,
                key_string_flags: 0,
                key: b"basetime",
                value_string_id: 1,
                value_string_flags: 0,
                value: b"1700000000",
            },
            EventRecordSpec::Option {
                key_string_id: 2,
                key_string_flags: 0,
                key: b"calls",
                value_string_id: 3,
                value_string_flags: 0,
                value: b"1",
            },
            EventRecordSpec::StartDeflate,
            EventRecordSpec::PidStart {
                pid: 1001,
                ppid: 1,
                start_time: 1_700_000_000,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 42,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"workload",
            },
            EventRecordSpec::PidEnd {
                pid: 1001,
                end_time: 1_700_000_042,
            },
        ]
    }

    fn assert_dual_output_owned(recs: &[OwnedEventRecord]) {
        assert_eq!(recs.len(), 9);
        match &recs[0] {
            OwnedEventRecord::Version { major, minor } => {
                assert_eq!((*major, *minor), (5, 0));
            }
            other => panic!("[0] expected Version, got {other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::Comment { text } => {
                assert_eq!(text, b"# dual-output prelude");
            }
            other => panic!("[1] expected Comment, got {other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::Attribute { key, value } => {
                assert_eq!(key, b"basetime");
                assert_eq!(value, b"1700000000");
            }
            other => panic!("[2] expected Attribute, got {other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::Option { key, value } => {
                assert_eq!(key, b"calls");
                assert_eq!(value, b"1");
            }
            other => panic!("[3] expected Option, got {other:?}"),
        }
        match &recs[4] {
            OwnedEventRecord::StartDeflate => {}
            other => panic!("[4] expected StartDeflate, got {other:?}"),
        }
        match &recs[5] {
            OwnedEventRecord::PidStart {
                pid,
                ppid,
                start_time,
            } => {
                assert_eq!((*pid, *ppid, *start_time), (1001, 1, 1_700_000_000));
            }
            other => panic!("[5] expected PidStart, got {other:?}"),
        }
        match &recs[6] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 42));
            }
            other => panic!("[6] expected TimeLine, got {other:?}"),
        }
        match &recs[7] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"workload"),
            other => panic!("[7] expected Mark, got {other:?}"),
        }
        match &recs[8] {
            OwnedEventRecord::PidEnd { pid, end_time } => {
                assert_eq!((*pid, *end_time), (1001, 1_700_000_042));
            }
            other => panic!("[8] expected PidEnd, got {other:?}"),
        }
    }

    #[test]
    fn dual_output_sequence_none_zlib_zstd_lz4_roundtrip() {
        let events = dual_output_sequence_events();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_dual_output_owned(&prof.records);

            // Default parse_chunk_frame / decode_prefix_chunk_stream stay non-inflating.
            if c != codec::NONE {
                let (stream_raw, _) = decode_prefix_chunk_stream(&wire).expect("raw stream");
                assert!(!stream_raw.chunks.is_empty(), "codec {c}");
                let frame = &stream_raw.chunks[0];
                assert_eq!(frame.codec, c);
                // Compressed payload is not plain event-body under default parse.
                assert_ne!(
                    frame.payload,
                    encode_event_body(&events).as_slice(),
                    "codec {c}: default parse must not expose inflated body as payload"
                );
            }

            let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, encode_event_body(&events), "codec {c}");
        }
    }

    /// Pre-switch prelude ending with START_DEFLATE (chunk codec NONE).
    fn mid_stream_pre_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::Version {
                major: 5,
                minor: 0,
            },
            EventRecordSpec::Comment {
                string_id: 0,
                string_flags: 0,
                text: b"# pre-switch",
            },
            EventRecordSpec::StartDeflate,
        ]
    }

    /// Post-switch workload under compressed EVENT chunk.
    fn mid_stream_post_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::PidStart {
                pid: 1001,
                ppid: 1,
                start_time: 1_700_000_000,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 42,
            },
            EventRecordSpec::PidEnd {
                pid: 1001,
                end_time: 1_700_000_042,
            },
        ]
    }

    fn assert_mid_stream_order(recs: &[OwnedEventRecord]) {
        assert_eq!(recs.len(), 6);
        match &recs[0] {
            OwnedEventRecord::Version { major, minor } => {
                assert_eq!((*major, *minor), (5, 0));
            }
            other => panic!("[0] Version, got {other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::Comment { text } => assert_eq!(text, b"# pre-switch"),
            other => panic!("[1] Comment, got {other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::StartDeflate => {}
            other => panic!("[2] StartDeflate, got {other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::PidStart {
                pid,
                ppid,
                start_time,
            } => assert_eq!((*pid, *ppid, *start_time), (1001, 1, 1_700_000_000)),
            other => panic!("[3] PidStart, got {other:?}"),
        }
        match &recs[4] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 42));
            }
            other => panic!("[4] TimeLine, got {other:?}"),
        }
        match &recs[5] {
            OwnedEventRecord::PidEnd { pid, end_time } => {
                assert_eq!((*pid, *end_time), (1001, 1_700_000_042));
            }
            other => panic!("[5] PidEnd, got {other:?}"),
        }
    }

    #[test]
    fn mid_stream_codec_switch_none_to_zlib_zstd_lz4_roundtrip() {
        let pre = mid_stream_pre_events();
        let post = mid_stream_post_events();
        for post_c in [codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_mid_stream_codec_switch_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                codec::NONE,
                &pre,
                post_c,
                &post,
                Some(b"switch-end"),
            )
            .unwrap_or_else(|e| panic!("encode post_codec {post_c}: {e}"));
            assert_eq!(&wire[..8], MAGIC.as_slice());

            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode post_codec {post_c}: {e}"));
            assert_eq!(n, wire.len(), "post_codec {post_c}");
            assert_eq!(prof.event_chunk_count, 2, "post_codec {post_c}");
            assert_eq!(
                prof.event_chunk_codecs,
                vec![codec::NONE, post_c],
                "post_codec {post_c}"
            );
            assert_eq!(prof.event_codec, codec::NONE, "first codec is pre");
            assert!(prof.has_footer);
            assert_eq!(prof.footer_payload.as_deref(), Some(b"switch-end".as_slice()));
            assert_mid_stream_order(&prof.records);

            // Default non-inflating stream: second EVENT payload is compressed, not plain body.
            let (stream_raw, _) = decode_prefix_chunk_stream(&wire).expect("raw stream");
            assert_eq!(stream_raw.chunks[0].codec, codec::NONE);
            assert_eq!(stream_raw.chunks[1].codec, post_c);
            let post_plain = encode_event_body(&post);
            assert_ne!(
                stream_raw.chunks[1].payload,
                post_plain.as_slice(),
                "post_codec {post_c}: default parse must not inflate"
            );

            // Always-inflate joined plains equal pre||post event-body.
            let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            let mut expect = encode_event_body(&pre);
            expect.extend_from_slice(&encode_event_body(&post));
            assert_eq!(joined, expect, "post_codec {post_c}");
        }
    }

    #[test]
    fn mid_stream_codec_switch_corrupt_post_zlib_err() {
        let pre = mid_stream_pre_events();
        let post = mid_stream_post_events();
        let mut wire = encode_decoded_event_mid_stream_codec_switch_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &pre,
            codec::ZLIB,
            &post,
            None,
        )
        .expect("encode");
        // Corrupt last payload bytes of the second (ZLIB) EVENT chunk.
        let len = wire.len();
        assert!(len > 8);
        wire[len - 4] ^= 0x5a;
        wire[len - 3] ^= 0xa5;
        match decode_decoded_event_profile(&wire, false) {
            Err(DecodedEventError::Stream(_)) | Err(DecodedEventError::EventBody(_)) => {}
            other => panic!("expected corrupt post-switch fail-closed, got {other:?}"),
        }
    }

    #[test]
    fn mid_stream_codec_switch_missing_marker_err() {
        let pre = [EventRecordSpec::Version {
            major: 5,
            minor: 0,
        }];
        let post = mid_stream_post_events();
        match encode_decoded_event_mid_stream_codec_switch_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &pre,
            codec::ZLIB,
            &post,
            None,
        ) {
            Err(DecodedEventError::MissingStartDeflateMarker) => {}
            other => panic!("expected MissingStartDeflateMarker, got {other:?}"),
        }
    }

    #[test]
    fn mid_stream_codec_switch_same_codec_err() {
        let pre = mid_stream_pre_events();
        let post = mid_stream_post_events();
        match encode_decoded_event_mid_stream_codec_switch_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &pre,
            codec::ZLIB,
            &post,
            None,
        ) {
            Err(DecodedEventError::MidStreamCodecsMustDiffer { codec: c }) if c == codec::ZLIB => {}
            other => panic!("expected MidStreamCodecsMustDiffer, got {other:?}"),
        }
    }

    /// Workload without body VERSION — auto-emit path supplies header-aligned VERSION.
    fn auto_version_workload() -> [EventRecordSpec<'static>; 2] {
        [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 42,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"auto-ver",
            },
        ]
    }

    fn assert_auto_version_header_first(recs: &[OwnedEventRecord], header_minor: u16) {
        assert!(recs.len() >= 3, "VERSION + workload");
        match &recs[0] {
            OwnedEventRecord::Version { major, minor } => {
                assert_eq!(*major, u64::from(SUPPORTED_MAJOR), "major from header");
                assert_eq!(*minor, u64::from(header_minor), "minor from header");
            }
            other => panic!("[0] expected Version from header, got {other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 42));
            }
            other => panic!("[1] TimeLine, got {other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"auto-ver"),
            other => panic!("[2] Mark, got {other:?}"),
        }
    }

    #[test]
    fn auto_version_encode_injects_and_roundtrips_none_zlib_zstd_lz4() {
        let workload = auto_version_workload();
        let header_minor = 1u16;
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile_auto_version(
                SUPPORTED_MAJOR,
                header_minor,
                0,
                0,
                0,
                &[],
                c,
                &workload,
                0,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            assert_eq!(&wire[..8], MAGIC.as_slice());

            let (prof, n) = decode_decoded_event_profile_auto_version(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.header.major, SUPPORTED_MAJOR);
            assert_eq!(prof.header.minor, header_minor);
            assert_eq!(prof.event_codec, c);
            assert_auto_version_header_first(&prof.records, header_minor);

            // Header-aligned helper agrees with profile header fields.
            let synthetic = version_record_from_header(&prof.header);
            match (&prof.records[0], &synthetic) {
                (
                    OwnedEventRecord::Version {
                        major: a,
                        minor: b,
                    },
                    OwnedEventRecord::Version {
                        major: c_m,
                        minor: c_n,
                    },
                ) => assert_eq!((*a, *b), (*c_m, *c_n), "codec {c}"),
                other => panic!("codec {c}: {other:?}"),
            }

            // Default stream parse stays non-inflating for compressed codecs.
            if c != codec::NONE {
                let (stream_raw, _) = decode_prefix_chunk_stream(&wire).expect("raw");
                assert_eq!(stream_raw.chunks[0].codec, c);
                // Encoded body includes auto-injected VERSION — payload still compressed.
                assert_ne!(
                    stream_raw.chunks[0].payload.len(),
                    0,
                    "codec {c} has payload"
                );
                let (stream_plain, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
                assert_ne!(
                    stream_raw.chunks[0].payload,
                    stream_plain.chunks[0].plain.as_slice(),
                    "codec {c}: default parse must not inflate"
                );
            }
        }
    }

    #[test]
    fn auto_version_decode_injects_when_body_omits_version() {
        let workload = auto_version_workload();
        let header_minor = 2u16;
        // Plain encode: no VERSION in body.
        let wire = encode_decoded_event_profile(
            SUPPORTED_MAJOR,
            header_minor,
            0,
            0,
            0,
            &[],
            codec::ZSTD,
            &workload,
            0,
            None,
        )
        .expect("encode without VERSION");
        let (plain_prof, _) = decode_decoded_event_profile(&wire, true).expect("plain decode");
        assert!(
            !plain_prof
                .records
                .iter()
                .any(|r| matches!(r, OwnedEventRecord::Version { .. })),
            "body has no VERSION"
        );
        let (auto_prof, n) =
            decode_decoded_event_profile_auto_version(&wire, true).expect("auto decode");
        assert_eq!(n, wire.len());
        assert_eq!(auto_prof.header.minor, header_minor);
        assert_auto_version_header_first(&auto_prof.records, header_minor);
    }

    #[test]
    fn auto_version_body_mismatch_header_fail_closed() {
        let events = [
            EventRecordSpec::Version {
                major: u64::from(SUPPORTED_MAJOR),
                minor: 99, // deliberate mismatch vs header minor 0
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 1,
            },
        ];
        match encode_decoded_event_profile_auto_version(
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
            Err(DecodedEventError::VersionHeaderMismatch {
                header_minor: 0,
                body_minor: 99,
                ..
            }) => {}
            other => panic!("expected VersionHeaderMismatch on encode, got {other:?}"),
        }

        // Wire with mismatched body VERSION still decodes plain, but auto path fails.
        let wire = encode_decoded_event_profile(
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
        )
        .expect("plain encode allows body VERSION independent of minor");
        match decode_decoded_event_profile_auto_version(&wire, true) {
            Err(DecodedEventError::VersionHeaderMismatch {
                header_major,
                header_minor,
                body_major,
                body_minor,
            }) => {
                assert_eq!(header_major, SUPPORTED_MAJOR);
                assert_eq!(header_minor, 0);
                assert_eq!(body_major, u64::from(SUPPORTED_MAJOR));
                assert_eq!(body_minor, 99);
            }
            other => panic!("expected VersionHeaderMismatch on decode, got {other:?}"),
        }
    }

    #[test]
    fn auto_version_matching_body_version_ok() {
        let events = [
            EventRecordSpec::Version {
                major: u64::from(SUPPORTED_MAJOR),
                minor: 3,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"ok",
            },
        ];
        let wire = encode_decoded_event_profile_auto_version(
            SUPPORTED_MAJOR,
            3,
            0,
            0,
            0,
            &[],
            codec::LZ4,
            &events,
            0,
            None,
        )
        .expect("matching VERSION");
        let (prof, _) = decode_decoded_event_profile_auto_version(&wire, true).expect("decode");
        assert_eq!(prof.header.minor, 3);
        match &prof.records[0] {
            OwnedEventRecord::Version { major, minor } => {
                assert_eq!((*major, *minor), (u64::from(SUPPORTED_MAJOR), 3));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(prof.records.len(), 2);
    }

    fn assert_known_key_owned(recs: &[OwnedEventRecord]) {
        assert_eq!(recs.len(), 5);
        match &recs[0] {
            OwnedEventRecord::Attribute { key, value } => {
                assert_eq!(key, crate::known_key::BASETIME);
                assert_eq!(value, b"1786111723");
            }
            other => panic!("[0] basetime, got {other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::Attribute { key, value } => {
                assert_eq!(key, crate::known_key::TICKS_PER_SEC);
                assert_eq!(value, b"10000000");
            }
            other => panic!("[1] ticks_per_sec, got {other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::Attribute { key, value } => {
                assert_eq!(key, crate::known_key::APPLICATION);
                assert_eq!(value, b"workload.pl");
            }
            other => panic!("[2] application, got {other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::Option { key, value } => {
                assert_eq!(key, crate::known_key::CALLS);
                assert_eq!(value, b"1");
            }
            other => panic!("[3] calls, got {other:?}"),
        }
        match &recs[4] {
            OwnedEventRecord::Option { key, value } => {
                assert_eq!(key, crate::known_key::BLOCKS);
                assert_eq!(value, b"0");
            }
            other => panic!("[4] blocks, got {other:?}"),
        }
    }

    #[test]
    fn known_key_attr_option_none_zlib_zstd_lz4_roundtrip() {
        let events = known_key_attr_option_sample_specs();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_known_key_owned(&prof.records);

            // Keys present are provisional known keys.
            for r in &prof.records {
                match r {
                    OwnedEventRecord::Attribute { key, .. } => {
                        assert!(
                            crate::known_key::is_known_attribute_key(key),
                            "codec {c}: {key:?}"
                        );
                    }
                    OwnedEventRecord::Option { key, .. } => {
                        assert!(
                            crate::known_key::is_known_option_key(key),
                            "codec {c}: {key:?}"
                        );
                    }
                    other => panic!("codec {c}: unexpected {other:?}"),
                }
            }

            if c != codec::NONE {
                let (stream_raw, _) = decode_prefix_chunk_stream(&wire).expect("raw");
                assert_eq!(stream_raw.chunks[0].codec, c);
                let (stream_plain, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
                assert_ne!(
                    stream_raw.chunks[0].payload,
                    stream_plain.chunks[0].plain.as_slice(),
                    "codec {c}: default parse must not inflate"
                );
            }

            let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, encode_event_body(&events), "codec {c}");
        }
    }

    /// Build EVENT plain with known … length-framed unknown skip … known.
    fn plain_with_unknown_optional_skip() -> Vec<u8> {
        let mut plain = encode_event_body(&[EventRecordSpec::TimeLine {
            fid: 1,
            line: 10,
            ticks: 42,
        }]);
        plain.extend_from_slice(
            &crate::event_body::encode_unknown_optional_skip_record(99, b"ext-payload")
                .expect("skip record"),
        );
        plain.extend_from_slice(&encode_event_body(&[EventRecordSpec::Mark {
            string_id: 0,
            string_flags: 0,
            label: b"after-skip",
        }]));
        plain
    }

    fn seal_event_plain_profile(codec_id: u8, plain: &[u8]) -> Vec<u8> {
        let frame = crate::compressed_profile::encode_event_chunk(
            codec_id,
            0,
            2, // two logical known records (skip not counted)
            plain,
        )
        .expect("seal EVENT");
        encode_prefix_sealed_chunks(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            &[frame.as_slice()],
        )
    }

    #[test]
    fn unknown_optional_skip_none_zlib_zstd_lz4_always_inflate() {
        let plain = plain_with_unknown_optional_skip();
        // Body layer recovers neighbors.
        let (body_recs, n) = crate::event_body::decode_event_body(&plain).unwrap();
        assert_eq!(n, plain.len());
        assert_eq!(body_recs.len(), 2);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = seal_event_plain_profile(c, &plain);
            let (prof, wn) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(wn, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(prof.records.len(), 2, "codec {c}: skip not emitted");
            match &prof.records[0] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (1, 10, 42));
                }
                other => panic!("codec {c}: TimeLine, got {other:?}"),
            }
            match &prof.records[1] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"after-skip"),
                other => panic!("codec {c}: Mark, got {other:?}"),
            }

            if c != codec::NONE {
                let (stream_raw, _) = decode_prefix_chunk_stream(&wire).expect("raw");
                assert_eq!(stream_raw.chunks[0].codec, c);
                let (stream_plain, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
                assert_ne!(
                    stream_raw.chunks[0].payload,
                    stream_plain.chunks[0].plain.as_slice(),
                    "codec {c}: default parse must not inflate"
                );
            }
        }
    }
}

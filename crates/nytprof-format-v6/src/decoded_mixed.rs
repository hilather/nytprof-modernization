//! Provisional **format v6** decoded multi-kind mixed profile (COL-007 runway).
//!
//! Schemas:
//! - `docs/schemas/v6-decoded-mixed-provisional-v0.md`
//! - `docs/schemas/v6-decoded-mixed-multi-chunk-provisional-v0.md`
//! - `docs/schemas/v6-decoded-mixed-mid-record-provisional-v0.md`
//! - `docs/schemas/v6-decoded-mixed-mid-record-source-provisional-v0.md`
//! - `docs/schemas/v6-decoded-mixed-mid-record-index-provisional-v0.md`
//! - `docs/schemas/v6-decoded-mixed-mid-record-summary-provisional-v0.md`
//! - `docs/schemas/v6-decoded-mixed-mid-record-concurrent-provisional-v0.md`
//!
//! Always-inflate multi-kind consumer: `decode_prefix_chunk_stream_plain`
//! (optional CRC) → join same-kind plains → `decode_*_body` per kind.
//! Encode reuses shipped `encode_compressed_mixed_profile_per_kind`,
//! multi-chunk partition encode, and mid-record body split helpers.
//! Does **not** change default `parse_chunk_frame`. Not COL-007 C writer.

use crate::chunk::{codec, encode_chunk_frame, kind};
use crate::compressed_mixed::{
    encode_compressed_mixed_profile_per_kind, encode_multi_chunk_summary_mixed_profile,
    KindCodecs, OwnedFooterRecord, OwnedIndexRecord, OwnedSourceRecord, OwnedSummaryRecord,
    CompressedMixedError,
};
use crate::compressed_profile::{
    encode_event_chunk, encode_kind_chunk, is_supported_event_codec, CompressedProfileError,
    OwnedEventRecord,
};
use crate::decoded_event::{
    align_event_records_version_with_header, DecodedEventError,
};
use crate::crc::compute_payload_crc;
use crate::decoded_stream::{
    decode_prefix_chunk_stream_plain, encode_prefix_sealed_chunks, DecodedStreamError,
};
use crate::event_body::{
    decode_event_body_full, encode_event_body, encode_event_body_with_site_deltas_and_seq_continuing,
    PackingEncodeState, EventBodyError, EventRecordSpec,
};
use crate::multi_chunk_event::partition_event_records;
use crate::string_dict::{
    decode_string_dictionary, encode_string_dictionary, resolve_event_records, StringDictError,
    StringDictionary,
};
use crate::footer_body::{decode_footer_body, encode_footer_body, FooterBodyError, FooterRecordSpec};
use crate::index_body::{decode_index_body, encode_index_body, IndexBodyError, IndexRecordSpec};
use crate::mid_record_span::{
    split_event_body_bytes, split_index_body_bytes, split_source_body_bytes,
    split_summary_body_bytes,
};
use crate::source_body::{decode_source_body, encode_source_body, SourceBodyError, SourceRecordSpec};
use crate::summary_body::{
    decode_summary_body, encode_summary_body, SummaryBodyError, SummaryRecordSpec,
};
use crate::FixedHeader;

/// Fail-closed decoded-mixed errors.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodedMixedError {
    Stream(DecodedStreamError),
    Encode(CompressedMixedError),
    /// Kind-chunk seal / mid-record encode errors.
    KindEncode(CompressedProfileError),
    EventBody(EventBodyError),
    SourceBody(SourceBodyError),
    IndexBody(IndexBodyError),
    SummaryBody(SummaryBodyError),
    FooterBody(FooterBodyError),
    UnsupportedCodec { codec: u8 },
    UnexpectedKind { kind: u8 },
    InvalidFooter,
    UnexpectedFooterCodec { codec: u8 },
    /// Mixed codecs within the same kind across chunks (MVP).
    KindCodecMismatch { kind: u8, expected: u8, got: u8 },
    /// Concurrent mid-record encode requires ≥2 kinds with interior splits.
    NeedConcurrentMidRecordKinds { got: usize },
    /// Body VERSION major/minor disagree with fixed-header / file-prefix version fields.
    VersionHeaderMismatch {
        header_major: u16,
        header_minor: u16,
        body_major: u64,
        body_minor: u64,
    },
    StringDict(StringDictError),
    /// String-dictionary preflight expects FOOTER payload carrying the dictionary table.
    MissingStringDictionaryFooter,
}

impl std::fmt::Display for DecodedMixedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodedMixedError::Stream(e) => write!(f, "decoded-mixed stream: {e}"),
            DecodedMixedError::Encode(e) => write!(f, "decoded-mixed encode: {e}"),
            DecodedMixedError::KindEncode(e) => write!(f, "decoded-mixed kind encode: {e}"),
            DecodedMixedError::EventBody(e) => write!(f, "decoded-mixed event-body: {e}"),
            DecodedMixedError::SourceBody(e) => write!(f, "decoded-mixed source-body: {e}"),
            DecodedMixedError::IndexBody(e) => write!(f, "decoded-mixed index-body: {e}"),
            DecodedMixedError::SummaryBody(e) => write!(f, "decoded-mixed summary-body: {e}"),
            DecodedMixedError::FooterBody(e) => write!(f, "decoded-mixed footer-body: {e}"),
            DecodedMixedError::UnsupportedCodec { codec } => {
                write!(f, "decoded-mixed unsupported codec {codec}")
            }
            DecodedMixedError::UnexpectedKind { kind } => {
                write!(f, "decoded-mixed unexpected kind {kind}")
            }
            DecodedMixedError::InvalidFooter => write!(f, "decoded-mixed invalid FOOTER placement"),
            DecodedMixedError::UnexpectedFooterCodec { codec } => {
                write!(f, "decoded-mixed FOOTER codec {codec} (NONE required)")
            }
            DecodedMixedError::KindCodecMismatch {
                kind,
                expected,
                got,
            } => write!(
                f,
                "decoded-mixed kind {kind} codec mismatch: expected {expected}, got {got}"
            ),
            DecodedMixedError::NeedConcurrentMidRecordKinds { got } => write!(
                f,
                "decoded-mixed concurrent mid-record needs ≥2 mid-split kinds, got {got}"
            ),
            DecodedMixedError::VersionHeaderMismatch {
                header_major,
                header_minor,
                body_major,
                body_minor,
            } => write!(
                f,
                "decoded-mixed VERSION body {body_major}.{body_minor} mismatches header {header_major}.{header_minor}"
            ),
            DecodedMixedError::StringDict(e) => write!(f, "decoded-mixed string-dict: {e}"),
            DecodedMixedError::MissingStringDictionaryFooter => {
                write!(f, "decoded-mixed missing string-dictionary FOOTER payload")
            }
        }
    }
}

impl std::error::Error for DecodedMixedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodedMixedError::Stream(e) => Some(e),
            DecodedMixedError::Encode(e) => Some(e),
            DecodedMixedError::KindEncode(e) => Some(e),
            DecodedMixedError::EventBody(e) => Some(e),
            DecodedMixedError::SourceBody(e) => Some(e),
            DecodedMixedError::IndexBody(e) => Some(e),
            DecodedMixedError::SummaryBody(e) => Some(e),
            DecodedMixedError::FooterBody(e) => Some(e),
            _ => None,
        }
    }
}

impl From<DecodedStreamError> for DecodedMixedError {
    fn from(e: DecodedStreamError) -> Self {
        DecodedMixedError::Stream(e)
    }
}
impl From<CompressedMixedError> for DecodedMixedError {
    fn from(e: CompressedMixedError) -> Self {
        DecodedMixedError::Encode(e)
    }
}
impl From<CompressedProfileError> for DecodedMixedError {
    fn from(e: CompressedProfileError) -> Self {
        DecodedMixedError::KindEncode(e)
    }
}
impl From<EventBodyError> for DecodedMixedError {
    fn from(e: EventBodyError) -> Self {
        DecodedMixedError::EventBody(e)
    }
}
impl From<SourceBodyError> for DecodedMixedError {
    fn from(e: SourceBodyError) -> Self {
        DecodedMixedError::SourceBody(e)
    }
}
impl From<IndexBodyError> for DecodedMixedError {
    fn from(e: IndexBodyError) -> Self {
        DecodedMixedError::IndexBody(e)
    }
}
impl From<SummaryBodyError> for DecodedMixedError {
    fn from(e: SummaryBodyError) -> Self {
        DecodedMixedError::SummaryBody(e)
    }
}
impl From<FooterBodyError> for DecodedMixedError {
    fn from(e: FooterBodyError) -> Self {
        DecodedMixedError::FooterBody(e)
    }
}
impl From<StringDictError> for DecodedMixedError {
    fn from(e: StringDictError) -> Self {
        DecodedMixedError::StringDict(e)
    }
}

pub type DecodedMixedResult<T> = std::result::Result<T, DecodedMixedError>;

/// Decoded multi-kind mixed profile after always-inflate (+ optional CRC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedMixedProfile {
    pub header: FixedHeader,
    /// Per-kind primary codec (first chunk of that kind). EVENT may mid-stream switch;
    /// see [`Self::event_chunk_codecs`].
    pub kind_codecs: KindCodecs,
    /// Payload codec of each EVENT chunk in file order (may differ after START_DEFLATE switch).
    pub event_chunk_codecs: Vec<u8>,
    pub event_records: Vec<OwnedEventRecord>,
    /// Parallel to [`Self::event_records`]: provisional logical sequence numbers
    /// when the EVENT body used `FLAG_HAS_SEQ`; otherwise all `None`.
    /// Length always equals `event_records.len()`. OI-001-03 runway only.
    pub event_sequences: Vec<Option<u64>>,
    pub source_records: Vec<OwnedSourceRecord>,
    pub index_records: Vec<OwnedIndexRecord>,
    pub summary_records: Vec<OwnedSummaryRecord>,
    pub footer_records: Vec<OwnedFooterRecord>,
    pub event_chunk_count: usize,
    pub source_chunk_count: usize,
    pub index_chunk_count: usize,
    pub summary_chunk_count: usize,
    pub has_footer: bool,
}

/// Encode a provisional decoded multi-kind mixed profile (one chunk per non-empty kind).
///
/// Wire order when non-empty: EVENT, SOURCE, INDEX, SUMMARY, optional FOOTER last.
/// Reuses shipped [`encode_compressed_mixed_profile_per_kind`].
/// Does **not** auto-emit VERSION; use [`encode_decoded_mixed_profile_auto_version`].
pub fn encode_decoded_mixed_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> DecodedMixedResult<Vec<u8>> {
    Ok(encode_compressed_mixed_profile_per_kind(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        codecs,
        events,
        sources,
        indexes,
        summaries,
        footer,
    )?)
}

/// Encode mixed profile with provisional string-dictionary table as structured FOOTER body bytes.
///
/// The FOOTER payload is the raw dictionary table (not footer-body records). Decode with
/// [`decode_decoded_mixed_profile_with_string_dict`].
pub fn encode_decoded_mixed_profile_with_string_dict(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    dict_entries: &[(u64, u8, &[u8])],
) -> DecodedMixedResult<Vec<u8>> {
    let dict_bytes = encode_string_dictionary(dict_entries)?;
    // Build mixed without structured footer records, then append FOOTER frame manually.
    let base = encode_decoded_mixed_profile(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        codecs,
        events,
        sources,
        indexes,
        summaries,
        None,
    )?;
    let checksum = compute_payload_crc(&dict_bytes);
    let footer = encode_chunk_frame(
        kind::FOOTER,
        codec::NONE,
        0,
        0,
        0,
        0,
        dict_bytes.len() as u32,
        &dict_bytes,
        checksum,
    );
    let mut out = base;
    out.extend_from_slice(&footer);
    Ok(out)
}

/// Encode mixed profile with **composed** FOOTER string-dictionary + site-delta/seq EVENT packing.
///
/// EVENT body uses continuous site/seq packing
/// ([`encode_event_body_with_site_deltas_and_seq_continuing`]); SOURCE/INDEX/SUMMARY absolute
/// as usual; FOOTER is the raw dictionary table (codec NONE).
///
/// `max_events_per_chunk == 0` → single EVENT chunk; `>= 1` → record-aligned multi-chunk
/// EVENT partition with packing bases continuing across chunks.
///
/// Decode with [`decode_decoded_mixed_profile_with_string_dict`].
///
/// Not a permanent global string-pool or packing ADR. Default parse stays non-inflating.
pub fn encode_decoded_mixed_profile_with_string_dict_and_site_deltas_and_seq(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    max_events_per_chunk: usize,
    dict_entries: &[(u64, u8, &[u8])],
) -> DecodedMixedResult<Vec<u8>> {
    if !events.is_empty() && !is_supported_event_codec(codecs.event) {
        return Err(DecodedMixedError::UnsupportedCodec {
            codec: codecs.event,
        });
    }

    let event_parts = if events.is_empty() {
        Vec::new()
    } else {
        partition_event_records(events, max_events_per_chunk)
    };

    let source_plain = if sources.is_empty() {
        Vec::new()
    } else {
        encode_source_body(sources)
    };
    let index_plain = if indexes.is_empty() {
        Vec::new()
    } else {
        encode_index_body(indexes)
    };
    let summary_plain = if summaries.is_empty() {
        Vec::new()
    } else {
        encode_summary_body(summaries)
    };
    let dict_bytes = encode_string_dictionary(dict_entries)?;

    let mut sealed: Vec<Vec<u8>> = Vec::with_capacity(
        event_parts.len()
            + usize::from(!source_plain.is_empty())
            + usize::from(!index_plain.is_empty())
            + usize::from(!summary_plain.is_empty())
            + 1,
    );
    let mut packing = PackingEncodeState::new();
    let mut frame_seq = 0u64;
    for part in event_parts.iter() {
        let plain = encode_event_body_with_site_deltas_and_seq_continuing(part, &mut packing)?;
        sealed.push(encode_event_chunk(
            codecs.event,
            frame_seq,
            part.len() as u32,
            &plain,
        )?);
        frame_seq += 1;
    }
    if !source_plain.is_empty() {
        sealed.push(encode_kind_chunk(
            kind::SOURCE,
            codecs.source,
            frame_seq,
            sources.len() as u32,
            &source_plain,
        )?);
        frame_seq += 1;
    }
    if !index_plain.is_empty() {
        sealed.push(encode_kind_chunk(
            kind::INDEX,
            codecs.index,
            frame_seq,
            indexes.len() as u32,
            &index_plain,
        )?);
        frame_seq += 1;
    }
    if !summary_plain.is_empty() {
        sealed.push(encode_kind_chunk(
            kind::SUMMARY,
            codecs.summary,
            frame_seq,
            summaries.len() as u32,
            &summary_plain,
        )?);
        frame_seq += 1;
    }
    let checksum = compute_payload_crc(&dict_bytes);
    sealed.push(encode_chunk_frame(
        kind::FOOTER,
        codec::NONE,
        0,
        frame_seq,
        0,
        0,
        dict_bytes.len() as u32,
        &dict_bytes,
        checksum,
    ));
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

/// Decode mixed profile and resolve EVENT string_ids via FOOTER dictionary table.
///
/// FOOTER payload is treated as a raw string-dictionary table (not footer-body records).
pub fn decode_decoded_mixed_profile_with_string_dict(
    buf: &[u8],
    verify_crc: bool,
) -> DecodedMixedResult<(DecodedMixedProfile, StringDictionary, usize)> {
    let (stream, n) = decode_prefix_chunk_stream_plain(buf, verify_crc)?;

    let mut event_plain = Vec::new();
    let mut source_plain = Vec::new();
    let mut index_plain = Vec::new();
    let mut summary_plain = Vec::new();
    let mut footer_raw: Option<Vec<u8>> = None;

    let mut event_chunk_count = 0usize;
    let mut event_chunk_codecs = Vec::new();
    let mut source_chunk_count = 0usize;
    let mut index_chunk_count = 0usize;
    let mut summary_chunk_count = 0usize;
    let mut has_footer = false;
    let mut saw_footer = false;
    let mut kind_codecs = KindCodecs::uniform(codec::NONE);

    for chunk in &stream.chunks {
        if saw_footer {
            return Err(DecodedMixedError::InvalidFooter);
        }
        match chunk.kind {
            k if k == kind::EVENT => {
                if chunk.codec != codec::NONE
                    && chunk.codec != codec::ZLIB
                    && chunk.codec != codec::ZSTD
                    && chunk.codec != codec::LZ4
                {
                    return Err(DecodedMixedError::UnsupportedCodec {
                        codec: chunk.codec,
                    });
                }
                if event_chunk_count == 0 {
                    kind_codecs.event = chunk.codec;
                }
                event_chunk_codecs.push(chunk.codec);
                event_chunk_count += 1;
                event_plain.extend_from_slice(&chunk.plain);
            }
            k if k == kind::SOURCE => {
                note_kind_codec(
                    &mut kind_codecs,
                    &mut source_chunk_count,
                    kind::SOURCE,
                    chunk.codec,
                )?;
                source_plain.extend_from_slice(&chunk.plain);
            }
            k if k == kind::INDEX => {
                note_kind_codec(
                    &mut kind_codecs,
                    &mut index_chunk_count,
                    kind::INDEX,
                    chunk.codec,
                )?;
                index_plain.extend_from_slice(&chunk.plain);
            }
            k if k == kind::SUMMARY => {
                note_kind_codec(
                    &mut kind_codecs,
                    &mut summary_chunk_count,
                    kind::SUMMARY,
                    chunk.codec,
                )?;
                summary_plain.extend_from_slice(&chunk.plain);
            }
            k if k == kind::FOOTER => {
                if chunk.codec != codec::NONE {
                    return Err(DecodedMixedError::UnexpectedFooterCodec {
                        codec: chunk.codec,
                    });
                }
                has_footer = true;
                footer_raw = Some(chunk.plain.clone());
                saw_footer = true;
            }
            other => {
                return Err(DecodedMixedError::UnexpectedKind { kind: other });
            }
        }
    }

    let footer = footer_raw.ok_or(DecodedMixedError::MissingStringDictionaryFooter)?;
    let (dict, dict_n) = decode_string_dictionary(&footer)?;
    if dict_n != footer.len() {
        return Err(DecodedMixedError::StringDict(StringDictError::Truncated {
            need: footer.len(),
            got: dict_n,
        }));
    }

    let mut event_records = Vec::new();
    let mut event_sequences = Vec::new();
    if !event_plain.is_empty() {
        let (decoded_body, body_n) = decode_event_body_full(&event_plain)?;
        if body_n != event_plain.len() {
            return Err(DecodedMixedError::EventBody(EventBodyError::Truncated {
                need: event_plain.len(),
                got: body_n,
            }));
        }
        event_records = resolve_event_records(&decoded_body.records, &dict)?;
        event_sequences = decoded_body.sequences;
    }

    let mut source_records = Vec::new();
    if !source_plain.is_empty() {
        let (recs, body_n) = decode_source_body(&source_plain)?;
        if body_n != source_plain.len() {
            return Err(DecodedMixedError::SourceBody(SourceBodyError::Truncated {
                need: source_plain.len(),
                got: body_n,
            }));
        }
        for r in recs {
            source_records.push(OwnedSourceRecord {
                fid: r.fid,
                line: r.line,
                text: r.text.data.to_vec(),
            });
        }
    }

    let mut index_records = Vec::new();
    if !index_plain.is_empty() {
        let (recs, body_n) = decode_index_body(&index_plain)?;
        if body_n != index_plain.len() {
            return Err(DecodedMixedError::IndexBody(IndexBodyError::Truncated {
                need: index_plain.len(),
                got: body_n,
            }));
        }
        for r in recs {
            index_records.push(OwnedIndexRecord {
                key_id: r.key_id,
                file_offset: r.file_offset,
                length: r.length,
                label: r.label.data.to_vec(),
            });
        }
    }

    let mut summary_records = Vec::new();
    if !summary_plain.is_empty() {
        let (recs, body_n) = decode_summary_body(&summary_plain)?;
        if body_n != summary_plain.len() {
            return Err(DecodedMixedError::SummaryBody(SummaryBodyError::Truncated {
                need: summary_plain.len(),
                got: body_n,
            }));
        }
        for r in recs {
            summary_records.push(OwnedSummaryRecord {
                key_id: r.key_id,
                count: r.count,
                value: r.value,
                label: r.label.data.to_vec(),
            });
        }
    }

    Ok((
        DecodedMixedProfile {
            header: stream.header,
            kind_codecs,
            event_chunk_codecs,
            event_records,
            event_sequences,
            source_records,
            index_records,
            summary_records,
            footer_records: Vec::new(), // raw dict footer, not structured footer-body
            event_chunk_count,
            source_chunk_count,
            index_chunk_count,
            summary_chunk_count,
            has_footer,
        },
        dict,
        n,
    ))
}

/// Decode FOOTER string-dict resolve, then auto-align EVENT VERSION with fixed-header.
///
/// Compose preflight for auto-VERSION + packing + FOOTER dict. Fail-closed on
/// VERSION mismatch or unknown dict id; keeps event sequences aligned when a
/// synthetic VERSION is prepended.
pub fn decode_decoded_mixed_profile_auto_version_with_string_dict(
    buf: &[u8],
    verify_crc: bool,
) -> DecodedMixedResult<(DecodedMixedProfile, StringDictionary, usize)> {
    let (mut prof, dict, n) = decode_decoded_mixed_profile_with_string_dict(buf, verify_crc)?;
    let before = prof.event_records.len();
    match align_event_records_version_with_header(&prof.header, &mut prof.event_records) {
        Ok(()) => {
            if prof.event_records.len() == before + 1 {
                prof.event_sequences.insert(0, None);
            }
            debug_assert_eq!(prof.event_records.len(), prof.event_sequences.len());
            Ok((prof, dict, n))
        }
        Err(DecodedEventError::VersionHeaderMismatch {
            header_major,
            header_minor,
            body_major,
            body_minor,
        }) => Err(DecodedMixedError::VersionHeaderMismatch {
            header_major,
            header_minor,
            body_major,
            body_minor,
        }),
        Err(other) => Err(DecodedMixedError::KindEncode(match other {
            DecodedEventError::Encode(e) => e,
            _ => CompressedProfileError::UnsupportedEventCodec { codec: 0xff },
        })),
    }
}

/// Validate header-matching VERSION or prepend one (mixed auto-version preflight).
fn mixed_events_with_auto_version<'a>(
    major: u16,
    minor: u16,
    events: &'a [EventRecordSpec<'a>],
) -> DecodedMixedResult<std::borrow::Cow<'a, [EventRecordSpec<'a>]>> {
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
                return Err(DecodedMixedError::VersionHeaderMismatch {
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
        return Ok(std::borrow::Cow::Borrowed(events));
    }
    let mut with_ver: Vec<EventRecordSpec<'a>> = Vec::with_capacity(events.len() + 1);
    with_ver.push(EventRecordSpec::Version {
        major: hm,
        minor: hn,
    });
    with_ver.extend_from_slice(events);
    Ok(std::borrow::Cow::Owned(with_ver))
}

/// Encode mixed profile with dump-aligned VERSION auto-emit from `major`/`minor`.
///
/// Prepends `VERSION` matching the header when EVENT body has none; fail-closed if a
/// body VERSION mismatches. Other kinds unchanged. Absolute EVENT body (not packing).
/// For packing compose see
/// [`encode_decoded_mixed_profile_auto_version_with_site_deltas_and_seq`].
pub fn encode_decoded_mixed_profile_auto_version(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> DecodedMixedResult<Vec<u8>> {
    let with_ver = mixed_events_with_auto_version(major, minor, events)?;
    encode_decoded_mixed_profile(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        codecs,
        with_ver.as_ref(),
        sources,
        indexes,
        summaries,
        footer,
    )
}

/// Encode mixed profile: auto-emit/validate VERSION, then site-delta/seq packing on EVENT
/// with optional multi-chunk continuity (no FOOTER dict required).
///
/// SOURCE/INDEX/SUMMARY absolute as usual. Decode with
/// [`decode_decoded_mixed_profile_auto_version`]. Not dual-equality freeze /
/// permanent packing ADR / COL-007 C writer.
///
/// For FOOTER string-dictionary compose see
/// [`encode_decoded_mixed_profile_auto_version_with_string_dict_and_site_deltas_and_seq`].
pub fn encode_decoded_mixed_profile_auto_version_with_site_deltas_and_seq(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    max_events_per_chunk: usize,
) -> DecodedMixedResult<Vec<u8>> {
    let with_ver = mixed_events_with_auto_version(major, minor, events)?;
    encode_decoded_mixed_profile_with_site_deltas_and_seq_parts(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        codecs,
        with_ver.as_ref(),
        sources,
        indexes,
        summaries,
        max_events_per_chunk,
        None,
    )
}

/// Encode mixed profile: auto-VERSION inject/validate + packing multi-chunk + FOOTER dict.
///
/// Decode with [`decode_decoded_mixed_profile_auto_version_with_string_dict`].
/// Not dual-equality freeze / permanent packing/string-pool ADR / COL-007 C writer.
pub fn encode_decoded_mixed_profile_auto_version_with_string_dict_and_site_deltas_and_seq(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    max_events_per_chunk: usize,
    dict_entries: &[(u64, u8, &[u8])],
) -> DecodedMixedResult<Vec<u8>> {
    let with_ver = mixed_events_with_auto_version(major, minor, events)?;
    encode_decoded_mixed_profile_with_string_dict_and_site_deltas_and_seq(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        codecs,
        with_ver.as_ref(),
        sources,
        indexes,
        summaries,
        max_events_per_chunk,
        dict_entries,
    )
}

/// Internal: packing multi-chunk EVENT without FOOTER dict. Caller supplies VERSION.
fn encode_decoded_mixed_profile_with_site_deltas_and_seq_parts(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    max_events_per_chunk: usize,
    _dict_entries: Option<&[(u64, u8, &[u8])]>,
) -> DecodedMixedResult<Vec<u8>> {
    if !events.is_empty() && !is_supported_event_codec(codecs.event) {
        return Err(DecodedMixedError::UnsupportedCodec {
            codec: codecs.event,
        });
    }

    let event_parts = if events.is_empty() {
        Vec::new()
    } else {
        partition_event_records(events, max_events_per_chunk)
    };

    let source_plain = if sources.is_empty() {
        Vec::new()
    } else {
        encode_source_body(sources)
    };
    let index_plain = if indexes.is_empty() {
        Vec::new()
    } else {
        encode_index_body(indexes)
    };
    let summary_plain = if summaries.is_empty() {
        Vec::new()
    } else {
        encode_summary_body(summaries)
    };

    let mut sealed: Vec<Vec<u8>> = Vec::with_capacity(
        event_parts.len()
            + usize::from(!source_plain.is_empty())
            + usize::from(!index_plain.is_empty())
            + usize::from(!summary_plain.is_empty()),
    );
    let mut packing = PackingEncodeState::new();
    let mut frame_seq = 0u64;
    for part in event_parts.iter() {
        let plain = encode_event_body_with_site_deltas_and_seq_continuing(part, &mut packing)?;
        sealed.push(encode_event_chunk(
            codecs.event,
            frame_seq,
            part.len() as u32,
            &plain,
        )?);
        frame_seq += 1;
    }
    if !source_plain.is_empty() {
        sealed.push(encode_kind_chunk(
            kind::SOURCE,
            codecs.source,
            frame_seq,
            sources.len() as u32,
            &source_plain,
        )?);
        frame_seq += 1;
    }
    if !index_plain.is_empty() {
        sealed.push(encode_kind_chunk(
            kind::INDEX,
            codecs.index,
            frame_seq,
            indexes.len() as u32,
            &index_plain,
        )?);
        frame_seq += 1;
    }
    if !summary_plain.is_empty() {
        sealed.push(encode_kind_chunk(
            kind::SUMMARY,
            codecs.summary,
            frame_seq,
            summaries.len() as u32,
            &summary_plain,
        )?);
    }
    let _ = frame_seq;
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

/// Encode a provisional multi-chunk record-aligned multi-kind mixed profile.
///
/// Each kind may be split via shipped `partition_*` helpers (`max_*_per_chunk`:
/// `0` = single partition of all records; `1` forces ≥2 chunks when that kind
/// has ≥2 records). Decode with [`decode_decoded_mixed_profile`] (join same-kind
/// plains then body decode). Reuses shipped
/// [`encode_multi_chunk_summary_mixed_profile`].
pub fn encode_decoded_mixed_multi_chunk_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    max_event_records_per_chunk: usize,
    sources: &[SourceRecordSpec<'_>],
    max_source_records_per_chunk: usize,
    indexes: &[IndexRecordSpec<'_>],
    max_index_records_per_chunk: usize,
    summaries: &[SummaryRecordSpec<'_>],
    max_summary_records_per_chunk: usize,
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> DecodedMixedResult<Vec<u8>> {
    Ok(encode_multi_chunk_summary_mixed_profile(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        codecs,
        events,
        max_event_records_per_chunk,
        sources,
        max_source_records_per_chunk,
        indexes,
        max_index_records_per_chunk,
        summaries,
        max_summary_records_per_chunk,
        footer,
    )?)
}

/// Encode a multi-kind mixed profile where **EVENT** body bytes span mid-record
/// across ≥2 EVENT chunks, with co-present SOURCE/INDEX/SUMMARY (full single
/// chunks when non-empty).
///
/// 1. `plain = encode_event_body(events)` then interior split via shipped
///    [`split_event_body_bytes`]
/// 2. Seal head/tail as EVENT frames under `codecs.event`
/// 3. Seal other kinds as one chunk each under their codecs
/// 4. Optional FOOTER codec NONE last
///
/// Decode with [`decode_decoded_mixed_profile`] (join EVENT plains → event-body).
pub fn encode_decoded_mixed_mid_record_event_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    event_split_at: usize,
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> DecodedMixedResult<Vec<u8>> {
    if events.is_empty() {
        return Err(DecodedMixedError::EventBody(EventBodyError::Truncated {
            need: 1,
            got: 0,
        }));
    }
    for (present, c) in [
        (true, codecs.event),
        (!sources.is_empty(), codecs.source),
        (!indexes.is_empty(), codecs.index),
        (!summaries.is_empty(), codecs.summary),
    ] {
        if present && !is_supported_event_codec(c) {
            return Err(DecodedMixedError::UnsupportedCodec { codec: c });
        }
    }

    let plain = encode_event_body(events);
    let (head, tail) = split_event_body_bytes(&plain, event_split_at).ok_or_else(|| {
        DecodedMixedError::EventBody(EventBodyError::Truncated {
            need: event_split_at.saturating_add(1),
            got: plain.len(),
        })
    })?;

    let mut sealed: Vec<Vec<u8>> = Vec::new();
    let mut seq = 0u64;

    // Mid-record EVENT pieces: full logical count on first; 0 on continuation.
    sealed.push(encode_kind_chunk(
        kind::EVENT,
        codecs.event,
        seq,
        events.len() as u32,
        head,
    )?);
    seq += 1;
    sealed.push(encode_kind_chunk(kind::EVENT, codecs.event, seq, 0, tail)?);
    seq += 1;

    if !sources.is_empty() {
        let p = encode_source_body(sources);
        sealed.push(encode_kind_chunk(
            kind::SOURCE,
            codecs.source,
            seq,
            sources.len() as u32,
            &p,
        )?);
        seq += 1;
    }
    if !indexes.is_empty() {
        let p = encode_index_body(indexes);
        sealed.push(encode_kind_chunk(
            kind::INDEX,
            codecs.index,
            seq,
            indexes.len() as u32,
            &p,
        )?);
        seq += 1;
    }
    if !summaries.is_empty() {
        let p = encode_summary_body(summaries);
        sealed.push(encode_kind_chunk(
            kind::SUMMARY,
            codecs.summary,
            seq,
            summaries.len() as u32,
            &p,
        )?);
        seq += 1;
    }
    if let Some(recs) = footer {
        let p = encode_footer_body(recs);
        let checksum = compute_payload_crc(&p);
        sealed.push(encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            seq,
            0,
            recs.len() as u32,
            p.len() as u32,
            &p,
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

/// Encode a multi-kind mixed profile where **SOURCE** body bytes span mid-record
/// across ≥2 SOURCE chunks, with co-present EVENT/INDEX/SUMMARY (full single
/// chunks when non-empty).
///
/// Wire order when non-empty: EVENT (full), SOURCE head/tail mid-span, INDEX,
/// SUMMARY, optional FOOTER last.
///
/// 1. Optional full EVENT/INDEX/SUMMARY chunks under their codecs
/// 2. `plain = encode_source_body(sources)` then interior split via shipped
///    [`split_source_body_bytes`]
/// 3. Seal head/tail as SOURCE frames under `codecs.source`
/// 4. Optional FOOTER codec NONE last
///
/// Decode with [`decode_decoded_mixed_profile`] (join SOURCE plains → source-body).
pub fn encode_decoded_mixed_mid_record_source_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    source_split_at: usize,
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> DecodedMixedResult<Vec<u8>> {
    if sources.is_empty() {
        return Err(DecodedMixedError::SourceBody(SourceBodyError::Truncated {
            need: 1,
            got: 0,
        }));
    }
    for (present, c) in [
        (!events.is_empty(), codecs.event),
        (true, codecs.source),
        (!indexes.is_empty(), codecs.index),
        (!summaries.is_empty(), codecs.summary),
    ] {
        if present && !is_supported_event_codec(c) {
            return Err(DecodedMixedError::UnsupportedCodec { codec: c });
        }
    }

    let plain = encode_source_body(sources);
    let (head, tail) = split_source_body_bytes(&plain, source_split_at).ok_or_else(|| {
        DecodedMixedError::SourceBody(SourceBodyError::Truncated {
            need: source_split_at.saturating_add(1),
            got: plain.len(),
        })
    })?;

    let mut sealed: Vec<Vec<u8>> = Vec::new();
    let mut seq = 0u64;

    // Co-present EVENT (full) first to match mixed wire order EVENT → SOURCE.
    if !events.is_empty() {
        let p = encode_event_body(events);
        sealed.push(encode_kind_chunk(
            kind::EVENT,
            codecs.event,
            seq,
            events.len() as u32,
            &p,
        )?);
        seq += 1;
    }

    // Mid-record SOURCE pieces: full logical count on first; 0 on continuation.
    sealed.push(encode_kind_chunk(
        kind::SOURCE,
        codecs.source,
        seq,
        sources.len() as u32,
        head,
    )?);
    seq += 1;
    sealed.push(encode_kind_chunk(
        kind::SOURCE,
        codecs.source,
        seq,
        0,
        tail,
    )?);
    seq += 1;

    if !indexes.is_empty() {
        let p = encode_index_body(indexes);
        sealed.push(encode_kind_chunk(
            kind::INDEX,
            codecs.index,
            seq,
            indexes.len() as u32,
            &p,
        )?);
        seq += 1;
    }
    if !summaries.is_empty() {
        let p = encode_summary_body(summaries);
        sealed.push(encode_kind_chunk(
            kind::SUMMARY,
            codecs.summary,
            seq,
            summaries.len() as u32,
            &p,
        )?);
        seq += 1;
    }
    if let Some(recs) = footer {
        let p = encode_footer_body(recs);
        let checksum = compute_payload_crc(&p);
        sealed.push(encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            seq,
            0,
            recs.len() as u32,
            p.len() as u32,
            &p,
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

/// Encode a multi-kind mixed profile where **INDEX** body bytes span mid-record
/// across ≥2 INDEX chunks, with co-present EVENT/SOURCE/SUMMARY (full single
/// chunks when non-empty).
///
/// Wire order when non-empty: EVENT (full), SOURCE (full), INDEX head/tail
/// mid-span, SUMMARY, optional FOOTER last.
///
/// 1. Optional full EVENT/SOURCE/SUMMARY chunks under their codecs
/// 2. `plain = encode_index_body(indexes)` then interior split via shipped
///    [`split_index_body_bytes`]
/// 3. Seal head/tail as INDEX frames under `codecs.index`
/// 4. Optional FOOTER codec NONE last
///
/// Decode with [`decode_decoded_mixed_profile`] (join INDEX plains → index-body).
pub fn encode_decoded_mixed_mid_record_index_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    index_split_at: usize,
    summaries: &[SummaryRecordSpec<'_>],
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> DecodedMixedResult<Vec<u8>> {
    if indexes.is_empty() {
        return Err(DecodedMixedError::IndexBody(IndexBodyError::Truncated {
            need: 1,
            got: 0,
        }));
    }
    for (present, c) in [
        (!events.is_empty(), codecs.event),
        (!sources.is_empty(), codecs.source),
        (true, codecs.index),
        (!summaries.is_empty(), codecs.summary),
    ] {
        if present && !is_supported_event_codec(c) {
            return Err(DecodedMixedError::UnsupportedCodec { codec: c });
        }
    }

    let plain = encode_index_body(indexes);
    let (head, tail) = split_index_body_bytes(&plain, index_split_at).ok_or_else(|| {
        DecodedMixedError::IndexBody(IndexBodyError::Truncated {
            need: index_split_at.saturating_add(1),
            got: plain.len(),
        })
    })?;

    let mut sealed: Vec<Vec<u8>> = Vec::new();
    let mut seq = 0u64;

    if !events.is_empty() {
        let p = encode_event_body(events);
        sealed.push(encode_kind_chunk(
            kind::EVENT,
            codecs.event,
            seq,
            events.len() as u32,
            &p,
        )?);
        seq += 1;
    }
    if !sources.is_empty() {
        let p = encode_source_body(sources);
        sealed.push(encode_kind_chunk(
            kind::SOURCE,
            codecs.source,
            seq,
            sources.len() as u32,
            &p,
        )?);
        seq += 1;
    }

    // Mid-record INDEX pieces: full logical count on first; 0 on continuation.
    sealed.push(encode_kind_chunk(
        kind::INDEX,
        codecs.index,
        seq,
        indexes.len() as u32,
        head,
    )?);
    seq += 1;
    sealed.push(encode_kind_chunk(
        kind::INDEX,
        codecs.index,
        seq,
        0,
        tail,
    )?);
    seq += 1;

    if !summaries.is_empty() {
        let p = encode_summary_body(summaries);
        sealed.push(encode_kind_chunk(
            kind::SUMMARY,
            codecs.summary,
            seq,
            summaries.len() as u32,
            &p,
        )?);
        seq += 1;
    }
    if let Some(recs) = footer {
        let p = encode_footer_body(recs);
        let checksum = compute_payload_crc(&p);
        sealed.push(encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            seq,
            0,
            recs.len() as u32,
            p.len() as u32,
            &p,
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

/// Encode a multi-kind mixed profile where **SUMMARY** body bytes span mid-record
/// across ≥2 SUMMARY chunks, with co-present EVENT/SOURCE/INDEX (full single
/// chunks when non-empty).
///
/// Wire order when non-empty: EVENT (full), SOURCE (full), INDEX (full), SUMMARY
/// head/tail mid-span, optional FOOTER last.
///
/// 1. Optional full EVENT/SOURCE/INDEX chunks under their codecs
/// 2. `plain = encode_summary_body(summaries)` then interior split via shipped
///    [`split_summary_body_bytes`]
/// 3. Seal head/tail as SUMMARY frames under `codecs.summary`
/// 4. Optional FOOTER codec NONE last
///
/// Decode with [`decode_decoded_mixed_profile`] (join SUMMARY plains → summary-body).
pub fn encode_decoded_mixed_mid_record_summary_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    summary_split_at: usize,
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> DecodedMixedResult<Vec<u8>> {
    if summaries.is_empty() {
        return Err(DecodedMixedError::SummaryBody(SummaryBodyError::Truncated {
            need: 1,
            got: 0,
        }));
    }
    for (present, c) in [
        (!events.is_empty(), codecs.event),
        (!sources.is_empty(), codecs.source),
        (!indexes.is_empty(), codecs.index),
        (true, codecs.summary),
    ] {
        if present && !is_supported_event_codec(c) {
            return Err(DecodedMixedError::UnsupportedCodec { codec: c });
        }
    }

    let plain = encode_summary_body(summaries);
    let (head, tail) = split_summary_body_bytes(&plain, summary_split_at).ok_or_else(|| {
        DecodedMixedError::SummaryBody(SummaryBodyError::Truncated {
            need: summary_split_at.saturating_add(1),
            got: plain.len(),
        })
    })?;

    let mut sealed: Vec<Vec<u8>> = Vec::new();
    let mut seq = 0u64;

    if !events.is_empty() {
        let p = encode_event_body(events);
        sealed.push(encode_kind_chunk(
            kind::EVENT,
            codecs.event,
            seq,
            events.len() as u32,
            &p,
        )?);
        seq += 1;
    }
    if !sources.is_empty() {
        let p = encode_source_body(sources);
        sealed.push(encode_kind_chunk(
            kind::SOURCE,
            codecs.source,
            seq,
            sources.len() as u32,
            &p,
        )?);
        seq += 1;
    }
    if !indexes.is_empty() {
        let p = encode_index_body(indexes);
        sealed.push(encode_kind_chunk(
            kind::INDEX,
            codecs.index,
            seq,
            indexes.len() as u32,
            &p,
        )?);
        seq += 1;
    }

    // Mid-record SUMMARY pieces: full logical count on first; 0 on continuation.
    sealed.push(encode_kind_chunk(
        kind::SUMMARY,
        codecs.summary,
        seq,
        summaries.len() as u32,
        head,
    )?);
    seq += 1;
    sealed.push(encode_kind_chunk(
        kind::SUMMARY,
        codecs.summary,
        seq,
        0,
        tail,
    )?);
    seq += 1;

    if let Some(recs) = footer {
        let p = encode_footer_body(recs);
        let checksum = compute_payload_crc(&p);
        sealed.push(encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            seq,
            0,
            recs.len() as u32,
            p.len() as u32,
            &p,
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

/// Optional interior mid-record split offset per kind (`None` = single full chunk
/// when records are non-empty; `Some(split_at)` = head/tail mid-span).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MidRecordKindSplits {
    pub event: Option<usize>,
    pub source: Option<usize>,
    pub index: Option<usize>,
    pub summary: Option<usize>,
}

/// Encode a multi-kind mixed profile where **≥2 kinds** each span mid-record
/// across ≥2 same-kind chunks (concurrent mid-record-on-mixed).
///
/// Wire order when non-empty: EVENT, SOURCE, INDEX, SUMMARY, optional FOOTER last.
/// For each kind:
/// - empty records → omit (split must be `None`)
/// - records + `split: None` → one full chunk
/// - records + `split: Some(at)` → head/tail mid-span (`logical_event_count` on
///   first piece; `0` on continuation)
///
/// Requires at least **two** kinds with a mid-record split. Codecs NONE/ZLIB/ZSTD/LZ4
/// per kind. Decode with [`decode_decoded_mixed_profile`].
pub fn encode_decoded_mixed_mid_record_concurrent_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    splits: MidRecordKindSplits,
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> DecodedMixedResult<Vec<u8>> {
    let mid_kinds = [
        splits.event.is_some() && !events.is_empty(),
        splits.source.is_some() && !sources.is_empty(),
        splits.index.is_some() && !indexes.is_empty(),
        splits.summary.is_some() && !summaries.is_empty(),
    ]
    .into_iter()
    .filter(|&x| x)
    .count();
    if mid_kinds < 2 {
        return Err(DecodedMixedError::NeedConcurrentMidRecordKinds { got: mid_kinds });
    }

    // Split requested on empty kind → fail closed.
    if splits.event.is_some() && events.is_empty() {
        return Err(DecodedMixedError::EventBody(EventBodyError::Truncated {
            need: 1,
            got: 0,
        }));
    }
    if splits.source.is_some() && sources.is_empty() {
        return Err(DecodedMixedError::SourceBody(SourceBodyError::Truncated {
            need: 1,
            got: 0,
        }));
    }
    if splits.index.is_some() && indexes.is_empty() {
        return Err(DecodedMixedError::IndexBody(IndexBodyError::Truncated {
            need: 1,
            got: 0,
        }));
    }
    if splits.summary.is_some() && summaries.is_empty() {
        return Err(DecodedMixedError::SummaryBody(SummaryBodyError::Truncated {
            need: 1,
            got: 0,
        }));
    }

    for (present, c) in [
        (!events.is_empty(), codecs.event),
        (!sources.is_empty(), codecs.source),
        (!indexes.is_empty(), codecs.index),
        (!summaries.is_empty(), codecs.summary),
    ] {
        if present && !is_supported_event_codec(c) {
            return Err(DecodedMixedError::UnsupportedCodec { codec: c });
        }
    }

    let mut sealed: Vec<Vec<u8>> = Vec::new();
    let mut seq = 0u64;

    // EVENT
    if !events.is_empty() {
        let plain = encode_event_body(events);
        if let Some(split_at) = splits.event {
            let (head, tail) = split_event_body_bytes(&plain, split_at).ok_or_else(|| {
                DecodedMixedError::EventBody(EventBodyError::Truncated {
                    need: split_at.saturating_add(1),
                    got: plain.len(),
                })
            })?;
            sealed.push(encode_kind_chunk(
                kind::EVENT,
                codecs.event,
                seq,
                events.len() as u32,
                head,
            )?);
            seq += 1;
            sealed.push(encode_kind_chunk(
                kind::EVENT,
                codecs.event,
                seq,
                0,
                tail,
            )?);
            seq += 1;
        } else {
            sealed.push(encode_kind_chunk(
                kind::EVENT,
                codecs.event,
                seq,
                events.len() as u32,
                &plain,
            )?);
            seq += 1;
        }
    }

    // SOURCE
    if !sources.is_empty() {
        let plain = encode_source_body(sources);
        if let Some(split_at) = splits.source {
            let (head, tail) = split_source_body_bytes(&plain, split_at).ok_or_else(|| {
                DecodedMixedError::SourceBody(SourceBodyError::Truncated {
                    need: split_at.saturating_add(1),
                    got: plain.len(),
                })
            })?;
            sealed.push(encode_kind_chunk(
                kind::SOURCE,
                codecs.source,
                seq,
                sources.len() as u32,
                head,
            )?);
            seq += 1;
            sealed.push(encode_kind_chunk(
                kind::SOURCE,
                codecs.source,
                seq,
                0,
                tail,
            )?);
            seq += 1;
        } else {
            sealed.push(encode_kind_chunk(
                kind::SOURCE,
                codecs.source,
                seq,
                sources.len() as u32,
                &plain,
            )?);
            seq += 1;
        }
    }

    // INDEX
    if !indexes.is_empty() {
        let plain = encode_index_body(indexes);
        if let Some(split_at) = splits.index {
            let (head, tail) = split_index_body_bytes(&plain, split_at).ok_or_else(|| {
                DecodedMixedError::IndexBody(IndexBodyError::Truncated {
                    need: split_at.saturating_add(1),
                    got: plain.len(),
                })
            })?;
            sealed.push(encode_kind_chunk(
                kind::INDEX,
                codecs.index,
                seq,
                indexes.len() as u32,
                head,
            )?);
            seq += 1;
            sealed.push(encode_kind_chunk(
                kind::INDEX,
                codecs.index,
                seq,
                0,
                tail,
            )?);
            seq += 1;
        } else {
            sealed.push(encode_kind_chunk(
                kind::INDEX,
                codecs.index,
                seq,
                indexes.len() as u32,
                &plain,
            )?);
            seq += 1;
        }
    }

    // SUMMARY
    if !summaries.is_empty() {
        let plain = encode_summary_body(summaries);
        if let Some(split_at) = splits.summary {
            let (head, tail) = split_summary_body_bytes(&plain, split_at).ok_or_else(|| {
                DecodedMixedError::SummaryBody(SummaryBodyError::Truncated {
                    need: split_at.saturating_add(1),
                    got: plain.len(),
                })
            })?;
            sealed.push(encode_kind_chunk(
                kind::SUMMARY,
                codecs.summary,
                seq,
                summaries.len() as u32,
                head,
            )?);
            seq += 1;
            sealed.push(encode_kind_chunk(
                kind::SUMMARY,
                codecs.summary,
                seq,
                0,
                tail,
            )?);
            seq += 1;
        } else {
            sealed.push(encode_kind_chunk(
                kind::SUMMARY,
                codecs.summary,
                seq,
                summaries.len() as u32,
                &plain,
            )?);
            seq += 1;
        }
    }

    if let Some(recs) = footer {
        let p = encode_footer_body(recs);
        let checksum = compute_payload_crc(&p);
        sealed.push(encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            seq,
            0,
            recs.len() as u32,
            p.len() as u32,
            &p,
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

fn note_kind_codec(
    kind_codecs: &mut KindCodecs,
    chunk_count: &mut usize,
    kind_id: u8,
    frame_codec: u8,
) -> DecodedMixedResult<()> {
    let slot = match kind_id {
        k if k == kind::EVENT => &mut kind_codecs.event,
        k if k == kind::SOURCE => &mut kind_codecs.source,
        k if k == kind::INDEX => &mut kind_codecs.index,
        k if k == kind::SUMMARY => &mut kind_codecs.summary,
        _ => return Err(DecodedMixedError::UnexpectedKind { kind: kind_id }),
    };
    if *chunk_count == 0 {
        *slot = frame_codec;
    } else if *slot != frame_codec {
        // SOURCE/INDEX/SUMMARY still require uniform codec within kind.
        // EVENT mid-stream switch is handled separately (see decode path).
        return Err(DecodedMixedError::KindCodecMismatch {
            kind: kind_id,
            expected: *slot,
            got: frame_codec,
        });
    }
    *chunk_count += 1;
    Ok(())
}

/// Validate mixed mid-stream codec-switch preconditions.
fn validate_mixed_mid_stream_codec_switch(
    pre_codec: u8,
    pre_events: &[EventRecordSpec<'_>],
    post_codec: u8,
    post_events: &[EventRecordSpec<'_>],
    source_codec: u8,
    sources: &[SourceRecordSpec<'_>],
) -> DecodedMixedResult<()> {
    if pre_events.is_empty() || post_events.is_empty() {
        return Err(DecodedMixedError::KindEncode(
            CompressedProfileError::UnsupportedEventCodec { codec: pre_codec },
        ));
    }
    if !is_supported_event_codec(pre_codec) {
        return Err(DecodedMixedError::UnsupportedCodec { codec: pre_codec });
    }
    if !is_supported_event_codec(post_codec) {
        return Err(DecodedMixedError::UnsupportedCodec { codec: post_codec });
    }
    if pre_codec == post_codec {
        return Err(DecodedMixedError::KindCodecMismatch {
            kind: kind::EVENT,
            expected: pre_codec,
            got: post_codec,
        });
    }
    if !pre_events
        .iter()
        .any(|e| matches!(e, EventRecordSpec::StartDeflate))
    {
        return Err(DecodedMixedError::EventBody(EventBodyError::UnknownOpcode {
            opcode: 16, // START_DEFLATE — marker missing in pre region
        }));
    }
    if !sources.is_empty() && !is_supported_event_codec(source_codec) {
        return Err(DecodedMixedError::UnsupportedCodec {
            codec: source_codec,
        });
    }
    Ok(())
}

/// Encode a mixed profile with EVENT mid-stream payload codec switch after START_DEFLATE.
///
/// Wire order: EVENT(pre_codec) → EVENT(post_codec) → optional SOURCE → optional FOOTER.
/// Pre body must include `START_DEFLATE`. `pre_codec` ≠ `post_codec`. Absolute EVENT bodies.
/// For packing continuity across the switch see
/// [`encode_decoded_mixed_mid_stream_codec_switch_with_site_deltas_and_seq`].
/// Decode with [`decode_decoded_mixed_profile`]. Not OI-001-03 freeze; default parse non-inflating.
pub fn encode_decoded_mixed_mid_stream_codec_switch_profile(
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
    source_codec: u8,
    sources: &[SourceRecordSpec<'_>],
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> DecodedMixedResult<Vec<u8>> {
    validate_mixed_mid_stream_codec_switch(
        pre_codec,
        pre_events,
        post_codec,
        post_events,
        source_codec,
        sources,
    )?;

    let pre_plain = encode_event_body(pre_events);
    let post_plain = encode_event_body(post_events);
    let footer_bytes = footer.map(|fr| encode_footer_body(fr));
    seal_mixed_mid_stream_profile(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        pre_codec,
        pre_events.len() as u32,
        &pre_plain,
        post_codec,
        post_events.len() as u32,
        &post_plain,
        source_codec,
        sources,
        footer_bytes.as_deref(),
    )
}

/// Ensure header-matching VERSION in pre||post; inject at start of pre if none (mixed).
fn mixed_mid_stream_pre_with_auto_version<'a>(
    major: u16,
    minor: u16,
    pre: &'a [EventRecordSpec<'a>],
    post: &'a [EventRecordSpec<'a>],
) -> DecodedMixedResult<std::borrow::Cow<'a, [EventRecordSpec<'a>]>> {
    let hm = u64::from(major);
    let hn = u64::from(minor);
    let mut saw = false;
    for e in pre.iter().chain(post.iter()) {
        if let EventRecordSpec::Version {
            major: bm,
            minor: bn,
        } = e
        {
            if *bm != hm || *bn != hn {
                return Err(DecodedMixedError::VersionHeaderMismatch {
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
        return Ok(std::borrow::Cow::Borrowed(pre));
    }
    let mut with_ver: Vec<EventRecordSpec<'a>> = Vec::with_capacity(pre.len() + 1);
    with_ver.push(EventRecordSpec::Version {
        major: hm,
        minor: hn,
    });
    with_ver.extend_from_slice(pre);
    Ok(std::borrow::Cow::Owned(with_ver))
}

/// Encode mixed mid-stream packing with **auto-emit/validate VERSION**.
///
/// Injects header-matching VERSION at start of pre when pre||post omit it; fail-closed on
/// mismatch. Then packing mid-stream encode. Decode with
/// [`decode_decoded_mixed_profile_auto_version`].
/// Not dual-equality freeze / permanent packing ADR / COL-007 C writer.
pub fn encode_decoded_mixed_mid_stream_codec_switch_auto_version_with_site_deltas_and_seq(
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
    source_codec: u8,
    sources: &[SourceRecordSpec<'_>],
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> DecodedMixedResult<Vec<u8>> {
    let pre = mixed_mid_stream_pre_with_auto_version(major, minor, pre_events, post_events)?;
    encode_decoded_mixed_mid_stream_codec_switch_with_site_deltas_and_seq(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        pre_codec,
        pre.as_ref(),
        post_codec,
        post_events,
        source_codec,
        sources,
        footer,
    )
}

/// Encode mixed mid-stream START_DEFLATE codec-switch with **site-delta/seq packing continuity**.
///
/// Pre and post EVENT plains share one [`PackingEncodeState`] (bases continue across switch).
/// Optional structured FOOTER body records (not string-dict). For FOOTER dictionary packing
/// see [`encode_decoded_mixed_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq`].
/// For auto-VERSION inject see
/// [`encode_decoded_mixed_mid_stream_codec_switch_auto_version_with_site_deltas_and_seq`].
/// Decode with [`decode_decoded_mixed_profile`]. Not permanent packing ADR / COL-007 C writer.
pub fn encode_decoded_mixed_mid_stream_codec_switch_with_site_deltas_and_seq(
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
    source_codec: u8,
    sources: &[SourceRecordSpec<'_>],
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> DecodedMixedResult<Vec<u8>> {
    validate_mixed_mid_stream_codec_switch(
        pre_codec,
        pre_events,
        post_codec,
        post_events,
        source_codec,
        sources,
    )?;

    let mut packing = PackingEncodeState::new();
    let pre_plain =
        encode_event_body_with_site_deltas_and_seq_continuing(pre_events, &mut packing)?;
    let post_plain =
        encode_event_body_with_site_deltas_and_seq_continuing(post_events, &mut packing)?;
    let footer_bytes = footer.map(|fr| encode_footer_body(fr));
    seal_mixed_mid_stream_profile(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        pre_codec,
        pre_events.len() as u32,
        &pre_plain,
        post_codec,
        post_events.len() as u32,
        &post_plain,
        source_codec,
        sources,
        footer_bytes.as_deref(),
    )
}

/// Encode mixed mid-stream packing continuity with **FOOTER string-dictionary**.
///
/// Same packing continuity as
/// [`encode_decoded_mixed_mid_stream_codec_switch_with_site_deltas_and_seq`], then FOOTER
/// is the provisional dictionary table (codec NONE). Decode with
/// [`decode_decoded_mixed_profile_with_string_dict`].
/// Not permanent packing/string-pool ADR / COL-007 C writer.
pub fn encode_decoded_mixed_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq(
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
    source_codec: u8,
    sources: &[SourceRecordSpec<'_>],
    dict_entries: &[(u64, u8, &[u8])],
) -> DecodedMixedResult<Vec<u8>> {
    validate_mixed_mid_stream_codec_switch(
        pre_codec,
        pre_events,
        post_codec,
        post_events,
        source_codec,
        sources,
    )?;

    let mut packing = PackingEncodeState::new();
    let pre_plain =
        encode_event_body_with_site_deltas_and_seq_continuing(pre_events, &mut packing)?;
    let post_plain =
        encode_event_body_with_site_deltas_and_seq_continuing(post_events, &mut packing)?;
    let dict_bytes = encode_string_dictionary(dict_entries)?;
    seal_mixed_mid_stream_profile(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        pre_codec,
        pre_events.len() as u32,
        &pre_plain,
        post_codec,
        post_events.len() as u32,
        &post_plain,
        source_codec,
        sources,
        Some(&dict_bytes),
    )
}

fn seal_mixed_mid_stream_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    pre_codec: u8,
    pre_count: u32,
    pre_plain: &[u8],
    post_codec: u8,
    post_count: u32,
    post_plain: &[u8],
    source_codec: u8,
    sources: &[SourceRecordSpec<'_>],
    footer_payload: Option<&[u8]>,
) -> DecodedMixedResult<Vec<u8>> {
    let mut sealed: Vec<Vec<u8>> = Vec::new();
    sealed.push(encode_event_chunk(
        pre_codec,
        0,
        pre_count,
        pre_plain,
    )?);
    sealed.push(encode_event_chunk(
        post_codec,
        1,
        post_count,
        post_plain,
    )?);

    if !sources.is_empty() {
        let src_plain = encode_source_body(sources);
        sealed.push(encode_kind_chunk(
            kind::SOURCE,
            source_codec,
            sealed.len() as u64,
            sources.len() as u32,
            &src_plain,
        )?);
    }

    if let Some(fp) = footer_payload {
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

/// Decode mixed profile then align EVENT VERSION with fixed-header (auto-emit preflight).
///
/// Auto-emits leading VERSION from header when body has none; fail-closed on mismatch.
pub fn decode_decoded_mixed_profile_auto_version(
    buf: &[u8],
    verify_crc: bool,
) -> DecodedMixedResult<(DecodedMixedProfile, usize)> {
    let (mut prof, n) = decode_decoded_mixed_profile(buf, verify_crc)?;
    let before = prof.event_records.len();
    match align_event_records_version_with_header(&prof.header, &mut prof.event_records) {
        Ok(()) => {
            if prof.event_records.len() == before + 1 {
                prof.event_sequences.insert(0, None);
            }
            debug_assert_eq!(prof.event_records.len(), prof.event_sequences.len());
            Ok((prof, n))
        }
        Err(DecodedEventError::VersionHeaderMismatch {
            header_major,
            header_minor,
            body_major,
            body_minor,
        }) => Err(DecodedMixedError::VersionHeaderMismatch {
            header_major,
            header_minor,
            body_major,
            body_minor,
        }),
        // `align_event_records_version_with_header` only returns VersionHeaderMismatch today.
        Err(other) => Err(DecodedMixedError::KindEncode(match other {
            DecodedEventError::Encode(e) => e,
            _ => CompressedProfileError::UnsupportedEventCodec { codec: 0xff },
        })),
    }
}

/// Decode a provisional multi-kind mixed profile via always-inflate stream.
///
/// 1. `decode_prefix_chunk_stream_plain` (optional per-chunk CRC)
/// 2. Append plains by kind in file order
/// 3. One `decode_*_body` over each kind's joined buffer
///
/// EVENT chunks may use different supported payload codecs (START_DEFLATE
/// mid-stream switch preflight). Other kinds still require uniform codec.
/// Default `parse_chunk_frame` remains non-inflating.
/// Does **not** auto-emit VERSION; use [`decode_decoded_mixed_profile_auto_version`].
pub fn decode_decoded_mixed_profile(
    buf: &[u8],
    verify_crc: bool,
) -> DecodedMixedResult<(DecodedMixedProfile, usize)> {
    let (stream, n) = decode_prefix_chunk_stream_plain(buf, verify_crc)?;

    let mut event_plain = Vec::new();
    let mut source_plain = Vec::new();
    let mut index_plain = Vec::new();
    let mut summary_plain = Vec::new();
    let mut footer_plain: Option<Vec<u8>> = None;

    let mut event_chunk_count = 0usize;
    let mut event_chunk_codecs = Vec::new();
    let mut source_chunk_count = 0usize;
    let mut index_chunk_count = 0usize;
    let mut summary_chunk_count = 0usize;
    let mut has_footer = false;
    let mut saw_footer = false;
    let mut kind_codecs = KindCodecs::uniform(codec::NONE);

    for chunk in &stream.chunks {
        if saw_footer {
            return Err(DecodedMixedError::InvalidFooter);
        }
        match chunk.kind {
            k if k == kind::EVENT
                || k == kind::SOURCE
                || k == kind::INDEX
                || k == kind::SUMMARY =>
            {
                if chunk.codec != codec::NONE
                    && chunk.codec != codec::ZLIB
                    && chunk.codec != codec::ZSTD
                    && chunk.codec != codec::LZ4
                {
                    return Err(DecodedMixedError::UnsupportedCodec {
                        codec: chunk.codec,
                    });
                }
                match k {
                    k if k == kind::EVENT => {
                        // Allow per-chunk codec change (mid-stream START_DEFLATE switch).
                        if event_chunk_count == 0 {
                            kind_codecs.event = chunk.codec;
                        }
                        event_chunk_codecs.push(chunk.codec);
                        event_chunk_count += 1;
                        event_plain.extend_from_slice(&chunk.plain);
                    }
                    k if k == kind::SOURCE => {
                        note_kind_codec(
                            &mut kind_codecs,
                            &mut source_chunk_count,
                            kind::SOURCE,
                            chunk.codec,
                        )?;
                        source_plain.extend_from_slice(&chunk.plain);
                    }
                    k if k == kind::INDEX => {
                        note_kind_codec(
                            &mut kind_codecs,
                            &mut index_chunk_count,
                            kind::INDEX,
                            chunk.codec,
                        )?;
                        index_plain.extend_from_slice(&chunk.plain);
                    }
                    k if k == kind::SUMMARY => {
                        note_kind_codec(
                            &mut kind_codecs,
                            &mut summary_chunk_count,
                            kind::SUMMARY,
                            chunk.codec,
                        )?;
                        summary_plain.extend_from_slice(&chunk.plain);
                    }
                    _ => unreachable!(),
                }
            }
            k if k == kind::FOOTER => {
                if chunk.codec != codec::NONE {
                    return Err(DecodedMixedError::UnexpectedFooterCodec {
                        codec: chunk.codec,
                    });
                }
                has_footer = true;
                footer_plain = Some(chunk.plain.clone());
                saw_footer = true;
            }
            other => {
                return Err(DecodedMixedError::UnexpectedKind { kind: other });
            }
        }
    }

    let mut event_records = Vec::new();
    let mut event_sequences = Vec::new();
    if !event_plain.is_empty() {
        let (decoded_body, body_n) = decode_event_body_full(&event_plain)?;
        if body_n != event_plain.len() {
            return Err(DecodedMixedError::EventBody(EventBodyError::Truncated {
                need: event_plain.len(),
                got: body_n,
            }));
        }
        for r in &decoded_body.records {
            event_records.push(OwnedEventRecord::from_borrowed(r));
        }
        event_sequences = decoded_body.sequences;
    }

    let mut source_records = Vec::new();
    if !source_plain.is_empty() {
        let (recs, body_n) = decode_source_body(&source_plain)?;
        if body_n != source_plain.len() {
            return Err(DecodedMixedError::SourceBody(SourceBodyError::Truncated {
                need: source_plain.len(),
                got: body_n,
            }));
        }
        for r in recs {
            source_records.push(OwnedSourceRecord {
                fid: r.fid,
                line: r.line,
                text: r.text.data.to_vec(),
            });
        }
    }

    let mut index_records = Vec::new();
    if !index_plain.is_empty() {
        let (recs, body_n) = decode_index_body(&index_plain)?;
        if body_n != index_plain.len() {
            return Err(DecodedMixedError::IndexBody(IndexBodyError::Truncated {
                need: index_plain.len(),
                got: body_n,
            }));
        }
        for r in recs {
            index_records.push(OwnedIndexRecord {
                key_id: r.key_id,
                file_offset: r.file_offset,
                length: r.length,
                label: r.label.data.to_vec(),
            });
        }
    }

    let mut summary_records = Vec::new();
    if !summary_plain.is_empty() {
        let (recs, body_n) = decode_summary_body(&summary_plain)?;
        if body_n != summary_plain.len() {
            return Err(DecodedMixedError::SummaryBody(SummaryBodyError::Truncated {
                need: summary_plain.len(),
                got: body_n,
            }));
        }
        for r in recs {
            summary_records.push(OwnedSummaryRecord {
                key_id: r.key_id,
                count: r.count,
                value: r.value,
                label: r.label.data.to_vec(),
            });
        }
    }

    let mut footer_records = Vec::new();
    if let Some(fp) = footer_plain {
        let (recs, body_n) = decode_footer_body(&fp)?;
        if body_n != fp.len() {
            return Err(DecodedMixedError::FooterBody(FooterBodyError::Truncated {
                need: fp.len(),
                got: body_n,
            }));
        }
        for r in recs {
            footer_records.push(OwnedFooterRecord {
                key_id: r.key_id,
                value: r.value,
                label: r.label.data.to_vec(),
            });
        }
    }

    Ok((
        DecodedMixedProfile {
            header: stream.header,
            kind_codecs,
            event_chunk_codecs,
            event_records,
            event_sequences,
            source_records,
            index_records,
            summary_records,
            footer_records,
            event_chunk_count,
            source_chunk_count,
            index_chunk_count,
            summary_chunk_count,
            has_footer,
        },
        n,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{encode_chunk_frame, parse_chunk_frame, CHUNK_HEADER_LEN, CHUNK_SYNC};
    use crate::crc::compute_payload_crc;
    use crate::decoded_chunk::DecodedChunkError;
    use crate::decoded_stream::DecodedStreamError;
    use crate::decoded_stream::decode_prefix_chunk_stream_plain;
    use crate::event_body::{
        decode_event_body, encode_event_body, known_key_attr_option_sample_specs,
    };
    use crate::index_body::encode_index_body;
    use crate::mid_record_span::{
        default_mid_body_split, split_event_body_bytes, split_index_body_bytes,
        split_source_body_bytes, split_summary_body_bytes,
    };
    use crate::payload_codec::{deflate_zlib, encode_chunk_frame_zlib};
    use crate::source_body::encode_source_body;
    use crate::summary_body::encode_summary_body;
    use crate::stream::{decode_prefix_chunk_stream, StreamError};
    use crate::{MAGIC, SUPPORTED_MAJOR};

    fn sample_events() -> [EventRecordSpec<'static>; 2] {
        [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 10,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"mixed-mark",
            },
        ]
    }

    fn sample_sources() -> [SourceRecordSpec<'static>; 2] {
        [
            SourceRecordSpec {
                fid: 1,
                line: 1,
                string_id: 0,
                string_flags: 0,
                text: b"src-a",
            },
            SourceRecordSpec {
                fid: 1,
                line: 2,
                string_id: 1,
                string_flags: 0,
                text: b"src-b-longer",
            },
        ]
    }

    fn sample_indexes() -> [IndexRecordSpec<'static>; 1] {
        [IndexRecordSpec {
            key_id: 7,
            file_offset: 42,
            length: 9,
            string_id: 0,
            string_flags: 0,
            label: b"idx",
        }]
    }

    fn sample_summaries() -> [SummaryRecordSpec<'static>; 1] {
        [SummaryRecordSpec {
            key_id: 3,
            count: 4,
            value: 5,
            string_id: 0,
            string_flags: 0,
            label: b"sum",
        }]
    }

    fn sample_footer() -> [FooterRecordSpec<'static>; 1] {
        [FooterRecordSpec {
            key_id: 1,
            value: 99,
            string_id: 0,
            string_flags: 0,
            label: b"end",
        }]
    }

    #[test]
    fn event_source_none_and_zlib_roundtrip() {
        let events = sample_events();
        let sources = sample_sources();
        let codecs = KindCodecs {
            event: codec::NONE,
            source: codec::ZLIB,
            index: codec::NONE,
            summary: codec::NONE,
        };
        let wire = encode_decoded_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &events,
            &sources,
            &[],
            &[],
            Some(&sample_footer()),
        )
        .expect("encode");
        assert_eq!(&wire[..8], MAGIC.as_slice());

        let (prof, n) = decode_decoded_mixed_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert_eq!(prof.event_chunk_count, 1);
        assert_eq!(prof.source_chunk_count, 1);
        assert_eq!(prof.kind_codecs.event, codec::NONE);
        assert_eq!(prof.kind_codecs.source, codec::ZLIB);
        assert_eq!(prof.event_records.len(), 2);
        assert_eq!(prof.source_records.len(), 2);
        assert_eq!(prof.source_records[0].text, b"src-a");
        assert_eq!(prof.source_records[1].text, b"src-b-longer");
        assert!(prof.has_footer);
        assert_eq!(prof.footer_records.len(), 1);
        assert_eq!(prof.footer_records[0].label, b"end");

        // Default parse stays non-inflating on SOURCE.
        let (raw, _) = decode_prefix_chunk_stream(&wire).unwrap();
        let src = raw.chunks.iter().find(|c| c.kind == kind::SOURCE).unwrap();
        assert_ne!(src.payload, encode_source_body(&sources).as_slice());
    }

    #[test]
    fn all_kinds_per_kind_codecs_roundtrip() {
        let events = sample_events();
        let sources = sample_sources();
        let indexes = sample_indexes();
        let summaries = sample_summaries();
        let codecs = KindCodecs {
            event: codec::ZSTD,
            source: codec::LZ4,
            index: codec::ZLIB,
            summary: codec::NONE,
        };
        let wire = encode_decoded_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &events,
            &sources,
            &indexes,
            &summaries,
            None,
        )
        .expect("encode");
        let (prof, n) = decode_decoded_mixed_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert_eq!(prof.kind_codecs, codecs);
        assert_eq!(prof.event_records.len(), 2);
        assert_eq!(prof.source_records.len(), 2);
        assert_eq!(prof.index_records.len(), 1);
        assert_eq!(prof.index_records[0].key_id, 7);
        assert_eq!(prof.summary_records.len(), 1);
        assert_eq!(prof.summary_records[0].label, b"sum");
        assert!(!prof.has_footer);
    }

    #[test]
    fn truncated_mid_stream_err() {
        let wire = encode_decoded_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::ZLIB,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &sample_events(),
            &sample_sources(),
            &[],
            &[],
            None,
        )
        .unwrap();
        let mut short = wire.clone();
        short.truncate(wire.len() - 5);
        match decode_decoded_mixed_profile(&short, false) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Stream(StreamError::Chunk(
                crate::chunk::ChunkError::Truncated { .. },
            )))) => {}
            other => panic!("expected truncated, got {other:?}"),
        }
    }

    #[test]
    fn corrupt_zlib_source_err() {
        let mut wire = encode_decoded_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::ZLIB,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &sample_events(),
            &sample_sources(),
            &[],
            &[],
            None,
        )
        .unwrap();
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        // Skip EVENT (first), corrupt SOURCE.
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        let second_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let f1 = parse_chunk_frame(&wire[second_off..]).unwrap();
        assert_eq!(f1.kind, kind::SOURCE);
        assert_eq!(f1.codec, codec::ZLIB);
        let payload_len = f1.payload.len();
        let payload_off = second_off + CHUNK_HEADER_LEN;
        wire[payload_off] ^= 0xFF;
        if payload_len > 1 {
            wire[payload_off + 1] ^= 0xAA;
        }
        match decode_decoded_mixed_profile(&wire, false) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Payload(_),
            ))) => {}
            other => panic!("expected payload err, got {other:?}"),
        }
        match decode_decoded_mixed_profile(&wire, true) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc err, got {other:?}"),
        }
    }

    #[test]
    fn crc_mismatch_when_verify_on_err() {
        let plain = encode_source_body(&sample_sources());
        let compressed = deflate_zlib(&plain).unwrap();
        let bad = encode_chunk_frame(
            kind::SOURCE,
            codec::ZLIB,
            0,
            0,
            0,
            2,
            plain.len() as u32,
            &compressed,
            compute_payload_crc(&compressed) ^ 0x1111,
        );
        let ev = encode_chunk_frame_zlib(kind::EVENT, 0, 1, 0, 2, &encode_event_body(&sample_events()))
            .unwrap();
        let wire = crate::decoded_stream::encode_prefix_sealed_chunks(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            &[&ev, &bad],
        );
        match decode_decoded_mixed_profile(&wire, true) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc mismatch, got {other:?}"),
        }
        let (prof, n) = decode_decoded_mixed_profile(&wire, false).expect("no crc");
        assert_eq!(n, wire.len());
        assert_eq!(prof.source_records.len(), 2);
        assert_eq!(prof.event_records.len(), 2);
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_decoded_mixed_profile(&[], true).is_err());
        assert!(decode_decoded_mixed_profile(b"nope", false).is_err());
        let mut enc = crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_decoded_mixed_profile(&enc, true) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Stream(StreamError::Chunk(
                crate::chunk::ChunkError::BadSync { expected, got },
            )))) => {
                assert_eq!(expected, CHUNK_SYNC);
                assert_eq!(got, 0xDEAD_BEEF);
            }
            other => panic!("expected bad sync, got {other:?}"),
        }
    }

    #[test]
    fn empty_prefix_only() {
        let wire = encode_decoded_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &[],
            &[],
            &[],
            &[],
            None,
        )
        .unwrap();
        let (prof, n) = decode_decoded_mixed_profile(&wire, true).unwrap();
        assert_eq!(n, wire.len());
        assert_eq!(prof.event_chunk_count, 0);
        assert!(prof.event_records.is_empty());
        assert!(prof.source_records.is_empty());
    }

    // --- multi-chunk record-aligned on always-inflate mixed path ---

    fn multi_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 1,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 2,
                ticks: 2,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"e3",
            },
        ]
    }

    fn multi_sources() -> [SourceRecordSpec<'static>; 3] {
        [
            SourceRecordSpec {
                fid: 1,
                line: 1,
                string_id: 0,
                string_flags: 0,
                text: b"s1",
            },
            SourceRecordSpec {
                fid: 1,
                line: 2,
                string_id: 1,
                string_flags: 0,
                text: b"s2",
            },
            SourceRecordSpec {
                fid: 1,
                line: 3,
                string_id: 2,
                string_flags: 0,
                text: b"s3-longer",
            },
        ]
    }

    #[test]
    fn multi_chunk_event_zlib_plus_source_none_roundtrip() {
        let events = multi_events();
        let sources = multi_sources();
        // ≥2 EVENT chunks (max 1) + co-present SOURCE (single chunk) under NONE.
        let wire = encode_decoded_mixed_multi_chunk_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::ZLIB,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &events,
            1, // force ≥2 EVENT chunks
            &sources,
            0, // one SOURCE partition
            &[],
            0,
            &[],
            0,
            None,
        )
        .expect("encode");
        assert_eq!(&wire[..8], MAGIC.as_slice());

        let (prof, n) = decode_decoded_mixed_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert!(prof.event_chunk_count >= 2);
        assert_eq!(prof.source_chunk_count, 1);
        assert_eq!(prof.event_records.len(), 3);
        assert_eq!(prof.source_records.len(), 3);
        assert_eq!(prof.source_records[2].text, b"s3-longer");
        match &prof.event_records[2] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"e3"),
            other => panic!("{other:?}"),
        }
        // Joined EVENT plains equal full encode_event_body.
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
    fn multi_chunk_source_lz4_plus_event_zstd_roundtrip() {
        let events = multi_events();
        let sources = multi_sources();
        let wire = encode_decoded_mixed_multi_chunk_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::ZSTD,
                source: codec::LZ4,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &events,
            0, // single EVENT
            &sources,
            1, // ≥2 SOURCE chunks
            &[],
            0,
            &[],
            0,
            Some(&sample_footer()),
        )
        .expect("encode");
        let (prof, n) = decode_decoded_mixed_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert_eq!(prof.event_chunk_count, 1);
        assert!(prof.source_chunk_count >= 2);
        assert_eq!(prof.event_records.len(), 3);
        assert_eq!(prof.source_records.len(), 3);
        assert_eq!(prof.source_records[0].text, b"s1");
        assert!(prof.has_footer);
        assert_eq!(prof.footer_records[0].label, b"end");
    }

    #[test]
    fn multi_chunk_corrupt_second_event_zlib_err() {
        let events = multi_events();
        let sources = sample_sources();
        let mut wire = encode_decoded_mixed_multi_chunk_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::ZLIB,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &events,
            1,
            &sources,
            0,
            &[],
            0,
            &[],
            0,
            None,
        )
        .unwrap();
        // Corrupt the second EVENT chunk payload.
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::EVENT);
        let second_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let f1 = parse_chunk_frame(&wire[second_off..]).unwrap();
        assert_eq!(f1.kind, kind::EVENT);
        assert_eq!(f1.codec, codec::ZLIB);
        let payload_len = f1.payload.len();
        let payload_off = second_off + CHUNK_HEADER_LEN;
        wire[payload_off] ^= 0xFF;
        if payload_len > 1 {
            wire[payload_off + 1] ^= 0xAA;
        }
        match decode_decoded_mixed_profile(&wire, false) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Payload(_),
            ))) => {}
            other => panic!("expected payload err, got {other:?}"),
        }
        match decode_decoded_mixed_profile(&wire, true) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc err, got {other:?}"),
        }
    }

    #[test]
    fn multi_chunk_truncated_mid_stream_err() {
        let wire = encode_decoded_mixed_multi_chunk_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::ZLIB,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &multi_events(),
            1,
            &sample_sources(),
            0,
            &[],
            0,
            &[],
            0,
            None,
        )
        .unwrap();
        let mut short = wire.clone();
        short.truncate(wire.len() - 6);
        match decode_decoded_mixed_profile(&short, false) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Stream(StreamError::Chunk(
                crate::chunk::ChunkError::Truncated { .. },
            )))) => {}
            other => panic!("expected truncated, got {other:?}"),
        }
    }

    #[test]
    fn multi_chunk_uses_shipped_partition_count() {
        let events = multi_events();
        let parts = crate::multi_chunk_event::partition_event_records(&events, 1);
        assert_eq!(parts.len(), 3);
        let wire = encode_decoded_mixed_multi_chunk_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &events,
            1,
            &sample_sources(),
            0,
            &[],
            0,
            &[],
            0,
            None,
        )
        .unwrap();
        let (prof, _) = decode_decoded_mixed_profile(&wire, false).unwrap();
        assert_eq!(prof.event_chunk_count, parts.len());
    }

    // --- mid-record span on always-inflate multi-kind mixed path ---

    fn mid_record_split_for_events(events: &[EventRecordSpec<'_>]) -> usize {
        let body = encode_event_body(events);
        let split = default_mid_body_split(&body).expect("body large enough");
        assert!(split > 0 && split < body.len());
        // First piece alone is not a complete valid full body decode of all records.
        assert!(decode_event_body(&body[..split]).is_err());
        split
    }

    #[test]
    fn mid_record_event_zlib_plus_source_none_roundtrip() {
        let events = multi_events();
        let sources = multi_sources();
        let split = mid_record_split_for_events(&events);
        let body = encode_event_body(&events);

        let wire = encode_decoded_mixed_mid_record_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::ZLIB,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &events,
            split,
            &sources,
            &[],
            &[],
            Some(&sample_footer()),
        )
        .expect("encode");
        assert_eq!(&wire[..8], MAGIC.as_slice());

        let (prof, n) = decode_decoded_mixed_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert_eq!(prof.event_chunk_count, 2);
        assert_eq!(prof.source_chunk_count, 1);
        assert_eq!(prof.event_records.len(), 3);
        assert_eq!(prof.source_records.len(), 3);
        assert_eq!(prof.source_records[2].text, b"s3-longer");
        assert!(prof.has_footer);

        // Joined EVENT plains reassemble full body; each piece alone fails full decode.
        let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
        let event_f: Vec<_> = stream
            .chunks
            .iter()
            .filter(|c| c.kind == kind::EVENT)
            .collect();
        assert_eq!(event_f.len(), 2);
        assert_eq!(event_f[0].plain.len(), split);
        assert_eq!(
            [event_f[0].plain.as_slice(), event_f[1].plain.as_slice()].concat(),
            body
        );
        assert!(decode_event_body(&event_f[0].plain).is_err());
        assert!(decode_event_body(&event_f[1].plain).is_err());

        // Wire SOURCE is NONE (plain on wire); EVENT is compressed.
        let (raw, _) = decode_prefix_chunk_stream(&wire).unwrap();
        let ev0 = raw.chunks.iter().find(|c| c.kind == kind::EVENT).unwrap();
        assert_ne!(ev0.payload, body.as_slice());
    }

    #[test]
    fn mid_record_event_none_zstd_lz4_with_source() {
        let events = multi_events();
        let sources = sample_sources();
        let split = mid_record_split_for_events(&events);
        for c in [codec::NONE, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_mid_record_event_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::ZLIB,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                split,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_chunk_count, 2, "codec {c}");
            assert_eq!(prof.event_records.len(), 3, "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.kind_codecs.source, codec::ZLIB);
        }
    }

    #[test]
    fn mid_record_invalid_split_err() {
        let events = multi_events();
        let body = encode_event_body(&events);
        match encode_decoded_mixed_mid_record_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &events,
            0,
            &sample_sources(),
            &[],
            &[],
            None,
        ) {
            Err(DecodedMixedError::EventBody(_)) => {}
            other => panic!("expected invalid split, got {other:?}"),
        }
        match encode_decoded_mixed_mid_record_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &events,
            body.len(),
            &sample_sources(),
            &[],
            &[],
            None,
        ) {
            Err(DecodedMixedError::EventBody(_)) => {}
            other => panic!("expected invalid split end, got {other:?}"),
        }
        assert!(split_event_body_bytes(&body, 0).is_none());
    }

    #[test]
    fn mid_record_corrupt_zlib_continuation_err() {
        let events = multi_events();
        let split = mid_record_split_for_events(&events);
        let mut wire = encode_decoded_mixed_mid_record_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::ZLIB,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &events,
            split,
            &sample_sources(),
            &[],
            &[],
            None,
        )
        .unwrap();
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::EVENT);
        let second_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let f1 = parse_chunk_frame(&wire[second_off..]).unwrap();
        assert_eq!(f1.kind, kind::EVENT);
        assert_eq!(f1.codec, codec::ZLIB);
        let payload_len = f1.payload.len();
        let payload_off = second_off + CHUNK_HEADER_LEN;
        wire[payload_off] ^= 0xFF;
        if payload_len > 1 {
            wire[payload_off + 1] ^= 0xAA;
        }
        match decode_decoded_mixed_profile(&wire, false) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Payload(_),
            ))) => {}
            other => panic!("expected payload err, got {other:?}"),
        }
        match decode_decoded_mixed_profile(&wire, true) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc err, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_truncated_after_first_event_err() {
        let events = multi_events();
        let split = mid_record_split_for_events(&events);
        let mut wire = encode_decoded_mixed_mid_record_event_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &events,
            split,
            &sample_sources(),
            &[],
            &[],
            None,
        )
        .unwrap();
        // Keep only prefix + first EVENT → join is truncated mid-record (no full events).
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        let only_first = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        wire.truncate(only_first);
        match decode_decoded_mixed_profile(&wire, false) {
            Err(DecodedMixedError::EventBody(_)) => {}
            other => panic!("expected truncated event-body after join, got {other:?}"),
        }
    }

    // --- SOURCE mid-record span on always-inflate multi-kind mixed path ---

    fn mid_record_split_for_sources(sources: &[SourceRecordSpec<'_>]) -> usize {
        let body = encode_source_body(sources);
        // Prefer interior split that is not a whole-record boundary (default mid
        // can land between short SOURCE records).
        let mut split = default_mid_body_split(&body).expect("body large enough");
        if decode_source_body(&body[..split]).is_ok() {
            // Nudge off a record boundary until first piece alone fails.
            for delta in 1..body.len() {
                for s in [split.saturating_sub(delta), split + delta] {
                    if s > 0 && s < body.len() && decode_source_body(&body[..s]).is_err() {
                        split = s;
                        break;
                    }
                }
                if decode_source_body(&body[..split]).is_err() {
                    break;
                }
            }
        }
        assert!(split > 0 && split < body.len());
        assert!(
            decode_source_body(&body[..split]).is_err(),
            "need true mid-record split; got {split} of {}",
            body.len()
        );
        split
    }

    #[test]
    fn mid_record_source_zlib_plus_event_none_roundtrip() {
        let events = multi_events();
        let sources = multi_sources();
        let split = mid_record_split_for_sources(&sources);
        let body = encode_source_body(&sources);

        let wire = encode_decoded_mixed_mid_record_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::ZLIB,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &events,
            &sources,
            split,
            &[],
            &[],
            Some(&sample_footer()),
        )
        .expect("encode");
        assert_eq!(&wire[..8], MAGIC.as_slice());

        let (prof, n) = decode_decoded_mixed_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert_eq!(prof.source_chunk_count, 2);
        assert_eq!(prof.event_chunk_count, 1);
        assert_eq!(prof.event_records.len(), 3);
        assert_eq!(prof.source_records.len(), 3);
        assert_eq!(prof.source_records[2].text, b"s3-longer");
        assert!(prof.has_footer);

        let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
        let src_f: Vec<_> = stream
            .chunks
            .iter()
            .filter(|c| c.kind == kind::SOURCE)
            .collect();
        assert_eq!(src_f.len(), 2);
        assert_eq!(src_f[0].plain.len(), split);
        assert_eq!(
            [src_f[0].plain.as_slice(), src_f[1].plain.as_slice()].concat(),
            body
        );
        assert!(decode_source_body(&src_f[0].plain).is_err());
        assert!(decode_source_body(&src_f[1].plain).is_err());

        let (raw, _) = decode_prefix_chunk_stream(&wire).unwrap();
        let s0 = raw.chunks.iter().find(|c| c.kind == kind::SOURCE).unwrap();
        assert_ne!(s0.payload, body.as_slice());
    }

    #[test]
    fn mid_record_source_none_zstd_lz4_with_event() {
        let events = sample_events();
        let sources = multi_sources();
        let split = mid_record_split_for_sources(&sources);
        for c in [codec::NONE, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_mid_record_source_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: codec::ZLIB,
                    source: c,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                split,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.source_chunk_count, 2, "codec {c}");
            assert_eq!(prof.source_records.len(), 3, "codec {c}");
            assert_eq!(prof.event_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.source, c);
            assert_eq!(prof.kind_codecs.event, codec::ZLIB);
        }
    }

    #[test]
    fn mid_record_source_invalid_split_err() {
        let sources = multi_sources();
        let body = encode_source_body(&sources);
        match encode_decoded_mixed_mid_record_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &sample_events(),
            &sources,
            0,
            &[],
            &[],
            None,
        ) {
            Err(DecodedMixedError::SourceBody(_)) => {}
            other => panic!("expected invalid split, got {other:?}"),
        }
        match encode_decoded_mixed_mid_record_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &sample_events(),
            &sources,
            body.len(),
            &[],
            &[],
            None,
        ) {
            Err(DecodedMixedError::SourceBody(_)) => {}
            other => panic!("expected invalid split end, got {other:?}"),
        }
        assert!(split_source_body_bytes(&body, 0).is_none());
    }

    #[test]
    fn mid_record_source_corrupt_zlib_continuation_err() {
        let sources = multi_sources();
        let split = mid_record_split_for_sources(&sources);
        let mut wire = encode_decoded_mixed_mid_record_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::ZLIB,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &sample_events(),
            &sources,
            split,
            &[],
            &[],
            None,
        )
        .unwrap();
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        // Skip EVENT (first), then first SOURCE, corrupt second SOURCE.
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::EVENT);
        let s0_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let s0 = parse_chunk_frame(&wire[s0_off..]).unwrap();
        assert_eq!(s0.kind, kind::SOURCE);
        let s1_off = s0_off + CHUNK_HEADER_LEN + s0.payload.len();
        let s1 = parse_chunk_frame(&wire[s1_off..]).unwrap();
        assert_eq!(s1.kind, kind::SOURCE);
        assert_eq!(s1.codec, codec::ZLIB);
        let payload_len = s1.payload.len();
        let payload_off = s1_off + CHUNK_HEADER_LEN;
        wire[payload_off] ^= 0xFF;
        if payload_len > 1 {
            wire[payload_off + 1] ^= 0xAA;
        }
        match decode_decoded_mixed_profile(&wire, false) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Payload(_),
            ))) => {}
            other => panic!("expected payload err, got {other:?}"),
        }
        match decode_decoded_mixed_profile(&wire, true) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc err, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_source_truncated_after_first_source_err() {
        let sources = multi_sources();
        let split = mid_record_split_for_sources(&sources);
        let mut wire = encode_decoded_mixed_mid_record_source_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &sample_events(),
            &sources,
            split,
            &[],
            &[],
            None,
        )
        .unwrap();
        // Keep prefix + EVENT + first SOURCE only → SOURCE join truncated.
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        let s0_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let s0 = parse_chunk_frame(&wire[s0_off..]).unwrap();
        assert_eq!(s0.kind, kind::SOURCE);
        let only_first_source = s0_off + CHUNK_HEADER_LEN + s0.payload.len();
        wire.truncate(only_first_source);
        match decode_decoded_mixed_profile(&wire, false) {
            Err(DecodedMixedError::SourceBody(_)) => {}
            other => panic!("expected truncated source-body after join, got {other:?}"),
        }
    }

    // --- INDEX mid-record span on always-inflate multi-kind mixed path ---

    fn multi_indexes() -> [IndexRecordSpec<'static>; 3] {
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

    fn mid_record_split_for_indexes(indexes: &[IndexRecordSpec<'_>]) -> usize {
        let body = encode_index_body(indexes);
        let mut split = default_mid_body_split(&body).expect("body large enough");
        if decode_index_body(&body[..split]).is_ok() {
            for delta in 1..body.len() {
                for s in [split.saturating_sub(delta), split + delta] {
                    if s > 0 && s < body.len() && decode_index_body(&body[..s]).is_err() {
                        split = s;
                        break;
                    }
                }
                if decode_index_body(&body[..split]).is_err() {
                    break;
                }
            }
        }
        assert!(split > 0 && split < body.len());
        assert!(
            decode_index_body(&body[..split]).is_err(),
            "need true mid-record split; got {split} of {}",
            body.len()
        );
        split
    }

    #[test]
    fn mid_record_index_zlib_plus_event_none_roundtrip() {
        let events = multi_events();
        let indexes = multi_indexes();
        let split = mid_record_split_for_indexes(&indexes);
        let body = encode_index_body(&indexes);

        let wire = encode_decoded_mixed_mid_record_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::ZLIB,
                summary: codec::NONE,
            },
            &events,
            &[],
            &indexes,
            split,
            &[],
            Some(&sample_footer()),
        )
        .expect("encode");
        assert_eq!(&wire[..8], MAGIC.as_slice());

        let (prof, n) = decode_decoded_mixed_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert_eq!(prof.index_chunk_count, 2);
        assert_eq!(prof.event_chunk_count, 1);
        assert_eq!(prof.event_records.len(), 3);
        assert_eq!(prof.index_records.len(), 3);
        assert_eq!(prof.index_records[2].label, b"third");
        assert!(prof.has_footer);

        let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
        let idx_f: Vec<_> = stream
            .chunks
            .iter()
            .filter(|c| c.kind == kind::INDEX)
            .collect();
        assert_eq!(idx_f.len(), 2);
        assert_eq!(idx_f[0].plain.len(), split);
        assert_eq!(
            [idx_f[0].plain.as_slice(), idx_f[1].plain.as_slice()].concat(),
            body
        );
        assert!(decode_index_body(&idx_f[0].plain).is_err());
        assert!(decode_index_body(&idx_f[1].plain).is_err());

        let (raw, _) = decode_prefix_chunk_stream(&wire).unwrap();
        let i0 = raw.chunks.iter().find(|c| c.kind == kind::INDEX).unwrap();
        assert_ne!(i0.payload, body.as_slice());
    }

    #[test]
    fn mid_record_index_none_zstd_lz4_with_event() {
        let events = sample_events();
        let indexes = multi_indexes();
        let split = mid_record_split_for_indexes(&indexes);
        for c in [codec::NONE, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_mid_record_index_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: codec::ZLIB,
                    source: codec::NONE,
                    index: c,
                    summary: codec::NONE,
                },
                &events,
                &[],
                &indexes,
                split,
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.index_chunk_count, 2, "codec {c}");
            assert_eq!(prof.index_records.len(), 3, "codec {c}");
            assert_eq!(prof.event_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.index, c);
            assert_eq!(prof.kind_codecs.event, codec::ZLIB);
        }
    }

    #[test]
    fn mid_record_index_invalid_split_err() {
        let indexes = multi_indexes();
        let body = encode_index_body(&indexes);
        match encode_decoded_mixed_mid_record_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &sample_events(),
            &[],
            &indexes,
            0,
            &[],
            None,
        ) {
            Err(DecodedMixedError::IndexBody(_)) => {}
            other => panic!("expected invalid split, got {other:?}"),
        }
        match encode_decoded_mixed_mid_record_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &sample_events(),
            &[],
            &indexes,
            body.len(),
            &[],
            None,
        ) {
            Err(DecodedMixedError::IndexBody(_)) => {}
            other => panic!("expected invalid split end, got {other:?}"),
        }
        assert!(split_index_body_bytes(&body, 0).is_none());
    }

    #[test]
    fn mid_record_index_corrupt_zlib_continuation_err() {
        let indexes = multi_indexes();
        let split = mid_record_split_for_indexes(&indexes);
        let mut wire = encode_decoded_mixed_mid_record_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::ZLIB,
                summary: codec::NONE,
            },
            &sample_events(),
            &[],
            &indexes,
            split,
            &[],
            None,
        )
        .unwrap();
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        // Skip EVENT (first), then first INDEX, corrupt second INDEX.
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::EVENT);
        let i0_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let i0 = parse_chunk_frame(&wire[i0_off..]).unwrap();
        assert_eq!(i0.kind, kind::INDEX);
        let i1_off = i0_off + CHUNK_HEADER_LEN + i0.payload.len();
        let i1 = parse_chunk_frame(&wire[i1_off..]).unwrap();
        assert_eq!(i1.kind, kind::INDEX);
        assert_eq!(i1.codec, codec::ZLIB);
        let payload_len = i1.payload.len();
        let payload_off = i1_off + CHUNK_HEADER_LEN;
        wire[payload_off] ^= 0xFF;
        if payload_len > 1 {
            wire[payload_off + 1] ^= 0xAA;
        }
        match decode_decoded_mixed_profile(&wire, false) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Payload(_),
            ))) => {}
            other => panic!("expected payload err, got {other:?}"),
        }
        match decode_decoded_mixed_profile(&wire, true) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc err, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_index_truncated_after_first_index_err() {
        let indexes = multi_indexes();
        let split = mid_record_split_for_indexes(&indexes);
        let mut wire = encode_decoded_mixed_mid_record_index_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &sample_events(),
            &[],
            &indexes,
            split,
            &[],
            None,
        )
        .unwrap();
        // Keep prefix + EVENT + first INDEX only → INDEX join truncated.
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        let i0_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let i0 = parse_chunk_frame(&wire[i0_off..]).unwrap();
        assert_eq!(i0.kind, kind::INDEX);
        let only_first_index = i0_off + CHUNK_HEADER_LEN + i0.payload.len();
        wire.truncate(only_first_index);
        match decode_decoded_mixed_profile(&wire, false) {
            Err(DecodedMixedError::IndexBody(_)) => {}
            other => panic!("expected truncated index-body after join, got {other:?}"),
        }
    }

    // --- SUMMARY mid-record span on always-inflate multi-kind mixed path ---

    fn multi_summaries() -> [SummaryRecordSpec<'static>; 3] {
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
                count: 30,
                value: 300,
                string_id: 2,
                string_flags: 0,
                label: b"third",
            },
        ]
    }

    fn mid_record_split_for_summaries(summaries: &[SummaryRecordSpec<'_>]) -> usize {
        let body = encode_summary_body(summaries);
        let mut split = default_mid_body_split(&body).expect("body large enough");
        if decode_summary_body(&body[..split]).is_ok() {
            for delta in 1..body.len() {
                for s in [split.saturating_sub(delta), split + delta] {
                    if s > 0 && s < body.len() && decode_summary_body(&body[..s]).is_err() {
                        split = s;
                        break;
                    }
                }
                if decode_summary_body(&body[..split]).is_err() {
                    break;
                }
            }
        }
        assert!(split > 0 && split < body.len());
        assert!(
            decode_summary_body(&body[..split]).is_err(),
            "need true mid-record split; got {split} of {}",
            body.len()
        );
        split
    }

    #[test]
    fn mid_record_summary_zlib_plus_event_none_roundtrip() {
        let events = multi_events();
        let summaries = multi_summaries();
        let split = mid_record_split_for_summaries(&summaries);
        let body = encode_summary_body(&summaries);

        let wire = encode_decoded_mixed_mid_record_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::ZLIB,
            },
            &events,
            &[],
            &[],
            &summaries,
            split,
            Some(&sample_footer()),
        )
        .expect("encode");
        assert_eq!(&wire[..8], MAGIC.as_slice());

        let (prof, n) = decode_decoded_mixed_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert_eq!(prof.summary_chunk_count, 2);
        assert_eq!(prof.event_chunk_count, 1);
        assert_eq!(prof.event_records.len(), 3);
        assert_eq!(prof.summary_records.len(), 3);
        assert_eq!(prof.summary_records[2].label, b"third");
        assert!(prof.has_footer);

        let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
        let sum_f: Vec<_> = stream
            .chunks
            .iter()
            .filter(|c| c.kind == kind::SUMMARY)
            .collect();
        assert_eq!(sum_f.len(), 2);
        assert_eq!(sum_f[0].plain.len(), split);
        assert_eq!(
            [sum_f[0].plain.as_slice(), sum_f[1].plain.as_slice()].concat(),
            body
        );
        assert!(decode_summary_body(&sum_f[0].plain).is_err());
        assert!(decode_summary_body(&sum_f[1].plain).is_err());

        let (raw, _) = decode_prefix_chunk_stream(&wire).unwrap();
        let s0 = raw.chunks.iter().find(|c| c.kind == kind::SUMMARY).unwrap();
        assert_ne!(s0.payload, body.as_slice());
    }

    #[test]
    fn mid_record_summary_none_zstd_lz4_with_event() {
        let events = sample_events();
        let summaries = multi_summaries();
        let split = mid_record_split_for_summaries(&summaries);
        for c in [codec::NONE, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_mid_record_summary_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: codec::ZLIB,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: c,
                },
                &events,
                &[],
                &[],
                &summaries,
                split,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.summary_chunk_count, 2, "codec {c}");
            assert_eq!(prof.summary_records.len(), 3, "codec {c}");
            assert_eq!(prof.event_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.summary, c);
            assert_eq!(prof.kind_codecs.event, codec::ZLIB);
        }
    }

    #[test]
    fn mid_record_summary_invalid_split_err() {
        let summaries = multi_summaries();
        let body = encode_summary_body(&summaries);
        match encode_decoded_mixed_mid_record_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &sample_events(),
            &[],
            &[],
            &summaries,
            0,
            None,
        ) {
            Err(DecodedMixedError::SummaryBody(_)) => {}
            other => panic!("expected invalid split, got {other:?}"),
        }
        match encode_decoded_mixed_mid_record_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &sample_events(),
            &[],
            &[],
            &summaries,
            body.len(),
            None,
        ) {
            Err(DecodedMixedError::SummaryBody(_)) => {}
            other => panic!("expected invalid split end, got {other:?}"),
        }
        assert!(split_summary_body_bytes(&body, 0).is_none());
    }

    #[test]
    fn mid_record_summary_corrupt_zlib_continuation_err() {
        let summaries = multi_summaries();
        let split = mid_record_split_for_summaries(&summaries);
        let mut wire = encode_decoded_mixed_mid_record_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::ZLIB,
            },
            &sample_events(),
            &[],
            &[],
            &summaries,
            split,
            None,
        )
        .unwrap();
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        // Skip EVENT (first), then first SUMMARY, corrupt second SUMMARY.
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::EVENT);
        let s0_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let s0 = parse_chunk_frame(&wire[s0_off..]).unwrap();
        assert_eq!(s0.kind, kind::SUMMARY);
        let s1_off = s0_off + CHUNK_HEADER_LEN + s0.payload.len();
        let s1 = parse_chunk_frame(&wire[s1_off..]).unwrap();
        assert_eq!(s1.kind, kind::SUMMARY);
        assert_eq!(s1.codec, codec::ZLIB);
        let payload_len = s1.payload.len();
        let payload_off = s1_off + CHUNK_HEADER_LEN;
        wire[payload_off] ^= 0xFF;
        if payload_len > 1 {
            wire[payload_off + 1] ^= 0xAA;
        }
        match decode_decoded_mixed_profile(&wire, false) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Payload(_),
            ))) => {}
            other => panic!("expected payload err, got {other:?}"),
        }
        match decode_decoded_mixed_profile(&wire, true) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc err, got {other:?}"),
        }
    }

    #[test]
    fn mid_record_summary_truncated_after_first_summary_err() {
        let summaries = multi_summaries();
        let split = mid_record_split_for_summaries(&summaries);
        let mut wire = encode_decoded_mixed_mid_record_summary_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &sample_events(),
            &[],
            &[],
            &summaries,
            split,
            None,
        )
        .unwrap();
        // Keep prefix + EVENT + first SUMMARY only → SUMMARY join truncated.
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        let s0_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let s0 = parse_chunk_frame(&wire[s0_off..]).unwrap();
        assert_eq!(s0.kind, kind::SUMMARY);
        let only_first_summary = s0_off + CHUNK_HEADER_LEN + s0.payload.len();
        wire.truncate(only_first_summary);
        match decode_decoded_mixed_profile(&wire, false) {
            Err(DecodedMixedError::SummaryBody(_)) => {}
            other => panic!("expected truncated summary-body after join, got {other:?}"),
        }
    }

    // --- Concurrent multi-kind mid-record on always-inflate mixed path ---

    #[test]
    fn concurrent_mid_record_event_source_zlib_roundtrip() {
        let events = multi_events();
        let sources = multi_sources();
        let event_split = mid_record_split_for_events(&events);
        let source_split = mid_record_split_for_sources(&sources);
        let event_body = encode_event_body(&events);
        let source_body = encode_source_body(&sources);

        let wire = encode_decoded_mixed_mid_record_concurrent_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::ZLIB,
                source: codec::ZLIB,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &events,
            &sources,
            &[],
            &[],
            MidRecordKindSplits {
                event: Some(event_split),
                source: Some(source_split),
                index: None,
                summary: None,
            },
            Some(&sample_footer()),
        )
        .expect("encode");
        assert_eq!(&wire[..8], MAGIC.as_slice());

        let (prof, n) = decode_decoded_mixed_profile(&wire, true).expect("decode");
        assert_eq!(n, wire.len());
        assert_eq!(prof.event_chunk_count, 2);
        assert_eq!(prof.source_chunk_count, 2);
        assert_eq!(prof.event_records.len(), 3);
        assert_eq!(prof.source_records.len(), 3);
        assert_eq!(prof.source_records[2].text, b"s3-longer");
        assert!(prof.has_footer);

        let (stream, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
        let ev_f: Vec<_> = stream
            .chunks
            .iter()
            .filter(|c| c.kind == kind::EVENT)
            .collect();
        let src_f: Vec<_> = stream
            .chunks
            .iter()
            .filter(|c| c.kind == kind::SOURCE)
            .collect();
        assert_eq!(ev_f.len(), 2);
        assert_eq!(src_f.len(), 2);
        assert_eq!(ev_f[0].plain.len(), event_split);
        assert_eq!(src_f[0].plain.len(), source_split);
        assert_eq!(
            [ev_f[0].plain.as_slice(), ev_f[1].plain.as_slice()].concat(),
            event_body
        );
        assert_eq!(
            [src_f[0].plain.as_slice(), src_f[1].plain.as_slice()].concat(),
            source_body
        );
        assert!(decode_event_body(&ev_f[0].plain).is_err());
        assert!(decode_source_body(&src_f[0].plain).is_err());
    }

    #[test]
    fn concurrent_mid_record_index_summary_none_zstd_lz4() {
        let indexes = multi_indexes();
        let summaries = multi_summaries();
        let index_split = mid_record_split_for_indexes(&indexes);
        let summary_split = mid_record_split_for_summaries(&summaries);

        for (ic, sc) in [
            (codec::NONE, codec::NONE),
            (codec::ZSTD, codec::LZ4),
            (codec::LZ4, codec::ZSTD),
        ] {
            let wire = encode_decoded_mixed_mid_record_concurrent_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: codec::NONE,
                    source: codec::NONE,
                    index: ic,
                    summary: sc,
                },
                &[],
                &[],
                &indexes,
                &summaries,
                MidRecordKindSplits {
                    event: None,
                    source: None,
                    index: Some(index_split),
                    summary: Some(summary_split),
                },
                None,
            )
            .unwrap_or_else(|e| panic!("encode index={ic} summary={sc}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode index={ic} summary={sc}: {e}"));
            assert_eq!(n, wire.len(), "index={ic} summary={sc}");
            assert_eq!(prof.index_chunk_count, 2, "index={ic}");
            assert_eq!(prof.summary_chunk_count, 2, "summary={sc}");
            assert_eq!(prof.index_records.len(), 3);
            assert_eq!(prof.summary_records.len(), 3);
            assert_eq!(prof.kind_codecs.index, ic);
            assert_eq!(prof.kind_codecs.summary, sc);
        }
    }

    #[test]
    fn concurrent_mid_record_all_four_kinds_mixed_codecs() {
        let events = multi_events();
        let sources = multi_sources();
        let indexes = multi_indexes();
        let summaries = multi_summaries();
        let splits = MidRecordKindSplits {
            event: Some(mid_record_split_for_events(&events)),
            source: Some(mid_record_split_for_sources(&sources)),
            index: Some(mid_record_split_for_indexes(&indexes)),
            summary: Some(mid_record_split_for_summaries(&summaries)),
        };
        let wire = encode_decoded_mixed_mid_record_concurrent_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::ZLIB,
                index: codec::ZSTD,
                summary: codec::LZ4,
            },
            &events,
            &sources,
            &indexes,
            &summaries,
            splits,
            Some(&sample_footer()),
        )
        .expect("encode all four");
        let (prof, n) = decode_decoded_mixed_profile(&wire, true).expect("decode all four");
        assert_eq!(n, wire.len());
        assert_eq!(prof.event_chunk_count, 2);
        assert_eq!(prof.source_chunk_count, 2);
        assert_eq!(prof.index_chunk_count, 2);
        assert_eq!(prof.summary_chunk_count, 2);
        assert_eq!(prof.event_records.len(), 3);
        assert_eq!(prof.source_records.len(), 3);
        assert_eq!(prof.index_records.len(), 3);
        assert_eq!(prof.summary_records.len(), 3);
        assert_eq!(prof.kind_codecs.event, codec::NONE);
        assert_eq!(prof.kind_codecs.source, codec::ZLIB);
        assert_eq!(prof.kind_codecs.index, codec::ZSTD);
        assert_eq!(prof.kind_codecs.summary, codec::LZ4);
        assert!(prof.has_footer);
    }

    #[test]
    fn concurrent_mid_record_need_two_kinds_err() {
        let events = multi_events();
        let split = mid_record_split_for_events(&events);
        match encode_decoded_mixed_mid_record_concurrent_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &events,
            &[],
            &[],
            &[],
            MidRecordKindSplits {
                event: Some(split),
                source: None,
                index: None,
                summary: None,
            },
            None,
        ) {
            Err(DecodedMixedError::NeedConcurrentMidRecordKinds { got: 1 }) => {}
            other => panic!("expected need ≥2 mid kinds, got {other:?}"),
        }
        match encode_decoded_mixed_mid_record_concurrent_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &events,
            &multi_sources(),
            &[],
            &[],
            MidRecordKindSplits::default(),
            None,
        ) {
            Err(DecodedMixedError::NeedConcurrentMidRecordKinds { got: 0 }) => {}
            other => panic!("expected need ≥2 mid kinds (0), got {other:?}"),
        }
    }

    #[test]
    fn concurrent_mid_record_invalid_split_err() {
        let events = multi_events();
        let sources = multi_sources();
        let body = encode_event_body(&events);
        let source_split = mid_record_split_for_sources(&sources);
        match encode_decoded_mixed_mid_record_concurrent_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &events,
            &sources,
            &[],
            &[],
            MidRecordKindSplits {
                event: Some(0),
                source: Some(source_split),
                index: None,
                summary: None,
            },
            None,
        ) {
            Err(DecodedMixedError::EventBody(_)) => {}
            other => panic!("expected invalid event split, got {other:?}"),
        }
        match encode_decoded_mixed_mid_record_concurrent_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &events,
            &sources,
            &[],
            &[],
            MidRecordKindSplits {
                event: Some(body.len()),
                source: Some(source_split),
                index: None,
                summary: None,
            },
            None,
        ) {
            Err(DecodedMixedError::EventBody(_)) => {}
            other => panic!("expected invalid event split end, got {other:?}"),
        }
    }

    #[test]
    fn concurrent_mid_record_corrupt_zlib_source_continuation_err() {
        let events = multi_events();
        let sources = multi_sources();
        let event_split = mid_record_split_for_events(&events);
        let source_split = mid_record_split_for_sources(&sources);
        let mut wire = encode_decoded_mixed_mid_record_concurrent_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::ZLIB,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &events,
            &sources,
            &[],
            &[],
            MidRecordKindSplits {
                event: Some(event_split),
                source: Some(source_split),
                index: None,
                summary: None,
            },
            None,
        )
        .unwrap();
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        // EVENT head, EVENT tail, SOURCE head, SOURCE tail — corrupt last SOURCE.
        let mut off = prefix_n;
        for expected in [kind::EVENT, kind::EVENT, kind::SOURCE] {
            let f = parse_chunk_frame(&wire[off..]).unwrap();
            assert_eq!(f.kind, expected);
            off += CHUNK_HEADER_LEN + f.payload.len();
        }
        let s1 = parse_chunk_frame(&wire[off..]).unwrap();
        assert_eq!(s1.kind, kind::SOURCE);
        assert_eq!(s1.codec, codec::ZLIB);
        let payload_len = s1.payload.len();
        let payload_off = off + CHUNK_HEADER_LEN;
        wire[payload_off] ^= 0xFF;
        if payload_len > 1 {
            wire[payload_off + 1] ^= 0xAA;
        }
        match decode_decoded_mixed_profile(&wire, false) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Payload(_),
            ))) => {}
            other => panic!("expected payload err, got {other:?}"),
        }
        match decode_decoded_mixed_profile(&wire, true) {
            Err(DecodedMixedError::Stream(DecodedStreamError::Chunk(
                DecodedChunkError::Crc(_),
            ))) => {}
            other => panic!("expected crc err, got {other:?}"),
        }
    }

    #[test]
    fn concurrent_mid_record_truncated_after_first_event_err() {
        let events = multi_events();
        let sources = multi_sources();
        let event_split = mid_record_split_for_events(&events);
        let source_split = mid_record_split_for_sources(&sources);
        let mut wire = encode_decoded_mixed_mid_record_concurrent_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &events,
            &sources,
            &[],
            &[],
            MidRecordKindSplits {
                event: Some(event_split),
                source: Some(source_split),
                index: None,
                summary: None,
            },
            None,
        )
        .unwrap();
        // Keep prefix + first EVENT only → EVENT join truncated (missing tail).
        let prefix_n =
            crate::file_prefix::encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&wire[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::EVENT);
        wire.truncate(prefix_n + CHUNK_HEADER_LEN + f0.payload.len());
        match decode_decoded_mixed_profile(&wire, false) {
            Err(DecodedMixedError::EventBody(_)) => {}
            other => panic!("expected truncated event-body after join, got {other:?}"),
        }
    }

    // --- TIME_BLOCK + SUB_ENTRY on always-inflate multi-kind mixed path ---

    fn time_block_sub_entry_events() -> [EventRecordSpec<'static>; 4] {
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
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"tb-sub",
            },
        ]
    }

    #[test]
    fn mixed_time_block_sub_entry_none_zlib_zstd_lz4_with_source() {
        let events = time_block_sub_entry_events();
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_records.len(), 4, "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            match &prof.event_records[0] {
                OwnedEventRecord::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                } => assert_eq!((*fid, *line, *block_line, *ticks), (1, 5, 4, 780)),
                other => panic!("codec {c}: expected TimeBlock, got {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::SubEntry {
                    caller_fid,
                    caller_line,
                } => assert_eq!((*caller_fid, *caller_line), (1, 12)),
                other => panic!("codec {c}: expected SubEntry, got {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 3),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            match &prof.event_records[3] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"tb-sub"),
                other => panic!("codec {c}: expected Mark, got {other:?}"),
            }
        }
    }

    fn sub_return_sub_info_events() -> [EventRecordSpec<'static>; 4] {
        [
            EventRecordSpec::SubReturn {
                depth: 2,
                incl: 1500,
                excl: 100,
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
            EventRecordSpec::SubEntry {
                caller_fid: 1,
                caller_line: 12,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"sr-si",
            },
        ]
    }

    #[test]
    fn mixed_sub_return_sub_info_none_zlib_zstd_lz4_with_source() {
        let events = sub_return_sub_info_events();
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_records.len(), 4, "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            match &prof.event_records[0] {
                OwnedEventRecord::SubReturn {
                    depth,
                    incl,
                    excl,
                    subname,
                } => {
                    assert_eq!((*depth, *incl, *excl), (2, 1500, 100));
                    assert_eq!(subname, b"main::leaf");
                }
                other => panic!("codec {c}: expected SubReturn, got {other:?}"),
            }
            match &prof.event_records[1] {
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
            match &prof.event_records[2] {
                OwnedEventRecord::SubEntry {
                    caller_fid,
                    caller_line,
                } => assert_eq!((*caller_fid, *caller_line), (1, 12)),
                other => panic!("codec {c}: expected SubEntry, got {other:?}"),
            }
            match &prof.event_records[3] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"sr-si"),
                other => panic!("codec {c}: expected Mark, got {other:?}"),
            }
        }
    }

    fn src_line_new_fid_events() -> [EventRecordSpec<'static>; 4] {
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
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"src-fid",
            },
        ]
    }

    #[test]
    fn mixed_src_line_new_fid_none_zlib_zstd_lz4_with_source() {
        let events = src_line_new_fid_events();
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_records.len(), 4, "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            match &prof.event_records[0] {
                OwnedEventRecord::NewFid { fid, filename } => {
                    assert_eq!(*fid, 1);
                    assert_eq!(filename, b"workload.pl");
                }
                other => panic!("codec {c}: expected NewFid, got {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::SrcLine { fid, line, text } => {
                    assert_eq!((*fid, *line), (1, 5));
                    assert_eq!(text, b"  my $x = 1;");
                }
                other => panic!("codec {c}: expected SrcLine, got {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            match &prof.event_records[3] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"src-fid"),
                other => panic!("codec {c}: expected Mark, got {other:?}"),
            }
        }
    }

    fn pid_start_end_events() -> [EventRecordSpec<'static>; 4] {
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
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"pid-pair",
            },
        ]
    }

    #[test]
    fn mixed_pid_start_end_none_zlib_zstd_lz4_with_source() {
        let events = pid_start_end_events();
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_records.len(), 4, "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            match &prof.event_records[0] {
                OwnedEventRecord::PidStart {
                    pid,
                    ppid,
                    start_time,
                } => assert_eq!((*pid, *ppid, *start_time), (1001, 1, 1_700_000_000)),
                other => panic!("codec {c}: expected PidStart, got {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::PidEnd { pid, end_time } => {
                    assert_eq!((*pid, *end_time), (1001, 1_700_000_042));
                }
                other => panic!("codec {c}: expected PidEnd, got {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            match &prof.event_records[3] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"pid-pair"),
                other => panic!("codec {c}: expected Mark, got {other:?}"),
            }
        }
    }

    fn sub_callers_discount_events() -> [EventRecordSpec<'static>; 4] {
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
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"sc-disc",
            },
        ]
    }

    #[test]
    fn mixed_sub_callers_discount_none_zlib_zstd_lz4_with_source() {
        let events = sub_callers_discount_events();
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_records.len(), 4, "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            match &prof.event_records[0] {
                OwnedEventRecord::SubCallers {
                    fid,
                    line,
                    count,
                    incl,
                    excl,
                    called,
                    caller,
                    ..
                } => {
                    assert_eq!((*fid, *line, *count, *incl, *excl), (1, 10, 15, 900, 50));
                    assert_eq!(called, b"main::leaf");
                    assert_eq!(caller, b"main::mid");
                }
                other => panic!("codec {c}: expected SubCallers, got {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::Discount => {}
                other => panic!("codec {c}: expected Discount, got {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            match &prof.event_records[3] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"sc-disc"),
                other => panic!("codec {c}: expected Mark, got {other:?}"),
            }
        }
    }

    fn attribute_option_events() -> [EventRecordSpec<'static>; 4] {
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
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"attr-opt",
            },
        ]
    }

    #[test]
    fn mixed_attribute_option_none_zlib_zstd_lz4_with_source() {
        let events = attribute_option_events();
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_records.len(), 4, "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            match &prof.event_records[0] {
                OwnedEventRecord::Attribute { key, value } => {
                    assert_eq!(key, b"basetime");
                    assert_eq!(value, b"1700000000");
                }
                other => panic!("codec {c}: expected Attribute, got {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::Option { key, value } => {
                    assert_eq!(key, b"calls");
                    assert_eq!(value, b"1");
                }
                other => panic!("codec {c}: expected Option, got {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            match &prof.event_records[3] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"attr-opt"),
                other => panic!("codec {c}: expected Mark, got {other:?}"),
            }
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
    fn mixed_comment_none_zlib_zstd_lz4_with_source() {
        let events = comment_events();
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_records.len(), 3, "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            match &prof.event_records[0] {
                OwnedEventRecord::Comment { text } => assert_eq!(text, b"# profiler note"),
                other => panic!("codec {c}: expected Comment, got {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"cmt"),
                other => panic!("codec {c}: expected Mark, got {other:?}"),
            }
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
    fn mixed_start_deflate_none_zlib_zstd_lz4_with_source() {
        let events = start_deflate_events();
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_records.len(), 3, "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            match &prof.event_records[0] {
                OwnedEventRecord::StartDeflate => {}
                other => panic!("codec {c}: expected StartDeflate, got {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"sd"),
                other => panic!("codec {c}: expected Mark, got {other:?}"),
            }
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
    fn mixed_version_none_zlib_zstd_lz4_with_source() {
        let events = version_events();
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_records.len(), 3, "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            match &prof.event_records[0] {
                OwnedEventRecord::Version { major, minor } => {
                    assert_eq!((*major, *minor), (5, 0));
                }
                other => panic!("codec {c}: expected Version, got {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::StartDeflate => {}
                other => panic!("codec {c}: expected StartDeflate, got {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
                other => panic!("codec {c}: expected TimeLine, got {other:?}"),
            }
        }
    }

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

    fn assert_dual_output_mixed_events(recs: &[OwnedEventRecord]) {
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
    fn mixed_dual_output_sequence_none_zlib_zstd_lz4_with_source() {
        let events = dual_output_sequence_events();
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "codec {c} co-kind SOURCE");
            assert_eq!(prof.kind_codecs.event, c);
            assert_dual_output_mixed_events(&prof.event_records);
        }
    }

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

    fn assert_mid_stream_mixed_events(recs: &[OwnedEventRecord]) {
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

    fn mid_stream_packing_pre_events() -> [EventRecordSpec<'static>; 3] {
        const TL: &[u64] = &[7, 8];
        [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 50,
                ticks: TL,
            },
            EventRecordSpec::StartDeflate,
        ]
    }

    fn mid_stream_packing_post_events() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 51,
                ticks: 9,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 2,
                caller_line: 50,
            },
            EventRecordSpec::TimeBlock {
                fid: 3,
                line: 9,
                block_line: 7,
                ticks: 3,
            },
        ]
    }

    fn mid_stream_dict_packing_pre_events() -> [EventRecordSpec<'static>; 3] {
        const TL: &[u64] = &[7, 8];
        [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 50,
                ticks: TL,
            },
            EventRecordSpec::StartDeflate,
        ]
    }

    fn mid_stream_dict_packing_post_events() -> [EventRecordSpec<'static>; 4] {
        [
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 51,
                ticks: 9,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 2,
                caller_line: 50,
            },
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"",
            },
            EventRecordSpec::Comment {
                string_id: 2,
                string_flags: 0,
                text: b"",
            },
        ]
    }

    #[test]
    fn mixed_mid_stream_codec_switch_auto_version_packing_none_to_zlib_zstd_lz4_with_source() {
        let pre = mid_stream_packing_pre_events();
        let post = mid_stream_packing_post_events();
        let sources = sample_sources();
        let header_minor = 7u16;

        let baseline =
            encode_decoded_mixed_mid_stream_codec_switch_auto_version_with_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                header_minor,
                0,
                0,
                0,
                &[],
                codec::NONE,
                &pre,
                codec::ZLIB,
                &post,
                codec::NONE,
                &sources,
                None,
            )
            .expect("baseline");
        let (single_prof, _) =
            decode_decoded_mixed_profile_auto_version(&baseline, true).unwrap();
        assert_eq!(single_prof.source_records.len(), 2);
        match &single_prof.event_records[0] {
            OwnedEventRecord::Version { major, minor } => {
                assert_eq!(
                    (*major, *minor),
                    (u64::from(SUPPORTED_MAJOR), u64::from(header_minor))
                );
            }
            other => panic!("expected VERSION, got {other:?}"),
        }
        match &single_prof.event_records[5] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("post-run [5] {other:?}"),
        }

        for post_c in [codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire =
                encode_decoded_mixed_mid_stream_codec_switch_auto_version_with_site_deltas_and_seq(
                    SUPPORTED_MAJOR,
                    header_minor,
                    0,
                    0,
                    0,
                    &[],
                    codec::NONE,
                    &pre,
                    post_c,
                    &post,
                    codec::NONE,
                    &sources,
                    None,
                )
                .unwrap_or_else(|e| panic!("encode post_codec {post_c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile_auto_version(&wire, true)
                .unwrap_or_else(|e| panic!("decode post_codec {post_c}: {e}"));
            assert_eq!(n, wire.len(), "codec {post_c}");
            assert_eq!(prof.event_chunk_count, 2);
            assert_eq!(prof.source_records.len(), 2);
            assert_eq!(
                prof.event_records, single_prof.event_records,
                "codec {post_c}"
            );
            assert_eq!(
                prof.event_sequences, single_prof.event_sequences,
                "codec {post_c}"
            );
            match &prof.event_records[5] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!(
                        (*fid, *line, *ticks),
                        (2, 51, 9),
                        "codec {post_c}: post-run across switch"
                    );
                }
                other => panic!("codec {post_c}: [5] {other:?}"),
            }
            let (stream_raw, _) = decode_prefix_chunk_stream(&wire).expect("raw");
            assert_eq!(stream_raw.chunks[1].codec, post_c);
            let (stream_plain, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            assert_ne!(
                stream_raw.chunks[1].payload,
                stream_plain.chunks[1].plain.as_slice(),
                "codec {post_c}: default parse must not inflate post"
            );
        }
    }

    #[test]
    fn mixed_mid_stream_codec_switch_dict_packing_none_to_zlib_zstd_lz4_with_source() {
        use crate::string::FLAG_UTF8;
        let pre = mid_stream_dict_packing_pre_events();
        let post = mid_stream_dict_packing_post_events();
        let sources = sample_sources();
        let dict_entries: &[(u64, u8, &[u8])] = &[
            (1, FLAG_UTF8, b"mixed-ms-dict-mark"),
            (2, 0, b"# mixed-ms-dict-end"),
        ];
        let baseline =
            encode_decoded_mixed_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq(
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
                codec::NONE,
                &sources,
                dict_entries,
            )
            .expect("baseline");
        let (single_prof, single_dict, _) =
            decode_decoded_mixed_profile_with_string_dict(&baseline, true).unwrap();
        assert_eq!(single_dict.get(1).unwrap().data, b"mixed-ms-dict-mark");
        assert_eq!(single_prof.source_records.len(), 2);
        match &single_prof.event_records[4] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("[4] {other:?}"),
        }

        for post_c in [codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire =
                encode_decoded_mixed_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq(
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
                    codec::NONE,
                    &sources,
                    dict_entries,
                )
                .unwrap_or_else(|e| panic!("encode post_codec {post_c}: {e}"));
            let (prof, dict, n) = decode_decoded_mixed_profile_with_string_dict(&wire, true)
                .unwrap_or_else(|e| panic!("decode post_codec {post_c}: {e}"));
            assert_eq!(n, wire.len(), "codec {post_c}");
            assert_eq!(prof.event_chunk_count, 2);
            assert_eq!(prof.source_records.len(), 2);
            assert_eq!(dict.get(1).unwrap().data, b"mixed-ms-dict-mark");
            assert_eq!(dict.get(2).unwrap().data, b"# mixed-ms-dict-end");
            assert_eq!(
                prof.event_records, single_prof.event_records,
                "codec {post_c}"
            );
            assert_eq!(
                prof.event_sequences, single_prof.event_sequences,
                "codec {post_c}"
            );
            match &prof.event_records[4] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!(
                        (*fid, *line, *ticks),
                        (2, 51, 9),
                        "codec {post_c}: post-run across switch"
                    );
                }
                other => panic!("codec {post_c}: [4] {other:?}"),
            }
            let last = prof.event_records.len() - 1;
            match &prof.event_records[last] {
                OwnedEventRecord::Comment { text } => {
                    assert_eq!(text, b"# mixed-ms-dict-end", "codec {post_c}");
                }
                other => panic!("codec {post_c}: last {other:?}"),
            }
            let (stream_raw, _) = decode_prefix_chunk_stream(&wire).expect("raw");
            assert_eq!(stream_raw.chunks[1].codec, post_c);
            let (stream_plain, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            assert_ne!(
                stream_raw.chunks[1].payload,
                stream_plain.chunks[1].plain.as_slice(),
                "codec {post_c}: default parse must not inflate post"
            );
        }
    }

    #[test]
    fn mixed_mid_stream_codec_switch_packing_none_to_zlib_zstd_lz4_with_source() {
        let pre = mid_stream_packing_pre_events();
        let post = mid_stream_packing_post_events();
        let sources = sample_sources();
        let mut all: Vec<EventRecordSpec<'static>> = pre.to_vec();
        all.extend_from_slice(&post);
        let single_plain =
            crate::event_body::encode_event_body_with_site_deltas_and_seq(&all).unwrap();
        let (single_body, _) =
            crate::event_body::decode_event_body_full(&single_plain).unwrap();
        let single_owned: Vec<_> = single_body
            .records
            .iter()
            .map(OwnedEventRecord::from_borrowed)
            .collect();

        for post_c in [codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_mid_stream_codec_switch_with_site_deltas_and_seq(
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
                codec::NONE,
                &sources,
                None,
            )
            .unwrap_or_else(|e| panic!("encode post_codec {post_c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode post_codec {post_c}: {e}"));
            assert_eq!(n, wire.len(), "codec {post_c}");
            assert_eq!(prof.event_chunk_count, 2);
            assert_eq!(prof.source_records.len(), 2);
            assert_eq!(prof.event_records, single_owned, "codec {post_c}");
            assert_eq!(
                prof.event_sequences, single_body.sequences,
                "codec {post_c}"
            );
            match &prof.event_records[4] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!(
                        (*fid, *line, *ticks),
                        (2, 51, 9),
                        "codec {post_c}: post-run across codec switch"
                    );
                }
                other => panic!("codec {post_c}: [4] {other:?}"),
            }
            let (stream_raw, _) = decode_prefix_chunk_stream(&wire).expect("raw");
            assert_eq!(stream_raw.chunks[1].codec, post_c);
            let (stream_plain, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            assert_ne!(
                stream_raw.chunks[1].payload,
                stream_plain.chunks[1].plain.as_slice(),
                "codec {post_c}: default parse must not inflate post"
            );
        }
    }

    #[test]
    fn mixed_mid_stream_codec_switch_none_to_zlib_zstd_lz4_with_source() {
        let pre = mid_stream_pre_events();
        let post = mid_stream_post_events();
        let sources = sample_sources();
        for post_c in [codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_mid_stream_codec_switch_profile(
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
                codec::NONE,
                &sources,
                None,
            )
            .unwrap_or_else(|e| panic!("encode post_codec {post_c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode post_codec {post_c}: {e}"));
            assert_eq!(n, wire.len(), "post_codec {post_c}");
            assert_eq!(prof.event_chunk_count, 2, "post_codec {post_c}");
            assert_eq!(
                prof.event_chunk_codecs,
                vec![codec::NONE, post_c],
                "post_codec {post_c}"
            );
            assert_eq!(prof.kind_codecs.event, codec::NONE);
            assert_eq!(prof.source_records.len(), 2, "co-kind SOURCE");
            assert_mid_stream_mixed_events(&prof.event_records);
        }
    }

    #[test]
    fn mixed_mid_stream_codec_switch_corrupt_post_zlib_err() {
        let pre = mid_stream_pre_events();
        let post = mid_stream_post_events();
        // No SOURCE co-kind so trailing bytes belong to the post-switch ZLIB EVENT payload.
        let mut wire = encode_decoded_mixed_mid_stream_codec_switch_profile(
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
            codec::NONE,
            &[],
            None,
        )
        .expect("encode");
        let len = wire.len();
        assert!(len > 8);
        wire[len - 4] ^= 0x5a;
        wire[len - 3] ^= 0xa5;
        match decode_decoded_mixed_profile(&wire, false) {
            Err(_) => {}
            Ok(_) => panic!("expected corrupt post-switch mixed fail-closed"),
        }
    }

    fn auto_version_mixed_workload() -> [EventRecordSpec<'static>; 2] {
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

    #[test]
    fn mixed_auto_version_none_zlib_zstd_lz4_with_source() {
        let workload = auto_version_mixed_workload();
        let sources = sample_sources();
        let header_minor = 1u16;
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile_auto_version(
                SUPPORTED_MAJOR,
                header_minor,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &workload,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile_auto_version(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.header.major, SUPPORTED_MAJOR);
            assert_eq!(prof.header.minor, header_minor);
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.source_records.len(), 2, "SOURCE co-kind");
            match &prof.event_records[0] {
                OwnedEventRecord::Version { major, minor } => {
                    assert_eq!(*major, u64::from(SUPPORTED_MAJOR));
                    assert_eq!(*minor, u64::from(header_minor));
                }
                other => panic!("codec {c}: expected Version first, got {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 42),
                other => panic!("codec {c}: {other:?}"),
            }
        }
    }

    #[test]
    fn mixed_auto_version_decode_injects_when_body_omits() {
        let workload = auto_version_mixed_workload();
        let sources = sample_sources();
        let header_minor = 4u16;
        let wire = encode_decoded_mixed_profile(
            SUPPORTED_MAJOR,
            header_minor,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::ZLIB,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &workload,
            &sources,
            &[],
            &[],
            None,
        )
        .expect("encode without VERSION");
        let (plain, _) = decode_decoded_mixed_profile(&wire, true).expect("plain");
        assert!(
            !plain
                .event_records
                .iter()
                .any(|r| matches!(r, OwnedEventRecord::Version { .. }))
        );
        let (auto, _) = decode_decoded_mixed_profile_auto_version(&wire, true).expect("auto");
        match &auto.event_records[0] {
            OwnedEventRecord::Version { major, minor } => {
                assert_eq!((*major, *minor), (u64::from(SUPPORTED_MAJOR), 4));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(auto.source_records.len(), 2);
    }

    #[test]
    fn mixed_auto_version_mismatch_fail_closed() {
        let events = [
            EventRecordSpec::Version {
                major: u64::from(SUPPORTED_MAJOR),
                minor: 77,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"x",
            },
        ];
        match encode_decoded_mixed_profile_auto_version(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::NONE),
            &events,
            &[],
            &[],
            &[],
            None,
        ) {
            Err(DecodedMixedError::VersionHeaderMismatch {
                body_minor: 77,
                header_minor: 0,
                ..
            }) => {}
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    #[test]
    fn mixed_known_key_attr_option_none_zlib_zstd_lz4_with_source() {
        let events = known_key_attr_option_sample_specs();
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.source_records.len(), 2, "SOURCE co-kind");
            assert_eq!(prof.event_records.len(), 5, "codec {c}");
            match &prof.event_records[0] {
                OwnedEventRecord::Attribute { key, value } => {
                    assert_eq!(key, crate::known_key::BASETIME);
                    assert_eq!(value, b"1786111723");
                }
                other => panic!("codec {c}: basetime, got {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::Attribute { key, value } => {
                    assert_eq!(key, crate::known_key::TICKS_PER_SEC);
                    assert_eq!(value, b"10000000");
                }
                other => panic!("codec {c}: ticks_per_sec, got {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::Attribute { key, value } => {
                    assert_eq!(key, crate::known_key::APPLICATION);
                    assert_eq!(value, b"workload.pl");
                }
                other => panic!("codec {c}: application, got {other:?}"),
            }
            match &prof.event_records[3] {
                OwnedEventRecord::Option { key, value } => {
                    assert_eq!(key, crate::known_key::CALLS);
                    assert_eq!(value, b"1");
                }
                other => panic!("codec {c}: calls, got {other:?}"),
            }
            match &prof.event_records[4] {
                OwnedEventRecord::Option { key, value } => {
                    assert_eq!(key, crate::known_key::BLOCKS);
                    assert_eq!(value, b"0");
                }
                other => panic!("codec {c}: blocks, got {other:?}"),
            }
        }
    }

    #[test]
    fn mixed_known_key_expanded_inventory_none_zlib_zstd_lz4_with_source() {
        use crate::event_body::known_key_attr_option_expanded_sample_specs;
        let events = known_key_attr_option_expanded_sample_specs();
        let expect_n = events.len();
        let sources = sample_sources();
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.source_records.len(), 2, "SOURCE co-kind");
            assert_eq!(prof.event_records.len(), expect_n, "codec {c}");
            for (i, r) in prof.event_records.iter().enumerate() {
                match r {
                    OwnedEventRecord::Attribute { key, value } => {
                        assert!(
                            crate::known_key::is_known_attribute_key(key),
                            "codec {c} [{i}]"
                        );
                        match &events[i] {
                            EventRecordSpec::Attribute {
                                key: sk,
                                value: sv,
                                ..
                            } => {
                                assert_eq!(key.as_slice(), *sk);
                                assert_eq!(value.as_slice(), *sv);
                            }
                            other => panic!("codec {c} [{i}] {other:?}"),
                        }
                    }
                    OwnedEventRecord::Option { key, value } => {
                        assert!(
                            crate::known_key::is_known_option_key(key),
                            "codec {c} [{i}]"
                        );
                        match &events[i] {
                            EventRecordSpec::Option {
                                key: sk,
                                value: sv,
                                ..
                            } => {
                                assert_eq!(key.as_slice(), *sk);
                                assert_eq!(value.as_slice(), *sv);
                            }
                            other => panic!("codec {c} [{i}] {other:?}"),
                        }
                    }
                    other => panic!("codec {c} [{i}] {other:?}"),
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
        }
    }

    fn mixed_plain_with_unknown_optional_skip() -> Vec<u8> {
        let mut plain = encode_event_body(&[EventRecordSpec::TimeLine {
            fid: 2,
            line: 20,
            ticks: 7,
        }]);
        plain.extend_from_slice(
            &crate::event_body::encode_unknown_optional_skip_record(88, b"mixed-skip")
                .expect("skip"),
        );
        plain.extend_from_slice(&encode_event_body(&[EventRecordSpec::Mark {
            string_id: 0,
            string_flags: 0,
            label: b"mixed-after",
        }]));
        plain
    }

    #[test]
    fn mixed_unknown_optional_skip_none_zlib_zstd_lz4_with_source() {
        let event_plain = mixed_plain_with_unknown_optional_skip();
        let sources = sample_sources();
        let src_plain = encode_source_body(&sources);
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let event_frame = crate::compressed_profile::encode_event_chunk(
                c,
                0,
                2,
                &event_plain,
            )
            .expect("event seal");
            let source_frame = crate::compressed_profile::encode_kind_chunk(
                kind::SOURCE,
                codec::NONE,
                1,
                sources.len() as u32,
                &src_plain,
            )
            .expect("source seal");
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &[event_frame.as_slice(), source_frame.as_slice()],
            );
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.source_records.len(), 2, "SOURCE co-kind");
            assert_eq!(prof.event_records.len(), 2, "codec {c}: skip not emitted");
            match &prof.event_records[0] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (2, 20, 7));
                }
                other => panic!("codec {c}: {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"mixed-after"),
                other => panic!("codec {c}: {other:?}"),
            }
        }
    }

    #[test]
    fn mixed_string_dict_and_site_delta_seq_compose_none_zlib_zstd_lz4_with_source() {
        use crate::string::FLAG_UTF8;
        let events = [
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"",
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 30,
                ticks: 1,
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 31,
                ticks: 2,
            },
            EventRecordSpec::Option {
                key_string_id: 2,
                key_string_flags: 0,
                key: b"",
                value_string_id: 0,
                value_string_flags: 0,
                value: b"1",
            },
            EventRecordSpec::SubEntry {
                caller_fid: 3,
                caller_line: 30,
            },
        ];
        let sources = sample_sources();
        let dict_entries: &[(u64, u8, &[u8])] =
            &[(1, FLAG_UTF8, b"mixed-compose-mark"), (2, 0, b"calls")];
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile_with_string_dict_and_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                0, // single-chunk compose
                dict_entries,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, dict, n) = decode_decoded_mixed_profile_with_string_dict(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.source_records.len(), 2, "SOURCE co-kind");
            assert_eq!(dict.len(), 2);
            assert_eq!(prof.event_records.len(), 5, "codec {c}");
            assert_eq!(prof.event_sequences.len(), 5, "codec {c}");
            for (i, s) in prof.event_sequences.iter().enumerate() {
                assert_eq!(*s, Some(i as u64), "codec {c} seq[{i}]");
            }
            match &prof.event_records[0] {
                OwnedEventRecord::Mark { label } => {
                    assert_eq!(label, b"mixed-compose-mark", "codec {c}");
                }
                other => panic!("codec {c}: [0] {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (3, 30, 1));
                }
                other => panic!("codec {c}: [1] {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (3, 31, 2));
                }
                other => panic!("codec {c}: [2] {other:?}"),
            }
            match &prof.event_records[3] {
                OwnedEventRecord::Option { key, value } => {
                    assert_eq!(key, b"calls");
                    assert_eq!(value, b"1");
                }
                other => panic!("codec {c}: [3] {other:?}"),
            }
            match &prof.event_records[4] {
                OwnedEventRecord::SubEntry {
                    caller_fid,
                    caller_line,
                } => assert_eq!((*caller_fid, *caller_line), (3, 30)),
                other => panic!("codec {c}: [4] {other:?}"),
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

    #[test]
    fn mixed_string_dict_intern_none_zlib_zstd_lz4_with_source() {
        use crate::string::FLAG_UTF8;
        let events = [
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"",
            },
            EventRecordSpec::Option {
                key_string_id: 2,
                key_string_flags: 0,
                key: b"",
                value_string_id: 0,
                value_string_flags: 0,
                value: b"1",
            },
        ];
        let sources = sample_sources();
        let dict_entries: &[(u64, u8, &[u8])] =
            &[(1, FLAG_UTF8, b"mixed-dict-mark"), (2, 0, b"calls")];
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile_with_string_dict(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &events,
                &sources,
                &[],
                &[],
                dict_entries,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, dict, n) = decode_decoded_mixed_profile_with_string_dict(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.source_records.len(), 2, "SOURCE co-kind");
            assert_eq!(dict.len(), 2);
            assert_eq!(prof.event_records.len(), 2);
            match &prof.event_records[0] {
                OwnedEventRecord::Mark { label } => {
                    assert_eq!(label, b"mixed-dict-mark", "codec {c}");
                }
                other => panic!("codec {c}: {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::Option { key, value } => {
                    assert_eq!(key, b"calls");
                    assert_eq!(value, b"1");
                }
                other => panic!("codec {c}: {other:?}"),
            }
        }
    }

    #[test]
    fn mixed_site_delta_none_zlib_zstd_lz4_with_source() {
        use crate::event_body::encode_event_body_with_site_deltas;
        let specs = [
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 30,
                ticks: 1,
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 31,
                ticks: 2,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 3,
                caller_line: 30,
            },
        ];
        let event_plain = encode_event_body_with_site_deltas(&specs).unwrap();
        let sources = sample_sources();
        let src_plain = encode_source_body(&sources);
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let event_frame =
                crate::compressed_profile::encode_event_chunk(c, 0, 3, &event_plain).unwrap();
            let source_frame = crate::compressed_profile::encode_kind_chunk(
                kind::SOURCE,
                codec::NONE,
                1,
                sources.len() as u32,
                &src_plain,
            )
            .unwrap();
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &[event_frame.as_slice(), source_frame.as_slice()],
            );
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.source_records.len(), 2);
            assert_eq!(prof.event_records.len(), 3);
            match &prof.event_records[0] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (3, 30, 1));
                }
                other => panic!("codec {c}: {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (3, 31, 2));
                }
                other => panic!("codec {c}: {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::SubEntry {
                    caller_fid,
                    caller_line,
                } => assert_eq!((*caller_fid, *caller_line), (3, 30)),
                other => panic!("codec {c}: {other:?}"),
            }
        }
    }

    #[test]
    fn mixed_time_line_run_none_zlib_zstd_lz4_with_source() {
        let specs = [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 4,
            },
            EventRecordSpec::TimeLineRun {
                fid: 9,
                line: 40,
                ticks: &[11, 22, 33],
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"run-mixed",
            },
        ];
        let event_plain = encode_event_body(&specs);
        let sources = sample_sources();
        let src_plain = encode_source_body(&sources);
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let event_frame =
                crate::compressed_profile::encode_event_chunk(c, 0, 5, &event_plain).unwrap();
            let source_frame = crate::compressed_profile::encode_kind_chunk(
                kind::SOURCE,
                codec::NONE,
                1,
                sources.len() as u32,
                &src_plain,
            )
            .unwrap();
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &[event_frame.as_slice(), source_frame.as_slice()],
            );
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.source_records.len(), 2);
            // 1 plain + 3 run + 1 mark
            assert_eq!(prof.event_records.len(), 5, "codec {c}");
            match &prof.event_records[0] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (1, 1, 4));
                }
                other => panic!("codec {c}: [0] {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (9, 40, 11));
                }
                other => panic!("codec {c}: [1] {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (9, 40, 22));
                }
                other => panic!("codec {c}: [2] {other:?}"),
            }
            match &prof.event_records[3] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (9, 40, 33));
                }
                other => panic!("codec {c}: [3] {other:?}"),
            }
            match &prof.event_records[4] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"run-mixed"),
                other => panic!("codec {c}: [4] {other:?}"),
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

    #[test]
    fn mixed_event_seq_none_zlib_zstd_lz4_with_source() {
        use crate::event_body::encode_event_body_with_seq;
        let specs = [
            EventRecordSpec::Version {
                major: 5,
                minor: 0,
            },
            EventRecordSpec::Comment {
                string_id: 0,
                string_flags: 0,
                text: b"# seq mixed",
            },
            EventRecordSpec::StartDeflate,
            EventRecordSpec::PidStart {
                pid: 42,
                ppid: 1,
                start_time: 100,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 7,
            },
            EventRecordSpec::PidEnd {
                pid: 42,
                end_time: 200,
            },
        ];
        let event_plain = encode_event_body_with_seq(&specs);
        let sources = sample_sources();
        let src_plain = encode_source_body(&sources);
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let event_frame =
                crate::compressed_profile::encode_event_chunk(c, 0, 6, &event_plain).unwrap();
            let source_frame = crate::compressed_profile::encode_kind_chunk(
                kind::SOURCE,
                codec::NONE,
                1,
                sources.len() as u32,
                &src_plain,
            )
            .unwrap();
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &[event_frame.as_slice(), source_frame.as_slice()],
            );
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.source_records.len(), 2);
            assert_eq!(prof.event_records.len(), 6, "codec {c}");
            assert_eq!(prof.event_sequences.len(), 6, "codec {c}");
            for (i, s) in prof.event_sequences.iter().enumerate() {
                assert_eq!(*s, Some(i as u64), "codec {c} seq[{i}]");
            }
            match &prof.event_records[0] {
                OwnedEventRecord::Version { major, minor } => {
                    assert_eq!((*major, *minor), (5, 0));
                }
                other => panic!("codec {c}: [0] {other:?}"),
            }
            match &prof.event_records[4] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (1, 10, 7));
                }
                other => panic!("codec {c}: [4] {other:?}"),
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

    #[test]
    fn mixed_site_delta_and_seq_compose_none_zlib_zstd_lz4_with_source() {
        use crate::event_body::encode_event_body_with_site_deltas_and_seq;
        let specs = [
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 30,
                ticks: 1,
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 31,
                ticks: 2,
            },
            EventRecordSpec::TimeBlock {
                fid: 3,
                line: 32,
                block_line: 4,
                ticks: 9,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 3,
                caller_line: 30,
            },
        ];
        let event_plain = encode_event_body_with_site_deltas_and_seq(&specs).unwrap();
        let sources = sample_sources();
        let src_plain = encode_source_body(&sources);
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let event_frame =
                crate::compressed_profile::encode_event_chunk(c, 0, 4, &event_plain).unwrap();
            let source_frame = crate::compressed_profile::encode_kind_chunk(
                kind::SOURCE,
                codec::NONE,
                1,
                sources.len() as u32,
                &src_plain,
            )
            .unwrap();
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &[event_frame.as_slice(), source_frame.as_slice()],
            );
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.source_records.len(), 2);
            assert_eq!(prof.event_records.len(), 4, "codec {c}");
            assert_eq!(prof.event_sequences.len(), 4, "codec {c}");
            for (i, s) in prof.event_sequences.iter().enumerate() {
                assert_eq!(*s, Some(i as u64), "codec {c} seq[{i}]");
            }
            match &prof.event_records[0] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (3, 30, 1));
                }
                other => panic!("codec {c}: [0] {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (3, 31, 2));
                }
                other => panic!("codec {c}: [1] {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                } => assert_eq!((*fid, *line, *block_line, *ticks), (3, 32, 4, 9)),
                other => panic!("codec {c}: [2] {other:?}"),
            }
            match &prof.event_records[3] {
                OwnedEventRecord::SubEntry {
                    caller_fid,
                    caller_line,
                } => assert_eq!((*caller_fid, *caller_line), (3, 30)),
                other => panic!("codec {c}: [3] {other:?}"),
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

    #[test]
    fn mixed_multi_chunk_site_delta_seq_packing_none_zlib_zstd_lz4_with_source() {
        use crate::event_body::{
            encode_event_body_with_site_deltas_and_seq,
            encode_event_body_with_site_deltas_and_seq_continuing, PackingEncodeState,
        };
        use crate::multi_chunk_event::partition_event_records;
        let specs = [
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 30,
                ticks: 1,
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 31,
                ticks: 2,
            },
            EventRecordSpec::TimeBlock {
                fid: 3,
                line: 32,
                block_line: 4,
                ticks: 9,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 3,
                caller_line: 30,
            },
            EventRecordSpec::TimeLine {
                fid: 4,
                line: 1,
                ticks: 3,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"mc-end",
            },
        ];
        let single_plain = encode_event_body_with_site_deltas_and_seq(&specs).unwrap();
        let parts = partition_event_records(&specs, 2);
        assert!(parts.len() >= 2);
        let sources = sample_sources();
        let src_plain = encode_source_body(&sources);
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let mut packing = PackingEncodeState::new();
            let mut event_frames: Vec<Vec<u8>> = Vec::new();
            for (i, part) in parts.iter().enumerate() {
                let plain =
                    encode_event_body_with_site_deltas_and_seq_continuing(part, &mut packing)
                        .unwrap();
                event_frames.push(
                    crate::compressed_profile::encode_event_chunk(
                        c,
                        i as u64,
                        part.len() as u32,
                        &plain,
                    )
                    .unwrap(),
                );
            }
            let source_frame = crate::compressed_profile::encode_kind_chunk(
                kind::SOURCE,
                codec::NONE,
                event_frames.len() as u64,
                sources.len() as u32,
                &src_plain,
            )
            .unwrap();
            let mut refs: Vec<&[u8]> = event_frames.iter().map(|v| v.as_slice()).collect();
            refs.push(source_frame.as_slice());
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &refs,
            );
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.event_chunk_count, parts.len(), "codec {c}");
            assert_eq!(prof.source_records.len(), 2);
            assert_eq!(prof.event_records.len(), 6, "codec {c}");
            for (i, s) in prof.event_sequences.iter().enumerate() {
                assert_eq!(*s, Some(i as u64), "codec {c} seq[{i}]");
            }
            match &prof.event_records[0] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (3, 30, 1));
                }
                other => panic!("codec {c}: [0] {other:?}"),
            }
            match &prof.event_records[4] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (4, 1, 3));
                }
                other => panic!("codec {c}: [4] {other:?}"),
            }
            // Joined EVENT plain equals single-chunk packing body.
            let (stream_plain, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            let mut joined = Vec::new();
            for ch in &stream_plain.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, single_plain, "codec {c}: multi-chunk join wire");

            if c != codec::NONE {
                let (stream_raw, _) = decode_prefix_chunk_stream(&wire).expect("raw");
                assert_eq!(stream_raw.chunks[0].codec, c);
                assert_ne!(
                    stream_raw.chunks[0].payload,
                    stream_plain.chunks[0].plain.as_slice(),
                    "codec {c}: default parse must not inflate"
                );
            }
        }
    }

    #[test]
    fn mixed_string_dict_multi_chunk_site_delta_seq_packing_none_zlib_zstd_lz4_with_source() {
        use crate::string::FLAG_UTF8;
        let specs = [
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"",
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 30,
                ticks: 1,
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 31,
                ticks: 2,
            },
            EventRecordSpec::TimeBlock {
                fid: 3,
                line: 32,
                block_line: 4,
                ticks: 9,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 3,
                caller_line: 30,
            },
            EventRecordSpec::Comment {
                string_id: 2,
                string_flags: 0,
                text: b"",
            },
        ];
        let dict_entries: &[(u64, u8, &[u8])] = &[
            (1, FLAG_UTF8, b"mixed-mc-dict-mark"),
            (2, 0, b"# mixed-mc-dict-end"),
        ];
        let sources = sample_sources();
        let parts = partition_event_records(&specs, 2);
        assert!(parts.len() >= 2);

        // Single-chunk dict+packing baseline of same specs (+ SOURCE).
        let single_wire = encode_decoded_mixed_profile_with_string_dict_and_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &specs,
            &sources,
            &[],
            &[],
            0,
            dict_entries,
        )
        .expect("single-chunk mixed dict+packing");
        let (single_prof, single_dict, _) =
            decode_decoded_mixed_profile_with_string_dict(&single_wire, true).unwrap();
        assert_eq!(single_prof.event_records.len(), 6);
        assert_eq!(single_dict.get(1).unwrap().data, b"mixed-mc-dict-mark");

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile_with_string_dict_and_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &specs,
                &sources,
                &[],
                &[],
                2, // multi-chunk packing + FOOTER dict
                dict_entries,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, dict, n) = decode_decoded_mixed_profile_with_string_dict(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.event_chunk_count, parts.len(), "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "SOURCE co-kind");
            assert_eq!(dict.len(), 2, "codec {c}");
            assert_eq!(dict.get(1).unwrap().data, b"mixed-mc-dict-mark");
            assert_eq!(dict.get(2).unwrap().data, b"# mixed-mc-dict-end");
            assert_eq!(prof.event_records.len(), 6, "codec {c}");
            assert_eq!(prof.event_sequences.len(), 6, "codec {c}");
            for (i, s) in prof.event_sequences.iter().enumerate() {
                assert_eq!(*s, Some(i as u64), "codec {c} seq[{i}]");
            }
            match &prof.event_records[0] {
                OwnedEventRecord::Mark { label } => {
                    assert_eq!(label, b"mixed-mc-dict-mark", "codec {c}");
                }
                other => panic!("codec {c}: [0] {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (3, 30, 1));
                }
                other => panic!("codec {c}: [1] {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (3, 31, 2));
                }
                other => panic!("codec {c}: [2] {other:?}"),
            }
            match &prof.event_records[3] {
                OwnedEventRecord::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                } => assert_eq!((*fid, *line, *block_line, *ticks), (3, 32, 4, 9)),
                other => panic!("codec {c}: [3] {other:?}"),
            }
            match &prof.event_records[4] {
                OwnedEventRecord::SubEntry {
                    caller_fid,
                    caller_line,
                } => assert_eq!((*caller_fid, *caller_line), (3, 30)),
                other => panic!("codec {c}: [4] {other:?}"),
            }
            match &prof.event_records[5] {
                OwnedEventRecord::Comment { text } => {
                    assert_eq!(text, b"# mixed-mc-dict-end", "codec {c}");
                }
                other => panic!("codec {c}: [5] {other:?}"),
            }
            // Multi-chunk join equals single-chunk compose of same specs.
            assert_eq!(prof.event_records, single_prof.event_records, "codec {c}");
            assert_eq!(
                prof.event_sequences, single_prof.event_sequences,
                "codec {c}"
            );

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

    #[test]
    fn mixed_multi_chunk_packing_with_time_runs_none_zlib_zstd_lz4_with_source() {
        use crate::event_body::{
            encode_event_body_with_site_deltas_and_seq,
            encode_event_body_with_site_deltas_and_seq_continuing, PackingEncodeState,
        };
        use crate::multi_chunk_event::partition_event_records;
        const TL: &[u64] = &[7, 8];
        const TB: &[u64] = &[10, 20];
        let specs = [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 50,
                ticks: TL,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 51,
                ticks: 9,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 2,
                caller_line: 50,
            },
            EventRecordSpec::TimeBlockRun {
                fid: 3,
                line: 8,
                block_line: 6,
                ticks: TB,
            },
            EventRecordSpec::TimeBlock {
                fid: 3,
                line: 9,
                block_line: 7,
                ticks: 3,
            },
        ];
        let sources = sample_sources();
        let parts = partition_event_records(&specs, 2);
        assert!(parts.len() >= 3);
        assert!(matches!(
            parts[0].last(),
            Some(EventRecordSpec::TimeLineRun { .. })
        ));

        let single_plain = encode_event_body_with_site_deltas_and_seq(&specs).unwrap();
        let mut packing_chk = PackingEncodeState::new();
        let mut joined = Vec::new();
        for part in &parts {
            joined.extend_from_slice(
                &encode_event_body_with_site_deltas_and_seq_continuing(part, &mut packing_chk)
                    .unwrap(),
            );
        }
        assert_eq!(joined, single_plain);

        // Single-chunk packing baseline + SOURCE co-kind (record-aligned frames).
        let src_plain = encode_source_body(&sources);
        let single_event_frame = crate::compressed_profile::encode_event_chunk(
            codec::NONE,
            0,
            specs.len() as u32,
            &single_plain,
        )
        .unwrap();
        let single_source_frame = crate::compressed_profile::encode_kind_chunk(
            kind::SOURCE,
            codec::NONE,
            1,
            sources.len() as u32,
            &src_plain,
        )
        .unwrap();
        let single_wire = encode_prefix_sealed_chunks(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            &[
                single_event_frame.as_slice(),
                single_source_frame.as_slice(),
            ],
        );
        let (single_prof, n0) = decode_decoded_mixed_profile(&single_wire, true).unwrap();
        assert_eq!(n0, single_wire.len());
        assert_eq!(single_prof.event_records.len(), 8);
        assert_eq!(single_prof.source_records.len(), 2);
        match &single_prof.event_records[3] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("single [3] {other:?}"),
        }

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let mut packing = PackingEncodeState::new();
            let mut event_frames: Vec<Vec<u8>> = Vec::new();
            for (i, part) in parts.iter().enumerate() {
                let plain =
                    encode_event_body_with_site_deltas_and_seq_continuing(part, &mut packing)
                        .unwrap();
                event_frames.push(
                    crate::compressed_profile::encode_event_chunk(
                        c,
                        i as u64,
                        part.len() as u32,
                        &plain,
                    )
                    .unwrap(),
                );
            }
            let source_frame = crate::compressed_profile::encode_kind_chunk(
                kind::SOURCE,
                codec::NONE,
                event_frames.len() as u64,
                sources.len() as u32,
                &src_plain,
            )
            .unwrap();
            let mut refs: Vec<&[u8]> = event_frames.iter().map(|v| v.as_slice()).collect();
            refs.push(source_frame.as_slice());
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &refs,
            );
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.event_chunk_count, parts.len(), "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "SOURCE co-kind");
            assert_eq!(prof.event_records.len(), 8, "codec {c}");
            assert_eq!(prof.event_sequences.len(), 8, "codec {c}");
            for (i, s) in prof.event_sequences.iter().enumerate() {
                assert_eq!(*s, Some(i as u64), "codec {c} seq[{i}]");
            }
            match &prof.event_records[3] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!(
                        (*fid, *line, *ticks),
                        (2, 51, 9),
                        "codec {c}: post-run site-delta across chunk"
                    );
                }
                other => panic!("codec {c}: [3] {other:?}"),
            }
            match &prof.event_records[7] {
                OwnedEventRecord::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                } => assert_eq!(
                    (*fid, *line, *block_line, *ticks),
                    (3, 9, 7, 3),
                    "codec {c}: post TIME_BLOCK_RUN site-delta"
                ),
                other => panic!("codec {c}: [7] {other:?}"),
            }
            assert_eq!(
                prof.event_records, single_prof.event_records,
                "codec {c}"
            );
            assert_eq!(
                prof.event_sequences, single_prof.event_sequences,
                "codec {c}"
            );

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

    #[test]
    fn mixed_auto_version_dict_multi_chunk_packing_with_time_runs_none_zlib_zstd_lz4_with_source()
    {
        use crate::string::FLAG_UTF8;
        const TL: &[u64] = &[7, 8];
        const TB: &[u64] = &[10, 20];
        let workload = [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 50,
                ticks: TL,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 51,
                ticks: 9,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 2,
                caller_line: 50,
            },
            EventRecordSpec::TimeBlockRun {
                fid: 3,
                line: 8,
                block_line: 6,
                ticks: TB,
            },
            EventRecordSpec::TimeBlock {
                fid: 3,
                line: 9,
                block_line: 7,
                ticks: 3,
            },
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"",
            },
            EventRecordSpec::Comment {
                string_id: 2,
                string_flags: 0,
                text: b"",
            },
        ];
        let dict_entries: &[(u64, u8, &[u8])] = &[
            (1, FLAG_UTF8, b"mixed-av-dict-mc-mark"),
            (2, 0, b"# mixed-av-dict-mc-end"),
        ];
        let sources = sample_sources();
        let header_minor = 5u16;

        let single_wire =
            encode_decoded_mixed_profile_auto_version_with_string_dict_and_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                header_minor,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: codec::NONE,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &workload,
                &sources,
                &[],
                &[],
                0,
                dict_entries,
            )
            .expect("single-chunk mixed auto-version dict packing");
        let (single_prof, single_dict, _) =
            decode_decoded_mixed_profile_auto_version_with_string_dict(&single_wire, true)
                .unwrap();
        assert_eq!(single_prof.source_records.len(), 2);
        assert_eq!(single_dict.get(1).unwrap().data, b"mixed-av-dict-mc-mark");
        match &single_prof.event_records[0] {
            OwnedEventRecord::Version { major, minor } => {
                assert_eq!(
                    (*major, *minor),
                    (u64::from(SUPPORTED_MAJOR), u64::from(header_minor))
                );
            }
            other => panic!("expected VERSION, got {other:?}"),
        }
        match &single_prof.event_records[4] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("post-run [4] {other:?}"),
        }

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire =
                encode_decoded_mixed_profile_auto_version_with_string_dict_and_site_deltas_and_seq(
                    SUPPORTED_MAJOR,
                    header_minor,
                    0,
                    0,
                    0,
                    &[],
                    KindCodecs {
                        event: c,
                        source: codec::NONE,
                        index: codec::NONE,
                        summary: codec::NONE,
                    },
                    &workload,
                    &sources,
                    &[],
                    &[],
                    1,
                    dict_entries,
                )
                .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, dict, n) =
                decode_decoded_mixed_profile_auto_version_with_string_dict(&wire, true)
                    .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert!(prof.event_chunk_count >= 4, "codec {c}");
            assert_eq!(prof.source_records.len(), 2);
            assert_eq!(dict.get(1).unwrap().data, b"mixed-av-dict-mc-mark");
            assert_eq!(dict.get(2).unwrap().data, b"# mixed-av-dict-mc-end");
            assert_eq!(
                prof.event_records, single_prof.event_records,
                "codec {c}"
            );
            assert_eq!(
                prof.event_sequences, single_prof.event_sequences,
                "codec {c}"
            );
            match &prof.event_records[4] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!(
                        (*fid, *line, *ticks),
                        (2, 51, 9),
                        "codec {c}: post-run across chunk"
                    );
                }
                other => panic!("codec {c}: [4] {other:?}"),
            }
            let last = prof.event_records.len() - 1;
            match &prof.event_records[last] {
                OwnedEventRecord::Comment { text } => {
                    assert_eq!(text, b"# mixed-av-dict-mc-end", "codec {c}");
                }
                other => panic!("codec {c}: last {other:?}"),
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

    #[test]
    fn mixed_auto_version_multi_chunk_packing_with_time_runs_none_zlib_zstd_lz4_with_source() {
        const TL: &[u64] = &[7, 8];
        const TB: &[u64] = &[10, 20];
        let workload = [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 50,
                ticks: TL,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 51,
                ticks: 9,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 2,
                caller_line: 50,
            },
            EventRecordSpec::TimeBlockRun {
                fid: 3,
                line: 8,
                block_line: 6,
                ticks: TB,
            },
            EventRecordSpec::TimeBlock {
                fid: 3,
                line: 9,
                block_line: 7,
                ticks: 3,
            },
        ];
        let sources = sample_sources();
        let header_minor = 2u16;

        let single_wire = encode_decoded_mixed_profile_auto_version_with_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            header_minor,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &workload,
            &sources,
            &[],
            &[],
            0,
        )
        .expect("single-chunk mixed auto-version packing");
        let (single_prof, _) =
            decode_decoded_mixed_profile_auto_version(&single_wire, true).unwrap();
        assert_eq!(single_prof.source_records.len(), 2);
        match &single_prof.event_records[0] {
            OwnedEventRecord::Version { major, minor } => {
                assert_eq!(
                    (*major, *minor),
                    (u64::from(SUPPORTED_MAJOR), u64::from(header_minor))
                );
            }
            other => panic!("expected VERSION, got {other:?}"),
        }
        match &single_prof.event_records[4] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("post-run [4] {other:?}"),
        }

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile_auto_version_with_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                header_minor,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &workload,
                &sources,
                &[],
                &[],
                1, // multi-chunk; run and post-run in different EVENT chunks
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_mixed_profile_auto_version(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert!(prof.event_chunk_count >= 4, "codec {c}");
            assert_eq!(prof.source_records.len(), 2);
            assert_eq!(
                prof.event_records, single_prof.event_records,
                "codec {c}"
            );
            assert_eq!(
                prof.event_sequences, single_prof.event_sequences,
                "codec {c}"
            );
            match &prof.event_records[4] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!(
                        (*fid, *line, *ticks),
                        (2, 51, 9),
                        "codec {c}: post-run across chunk"
                    );
                }
                other => panic!("codec {c}: [4] {other:?}"),
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

    #[test]
    fn mixed_string_dict_multi_chunk_packing_with_time_runs_none_zlib_zstd_lz4_with_source() {
        use crate::string::FLAG_UTF8;
        const TL: &[u64] = &[7, 8];
        const TB: &[u64] = &[10, 20];
        let specs = [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 50,
                ticks: TL,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 51,
                ticks: 9,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 2,
                caller_line: 50,
            },
            EventRecordSpec::TimeBlockRun {
                fid: 3,
                line: 8,
                block_line: 6,
                ticks: TB,
            },
            EventRecordSpec::TimeBlock {
                fid: 3,
                line: 9,
                block_line: 7,
                ticks: 3,
            },
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"",
            },
            EventRecordSpec::Comment {
                string_id: 2,
                string_flags: 0,
                text: b"",
            },
        ];
        let dict_entries: &[(u64, u8, &[u8])] = &[
            (1, FLAG_UTF8, b"mixed-dict-run-mc-mark"),
            (2, 0, b"# mixed-dict-run-mc-end"),
        ];
        let sources = sample_sources();
        let parts = partition_event_records(&specs, 2);
        assert!(parts.len() >= 3);
        assert!(matches!(
            parts[0].last(),
            Some(EventRecordSpec::TimeLineRun { .. })
        ));

        let single_wire = encode_decoded_mixed_profile_with_string_dict_and_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &specs,
            &sources,
            &[],
            &[],
            0,
            dict_entries,
        )
        .expect("single-chunk mixed dict+packing+run");
        let (single_prof, single_dict, _) =
            decode_decoded_mixed_profile_with_string_dict(&single_wire, true).unwrap();
        assert_eq!(single_prof.event_records.len(), 10);
        assert_eq!(single_prof.source_records.len(), 2);
        assert_eq!(single_dict.get(1).unwrap().data, b"mixed-dict-run-mc-mark");
        match &single_prof.event_records[3] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("single [3] {other:?}"),
        }
        match &single_prof.event_records[8] {
            OwnedEventRecord::Mark { label } => {
                assert_eq!(label, b"mixed-dict-run-mc-mark");
            }
            other => panic!("single [8] {other:?}"),
        }

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_mixed_profile_with_string_dict_and_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                KindCodecs {
                    event: c,
                    source: codec::NONE,
                    index: codec::NONE,
                    summary: codec::NONE,
                },
                &specs,
                &sources,
                &[],
                &[],
                2, // multi-chunk packing + runs + FOOTER dict
                dict_entries,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, dict, n) = decode_decoded_mixed_profile_with_string_dict(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.event_chunk_count, parts.len(), "codec {c}");
            assert_eq!(prof.source_records.len(), 2, "SOURCE co-kind");
            assert_eq!(dict.len(), 2, "codec {c}");
            assert_eq!(dict.get(1).unwrap().data, b"mixed-dict-run-mc-mark");
            assert_eq!(dict.get(2).unwrap().data, b"# mixed-dict-run-mc-end");
            assert_eq!(prof.event_records.len(), 10, "codec {c}");
            assert_eq!(prof.event_sequences.len(), 10, "codec {c}");
            for (i, s) in prof.event_sequences.iter().enumerate() {
                assert_eq!(*s, Some(i as u64), "codec {c} seq[{i}]");
            }
            match &prof.event_records[3] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!(
                        (*fid, *line, *ticks),
                        (2, 51, 9),
                        "codec {c}: post-run site-delta across chunk"
                    );
                }
                other => panic!("codec {c}: [3] {other:?}"),
            }
            match &prof.event_records[8] {
                OwnedEventRecord::Mark { label } => {
                    assert_eq!(label, b"mixed-dict-run-mc-mark", "codec {c}");
                }
                other => panic!("codec {c}: [8] {other:?}"),
            }
            match &prof.event_records[9] {
                OwnedEventRecord::Comment { text } => {
                    assert_eq!(text, b"# mixed-dict-run-mc-end", "codec {c}");
                }
                other => panic!("codec {c}: [9] {other:?}"),
            }
            assert_eq!(
                prof.event_records, single_prof.event_records,
                "codec {c}"
            );
            assert_eq!(
                prof.event_sequences, single_prof.event_sequences,
                "codec {c}"
            );

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

    #[test]
    fn mixed_string_dict_multi_chunk_packing_unknown_id_fail_closed() {
        let specs = [
            EventRecordSpec::Mark {
                string_id: 99,
                string_flags: 0,
                label: b"",
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 1,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 6,
                ticks: 2,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 1,
                caller_line: 5,
            },
        ];
        let sources = sample_sources();
        assert!(
            partition_event_records(&specs, 2).len() >= 2,
            "must partition to multi-chunk"
        );
        let wire = encode_decoded_mixed_profile_with_string_dict_and_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &specs,
            &sources,
            &[],
            &[],
            2, // multi-chunk packing + FOOTER dict
            &[(1, 0, b"other")],
        )
        .expect("encode");
        match decode_decoded_mixed_profile_with_string_dict(&wire, true) {
            Err(DecodedMixedError::StringDict(
                crate::string_dict::StringDictError::UnknownId { id: 99 },
            )) => {}
            other => panic!("expected UnknownId 99, got {other:?}"),
        }
    }

    #[test]
    fn mixed_time_block_run_none_zlib_zstd_lz4_with_source() {
        let specs = [
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 1,
                block_line: 1,
                ticks: 4,
            },
            EventRecordSpec::TimeBlockRun {
                fid: 9,
                line: 40,
                block_line: 4,
                ticks: &[11, 22, 33],
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 5,
                ticks: &[7, 8],
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"block-run-mixed",
            },
        ];
        let event_plain = encode_event_body(&specs);
        let sources = sample_sources();
        let src_plain = encode_source_body(&sources);
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let event_frame =
                crate::compressed_profile::encode_event_chunk(c, 0, 7, &event_plain).unwrap();
            let source_frame = crate::compressed_profile::encode_kind_chunk(
                kind::SOURCE,
                codec::NONE,
                1,
                sources.len() as u32,
                &src_plain,
            )
            .unwrap();
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &[event_frame.as_slice(), source_frame.as_slice()],
            );
            let (prof, n) = decode_decoded_mixed_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.kind_codecs.event, c);
            assert_eq!(prof.source_records.len(), 2);
            // 1 plain TB + 3 TB run + 2 TL run + 1 mark = 7
            assert_eq!(prof.event_records.len(), 7, "codec {c}");
            match &prof.event_records[0] {
                OwnedEventRecord::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                } => assert_eq!((*fid, *line, *block_line, *ticks), (1, 1, 1, 4)),
                other => panic!("codec {c}: [0] {other:?}"),
            }
            match &prof.event_records[1] {
                OwnedEventRecord::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                } => assert_eq!((*fid, *line, *block_line, *ticks), (9, 40, 4, 11)),
                other => panic!("codec {c}: [1] {other:?}"),
            }
            match &prof.event_records[2] {
                OwnedEventRecord::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                } => assert_eq!((*fid, *line, *block_line, *ticks), (9, 40, 4, 22)),
                other => panic!("codec {c}: [2] {other:?}"),
            }
            match &prof.event_records[3] {
                OwnedEventRecord::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                } => assert_eq!((*fid, *line, *block_line, *ticks), (9, 40, 4, 33)),
                other => panic!("codec {c}: [3] {other:?}"),
            }
            match &prof.event_records[4] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (2, 5, 7));
                }
                other => panic!("codec {c}: [4] {other:?}"),
            }
            match &prof.event_records[5] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), (2, 5, 8));
                }
                other => panic!("codec {c}: [5] {other:?}"),
            }
            match &prof.event_records[6] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"block-run-mixed"),
                other => panic!("codec {c}: [6] {other:?}"),
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

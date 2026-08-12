//! Provisional **format v6** compressed multi-kind mixed profile (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-compressed-mixed-provisional-v0.md`
//!
//! Composes EVENT/SOURCE/INDEX/SUMMARY bodies under NONE/ZLIB/ZSTD/LZ4 with
//! optional FOOTER (codec NONE, last). Default `parse_chunk_frame` stays
//! non-inflating; decode uses `decode_chunk_payload` explicitly.
//! Not dictionaries, not COL-007 C writer.

use crate::chunk::{codec, encode_chunk_frame, kind, ChunkError};
use crate::compressed_profile::{
    encode_kind_chunk, is_supported_event_codec, OwnedEventRecord,
};
use crate::crc::compute_payload_crc;
use crate::event_body::{
    decode_event_body, encode_event_body, EventBodyError, EventRecordSpec,
};
use crate::file_prefix::encode_file_prefix;
use crate::footer_body::{
    decode_footer_body, encode_footer_body, FooterBodyError, FooterRecordSpec,
};
use crate::index_body::{
    decode_index_body, encode_index_body, IndexBodyError, IndexRecordSpec,
};
use crate::multi_chunk_event::partition_event_records;
use crate::payload_codec::{decode_chunk_payload, PayloadCodecError};
use crate::source_body::{
    decode_source_body, encode_source_body, SourceBodyError, SourceRecordSpec,
};
use crate::stream::{decode_prefix_chunk_stream, StreamError};
use crate::summary_body::{
    decode_summary_body, encode_summary_body, SummaryBodyError, SummaryRecordSpec,
};
use crate::FixedHeader;

/// Fail-closed compressed mixed-profile errors.
#[derive(Debug, PartialEq, Eq)]
pub enum CompressedMixedError {
    Stream(StreamError),
    EventBody(EventBodyError),
    SourceBody(SourceBodyError),
    IndexBody(IndexBodyError),
    SummaryBody(SummaryBodyError),
    FooterBody(FooterBodyError),
    Payload(PayloadCodecError),
    UnsupportedCodec { codec: u8 },
    UnexpectedKind { kind: u8 },
    InvalidFooter,
    UnexpectedFooterCodec { codec: u8 },
}

impl std::fmt::Display for CompressedMixedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressedMixedError::Stream(e) => write!(f, "compressed mixed stream: {e}"),
            CompressedMixedError::EventBody(e) => write!(f, "compressed mixed event-body: {e}"),
            CompressedMixedError::SourceBody(e) => write!(f, "compressed mixed source-body: {e}"),
            CompressedMixedError::IndexBody(e) => write!(f, "compressed mixed index-body: {e}"),
            CompressedMixedError::SummaryBody(e) => {
                write!(f, "compressed mixed summary-body: {e}")
            }
            CompressedMixedError::FooterBody(e) => write!(f, "compressed mixed footer-body: {e}"),
            CompressedMixedError::Payload(e) => write!(f, "compressed mixed payload: {e}"),
            CompressedMixedError::UnsupportedCodec { codec } => {
                write!(f, "compressed mixed unsupported codec {codec}")
            }
            CompressedMixedError::UnexpectedKind { kind } => {
                write!(f, "compressed mixed unexpected kind {kind}")
            }
            CompressedMixedError::InvalidFooter => {
                write!(f, "compressed mixed invalid FOOTER placement")
            }
            CompressedMixedError::UnexpectedFooterCodec { codec } => {
                write!(f, "compressed mixed FOOTER codec {codec} (NONE required)")
            }
        }
    }
}

impl std::error::Error for CompressedMixedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CompressedMixedError::Stream(e) => Some(e),
            CompressedMixedError::EventBody(e) => Some(e),
            CompressedMixedError::SourceBody(e) => Some(e),
            CompressedMixedError::IndexBody(e) => Some(e),
            CompressedMixedError::SummaryBody(e) => Some(e),
            CompressedMixedError::FooterBody(e) => Some(e),
            CompressedMixedError::Payload(e) => Some(e),
            _ => None,
        }
    }
}

impl From<StreamError> for CompressedMixedError {
    fn from(e: StreamError) -> Self {
        CompressedMixedError::Stream(e)
    }
}
impl From<EventBodyError> for CompressedMixedError {
    fn from(e: EventBodyError) -> Self {
        CompressedMixedError::EventBody(e)
    }
}
impl From<SourceBodyError> for CompressedMixedError {
    fn from(e: SourceBodyError) -> Self {
        CompressedMixedError::SourceBody(e)
    }
}
impl From<IndexBodyError> for CompressedMixedError {
    fn from(e: IndexBodyError) -> Self {
        CompressedMixedError::IndexBody(e)
    }
}
impl From<SummaryBodyError> for CompressedMixedError {
    fn from(e: SummaryBodyError) -> Self {
        CompressedMixedError::SummaryBody(e)
    }
}
impl From<FooterBodyError> for CompressedMixedError {
    fn from(e: FooterBodyError) -> Self {
        CompressedMixedError::FooterBody(e)
    }
}
impl From<PayloadCodecError> for CompressedMixedError {
    fn from(e: PayloadCodecError) -> Self {
        CompressedMixedError::Payload(e)
    }
}
impl From<crate::compressed_profile::CompressedProfileError> for CompressedMixedError {
    fn from(e: crate::compressed_profile::CompressedProfileError) -> Self {
        use crate::compressed_profile::CompressedProfileError as C;
        match e {
            C::Stream(s) => CompressedMixedError::Stream(s),
            C::EventBody(b) => CompressedMixedError::EventBody(b),
            C::SourceBody(b) => CompressedMixedError::SourceBody(b),
            C::IndexBody(b) => CompressedMixedError::IndexBody(b),
            C::SummaryBody(b) => CompressedMixedError::SummaryBody(b),
            C::Payload(p) => CompressedMixedError::Payload(p),
            C::UnsupportedEventCodec { codec } => {
                CompressedMixedError::UnsupportedCodec { codec }
            }
            C::UnexpectedKind { kind } => CompressedMixedError::UnexpectedKind { kind },
            C::InvalidFooter => CompressedMixedError::InvalidFooter,
            C::UnexpectedFooterCodec { codec } => {
                CompressedMixedError::UnexpectedFooterCodec { codec }
            }
        }
    }
}
impl From<ChunkError> for CompressedMixedError {
    fn from(e: ChunkError) -> Self {
        CompressedMixedError::Stream(StreamError::Chunk(e))
    }
}

pub type CompressedMixedResult<T> = std::result::Result<T, CompressedMixedError>;

/// Owned SOURCE record after inflate + body decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedSourceRecord {
    pub fid: u64,
    pub line: u64,
    pub text: Vec<u8>,
}

/// Owned INDEX record after inflate + body decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedIndexRecord {
    pub key_id: u64,
    pub file_offset: u64,
    pub length: u64,
    pub label: Vec<u8>,
}

/// Owned SUMMARY record after inflate + body decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedSummaryRecord {
    pub key_id: u64,
    pub count: u64,
    pub value: u64,
    pub label: Vec<u8>,
}

/// Owned FOOTER record (FOOTER stays uncompressed codec NONE).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedFooterRecord {
    pub key_id: u64,
    pub value: u64,
    pub label: Vec<u8>,
}

/// Per-kind payload codecs for EVENT/SOURCE/INDEX/SUMMARY (FOOTER is always NONE).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindCodecs {
    pub event: u8,
    pub source: u8,
    pub index: u8,
    pub summary: u8,
}

impl KindCodecs {
    /// Same codec for all compressible kinds (shared-codec MVP path).
    pub const fn uniform(codec: u8) -> Self {
        Self {
            event: codec,
            source: codec,
            index: codec,
            summary: codec,
        }
    }

}

/// Decoded compressed multi-kind mixed profile (owned logical records).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressedMixedProfile {
    pub header: FixedHeader,
    /// First compressible kind's codec (compat); prefer [`Self::kind_codecs`].
    pub payload_codec: u8,
    /// Observed codecs for kinds that appeared (NONE if kind absent).
    pub kind_codecs: KindCodecs,
    pub event_records: Vec<OwnedEventRecord>,
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

/// Encode a compressed multi-kind mixed profile with a **shared** payload codec.
///
/// Wire order when non-empty: EVENT, SOURCE, INDEX, SUMMARY, optional FOOTER last.
/// `payload_codec` applies to EVENT/SOURCE/INDEX/SUMMARY; FOOTER is always codec NONE.
///
/// For different codecs per kind, use [`encode_compressed_mixed_profile_per_kind`].
pub fn encode_compressed_mixed_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    payload_codec: u8,
    events: &[EventRecordSpec<'_>],
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> CompressedMixedResult<Vec<u8>> {
    encode_compressed_mixed_profile_per_kind(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        KindCodecs::uniform(payload_codec),
        events,
        sources,
        indexes,
        summaries,
        footer,
    )
}

/// Encode a compressed multi-kind mixed profile with **per-kind** payload codecs.
///
/// Each of EVENT/SOURCE/INDEX/SUMMARY may use NONE/ZLIB/ZSTD/LZ4 independently.
/// FOOTER remains codec NONE and last when present.
pub fn encode_compressed_mixed_profile_per_kind(
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
) -> CompressedMixedResult<Vec<u8>> {
    for (present, c) in [
        (!events.is_empty(), codecs.event),
        (!sources.is_empty(), codecs.source),
        (!indexes.is_empty(), codecs.index),
        (!summaries.is_empty(), codecs.summary),
    ] {
        if present && !is_supported_event_codec(c) {
            return Err(CompressedMixedError::UnsupportedCodec { codec: c });
        }
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
        let frame =
            encode_kind_chunk(kind::EVENT, codecs.event, seq, events.len() as u32, &plain)?;
        out.extend_from_slice(&frame);
        seq += 1;
    }
    if !sources.is_empty() {
        let plain = encode_source_body(sources);
        let frame = encode_kind_chunk(
            kind::SOURCE,
            codecs.source,
            seq,
            sources.len() as u32,
            &plain,
        )?;
        out.extend_from_slice(&frame);
        seq += 1;
    }
    if !indexes.is_empty() {
        let plain = encode_index_body(indexes);
        let frame =
            encode_kind_chunk(kind::INDEX, codecs.index, seq, indexes.len() as u32, &plain)?;
        out.extend_from_slice(&frame);
        seq += 1;
    }
    if !summaries.is_empty() {
        let plain = encode_summary_body(summaries);
        let frame = encode_kind_chunk(
            kind::SUMMARY,
            codecs.summary,
            seq,
            summaries.len() as u32,
            &plain,
        )?;
        out.extend_from_slice(&frame);
        seq += 1;
    }
    if let Some(recs) = footer {
        let plain = encode_footer_body(recs);
        let checksum = compute_payload_crc(&plain);
        out.extend_from_slice(&encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            seq,
            0,
            recs.len() as u32,
            plain.len() as u32,
            &plain,
            checksum,
        ));
    }

    Ok(out)
}

/// Partition `sources` into slices of at most `max_records_per_chunk` records.
///
/// Same rules as [`partition_event_records`]:
/// - `max_records_per_chunk == 0` → one partition of all records (or empty list)
/// - `max_records_per_chunk >= 1` → consecutive windows (last may be shorter)
pub fn partition_source_records<'a>(
    sources: &'a [SourceRecordSpec<'a>],
    max_records_per_chunk: usize,
) -> Vec<&'a [SourceRecordSpec<'a>]> {
    if sources.is_empty() {
        return Vec::new();
    }
    if max_records_per_chunk == 0 {
        return vec![sources];
    }
    sources.chunks(max_records_per_chunk).collect()
}

/// Encode a mixed profile where **EVENT** may span **≥2** compressed chunks.
///
/// - EVENT records are split via shipped [`partition_event_records`] with
///   `max_records_per_chunk` (use `1` to force ≥2 chunks when `events.len() ≥ 2`).
/// - Each EVENT partition is sealed with `codecs.event`.
/// - SOURCE / INDEX / SUMMARY remain **single** chunks (their per-kind codecs).
/// - FOOTER remains codec NONE, last when present.
///
/// For multi-chunk SOURCE, use [`encode_multi_chunk_source_mixed_profile`].
/// Decode with [`decode_compressed_mixed_profile`] (ordered EVENT reassembly).
pub fn encode_multi_chunk_kind_mixed_profile(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    codecs: KindCodecs,
    events: &[EventRecordSpec<'_>],
    max_records_per_chunk: usize,
    sources: &[SourceRecordSpec<'_>],
    indexes: &[IndexRecordSpec<'_>],
    summaries: &[SummaryRecordSpec<'_>],
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> CompressedMixedResult<Vec<u8>> {
    // Single-chunk SOURCE (max 0 = unlimited partition of whole list).
    encode_multi_chunk_source_mixed_profile(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        codecs,
        events,
        max_records_per_chunk,
        sources,
        0,
        indexes,
        summaries,
        footer,
    )
}

/// Partition `indexes` into slices of at most `max_records_per_chunk` records.
///
/// Same rules as [`partition_event_records`] / [`partition_source_records`].
pub fn partition_index_records<'a>(
    indexes: &'a [IndexRecordSpec<'a>],
    max_records_per_chunk: usize,
) -> Vec<&'a [IndexRecordSpec<'a>]> {
    if indexes.is_empty() {
        return Vec::new();
    }
    if max_records_per_chunk == 0 {
        return vec![indexes];
    }
    indexes.chunks(max_records_per_chunk).collect()
}

/// Encode a mixed profile where **SOURCE** (and optionally **EVENT**) may span
/// **≥2** compressed chunks.
///
/// - EVENT: [`partition_event_records`] with `max_event_records_per_chunk`
/// - SOURCE: [`partition_source_records`] with `max_source_records_per_chunk`
///   (use `1` to force ≥2 SOURCE chunks when `sources.len() ≥ 2`)
/// - INDEX / SUMMARY: single chunks
/// - FOOTER: codec NONE, last
///
/// For multi-chunk INDEX, use [`encode_multi_chunk_index_mixed_profile`].
/// Decode with [`decode_compressed_mixed_profile`] (ordered multi-SOURCE reassembly).
pub fn encode_multi_chunk_source_mixed_profile(
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
    summaries: &[SummaryRecordSpec<'_>],
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> CompressedMixedResult<Vec<u8>> {
    // Single-chunk INDEX (max 0 = unlimited partition of whole list).
    encode_multi_chunk_index_mixed_profile(
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
        0,
        summaries,
        footer,
    )
}

/// Partition `summaries` into slices of at most `max_records_per_chunk` records.
///
/// Same rules as [`partition_event_records`] / [`partition_index_records`].
pub fn partition_summary_records<'a>(
    summaries: &'a [SummaryRecordSpec<'a>],
    max_records_per_chunk: usize,
) -> Vec<&'a [SummaryRecordSpec<'a>]> {
    if summaries.is_empty() {
        return Vec::new();
    }
    if max_records_per_chunk == 0 {
        return vec![summaries];
    }
    summaries.chunks(max_records_per_chunk).collect()
}

/// Encode a mixed profile where **INDEX** (and optionally EVENT/SOURCE) may span
/// **≥2** compressed chunks.
///
/// - EVENT / SOURCE / INDEX: partition helpers with their max_records_per_chunk
///   (use `1` to force ≥2 INDEX chunks when `indexes.len() ≥ 2`)
/// - SUMMARY: single chunk
/// - FOOTER: codec NONE, last
///
/// For multi-chunk SUMMARY, use [`encode_multi_chunk_summary_mixed_profile`].
/// Decode with [`decode_compressed_mixed_profile`] (ordered multi-INDEX reassembly).
pub fn encode_multi_chunk_index_mixed_profile(
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
    footer: Option<&[FooterRecordSpec<'_>]>,
) -> CompressedMixedResult<Vec<u8>> {
    // Single-chunk SUMMARY (max 0 = unlimited partition of whole list).
    encode_multi_chunk_summary_mixed_profile(
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
        0,
        footer,
    )
}

/// Encode a mixed profile where **SUMMARY** (and optionally EVENT/SOURCE/INDEX)
/// may span **≥2** compressed chunks.
///
/// - EVENT / SOURCE / INDEX / SUMMARY: partition helpers with their max_records
///   (use `1` to force ≥2 SUMMARY chunks when `summaries.len() ≥ 2`)
/// - FOOTER: codec NONE, last
///
/// Decode with [`decode_compressed_mixed_profile`] (ordered multi-SUMMARY reassembly).
pub fn encode_multi_chunk_summary_mixed_profile(
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
) -> CompressedMixedResult<Vec<u8>> {
    for (present, c) in [
        (!events.is_empty(), codecs.event),
        (!sources.is_empty(), codecs.source),
        (!indexes.is_empty(), codecs.index),
        (!summaries.is_empty(), codecs.summary),
    ] {
        if present && !is_supported_event_codec(c) {
            return Err(CompressedMixedError::UnsupportedCodec { codec: c });
        }
    }

    let event_parts = partition_event_records(events, max_event_records_per_chunk);
    let source_parts = partition_source_records(sources, max_source_records_per_chunk);
    let index_parts = partition_index_records(indexes, max_index_records_per_chunk);
    let summary_parts = partition_summary_records(summaries, max_summary_records_per_chunk);
    let mut out = encode_file_prefix(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
    );
    let mut seq = 0u64;

    for part in &event_parts {
        let plain = encode_event_body(part);
        let frame =
            encode_kind_chunk(kind::EVENT, codecs.event, seq, part.len() as u32, &plain)?;
        out.extend_from_slice(&frame);
        seq += 1;
    }
    for part in &source_parts {
        let plain = encode_source_body(part);
        let frame =
            encode_kind_chunk(kind::SOURCE, codecs.source, seq, part.len() as u32, &plain)?;
        out.extend_from_slice(&frame);
        seq += 1;
    }
    for part in &index_parts {
        let plain = encode_index_body(part);
        let frame =
            encode_kind_chunk(kind::INDEX, codecs.index, seq, part.len() as u32, &plain)?;
        out.extend_from_slice(&frame);
        seq += 1;
    }
    for part in &summary_parts {
        let plain = encode_summary_body(part);
        let frame = encode_kind_chunk(
            kind::SUMMARY,
            codecs.summary,
            seq,
            part.len() as u32,
            &plain,
        )?;
        out.extend_from_slice(&frame);
        seq += 1;
    }
    if let Some(recs) = footer {
        let plain = encode_footer_body(recs);
        let checksum = compute_payload_crc(&plain);
        out.extend_from_slice(&encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            seq,
            0,
            recs.len() as u32,
            plain.len() as u32,
            &plain,
            checksum,
        ));
    }

    Ok(out)
}

/// Decode a compressed multi-kind mixed profile (shared or per-kind codecs).
///
/// Non-inflating stream parse → per compressible frame `decode_chunk_payload`
/// (honors that frame's codec) → matching body decoder. Multiple EVENT, SOURCE,
/// INDEX, and SUMMARY chunks append records in order. FOOTER must be last and
/// codec NONE. Different codecs across kinds are allowed.
pub fn decode_compressed_mixed_profile(
    buf: &[u8],
) -> CompressedMixedResult<(CompressedMixedProfile, usize)> {
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
    let mut saw_footer = false;
    let mut kind_codecs = KindCodecs::uniform(codec::NONE);
    let mut first_codec = codec::NONE;
    let mut saw_compressible = false;

    for frame in &stream.chunks {
        if saw_footer {
            return Err(CompressedMixedError::InvalidFooter);
        }
        match frame.kind {
            k if k == kind::EVENT
                || k == kind::SOURCE
                || k == kind::INDEX
                || k == kind::SUMMARY =>
            {
                if !is_supported_event_codec(frame.codec) {
                    return Err(CompressedMixedError::UnsupportedCodec {
                        codec: frame.codec,
                    });
                }
                if !saw_compressible {
                    first_codec = frame.codec;
                    saw_compressible = true;
                }
                // Record observed codec for this kind (MVP: one chunk per kind).
                if k == kind::EVENT {
                    kind_codecs.event = frame.codec;
                } else if k == kind::SOURCE {
                    kind_codecs.source = frame.codec;
                } else if k == kind::INDEX {
                    kind_codecs.index = frame.codec;
                } else if k == kind::SUMMARY {
                    kind_codecs.summary = frame.codec;
                }
                let plain = decode_chunk_payload(frame)?;
                match k {
                    k if k == kind::EVENT => {
                        let (recs, body_n) = decode_event_body(&plain)?;
                        if body_n != plain.len() {
                            return Err(CompressedMixedError::EventBody(
                                EventBodyError::Truncated {
                                    need: plain.len(),
                                    got: body_n,
                                },
                            ));
                        }
                        for r in &recs {
                            event_records.push(OwnedEventRecord::from_borrowed(r));
                        }
                        event_chunk_count += 1;
                    }
                    k if k == kind::SOURCE => {
                        let (recs, body_n) = decode_source_body(&plain)?;
                        if body_n != plain.len() {
                            return Err(CompressedMixedError::SourceBody(
                                SourceBodyError::Truncated {
                                    need: plain.len(),
                                    got: body_n,
                                },
                            ));
                        }
                        for r in recs {
                            source_records.push(OwnedSourceRecord {
                                fid: r.fid,
                                line: r.line,
                                text: r.text.data.to_vec(),
                            });
                        }
                        source_chunk_count += 1;
                    }
                    k if k == kind::INDEX => {
                        let (recs, body_n) = decode_index_body(&plain)?;
                        if body_n != plain.len() {
                            return Err(CompressedMixedError::IndexBody(
                                IndexBodyError::Truncated {
                                    need: plain.len(),
                                    got: body_n,
                                },
                            ));
                        }
                        for r in recs {
                            index_records.push(OwnedIndexRecord {
                                key_id: r.key_id,
                                file_offset: r.file_offset,
                                length: r.length,
                                label: r.label.data.to_vec(),
                            });
                        }
                        index_chunk_count += 1;
                    }
                    k if k == kind::SUMMARY => {
                        let (recs, body_n) = decode_summary_body(&plain)?;
                        if body_n != plain.len() {
                            return Err(CompressedMixedError::SummaryBody(
                                SummaryBodyError::Truncated {
                                    need: plain.len(),
                                    got: body_n,
                                },
                            ));
                        }
                        for r in recs {
                            summary_records.push(OwnedSummaryRecord {
                                key_id: r.key_id,
                                count: r.count,
                                value: r.value,
                                label: r.label.data.to_vec(),
                            });
                        }
                        summary_chunk_count += 1;
                    }
                    _ => unreachable!(),
                }
            }
            k if k == kind::FOOTER => {
                if frame.codec != codec::NONE {
                    return Err(CompressedMixedError::UnexpectedFooterCodec {
                        codec: frame.codec,
                    });
                }
                let plain = frame.payload; // FOOTER is identity / NONE
                let (recs, body_n) = decode_footer_body(plain)?;
                if body_n != plain.len() {
                    return Err(CompressedMixedError::FooterBody(FooterBodyError::Truncated {
                        need: plain.len(),
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
                has_footer = true;
                saw_footer = true;
            }
            other => {
                return Err(CompressedMixedError::UnexpectedKind { kind: other });
            }
        }
    }

    Ok((
        CompressedMixedProfile {
            header: stream.prefix.header,
            payload_codec: first_codec,
            kind_codecs,
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
        },
        n,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{CHUNK_HEADER_LEN, CHUNK_SYNC};
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

    fn sample_sources() -> [SourceRecordSpec<'static>; 1] {
        [SourceRecordSpec {
            fid: 1,
            line: 5,
            string_id: 0,
            string_flags: 0,
            text: b"$x++",
        }]
    }

    fn sample_indexes() -> [IndexRecordSpec<'static>; 1] {
        [IndexRecordSpec {
            key_id: 1,
            file_offset: 100,
            length: 20,
            string_id: 0,
            string_flags: 0,
            label: b"fid1",
        }]
    }

    fn sample_summaries() -> [SummaryRecordSpec<'static>; 1] {
        [SummaryRecordSpec {
            key_id: 7,
            count: 15,
            value: 99,
            string_id: 0,
            string_flags: 0,
            label: b"leaf",
        }]
    }

    fn sample_footer() -> [FooterRecordSpec<'static>; 1] {
        [FooterRecordSpec {
            key_id: 1,
            value: 2474,
            string_id: 0,
            string_flags: 0,
            label: b"total_events",
        }]
    }

    fn assert_recovered(prof: &CompressedMixedProfile) {
        assert_eq!(prof.event_records.len(), 2);
        match &prof.event_records[0] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 5, 42));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(prof.source_records.len(), 1);
        assert_eq!(prof.source_records[0].text, b"$x++");
        assert_eq!(prof.index_records.len(), 1);
        assert_eq!(prof.index_records[0].label, b"fid1");
        assert_eq!(prof.summary_records.len(), 1);
        assert_eq!(prof.summary_records[0].count, 15);
        assert!(prof.has_footer);
        assert_eq!(prof.footer_records.len(), 1);
        assert_eq!(prof.footer_records[0].value, 2474);
    }

    #[test]
    fn roundtrip_none_zlib_zstd_lz4_event_source_index_summary() {
        let events = sample_events();
        let sources = sample_sources();
        let indexes = sample_indexes();
        let summaries = sample_summaries();
        let footer = sample_footer();

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let enc_a = encode_compressed_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                &sources,
                &indexes,
                &summaries,
                Some(&footer),
            )
            .expect("encode");
            let enc_b = encode_compressed_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                &sources,
                &indexes,
                &summaries,
                Some(&footer),
            )
            .expect("encode 2");
            assert_eq!(enc_a, enc_b, "deterministic codec {c}");
            assert_eq!(&enc_a[..8], MAGIC.as_slice());

            let (prof, n) = decode_compressed_mixed_profile(&enc_a).expect("decode");
            assert_eq!(n, enc_a.len());
            assert_eq!(prof.payload_codec, c);
            assert_eq!(prof.event_chunk_count, 1);
            assert_eq!(prof.source_chunk_count, 1);
            assert_eq!(prof.index_chunk_count, 1);
            assert_eq!(prof.summary_chunk_count, 1);
            assert_recovered(&prof);

            // Wire payloads stay compressed for EVENT/SOURCE when not NONE.
            if c != codec::NONE {
                let (stream, _) = decode_prefix_chunk_stream(&enc_a).unwrap();
                let event = stream
                    .chunks
                    .iter()
                    .find(|f| f.kind == kind::EVENT)
                    .unwrap();
                let source = stream
                    .chunks
                    .iter()
                    .find(|f| f.kind == kind::SOURCE)
                    .unwrap();
                let plain_e = encode_event_body(&events);
                let plain_s = encode_source_body(&sources);
                assert_ne!(event.payload, plain_e.as_slice());
                assert_ne!(source.payload, plain_s.as_slice());
                // FOOTER uncompressed
                let footer_f = stream
                    .chunks
                    .iter()
                    .find(|f| f.kind == kind::FOOTER)
                    .unwrap();
                assert_eq!(footer_f.codec, codec::NONE);
            }
        }
    }

    #[test]
    fn event_source_only_zstd() {
        let enc = encode_compressed_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZSTD,
            &sample_events(),
            &sample_sources(),
            &[],
            &[],
            None,
        )
        .unwrap();
        let (prof, n) = decode_compressed_mixed_profile(&enc).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(prof.payload_codec, codec::ZSTD);
        assert_eq!(prof.event_chunk_count, 1);
        assert_eq!(prof.source_chunk_count, 1);
        assert_eq!(prof.index_chunk_count, 0);
        assert!(!prof.has_footer);
        assert_eq!(prof.event_records.len(), 2);
        assert_eq!(prof.source_records[0].text, b"$x++");
    }

    #[test]
    fn corrupt_zlib_source_payload_err() {
        let mut enc = encode_compressed_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &sample_events(),
            &sample_sources(),
            &[],
            &[],
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::EVENT);
        let source_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let f1 = parse_chunk_frame(&enc[source_off..]).unwrap();
        assert_eq!(f1.kind, kind::SOURCE);
        assert_eq!(f1.codec, codec::ZLIB);
        let payload_len = f1.payload.len();
        let payload_off = source_off + CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        if payload_len > 1 {
            enc[payload_off + 1] ^= 0xAA;
        }
        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Payload(_)) => {}
            other => panic!("expected payload error, got {other:?}"),
        }
    }

    #[test]
    fn size_mismatch_zstd_event_err() {
        use crate::payload_codec::compress_zstd;
        let plain = encode_event_body(&sample_events());
        let compressed = compress_zstd(&plain).unwrap();
        let wrong = (plain.len() as u32) / 2;
        assert!(wrong > 0 && wrong != plain.len() as u32);
        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        enc.extend_from_slice(&encode_chunk_frame(
            kind::EVENT,
            codec::ZSTD,
            0,
            0,
            0,
            2,
            wrong,
            &compressed,
            compute_payload_crc(&compressed),
        ));
        // Valid SOURCE after so stream has mixed shape if event somehow passed.
        let src = encode_source_body(&sample_sources());
        enc.extend_from_slice(
            &encode_kind_chunk(kind::SOURCE, codec::ZSTD, 1, 1, &src).unwrap(),
        );
        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Payload(
                PayloadCodecError::SizeMismatch { .. } | PayloadCodecError::Zstd { .. },
            )) => {}
            other => panic!("expected size/zstd error, got {other:?}"),
        }
    }

    #[test]
    fn zlib_source_wire_matches_deflate() {
        let sources = sample_sources();
        let plain = encode_source_body(&sources);
        let expected = deflate_zlib(&plain).unwrap();
        let enc = encode_compressed_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &sample_events(),
            &sources,
            &[],
            &[],
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        let source_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let f1 = parse_chunk_frame(&enc[source_off..]).unwrap();
        assert_eq!(f1.kind, kind::SOURCE);
        assert_eq!(f1.payload, expected.as_slice());
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_compressed_mixed_profile(&[]).is_err());
        assert!(decode_compressed_mixed_profile(b"nope").is_err());
        let mut enc = encode_compressed_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &[],
            &[],
            &[],
            &[],
            None,
        )
        .unwrap();
        let mut bad = vec![0u8; CHUNK_HEADER_LEN];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        enc.extend_from_slice(&bad);
        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Stream(StreamError::Chunk(ChunkError::BadSync {
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
    fn empty_prefix_only() {
        let enc = encode_compressed_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::LZ4,
            &[],
            &[],
            &[],
            &[],
            None,
        )
        .unwrap();
        assert_eq!(enc, encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]));
        let (prof, n) = decode_compressed_mixed_profile(&enc).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(prof.payload_codec, codec::NONE);
        assert_eq!(prof.event_chunk_count, 0);
    }

    #[test]
    fn per_kind_zstd_event_lz4_source_roundtrip() {
        let events = sample_events();
        let sources = sample_sources();
        let codecs = KindCodecs {
            event: codec::ZSTD,
            source: codec::LZ4,
            index: codec::NONE,
            summary: codec::NONE,
        };
        assert_ne!(codecs.event, codecs.source);
        assert_ne!(codecs.event, codec::NONE);
        assert_ne!(codecs.source, codec::NONE);

        let enc_a = encode_compressed_mixed_profile_per_kind(
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
            None,
        )
        .expect("encode per-kind");
        let enc_b = encode_compressed_mixed_profile_per_kind(
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
            None,
        )
        .expect("encode 2");
        assert_eq!(enc_a, enc_b);
        assert_eq!(&enc_a[..8], MAGIC.as_slice());

        let (prof, n) = decode_compressed_mixed_profile(&enc_a).expect("decode");
        assert_eq!(n, enc_a.len());
        assert_eq!(prof.payload_codec, codec::ZSTD); // first compressible kind
        assert_eq!(prof.kind_codecs.event, codec::ZSTD);
        assert_eq!(prof.kind_codecs.source, codec::LZ4);
        assert_eq!(prof.event_chunk_count, 1);
        assert_eq!(prof.source_chunk_count, 1);
        assert_eq!(prof.event_records.len(), 2);
        assert_eq!(prof.source_records[0].text, b"$x++");

        // Wire: EVENT is zstd-compressed, SOURCE is lz4-compressed (not plain).
        let (stream, _) = decode_prefix_chunk_stream(&enc_a).unwrap();
        let event = stream
            .chunks
            .iter()
            .find(|f| f.kind == kind::EVENT)
            .unwrap();
        let source = stream
            .chunks
            .iter()
            .find(|f| f.kind == kind::SOURCE)
            .unwrap();
        assert_eq!(event.codec, codec::ZSTD);
        assert_eq!(source.codec, codec::LZ4);
        let plain_e = encode_event_body(&events);
        let plain_s = encode_source_body(&sources);
        assert_ne!(event.payload, plain_e.as_slice());
        assert_ne!(source.payload, plain_s.as_slice());
    }

    #[test]
    fn per_kind_zstd_event_zlib_source_index_summary() {
        let codecs = KindCodecs {
            event: codec::ZSTD,
            source: codec::ZLIB,
            index: codec::LZ4,
            summary: codec::NONE,
        };
        let enc = encode_compressed_mixed_profile_per_kind(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &sample_events(),
            &sample_sources(),
            &sample_indexes(),
            &sample_summaries(),
            Some(&sample_footer()),
        )
        .unwrap();
        let (prof, n) = decode_compressed_mixed_profile(&enc).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(prof.kind_codecs.event, codec::ZSTD);
        assert_eq!(prof.kind_codecs.source, codec::ZLIB);
        assert_eq!(prof.kind_codecs.index, codec::LZ4);
        assert_eq!(prof.kind_codecs.summary, codec::NONE);
        assert_recovered(&prof);
    }

    #[test]
    fn per_kind_corrupt_lz4_source_err() {
        let codecs = KindCodecs {
            event: codec::ZSTD,
            source: codec::LZ4,
            index: codec::NONE,
            summary: codec::NONE,
        };
        let mut enc = encode_compressed_mixed_profile_per_kind(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &sample_events(),
            &sample_sources(),
            &[],
            &[],
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        assert_eq!(f0.codec, codec::ZSTD);
        let source_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let f1 = parse_chunk_frame(&enc[source_off..]).unwrap();
        assert_eq!(f1.codec, codec::LZ4);
        let payload_len = f1.payload.len();
        let payload_off = source_off + CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        if payload_len > 1 {
            enc[payload_off + 1] ^= 0x55;
        }
        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Payload(_)) => {}
            other => panic!("expected payload error on corrupt LZ4 SOURCE, got {other:?}"),
        }
    }

    #[test]
    fn per_kind_size_mismatch_on_event_still_fail_closed() {
        use crate::payload_codec::compress_zstd;
        let plain = encode_event_body(&sample_events());
        let compressed = compress_zstd(&plain).unwrap();
        let wrong = plain.len() as u32 + 5;
        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        enc.extend_from_slice(&encode_chunk_frame(
            kind::EVENT,
            codec::ZSTD,
            0,
            0,
            0,
            2,
            wrong,
            &compressed,
            compute_payload_crc(&compressed),
        ));
        // Different codec on SOURCE — multi-codec stream.
        let src = encode_source_body(&sample_sources());
        enc.extend_from_slice(
            &encode_kind_chunk(kind::SOURCE, codec::LZ4, 1, 1, &src).unwrap(),
        );
        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Payload(
                PayloadCodecError::SizeMismatch { .. } | PayloadCodecError::Zstd { .. },
            )) => {}
            other => panic!("expected size/zstd error, got {other:?}"),
        }
    }

    #[test]
    fn shared_codec_api_is_uniform_kind_codecs() {
        let enc = encode_compressed_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::ZLIB,
            &sample_events(),
            &sample_sources(),
            &[],
            &[],
            None,
        )
        .unwrap();
        let via_per = encode_compressed_mixed_profile_per_kind(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs::uniform(codec::ZLIB),
            &sample_events(),
            &sample_sources(),
            &[],
            &[],
            None,
        )
        .unwrap();
        assert_eq!(enc, via_per);
        let (prof, _) = decode_compressed_mixed_profile(&enc).unwrap();
        assert_eq!(prof.kind_codecs.event, codec::ZLIB);
        assert_eq!(prof.kind_codecs.source, codec::ZLIB);
    }

    fn multi_events() -> [EventRecordSpec<'static>; 4] {
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

    fn assert_ordered_four_events(recs: &[OwnedEventRecord]) {
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
    fn multi_chunk_event_zstd_plus_lz4_source_ordered() {
        let events = multi_events();
        let sources = sample_sources();
        // max_records_per_chunk=1 → 4 EVENT chunks (≥2).
        assert_eq!(partition_event_records(&events, 1).len(), 4);
        let codecs = KindCodecs {
            event: codec::ZSTD,
            source: codec::LZ4,
            index: codec::NONE,
            summary: codec::NONE,
        };
        assert_ne!(codecs.event, codecs.source);

        let enc_a = encode_multi_chunk_kind_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &events,
            1,
            &sources,
            &[],
            &[],
            Some(&sample_footer()),
        )
        .expect("encode multi-chunk mixed");
        let enc_b = encode_multi_chunk_kind_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &events,
            1,
            &sources,
            &[],
            &[],
            Some(&sample_footer()),
        )
        .expect("encode 2");
        assert_eq!(enc_a, enc_b);
        assert_eq!(&enc_a[..8], MAGIC.as_slice());

        let (prof, n) = decode_compressed_mixed_profile(&enc_a).expect("decode");
        assert_eq!(n, enc_a.len());
        assert_eq!(prof.event_chunk_count, 4);
        assert_eq!(prof.source_chunk_count, 1);
        assert_eq!(prof.kind_codecs.event, codec::ZSTD);
        assert_eq!(prof.kind_codecs.source, codec::LZ4);
        assert_ordered_four_events(&prof.event_records);
        assert_eq!(prof.source_records[0].text, b"$x++");
        assert!(prof.has_footer);

        // All EVENT frames compressed; SOURCE different codec.
        let (stream, _) = decode_prefix_chunk_stream(&enc_a).unwrap();
        let event_frames: Vec<_> = stream
            .chunks
            .iter()
            .filter(|f| f.kind == kind::EVENT)
            .collect();
        assert_eq!(event_frames.len(), 4);
        for (i, f) in event_frames.iter().enumerate() {
            assert_eq!(f.codec, codec::ZSTD);
            let part = partition_event_records(&events, 1)[i];
            let plain = encode_event_body(part);
            assert_ne!(f.payload, plain.as_slice());
        }
        let source = stream
            .chunks
            .iter()
            .find(|f| f.kind == kind::SOURCE)
            .unwrap();
        assert_eq!(source.codec, codec::LZ4);
        assert_ne!(source.payload, encode_source_body(&sources).as_slice());
    }

    #[test]
    fn multi_chunk_event_none_and_zlib_with_source() {
        let events = multi_events();
        for event_codec in [codec::NONE, codec::ZLIB, codec::LZ4] {
            let codecs = KindCodecs {
                event: event_codec,
                source: codec::ZSTD,
                index: codec::NONE,
                summary: codec::NONE,
            };
            let enc = encode_multi_chunk_kind_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                codecs,
                &events,
                2, // 2 EVENT chunks
                &sample_sources(),
                &[],
                &[],
                None,
            )
            .unwrap();
            let (prof, n) = decode_compressed_mixed_profile(&enc).unwrap();
            assert_eq!(n, enc.len(), "codec {event_codec}");
            assert_eq!(prof.event_chunk_count, 2);
            assert_eq!(prof.source_chunk_count, 1);
            assert_eq!(prof.kind_codecs.event, event_codec);
            assert_eq!(prof.kind_codecs.source, codec::ZSTD);
            assert_ordered_four_events(&prof.event_records);
        }
    }

    #[test]
    fn multi_chunk_uses_shipped_partition_count() {
        let events = multi_events();
        let parts = partition_event_records(&events, 1);
        let enc = encode_multi_chunk_kind_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::LZ4,
                source: codec::NONE,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &events,
            1,
            &sample_sources(),
            &[],
            &[],
            None,
        )
        .unwrap();
        let (prof, _) = decode_compressed_mixed_profile(&enc).unwrap();
        assert_eq!(prof.event_chunk_count, parts.len());
    }

    #[test]
    fn multi_chunk_corrupt_second_event_zlib_err() {
        let events = multi_events();
        let mut enc = encode_multi_chunk_kind_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::ZLIB,
                source: codec::LZ4,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &events,
            2, // 2 EVENT chunks
            &sample_sources(),
            &[],
            &[],
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::EVENT);
        let second_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let f1 = parse_chunk_frame(&enc[second_off..]).unwrap();
        assert_eq!(f1.kind, kind::EVENT);
        assert_eq!(f1.codec, codec::ZLIB);
        let payload_len = f1.payload.len();
        let payload_off = second_off + CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        if payload_len > 1 {
            enc[payload_off + 1] ^= 0xAA;
        }
        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Payload(_)) => {}
            other => panic!("expected payload error on corrupt 2nd EVENT, got {other:?}"),
        }
    }

    #[test]
    fn multi_chunk_size_mismatch_first_event_err() {
        use crate::payload_codec::compress_zstd;
        let events = multi_events();
        let parts = partition_event_records(&events, 2);
        assert_eq!(parts.len(), 2);
        let plain0 = encode_event_body(parts[0]);
        let compressed = compress_zstd(&plain0).unwrap();
        let wrong = (plain0.len() as u32).saturating_sub(1).max(1);
        assert_ne!(wrong, plain0.len() as u32);

        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        enc.extend_from_slice(&encode_chunk_frame(
            kind::EVENT,
            codec::ZSTD,
            0,
            0,
            0,
            parts[0].len() as u32,
            wrong,
            &compressed,
            compute_payload_crc(&compressed),
        ));
        // Valid second EVENT + SOURCE so stream is multi-chunk mixed shape.
        let plain1 = encode_event_body(parts[1]);
        enc.extend_from_slice(
            &encode_kind_chunk(kind::EVENT, codec::ZSTD, 1, parts[1].len() as u32, &plain1)
                .unwrap(),
        );
        let src = encode_source_body(&sample_sources());
        enc.extend_from_slice(&encode_kind_chunk(kind::SOURCE, codec::LZ4, 2, 1, &src).unwrap());

        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Payload(
                PayloadCodecError::SizeMismatch { .. } | PayloadCodecError::Zstd { .. },
            )) => {}
            other => panic!("expected size/zstd error, got {other:?}"),
        }
    }

    fn multi_sources() -> [SourceRecordSpec<'static>; 4] {
        [
            SourceRecordSpec {
                fid: 1,
                line: 1,
                string_id: 0,
                string_flags: 0,
                text: b"line1",
            },
            SourceRecordSpec {
                fid: 1,
                line: 2,
                string_id: 0,
                string_flags: 0,
                text: b"line2",
            },
            SourceRecordSpec {
                fid: 1,
                line: 3,
                string_id: 0,
                string_flags: 0,
                text: b"line3",
            },
            SourceRecordSpec {
                fid: 1,
                line: 4,
                string_id: 0,
                string_flags: 0,
                text: b"line4",
            },
        ]
    }

    fn assert_ordered_four_sources(recs: &[OwnedSourceRecord]) {
        assert_eq!(recs.len(), 4);
        assert_eq!(recs[0].text, b"line1");
        assert_eq!(recs[1].text, b"line2");
        assert_eq!(recs[2].text, b"line3");
        assert_eq!(recs[3].text, b"line4");
        assert_eq!(recs[0].line, 1);
        assert_eq!(recs[3].line, 4);
    }

    #[test]
    fn partition_source_records_rules() {
        let sources = multi_sources();
        assert!(partition_source_records(&[], 2).is_empty());
        let p0 = partition_source_records(&sources, 0);
        assert_eq!(p0.len(), 1);
        assert_eq!(p0[0].len(), 4);
        let p1 = partition_source_records(&sources, 1);
        assert_eq!(p1.len(), 4);
        let p2 = partition_source_records(&sources, 2);
        assert_eq!(p2.len(), 2);
        assert_eq!(p2[0].len(), 2);
        assert_eq!(p2[1].len(), 2);
    }

    #[test]
    fn multi_chunk_source_lz4_plus_zstd_event_ordered() {
        let events = sample_events();
        let sources = multi_sources();
        assert_eq!(partition_source_records(&sources, 1).len(), 4);
        let codecs = KindCodecs {
            event: codec::ZSTD,
            source: codec::LZ4,
            index: codec::NONE,
            summary: codec::NONE,
        };
        assert_ne!(codecs.event, codecs.source);

        let enc_a = encode_multi_chunk_source_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &events,
            0, // single EVENT
            &sources,
            1, // ≥2 SOURCE chunks
            &[],
            &[],
            Some(&sample_footer()),
        )
        .expect("encode multi-chunk SOURCE");
        let enc_b = encode_multi_chunk_source_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &events,
            0,
            &sources,
            1,
            &[],
            &[],
            Some(&sample_footer()),
        )
        .expect("encode 2");
        assert_eq!(enc_a, enc_b);
        assert_eq!(&enc_a[..8], MAGIC.as_slice());

        let (prof, n) = decode_compressed_mixed_profile(&enc_a).expect("decode");
        assert_eq!(n, enc_a.len());
        assert_eq!(prof.event_chunk_count, 1);
        assert_eq!(prof.source_chunk_count, 4);
        assert_eq!(prof.kind_codecs.event, codec::ZSTD);
        assert_eq!(prof.kind_codecs.source, codec::LZ4);
        assert_eq!(prof.event_records.len(), 2);
        assert_ordered_four_sources(&prof.source_records);
        assert!(prof.has_footer);

        let (stream, _) = decode_prefix_chunk_stream(&enc_a).unwrap();
        let source_frames: Vec<_> = stream
            .chunks
            .iter()
            .filter(|f| f.kind == kind::SOURCE)
            .collect();
        assert_eq!(source_frames.len(), 4);
        for (i, f) in source_frames.iter().enumerate() {
            assert_eq!(f.codec, codec::LZ4);
            let part = partition_source_records(&sources, 1)[i];
            let plain = encode_source_body(part);
            assert_ne!(f.payload, plain.as_slice());
        }
    }

    #[test]
    fn multi_chunk_source_none_zlib_zstd_with_event() {
        let sources = multi_sources();
        for source_codec in [codec::NONE, codec::ZLIB, codec::ZSTD] {
            let codecs = KindCodecs {
                event: codec::LZ4,
                source: source_codec,
                index: codec::NONE,
                summary: codec::NONE,
            };
            let enc = encode_multi_chunk_source_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                codecs,
                &sample_events(),
                0,
                &sources,
                2, // 2 SOURCE chunks
                &[],
                &[],
                None,
            )
            .unwrap();
            let (prof, n) = decode_compressed_mixed_profile(&enc).unwrap();
            assert_eq!(n, enc.len(), "codec {source_codec}");
            assert_eq!(prof.source_chunk_count, 2);
            assert_eq!(prof.event_chunk_count, 1);
            assert_eq!(prof.kind_codecs.source, source_codec);
            assert_ordered_four_sources(&prof.source_records);
        }
    }

    #[test]
    fn multi_chunk_source_and_event_both_split() {
        let events = multi_events();
        let sources = multi_sources();
        let codecs = KindCodecs {
            event: codec::ZLIB,
            source: codec::ZSTD,
            index: codec::NONE,
            summary: codec::NONE,
        };
        let enc = encode_multi_chunk_source_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &events,
            2, // 2 EVENT
            &sources,
            2, // 2 SOURCE
            &[],
            &[],
            None,
        )
        .unwrap();
        let (prof, n) = decode_compressed_mixed_profile(&enc).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(prof.event_chunk_count, 2);
        assert_eq!(prof.source_chunk_count, 2);
        assert_ordered_four_events(&prof.event_records);
        assert_ordered_four_sources(&prof.source_records);
    }

    #[test]
    fn multi_chunk_source_uses_shipped_partition_count() {
        let sources = multi_sources();
        let parts = partition_source_records(&sources, 1);
        let enc = encode_multi_chunk_source_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::LZ4,
                index: codec::NONE,
                summary: codec::NONE,
            },
            &sample_events(),
            0,
            &sources,
            1,
            &[],
            &[],
            None,
        )
        .unwrap();
        let (prof, _) = decode_compressed_mixed_profile(&enc).unwrap();
        assert_eq!(prof.source_chunk_count, parts.len());
    }

    #[test]
    fn multi_chunk_source_corrupt_second_lz4_err() {
        let sources = multi_sources();
        let mut enc = encode_multi_chunk_source_mixed_profile(
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
            &sample_events(),
            0,
            &sources,
            2, // 2 SOURCE
            &[],
            &[],
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        // Skip EVENT then first SOURCE.
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::EVENT);
        let s0_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let s0 = parse_chunk_frame(&enc[s0_off..]).unwrap();
        assert_eq!(s0.kind, kind::SOURCE);
        let s1_off = s0_off + CHUNK_HEADER_LEN + s0.payload.len();
        let s1 = parse_chunk_frame(&enc[s1_off..]).unwrap();
        assert_eq!(s1.kind, kind::SOURCE);
        assert_eq!(s1.codec, codec::LZ4);
        let payload_len = s1.payload.len();
        let payload_off = s1_off + CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        if payload_len > 1 {
            enc[payload_off + 1] ^= 0xAA;
        }
        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Payload(_)) => {}
            other => panic!("expected payload error on corrupt 2nd SOURCE, got {other:?}"),
        }
    }

    #[test]
    fn multi_chunk_source_size_mismatch_first_err() {
        use crate::payload_codec::compress_zstd;
        let sources = multi_sources();
        let parts = partition_source_records(&sources, 2);
        assert_eq!(parts.len(), 2);
        let plain0 = encode_source_body(parts[0]);
        let compressed = compress_zstd(&plain0).unwrap();
        let wrong = plain0.len() as u32 + 3;

        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        // EVENT first (valid).
        let ev = encode_event_body(&sample_events());
        enc.extend_from_slice(
            &encode_kind_chunk(kind::EVENT, codec::NONE, 0, 2, &ev).unwrap(),
        );
        enc.extend_from_slice(&encode_chunk_frame(
            kind::SOURCE,
            codec::ZSTD,
            0,
            1,
            0,
            parts[0].len() as u32,
            wrong,
            &compressed,
            compute_payload_crc(&compressed),
        ));
        let plain1 = encode_source_body(parts[1]);
        enc.extend_from_slice(
            &encode_kind_chunk(kind::SOURCE, codec::ZSTD, 2, parts[1].len() as u32, &plain1)
                .unwrap(),
        );

        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Payload(
                PayloadCodecError::SizeMismatch { .. } | PayloadCodecError::Zstd { .. },
            )) => {}
            other => panic!("expected size/zstd error on multi SOURCE, got {other:?}"),
        }
    }

    #[test]
    fn multi_chunk_kind_api_is_source_unlimited_wrapper() {
        let codecs = KindCodecs {
            event: codec::ZSTD,
            source: codec::LZ4,
            index: codec::NONE,
            summary: codec::NONE,
        };
        let via_kind = encode_multi_chunk_kind_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &sample_events(),
            1,
            &multi_sources(),
            &[],
            &[],
            None,
        )
        .unwrap();
        let via_src = encode_multi_chunk_source_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &sample_events(),
            1,
            &multi_sources(),
            0, // unlimited SOURCE → single chunk
            &[],
            &[],
            None,
        )
        .unwrap();
        assert_eq!(via_kind, via_src);
    }

    fn multi_indexes() -> [IndexRecordSpec<'static>; 4] {
        [
            IndexRecordSpec {
                key_id: 1,
                file_offset: 10,
                length: 4,
                string_id: 0,
                string_flags: 0,
                label: b"i1",
            },
            IndexRecordSpec {
                key_id: 2,
                file_offset: 20,
                length: 4,
                string_id: 0,
                string_flags: 0,
                label: b"i2",
            },
            IndexRecordSpec {
                key_id: 3,
                file_offset: 30,
                length: 4,
                string_id: 0,
                string_flags: 0,
                label: b"i3",
            },
            IndexRecordSpec {
                key_id: 4,
                file_offset: 40,
                length: 4,
                string_id: 0,
                string_flags: 0,
                label: b"i4",
            },
        ]
    }

    fn assert_ordered_four_indexes(recs: &[OwnedIndexRecord]) {
        assert_eq!(recs.len(), 4);
        assert_eq!(recs[0].label, b"i1");
        assert_eq!(recs[1].label, b"i2");
        assert_eq!(recs[2].label, b"i3");
        assert_eq!(recs[3].label, b"i4");
        assert_eq!(recs[0].key_id, 1);
        assert_eq!(recs[3].key_id, 4);
    }

    #[test]
    fn partition_index_records_rules() {
        let indexes = multi_indexes();
        assert!(partition_index_records(&[], 2).is_empty());
        let p0 = partition_index_records(&indexes, 0);
        assert_eq!(p0.len(), 1);
        assert_eq!(p0[0].len(), 4);
        let p1 = partition_index_records(&indexes, 1);
        assert_eq!(p1.len(), 4);
        let p2 = partition_index_records(&indexes, 2);
        assert_eq!(p2.len(), 2);
        assert_eq!(p2[0].len(), 2);
    }

    #[test]
    fn multi_chunk_index_zstd_plus_event_ordered() {
        let events = sample_events();
        let indexes = multi_indexes();
        assert_eq!(partition_index_records(&indexes, 1).len(), 4);
        let codecs = KindCodecs {
            event: codec::LZ4,
            source: codec::NONE,
            index: codec::ZSTD,
            summary: codec::NONE,
        };
        assert_ne!(codecs.event, codecs.index);

        let enc_a = encode_multi_chunk_index_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &events,
            0,
            &[],
            0,
            &indexes,
            1, // ≥2 INDEX
            &[],
            Some(&sample_footer()),
        )
        .expect("encode multi-chunk INDEX");
        let enc_b = encode_multi_chunk_index_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &events,
            0,
            &[],
            0,
            &indexes,
            1,
            &[],
            Some(&sample_footer()),
        )
        .expect("encode 2");
        assert_eq!(enc_a, enc_b);
        assert_eq!(&enc_a[..8], MAGIC.as_slice());

        let (prof, n) = decode_compressed_mixed_profile(&enc_a).expect("decode");
        assert_eq!(n, enc_a.len());
        assert_eq!(prof.event_chunk_count, 1);
        assert_eq!(prof.index_chunk_count, 4);
        assert_eq!(prof.kind_codecs.event, codec::LZ4);
        assert_eq!(prof.kind_codecs.index, codec::ZSTD);
        assert_eq!(prof.event_records.len(), 2);
        assert_ordered_four_indexes(&prof.index_records);
        assert!(prof.has_footer);

        let (stream, _) = decode_prefix_chunk_stream(&enc_a).unwrap();
        let index_frames: Vec<_> = stream
            .chunks
            .iter()
            .filter(|f| f.kind == kind::INDEX)
            .collect();
        assert_eq!(index_frames.len(), 4);
        for (i, f) in index_frames.iter().enumerate() {
            assert_eq!(f.codec, codec::ZSTD);
            let part = partition_index_records(&indexes, 1)[i];
            let plain = encode_index_body(part);
            assert_ne!(f.payload, plain.as_slice());
        }
    }

    #[test]
    fn multi_chunk_index_none_zlib_lz4_with_event() {
        let indexes = multi_indexes();
        for index_codec in [codec::NONE, codec::ZLIB, codec::LZ4] {
            let codecs = KindCodecs {
                event: codec::ZSTD,
                source: codec::NONE,
                index: index_codec,
                summary: codec::NONE,
            };
            let enc = encode_multi_chunk_index_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                codecs,
                &sample_events(),
                0,
                &[],
                0,
                &indexes,
                2, // 2 INDEX chunks
                &[],
                None,
            )
            .unwrap();
            let (prof, n) = decode_compressed_mixed_profile(&enc).unwrap();
            assert_eq!(n, enc.len(), "codec {index_codec}");
            assert_eq!(prof.index_chunk_count, 2);
            assert_eq!(prof.event_chunk_count, 1);
            assert_eq!(prof.kind_codecs.index, index_codec);
            assert_ordered_four_indexes(&prof.index_records);
        }
    }

    #[test]
    fn multi_chunk_index_with_source_and_event_split() {
        let codecs = KindCodecs {
            event: codec::ZLIB,
            source: codec::LZ4,
            index: codec::ZSTD,
            summary: codec::NONE,
        };
        let enc = encode_multi_chunk_index_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &multi_events(),
            2, // 2 EVENT
            &multi_sources(),
            2, // 2 SOURCE
            &multi_indexes(),
            2, // 2 INDEX
            &[],
            None,
        )
        .unwrap();
        let (prof, n) = decode_compressed_mixed_profile(&enc).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(prof.event_chunk_count, 2);
        assert_eq!(prof.source_chunk_count, 2);
        assert_eq!(prof.index_chunk_count, 2);
        assert_ordered_four_events(&prof.event_records);
        assert_ordered_four_sources(&prof.source_records);
        assert_ordered_four_indexes(&prof.index_records);
    }

    #[test]
    fn multi_chunk_index_uses_shipped_partition_count() {
        let indexes = multi_indexes();
        let parts = partition_index_records(&indexes, 1);
        let enc = encode_multi_chunk_index_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::LZ4,
                summary: codec::NONE,
            },
            &sample_events(),
            0,
            &[],
            0,
            &indexes,
            1,
            &[],
            None,
        )
        .unwrap();
        let (prof, _) = decode_compressed_mixed_profile(&enc).unwrap();
        assert_eq!(prof.index_chunk_count, parts.len());
    }

    #[test]
    fn multi_chunk_index_corrupt_second_zstd_err() {
        let indexes = multi_indexes();
        let mut enc = encode_multi_chunk_index_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            KindCodecs {
                event: codec::NONE,
                source: codec::NONE,
                index: codec::ZSTD,
                summary: codec::NONE,
            },
            &sample_events(),
            0,
            &[],
            0,
            &indexes,
            2, // 2 INDEX
            &[],
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::EVENT);
        let i0_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let i0 = parse_chunk_frame(&enc[i0_off..]).unwrap();
        assert_eq!(i0.kind, kind::INDEX);
        let i1_off = i0_off + CHUNK_HEADER_LEN + i0.payload.len();
        let i1 = parse_chunk_frame(&enc[i1_off..]).unwrap();
        assert_eq!(i1.kind, kind::INDEX);
        assert_eq!(i1.codec, codec::ZSTD);
        let payload_len = i1.payload.len();
        let payload_off = i1_off + CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        if payload_len > 1 {
            enc[payload_off + 1] ^= 0xAA;
        }
        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Payload(_)) => {}
            other => panic!("expected payload error on corrupt 2nd INDEX, got {other:?}"),
        }
    }

    #[test]
    fn multi_chunk_index_size_mismatch_first_err() {
        use crate::payload_codec::compress_zstd;
        let indexes = multi_indexes();
        let parts = partition_index_records(&indexes, 2);
        assert_eq!(parts.len(), 2);
        let plain0 = encode_index_body(parts[0]);
        let compressed = compress_zstd(&plain0).unwrap();
        let wrong = plain0.len() as u32 + 3;

        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let ev = encode_event_body(&sample_events());
        enc.extend_from_slice(
            &encode_kind_chunk(kind::EVENT, codec::NONE, 0, 2, &ev).unwrap(),
        );
        enc.extend_from_slice(&encode_chunk_frame(
            kind::INDEX,
            codec::ZSTD,
            0,
            1,
            0,
            parts[0].len() as u32,
            wrong,
            &compressed,
            compute_payload_crc(&compressed),
        ));
        let plain1 = encode_index_body(parts[1]);
        enc.extend_from_slice(
            &encode_kind_chunk(kind::INDEX, codec::ZSTD, 2, parts[1].len() as u32, &plain1)
                .unwrap(),
        );

        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Payload(
                PayloadCodecError::SizeMismatch { .. } | PayloadCodecError::Zstd { .. },
            )) => {}
            other => panic!("expected size/zstd error on multi INDEX, got {other:?}"),
        }
    }

    #[test]
    fn multi_chunk_source_api_is_index_unlimited_wrapper() {
        let codecs = KindCodecs {
            event: codec::ZSTD,
            source: codec::LZ4,
            index: codec::ZLIB,
            summary: codec::NONE,
        };
        let via_src = encode_multi_chunk_source_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &sample_events(),
            0,
            &multi_sources(),
            1,
            &multi_indexes(),
            &[],
            None,
        )
        .unwrap();
        let via_idx = encode_multi_chunk_index_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &sample_events(),
            0,
            &multi_sources(),
            1,
            &multi_indexes(),
            0, // unlimited INDEX → single chunk
            &[],
            None,
        )
        .unwrap();
        assert_eq!(via_src, via_idx);
    }

    fn multi_summaries() -> [SummaryRecordSpec<'static>; 4] {
        [
            SummaryRecordSpec {
                key_id: 1,
                count: 10,
                value: 100,
                string_id: 0,
                string_flags: 0,
                label: b"s1",
            },
            SummaryRecordSpec {
                key_id: 2,
                count: 20,
                value: 200,
                string_id: 0,
                string_flags: 0,
                label: b"s2",
            },
            SummaryRecordSpec {
                key_id: 3,
                count: 30,
                value: 300,
                string_id: 0,
                string_flags: 0,
                label: b"s3",
            },
            SummaryRecordSpec {
                key_id: 4,
                count: 40,
                value: 400,
                string_id: 0,
                string_flags: 0,
                label: b"s4",
            },
        ]
    }

    fn assert_ordered_four_summaries(recs: &[OwnedSummaryRecord]) {
        assert_eq!(recs.len(), 4);
        assert_eq!(recs[0].label, b"s1");
        assert_eq!(recs[1].label, b"s2");
        assert_eq!(recs[2].label, b"s3");
        assert_eq!(recs[3].label, b"s4");
        assert_eq!(recs[0].key_id, 1);
        assert_eq!(recs[3].count, 40);
    }

    #[test]
    fn partition_summary_records_rules() {
        let summaries = multi_summaries();
        assert!(partition_summary_records(&[], 2).is_empty());
        let p0 = partition_summary_records(&summaries, 0);
        assert_eq!(p0.len(), 1);
        assert_eq!(p0[0].len(), 4);
        let p1 = partition_summary_records(&summaries, 1);
        assert_eq!(p1.len(), 4);
        let p2 = partition_summary_records(&summaries, 2);
        assert_eq!(p2.len(), 2);
        assert_eq!(p2[0].len(), 2);
    }

    #[test]
    fn multi_chunk_summary_zstd_plus_event_ordered() {
        let events = sample_events();
        let summaries = multi_summaries();
        assert_eq!(partition_summary_records(&summaries, 1).len(), 4);
        let codecs = KindCodecs {
            event: codec::LZ4,
            source: codec::NONE,
            index: codec::NONE,
            summary: codec::ZSTD,
        };
        assert_ne!(codecs.event, codecs.summary);

        let enc_a = encode_multi_chunk_summary_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &events,
            0,
            &[],
            0,
            &[],
            0,
            &summaries,
            1, // ≥2 SUMMARY
            Some(&sample_footer()),
        )
        .expect("encode multi-chunk SUMMARY");
        let enc_b = encode_multi_chunk_summary_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &events,
            0,
            &[],
            0,
            &[],
            0,
            &summaries,
            1,
            Some(&sample_footer()),
        )
        .expect("encode 2");
        assert_eq!(enc_a, enc_b);
        assert_eq!(&enc_a[..8], MAGIC.as_slice());

        let (prof, n) = decode_compressed_mixed_profile(&enc_a).expect("decode");
        assert_eq!(n, enc_a.len());
        assert_eq!(prof.event_chunk_count, 1);
        assert_eq!(prof.summary_chunk_count, 4);
        assert_eq!(prof.kind_codecs.event, codec::LZ4);
        assert_eq!(prof.kind_codecs.summary, codec::ZSTD);
        assert_eq!(prof.event_records.len(), 2);
        assert_ordered_four_summaries(&prof.summary_records);
        assert!(prof.has_footer);

        let (stream, _) = decode_prefix_chunk_stream(&enc_a).unwrap();
        let summary_frames: Vec<_> = stream
            .chunks
            .iter()
            .filter(|f| f.kind == kind::SUMMARY)
            .collect();
        assert_eq!(summary_frames.len(), 4);
        for (i, f) in summary_frames.iter().enumerate() {
            assert_eq!(f.codec, codec::ZSTD);
            let part = partition_summary_records(&summaries, 1)[i];
            let plain = encode_summary_body(part);
            assert_ne!(f.payload, plain.as_slice());
        }
    }

    #[test]
    fn multi_chunk_summary_none_zlib_lz4_with_event() {
        let summaries = multi_summaries();
        for summary_codec in [codec::NONE, codec::ZLIB, codec::LZ4] {
            let codecs = KindCodecs {
                event: codec::ZSTD,
                source: codec::NONE,
                index: codec::NONE,
                summary: summary_codec,
            };
            let enc = encode_multi_chunk_summary_mixed_profile(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                codecs,
                &sample_events(),
                0,
                &[],
                0,
                &[],
                0,
                &summaries,
                2, // 2 SUMMARY chunks
                None,
            )
            .unwrap();
            let (prof, n) = decode_compressed_mixed_profile(&enc).unwrap();
            assert_eq!(n, enc.len(), "codec {summary_codec}");
            assert_eq!(prof.summary_chunk_count, 2);
            assert_eq!(prof.event_chunk_count, 1);
            assert_eq!(prof.kind_codecs.summary, summary_codec);
            assert_ordered_four_summaries(&prof.summary_records);
        }
    }

    #[test]
    fn multi_chunk_summary_with_all_kinds_split() {
        let codecs = KindCodecs {
            event: codec::ZLIB,
            source: codec::LZ4,
            index: codec::ZSTD,
            summary: codec::NONE,
        };
        let enc = encode_multi_chunk_summary_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &multi_events(),
            2,
            &multi_sources(),
            2,
            &multi_indexes(),
            2,
            &multi_summaries(),
            2,
            None,
        )
        .unwrap();
        let (prof, n) = decode_compressed_mixed_profile(&enc).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(prof.event_chunk_count, 2);
        assert_eq!(prof.source_chunk_count, 2);
        assert_eq!(prof.index_chunk_count, 2);
        assert_eq!(prof.summary_chunk_count, 2);
        assert_ordered_four_events(&prof.event_records);
        assert_ordered_four_sources(&prof.source_records);
        assert_ordered_four_indexes(&prof.index_records);
        assert_ordered_four_summaries(&prof.summary_records);
    }

    #[test]
    fn multi_chunk_summary_uses_shipped_partition_count() {
        let summaries = multi_summaries();
        let parts = partition_summary_records(&summaries, 1);
        let enc = encode_multi_chunk_summary_mixed_profile(
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
                summary: codec::LZ4,
            },
            &sample_events(),
            0,
            &[],
            0,
            &[],
            0,
            &summaries,
            1,
            None,
        )
        .unwrap();
        let (prof, _) = decode_compressed_mixed_profile(&enc).unwrap();
        assert_eq!(prof.summary_chunk_count, parts.len());
    }

    #[test]
    fn multi_chunk_summary_corrupt_second_zstd_err() {
        let summaries = multi_summaries();
        let mut enc = encode_multi_chunk_summary_mixed_profile(
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
                summary: codec::ZSTD,
            },
            &sample_events(),
            0,
            &[],
            0,
            &[],
            0,
            &summaries,
            2, // 2 SUMMARY
            None,
        )
        .unwrap();
        let prefix_n = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]).len();
        let f0 = parse_chunk_frame(&enc[prefix_n..]).unwrap();
        assert_eq!(f0.kind, kind::EVENT);
        let s0_off = prefix_n + CHUNK_HEADER_LEN + f0.payload.len();
        let s0 = parse_chunk_frame(&enc[s0_off..]).unwrap();
        assert_eq!(s0.kind, kind::SUMMARY);
        let s1_off = s0_off + CHUNK_HEADER_LEN + s0.payload.len();
        let s1 = parse_chunk_frame(&enc[s1_off..]).unwrap();
        assert_eq!(s1.kind, kind::SUMMARY);
        assert_eq!(s1.codec, codec::ZSTD);
        let payload_len = s1.payload.len();
        let payload_off = s1_off + CHUNK_HEADER_LEN;
        enc[payload_off] ^= 0xFF;
        if payload_len > 1 {
            enc[payload_off + 1] ^= 0xAA;
        }
        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Payload(_)) => {}
            other => panic!("expected payload error on corrupt 2nd SUMMARY, got {other:?}"),
        }
    }

    #[test]
    fn multi_chunk_summary_size_mismatch_first_err() {
        use crate::payload_codec::compress_zstd;
        let summaries = multi_summaries();
        let parts = partition_summary_records(&summaries, 2);
        assert_eq!(parts.len(), 2);
        let plain0 = encode_summary_body(parts[0]);
        let compressed = compress_zstd(&plain0).unwrap();
        let wrong = plain0.len() as u32 + 3;

        let mut enc = encode_file_prefix(SUPPORTED_MAJOR, 0, 0, 0, 0, &[]);
        let ev = encode_event_body(&sample_events());
        enc.extend_from_slice(
            &encode_kind_chunk(kind::EVENT, codec::NONE, 0, 2, &ev).unwrap(),
        );
        enc.extend_from_slice(&encode_chunk_frame(
            kind::SUMMARY,
            codec::ZSTD,
            0,
            1,
            0,
            parts[0].len() as u32,
            wrong,
            &compressed,
            compute_payload_crc(&compressed),
        ));
        let plain1 = encode_summary_body(parts[1]);
        enc.extend_from_slice(
            &encode_kind_chunk(
                kind::SUMMARY,
                codec::ZSTD,
                2,
                parts[1].len() as u32,
                &plain1,
            )
            .unwrap(),
        );

        match decode_compressed_mixed_profile(&enc) {
            Err(CompressedMixedError::Payload(
                PayloadCodecError::SizeMismatch { .. } | PayloadCodecError::Zstd { .. },
            )) => {}
            other => panic!("expected size/zstd error on multi SUMMARY, got {other:?}"),
        }
    }

    #[test]
    fn multi_chunk_index_api_is_summary_unlimited_wrapper() {
        let codecs = KindCodecs {
            event: codec::ZSTD,
            source: codec::LZ4,
            index: codec::ZLIB,
            summary: codec::NONE,
        };
        let via_idx = encode_multi_chunk_index_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &sample_events(),
            0,
            &[],
            0,
            &multi_indexes(),
            1,
            &multi_summaries(),
            None,
        )
        .unwrap();
        let via_sum = encode_multi_chunk_summary_mixed_profile(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codecs,
            &sample_events(),
            0,
            &[],
            0,
            &multi_indexes(),
            1,
            &multi_summaries(),
            0, // unlimited SUMMARY → single chunk
            None,
        )
        .unwrap();
        assert_eq!(via_idx, via_sum);
    }
}

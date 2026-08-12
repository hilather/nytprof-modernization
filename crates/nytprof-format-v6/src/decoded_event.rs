//! Provisional **format v6** decoded EVENT profile consumer path (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-decoded-event-provisional-v0.md`
//!
//! Stream → always-inflate EVENT payloads (optional CRC) → join plain EVENT
//! bytes → `decode_event_body`. Composes shipped decoded-stream + event-body +
//! payload seal helpers. Does **not** change default `parse_chunk_frame`.
//! Not full opcode catalog freeze, not COL-007 C writer, not CLI v6 default.

use std::borrow::Cow;

use crate::chunk::{codec, encode_chunk_frame, kind};
use crate::compressed_profile::{
    encode_event_chunk, is_supported_event_codec, CompressedProfileError, OwnedEventRecord,
};
use crate::crc::compute_payload_crc;
use crate::decoded_stream::{
    decode_prefix_chunk_stream_plain, encode_prefix_sealed_chunks, DecodedStreamError,
};
use crate::event_body::{
    decode_event_body_full, encode_event_body,
    encode_event_body_with_site_deltas_and_seq_continuing, EventBodyError, EventRecordSpec,
    PackingEncodeState,
};
use crate::multi_chunk_event::partition_event_records;
use crate::string_dict::{
    decode_string_dictionary, encode_string_dictionary, resolve_event_records, StringDictError,
    StringDictionary,
};
use crate::FixedHeader;

/// Fail-closed decoded-EVENT profile errors.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodedEventError {
    Stream(DecodedStreamError),
    EventBody(EventBodyError),
    Encode(CompressedProfileError),
    StringDict(StringDictError),
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
    /// String-dictionary preflight expects FOOTER payload carrying the dictionary table.
    MissingStringDictionaryFooter,
}

impl std::fmt::Display for DecodedEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodedEventError::Stream(e) => write!(f, "decoded-event stream: {e}"),
            DecodedEventError::EventBody(e) => write!(f, "decoded-event body: {e}"),
            DecodedEventError::Encode(e) => write!(f, "decoded-event encode: {e}"),
            DecodedEventError::StringDict(e) => write!(f, "decoded-event string-dict: {e}"),
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
            DecodedEventError::MissingStringDictionaryFooter => {
                write!(f, "decoded-event missing string-dictionary FOOTER payload")
            }
        }
    }
}

impl std::error::Error for DecodedEventError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodedEventError::Stream(e) => Some(e),
            DecodedEventError::EventBody(e) => Some(e),
            DecodedEventError::Encode(e) => Some(e),
            DecodedEventError::StringDict(e) => Some(e),
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

impl From<StringDictError> for DecodedEventError {
    fn from(e: StringDictError) -> Self {
        DecodedEventError::StringDict(e)
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
    /// Parallel to [`Self::records`]: provisional logical sequence numbers when
    /// the event body used [`crate::event_body::FLAG_HAS_SEQ`]; otherwise all `None`.
    /// Length always equals `records.len()`. OI-001-03 runway only — not a freeze.
    pub sequences: Vec<Option<u64>>,
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

/// Validate header-matching VERSION records, or prepend one when body has none.
///
/// Shared by absolute auto-version and packing auto-version compose preflights.
fn events_with_auto_version<'a>(
    major: u16,
    minor: u16,
    events: &'a [EventRecordSpec<'a>],
) -> DecodedEventResult<Cow<'a, [EventRecordSpec<'a>]>> {
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
        return Ok(Cow::Borrowed(events));
    }
    let mut with_ver: Vec<EventRecordSpec<'a>> = Vec::with_capacity(events.len() + 1);
    with_ver.push(EventRecordSpec::Version {
        major: hm,
        minor: hn,
    });
    with_ver.extend_from_slice(events);
    Ok(Cow::Owned(with_ver))
}

/// Encode helper: ensure a body VERSION matching `major`/`minor` is present, then seal.
///
/// If `events` already contains any `VERSION`, each must match header fields or encode fails.
/// If none, a VERSION record is prepended before encode. Decode with
/// [`decode_decoded_event_profile_auto_version`] or plain decode (body carries VERSION).
///
/// Uses absolute event-body encode (not site-delta packing). For packing compose see
/// [`encode_decoded_event_profile_auto_version_with_site_deltas_and_seq`].
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
    let with_ver = events_with_auto_version(major, minor, events)?;
    encode_decoded_event_profile(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        event_codec,
        with_ver.as_ref(),
        max_events_per_chunk,
        footer,
    )
}

/// Encode helper: auto-emit/validate header-matching VERSION, then **site-delta/seq packing**
/// with optional multi-chunk continuity.
///
/// Prepends VERSION when body has none; fail-closed on header/body VERSION mismatch.
/// Packing uses [`encode_decoded_event_profile_with_site_deltas_and_seq`] so site bases and
/// sequence numbers continue across chunk boundaries (incl. after TIME_*_RUN).
///
/// Decode with [`decode_decoded_event_profile_auto_version`] (synthetic VERSION inject when
/// body omits it is not needed if encode injected; sequences stay aligned).
/// Not dual-equality freeze / permanent packing ADR / COL-007 C writer.
pub fn encode_decoded_event_profile_auto_version_with_site_deltas_and_seq(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    event_codec: u8,
    events: &[EventRecordSpec<'_>],
    max_events_per_chunk: usize,
    dict_entries: Option<&[(u64, u8, &[u8])]>,
) -> DecodedEventResult<Vec<u8>> {
    let with_ver = events_with_auto_version(major, minor, events)?;
    encode_decoded_event_profile_with_site_deltas_and_seq(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        event_codec,
        with_ver.as_ref(),
        max_events_per_chunk,
        dict_entries,
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

/// Validate mid-stream codec-switch preconditions (START_DEFLATE + codecs differ).
fn validate_mid_stream_codec_switch(
    pre_codec: u8,
    pre_events: &[EventRecordSpec<'_>],
    post_codec: u8,
    post_events: &[EventRecordSpec<'_>],
) -> DecodedEventResult<()> {
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
    Ok(())
}

/// Encode a provisional EVENT profile that **switches payload codec after a START_DEFLATE marker**.
///
/// Wire model (chunk-framed preflight, not v5 mid-payload byte-stream deflate):
/// 1. One EVENT chunk under `pre_codec` (typically NONE) whose body includes
///    `START_DEFLATE` (usually last pre-switch record).
/// 2. One EVENT chunk under `post_codec` (≠ pre) carrying post-switch workload records.
/// 3. Optional FOOTER codec NONE last.
///
/// Bodies use **absolute** event encode (not packing). For packing continuity across
/// the switch see [`encode_decoded_event_mid_stream_codec_switch_with_site_deltas_and_seq`].
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
    validate_mid_stream_codec_switch(pre_codec, pre_events, post_codec, post_events)?;

    let pre_plain = encode_event_body(pre_events);
    let post_plain = encode_event_body(post_events);
    seal_mid_stream_event_profile(
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
        footer,
    )
}

/// Ensure header-matching VERSION appears in pre||post; inject at start of pre if none.
///
/// Validates VERSION in both regions; fails on mismatch. Inject only into **pre** so
/// START_DEFLATE packing continuity is unchanged. Used by auto-VERSION mid-stream compose.
fn mid_stream_pre_with_auto_version<'a>(
    major: u16,
    minor: u16,
    pre: &'a [EventRecordSpec<'a>],
    post: &'a [EventRecordSpec<'a>],
) -> DecodedEventResult<Cow<'a, [EventRecordSpec<'a>]>> {
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
        return Ok(Cow::Borrowed(pre));
    }
    let mut with_ver: Vec<EventRecordSpec<'a>> = Vec::with_capacity(pre.len() + 1);
    with_ver.push(EventRecordSpec::Version {
        major: hm,
        minor: hn,
    });
    with_ver.extend_from_slice(pre);
    Ok(Cow::Owned(with_ver))
}

/// Encode mid-stream packing continuity with **auto-emit/validate VERSION**.
///
/// Ensures a header-matching VERSION is present in pre||post (injects at start of **pre**
/// when absent), then encodes with
/// [`encode_decoded_event_mid_stream_codec_switch_with_site_deltas_and_seq`] so packing
/// bases continue across the codec switch. Decode with
/// [`decode_decoded_event_profile_auto_version`].
/// Not dual-equality freeze / permanent packing ADR / COL-007 C writer.
pub fn encode_decoded_event_mid_stream_codec_switch_auto_version_with_site_deltas_and_seq(
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
    let pre = mid_stream_pre_with_auto_version(major, minor, pre_events, post_events)?;
    encode_decoded_event_mid_stream_codec_switch_with_site_deltas_and_seq(
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
        footer,
    )
}

/// Encode mid-stream START_DEFLATE codec-switch with **site-delta/seq packing continuity**.
///
/// Pre and post EVENT plains use one shared [`PackingEncodeState`]: site bases and sequence
/// numbers continue across the codec switch (join equals single continuous packing of
/// `pre || post`). Prefer TIME_*_RUN in pre with post-run site-delta in post.
///
/// `footer` is optional opaque FOOTER bytes (codec NONE). For FOOTER string-dictionary
/// packaging see
/// [`encode_decoded_event_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq`].
/// For auto-VERSION inject see
/// [`encode_decoded_event_mid_stream_codec_switch_auto_version_with_site_deltas_and_seq`].
///
/// Decode with [`decode_decoded_event_profile`] (always-inflate join). Default parse
/// remains non-inflating. Not permanent packing ADR / COL-007 C writer.
pub fn encode_decoded_event_mid_stream_codec_switch_with_site_deltas_and_seq(
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
    validate_mid_stream_codec_switch(pre_codec, pre_events, post_codec, post_events)?;

    let mut packing = PackingEncodeState::new();
    let pre_plain =
        encode_event_body_with_site_deltas_and_seq_continuing(pre_events, &mut packing)?;
    let post_plain =
        encode_event_body_with_site_deltas_and_seq_continuing(post_events, &mut packing)?;
    seal_mid_stream_event_profile(
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
        footer,
    )
}

/// Encode mid-stream packing continuity with **FOOTER string-dictionary**.
///
/// Same packing continuity as
/// [`encode_decoded_event_mid_stream_codec_switch_with_site_deltas_and_seq`], then FOOTER
/// is the provisional dictionary table (codec NONE). Decode with
/// [`decode_decoded_event_profile_with_string_dict`].
/// Not permanent packing/string-pool ADR / COL-007 C writer.
pub fn encode_decoded_event_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq(
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
    dict_entries: &[(u64, u8, &[u8])],
) -> DecodedEventResult<Vec<u8>> {
    let dict_bytes = encode_string_dictionary(dict_entries)?;
    encode_decoded_event_mid_stream_codec_switch_with_site_deltas_and_seq(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        pre_codec,
        pre_events,
        post_codec,
        post_events,
        Some(&dict_bytes),
    )
}

fn seal_mid_stream_event_profile(
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
    footer: Option<&[u8]>,
) -> DecodedEventResult<Vec<u8>> {
    let pre_frame = encode_event_chunk(pre_codec, 0, pre_count, pre_plain)?;
    let post_frame = encode_event_chunk(post_codec, 1, post_count, post_plain)?;

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
    let before = prof.records.len();
    align_event_records_version_with_header(&prof.header, &mut prof.records)?;
    // Keep parallel sequences aligned when a synthetic VERSION is prepended.
    if prof.records.len() == before + 1 {
        prof.sequences.insert(0, None);
    }
    debug_assert_eq!(prof.records.len(), prof.sequences.len());
    Ok((prof, n))
}

/// Encode EVENT profile with a provisional string-dictionary table in FOOTER (codec NONE).
///
/// Event-body string-blobs may use non-zero `string_id` with empty inline payloads;
/// decode with [`decode_decoded_event_profile_with_string_dict`] to resolve them.
/// Default `parse_chunk_frame` remains non-inflating.
pub fn encode_decoded_event_profile_with_string_dict(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    event_codec: u8,
    events: &[EventRecordSpec<'_>],
    max_events_per_chunk: usize,
    dict_entries: &[(u64, u8, &[u8])],
) -> DecodedEventResult<Vec<u8>> {
    let dict_bytes = encode_string_dictionary(dict_entries)?;
    encode_decoded_event_profile(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        event_codec,
        events,
        max_events_per_chunk,
        Some(&dict_bytes),
    )
}

/// Encode EVENT profile with **composed** FOOTER string-dictionary + site-delta/seq packing.
///
/// EVENT body packing uses continuous site/seq bases (via
/// [`encode_decoded_event_profile_with_site_deltas_and_seq`]). FOOTER is the provisional
/// dictionary table (codec NONE).
///
/// `max_events_per_chunk == 0` → single EVENT chunk; `>= 1` → record-aligned multi-chunk
/// partition with bases continuing across chunks.
///
/// Decode with [`decode_decoded_event_profile_with_string_dict`] (always-inflate + resolve;
/// recovers absolute sites, sequences, and dictionary string bytes).
///
/// Not a permanent global string-pool or packing ADR. Default `parse_chunk_frame` stays non-inflating.
pub fn encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    event_codec: u8,
    events: &[EventRecordSpec<'_>],
    max_events_per_chunk: usize,
    dict_entries: &[(u64, u8, &[u8])],
) -> DecodedEventResult<Vec<u8>> {
    encode_decoded_event_profile_with_site_deltas_and_seq(
        major,
        minor,
        required_features,
        optional_features,
        header_crc,
        tlv_items,
        event_codec,
        events,
        max_events_per_chunk,
        Some(dict_entries),
    )
}

/// Encode EVENT profile with site-delta/seq packing, optional multi-chunk partition,
/// and optional FOOTER string-dictionary.
///
/// When `max_events_per_chunk >= 1`, partitions via
/// [`crate::multi_chunk_event::partition_event_records`] and encodes each partition with
/// [`encode_event_body_with_site_deltas_and_seq_continuing`] so **site bases and sequence
/// numbers continue across chunk boundaries** (join equals single-chunk packing wire).
///
/// `max_events_per_chunk == 0` → single EVENT chunk (unlimited).
/// `dict_entries == None` → no FOOTER; `Some` → FOOTER dictionary table (codec NONE).
///
/// Decode with [`decode_decoded_event_profile`] or
/// [`decode_decoded_event_profile_with_string_dict`] when a dictionary FOOTER is present.
/// Not a permanent packing ADR. Default `parse_chunk_frame` stays non-inflating.
pub fn encode_decoded_event_profile_with_site_deltas_and_seq(
    major: u16,
    minor: u16,
    required_features: u64,
    optional_features: u64,
    header_crc: u32,
    tlv_items: &[(u64, u8, &[u8])],
    event_codec: u8,
    events: &[EventRecordSpec<'_>],
    max_events_per_chunk: usize,
    dict_entries: Option<&[(u64, u8, &[u8])]>,
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

    let mut sealed: Vec<Vec<u8>> =
        Vec::with_capacity(parts.len() + usize::from(dict_entries.is_some()));
    let mut packing = PackingEncodeState::new();
    for (i, part) in parts.iter().enumerate() {
        let plain = encode_event_body_with_site_deltas_and_seq_continuing(part, &mut packing)?;
        sealed.push(encode_event_chunk(
            event_codec,
            i as u64,
            part.len() as u32,
            &plain,
        )?);
    }

    if let Some(entries) = dict_entries {
        let dict_bytes = encode_string_dictionary(entries)?;
        let checksum = compute_payload_crc(&dict_bytes);
        sealed.push(encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            sealed.len() as u64,
            0,
            0,
            dict_bytes.len() as u32,
            &dict_bytes,
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

/// Decode always-inflate EVENT profile and resolve string_ids via FOOTER dictionary.
///
/// FOOTER payload must be a complete [`decode_string_dictionary`] table (no trailing
/// garbage). Records are replaced with dictionary-resolved owned strings.
pub fn decode_decoded_event_profile_with_string_dict(
    buf: &[u8],
    verify_crc: bool,
) -> DecodedEventResult<(DecodedEventProfile, StringDictionary, usize)> {
    let (mut prof, n) = decode_decoded_event_profile(buf, verify_crc)?;
    let footer = prof
        .footer_payload
        .as_deref()
        .ok_or(DecodedEventError::MissingStringDictionaryFooter)?;
    let (dict, dict_n) = decode_string_dictionary(footer)?;
    if dict_n != footer.len() {
        return Err(DecodedEventError::StringDict(StringDictError::Truncated {
            need: footer.len(),
            got: dict_n,
        }));
    }
    // Re-decode body plains is not available on OwnedEventRecord; resolve by
    // re-walking via event-body when possible is heavy. Instead re-inflate stream
    // is already owned from inline data only — re-resolve from raw body.
    // Re-parse EVENT plains from wire for correct string_id retention.
    let (stream, _) = decode_prefix_chunk_stream_plain(buf, verify_crc)?;
    let mut plain = Vec::new();
    for ch in &stream.chunks {
        if ch.kind == kind::EVENT {
            plain.extend_from_slice(&ch.plain);
        }
    }
    if plain.is_empty() {
        prof.records.clear();
        prof.sequences.clear();
        return Ok((prof, dict, n));
    }
    let (decoded_body, body_n) = decode_event_body_full(&plain)?;
    if body_n != plain.len() {
        return Err(DecodedEventError::EventBody(EventBodyError::Truncated {
            need: plain.len(),
            got: body_n,
        }));
    }
    prof.records = resolve_event_records(&decoded_body.records, &dict)?;
    prof.sequences = decoded_body.sequences;
    Ok((prof, dict, n))
}

/// Decode FOOTER string-dict resolve, then auto-align VERSION with fixed-header.
///
/// Resolves dictionary string_ids, then prepends synthetic VERSION when body omits it
/// (keeping `sequences` aligned). Fail-closed on header/body VERSION mismatch or
/// unknown dict id. Compose preflight for auto-VERSION + packing + FOOTER dict.
pub fn decode_decoded_event_profile_auto_version_with_string_dict(
    buf: &[u8],
    verify_crc: bool,
) -> DecodedEventResult<(DecodedEventProfile, StringDictionary, usize)> {
    let (mut prof, dict, n) = decode_decoded_event_profile_with_string_dict(buf, verify_crc)?;
    let before = prof.records.len();
    align_event_records_version_with_header(&prof.header, &mut prof.records)?;
    if prof.records.len() == before + 1 {
        prof.sequences.insert(0, None);
    }
    debug_assert_eq!(prof.records.len(), prof.sequences.len());
    Ok((prof, dict, n))
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
                sequences: Vec::new(),
                has_footer,
                footer_payload,
            },
            n,
        ));
    }

    if event_chunk_count == 0 {
        return Err(DecodedEventError::MissingEventChunks);
    }

    let (decoded_body, body_n) = decode_event_body_full(&plain)?;
    if body_n != plain.len() {
        return Err(DecodedEventError::EventBody(EventBodyError::Truncated {
            need: plain.len(),
            got: body_n,
        }));
    }

    let mut records = Vec::with_capacity(decoded_body.records.len());
    for r in &decoded_body.records {
        records.push(OwnedEventRecord::from_borrowed(r));
    }

    Ok((
        DecodedEventProfile {
            header: stream.header,
            event_codec,
            event_chunk_codecs,
            event_chunk_count,
            records,
            sequences: decoded_body.sequences,
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

    /// Pre region with TIME_LINE_RUN ending before START_DEFLATE; packing continues into post.
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
            // Site-delta after run in pre — must land on (2,51) with continued packing.
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

    #[test]
    fn mid_stream_codec_switch_packing_none_to_zlib_zstd_lz4_always_inflate() {
        let pre = mid_stream_packing_pre_events();
        let post = mid_stream_packing_post_events();
        // Single continuous packing of pre||post as equality baseline.
        let mut all: Vec<EventRecordSpec<'static>> = pre.to_vec();
        all.extend_from_slice(&post);
        let single_plain =
            crate::event_body::encode_event_body_with_site_deltas_and_seq(&all).unwrap();
        let (single_body, _) =
            crate::event_body::decode_event_body_full(&single_plain).unwrap();

        for post_c in [codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_mid_stream_codec_switch_with_site_deltas_and_seq(
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
                None,
            )
            .unwrap_or_else(|e| panic!("encode post_codec {post_c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode post_codec {post_c}: {e}"));
            assert_eq!(n, wire.len(), "codec {post_c}");
            assert_eq!(prof.event_chunk_count, 2);
            assert_eq!(prof.event_chunk_codecs, vec![codec::NONE, post_c]);
            // Join equals continuous packing of pre||post (absolute sites + seq).
            let owned: Vec<_> = single_body
                .records
                .iter()
                .map(OwnedEventRecord::from_borrowed)
                .collect();
            assert_eq!(prof.records, owned, "codec {post_c}");
            assert_eq!(prof.sequences, single_body.sequences, "codec {post_c}");
            // Post-run site-delta is after TL + 2 expanded run + StartDeflate = index 4
            match &prof.records[4] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!(
                        (*fid, *line, *ticks),
                        (2, 51, 9),
                        "codec {post_c}: post-run site-delta across codec switch"
                    );
                }
                other => panic!("codec {post_c}: [4] {other:?}"),
            }
            // Default parse must not inflate compressed post chunk.
            let (stream_raw, _) = decode_prefix_chunk_stream(&wire).expect("raw");
            assert_eq!(stream_raw.chunks[1].codec, post_c);
            let (stream_plain, _) = decode_prefix_chunk_stream_plain(&wire, true).unwrap();
            assert_ne!(
                stream_raw.chunks[1].payload,
                stream_plain.chunks[1].plain.as_slice(),
                "codec {post_c}: default parse must not inflate post chunk"
            );
            // Joined plains equal continuous packing body.
            let mut joined = Vec::new();
            for ch in &stream_plain.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, single_plain, "codec {post_c}");
        }
    }

    #[test]
    fn mid_stream_codec_switch_packing_missing_marker_err() {
        let pre = [
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
        ];
        let post = mid_stream_packing_post_events();
        match encode_decoded_event_mid_stream_codec_switch_with_site_deltas_and_seq(
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
    fn mid_stream_codec_switch_packing_same_codec_err() {
        let pre = mid_stream_packing_pre_events();
        let post = mid_stream_packing_post_events();
        match encode_decoded_event_mid_stream_codec_switch_with_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &pre,
            codec::NONE,
            &post,
            None,
        ) {
            Err(DecodedEventError::MidStreamCodecsMustDiffer { codec: 0 }) => {}
            other => panic!("expected MidStreamCodecsMustDiffer, got {other:?}"),
        }
    }

    /// Pre with TIME_LINE_RUN + START_DEFLATE; post with post-run site-delta + MARK/COMMENT dict ids.
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
    fn mid_stream_codec_switch_dict_packing_none_to_zlib_zstd_lz4_always_inflate() {
        use crate::string::FLAG_UTF8;
        let pre = mid_stream_dict_packing_pre_events();
        let post = mid_stream_dict_packing_post_events();
        let dict_entries: &[(u64, u8, &[u8])] = &[
            (1, FLAG_UTF8, b"ms-dict-mark"),
            (2, 0, b"# ms-dict-end"),
        ];
        let mut all: Vec<EventRecordSpec<'static>> = pre.to_vec();
        all.extend_from_slice(&post);
        let single_plain =
            crate::event_body::encode_event_body_with_site_deltas_and_seq(&all).unwrap();
        // Single continuous packing + dict FOOTER baseline via mid-stream helper with same codec
        // is invalid (codecs must differ). Use packing mid-stream with post ZLIB as multi==single
        // via first successful encode as baseline of records.
        let baseline_wire =
            encode_decoded_event_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq(
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
                dict_entries,
            )
            .expect("baseline encode");
        let (single_prof, single_dict, _) =
            decode_decoded_event_profile_with_string_dict(&baseline_wire, true).unwrap();
        assert_eq!(single_dict.get(1).unwrap().data, b"ms-dict-mark");
        assert_eq!(single_dict.get(2).unwrap().data, b"# ms-dict-end");
        // Indices: TL, TLR×2, StartDeflate, TL@51 = 4
        match &single_prof.records[4] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("[4] {other:?}"),
        }
        let n = single_prof.records.len();
        match &single_prof.records[n - 2] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"ms-dict-mark"),
            other => panic!("mark {other:?}"),
        }
        match &single_prof.records[n - 1] {
            OwnedEventRecord::Comment { text } => assert_eq!(text, b"# ms-dict-end"),
            other => panic!("comment {other:?}"),
        }

        for post_c in [codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire =
                encode_decoded_event_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq(
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
                    dict_entries,
                )
                .unwrap_or_else(|e| panic!("encode post_codec {post_c}: {e}"));
            let (prof, dict, n_read) =
                decode_decoded_event_profile_with_string_dict(&wire, true)
                    .unwrap_or_else(|e| panic!("decode post_codec {post_c}: {e}"));
            assert_eq!(n_read, wire.len(), "codec {post_c}");
            assert_eq!(prof.event_chunk_count, 2);
            assert_eq!(dict.get(1).unwrap().data, b"ms-dict-mark");
            assert_eq!(dict.get(2).unwrap().data, b"# ms-dict-end");
            assert_eq!(prof.records, single_prof.records, "codec {post_c}");
            assert_eq!(prof.sequences, single_prof.sequences, "codec {post_c}");
            match &prof.records[4] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!(
                        (*fid, *line, *ticks),
                        (2, 51, 9),
                        "codec {post_c}: post-run site-delta across switch"
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
            // Joined EVENT plains equal continuous packing body.
            let mut joined = Vec::new();
            for ch in &stream_plain.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, single_plain, "codec {post_c}");
        }
    }

    #[test]
    fn mid_stream_codec_switch_auto_version_packing_none_to_zlib_zstd_lz4_always_inflate() {
        let pre = mid_stream_packing_pre_events(); // no VERSION; ends with START_DEFLATE
        let post = mid_stream_packing_post_events();
        let header_minor = 6u16;

        let baseline =
            encode_decoded_event_mid_stream_codec_switch_auto_version_with_site_deltas_and_seq(
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
                None,
            )
            .expect("baseline");
        let (single_prof, _) =
            decode_decoded_event_profile_auto_version(&baseline, true).unwrap();
        match &single_prof.records[0] {
            OwnedEventRecord::Version { major, minor } => {
                assert_eq!(
                    (*major, *minor),
                    (u64::from(SUPPORTED_MAJOR), u64::from(header_minor))
                );
            }
            other => panic!("expected VERSION, got {other:?}"),
        }
        // VERSION + TL + TLR×2 + StartDeflate + TL@51 = index 5
        match &single_prof.records[5] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("post-run [5] {other:?}"),
        }

        // Continuous packing of VERSION||pre||post for plain equality.
        let mut all: Vec<EventRecordSpec<'static>> = vec![EventRecordSpec::Version {
            major: u64::from(SUPPORTED_MAJOR),
            minor: u64::from(header_minor),
        }];
        all.extend_from_slice(&pre);
        all.extend_from_slice(&post);
        let single_plain =
            crate::event_body::encode_event_body_with_site_deltas_and_seq(&all).unwrap();

        for post_c in [codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire =
                encode_decoded_event_mid_stream_codec_switch_auto_version_with_site_deltas_and_seq(
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
                    None,
                )
                .unwrap_or_else(|e| panic!("encode post_codec {post_c}: {e}"));
            let (prof, n) = decode_decoded_event_profile_auto_version(&wire, true)
                .unwrap_or_else(|e| panic!("decode post_codec {post_c}: {e}"));
            assert_eq!(n, wire.len(), "codec {post_c}");
            assert_eq!(prof.event_chunk_count, 2);
            assert_eq!(prof.records, single_prof.records, "codec {post_c}");
            assert_eq!(prof.sequences, single_prof.sequences, "codec {post_c}");
            match &prof.records[5] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!(
                        (*fid, *line, *ticks),
                        (2, 51, 9),
                        "codec {post_c}: post-run site-delta across switch"
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
            let mut joined = Vec::new();
            for ch in &stream_plain.chunks {
                if ch.kind == kind::EVENT {
                    joined.extend_from_slice(&ch.plain);
                }
            }
            assert_eq!(joined, single_plain, "codec {post_c}");
        }
    }

    #[test]
    fn mid_stream_codec_switch_auto_version_packing_mismatch_fail_closed() {
        let mut pre = mid_stream_packing_pre_events().to_vec();
        pre.insert(
            0,
            EventRecordSpec::Version {
                major: u64::from(SUPPORTED_MAJOR),
                minor: 88,
            },
        );
        let post = mid_stream_packing_post_events();
        match encode_decoded_event_mid_stream_codec_switch_auto_version_with_site_deltas_and_seq(
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
            Err(DecodedEventError::VersionHeaderMismatch {
                body_minor: 88, ..
            }) => {}
            other => panic!("expected VersionHeaderMismatch, got {other:?}"),
        }
    }

    #[test]
    fn mid_stream_codec_switch_dict_packing_unknown_id_fail_closed() {
        let pre = mid_stream_dict_packing_pre_events();
        let post = [
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 51,
                ticks: 9,
            },
            EventRecordSpec::Mark {
                string_id: 99,
                string_flags: 0,
                label: b"",
            },
        ];
        let wire =
            encode_decoded_event_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq(
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
                &[(1, 0, b"other")],
            )
            .expect("encode");
        match decode_decoded_event_profile_with_string_dict(&wire, true) {
            Err(DecodedEventError::StringDict(crate::string_dict::StringDictError::UnknownId {
                id: 99,
            })) => {}
            other => panic!("expected UnknownId 99, got {other:?}"),
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

    #[test]
    fn known_key_expanded_inventory_none_zlib_zstd_lz4_always_inflate() {
        use crate::event_body::known_key_attr_option_expanded_sample_specs;
        let events = known_key_attr_option_expanded_sample_specs();
        let expect_n = events.len();
        assert_eq!(
            expect_n,
            crate::known_key::KNOWN_ATTRIBUTE_KEYS.len()
                + crate::known_key::KNOWN_OPTION_KEYS.len()
        );
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
            assert_eq!(prof.records.len(), expect_n, "codec {c}");
            for (i, r) in prof.records.iter().enumerate() {
                match r {
                    OwnedEventRecord::Attribute { key, value } => {
                        assert!(
                            crate::known_key::is_known_attribute_key(key),
                            "codec {c} [{i}] ATTRIBUTE {key:?}"
                        );
                        // Value recovered from real encode path (not hard-coded alone).
                        match &events[i] {
                            EventRecordSpec::Attribute {
                                key: sk,
                                value: sv,
                                ..
                            } => {
                                assert_eq!(key.as_slice(), *sk);
                                assert_eq!(value.as_slice(), *sv);
                            }
                            other => panic!("codec {c} [{i}] expected Attribute spec, {other:?}"),
                        }
                    }
                    OwnedEventRecord::Option { key, value } => {
                        assert!(
                            crate::known_key::is_known_option_key(key),
                            "codec {c} [{i}] OPTION {key:?}"
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
                            other => panic!("codec {c} [{i}] expected Option spec, {other:?}"),
                        }
                    }
                    other => panic!("codec {c} [{i}] unexpected {other:?}"),
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

    #[test]
    fn string_dict_intern_none_zlib_zstd_lz4_always_inflate() {
        use crate::string::FLAG_UTF8;
        let events = [
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"", // interned via dict id 1
            },
            EventRecordSpec::Attribute {
                key_string_id: 2,
                key_string_flags: 0,
                key: b"",
                value_string_id: 0,
                value_string_flags: 0,
                value: b"1786111723", // inline value
            },
            EventRecordSpec::Comment {
                string_id: 3,
                string_flags: 0,
                text: b"",
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"inline-mark",
            },
        ];
        let dict_entries: &[(u64, u8, &[u8])] = &[
            (1, FLAG_UTF8, b"dict-label"),
            (2, 0, b"basetime"),
            (3, 0, b"# dict comment"),
        ];
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile_with_string_dict(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0,
                dict_entries,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, dict, n) = decode_decoded_event_profile_with_string_dict(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(dict.len(), 3, "codec {c}");
            assert_eq!(dict.get(1).unwrap().data, b"dict-label");
            assert_eq!(prof.records.len(), 4, "codec {c}");
            match &prof.records[0] {
                OwnedEventRecord::Mark { label } => {
                    assert_eq!(label, b"dict-label", "codec {c}: resolved MARK");
                }
                other => panic!("codec {c}: {other:?}"),
            }
            match &prof.records[1] {
                OwnedEventRecord::Attribute { key, value } => {
                    assert_eq!(key, b"basetime", "codec {c}: resolved ATTRIBUTE key");
                    assert_eq!(value, b"1786111723");
                }
                other => panic!("codec {c}: {other:?}"),
            }
            match &prof.records[2] {
                OwnedEventRecord::Comment { text } => {
                    assert_eq!(text, b"# dict comment", "codec {c}");
                }
                other => panic!("codec {c}: {other:?}"),
            }
            match &prof.records[3] {
                OwnedEventRecord::Mark { label } => assert_eq!(label, b"inline-mark"),
                other => panic!("codec {c}: {other:?}"),
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
    fn string_dict_unknown_id_in_footer_resolved_path_err() {
        let events = [EventRecordSpec::Mark {
            string_id: 42,
            string_flags: 0,
            label: b"",
        }];
        // Dict missing id 42.
        let wire = encode_decoded_event_profile_with_string_dict(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &events,
            0,
            &[(1, 0, b"other")],
        )
        .expect("encode");
        match decode_decoded_event_profile_with_string_dict(&wire, true) {
            Err(DecodedEventError::StringDict(crate::string_dict::StringDictError::UnknownId {
                id: 42,
            })) => {}
            other => panic!("expected UnknownId 42, got {other:?}"),
        }
    }

    /// Compose: FOOTER dictionary + site-delta/seq packing on EVENT body.
    fn string_dict_and_packing_compose_specs() -> [EventRecordSpec<'static>; 6] {
        [
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"", // resolved from dict
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 11,
                ticks: 6,
            },
            EventRecordSpec::Attribute {
                key_string_id: 2,
                key_string_flags: 0,
                key: b"",
                value_string_id: 0,
                value_string_flags: 0,
                value: b"1786111723", // inline value
            },
            EventRecordSpec::SubEntry {
                caller_fid: 1,
                caller_line: 10,
            },
            EventRecordSpec::Comment {
                string_id: 3,
                string_flags: 0,
                text: b"",
            },
        ]
    }

    fn assert_string_dict_and_packing_compose(
        recs: &[OwnedEventRecord],
        sequences: &[Option<u64>],
    ) {
        assert_eq!(recs.len(), 6);
        assert_eq!(sequences.len(), 6);
        for (i, s) in sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
        match &recs[0] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"compose-dict-mark"),
            other => panic!("[0] {other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("[1] {other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 11, 6));
            }
            other => panic!("[2] {other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::Attribute { key, value } => {
                assert_eq!(key, b"basetime");
                assert_eq!(value, b"1786111723");
            }
            other => panic!("[3] {other:?}"),
        }
        match &recs[4] {
            OwnedEventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => assert_eq!((*caller_fid, *caller_line), (1, 10)),
            other => panic!("[4] {other:?}"),
        }
        match &recs[5] {
            OwnedEventRecord::Comment { text } => assert_eq!(text, b"# compose dict packing"),
            other => panic!("[5] {other:?}"),
        }
    }

    #[test]
    fn string_dict_and_site_delta_seq_compose_none_zlib_zstd_lz4_always_inflate() {
        use crate::string::FLAG_UTF8;
        let events = string_dict_and_packing_compose_specs();
        let dict_entries: &[(u64, u8, &[u8])] = &[
            (1, FLAG_UTF8, b"compose-dict-mark"),
            (2, 0, b"basetime"),
            (3, 0, b"# compose dict packing"),
        ];
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &events,
                0, // single-chunk compose
                dict_entries,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, dict, n) = decode_decoded_event_profile_with_string_dict(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(dict.len(), 3, "codec {c}");
            assert_eq!(dict.get(1).unwrap().data, b"compose-dict-mark");
            assert_string_dict_and_packing_compose(&prof.records, &prof.sequences);

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
    fn string_dict_and_packing_compose_unknown_id_fail_closed() {
        let events = [
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
        ];
        let wire = encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &events,
            0,
            &[(1, 0, b"other")],
        )
        .expect("encode");
        match decode_decoded_event_profile_with_string_dict(&wire, true) {
            Err(DecodedEventError::StringDict(crate::string_dict::StringDictError::UnknownId {
                id: 99,
            })) => {}
            other => panic!("expected UnknownId 99, got {other:?}"),
        }
    }

    fn site_delta_sample_specs() -> [EventRecordSpec<'static>; 5] {
        [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 11,
                ticks: 6,
            },
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 12,
                block_line: 4,
                ticks: 20,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 1,
                caller_line: 10,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 2,
                caller_line: 3,
            },
        ]
    }

    fn assert_site_delta_abs(recs: &[OwnedEventRecord]) {
        assert_eq!(recs.len(), 5);
        match &recs[0] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("[0] {other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 11, 6));
            }
            other => panic!("[1] {other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (1, 12, 4, 20)),
            other => panic!("[2] {other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => assert_eq!((*caller_fid, *caller_line), (1, 10)),
            other => panic!("[3] {other:?}"),
        }
        match &recs[4] {
            OwnedEventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => assert_eq!((*caller_fid, *caller_line), (2, 3)),
            other => panic!("[4] {other:?}"),
        }
    }

    #[test]
    fn site_delta_none_zlib_zstd_lz4_always_inflate() {
        use crate::event_body::encode_event_body_with_site_deltas;
        let specs = site_delta_sample_specs();
        let plain = encode_event_body_with_site_deltas(&specs).expect("delta plain");
        // Body-layer absolute reconstruction first.
        let (body_recs, bn) = crate::event_body::decode_event_body(&plain).unwrap();
        assert_eq!(bn, plain.len());
        let owned: Vec<_> = body_recs
            .iter()
            .map(OwnedEventRecord::from_borrowed)
            .collect();
        assert_site_delta_abs(&owned);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let frame = crate::compressed_profile::encode_event_chunk(c, 0, 5, &plain)
                .unwrap_or_else(|e| panic!("seal codec {c}: {e}"));
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &[frame.as_slice()],
            );
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_site_delta_abs(&prof.records);

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

    fn time_line_run_sample_specs() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 9,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 50,
                ticks: &[10, 20, 7],
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 99,
                ticks: 1,
            },
        ]
    }

    fn assert_time_line_run_expanded(recs: &[OwnedEventRecord]) {
        // plain + 3 expanded run + plain = 5
        assert_eq!(recs.len(), 5);
        match &recs[0] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 1, 9));
            }
            other => panic!("[0] {other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 50, 10));
            }
            other => panic!("[1] {other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 50, 20));
            }
            other => panic!("[2] {other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 50, 7));
            }
            other => panic!("[3] {other:?}"),
        }
        match &recs[4] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (3, 99, 1));
            }
            other => panic!("[4] {other:?}"),
        }
    }

    #[test]
    fn time_line_run_none_zlib_zstd_lz4_always_inflate() {
        let specs = time_line_run_sample_specs();
        let plain = encode_event_body(&specs);
        let (body_recs, bn) = crate::event_body::decode_event_body(&plain).unwrap();
        assert_eq!(bn, plain.len());
        let owned: Vec<_> = body_recs
            .iter()
            .map(OwnedEventRecord::from_borrowed)
            .collect();
        assert_time_line_run_expanded(&owned);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let frame = crate::compressed_profile::encode_event_chunk(c, 0, 5, &plain)
                .unwrap_or_else(|e| panic!("seal codec {c}: {e}"));
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &[frame.as_slice()],
            );
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_time_line_run_expanded(&prof.records);

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

    fn time_block_run_sample_specs() -> [EventRecordSpec<'static>; 3] {
        [
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 1,
                block_line: 1,
                ticks: 9,
            },
            EventRecordSpec::TimeBlockRun {
                fid: 2,
                line: 50,
                block_line: 4,
                ticks: &[10, 20, 7],
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 99,
                ticks: 1,
            },
        ]
    }

    fn assert_time_block_run_expanded(recs: &[OwnedEventRecord]) {
        // 1 plain TB + 3 run + 1 TL = 5
        assert_eq!(recs.len(), 5);
        match &recs[0] {
            OwnedEventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (1, 1, 1, 9)),
            other => panic!("[0] {other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (2, 50, 4, 10)),
            other => panic!("[1] {other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (2, 50, 4, 20)),
            other => panic!("[2] {other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (2, 50, 4, 7)),
            other => panic!("[3] {other:?}"),
        }
        match &recs[4] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (3, 99, 1));
            }
            other => panic!("[4] {other:?}"),
        }
    }

    #[test]
    fn time_block_run_none_zlib_zstd_lz4_always_inflate() {
        let specs = time_block_run_sample_specs();
        let plain = encode_event_body(&specs);
        let (body_recs, bn) = crate::event_body::decode_event_body(&plain).unwrap();
        assert_eq!(bn, plain.len());
        let owned: Vec<_> = body_recs
            .iter()
            .map(OwnedEventRecord::from_borrowed)
            .collect();
        assert_time_block_run_expanded(&owned);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let frame = crate::compressed_profile::encode_event_chunk(c, 0, 5, &plain)
                .unwrap_or_else(|e| panic!("seal codec {c}: {e}"));
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &[frame.as_slice()],
            );
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_time_block_run_expanded(&prof.records);

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

    fn event_seq_dual_output_specs() -> [EventRecordSpec<'static>; 9] {
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

    fn assert_event_seq_dual_output(recs: &[OwnedEventRecord], sequences: &[Option<u64>]) {
        assert_eq!(recs.len(), 9);
        assert_eq!(sequences.len(), 9);
        for (i, s) in sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
        match &recs[0] {
            OwnedEventRecord::Version { major, minor } => assert_eq!((*major, *minor), (5, 0)),
            other => panic!("[0] {other:?}"),
        }
        match &recs[4] {
            OwnedEventRecord::StartDeflate => {}
            other => panic!("[4] {other:?}"),
        }
        match &recs[6] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 42));
            }
            other => panic!("[6] {other:?}"),
        }
        match &recs[8] {
            OwnedEventRecord::PidEnd { pid, end_time } => {
                assert_eq!((*pid, *end_time), (1001, 1_700_000_042));
            }
            other => panic!("[8] {other:?}"),
        }
    }

    #[test]
    fn event_seq_none_zlib_zstd_lz4_always_inflate() {
        use crate::event_body::encode_event_body_with_seq;
        let specs = event_seq_dual_output_specs();
        let plain = encode_event_body_with_seq(&specs);
        let (decoded_body, bn) = crate::event_body::decode_event_body_full(&plain).unwrap();
        assert_eq!(bn, plain.len());
        let owned: Vec<_> = decoded_body
            .records
            .iter()
            .map(OwnedEventRecord::from_borrowed)
            .collect();
        assert_event_seq_dual_output(&owned, &decoded_body.sequences);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let frame = crate::compressed_profile::encode_event_chunk(c, 0, 9, &plain)
                .unwrap_or_else(|e| panic!("seal codec {c}: {e}"));
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &[frame.as_slice()],
            );
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_event_seq_dual_output(&prof.records, &prof.sequences);

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

    fn site_delta_and_seq_compose_specs() -> [EventRecordSpec<'static>; 7] {
        [
            EventRecordSpec::Version {
                major: 5,
                minor: 0,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 11,
                ticks: 6,
            },
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 12,
                block_line: 4,
                ticks: 20,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 1,
                caller_line: 10,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 2,
                caller_line: 3,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"compose-end",
            },
        ]
    }

    fn assert_site_delta_and_seq_compose_owned(
        recs: &[OwnedEventRecord],
        sequences: &[Option<u64>],
    ) {
        assert_eq!(recs.len(), 7);
        assert_eq!(sequences.len(), 7);
        for (i, s) in sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
        match &recs[0] {
            OwnedEventRecord::Version { major, minor } => assert_eq!((*major, *minor), (5, 0)),
            other => panic!("[0] {other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("[1] {other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 11, 6));
            }
            other => panic!("[2] {other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (1, 12, 4, 20)),
            other => panic!("[3] {other:?}"),
        }
        match &recs[4] {
            OwnedEventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => assert_eq!((*caller_fid, *caller_line), (1, 10)),
            other => panic!("[4] {other:?}"),
        }
        match &recs[5] {
            OwnedEventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => assert_eq!((*caller_fid, *caller_line), (2, 3)),
            other => panic!("[5] {other:?}"),
        }
        match &recs[6] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"compose-end"),
            other => panic!("[6] {other:?}"),
        }
    }

    #[test]
    fn site_delta_and_seq_compose_none_zlib_zstd_lz4_always_inflate() {
        use crate::event_body::encode_event_body_with_site_deltas_and_seq;
        let specs = site_delta_and_seq_compose_specs();
        let plain = encode_event_body_with_site_deltas_and_seq(&specs).expect("compose plain");
        let (decoded_body, bn) = crate::event_body::decode_event_body_full(&plain).unwrap();
        assert_eq!(bn, plain.len());
        let owned: Vec<_> = decoded_body
            .records
            .iter()
            .map(OwnedEventRecord::from_borrowed)
            .collect();
        assert_site_delta_and_seq_compose_owned(&owned, &decoded_body.sequences);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let frame = crate::compressed_profile::encode_event_chunk(c, 0, 7, &plain)
                .unwrap_or_else(|e| panic!("seal codec {c}: {e}"));
            let wire = encode_prefix_sealed_chunks(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                &[frame.as_slice()],
            );
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_site_delta_and_seq_compose_owned(&prof.records, &prof.sequences);

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

    /// ≥4 logical events with site-delta+seq, split into ≥2 EVENT chunks.
    fn multi_chunk_packing_specs() -> [EventRecordSpec<'static>; 6] {
        [
            EventRecordSpec::Version {
                major: 5,
                minor: 0,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 11,
                ticks: 6,
            },
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 12,
                block_line: 4,
                ticks: 20,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 1,
                caller_line: 10,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 1,
                ticks: 7,
            },
        ]
    }

    fn assert_multi_chunk_packing_owned(recs: &[OwnedEventRecord], sequences: &[Option<u64>]) {
        assert_eq!(recs.len(), 6);
        assert_eq!(sequences.len(), 6);
        for (i, s) in sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
        match &recs[1] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("[1] {other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 11, 6));
            }
            other => panic!("[2] {other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (1, 12, 4, 20)),
            other => panic!("[3] {other:?}"),
        }
        match &recs[5] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 1, 7));
            }
            other => panic!("[5] {other:?}"),
        }
    }

    #[test]
    fn multi_chunk_site_delta_seq_packing_none_zlib_zstd_lz4_always_inflate() {
        let specs = multi_chunk_packing_specs();
        // Single-chunk baseline for equality of recovered records/seq.
        let single_wire = encode_decoded_event_profile_with_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &specs,
            0,
            None,
        )
        .expect("single-chunk packing encode");
        let (single_prof, _) = decode_decoded_event_profile(&single_wire, true).unwrap();
        assert_multi_chunk_packing_owned(&single_prof.records, &single_prof.sequences);

        // max_events_per_chunk=2 → ≥3 EVENT chunks
        let parts = partition_event_records(&specs, 2);
        assert!(parts.len() >= 2);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile_with_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &specs,
                2, // multi-chunk
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_chunk_count, parts.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_multi_chunk_packing_owned(&prof.records, &prof.sequences);
            // Recovered logical stream matches single-chunk packing.
            assert_eq!(prof.records, single_prof.records, "codec {c}");
            assert_eq!(prof.sequences, single_prof.sequences, "codec {c}");

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

    /// Multi-chunk packing + FOOTER string-dictionary compose (≥4 logical events, ≥2 EVENT chunks).
    fn string_dict_multi_chunk_packing_specs() -> [EventRecordSpec<'static>; 6] {
        [
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"",
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 11,
                ticks: 6,
            },
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 12,
                block_line: 4,
                ticks: 20,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 1,
                caller_line: 10,
            },
            EventRecordSpec::Comment {
                string_id: 2,
                string_flags: 0,
                text: b"",
            },
        ]
    }

    fn assert_string_dict_multi_chunk_packing(
        recs: &[OwnedEventRecord],
        sequences: &[Option<u64>],
    ) {
        assert_eq!(recs.len(), 6);
        assert_eq!(sequences.len(), 6);
        for (i, s) in sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
        match &recs[0] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"mc-dict-mark"),
            other => panic!("[0] {other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("[1] {other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 11, 6));
            }
            other => panic!("[2] {other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (1, 12, 4, 20)),
            other => panic!("[3] {other:?}"),
        }
        match &recs[4] {
            OwnedEventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => assert_eq!((*caller_fid, *caller_line), (1, 10)),
            other => panic!("[4] {other:?}"),
        }
        match &recs[5] {
            OwnedEventRecord::Comment { text } => assert_eq!(text, b"# mc-dict-end"),
            other => panic!("[5] {other:?}"),
        }
    }

    #[test]
    fn string_dict_multi_chunk_site_delta_seq_packing_none_zlib_zstd_lz4_always_inflate() {
        use crate::string::FLAG_UTF8;
        let specs = string_dict_multi_chunk_packing_specs();
        let dict_entries: &[(u64, u8, &[u8])] = &[
            (1, FLAG_UTF8, b"mc-dict-mark"),
            (2, 0, b"# mc-dict-end"),
        ];
        // Single-chunk dict+packing baseline of the same specs.
        let single_wire = encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &specs,
            0,
            dict_entries,
        )
        .expect("single-chunk dict+packing encode");
        let (single_prof, single_dict, _) =
            decode_decoded_event_profile_with_string_dict(&single_wire, true).unwrap();
        assert_string_dict_multi_chunk_packing(&single_prof.records, &single_prof.sequences);
        assert_eq!(single_dict.get(1).unwrap().data, b"mc-dict-mark");
        assert_eq!(single_dict.get(2).unwrap().data, b"# mc-dict-end");

        // max_events_per_chunk=2 on 6 events → ≥2 EVENT chunks + FOOTER dict
        let parts = partition_event_records(&specs, 2);
        assert!(parts.len() >= 2, "must multi-chunk");
        assert!(
            parts.iter().all(|p| p.len() <= 2),
            "each partition ≤ max_events_per_chunk"
        );
        // Wire packing body: continuous multi-chunk join equals single-chunk packing.
        let single_plain =
            crate::event_body::encode_event_body_with_site_deltas_and_seq(&specs).unwrap();
        let mut packing = PackingEncodeState::new();
        let mut joined = Vec::new();
        for part in &parts {
            joined.extend_from_slice(
                &encode_event_body_with_site_deltas_and_seq_continuing(part, &mut packing).unwrap(),
            );
        }
        assert_eq!(
            joined, single_plain,
            "multi-chunk packing plains must equal single-chunk packing body"
        );

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &specs,
                2, // multi-chunk packing continuity + FOOTER dict
                dict_entries,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, dict, n) = decode_decoded_event_profile_with_string_dict(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_chunk_count, parts.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(dict.len(), 2, "codec {c}");
            assert_eq!(dict.get(1).unwrap().data, b"mc-dict-mark");
            assert_eq!(dict.get(2).unwrap().data, b"# mc-dict-end");
            assert_string_dict_multi_chunk_packing(&prof.records, &prof.sequences);
            // Multi-chunk join equals single-chunk compose of same specs.
            assert_eq!(prof.records, single_prof.records, "codec {c}");
            assert_eq!(prof.sequences, single_prof.sequences, "codec {c}");

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
    fn string_dict_multi_chunk_packing_unknown_id_fail_closed() {
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
        let wire = encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &specs,
            2, // multi-chunk
            &[(1, 0, b"other")],
        )
        .expect("encode");
        assert!(
            partition_event_records(&specs, 2).len() >= 2,
            "must partition to multi-chunk"
        );
        match decode_decoded_event_profile_with_string_dict(&wire, true) {
            Err(DecodedEventError::StringDict(crate::string_dict::StringDictError::UnknownId {
                id: 99,
            })) => {}
            other => panic!("expected UnknownId 99, got {other:?}"),
        }
    }

    /// Multi-chunk packing + TIME_LINE_RUN / TIME_BLOCK_RUN: site-delta after run
    /// across a chunk boundary; always-inflate under all codecs.
    fn multi_chunk_packing_with_runs_specs() -> [EventRecordSpec<'static>; 6] {
        // Static tick slices for 'static specs used in codec loops.
        const TL: &[u64] = &[7, 8];
        const TB: &[u64] = &[10, 20];
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
        ]
    }

    fn assert_multi_chunk_packing_with_runs_owned(
        recs: &[OwnedEventRecord],
        sequences: &[Option<u64>],
    ) {
        assert_eq!(recs.len(), 8);
        assert_eq!(sequences.len(), 8);
        for (i, s) in sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
        match &recs[0] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("[0] {other:?}"),
        }
        match &recs[1] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 50, 7));
            }
            other => panic!("[1] {other:?}"),
        }
        match &recs[2] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 50, 8));
            }
            other => panic!("[2] {other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!(
                    (*fid, *line, *ticks),
                    (2, 51, 9),
                    "post TIME_LINE_RUN site-delta across chunk boundary"
                );
            }
            other => panic!("[3] {other:?}"),
        }
        match &recs[4] {
            OwnedEventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => assert_eq!((*caller_fid, *caller_line), (2, 50)),
            other => panic!("[4] {other:?}"),
        }
        match &recs[5] {
            OwnedEventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (3, 8, 6, 10)),
            other => panic!("[5] {other:?}"),
        }
        match &recs[6] {
            OwnedEventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (3, 8, 6, 20)),
            other => panic!("[6] {other:?}"),
        }
        match &recs[7] {
            OwnedEventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!(
                (*fid, *line, *block_line, *ticks),
                (3, 9, 7, 3),
                "post TIME_BLOCK_RUN site-delta across chunk boundary"
            ),
            other => panic!("[7] {other:?}"),
        }
    }

    #[test]
    fn multi_chunk_packing_with_time_runs_none_zlib_zstd_lz4_always_inflate() {
        let specs = multi_chunk_packing_with_runs_specs();
        let single_wire = encode_decoded_event_profile_with_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &specs,
            0,
            None,
        )
        .expect("single-chunk packing+run encode");
        let (single_prof, _) = decode_decoded_event_profile(&single_wire, true).unwrap();
        assert_multi_chunk_packing_with_runs_owned(&single_prof.records, &single_prof.sequences);

        let parts = partition_event_records(&specs, 2);
        assert!(parts.len() >= 3, "expected multi-chunk, got {}", parts.len());
        assert!(
            matches!(parts[0].last(), Some(EventRecordSpec::TimeLineRun { .. })),
            "part0 ends with TIME_LINE_RUN so post-run site-delta is in a later chunk"
        );

        // Wire plains join equals single-chunk packing body.
        let single_plain =
            crate::event_body::encode_event_body_with_site_deltas_and_seq(&specs).unwrap();
        let mut packing = PackingEncodeState::new();
        let mut joined = Vec::new();
        for part in &parts {
            joined.extend_from_slice(
                &encode_event_body_with_site_deltas_and_seq_continuing(part, &mut packing).unwrap(),
            );
        }
        assert_eq!(joined, single_plain);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile_with_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &specs,
                2,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_chunk_count, parts.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_multi_chunk_packing_with_runs_owned(&prof.records, &prof.sequences);
            assert_eq!(prof.records, single_prof.records, "codec {c}");
            assert_eq!(prof.sequences, single_prof.sequences, "codec {c}");

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

    /// Triple compose: FOOTER string-dict + multi-chunk packing + TIME_*_RUN
    /// (post-run site-delta across chunk boundary + resolved strings).
    fn string_dict_multi_chunk_packing_with_runs_specs() -> [EventRecordSpec<'static>; 8] {
        const TL: &[u64] = &[7, 8];
        const TB: &[u64] = &[10, 20];
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
            // Chunk boundary (max=2): post-run site-delta in later partition.
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
        ]
    }

    fn assert_string_dict_multi_chunk_packing_with_runs(
        recs: &[OwnedEventRecord],
        sequences: &[Option<u64>],
    ) {
        // 1 + 2 expanded + 1 + 1 + 2 expanded + 1 + Mark + Comment = 10 logical
        assert_eq!(recs.len(), 10);
        assert_eq!(sequences.len(), 10);
        for (i, s) in sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
        match &recs[0] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("[0] {other:?}"),
        }
        match &recs[3] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!(
                    (*fid, *line, *ticks),
                    (2, 51, 9),
                    "post TIME_LINE_RUN site-delta across chunk boundary"
                );
            }
            other => panic!("[3] {other:?}"),
        }
        match &recs[7] {
            OwnedEventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!(
                (*fid, *line, *block_line, *ticks),
                (3, 9, 7, 3),
                "post TIME_BLOCK_RUN site-delta"
            ),
            other => panic!("[7] {other:?}"),
        }
        match &recs[8] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"dict-run-mc-mark"),
            other => panic!("[8] {other:?}"),
        }
        match &recs[9] {
            OwnedEventRecord::Comment { text } => assert_eq!(text, b"# dict-run-mc-end"),
            other => panic!("[9] {other:?}"),
        }
    }

    #[test]
    fn string_dict_multi_chunk_packing_with_time_runs_none_zlib_zstd_lz4_always_inflate() {
        use crate::string::FLAG_UTF8;
        let specs = string_dict_multi_chunk_packing_with_runs_specs();
        let dict_entries: &[(u64, u8, &[u8])] = &[
            (1, FLAG_UTF8, b"dict-run-mc-mark"),
            (2, 0, b"# dict-run-mc-end"),
        ];

        let single_wire = encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &specs,
            0,
            dict_entries,
        )
        .expect("single-chunk dict+packing+run");
        let (single_prof, single_dict, _) =
            decode_decoded_event_profile_with_string_dict(&single_wire, true).unwrap();
        assert_string_dict_multi_chunk_packing_with_runs(
            &single_prof.records,
            &single_prof.sequences,
        );
        assert_eq!(single_dict.get(1).unwrap().data, b"dict-run-mc-mark");
        assert_eq!(single_dict.get(2).unwrap().data, b"# dict-run-mc-end");

        let parts = partition_event_records(&specs, 2);
        assert!(parts.len() >= 3, "expected multi-chunk, got {}", parts.len());
        assert!(
            matches!(parts[0].last(), Some(EventRecordSpec::TimeLineRun { .. })),
            "part0 ends with TIME_LINE_RUN so post-run site-delta is later chunk"
        );
        assert!(parts.iter().all(|p| p.len() <= 2));

        // Wire packing plains join equals single-chunk packing body (dict is FOOTER).
        let single_plain =
            crate::event_body::encode_event_body_with_site_deltas_and_seq(&specs).unwrap();
        let mut packing = PackingEncodeState::new();
        let mut joined = Vec::new();
        for part in &parts {
            joined.extend_from_slice(
                &encode_event_body_with_site_deltas_and_seq_continuing(part, &mut packing).unwrap(),
            );
        }
        assert_eq!(joined, single_plain);

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                0,
                0,
                0,
                0,
                &[],
                c,
                &specs,
                2, // multi-chunk packing continuity + FOOTER dict + runs
                dict_entries,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, dict, n) = decode_decoded_event_profile_with_string_dict(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_chunk_count, parts.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(dict.len(), 2, "codec {c}");
            assert_eq!(dict.get(1).unwrap().data, b"dict-run-mc-mark");
            assert_eq!(dict.get(2).unwrap().data, b"# dict-run-mc-end");
            assert_string_dict_multi_chunk_packing_with_runs(&prof.records, &prof.sequences);
            assert_eq!(prof.records, single_prof.records, "codec {c}");
            assert_eq!(prof.sequences, single_prof.sequences, "codec {c}");

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

    /// Auto-VERSION + multi-chunk packing + TIME_*_RUN (post-run site-delta across chunks).
    fn auto_version_multi_chunk_packing_with_runs_specs() -> [EventRecordSpec<'static>; 6] {
        const TL: &[u64] = &[7, 8];
        const TB: &[u64] = &[10, 20];
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
        ]
    }

    #[test]
    fn auto_version_multi_chunk_packing_with_time_runs_none_zlib_zstd_lz4_always_inflate() {
        let workload = auto_version_multi_chunk_packing_with_runs_specs();
        let header_minor = 3u16;
        // Single-chunk packing baseline with auto-VERSION inject.
        let single_wire = encode_decoded_event_profile_auto_version_with_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            header_minor,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &workload,
            0,
            None,
        )
        .expect("single-chunk auto-version packing");
        let (single_prof, _) =
            decode_decoded_event_profile_auto_version(&single_wire, true).unwrap();
        // VERSION + 8 expanded logical events from packing with runs
        assert!(single_prof.records.len() >= 9);
        match &single_prof.records[0] {
            OwnedEventRecord::Version { major, minor } => {
                assert_eq!(
                    (*major, *minor),
                    (u64::from(SUPPORTED_MAJOR), u64::from(header_minor))
                );
            }
            other => panic!("expected leading VERSION, got {other:?}"),
        }
        // Post TIME_LINE_RUN site-delta is index 1+1+2 = 4 (VERSION, TL, TLR×2, TL@51)
        match &single_prof.records[4] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("post-run [4] {other:?}"),
        }
        // Sequences: VERSION may be Some(0) if packed with seq, then 1..
        assert_eq!(single_prof.sequences.len(), single_prof.records.len());
        assert_eq!(single_prof.sequences[0], Some(0)); // VERSION gets seq on packing path

        // After auto-VERSION inject: [VERSION, TL, TLR, TL@51, ...]
        // max=1 → run and post-run site-delta in different EVENT chunks.
        let with_ver: Vec<EventRecordSpec<'static>> = std::iter::once(EventRecordSpec::Version {
            major: u64::from(SUPPORTED_MAJOR),
            minor: u64::from(header_minor),
        })
        .chain(workload.iter().copied())
        .collect();
        let parts_sep = partition_event_records(&with_ver, 1);
        assert!(parts_sep.len() >= 4);
        assert!(matches!(
            parts_sep[2].first(),
            Some(EventRecordSpec::TimeLineRun { .. })
        ));
        assert!(matches!(
            parts_sep[3].first(),
            Some(EventRecordSpec::TimeLine {
                fid: 2,
                line: 51,
                ..
            })
        ));

        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            // max=1 forces post-run site-delta into a later EVENT chunk than the run.
            let wire = encode_decoded_event_profile_auto_version_with_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                header_minor,
                0,
                0,
                0,
                &[],
                c,
                &workload,
                1,
                None,
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, n) = decode_decoded_event_profile_auto_version(&wire, true)
                .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert_eq!(
                prof.event_chunk_count,
                parts_sep.len(),
                "codec {c}: multi-chunk"
            );
            assert_eq!(prof.records, single_prof.records, "codec {c}");
            assert_eq!(prof.sequences, single_prof.sequences, "codec {c}");
            match &prof.records[4] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!(
                        (*fid, *line, *ticks),
                        (2, 51, 9),
                        "codec {c}: post-run site-delta across chunk"
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
    fn auto_version_multi_chunk_packing_mismatch_fail_closed() {
        let workload = auto_version_multi_chunk_packing_with_runs_specs();
        let mut with_bad = vec![EventRecordSpec::Version {
            major: u64::from(SUPPORTED_MAJOR),
            minor: 99,
        }];
        with_bad.extend_from_slice(&workload);
        match encode_decoded_event_profile_auto_version_with_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &with_bad,
            2,
            None,
        ) {
            Err(DecodedEventError::VersionHeaderMismatch {
                header_minor: 0,
                body_minor: 99,
                ..
            }) => {}
            other => panic!("expected VersionHeaderMismatch, got {other:?}"),
        }
    }

    #[test]
    fn string_dict_multi_chunk_packing_with_time_runs_unknown_id_fail_closed() {
        const TL: &[u64] = &[1, 2];
        let specs = [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 1,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 5,
                ticks: TL,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 6,
                ticks: 3,
            },
            EventRecordSpec::Mark {
                string_id: 99,
                string_flags: 0,
                label: b"",
            },
        ];
        assert!(partition_event_records(&specs, 2).len() >= 2);
        let wire = encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &specs,
            2,
            &[(1, 0, b"other")],
        )
        .expect("encode");
        match decode_decoded_event_profile_with_string_dict(&wire, true) {
            Err(DecodedEventError::StringDict(crate::string_dict::StringDictError::UnknownId {
                id: 99,
            })) => {}
            other => panic!("expected UnknownId 99, got {other:?}"),
        }
    }

    /// Auto-VERSION + FOOTER dict + multi-chunk packing + TIME_*_RUN.
    fn auto_version_dict_multi_chunk_packing_with_runs_specs() -> [EventRecordSpec<'static>; 8] {
        const TL: &[u64] = &[7, 8];
        const TB: &[u64] = &[10, 20];
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
        ]
    }

    #[test]
    fn auto_version_dict_multi_chunk_packing_with_time_runs_none_zlib_zstd_lz4_always_inflate() {
        use crate::string::FLAG_UTF8;
        let workload = auto_version_dict_multi_chunk_packing_with_runs_specs();
        let header_minor = 4u16;
        let dict_entries: &[(u64, u8, &[u8])] = &[
            (1, FLAG_UTF8, b"av-dict-mc-mark"),
            (2, 0, b"# av-dict-mc-end"),
        ];

        let single_wire = encode_decoded_event_profile_auto_version_with_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            header_minor,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &workload,
            0,
            Some(dict_entries),
        )
        .expect("single-chunk auto-version dict packing");
        let (single_prof, single_dict, _) =
            decode_decoded_event_profile_auto_version_with_string_dict(&single_wire, true)
                .unwrap();
        assert_eq!(single_dict.get(1).unwrap().data, b"av-dict-mc-mark");
        assert_eq!(single_dict.get(2).unwrap().data, b"# av-dict-mc-end");
        match &single_prof.records[0] {
            OwnedEventRecord::Version { major, minor } => {
                assert_eq!(
                    (*major, *minor),
                    (u64::from(SUPPORTED_MAJOR), u64::from(header_minor))
                );
            }
            other => panic!("expected VERSION, got {other:?}"),
        }
        // VERSION + TL + TLR×2 + TL@51 = index 4
        match &single_prof.records[4] {
            OwnedEventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("post-run [4] {other:?}"),
        }
        let n = single_prof.records.len();
        match &single_prof.records[n - 2] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"av-dict-mc-mark"),
            other => panic!("mark {other:?}"),
        }
        match &single_prof.records[n - 1] {
            OwnedEventRecord::Comment { text } => assert_eq!(text, b"# av-dict-mc-end"),
            other => panic!("comment {other:?}"),
        }

        // max=1 → VERSION alone in chunk0; run and post-run in different chunks.
        for c in [codec::NONE, codec::ZLIB, codec::ZSTD, codec::LZ4] {
            let wire = encode_decoded_event_profile_auto_version_with_site_deltas_and_seq(
                SUPPORTED_MAJOR,
                header_minor,
                0,
                0,
                0,
                &[],
                c,
                &workload,
                1,
                Some(dict_entries),
            )
            .unwrap_or_else(|e| panic!("encode codec {c}: {e}"));
            let (prof, dict, n_read) =
                decode_decoded_event_profile_auto_version_with_string_dict(&wire, true)
                    .unwrap_or_else(|e| panic!("decode codec {c}: {e}"));
            assert_eq!(n_read, wire.len(), "codec {c}");
            assert_eq!(prof.event_codec, c);
            assert!(prof.event_chunk_count >= 4, "codec {c}");
            assert_eq!(dict.get(1).unwrap().data, b"av-dict-mc-mark");
            assert_eq!(dict.get(2).unwrap().data, b"# av-dict-mc-end");
            assert_eq!(prof.records, single_prof.records, "codec {c}");
            assert_eq!(prof.sequences, single_prof.sequences, "codec {c}");
            match &prof.records[4] {
                OwnedEventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!(
                        (*fid, *line, *ticks),
                        (2, 51, 9),
                        "codec {c}: post-run site-delta across chunk"
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
    fn auto_version_dict_multi_chunk_packing_version_mismatch_fail_closed() {
        let workload = auto_version_dict_multi_chunk_packing_with_runs_specs();
        let mut with_bad = vec![EventRecordSpec::Version {
            major: u64::from(SUPPORTED_MAJOR),
            minor: 77,
        }];
        with_bad.extend_from_slice(&workload);
        match encode_decoded_event_profile_auto_version_with_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &with_bad,
            2,
            Some(&[(1, 0, b"x")]),
        ) {
            Err(DecodedEventError::VersionHeaderMismatch {
                body_minor: 77, ..
            }) => {}
            other => panic!("expected VersionHeaderMismatch, got {other:?}"),
        }
    }

    #[test]
    fn auto_version_dict_multi_chunk_packing_unknown_id_fail_closed() {
        const TL: &[u64] = &[1, 2];
        let specs = [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 1,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 5,
                ticks: TL,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 6,
                ticks: 3,
            },
            EventRecordSpec::Mark {
                string_id: 99,
                string_flags: 0,
                label: b"",
            },
        ];
        let wire = encode_decoded_event_profile_auto_version_with_site_deltas_and_seq(
            SUPPORTED_MAJOR,
            0,
            0,
            0,
            0,
            &[],
            codec::NONE,
            &specs,
            1,
            Some(&[(1, 0, b"other")]),
        )
        .expect("encode");
        match decode_decoded_event_profile_auto_version_with_string_dict(&wire, true) {
            Err(DecodedEventError::StringDict(crate::string_dict::StringDictError::UnknownId {
                id: 99,
            })) => {}
            other => panic!("expected UnknownId 99, got {other:?}"),
        }
    }
}

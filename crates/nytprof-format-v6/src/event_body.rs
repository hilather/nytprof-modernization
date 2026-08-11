//! Provisional **format v6** event-body opcode codec (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-event-body-provisional-v0.md`
//!
//! Codec **NONE** chunk payloads: ordered records with ULEB128 opcodes + typed
//! fields composed from shipped varint / string-blob primitives.
//! Does **not** inflate zlib/zstd/LZ4, implement full v5 tag parity, or the C writer.

use crate::string::{decode_string_blob, encode_string_blob, StringBlob, StringError};
use crate::varint::{decode_u64, encode_u64, VarintError};

/// Fail-closed upper bound on total event-body size (64 MiB).
pub const MAX_EVENT_BODY_BYTES: usize = 64 * 1024 * 1024;

/// Flag: unknown opcode must fail closed (required opcode).
pub const FLAG_OPCODE_REQUIRED: u8 = 0x01;

/// Flag: typed body is length-framed (`ULEB128 body_len || body_len bytes`).
///
/// Provisional preflight for **unknown optional** opcode skip. Known opcodes use
/// their fixed typed layouts and ignore this bit. Not a permanent wire freeze of
/// the flag space (future ADR may reassign bits).
pub const FLAG_BODY_LENGTH: u8 = 0x02;

/// Flag: TIME_LINE / TIME_BLOCK / SUB_ENTRY site fields are **signed deltas**
/// (ZigZag+ULEB) relative to a running base, not absolute ULEB sites.
///
/// Provisional packing ID lockfile value (ADR-0001 intent). Full packing encode /
/// decode is residual until packing preflight / COL-007; not a wire freeze.
pub const FLAG_SITE_DELTA: u8 = 0x04;

/// Flag: record carries a provisional **logical event sequence number** (ULEB128)
/// immediately after the flags byte and before the typed body.
///
/// Provisional packing / OI-001-03 ID lockfile value (ADR-0001 intent). Not a
/// permanent dual-output sequence policy freeze.
pub const FLAG_HAS_SEQ: u8 = 0x08;

/// Fail-closed upper bound on a single length-framed unknown body (same as event-body cap).
pub const MAX_SKIP_BODY_BYTES: usize = MAX_EVENT_BODY_BYTES;

/// Fail-closed upper bound on TIME_LINE_RUN / TIME_BLOCK_RUN packed length `N`.
///
/// Provisional packing cap (ADR-0001 / lockfile). Checked before expand when run
/// codecs ship; value reserved for COL-007 implementers.
pub const MAX_TIME_RUN_LEN: usize = 1_048_576;

/// Provisional event opcodes.
pub mod opcode {
    /// Reserved — always fail closed.
    pub const RESERVED: u64 = 0;
    /// Metadata mark: body is a length-prefixed string/blob.
    pub const MARK: u64 = 1;
    /// Timing-like sample: fid, line, ticks as three ULEB128 u64 fields.
    pub const TIME_LINE: u64 = 2;
    /// Block timing sample: fid, line, block_line, ticks as four ULEB128 u64 fields.
    pub const TIME_BLOCK: u64 = 3;
    /// Sub entry sample: caller_fid, caller_line as two ULEB128 u64 fields.
    pub const SUB_ENTRY: u64 = 4;
    /// Sub return sample: depth, incl, excl ticks + subname string-blob.
    pub const SUB_RETURN: u64 = 5;
    /// Sub info sample: fid, first_line, last_line + name string-blob.
    pub const SUB_INFO: u64 = 6;
    /// Source line sample: fid, line + text string-blob.
    pub const SRC_LINE: u64 = 7;
    /// New file id sample: fid + filename string-blob.
    pub const NEW_FID: u64 = 8;
    /// Process start: pid, ppid, start_time as three ULEB128 u64 fields.
    pub const PID_START: u64 = 9;
    /// Process end: pid, end_time as two ULEB128 u64 fields.
    pub const PID_END: u64 = 10;
    /// Sub callers edge: site + counts/times + called/caller name blobs.
    pub const SUB_CALLERS: u64 = 11;
    /// Overhead discount marker: empty typed body (opcode + flags only).
    pub const DISCOUNT: u64 = 12;
    /// Profile attribute: key + value string-blobs.
    pub const ATTRIBUTE: u64 = 13;
    /// Profile option: key + value string-blobs.
    pub const OPTION: u64 = 14;
    /// Free-form comment: text string-blob.
    pub const COMMENT: u64 = 15;
    /// Stream control: start deflate / compressed payload region marker (empty typed body).
    pub const START_DEFLATE: u64 = 16;
    /// Profile format version: major + minor as two ULEB128 u64 fields.
    pub const VERSION: u64 = 17;
    /// Packed run of consecutive same-site TIME_LINE events (ADR-0001 packing).
    ///
    /// Provisional ID lockfile value — encode/decode residual until packing
    /// preflight / COL-007. Not a wire freeze.
    pub const TIME_LINE_RUN: u64 = 18;
    /// Packed run of consecutive same-site TIME_BLOCK events (ADR-0001 packing).
    ///
    /// Provisional ID lockfile value — encode/decode residual until packing
    /// preflight / COL-007. Not a wire freeze.
    pub const TIME_BLOCK_RUN: u64 = 19;
}

/// True if `opcode` is a known provisional type (excludes RESERVED).
pub fn is_known_opcode(opcode: u64) -> bool {
    matches!(
        opcode,
        opcode::MARK
            | opcode::TIME_LINE
            | opcode::TIME_BLOCK
            | opcode::SUB_ENTRY
            | opcode::SUB_RETURN
            | opcode::SUB_INFO
            | opcode::SRC_LINE
            | opcode::NEW_FID
            | opcode::PID_START
            | opcode::PID_END
            | opcode::SUB_CALLERS
            | opcode::DISCOUNT
            | opcode::ATTRIBUTE
            | opcode::OPTION
            | opcode::COMMENT
            | opcode::START_DEFLATE
            | opcode::VERSION
    )
}

/// Provisional ATTRIBUTE / OPTION **known-key** vocabulary (OI-002-03/04 runway).
///
/// Keys are dump/JSON-surface aligned (`basetime`, `ticks_per_sec`, `calls`, …).
/// This is **not** a complete writer inventory freeze — unknown string keys may still
/// encode/decode as free-form projections; the table documents keys exercised by
/// shipped preflight tests and dual-path JSON samples.
///
/// Schema: `docs/schemas/v6-attr-option-known-key-provisional-v0.md`
pub mod known_key {
    /// ATTRIBUTE `basetime` (dump sample / JSON-ATTR-BASETIME-MVP).
    pub const BASETIME: &[u8] = b"basetime";
    /// ATTRIBUTE `ticks_per_sec` (JSON-META-FILES-MVP).
    pub const TICKS_PER_SEC: &[u8] = b"ticks_per_sec";
    /// ATTRIBUTE `application` (COMPAT-002 volatile; basename normalize residual).
    pub const APPLICATION: &[u8] = b"application";
    /// ATTRIBUTE `xs_version` (dump meta sample).
    pub const XS_VERSION: &[u8] = b"xs_version";

    /// OPTION `calls` (default-calls1 multiplicity surface).
    pub const CALLS: &[u8] = b"calls";
    /// OPTION `blocks`.
    pub const BLOCKS: &[u8] = b"blocks";
    /// OPTION `stmts`.
    pub const STMTS: &[u8] = b"stmts";
    /// OPTION `compress`.
    pub const COMPRESS: &[u8] = b"compress";

    /// Provisional known ATTRIBUTE keys (OI-002-03 runway — not exhaustive freeze).
    pub const KNOWN_ATTRIBUTE_KEYS: &[&[u8]] =
        &[BASETIME, TICKS_PER_SEC, APPLICATION, XS_VERSION];

    /// Provisional known OPTION keys (OI-002-04 runway — not exhaustive freeze).
    pub const KNOWN_OPTION_KEYS: &[&[u8]] = &[CALLS, BLOCKS, STMTS, COMPRESS];

    /// True if `key` is in the provisional ATTRIBUTE known-key set.
    pub fn is_known_attribute_key(key: &[u8]) -> bool {
        KNOWN_ATTRIBUTE_KEYS.iter().any(|k| *k == key)
    }

    /// True if `key` is in the provisional OPTION known-key set.
    pub fn is_known_option_key(key: &[u8]) -> bool {
        KNOWN_OPTION_KEYS.iter().any(|k| *k == key)
    }

    /// True if `key` is known as either ATTRIBUTE or OPTION key.
    pub fn is_known_meta_key(key: &[u8]) -> bool {
        is_known_attribute_key(key) || is_known_option_key(key)
    }
}

/// Build an ATTRIBUTE record for a provisional known (or free-form) key/value pair.
///
/// String-blob ids default to 0; flags default to 0. Callers that need UTF-8 flags
/// can use [`EventRecordSpec::Attribute`] directly.
#[inline]
pub fn attribute_kv<'a>(key: &'a [u8], value: &'a [u8]) -> EventRecordSpec<'a> {
    EventRecordSpec::Attribute {
        key_string_id: 0,
        key_string_flags: 0,
        key,
        value_string_id: 0,
        value_string_flags: 0,
        value,
    }
}

/// Build an OPTION record for a provisional known (or free-form) key/value pair.
#[inline]
pub fn option_kv<'a>(key: &'a [u8], value: &'a [u8]) -> EventRecordSpec<'a> {
    EventRecordSpec::Option {
        key_string_id: 0,
        key_string_flags: 0,
        key,
        value_string_id: 0,
        value_string_flags: 0,
        value,
    }
}

/// Representative dump-aligned known-key ATTRIBUTE/OPTION body for preflight tests.
///
/// Order: ATTRIBUTE basetime, ticks_per_sec, application; OPTION calls, blocks.
/// Values are fixture-shaped string projections (not live wall-clock).
pub fn known_key_attr_option_sample_specs() -> [EventRecordSpec<'static>; 5] {
    [
        attribute_kv(known_key::BASETIME, b"1786111723"),
        attribute_kv(known_key::TICKS_PER_SEC, b"10000000"),
        attribute_kv(known_key::APPLICATION, b"workload.pl"),
        option_kv(known_key::CALLS, b"1"),
        option_kv(known_key::BLOCKS, b"0"),
    ]
}

/// One decoded event-body record (payloads borrowed from input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventRecord<'a> {
    /// `opcode::MARK` — string/blob label (id/flags from string-blob frame).
    Mark { label: StringBlob<'a> },
    /// `opcode::TIME_LINE` — fid / line / ticks.
    TimeLine { fid: u64, line: u64, ticks: u64 },
    /// `opcode::TIME_BLOCK` — fid / line / block_line / ticks.
    TimeBlock {
        fid: u64,
        line: u64,
        block_line: u64,
        ticks: u64,
    },
    /// `opcode::SUB_ENTRY` — caller_fid / caller_line.
    SubEntry { caller_fid: u64, caller_line: u64 },
    /// `opcode::SUB_RETURN` — depth / incl / excl / subname.
    SubReturn {
        depth: u64,
        incl: u64,
        excl: u64,
        subname: StringBlob<'a>,
    },
    /// `opcode::SUB_INFO` — fid / first_line / last_line / name.
    SubInfo {
        fid: u64,
        first_line: u64,
        last_line: u64,
        name: StringBlob<'a>,
    },
    /// `opcode::SRC_LINE` — fid / line / text.
    SrcLine {
        fid: u64,
        line: u64,
        text: StringBlob<'a>,
    },
    /// `opcode::NEW_FID` — fid / filename.
    NewFid {
        fid: u64,
        filename: StringBlob<'a>,
    },
    /// `opcode::PID_START` — pid / ppid / start_time.
    PidStart {
        pid: u64,
        ppid: u64,
        start_time: u64,
    },
    /// `opcode::PID_END` — pid / end_time.
    PidEnd { pid: u64, end_time: u64 },
    /// `opcode::SUB_CALLERS` — call edge at fid/line with counts and names.
    SubCallers {
        fid: u64,
        line: u64,
        count: u64,
        incl: u64,
        excl: u64,
        reci: u64,
        rec_depth: u64,
        called: StringBlob<'a>,
        caller: StringBlob<'a>,
    },
    /// `opcode::DISCOUNT` — no typed body fields.
    Discount,
    /// `opcode::ATTRIBUTE` — key / value string-blobs.
    Attribute {
        key: StringBlob<'a>,
        value: StringBlob<'a>,
    },
    /// `opcode::OPTION` — key / value string-blobs.
    Option {
        key: StringBlob<'a>,
        value: StringBlob<'a>,
    },
    /// `opcode::COMMENT` — free-form text.
    Comment { text: StringBlob<'a> },
    /// `opcode::START_DEFLATE` — empty typed body (marker only).
    StartDeflate,
    /// `opcode::VERSION` — major / minor.
    Version { major: u64, minor: u64 },
}

/// Spec for encoding one event-body record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventRecordSpec<'a> {
    /// MARK with string-blob fields (composes `encode_string_blob`).
    Mark {
        string_id: u64,
        string_flags: u8,
        label: &'a [u8],
    },
    /// TIME_LINE sample.
    TimeLine { fid: u64, line: u64, ticks: u64 },
    /// TIME_BLOCK sample (provisional; not full v5 freeze).
    TimeBlock {
        fid: u64,
        line: u64,
        block_line: u64,
        ticks: u64,
    },
    /// SUB_ENTRY sample (provisional; caller site only).
    SubEntry { caller_fid: u64, caller_line: u64 },
    /// SUB_RETURN sample (provisional integer ticks; not float/NV freeze).
    SubReturn {
        depth: u64,
        incl: u64,
        excl: u64,
        string_id: u64,
        string_flags: u8,
        subname: &'a [u8],
    },
    /// SUB_INFO sample (provisional; not full catalog freeze).
    SubInfo {
        fid: u64,
        first_line: u64,
        last_line: u64,
        string_id: u64,
        string_flags: u8,
        name: &'a [u8],
    },
    /// SRC_LINE sample (provisional layout; not full COMPAT-001 freeze).
    SrcLine {
        fid: u64,
        line: u64,
        string_id: u64,
        string_flags: u8,
        text: &'a [u8],
    },
    /// NEW_FID sample (provisional layout; not full COMPAT-001 freeze).
    NewFid {
        fid: u64,
        string_id: u64,
        string_flags: u8,
        filename: &'a [u8],
    },
    /// PID_START sample (provisional integer times; not float/NV or COL-015 freeze).
    PidStart {
        pid: u64,
        ppid: u64,
        start_time: u64,
    },
    /// PID_END sample (provisional integer times; not float/NV or COL-015 freeze).
    PidEnd { pid: u64, end_time: u64 },
    /// SUB_CALLERS sample (provisional integer times; not float/NV freeze).
    SubCallers {
        fid: u64,
        line: u64,
        count: u64,
        incl: u64,
        excl: u64,
        reci: u64,
        rec_depth: u64,
        called_string_id: u64,
        called_string_flags: u8,
        called: &'a [u8],
        caller_string_id: u64,
        caller_string_flags: u8,
        caller: &'a [u8],
    },
    /// DISCOUNT sample (empty typed body).
    Discount,
    /// ATTRIBUTE sample (provisional string projection; not key vocabulary freeze).
    Attribute {
        key_string_id: u64,
        key_string_flags: u8,
        key: &'a [u8],
        value_string_id: u64,
        value_string_flags: u8,
        value: &'a [u8],
    },
    /// OPTION sample (provisional string projection; not key vocabulary freeze).
    Option {
        key_string_id: u64,
        key_string_flags: u8,
        key: &'a [u8],
        value_string_id: u64,
        value_string_flags: u8,
        value: &'a [u8],
    },
    /// COMMENT sample (provisional string-blob; not COMPAT-002 volatile normalize freeze).
    Comment {
        string_id: u64,
        string_flags: u8,
        text: &'a [u8],
    },
    /// START_DEFLATE sample (empty typed body; not mid-stream codec switch).
    StartDeflate,
    /// VERSION sample (provisional dump-aligned major/minor; not OI-001-03 dual-output freeze).
    Version { major: u64, minor: u64 },
}

/// Fail-closed event-body errors (never panic on crafted input).
#[derive(Debug, PartialEq, Eq)]
pub enum EventBodyError {
    Varint(VarintError),
    String(StringError),
    Truncated { need: usize, got: usize },
    Oversize { len: usize },
    /// Opcode 0 is reserved.
    ReservedOpcode,
    /// Unknown opcode with `FLAG_OPCODE_REQUIRED` set.
    UnknownRequiredOpcode { opcode: u64 },
    /// Unknown optional opcode without [`FLAG_BODY_LENGTH`] — cannot skip safely.
    UnknownOpcode { opcode: u64 },
    /// Length-framed skip body exceeds [`MAX_SKIP_BODY_BYTES`].
    OversizeSkipBody { len: usize },
}

impl std::fmt::Display for EventBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventBodyError::Varint(e) => write!(f, "event-body varint: {e}"),
            EventBodyError::String(e) => write!(f, "event-body string: {e}"),
            EventBodyError::Truncated { need, got } => {
                write!(f, "truncated event-body: need {need} bytes, got {got}")
            }
            EventBodyError::Oversize { len } => {
                write!(
                    f,
                    "oversize event-body {len} bytes (max {MAX_EVENT_BODY_BYTES})"
                )
            }
            EventBodyError::ReservedOpcode => write!(f, "reserved event opcode 0"),
            EventBodyError::UnknownRequiredOpcode { opcode } => {
                write!(f, "unknown required event opcode {opcode}")
            }
            EventBodyError::UnknownOpcode { opcode } => {
                write!(
                    f,
                    "unknown event opcode {opcode} (optional but not length-framed for skip)"
                )
            }
            EventBodyError::OversizeSkipBody { len } => {
                write!(
                    f,
                    "oversize length-framed skip body {len} bytes (max {MAX_SKIP_BODY_BYTES})"
                )
            }
        }
    }
}

impl std::error::Error for EventBodyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EventBodyError::Varint(e) => Some(e),
            EventBodyError::String(e) => Some(e),
            _ => None,
        }
    }
}

impl From<VarintError> for EventBodyError {
    fn from(e: VarintError) -> Self {
        EventBodyError::Varint(e)
    }
}

impl From<StringError> for EventBodyError {
    fn from(e: StringError) -> Self {
        EventBodyError::String(e)
    }
}

pub type EventBodyResult<T> = std::result::Result<T, EventBodyError>;

/// Encode one record (opcode ULEB + flags + typed body) into `out`.
fn encode_record_into(out: &mut Vec<u8>, rec: &EventRecordSpec<'_>) {
    match rec {
        EventRecordSpec::Mark {
            string_id,
            string_flags,
            label,
        } => {
            out.extend_from_slice(&encode_u64(opcode::MARK));
            out.push(0); // flags
            out.extend_from_slice(&encode_string_blob(*string_id, *string_flags, label));
        }
        EventRecordSpec::TimeLine { fid, line, ticks } => {
            out.extend_from_slice(&encode_u64(opcode::TIME_LINE));
            out.push(0); // flags
            out.extend_from_slice(&encode_u64(*fid));
            out.extend_from_slice(&encode_u64(*line));
            out.extend_from_slice(&encode_u64(*ticks));
        }
        EventRecordSpec::TimeBlock {
            fid,
            line,
            block_line,
            ticks,
        } => {
            out.extend_from_slice(&encode_u64(opcode::TIME_BLOCK));
            out.push(0); // flags
            out.extend_from_slice(&encode_u64(*fid));
            out.extend_from_slice(&encode_u64(*line));
            out.extend_from_slice(&encode_u64(*block_line));
            out.extend_from_slice(&encode_u64(*ticks));
        }
        EventRecordSpec::SubEntry {
            caller_fid,
            caller_line,
        } => {
            out.extend_from_slice(&encode_u64(opcode::SUB_ENTRY));
            out.push(0); // flags
            out.extend_from_slice(&encode_u64(*caller_fid));
            out.extend_from_slice(&encode_u64(*caller_line));
        }
        EventRecordSpec::SubReturn {
            depth,
            incl,
            excl,
            string_id,
            string_flags,
            subname,
        } => {
            out.extend_from_slice(&encode_u64(opcode::SUB_RETURN));
            out.push(0); // flags
            out.extend_from_slice(&encode_u64(*depth));
            out.extend_from_slice(&encode_u64(*incl));
            out.extend_from_slice(&encode_u64(*excl));
            out.extend_from_slice(&encode_string_blob(*string_id, *string_flags, subname));
        }
        EventRecordSpec::SubInfo {
            fid,
            first_line,
            last_line,
            string_id,
            string_flags,
            name,
        } => {
            out.extend_from_slice(&encode_u64(opcode::SUB_INFO));
            out.push(0); // flags
            out.extend_from_slice(&encode_u64(*fid));
            out.extend_from_slice(&encode_u64(*first_line));
            out.extend_from_slice(&encode_u64(*last_line));
            out.extend_from_slice(&encode_string_blob(*string_id, *string_flags, name));
        }
        EventRecordSpec::SrcLine {
            fid,
            line,
            string_id,
            string_flags,
            text,
        } => {
            out.extend_from_slice(&encode_u64(opcode::SRC_LINE));
            out.push(0); // flags
            out.extend_from_slice(&encode_u64(*fid));
            out.extend_from_slice(&encode_u64(*line));
            out.extend_from_slice(&encode_string_blob(*string_id, *string_flags, text));
        }
        EventRecordSpec::NewFid {
            fid,
            string_id,
            string_flags,
            filename,
        } => {
            out.extend_from_slice(&encode_u64(opcode::NEW_FID));
            out.push(0); // flags
            out.extend_from_slice(&encode_u64(*fid));
            out.extend_from_slice(&encode_string_blob(*string_id, *string_flags, filename));
        }
        EventRecordSpec::PidStart {
            pid,
            ppid,
            start_time,
        } => {
            out.extend_from_slice(&encode_u64(opcode::PID_START));
            out.push(0); // flags
            out.extend_from_slice(&encode_u64(*pid));
            out.extend_from_slice(&encode_u64(*ppid));
            out.extend_from_slice(&encode_u64(*start_time));
        }
        EventRecordSpec::PidEnd { pid, end_time } => {
            out.extend_from_slice(&encode_u64(opcode::PID_END));
            out.push(0); // flags
            out.extend_from_slice(&encode_u64(*pid));
            out.extend_from_slice(&encode_u64(*end_time));
        }
        EventRecordSpec::SubCallers {
            fid,
            line,
            count,
            incl,
            excl,
            reci,
            rec_depth,
            called_string_id,
            called_string_flags,
            called,
            caller_string_id,
            caller_string_flags,
            caller,
        } => {
            out.extend_from_slice(&encode_u64(opcode::SUB_CALLERS));
            out.push(0); // flags
            out.extend_from_slice(&encode_u64(*fid));
            out.extend_from_slice(&encode_u64(*line));
            out.extend_from_slice(&encode_u64(*count));
            out.extend_from_slice(&encode_u64(*incl));
            out.extend_from_slice(&encode_u64(*excl));
            out.extend_from_slice(&encode_u64(*reci));
            out.extend_from_slice(&encode_u64(*rec_depth));
            out.extend_from_slice(&encode_string_blob(
                *called_string_id,
                *called_string_flags,
                called,
            ));
            out.extend_from_slice(&encode_string_blob(
                *caller_string_id,
                *caller_string_flags,
                caller,
            ));
        }
        EventRecordSpec::Discount => {
            out.extend_from_slice(&encode_u64(opcode::DISCOUNT));
            out.push(0); // flags; empty typed body
        }
        EventRecordSpec::Attribute {
            key_string_id,
            key_string_flags,
            key,
            value_string_id,
            value_string_flags,
            value,
        } => {
            out.extend_from_slice(&encode_u64(opcode::ATTRIBUTE));
            out.push(0); // flags
            out.extend_from_slice(&encode_string_blob(
                *key_string_id,
                *key_string_flags,
                key,
            ));
            out.extend_from_slice(&encode_string_blob(
                *value_string_id,
                *value_string_flags,
                value,
            ));
        }
        EventRecordSpec::Option {
            key_string_id,
            key_string_flags,
            key,
            value_string_id,
            value_string_flags,
            value,
        } => {
            out.extend_from_slice(&encode_u64(opcode::OPTION));
            out.push(0); // flags
            out.extend_from_slice(&encode_string_blob(
                *key_string_id,
                *key_string_flags,
                key,
            ));
            out.extend_from_slice(&encode_string_blob(
                *value_string_id,
                *value_string_flags,
                value,
            ));
        }
        EventRecordSpec::Comment {
            string_id,
            string_flags,
            text,
        } => {
            out.extend_from_slice(&encode_u64(opcode::COMMENT));
            out.push(0); // flags
            out.extend_from_slice(&encode_string_blob(*string_id, *string_flags, text));
        }
        EventRecordSpec::StartDeflate => {
            out.extend_from_slice(&encode_u64(opcode::START_DEFLATE));
            out.push(0); // flags; empty typed body
        }
        EventRecordSpec::Version { major, minor } => {
            out.extend_from_slice(&encode_u64(opcode::VERSION));
            out.push(0); // flags
            out.extend_from_slice(&encode_u64(*major));
            out.extend_from_slice(&encode_u64(*minor));
        }
    }
}

/// Encode a provisional event-body (codec NONE payload): ordered records.
///
/// Empty `records` yields an empty body (valid). Pure byte-slice / `Vec` API.
pub fn encode_event_body(records: &[EventRecordSpec<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    for rec in records {
        encode_record_into(&mut out, rec);
    }
    out
}

/// Encode a provisional **unknown optional** length-framed record for skip preflight.
///
/// Wire: `ULEB128 opcode || u8 flags(FLAG_BODY_LENGTH) || ULEB128 body_len || body`.
/// `opcode` must not be reserved (0) or a known provisional opcode; required flag is
/// never set. Used by tests and fixture builders — not a permanent extension codec.
pub fn encode_unknown_optional_skip_record(opcode: u64, body: &[u8]) -> EventBodyResult<Vec<u8>> {
    if opcode == opcode::RESERVED {
        return Err(EventBodyError::ReservedOpcode);
    }
    if is_known_opcode(opcode) {
        return Err(EventBodyError::UnknownOpcode { opcode });
    }
    if body.len() > MAX_SKIP_BODY_BYTES {
        return Err(EventBodyError::OversizeSkipBody { len: body.len() });
    }
    let mut out = encode_u64(opcode);
    out.push(FLAG_BODY_LENGTH);
    out.extend_from_slice(&encode_u64(body.len() as u64));
    out.extend_from_slice(body);
    Ok(out)
}

/// Decode one record starting at `pos`.
///
/// Returns `(Some(record), bytes)` for known opcodes, or `(None, bytes)` when an
/// unknown **optional** opcode was length-framed and skipped. Fail-closed otherwise.
fn decode_record<'a>(
    data: &'a [u8],
    pos: usize,
) -> EventBodyResult<(Option<EventRecord<'a>>, usize)> {
    if pos >= data.len() {
        return Err(EventBodyError::Truncated {
            need: pos + 1,
            got: data.len(),
        });
    }
    let (op, n_op) = decode_u64(data, pos)?;
    let mut p = pos + n_op;

    // flags byte required after opcode
    if p >= data.len() {
        return Err(EventBodyError::Truncated {
            need: p + 1,
            got: data.len(),
        });
    }
    let flags = data[p];
    p += 1;

    if op == opcode::RESERVED {
        return Err(EventBodyError::ReservedOpcode);
    }

    if !is_known_opcode(op) {
        if (flags & FLAG_OPCODE_REQUIRED) != 0 {
            return Err(EventBodyError::UnknownRequiredOpcode { opcode: op });
        }
        // Provisional: optional unknown + FLAG_BODY_LENGTH → skip ULEB len + body.
        if (flags & FLAG_BODY_LENGTH) != 0 {
            let (body_len_u, n_len) = decode_u64(data, p)?;
            p += n_len;
            if body_len_u > MAX_SKIP_BODY_BYTES as u64 {
                return Err(EventBodyError::OversizeSkipBody {
                    len: body_len_u as usize,
                });
            }
            let body_len = body_len_u as usize;
            let end = p.checked_add(body_len).ok_or(EventBodyError::Truncated {
                need: usize::MAX,
                got: data.len(),
            })?;
            if end > data.len() {
                return Err(EventBodyError::Truncated {
                    need: end,
                    got: data.len(),
                });
            }
            p = end;
            return Ok((None, p - pos));
        }
        return Err(EventBodyError::UnknownOpcode { opcode: op });
    }

    match op {
        opcode::MARK => {
            let (label, n_str) = decode_string_blob(data, p)?;
            p += n_str;
            Ok((Some(EventRecord::Mark { label }), p - pos))
        }
        opcode::TIME_LINE => {
            let (fid, n1) = decode_u64(data, p)?;
            p += n1;
            let (line, n2) = decode_u64(data, p)?;
            p += n2;
            let (ticks, n3) = decode_u64(data, p)?;
            p += n3;
            Ok((Some(EventRecord::TimeLine { fid, line, ticks }), p - pos))
        }
        opcode::TIME_BLOCK => {
            let (fid, n1) = decode_u64(data, p)?;
            p += n1;
            let (line, n2) = decode_u64(data, p)?;
            p += n2;
            let (block_line, n3) = decode_u64(data, p)?;
            p += n3;
            let (ticks, n4) = decode_u64(data, p)?;
            p += n4;
            Ok((
                Some(EventRecord::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                }),
                p - pos,
            ))
        }
        opcode::SUB_ENTRY => {
            let (caller_fid, n1) = decode_u64(data, p)?;
            p += n1;
            let (caller_line, n2) = decode_u64(data, p)?;
            p += n2;
            Ok((
                Some(EventRecord::SubEntry {
                    caller_fid,
                    caller_line,
                }),
                p - pos,
            ))
        }
        opcode::SUB_RETURN => {
            let (depth, n1) = decode_u64(data, p)?;
            p += n1;
            let (incl, n2) = decode_u64(data, p)?;
            p += n2;
            let (excl, n3) = decode_u64(data, p)?;
            p += n3;
            let (subname, n_str) = decode_string_blob(data, p)?;
            p += n_str;
            Ok((
                Some(EventRecord::SubReturn {
                    depth,
                    incl,
                    excl,
                    subname,
                }),
                p - pos,
            ))
        }
        opcode::SUB_INFO => {
            let (fid, n1) = decode_u64(data, p)?;
            p += n1;
            let (first_line, n2) = decode_u64(data, p)?;
            p += n2;
            let (last_line, n3) = decode_u64(data, p)?;
            p += n3;
            let (name, n_str) = decode_string_blob(data, p)?;
            p += n_str;
            Ok((
                Some(EventRecord::SubInfo {
                    fid,
                    first_line,
                    last_line,
                    name,
                }),
                p - pos,
            ))
        }
        opcode::SRC_LINE => {
            let (fid, n1) = decode_u64(data, p)?;
            p += n1;
            let (line, n2) = decode_u64(data, p)?;
            p += n2;
            let (text, n_str) = decode_string_blob(data, p)?;
            p += n_str;
            Ok((Some(EventRecord::SrcLine { fid, line, text }), p - pos))
        }
        opcode::NEW_FID => {
            let (fid, n1) = decode_u64(data, p)?;
            p += n1;
            let (filename, n_str) = decode_string_blob(data, p)?;
            p += n_str;
            Ok((Some(EventRecord::NewFid { fid, filename }), p - pos))
        }
        opcode::PID_START => {
            let (pid, n1) = decode_u64(data, p)?;
            p += n1;
            let (ppid, n2) = decode_u64(data, p)?;
            p += n2;
            let (start_time, n3) = decode_u64(data, p)?;
            p += n3;
            Ok((
                Some(EventRecord::PidStart {
                    pid,
                    ppid,
                    start_time,
                }),
                p - pos,
            ))
        }
        opcode::PID_END => {
            let (pid, n1) = decode_u64(data, p)?;
            p += n1;
            let (end_time, n2) = decode_u64(data, p)?;
            p += n2;
            Ok((Some(EventRecord::PidEnd { pid, end_time }), p - pos))
        }
        opcode::SUB_CALLERS => {
            let (fid, n1) = decode_u64(data, p)?;
            p += n1;
            let (line, n2) = decode_u64(data, p)?;
            p += n2;
            let (count, n3) = decode_u64(data, p)?;
            p += n3;
            let (incl, n4) = decode_u64(data, p)?;
            p += n4;
            let (excl, n5) = decode_u64(data, p)?;
            p += n5;
            let (reci, n6) = decode_u64(data, p)?;
            p += n6;
            let (rec_depth, n7) = decode_u64(data, p)?;
            p += n7;
            let (called, n_called) = decode_string_blob(data, p)?;
            p += n_called;
            let (caller, n_caller) = decode_string_blob(data, p)?;
            p += n_caller;
            Ok((
                Some(EventRecord::SubCallers {
                    fid,
                    line,
                    count,
                    incl,
                    excl,
                    reci,
                    rec_depth,
                    called,
                    caller,
                }),
                p - pos,
            ))
        }
        opcode::DISCOUNT => Ok((Some(EventRecord::Discount), p - pos)),
        opcode::ATTRIBUTE => {
            let (key, n_key) = decode_string_blob(data, p)?;
            p += n_key;
            let (value, n_val) = decode_string_blob(data, p)?;
            p += n_val;
            Ok((Some(EventRecord::Attribute { key, value }), p - pos))
        }
        opcode::OPTION => {
            let (key, n_key) = decode_string_blob(data, p)?;
            p += n_key;
            let (value, n_val) = decode_string_blob(data, p)?;
            p += n_val;
            Ok((Some(EventRecord::Option { key, value }), p - pos))
        }
        opcode::COMMENT => {
            let (text, n_str) = decode_string_blob(data, p)?;
            p += n_str;
            Ok((Some(EventRecord::Comment { text }), p - pos))
        }
        opcode::START_DEFLATE => Ok((Some(EventRecord::StartDeflate), p - pos)),
        opcode::VERSION => {
            let (major, n1) = decode_u64(data, p)?;
            p += n1;
            let (minor, n2) = decode_u64(data, p)?;
            p += n2;
            Ok((Some(EventRecord::Version { major, minor }), p - pos))
        }
        _ => unreachable!("is_known_opcode filtered"),
    }
}

/// Decode a provisional event-body until the buffer is exhausted.
///
/// Empty input → empty record list. Fail-closed on truncated mid-record,
/// reserved opcode 0, unknown **required** opcodes, and unknown optional opcodes
/// without [`FLAG_BODY_LENGTH`]. Unknown optional opcodes with length framing are
/// **skipped** (not emitted). Returns `(records, bytes_consumed)`
/// (`bytes_consumed == data.len()` on success).
pub fn decode_event_body(data: &[u8]) -> EventBodyResult<(Vec<EventRecord<'_>>, usize)> {
    if data.len() > MAX_EVENT_BODY_BYTES {
        return Err(EventBodyError::Oversize { len: data.len() });
    }
    let mut pos = 0;
    let mut out = Vec::new();
    while pos < data.len() {
        if pos > MAX_EVENT_BODY_BYTES {
            return Err(EventBodyError::Oversize { len: pos });
        }
        let (rec, n) = decode_record(data, pos)?;
        pos += n;
        if let Some(r) = rec {
            out.push(r);
        }
    }
    Ok((out, pos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{codec, encode_chunk_frame, kind, parse_chunk_frame};
    use crate::string::FLAG_UTF8;

    #[test]
    fn empty_body_roundtrip() {
        let enc_a = encode_event_body(&[]);
        let enc_b = encode_event_body(&[]);
        assert_eq!(enc_a, enc_b);
        assert!(enc_a.is_empty());
        let (recs, n) = decode_event_body(&enc_a).expect("empty");
        assert_eq!(n, 0);
        assert!(recs.is_empty());
        // Dual decode stability.
        let (recs2, n2) = decode_event_body(&enc_a).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn mark_and_time_line_roundtrip() {
        let specs = [
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: FLAG_UTF8,
                label: b"main::leaf",
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 42,
            },
        ];
        let enc = encode_event_body(&specs);
        // Length must equal encode of parts (no detached golden).
        let mut expect = Vec::new();
        encode_record_into(&mut expect, &specs[0]);
        encode_record_into(&mut expect, &specs[1]);
        assert_eq!(enc, expect);

        let (recs, n) = decode_event_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 2);
        match &recs[0] {
            EventRecord::Mark { label } => {
                assert_eq!(label.id, 0);
                assert_eq!(label.flags, FLAG_UTF8);
                assert_eq!(label.data, b"main::leaf");
            }
            other => panic!("expected Mark, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!(*fid, 1);
                assert_eq!(*line, 5);
                assert_eq!(*ticks, 42);
            }
            other => panic!("expected TimeLine, got {other:?}"),
        }
    }

    #[test]
    fn reserved_opcode_zero_err() {
        // Craft opcode 0 + flags 0 manually via encode_u64.
        let mut bad = encode_u64(opcode::RESERVED);
        bad.push(0);
        assert_eq!(
            decode_event_body(&bad),
            Err(EventBodyError::ReservedOpcode)
        );
    }

    #[test]
    fn unknown_required_opcode_err() {
        let mut bad = encode_u64(99);
        bad.push(FLAG_OPCODE_REQUIRED);
        assert_eq!(
            decode_event_body(&bad),
            Err(EventBodyError::UnknownRequiredOpcode { opcode: 99 })
        );
    }

    #[test]
    fn unknown_optional_opcode_without_length_frame_still_err() {
        // Without FLAG_BODY_LENGTH, optional unknown cannot be skipped safely.
        let mut bad = encode_u64(99);
        bad.push(0);
        assert_eq!(
            decode_event_body(&bad),
            Err(EventBodyError::UnknownOpcode { opcode: 99 })
        );
    }

    #[test]
    fn unknown_optional_length_framed_skip_preserves_neighbors() {
        let mut body = encode_event_body(&[EventRecordSpec::TimeLine {
            fid: 1,
            line: 10,
            ticks: 42,
        }]);
        body.extend_from_slice(
            &encode_unknown_optional_skip_record(99, b"opaque-extension-bytes")
                .expect("encode skip"),
        );
        body.extend_from_slice(&encode_event_body(&[EventRecordSpec::Mark {
            string_id: 0,
            string_flags: 0,
            label: b"after-skip",
        }]));

        let (recs, n) = decode_event_body(&body).expect("skip unknown optional");
        assert_eq!(n, body.len());
        assert_eq!(recs.len(), 2, "skipped record must not appear");
        match &recs[0] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 42));
            }
            other => panic!("expected TimeLine before skip, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"after-skip"),
            other => panic!("expected Mark after skip, got {other:?}"),
        }
        // Dual decode stability.
        let (recs2, n2) = decode_event_body(&body).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn unknown_optional_empty_length_framed_skip_ok() {
        let mut body = encode_event_body(&[EventRecordSpec::Version {
            major: 6,
            minor: 0,
        }]);
        body.extend_from_slice(&encode_unknown_optional_skip_record(100, b"").unwrap());
        body.extend_from_slice(&encode_event_body(&[EventRecordSpec::Discount]));
        let (recs, n) = decode_event_body(&body).unwrap();
        assert_eq!(n, body.len());
        assert_eq!(recs.len(), 2);
        assert!(matches!(recs[0], EventRecord::Version { major: 6, minor: 0 }));
        assert!(matches!(recs[1], EventRecord::Discount));
    }

    #[test]
    fn unknown_optional_truncated_length_frame_err() {
        let mut partial = encode_u64(99);
        partial.push(FLAG_BODY_LENGTH);
        partial.extend_from_slice(&encode_u64(8)); // claims 8 body bytes
        partial.extend_from_slice(b"short"); // only 5
        match decode_event_body(&partial) {
            Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated skip body, got {other:?}"),
        }
    }

    #[test]
    fn unknown_optional_missing_length_uleb_err() {
        let mut partial = encode_u64(99);
        partial.push(FLAG_BODY_LENGTH);
        // no length ULEB
        match decode_event_body(&partial) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated length uleb, got {other:?}"),
        }
    }

    #[test]
    fn encode_unknown_optional_skip_rejects_reserved_and_known() {
        assert_eq!(
            encode_unknown_optional_skip_record(0, b"x"),
            Err(EventBodyError::ReservedOpcode)
        );
        assert_eq!(
            encode_unknown_optional_skip_record(opcode::MARK, b"x"),
            Err(EventBodyError::UnknownOpcode {
                opcode: opcode::MARK
            })
        );
    }

    #[test]
    fn truncated_mid_time_line_err() {
        let full = encode_event_body(&[EventRecordSpec::TimeLine {
            fid: 1,
            line: 2,
            ticks: 3,
        }]);
        // Drop last byte of last varint.
        assert!(full.len() > 3);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid-record, got {other:?}"),
        }
    }

    #[test]
    fn truncated_after_opcode_before_flags_err() {
        let mut partial = encode_u64(opcode::TIME_LINE);
        // no flags byte
        match decode_event_body(&partial) {
            Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated flags, got {other:?}"),
        }
        // flags present but no body fields
        partial.push(0);
        match decode_event_body(&partial) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated body, got {other:?}"),
        }
    }

    #[test]
    fn never_panic_on_garbage() {
        assert!(decode_event_body(&[]).is_ok());
        let _ = decode_event_body(&[0xFF; 8]);
        let _ = decode_event_body(b"\x01"); // MARK opcode incomplete
    }

    #[test]
    fn codec_none_chunk_payload_is_event_body() {
        // Optional composition smoke: EVENT chunk + codec NONE carries event-body bytes.
        let body = encode_event_body(&[
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 100,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"x",
            },
        ]);
        let frame = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            2,
            body.len() as u32,
            &body,
            0,
        );
        let parsed = parse_chunk_frame(&frame).expect("chunk");
        assert_eq!(parsed.codec, codec::NONE);
        assert_eq!(parsed.payload, body.as_slice());
        let (recs, n) = decode_event_body(parsed.payload).expect("body from chunk");
        assert_eq!(n, body.len());
        assert_eq!(recs.len(), 2);
        match &recs[0] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 5, 100));
            }
            other => panic!("expected TimeLine, got {other:?}"),
        }
    }

    #[test]
    fn time_block_and_sub_entry_roundtrip() {
        let specs = [
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 5,
                block_line: 4,
                ticks: 780,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 1,
                caller_line: 10,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 6,
                ticks: 3,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"mixed-ops",
            },
        ];
        let enc = encode_event_body(&specs);
        let mut expect = Vec::new();
        for s in &specs {
            encode_record_into(&mut expect, s);
        }
        assert_eq!(enc, expect);

        let (recs, n) = decode_event_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 4);
        match &recs[0] {
            EventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => {
                assert_eq!((*fid, *line, *block_line, *ticks), (1, 5, 4, 780));
            }
            other => panic!("expected TimeBlock, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => {
                assert_eq!((*caller_fid, *caller_line), (1, 10));
            }
            other => panic!("expected SubEntry, got {other:?}"),
        }
        match &recs[2] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 6, 3));
            }
            other => panic!("expected TimeLine, got {other:?}"),
        }
        match &recs[3] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"mixed-ops"),
            other => panic!("expected Mark, got {other:?}"),
        }
        // Dual decode stability.
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn truncated_mid_time_block_err() {
        let full = encode_event_body(&[EventRecordSpec::TimeBlock {
            fid: 1,
            line: 2,
            block_line: 3,
            ticks: 4,
        }]);
        assert!(full.len() > 4);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid TIME_BLOCK, got {other:?}"),
        }
    }

    #[test]
    fn truncated_mid_sub_entry_err() {
        let full = encode_event_body(&[EventRecordSpec::SubEntry {
            caller_fid: 2,
            caller_line: 9,
        }]);
        assert!(full.len() > 2);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid SUB_ENTRY, got {other:?}"),
        }
    }

    #[test]
    fn is_known_opcode_covers_new_ops() {
        assert!(is_known_opcode(opcode::MARK));
        assert!(is_known_opcode(opcode::TIME_LINE));
        assert!(is_known_opcode(opcode::TIME_BLOCK));
        assert!(is_known_opcode(opcode::SUB_ENTRY));
        assert!(is_known_opcode(opcode::SUB_RETURN));
        assert!(is_known_opcode(opcode::SUB_INFO));
        assert!(is_known_opcode(opcode::SRC_LINE));
        assert!(is_known_opcode(opcode::NEW_FID));
        assert!(is_known_opcode(opcode::PID_START));
        assert!(is_known_opcode(opcode::PID_END));
        assert!(is_known_opcode(opcode::SUB_CALLERS));
        assert!(is_known_opcode(opcode::DISCOUNT));
        assert!(is_known_opcode(opcode::ATTRIBUTE));
        assert!(is_known_opcode(opcode::OPTION));
        assert!(is_known_opcode(opcode::COMMENT));
        assert!(is_known_opcode(opcode::START_DEFLATE));
        assert!(is_known_opcode(opcode::VERSION));
        assert!(!is_known_opcode(opcode::RESERVED));
        assert!(!is_known_opcode(99));
    }

    #[test]
    fn version_roundtrip() {
        let specs = [
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
        ];
        let enc = encode_event_body(&specs);
        let mut expect = Vec::new();
        for s in &specs {
            encode_record_into(&mut expect, s);
        }
        assert_eq!(enc, expect);

        let (recs, n) = decode_event_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 3);
        match &recs[0] {
            EventRecord::Version { major, minor } => assert_eq!((*major, *minor), (5, 0)),
            other => panic!("expected Version, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::StartDeflate => {}
            other => panic!("expected StartDeflate, got {other:?}"),
        }
        match &recs[2] {
            EventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
            other => panic!("expected TimeLine, got {other:?}"),
        }
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    /// Dump-aligned provisional dual-output sequence (COMPAT-001 illustrative shape):
    /// VERSION → COMMENT + ATTRIBUTE + OPTION → START_DEFLATE → PID_START … workload … PID_END.
    /// Preflight only — not OI-001-03 seq-number freeze / mid-stream codec switch.
    fn dual_output_sequence_specs() -> [EventRecordSpec<'static>; 9] {
        [
            EventRecordSpec::Version {
                major: 5,
                minor: 0,
            },
            EventRecordSpec::Comment {
                string_id: 0,
                string_flags: FLAG_UTF8,
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

    fn assert_dual_output_sequence_order(recs: &[EventRecord<'_>]) {
        assert_eq!(recs.len(), 9, "dual-output sequence record count");
        match &recs[0] {
            EventRecord::Version { major, minor } => assert_eq!((*major, *minor), (5, 0)),
            other => panic!("[0] expected Version, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::Comment { text } => {
                assert_eq!(text.flags, FLAG_UTF8);
                assert_eq!(text.data, b"# dual-output prelude");
            }
            other => panic!("[1] expected Comment, got {other:?}"),
        }
        match &recs[2] {
            EventRecord::Attribute { key, value } => {
                assert_eq!(key.data, b"basetime");
                assert_eq!(value.data, b"1700000000");
            }
            other => panic!("[2] expected Attribute, got {other:?}"),
        }
        match &recs[3] {
            EventRecord::Option { key, value } => {
                assert_eq!(key.data, b"calls");
                assert_eq!(value.data, b"1");
            }
            other => panic!("[3] expected Option, got {other:?}"),
        }
        match &recs[4] {
            EventRecord::StartDeflate => {}
            other => panic!("[4] expected StartDeflate, got {other:?}"),
        }
        match &recs[5] {
            EventRecord::PidStart {
                pid,
                ppid,
                start_time,
            } => {
                assert_eq!((*pid, *ppid, *start_time), (1001, 1, 1_700_000_000));
            }
            other => panic!("[5] expected PidStart, got {other:?}"),
        }
        match &recs[6] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 42));
            }
            other => panic!("[6] expected TimeLine interior, got {other:?}"),
        }
        match &recs[7] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"workload"),
            other => panic!("[7] expected Mark interior, got {other:?}"),
        }
        match &recs[8] {
            EventRecord::PidEnd { pid, end_time } => {
                assert_eq!((*pid, *end_time), (1001, 1_700_000_042));
            }
            other => panic!("[8] expected PidEnd, got {other:?}"),
        }
        // Order invariants (not only "some VERSION exists").
        let op_rank = |r: &EventRecord<'_>| -> u8 {
            match r {
                EventRecord::Version { .. } => 0,
                EventRecord::Comment { .. }
                | EventRecord::Attribute { .. }
                | EventRecord::Option { .. } => 1,
                EventRecord::StartDeflate => 2,
                EventRecord::PidStart { .. } => 3,
                EventRecord::PidEnd { .. } => 5,
                _ => 4, // interior workload between PID_START and PID_END
            }
        };
        let ranks: Vec<u8> = recs.iter().map(op_rank).collect();
        assert!(ranks.windows(2).all(|w| w[0] <= w[1]), "order ranks {ranks:?}");
        assert_eq!(ranks[0], 0);
        assert!(ranks.iter().any(|&r| r == 1), "meta present");
        assert!(ranks.iter().any(|&r| r == 2), "START_DEFLATE present");
        assert!(ranks.iter().any(|&r| r == 3), "PID_START present");
        assert!(ranks.iter().any(|&r| r == 4), "interior present");
        assert!(ranks.iter().any(|&r| r == 5), "PID_END present");
    }

    #[test]
    fn dual_output_sequence_roundtrip_order_and_fields() {
        let specs = dual_output_sequence_specs();
        let enc = encode_event_body(&specs);
        let mut expect = Vec::new();
        for s in &specs {
            encode_record_into(&mut expect, s);
        }
        assert_eq!(enc, expect);

        let (recs, n) = decode_event_body(&enc).expect("dual-output sequence roundtrip");
        assert_eq!(n, enc.len());
        assert_dual_output_sequence_order(&recs);
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn dual_output_sequence_truncated_mid_body_err() {
        let full = encode_event_body(&dual_output_sequence_specs());
        assert!(full.len() > 16);
        // Truncate after VERSION + part of COMMENT so order recovery cannot complete.
        let trunc = &full[..full.len() / 2];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_))
            | Err(EventBodyError::Truncated { .. })
            | Err(EventBodyError::String(_)) => {}
            other => panic!("expected truncated dual-output sequence, got {other:?}"),
        }
    }

    #[test]
    fn dual_output_sequence_unknown_opcode_still_fail_closed() {
        // Append reserved/unknown after a valid dual-output prefix.
        let mut enc = encode_event_body(&[EventRecordSpec::Version {
            major: 5,
            minor: 0,
        }]);
        enc.extend_from_slice(&encode_u64(99));
        enc.push(FLAG_OPCODE_REQUIRED);
        match decode_event_body(&enc) {
            Err(EventBodyError::UnknownRequiredOpcode { opcode: 99 }) => {}
            other => panic!("expected unknown required opcode, got {other:?}"),
        }
    }

    #[test]
    fn truncated_mid_version_err() {
        let full = encode_event_body(&[EventRecordSpec::Version {
            major: 6,
            minor: 15,
        }]);
        assert!(full.len() > 3);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid VERSION, got {other:?}"),
        }
    }

    #[test]
    fn start_deflate_roundtrip() {
        let specs = [
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
        ];
        let enc = encode_event_body(&specs);
        let mut expect = Vec::new();
        for s in &specs {
            encode_record_into(&mut expect, s);
        }
        assert_eq!(enc, expect);

        let (recs, n) = decode_event_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 3);
        match &recs[0] {
            EventRecord::StartDeflate => {}
            other => panic!("expected StartDeflate, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
            other => panic!("expected TimeLine, got {other:?}"),
        }
        match &recs[2] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"sd"),
            other => panic!("expected Mark, got {other:?}"),
        }
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn start_deflate_opcode_flags_only_body() {
        let enc = encode_event_body(&[EventRecordSpec::StartDeflate]);
        // opcode ULEB for 16 is 1 byte + 1 flags byte
        assert_eq!(enc, vec![opcode::START_DEFLATE as u8, 0]);
        let (recs, n) = decode_event_body(&enc).unwrap();
        assert_eq!(n, 2);
        assert_eq!(recs, vec![EventRecord::StartDeflate]);
    }

    #[test]
    fn truncated_start_deflate_missing_flags_err() {
        let partial = encode_u64(opcode::START_DEFLATE);
        match decode_event_body(&partial) {
            Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated flags for START_DEFLATE, got {other:?}"),
        }
    }

    #[test]
    fn comment_roundtrip() {
        let specs = [
            EventRecordSpec::Comment {
                string_id: 0,
                string_flags: FLAG_UTF8,
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
        ];
        let enc = encode_event_body(&specs);
        let mut expect = Vec::new();
        for s in &specs {
            encode_record_into(&mut expect, s);
        }
        assert_eq!(enc, expect);

        let (recs, n) = decode_event_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 3);
        match &recs[0] {
            EventRecord::Comment { text } => {
                assert_eq!(text.flags, FLAG_UTF8);
                assert_eq!(text.data, b"# profiler note");
            }
            other => panic!("expected Comment, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
            other => panic!("expected TimeLine, got {other:?}"),
        }
        match &recs[2] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"cmt"),
            other => panic!("expected Mark, got {other:?}"),
        }
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn truncated_mid_comment_err() {
        let full = encode_event_body(&[EventRecordSpec::Comment {
            string_id: 0,
            string_flags: 0,
            text: b"hello comment",
        }]);
        assert!(full.len() > 3);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_))
            | Err(EventBodyError::String(_))
            | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid COMMENT, got {other:?}"),
        }
    }

    #[test]
    fn attribute_and_option_roundtrip() {
        let specs = [
            EventRecordSpec::Attribute {
                key_string_id: 0,
                key_string_flags: FLAG_UTF8,
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
                value_string_flags: FLAG_UTF8,
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
        ];
        let enc = encode_event_body(&specs);
        let mut expect = Vec::new();
        for s in &specs {
            encode_record_into(&mut expect, s);
        }
        assert_eq!(enc, expect);

        let (recs, n) = decode_event_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 4);
        match &recs[0] {
            EventRecord::Attribute { key, value } => {
                assert_eq!(key.flags, FLAG_UTF8);
                assert_eq!(key.data, b"basetime");
                assert_eq!(value.data, b"1700000000");
            }
            other => panic!("expected Attribute, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::Option { key, value } => {
                assert_eq!(key.data, b"calls");
                assert_eq!(value.flags, FLAG_UTF8);
                assert_eq!(value.data, b"1");
            }
            other => panic!("expected Option, got {other:?}"),
        }
        match &recs[2] {
            EventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
            other => panic!("expected TimeLine, got {other:?}"),
        }
        match &recs[3] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"attr-opt"),
            other => panic!("expected Mark, got {other:?}"),
        }
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn truncated_mid_attribute_err() {
        let full = encode_event_body(&[EventRecordSpec::Attribute {
            key_string_id: 0,
            key_string_flags: 0,
            key: b"k",
            value_string_id: 1,
            value_string_flags: 0,
            value: b"v",
        }]);
        assert!(full.len() > 4);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_))
            | Err(EventBodyError::String(_))
            | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid ATTRIBUTE, got {other:?}"),
        }
    }

    #[test]
    fn truncated_mid_option_err() {
        let full = encode_event_body(&[EventRecordSpec::Option {
            key_string_id: 0,
            key_string_flags: 0,
            key: b"opt",
            value_string_id: 1,
            value_string_flags: 0,
            value: b"val",
        }]);
        assert!(full.len() > 4);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_))
            | Err(EventBodyError::String(_))
            | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid OPTION, got {other:?}"),
        }
    }

    #[test]
    fn known_key_table_covers_dump_json_surfaces() {
        // Required by known-key preflight plan.
        assert!(known_key::is_known_attribute_key(known_key::BASETIME));
        assert!(known_key::is_known_attribute_key(known_key::TICKS_PER_SEC));
        assert!(known_key::is_known_option_key(known_key::CALLS));
        // ≥1 additional documented keys.
        assert!(known_key::is_known_attribute_key(known_key::APPLICATION));
        assert!(known_key::is_known_attribute_key(known_key::XS_VERSION));
        assert!(known_key::is_known_option_key(known_key::BLOCKS));
        assert!(known_key::is_known_option_key(known_key::STMTS));
        assert!(known_key::is_known_option_key(known_key::COMPRESS));
        assert!(known_key::is_known_meta_key(b"basetime"));
        assert!(known_key::is_known_meta_key(b"calls"));
        assert!(!known_key::is_known_attribute_key(b"not-a-key"));
        assert!(!known_key::is_known_option_key(b"not-a-key"));
        assert!(!known_key::is_known_meta_key(b""));
        // Tables non-empty and constants appear in tables.
        assert!(known_key::KNOWN_ATTRIBUTE_KEYS.len() >= 3);
        assert!(known_key::KNOWN_OPTION_KEYS.len() >= 1);
        assert!(known_key::KNOWN_ATTRIBUTE_KEYS.contains(&known_key::BASETIME));
        assert!(known_key::KNOWN_OPTION_KEYS.contains(&known_key::CALLS));
    }

    fn assert_known_key_sample_records(recs: &[EventRecord<'_>]) {
        assert_eq!(recs.len(), 5);
        match &recs[0] {
            EventRecord::Attribute { key, value } => {
                assert!(known_key::is_known_attribute_key(key.data));
                assert_eq!(key.data, known_key::BASETIME);
                assert_eq!(value.data, b"1786111723");
            }
            other => panic!("[0] Attribute basetime, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::Attribute { key, value } => {
                assert_eq!(key.data, known_key::TICKS_PER_SEC);
                assert_eq!(value.data, b"10000000");
            }
            other => panic!("[1] Attribute ticks_per_sec, got {other:?}"),
        }
        match &recs[2] {
            EventRecord::Attribute { key, value } => {
                assert_eq!(key.data, known_key::APPLICATION);
                assert_eq!(value.data, b"workload.pl");
            }
            other => panic!("[2] Attribute application, got {other:?}"),
        }
        match &recs[3] {
            EventRecord::Option { key, value } => {
                assert!(known_key::is_known_option_key(key.data));
                assert_eq!(key.data, known_key::CALLS);
                assert_eq!(value.data, b"1");
            }
            other => panic!("[3] Option calls, got {other:?}"),
        }
        match &recs[4] {
            EventRecord::Option { key, value } => {
                assert_eq!(key.data, known_key::BLOCKS);
                assert_eq!(value.data, b"0");
            }
            other => panic!("[4] Option blocks, got {other:?}"),
        }
    }

    #[test]
    fn known_key_attr_option_sample_roundtrip() {
        let specs = known_key_attr_option_sample_specs();
        // Every sample key is in the provisional table.
        for s in &specs {
            match s {
                EventRecordSpec::Attribute { key, .. } => {
                    assert!(
                        known_key::is_known_attribute_key(key),
                        "unknown ATTRIBUTE key {key:?}"
                    );
                }
                EventRecordSpec::Option { key, .. } => {
                    assert!(
                        known_key::is_known_option_key(key),
                        "unknown OPTION key {key:?}"
                    );
                }
                other => panic!("sample must be Attribute/Option, got {other:?}"),
            }
        }
        let enc = encode_event_body(&specs);
        let (recs, n) = decode_event_body(&enc).expect("known-key sample roundtrip");
        assert_eq!(n, enc.len());
        assert_known_key_sample_records(&recs);
        // Dual decode stability.
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn known_key_helpers_compose_encode_decode() {
        let specs = [
            attribute_kv(known_key::XS_VERSION, b"6.15"),
            option_kv(known_key::STMTS, b"1"),
            option_kv(known_key::COMPRESS, b"0"),
        ];
        let enc = encode_event_body(&specs);
        let (recs, n) = decode_event_body(&enc).expect("helpers roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 3);
        match &recs[0] {
            EventRecord::Attribute { key, value } => {
                assert_eq!(key.data, known_key::XS_VERSION);
                assert_eq!(value.data, b"6.15");
            }
            other => panic!("{other:?}"),
        }
        match &recs[1] {
            EventRecord::Option { key, value } => {
                assert_eq!(key.data, known_key::STMTS);
                assert_eq!(value.data, b"1");
            }
            other => panic!("{other:?}"),
        }
        match &recs[2] {
            EventRecord::Option { key, value } => {
                assert_eq!(key.data, known_key::COMPRESS);
                assert_eq!(value.data, b"0");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn known_key_truncated_mid_basetime_attr_still_fail_closed() {
        let full = encode_event_body(&[attribute_kv(known_key::BASETIME, b"1786111723")]);
        assert!(full.len() > 4);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_))
            | Err(EventBodyError::String(_))
            | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated known-key ATTRIBUTE, got {other:?}"),
        }
    }

    #[test]
    fn sub_callers_and_discount_roundtrip() {
        let specs = [
            EventRecordSpec::SubCallers {
                fid: 1,
                line: 10,
                count: 15,
                incl: 900,
                excl: 50,
                reci: 0,
                rec_depth: 0,
                called_string_id: 0,
                called_string_flags: FLAG_UTF8,
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
        ];
        let enc = encode_event_body(&specs);
        let mut expect = Vec::new();
        for s in &specs {
            encode_record_into(&mut expect, s);
        }
        assert_eq!(enc, expect);

        let (recs, n) = decode_event_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 4);
        match &recs[0] {
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
            } => {
                assert_eq!(
                    (*fid, *line, *count, *incl, *excl, *reci, *rec_depth),
                    (1, 10, 15, 900, 50, 0, 0)
                );
                assert_eq!(called.flags, FLAG_UTF8);
                assert_eq!(called.data, b"main::leaf");
                assert_eq!(caller.data, b"main::mid");
            }
            other => panic!("expected SubCallers, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::Discount => {}
            other => panic!("expected Discount, got {other:?}"),
        }
        match &recs[2] {
            EventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
            other => panic!("expected TimeLine, got {other:?}"),
        }
        match &recs[3] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"sc-disc"),
            other => panic!("expected Mark, got {other:?}"),
        }
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn discount_opcode_flags_only_body() {
        let enc = encode_event_body(&[EventRecordSpec::Discount]);
        // opcode ULEB for 12 is 1 byte + 1 flags byte
        assert_eq!(enc, vec![opcode::DISCOUNT as u8, 0]);
        let (recs, n) = decode_event_body(&enc).unwrap();
        assert_eq!(n, 2);
        assert_eq!(recs, vec![EventRecord::Discount]);
    }

    #[test]
    fn truncated_mid_sub_callers_err() {
        let full = encode_event_body(&[EventRecordSpec::SubCallers {
            fid: 1,
            line: 2,
            count: 3,
            incl: 4,
            excl: 5,
            reci: 6,
            rec_depth: 7,
            called_string_id: 0,
            called_string_flags: 0,
            called: b"a",
            caller_string_id: 1,
            caller_string_flags: 0,
            caller: b"b",
        }]);
        assert!(full.len() > 8);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_))
            | Err(EventBodyError::String(_))
            | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid SUB_CALLERS, got {other:?}"),
        }
    }

    #[test]
    fn truncated_discount_missing_flags_err() {
        let partial = encode_u64(opcode::DISCOUNT);
        // no flags byte
        match decode_event_body(&partial) {
            Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated flags for DISCOUNT, got {other:?}"),
        }
    }

    #[test]
    fn pid_start_and_pid_end_roundtrip() {
        let specs = [
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
        ];
        let enc = encode_event_body(&specs);
        let mut expect = Vec::new();
        for s in &specs {
            encode_record_into(&mut expect, s);
        }
        assert_eq!(enc, expect);

        let (recs, n) = decode_event_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 4);
        match &recs[0] {
            EventRecord::PidStart {
                pid,
                ppid,
                start_time,
            } => assert_eq!((*pid, *ppid, *start_time), (1001, 1, 1_700_000_000)),
            other => panic!("expected PidStart, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::PidEnd { pid, end_time } => {
                assert_eq!((*pid, *end_time), (1001, 1_700_000_042));
            }
            other => panic!("expected PidEnd, got {other:?}"),
        }
        match &recs[2] {
            EventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
            other => panic!("expected TimeLine, got {other:?}"),
        }
        match &recs[3] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"pid-pair"),
            other => panic!("expected Mark, got {other:?}"),
        }
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn truncated_mid_pid_start_err() {
        let full = encode_event_body(&[EventRecordSpec::PidStart {
            pid: 1,
            ppid: 2,
            start_time: 3,
        }]);
        assert!(full.len() > 3);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid PID_START, got {other:?}"),
        }
    }

    #[test]
    fn truncated_mid_pid_end_err() {
        let full = encode_event_body(&[EventRecordSpec::PidEnd {
            pid: 9,
            end_time: 99,
        }]);
        assert!(full.len() > 2);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid PID_END, got {other:?}"),
        }
    }

    #[test]
    fn src_line_and_new_fid_roundtrip() {
        let specs = [
            EventRecordSpec::NewFid {
                fid: 1,
                string_id: 0,
                string_flags: FLAG_UTF8,
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
        ];
        let enc = encode_event_body(&specs);
        let mut expect = Vec::new();
        for s in &specs {
            encode_record_into(&mut expect, s);
        }
        assert_eq!(enc, expect);

        let (recs, n) = decode_event_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 4);
        match &recs[0] {
            EventRecord::NewFid { fid, filename } => {
                assert_eq!(*fid, 1);
                assert_eq!(filename.id, 0);
                assert_eq!(filename.flags, FLAG_UTF8);
                assert_eq!(filename.data, b"workload.pl");
            }
            other => panic!("expected NewFid, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::SrcLine { fid, line, text } => {
                assert_eq!((*fid, *line), (1, 5));
                assert_eq!(text.id, 1);
                assert_eq!(text.data, b"  my $x = 1;");
            }
            other => panic!("expected SrcLine, got {other:?}"),
        }
        match &recs[2] {
            EventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 10),
            other => panic!("expected TimeLine, got {other:?}"),
        }
        match &recs[3] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"src-fid"),
            other => panic!("expected Mark, got {other:?}"),
        }
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn truncated_mid_src_line_err() {
        let full = encode_event_body(&[EventRecordSpec::SrcLine {
            fid: 1,
            line: 2,
            string_id: 0,
            string_flags: 0,
            text: b"line",
        }]);
        assert!(full.len() > 3);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_))
            | Err(EventBodyError::String(_))
            | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid SRC_LINE, got {other:?}"),
        }
    }

    #[test]
    fn truncated_mid_new_fid_err() {
        let full = encode_event_body(&[EventRecordSpec::NewFid {
            fid: 2,
            string_id: 0,
            string_flags: 0,
            filename: b"f.pl",
        }]);
        assert!(full.len() > 2);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_))
            | Err(EventBodyError::String(_))
            | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid NEW_FID, got {other:?}"),
        }
    }

    #[test]
    fn sub_return_and_sub_info_roundtrip() {
        let specs = [
            EventRecordSpec::SubReturn {
                depth: 2,
                incl: 1500,
                excl: 100,
                string_id: 0,
                string_flags: FLAG_UTF8,
                subname: b"main::leaf",
            },
            EventRecordSpec::SubInfo {
                fid: 1,
                first_line: 3,
                last_line: 7,
                string_id: 1,
                string_flags: 0,
                name: b"main::mid",
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
        ];
        let enc = encode_event_body(&specs);
        let mut expect = Vec::new();
        for s in &specs {
            encode_record_into(&mut expect, s);
        }
        assert_eq!(enc, expect);

        let (recs, n) = decode_event_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 4);
        match &recs[0] {
            EventRecord::SubReturn {
                depth,
                incl,
                excl,
                subname,
            } => {
                assert_eq!((*depth, *incl, *excl), (2, 1500, 100));
                assert_eq!(subname.id, 0);
                assert_eq!(subname.flags, FLAG_UTF8);
                assert_eq!(subname.data, b"main::leaf");
            }
            other => panic!("expected SubReturn, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::SubInfo {
                fid,
                first_line,
                last_line,
                name,
            } => {
                assert_eq!((*fid, *first_line, *last_line), (1, 3, 7));
                assert_eq!(name.id, 1);
                assert_eq!(name.data, b"main::mid");
            }
            other => panic!("expected SubInfo, got {other:?}"),
        }
        match &recs[2] {
            EventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => assert_eq!((*caller_fid, *caller_line), (1, 12)),
            other => panic!("expected SubEntry, got {other:?}"),
        }
        match &recs[3] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"sr-si"),
            other => panic!("expected Mark, got {other:?}"),
        }
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn truncated_mid_sub_return_err() {
        let full = encode_event_body(&[EventRecordSpec::SubReturn {
            depth: 1,
            incl: 10,
            excl: 2,
            string_id: 0,
            string_flags: 0,
            subname: b"x",
        }]);
        assert!(full.len() > 4);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_))
            | Err(EventBodyError::String(_))
            | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid SUB_RETURN, got {other:?}"),
        }
    }

    #[test]
    fn truncated_mid_sub_info_err() {
        let full = encode_event_body(&[EventRecordSpec::SubInfo {
            fid: 1,
            first_line: 1,
            last_line: 2,
            string_id: 0,
            string_flags: 0,
            name: b"y",
        }]);
        assert!(full.len() > 4);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_))
            | Err(EventBodyError::String(_))
            | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid SUB_INFO, got {other:?}"),
        }
    }

    /// Provisional ID lockfile alignment (docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md
    /// + collector/include/nytprof_v6_ids.h). Not a wire freeze; packing encode residual.
    #[test]
    fn provisional_id_lockfile_packing_constants() {
        assert_eq!(FLAG_OPCODE_REQUIRED, 0x01);
        assert_eq!(FLAG_BODY_LENGTH, 0x02);
        assert_eq!(FLAG_SITE_DELTA, 0x04);
        assert_eq!(FLAG_HAS_SEQ, 0x08);
        assert_eq!(opcode::VERSION, 17);
        assert_eq!(opcode::TIME_LINE_RUN, 18);
        assert_eq!(opcode::TIME_BLOCK_RUN, 19);
        assert_eq!(MAX_TIME_RUN_LEN, 1_048_576);
        // Run opcodes reserved in lockfile but not yet absolute-path known codecs.
        assert!(!is_known_opcode(opcode::TIME_LINE_RUN));
        assert!(!is_known_opcode(opcode::TIME_BLOCK_RUN));
    }
}

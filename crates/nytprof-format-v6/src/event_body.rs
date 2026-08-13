//! Provisional **format v6** event-body opcode codec (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-event-body-provisional-v0.md`
//!
//! Codec **NONE** chunk payloads: ordered records with ULEB128 opcodes + typed
//! fields composed from shipped varint / string-blob primitives.
//! Does **not** inflate zlib/zstd/LZ4, implement full v5 tag parity, or the C writer.

use crate::string::{decode_string_blob, encode_string_blob, StringBlob, StringError};
use crate::varint::{decode_i64, decode_u64, encode_i64, encode_u64, VarintError};

/// Fail-closed upper bound on total event-body size (64 MiB).
pub const MAX_EVENT_BODY_BYTES: usize = 64 * 1024 * 1024;

/// Flag: unknown opcode must fail closed (required opcode).
pub const FLAG_OPCODE_REQUIRED: u8 = 0x01;

/// Flag: typed body is length-framed (`ULEB128 body_len || body_len bytes`).
///
/// Unknown optional opcode skip. Known opcodes use their fixed typed layouts and
/// ignore this bit. **Frozen** bit assignment for major=6 ([ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md)).
pub const FLAG_BODY_LENGTH: u8 = 0x02;

/// Flag: TIME_LINE / TIME_BLOCK / SUB_ENTRY site fields are **signed deltas**
/// (ZigZag+ULEB) relative to a running base, not absolute ULEB sites.
///
/// Packing form per ADR-0001; **frozen** bit for major=6 (ADR-0006). Absolute path
/// remains flags `0` (default encode_event_body).
pub const FLAG_SITE_DELTA: u8 = 0x04;

/// Flag: record carries a **logical event sequence number** (ULEB128) immediately
/// after the flags byte and before the typed body.
///
/// **Frozen** optional bit (ADR-0006 §3 / OQ-5): when dual-output seq is emitted,
/// VERSION and START_DEFLATE may participate in the same monotonic space. Default
/// [`encode_event_body`] omits the flag (no seq field).
pub const FLAG_HAS_SEQ: u8 = 0x08;

/// Fail-closed upper bound on a single length-framed unknown body (same as event-body cap).
pub const MAX_SKIP_BODY_BYTES: usize = MAX_EVENT_BODY_BYTES;

/// Fail-closed upper bound on TIME_LINE_RUN packed length `N` (ticks count).
///
/// Frozen cap for major=6 (ADR-0006). Checked **before** expanding to N logical
/// TIME_LINE records.
pub const MAX_TIME_LINE_RUN_LEN: usize = 1_048_576;

/// Fail-closed upper bound on TIME_BLOCK_RUN packed length `N` (ticks count).
///
/// Frozen cap for major=6 (ADR-0006). Checked **before** expanding to N logical
/// TIME_BLOCK records.
pub const MAX_TIME_BLOCK_RUN_LEN: usize = 1_048_576;

/// Event opcodes (numeric IDs frozen for major=6 — ADR-0006).
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
    /// Packed run of consecutive same-site TIME_LINE events (expands on decode).
    ///
    /// Body: `fid`, `line`, `N`, then `N` × `ticks` (all ULEB128). Decode expands
    /// to N logical TIME_LINE records retaining every per-event ticks value.
    /// Packing form ADR-0001; opcode number frozen ADR-0006.
    pub const TIME_LINE_RUN: u64 = 18;
    /// Packed run of consecutive same-site TIME_BLOCK events (expands on decode).
    ///
    /// Body: `fid`, `line`, `block_line`, `N`, then `N` × `ticks` (all ULEB128).
    /// Decode expands to N logical TIME_BLOCK records retaining every per-event ticks.
    /// Packing form ADR-0001; opcode number frozen ADR-0006.
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
            | opcode::TIME_LINE_RUN
            | opcode::TIME_BLOCK_RUN
    )
}

/// Provisional ATTRIBUTE / OPTION **known-key** vocabulary (OI-002-03/04 runway).
///
/// Keys are dump/JSON-surface aligned and expanded from golden fixture
/// `fixtures/v5/*/readstream.jsonl` ATTRIBUTE/OPTION tags (default-calls1 + siblings).
/// This is **not** a complete writer inventory freeze — unknown string keys may still
/// encode/decode as free-form projections; the table documents fixture-observed keys
/// plus dual-path JSON samples.
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
    /// ATTRIBUTE `perl_version` (fixture dump).
    pub const PERL_VERSION: &[u8] = b"perl_version";
    /// ATTRIBUTE `nv_size` (fixture dump).
    pub const NV_SIZE: &[u8] = b"nv_size";
    /// ATTRIBUTE `PL_perldb` (fixture dump).
    pub const PL_PERLDB: &[u8] = b"PL_perldb";
    /// ATTRIBUTE `clock_id` (fixture dump).
    pub const CLOCK_ID: &[u8] = b"clock_id";
    /// ATTRIBUTE `cumulative_overhead_ticks` (fixture dump; late stream).
    pub const CUMULATIVE_OVERHEAD_TICKS: &[u8] = b"cumulative_overhead_ticks";

    /// OPTION `calls` (default-calls1 multiplicity surface).
    pub const CALLS: &[u8] = b"calls";
    /// OPTION `blocks`.
    pub const BLOCKS: &[u8] = b"blocks";
    /// OPTION `stmts`.
    pub const STMTS: &[u8] = b"stmts";
    /// OPTION `compress`.
    pub const COMPRESS: &[u8] = b"compress";
    /// OPTION `usecputime` (fixture dump).
    pub const USECPUTIME: &[u8] = b"usecputime";
    /// OPTION `subs` (fixture dump).
    pub const SUBS: &[u8] = b"subs";
    /// OPTION `leave` (fixture dump).
    pub const LEAVE: &[u8] = b"leave";
    /// OPTION `expand` (fixture dump).
    pub const EXPAND: &[u8] = b"expand";
    /// OPTION `trace` (fixture dump).
    pub const TRACE: &[u8] = b"trace";
    /// OPTION `use_db_sub` (fixture dump).
    pub const USE_DB_SUB: &[u8] = b"use_db_sub";
    /// OPTION `clock` (fixture dump).
    pub const CLOCK: &[u8] = b"clock";
    /// OPTION `slowops` (fixture dump).
    pub const SLOWOPS: &[u8] = b"slowops";
    /// OPTION `findcaller` (fixture dump).
    pub const FINDCALLER: &[u8] = b"findcaller";
    /// OPTION `forkdepth` (fixture dump).
    pub const FORKDEPTH: &[u8] = b"forkdepth";
    /// OPTION `perldb` (fixture dump).
    pub const PERLDB: &[u8] = b"perldb";
    /// OPTION `nameevals` (fixture dump).
    pub const NAMEEVALS: &[u8] = b"nameevals";
    /// OPTION `nameanonsubs` (fixture dump).
    pub const NAMEANONSUBS: &[u8] = b"nameanonsubs";
    /// OPTION `evals` (fixture dump).
    pub const EVALS: &[u8] = b"evals";

    /// Provisional known ATTRIBUTE keys (OI-002-03 runway — fixture-expanded; not full freeze).
    ///
    /// Source: union of ATTRIBUTE keys in `fixtures/v5/{default-calls1,calls2-default,blocks-calls1,default-calls2}/readstream.jsonl`.
    pub const KNOWN_ATTRIBUTE_KEYS: &[&[u8]] = &[
        BASETIME,
        TICKS_PER_SEC,
        APPLICATION,
        XS_VERSION,
        PERL_VERSION,
        NV_SIZE,
        PL_PERLDB,
        CLOCK_ID,
        CUMULATIVE_OVERHEAD_TICKS,
    ];

    /// Provisional known OPTION keys (OI-002-04 runway — fixture-expanded; not full freeze).
    ///
    /// Source: union of OPTION keys in the same golden fixture dumps.
    pub const KNOWN_OPTION_KEYS: &[&[u8]] = &[
        CALLS,
        BLOCKS,
        STMTS,
        COMPRESS,
        USECPUTIME,
        SUBS,
        LEAVE,
        EXPAND,
        TRACE,
        USE_DB_SUB,
        CLOCK,
        SLOWOPS,
        FINDCALLER,
        FORKDEPTH,
        PERLDB,
        NAMEEVALS,
        NAMEANONSUBS,
        EVALS,
    ];

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
/// Compact sample (subset) for always-inflate smoke: basetime, ticks_per_sec,
/// application; OPTION calls, blocks. Values are fixture-shaped string projections.
pub fn known_key_attr_option_sample_specs() -> [EventRecordSpec<'static>; 5] {
    [
        attribute_kv(known_key::BASETIME, b"1786111723"),
        attribute_kv(known_key::TICKS_PER_SEC, b"10000000"),
        attribute_kv(known_key::APPLICATION, b"workload.pl"),
        option_kv(known_key::CALLS, b"1"),
        option_kv(known_key::BLOCKS, b"0"),
    ]
}

/// Expanded dump-aligned sample covering **every** provisional known ATTRIBUTE/OPTION key
/// (fixture inventory expand preflight). Values are default-calls1-shaped projections.
pub fn known_key_attr_option_expanded_sample_specs() -> Vec<EventRecordSpec<'static>> {
    vec![
        // ATTRIBUTE (9 fixture keys)
        attribute_kv(known_key::BASETIME, b"1786111723"),
        attribute_kv(known_key::APPLICATION, b"workload.pl"),
        attribute_kv(known_key::PERL_VERSION, b"5.38.2"),
        attribute_kv(known_key::NV_SIZE, b"8"),
        attribute_kv(known_key::XS_VERSION, b"6.15"),
        attribute_kv(known_key::PL_PERLDB, b"3856"),
        attribute_kv(known_key::CLOCK_ID, b"1"),
        attribute_kv(known_key::TICKS_PER_SEC, b"10000000"),
        attribute_kv(known_key::CUMULATIVE_OVERHEAD_TICKS, b"4949"),
        // OPTION (18 fixture keys)
        option_kv(known_key::USECPUTIME, b"0"),
        option_kv(known_key::SUBS, b"1"),
        option_kv(known_key::BLOCKS, b"0"),
        option_kv(known_key::LEAVE, b"1"),
        option_kv(known_key::EXPAND, b"0"),
        option_kv(known_key::TRACE, b"0"),
        option_kv(known_key::USE_DB_SUB, b"0"),
        option_kv(known_key::COMPRESS, b"6"),
        option_kv(known_key::CLOCK, b"1"),
        option_kv(known_key::STMTS, b"1"),
        option_kv(known_key::SLOWOPS, b"2"),
        option_kv(known_key::FINDCALLER, b"0"),
        option_kv(known_key::FORKDEPTH, b"-1"),
        option_kv(known_key::PERLDB, b"0"),
        option_kv(known_key::NAMEEVALS, b"1"),
        option_kv(known_key::NAMEANONSUBS, b"1"),
        option_kv(known_key::CALLS, b"1"),
        option_kv(known_key::EVALS, b"0"),
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
    NewFid { fid: u64, filename: StringBlob<'a> },
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
    /// TIME_LINE_RUN packed form: same-site consecutive TIME_LINE samples.
    ///
    /// Decode expands to `ticks.len()` logical TIME_LINE records (same fid/line,
    /// each per-event ticks retained). Provisional — not permanent packing ADR.
    /// `ticks` must be non-empty and ≤ [`MAX_TIME_LINE_RUN_LEN`] for a round-trip
    /// that succeeds on decode.
    TimeLineRun {
        fid: u64,
        line: u64,
        ticks: &'a [u64],
    },
    /// TIME_BLOCK_RUN packed form: same-site consecutive TIME_BLOCK samples.
    ///
    /// Decode expands to `ticks.len()` logical TIME_BLOCK records (same
    /// fid/line/block_line, each per-event ticks retained). Provisional — not
    /// permanent packing ADR. `ticks` must be non-empty and ≤
    /// [`MAX_TIME_BLOCK_RUN_LEN`] for a round-trip that succeeds on decode.
    TimeBlockRun {
        fid: u64,
        line: u64,
        block_line: u64,
        ticks: &'a [u64],
    },
}

/// Fail-closed event-body errors (never panic on crafted input).
#[derive(Debug, PartialEq, Eq)]
pub enum EventBodyError {
    Varint(VarintError),
    String(StringError),
    Truncated {
        need: usize,
        got: usize,
    },
    Oversize {
        len: usize,
    },
    /// Opcode 0 is reserved.
    ReservedOpcode,
    /// Unknown opcode with `FLAG_OPCODE_REQUIRED` set.
    UnknownRequiredOpcode {
        opcode: u64,
    },
    /// Unknown optional opcode without [`FLAG_BODY_LENGTH`] — cannot skip safely.
    UnknownOpcode {
        opcode: u64,
    },
    /// Length-framed skip body exceeds [`MAX_SKIP_BODY_BYTES`].
    OversizeSkipBody {
        len: usize,
    },
    /// Site-delta reconstruction left the absolute domain of `u64`.
    InvalidSiteDelta,
    /// TIME_LINE_RUN declared length is zero (no expansion target).
    EmptyTimeLineRun,
    /// TIME_LINE_RUN declared length exceeds [`MAX_TIME_LINE_RUN_LEN`].
    OversizeTimeLineRun {
        len: usize,
    },
    /// TIME_BLOCK_RUN declared length is zero (no expansion target).
    EmptyTimeBlockRun,
    /// TIME_BLOCK_RUN declared length exceeds [`MAX_TIME_BLOCK_RUN_LEN`].
    OversizeTimeBlockRun {
        len: usize,
    },
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
            EventBodyError::InvalidSiteDelta => {
                write!(f, "invalid site-delta reconstruction (out of u64 domain)")
            }
            EventBodyError::EmptyTimeLineRun => {
                write!(f, "TIME_LINE_RUN with zero length (empty packed run)")
            }
            EventBodyError::OversizeTimeLineRun { len } => {
                write!(
                    f,
                    "oversize TIME_LINE_RUN length {len} (max {MAX_TIME_LINE_RUN_LEN})"
                )
            }
            EventBodyError::EmptyTimeBlockRun => {
                write!(f, "TIME_BLOCK_RUN with zero length (empty packed run)")
            }
            EventBodyError::OversizeTimeBlockRun { len } => {
                write!(
                    f,
                    "oversize TIME_BLOCK_RUN length {len} (max {MAX_TIME_BLOCK_RUN_LEN})"
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
            out.extend_from_slice(&encode_string_blob(*key_string_id, *key_string_flags, key));
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
            out.extend_from_slice(&encode_string_blob(*key_string_id, *key_string_flags, key));
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
        EventRecordSpec::TimeLineRun { fid, line, ticks } => {
            out.extend_from_slice(&encode_u64(opcode::TIME_LINE_RUN));
            out.push(0); // flags — absolute site; not FLAG_SITE_DELTA
            out.extend_from_slice(&encode_u64(*fid));
            out.extend_from_slice(&encode_u64(*line));
            out.extend_from_slice(&encode_u64(ticks.len() as u64));
            for t in *ticks {
                out.extend_from_slice(&encode_u64(*t));
            }
        }
        EventRecordSpec::TimeBlockRun {
            fid,
            line,
            block_line,
            ticks,
        } => {
            out.extend_from_slice(&encode_u64(opcode::TIME_BLOCK_RUN));
            out.push(0); // flags — absolute site; not FLAG_SITE_DELTA
            out.extend_from_slice(&encode_u64(*fid));
            out.extend_from_slice(&encode_u64(*line));
            out.extend_from_slice(&encode_u64(*block_line));
            out.extend_from_slice(&encode_u64(ticks.len() as u64));
            for t in *ticks {
                out.extend_from_slice(&encode_u64(*t));
            }
        }
    }
}

/// Encode a provisional event-body (codec NONE payload): ordered records.
///
/// Empty `records` yields an empty body (valid). Pure byte-slice / `Vec` API.
/// Site fields on TIME_LINE / TIME_BLOCK / SUB_ENTRY are **absolute** ULEB.
/// Does **not** emit [`FLAG_HAS_SEQ`] (use [`encode_event_body_with_seq`]).
pub fn encode_event_body(records: &[EventRecordSpec<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    for rec in records {
        encode_record_into(&mut out, rec);
    }
    out
}

/// Encode event-body with provisional **monotonic logical sequence numbers**.
///
/// Each logical recovered event is assigned `0, 1, 2, …` in stream order.
/// Wire: `opcode || flags(|FLAG_HAS_SEQ) || ULEB128 seq || typed-body`.
/// Packed runs write a **base** sequence for the wire record; decode expands to
/// `base .. base+N-1` (one seq per logical TIME_LINE / TIME_BLOCK).
///
/// Not OI-001-03 / COL-003 freeze of VERSION/START_DEFLATE dual-output participation.
pub fn encode_event_body_with_seq(records: &[EventRecordSpec<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut next = 0u64;
    for rec in records {
        match rec {
            EventRecordSpec::TimeLineRun { fid, line, ticks } => {
                let base = next;
                out.extend_from_slice(&encode_u64(opcode::TIME_LINE_RUN));
                out.push(FLAG_HAS_SEQ);
                out.extend_from_slice(&encode_u64(base));
                out.extend_from_slice(&encode_u64(*fid));
                out.extend_from_slice(&encode_u64(*line));
                out.extend_from_slice(&encode_u64(ticks.len() as u64));
                for t in *ticks {
                    out.extend_from_slice(&encode_u64(*t));
                }
                next = next.saturating_add(ticks.len() as u64);
            }
            EventRecordSpec::TimeBlockRun {
                fid,
                line,
                block_line,
                ticks,
            } => {
                let base = next;
                out.extend_from_slice(&encode_u64(opcode::TIME_BLOCK_RUN));
                out.push(FLAG_HAS_SEQ);
                out.extend_from_slice(&encode_u64(base));
                out.extend_from_slice(&encode_u64(*fid));
                out.extend_from_slice(&encode_u64(*line));
                out.extend_from_slice(&encode_u64(*block_line));
                out.extend_from_slice(&encode_u64(ticks.len() as u64));
                for t in *ticks {
                    out.extend_from_slice(&encode_u64(*t));
                }
                next = next.saturating_add(ticks.len() as u64);
            }
            other => {
                encode_record_with_seq_into(&mut out, other, next);
                next = next.saturating_add(1);
            }
        }
    }
    out
}

/// Encode one absolute record with [`FLAG_HAS_SEQ`] and the given sequence value.
fn encode_record_with_seq_into(out: &mut Vec<u8>, rec: &EventRecordSpec<'_>, seq: u64) {
    // Reuse absolute typed-body layout from encode_record_into, but rewrite the
    // flags byte to FLAG_HAS_SEQ and insert the seq ULEB after it.
    let mut tmp = Vec::new();
    encode_record_into(&mut tmp, rec);
    // tmp = opcode_uleb || flags(0) || typed_body
    let (op, n_op) = decode_u64(&tmp, 0).expect("encode_record_into produces valid opcode");
    debug_assert_eq!(tmp[n_op], 0, "encode_record_into uses flags 0");
    out.extend_from_slice(&encode_u64(op));
    out.push(FLAG_HAS_SEQ);
    out.extend_from_slice(&encode_u64(seq));
    out.extend_from_slice(&tmp[n_op + 1..]);
}

/// Running site bases for provisional location-delta encode/decode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SiteCursor {
    fid: u64,
    line: u64,
    block_line: u64,
    caller_fid: u64,
    caller_line: u64,
}

fn apply_u64_delta(base: u64, delta: i64) -> EventBodyResult<u64> {
    let v = i128::from(base)
        .checked_add(i128::from(delta))
        .ok_or(EventBodyError::InvalidSiteDelta)?;
    if v < 0 || v > i128::from(u64::MAX) {
        return Err(EventBodyError::InvalidSiteDelta);
    }
    Ok(v as u64)
}

fn i64_delta(from: u64, to: u64) -> EventBodyResult<i64> {
    let d = i128::from(to)
        .checked_sub(i128::from(from))
        .ok_or(EventBodyError::InvalidSiteDelta)?;
    if d < i128::from(i64::MIN) || d > i128::from(i64::MAX) {
        return Err(EventBodyError::InvalidSiteDelta);
    }
    Ok(d as i64)
}

/// Encode event-body with provisional **site deltas** for TIME_LINE / TIME_BLOCK / SUB_ENTRY.
///
/// Other opcodes are encoded absolute (same as [`encode_event_body`]). Site fields use
/// ZigZag signed deltas relative to a running base starting at `(0,0)`; reconstructed
/// absolute sites on decode match the input specs. Sets [`FLAG_SITE_DELTA`] on those
/// records. Not a permanent packing freeze.
pub fn encode_event_body_with_site_deltas(
    records: &[EventRecordSpec<'_>],
) -> EventBodyResult<Vec<u8>> {
    let mut out = Vec::new();
    let mut site = SiteCursor::default();
    for rec in records {
        match rec {
            EventRecordSpec::TimeLine { fid, line, ticks } => {
                let df = i64_delta(site.fid, *fid)?;
                let dl = i64_delta(site.line, *line)?;
                out.extend_from_slice(&encode_u64(opcode::TIME_LINE));
                out.push(FLAG_SITE_DELTA);
                out.extend_from_slice(&encode_i64(df));
                out.extend_from_slice(&encode_i64(dl));
                out.extend_from_slice(&encode_u64(*ticks));
                site.fid = *fid;
                site.line = *line;
            }
            EventRecordSpec::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => {
                let df = i64_delta(site.fid, *fid)?;
                let dl = i64_delta(site.line, *line)?;
                let db = i64_delta(site.block_line, *block_line)?;
                out.extend_from_slice(&encode_u64(opcode::TIME_BLOCK));
                out.push(FLAG_SITE_DELTA);
                out.extend_from_slice(&encode_i64(df));
                out.extend_from_slice(&encode_i64(dl));
                out.extend_from_slice(&encode_i64(db));
                out.extend_from_slice(&encode_u64(*ticks));
                site.fid = *fid;
                site.line = *line;
                site.block_line = *block_line;
            }
            EventRecordSpec::SubEntry {
                caller_fid,
                caller_line,
            } => {
                let df = i64_delta(site.caller_fid, *caller_fid)?;
                let dl = i64_delta(site.caller_line, *caller_line)?;
                out.extend_from_slice(&encode_u64(opcode::SUB_ENTRY));
                out.push(FLAG_SITE_DELTA);
                out.extend_from_slice(&encode_i64(df));
                out.extend_from_slice(&encode_i64(dl));
                site.caller_fid = *caller_fid;
                site.caller_line = *caller_line;
            }
            // Packed runs are absolute on the wire, but decode advances SiteCursor to
            // the run site — keep encode cursor in sync so a following site-delta
            // TIME_LINE/TIME_BLOCK reconstructs correctly.
            EventRecordSpec::TimeLineRun { fid, line, .. } => {
                encode_record_into(&mut out, rec);
                site.fid = *fid;
                site.line = *line;
            }
            EventRecordSpec::TimeBlockRun {
                fid,
                line,
                block_line,
                ..
            } => {
                encode_record_into(&mut out, rec);
                site.fid = *fid;
                site.line = *line;
                site.block_line = *block_line;
            }
            other => encode_record_into(&mut out, other),
        }
    }
    Ok(out)
}

/// Running packing-encode state for **multi-chunk continuity** of site bases and
/// logical sequence numbers (OI packing runway).
///
/// Pass the same state across consecutive record-aligned partitions so site-delta
/// and sequence numbers **do not reset** at chunk boundaries. Decode joins plains
/// and reconstructs with a single continuous cursor — encode must match.
///
/// Not a permanent packing ADR.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PackingEncodeState {
    site: SiteCursor,
    next_seq: u64,
}

impl PackingEncodeState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Next logical sequence number that will be assigned.
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }
}

/// Encode event-body with **composed** provisional packing: site deltas **and**
/// monotonic logical sequence numbers on the same wire records.
///
/// - TIME_LINE / TIME_BLOCK / SUB_ENTRY: flags `FLAG_SITE_DELTA | FLAG_HAS_SEQ`,
///   wire order `opcode || flags || ULEB seq || ZigZag site deltas || …ticks…`.
/// - Other opcodes (incl. packed runs): absolute body + [`FLAG_HAS_SEQ`] only
///   (same as [`encode_event_body_with_seq`]); runs still expand base..base+N-1.
///
/// Starts packing state at defaults (site bases 0, seq 0). For multi-chunk
/// continuity use [`encode_event_body_with_site_deltas_and_seq_continuing`].
///
/// Decode recovers absolute sites and per-event sequences via
/// [`decode_event_body_full`]. Not a permanent packing ADR / flag freeze.
pub fn encode_event_body_with_site_deltas_and_seq(
    records: &[EventRecordSpec<'_>],
) -> EventBodyResult<Vec<u8>> {
    let mut state = PackingEncodeState::new();
    encode_event_body_with_site_deltas_and_seq_continuing(records, &mut state)
}

/// Encode one record-aligned partition of a packing stream, **continuing** site
/// bases and sequence numbers from `state` (updated in place).
///
/// Multi-chunk preflight: call once per partition from
/// [`crate::multi_chunk_event::partition_event_records`] with the same `state`.
/// Concatenating partition plains must equal a single-chunk
/// [`encode_event_body_with_site_deltas_and_seq`] of the full record list.
pub fn encode_event_body_with_site_deltas_and_seq_continuing(
    records: &[EventRecordSpec<'_>],
    state: &mut PackingEncodeState,
) -> EventBodyResult<Vec<u8>> {
    let mut out = Vec::new();
    let site = &mut state.site;
    let next = &mut state.next_seq;
    for rec in records {
        match rec {
            EventRecordSpec::TimeLine { fid, line, ticks } => {
                let df = i64_delta(site.fid, *fid)?;
                let dl = i64_delta(site.line, *line)?;
                out.extend_from_slice(&encode_u64(opcode::TIME_LINE));
                out.push(FLAG_SITE_DELTA | FLAG_HAS_SEQ);
                out.extend_from_slice(&encode_u64(*next));
                out.extend_from_slice(&encode_i64(df));
                out.extend_from_slice(&encode_i64(dl));
                out.extend_from_slice(&encode_u64(*ticks));
                site.fid = *fid;
                site.line = *line;
                *next = next.saturating_add(1);
            }
            EventRecordSpec::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => {
                let df = i64_delta(site.fid, *fid)?;
                let dl = i64_delta(site.line, *line)?;
                let db = i64_delta(site.block_line, *block_line)?;
                out.extend_from_slice(&encode_u64(opcode::TIME_BLOCK));
                out.push(FLAG_SITE_DELTA | FLAG_HAS_SEQ);
                out.extend_from_slice(&encode_u64(*next));
                out.extend_from_slice(&encode_i64(df));
                out.extend_from_slice(&encode_i64(dl));
                out.extend_from_slice(&encode_i64(db));
                out.extend_from_slice(&encode_u64(*ticks));
                site.fid = *fid;
                site.line = *line;
                site.block_line = *block_line;
                *next = next.saturating_add(1);
            }
            EventRecordSpec::SubEntry {
                caller_fid,
                caller_line,
            } => {
                let df = i64_delta(site.caller_fid, *caller_fid)?;
                let dl = i64_delta(site.caller_line, *caller_line)?;
                out.extend_from_slice(&encode_u64(opcode::SUB_ENTRY));
                out.push(FLAG_SITE_DELTA | FLAG_HAS_SEQ);
                out.extend_from_slice(&encode_u64(*next));
                out.extend_from_slice(&encode_i64(df));
                out.extend_from_slice(&encode_i64(dl));
                site.caller_fid = *caller_fid;
                site.caller_line = *caller_line;
                *next = next.saturating_add(1);
            }
            EventRecordSpec::TimeLineRun { fid, line, ticks } => {
                let base = *next;
                out.extend_from_slice(&encode_u64(opcode::TIME_LINE_RUN));
                out.push(FLAG_HAS_SEQ);
                out.extend_from_slice(&encode_u64(base));
                out.extend_from_slice(&encode_u64(*fid));
                out.extend_from_slice(&encode_u64(*line));
                out.extend_from_slice(&encode_u64(ticks.len() as u64));
                for t in *ticks {
                    out.extend_from_slice(&encode_u64(*t));
                }
                // Match decode: runs advance SiteCursor so following site-delta sites
                // are relative to the run location (not the pre-run base).
                site.fid = *fid;
                site.line = *line;
                *next = next.saturating_add(ticks.len() as u64);
            }
            EventRecordSpec::TimeBlockRun {
                fid,
                line,
                block_line,
                ticks,
            } => {
                let base = *next;
                out.extend_from_slice(&encode_u64(opcode::TIME_BLOCK_RUN));
                out.push(FLAG_HAS_SEQ);
                out.extend_from_slice(&encode_u64(base));
                out.extend_from_slice(&encode_u64(*fid));
                out.extend_from_slice(&encode_u64(*line));
                out.extend_from_slice(&encode_u64(*block_line));
                out.extend_from_slice(&encode_u64(ticks.len() as u64));
                for t in *ticks {
                    out.extend_from_slice(&encode_u64(*t));
                }
                site.fid = *fid;
                site.line = *line;
                site.block_line = *block_line;
                *next = next.saturating_add(ticks.len() as u64);
            }
            other => {
                encode_record_with_seq_into(&mut out, other, *next);
                *next = next.saturating_add(1);
            }
        }
    }
    Ok(out)
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

/// Decode one wire record starting at `pos`, pushing zero or more logical records
/// into `out` and parallel sequence slots into `seqs` (same length).
///
/// Known opcodes push one logical record (except packed-run opcodes:
/// [`opcode::TIME_LINE_RUN`] expands to N logical TIME_LINE records;
/// [`opcode::TIME_BLOCK_RUN`] expands to N logical TIME_BLOCK records).
/// Unknown optional length-framed opcodes push nothing (skipped). Fail-closed otherwise.
/// `site` tracks running bases for [`FLAG_SITE_DELTA`] TIME_LINE / TIME_BLOCK / SUB_ENTRY.
/// When [`FLAG_HAS_SEQ`] is set, a ULEB sequence is read after flags (before typed body);
/// packed runs use that value as the **base** for expanded logical events.
/// Returns bytes consumed.
fn decode_record<'a>(
    data: &'a [u8],
    pos: usize,
    site: &mut SiteCursor,
    out: &mut Vec<EventRecord<'a>>,
    seqs: &mut Vec<Option<u64>>,
) -> EventBodyResult<usize> {
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
        // FLAG_HAS_SEQ is not defined for unknown skip preflight; ignore if set.
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
            return Ok(p - pos);
        }
        return Err(EventBodyError::UnknownOpcode { opcode: op });
    }

    // Optional logical sequence number (OI-001-03 runway).
    let wire_seq = if (flags & FLAG_HAS_SEQ) != 0 {
        let (s, n_s) = decode_u64(data, p)?;
        p += n_s;
        Some(s)
    } else {
        None
    };

    match op {
        opcode::MARK => {
            let (label, n_str) = decode_string_blob(data, p)?;
            p += n_str;
            out.push(EventRecord::Mark { label });
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::TIME_LINE => {
            let (fid, line) = if (flags & FLAG_SITE_DELTA) != 0 {
                let (df, n1) = decode_i64(data, p)?;
                p += n1;
                let (dl, n2) = decode_i64(data, p)?;
                p += n2;
                let fid = apply_u64_delta(site.fid, df)?;
                let line = apply_u64_delta(site.line, dl)?;
                site.fid = fid;
                site.line = line;
                (fid, line)
            } else {
                let (fid, n1) = decode_u64(data, p)?;
                p += n1;
                let (line, n2) = decode_u64(data, p)?;
                p += n2;
                site.fid = fid;
                site.line = line;
                (fid, line)
            };
            let (ticks, n3) = decode_u64(data, p)?;
            p += n3;
            out.push(EventRecord::TimeLine { fid, line, ticks });
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::TIME_BLOCK => {
            let (fid, line, block_line) = if (flags & FLAG_SITE_DELTA) != 0 {
                let (df, n1) = decode_i64(data, p)?;
                p += n1;
                let (dl, n2) = decode_i64(data, p)?;
                p += n2;
                let (db, n3) = decode_i64(data, p)?;
                p += n3;
                let fid = apply_u64_delta(site.fid, df)?;
                let line = apply_u64_delta(site.line, dl)?;
                let block_line = apply_u64_delta(site.block_line, db)?;
                site.fid = fid;
                site.line = line;
                site.block_line = block_line;
                (fid, line, block_line)
            } else {
                let (fid, n1) = decode_u64(data, p)?;
                p += n1;
                let (line, n2) = decode_u64(data, p)?;
                p += n2;
                let (block_line, n3) = decode_u64(data, p)?;
                p += n3;
                site.fid = fid;
                site.line = line;
                site.block_line = block_line;
                (fid, line, block_line)
            };
            let (ticks, n4) = decode_u64(data, p)?;
            p += n4;
            out.push(EventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            });
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::SUB_ENTRY => {
            let (caller_fid, caller_line) = if (flags & FLAG_SITE_DELTA) != 0 {
                let (df, n1) = decode_i64(data, p)?;
                p += n1;
                let (dl, n2) = decode_i64(data, p)?;
                p += n2;
                let caller_fid = apply_u64_delta(site.caller_fid, df)?;
                let caller_line = apply_u64_delta(site.caller_line, dl)?;
                site.caller_fid = caller_fid;
                site.caller_line = caller_line;
                (caller_fid, caller_line)
            } else {
                let (caller_fid, n1) = decode_u64(data, p)?;
                p += n1;
                let (caller_line, n2) = decode_u64(data, p)?;
                p += n2;
                site.caller_fid = caller_fid;
                site.caller_line = caller_line;
                (caller_fid, caller_line)
            };
            out.push(EventRecord::SubEntry {
                caller_fid,
                caller_line,
            });
            seqs.push(wire_seq);
            Ok(p - pos)
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
            out.push(EventRecord::SubReturn {
                depth,
                incl,
                excl,
                subname,
            });
            seqs.push(wire_seq);
            Ok(p - pos)
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
            out.push(EventRecord::SubInfo {
                fid,
                first_line,
                last_line,
                name,
            });
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::SRC_LINE => {
            let (fid, n1) = decode_u64(data, p)?;
            p += n1;
            let (line, n2) = decode_u64(data, p)?;
            p += n2;
            let (text, n_str) = decode_string_blob(data, p)?;
            p += n_str;
            out.push(EventRecord::SrcLine { fid, line, text });
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::NEW_FID => {
            let (fid, n1) = decode_u64(data, p)?;
            p += n1;
            let (filename, n_str) = decode_string_blob(data, p)?;
            p += n_str;
            out.push(EventRecord::NewFid { fid, filename });
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::PID_START => {
            let (pid, n1) = decode_u64(data, p)?;
            p += n1;
            let (ppid, n2) = decode_u64(data, p)?;
            p += n2;
            let (start_time, n3) = decode_u64(data, p)?;
            p += n3;
            out.push(EventRecord::PidStart {
                pid,
                ppid,
                start_time,
            });
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::PID_END => {
            let (pid, n1) = decode_u64(data, p)?;
            p += n1;
            let (end_time, n2) = decode_u64(data, p)?;
            p += n2;
            out.push(EventRecord::PidEnd { pid, end_time });
            seqs.push(wire_seq);
            Ok(p - pos)
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
            out.push(EventRecord::SubCallers {
                fid,
                line,
                count,
                incl,
                excl,
                reci,
                rec_depth,
                called,
                caller,
            });
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::DISCOUNT => {
            out.push(EventRecord::Discount);
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::ATTRIBUTE => {
            let (key, n_key) = decode_string_blob(data, p)?;
            p += n_key;
            let (value, n_val) = decode_string_blob(data, p)?;
            p += n_val;
            out.push(EventRecord::Attribute { key, value });
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::OPTION => {
            let (key, n_key) = decode_string_blob(data, p)?;
            p += n_key;
            let (value, n_val) = decode_string_blob(data, p)?;
            p += n_val;
            out.push(EventRecord::Option { key, value });
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::COMMENT => {
            let (text, n_str) = decode_string_blob(data, p)?;
            p += n_str;
            out.push(EventRecord::Comment { text });
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::START_DEFLATE => {
            out.push(EventRecord::StartDeflate);
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::VERSION => {
            let (major, n1) = decode_u64(data, p)?;
            p += n1;
            let (minor, n2) = decode_u64(data, p)?;
            p += n2;
            out.push(EventRecord::Version { major, minor });
            seqs.push(wire_seq);
            Ok(p - pos)
        }
        opcode::TIME_LINE_RUN => {
            // Absolute site only (FLAG_SITE_DELTA not defined for run form).
            let (fid, n1) = decode_u64(data, p)?;
            p += n1;
            let (line, n2) = decode_u64(data, p)?;
            p += n2;
            let (count_u, n3) = decode_u64(data, p)?;
            p += n3;
            if count_u == 0 {
                return Err(EventBodyError::EmptyTimeLineRun);
            }
            if count_u > MAX_TIME_LINE_RUN_LEN as u64 {
                return Err(EventBodyError::OversizeTimeLineRun {
                    len: count_u as usize,
                });
            }
            let count = count_u as usize;
            // Fail-closed before allocating N records if remaining bytes cannot
            // hold at least one byte per tick (ULEB lower bound).
            let remaining = data.len().saturating_sub(p);
            if remaining < count {
                return Err(EventBodyError::Truncated {
                    need: p + count,
                    got: data.len(),
                });
            }
            site.fid = fid;
            site.line = line;
            for i in 0..count {
                let (ticks, n_t) = decode_u64(data, p)?;
                p += n_t;
                out.push(EventRecord::TimeLine { fid, line, ticks });
                seqs.push(wire_seq.map(|b| b.saturating_add(i as u64)));
            }
            Ok(p - pos)
        }
        opcode::TIME_BLOCK_RUN => {
            // Absolute site only (FLAG_SITE_DELTA not defined for run form).
            let (fid, n1) = decode_u64(data, p)?;
            p += n1;
            let (line, n2) = decode_u64(data, p)?;
            p += n2;
            let (block_line, n3) = decode_u64(data, p)?;
            p += n3;
            let (count_u, n4) = decode_u64(data, p)?;
            p += n4;
            if count_u == 0 {
                return Err(EventBodyError::EmptyTimeBlockRun);
            }
            if count_u > MAX_TIME_BLOCK_RUN_LEN as u64 {
                return Err(EventBodyError::OversizeTimeBlockRun {
                    len: count_u as usize,
                });
            }
            let count = count_u as usize;
            let remaining = data.len().saturating_sub(p);
            if remaining < count {
                return Err(EventBodyError::Truncated {
                    need: p + count,
                    got: data.len(),
                });
            }
            site.fid = fid;
            site.line = line;
            site.block_line = block_line;
            for i in 0..count {
                let (ticks, n_t) = decode_u64(data, p)?;
                p += n_t;
                out.push(EventRecord::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                });
                seqs.push(wire_seq.map(|b| b.saturating_add(i as u64)));
            }
            Ok(p - pos)
        }
        _ => unreachable!("is_known_opcode filtered"),
    }
}

/// Decoded event-body: logical records plus parallel provisional sequence slots.
///
/// `sequences[i]` is `Some(n)` when the producing wire record (or packed-run base
/// + offset) carried [`FLAG_HAS_SEQ`]; otherwise `None`. Length always equals
/// `records.len()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventBodyDecoded<'a> {
    pub records: Vec<EventRecord<'a>>,
    pub sequences: Vec<Option<u64>>,
}

/// Decode a provisional event-body until the buffer is exhausted (full form).
///
/// Same fail-closed rules as [`decode_event_body`], and recovers optional
/// logical sequence numbers when [`FLAG_HAS_SEQ`] is set.
pub fn decode_event_body_full(data: &[u8]) -> EventBodyResult<(EventBodyDecoded<'_>, usize)> {
    if data.len() > MAX_EVENT_BODY_BYTES {
        return Err(EventBodyError::Oversize { len: data.len() });
    }
    let mut pos = 0;
    let mut out = Vec::new();
    let mut seqs = Vec::new();
    let mut site = SiteCursor::default();
    while pos < data.len() {
        if pos > MAX_EVENT_BODY_BYTES {
            return Err(EventBodyError::Oversize { len: pos });
        }
        let n = decode_record(data, pos, &mut site, &mut out, &mut seqs)?;
        pos += n;
    }
    debug_assert_eq!(out.len(), seqs.len());
    Ok((
        EventBodyDecoded {
            records: out,
            sequences: seqs,
        },
        pos,
    ))
}

/// Decode a provisional event-body until the buffer is exhausted.
///
/// Empty input → empty record list. Fail-closed on truncated mid-record,
/// reserved opcode 0, unknown **required** opcodes, and unknown optional opcodes
/// without [`FLAG_BODY_LENGTH`]. Unknown optional opcodes with length framing are
/// **skipped** (not emitted). TIME_LINE / TIME_BLOCK / SUB_ENTRY with
/// [`FLAG_SITE_DELTA`] reconstruct absolute sites from a running base.
/// [`opcode::TIME_LINE_RUN`] expands to N ordered logical TIME_LINE records
/// (same site, every per-event ticks retained).
/// [`opcode::TIME_BLOCK_RUN`] expands to N ordered logical TIME_BLOCK records
/// (same fid/line/block_line, every per-event ticks retained).
/// Sequence numbers (when present) are available via [`decode_event_body_full`].
/// Returns `(records, bytes_consumed)` (`bytes_consumed == data.len()` on success).
pub fn decode_event_body(data: &[u8]) -> EventBodyResult<(Vec<EventRecord<'_>>, usize)> {
    let (decoded, n) = decode_event_body_full(data)?;
    Ok((decoded.records, n))
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
        assert_eq!(decode_event_body(&bad), Err(EventBodyError::ReservedOpcode));
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
        let mut body = encode_event_body(&[EventRecordSpec::Version { major: 6, minor: 0 }]);
        body.extend_from_slice(&encode_unknown_optional_skip_record(100, b"").unwrap());
        body.extend_from_slice(&encode_event_body(&[EventRecordSpec::Discount]));
        let (recs, n) = decode_event_body(&body).unwrap();
        assert_eq!(n, body.len());
        assert_eq!(recs.len(), 2);
        assert!(matches!(
            recs[0],
            EventRecord::Version { major: 6, minor: 0 }
        ));
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
        assert!(is_known_opcode(opcode::TIME_LINE_RUN));
        assert!(is_known_opcode(opcode::TIME_BLOCK_RUN));
        assert!(!is_known_opcode(opcode::RESERVED));
        assert!(!is_known_opcode(99));
    }

    #[test]
    fn version_roundtrip() {
        let specs = [
            EventRecordSpec::Version { major: 5, minor: 0 },
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
            EventRecordSpec::Version { major: 5, minor: 0 },
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
        assert!(
            ranks.windows(2).all(|w| w[0] <= w[1]),
            "order ranks {ranks:?}"
        );
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
        let mut enc = encode_event_body(&[EventRecordSpec::Version { major: 5, minor: 0 }]);
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
        // Fixture-expanded inventory.
        assert!(known_key::is_known_attribute_key(known_key::PERL_VERSION));
        assert!(known_key::is_known_attribute_key(known_key::NV_SIZE));
        assert!(known_key::is_known_attribute_key(known_key::PL_PERLDB));
        assert!(known_key::is_known_attribute_key(known_key::CLOCK_ID));
        assert!(known_key::is_known_attribute_key(
            known_key::CUMULATIVE_OVERHEAD_TICKS
        ));
        assert!(known_key::is_known_option_key(known_key::SLOWOPS));
        assert!(known_key::is_known_option_key(known_key::FORKDEPTH));
        assert!(known_key::is_known_option_key(known_key::EVALS));
        assert!(known_key::is_known_meta_key(b"basetime"));
        assert!(known_key::is_known_meta_key(b"calls"));
        assert!(!known_key::is_known_attribute_key(b"not-a-key"));
        assert!(!known_key::is_known_option_key(b"not-a-key"));
        assert!(!known_key::is_known_meta_key(b""));
        // Tables match fixture inventory sizes (9 ATTRIBUTE + 18 OPTION).
        assert_eq!(known_key::KNOWN_ATTRIBUTE_KEYS.len(), 9);
        assert_eq!(known_key::KNOWN_OPTION_KEYS.len(), 18);
        assert!(known_key::KNOWN_ATTRIBUTE_KEYS.contains(&known_key::BASETIME));
        assert!(known_key::KNOWN_OPTION_KEYS.contains(&known_key::CALLS));
    }

    /// Collect ATTRIBUTE / OPTION keys from a golden `readstream.jsonl` dump path.
    ///
    /// Expects lines shaped like `{"args":["key","value"],…,"tag":"ATTRIBUTE"}`.
    /// Uses a minimal parser (no serde) so tests drive real fixture files.
    fn collect_attr_option_keys_from_jsonl(
        path: &std::path::Path,
    ) -> (
        std::collections::BTreeSet<Vec<u8>>,
        std::collections::BTreeSet<Vec<u8>>,
    ) {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let mut attrs = std::collections::BTreeSet::new();
        let mut opts = std::collections::BTreeSet::new();
        for (lineno, line) in text.lines().enumerate() {
            let is_attr = line.contains("\"tag\":\"ATTRIBUTE\"");
            let is_opt = line.contains("\"tag\":\"OPTION\"");
            if !is_attr && !is_opt {
                continue;
            }
            let key = first_args_string(line).unwrap_or_else(|| {
                panic!(
                    "{}:{}: missing args[0] string for ATTRIBUTE/OPTION",
                    path.display(),
                    lineno + 1
                )
            });
            if is_attr {
                attrs.insert(key);
            } else {
                opts.insert(key);
            }
        }
        (attrs, opts)
    }

    fn first_args_string(line: &str) -> Option<Vec<u8>> {
        // "args":["key",...
        let marker = "\"args\":[\"";
        let start = line.find(marker)? + marker.len();
        let end = line[start..].find('"')? + start;
        Some(line[start..end].as_bytes().to_vec())
    }

    fn fixture_readstream(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/v5")
            .join(name)
            .join("readstream.jsonl")
    }

    #[test]
    fn known_key_covers_all_keys_in_golden_fixture_jsonl() {
        // Real golden dumps are the source of truth for this inventory expand.
        let fixtures = [
            "default-calls1",
            "calls2-default",
            "blocks-calls1",
            "default-calls2",
        ];
        let mut all_attr = std::collections::BTreeSet::new();
        let mut all_opt = std::collections::BTreeSet::new();
        for name in fixtures {
            let path = fixture_readstream(name);
            assert!(path.is_file(), "missing golden fixture {}", path.display());
            let (attrs, opts) = collect_attr_option_keys_from_jsonl(&path);
            assert!(!attrs.is_empty(), "{name}: expected ATTRIBUTE keys in dump");
            assert!(!opts.is_empty(), "{name}: expected OPTION keys in dump");
            for k in &attrs {
                assert!(
                    known_key::is_known_attribute_key(k),
                    "{name}: ATTRIBUTE key {:?} not in known_key table",
                    String::from_utf8_lossy(k)
                );
            }
            for k in &opts {
                assert!(
                    known_key::is_known_option_key(k),
                    "{name}: OPTION key {:?} not in known_key table",
                    String::from_utf8_lossy(k)
                );
            }
            all_attr.extend(attrs);
            all_opt.extend(opts);
        }
        // Table is exactly the fixture union (no phantom keys left unobserved).
        assert_eq!(all_attr.len(), known_key::KNOWN_ATTRIBUTE_KEYS.len());
        assert_eq!(all_opt.len(), known_key::KNOWN_OPTION_KEYS.len());
        for k in known_key::KNOWN_ATTRIBUTE_KEYS {
            assert!(
                all_attr.contains(*k),
                "table ATTRIBUTE key {:?} never seen in fixtures",
                String::from_utf8_lossy(k)
            );
        }
        for k in known_key::KNOWN_OPTION_KEYS {
            assert!(
                all_opt.contains(*k),
                "table OPTION key {:?} never seen in fixtures",
                String::from_utf8_lossy(k)
            );
        }
    }

    #[test]
    fn known_key_expanded_sample_roundtrip_and_freeform_unknown() {
        let specs = known_key_attr_option_expanded_sample_specs();
        assert_eq!(
            specs.len(),
            known_key::KNOWN_ATTRIBUTE_KEYS.len() + known_key::KNOWN_OPTION_KEYS.len()
        );
        for s in &specs {
            match s {
                EventRecordSpec::Attribute { key, .. } => {
                    assert!(known_key::is_known_attribute_key(key), "ATTRIBUTE {key:?}");
                }
                EventRecordSpec::Option { key, .. } => {
                    assert!(known_key::is_known_option_key(key), "OPTION {key:?}");
                }
                other => panic!("expected Attribute/Option, got {other:?}"),
            }
        }
        let enc = encode_event_body(&specs);
        let (recs, n) = decode_event_body(&enc).expect("expanded known-key roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), specs.len());
        for (i, r) in recs.iter().enumerate() {
            match (r, &specs[i]) {
                (
                    EventRecord::Attribute { key, value },
                    EventRecordSpec::Attribute {
                        key: sk, value: sv, ..
                    },
                ) => {
                    assert_eq!(key.data, *sk);
                    assert_eq!(value.data, *sv);
                    assert!(known_key::is_known_attribute_key(key.data));
                }
                (
                    EventRecord::Option { key, value },
                    EventRecordSpec::Option {
                        key: sk, value: sv, ..
                    },
                ) => {
                    assert_eq!(key.data, *sk);
                    assert_eq!(value.data, *sv);
                    assert!(known_key::is_known_option_key(key.data));
                }
                other => panic!("pair {i}: {other:?}"),
            }
        }
        // Free-form unknown key still encodes/decodes (not reject-unknown freeze).
        let free = encode_event_body(&[
            attribute_kv(b"not-in-known-table", b"x"),
            option_kv(b"also-unknown", b"y"),
        ]);
        let (free_recs, _) = decode_event_body(&free).expect("free-form unknown keys");
        assert_eq!(free_recs.len(), 2);
        match &free_recs[0] {
            EventRecord::Attribute { key, value } => {
                assert_eq!(key.data, b"not-in-known-table");
                assert_eq!(value.data, b"x");
                assert!(!known_key::is_known_attribute_key(key.data));
            }
            other => panic!("{other:?}"),
        }
        match &free_recs[1] {
            EventRecord::Option { key, value } => {
                assert_eq!(key.data, b"also-unknown");
                assert_eq!(value.data, b"y");
                assert!(!known_key::is_known_option_key(key.data));
            }
            other => panic!("{other:?}"),
        }
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

    #[test]
    fn site_delta_time_line_reconstructs_absolute_sequence() {
        let specs = [
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
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 1,
                ticks: 7,
            },
        ];
        let enc = encode_event_body_with_site_deltas(&specs).expect("encode deltas");
        // Delta wire differs from absolute encode.
        let abs = encode_event_body(&specs);
        assert_ne!(enc, abs, "site-delta body must not equal absolute body");
        assert!(enc.contains(&FLAG_SITE_DELTA));

        let (recs, n) = decode_event_body(&enc).expect("decode deltas");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 3);
        match &recs[0] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("[0] {other:?}"),
        }
        match &recs[1] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 11, 6));
            }
            other => panic!("[1] {other:?}"),
        }
        match &recs[2] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 1, 7));
            }
            other => panic!("[2] {other:?}"),
        }
    }

    #[test]
    fn site_delta_time_block_and_sub_entry_roundtrip() {
        let specs = [
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 5,
                block_line: 4,
                ticks: 780,
            },
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 6,
                block_line: 4,
                ticks: 10,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 1,
                caller_line: 10,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 1,
                caller_line: 12,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"after-sites",
            },
        ];
        let enc = encode_event_body_with_site_deltas(&specs).unwrap();
        let (recs, n) = decode_event_body(&enc).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 5);
        match &recs[0] {
            EventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (1, 5, 4, 780)),
            other => panic!("{other:?}"),
        }
        match &recs[1] {
            EventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (1, 6, 4, 10)),
            other => panic!("{other:?}"),
        }
        match &recs[2] {
            EventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => assert_eq!((*caller_fid, *caller_line), (1, 10)),
            other => panic!("{other:?}"),
        }
        match &recs[3] {
            EventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => assert_eq!((*caller_fid, *caller_line), (1, 12)),
            other => panic!("{other:?}"),
        }
        match &recs[4] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"after-sites"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn site_delta_absolute_path_still_works() {
        // Prior absolute preflight must remain green via encode_event_body.
        let specs = [EventRecordSpec::TimeLine {
            fid: 9,
            line: 99,
            ticks: 1,
        }];
        let enc = encode_event_body(&specs);
        assert!(!enc.contains(&FLAG_SITE_DELTA));
        let (recs, _) = decode_event_body(&enc).unwrap();
        match &recs[0] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (9, 99, 1));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn site_delta_truncated_mid_time_line_err() {
        let full = encode_event_body_with_site_deltas(&[
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
        ])
        .unwrap();
        assert!(full.len() > 4);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid site-delta TIME_LINE, got {other:?}"),
        }
    }

    /// Assert recovered logical TIME_LINE sequence equals expected (fid, line, ticks).
    fn assert_time_line_seq(recs: &[EventRecord<'_>], expect: &[(u64, u64, u64)]) {
        assert_eq!(recs.len(), expect.len(), "TIME_LINE count mismatch");
        for (i, exp) in expect.iter().enumerate() {
            match &recs[i] {
                EventRecord::TimeLine { fid, line, ticks } => {
                    assert_eq!((*fid, *line, *ticks), *exp, "TIME_LINE[{i}] mismatch");
                }
                other => panic!("TIME_LINE[{i}] expected TimeLine, got {other:?}"),
            }
        }
    }

    #[test]
    fn time_line_run_expands_to_ordered_same_site_time_lines() {
        // N≥2 distinct tick values at same site — every ticks retained (not sum/count).
        let ticks = [10u64, 20, 7, 99];
        let specs = [EventRecordSpec::TimeLineRun {
            fid: 1,
            line: 5,
            ticks: &ticks,
        }];
        let enc = encode_event_body(&specs);
        // Wire uses TIME_LINE_RUN opcode, not plain TIME_LINE.
        assert_ne!(
            enc,
            encode_event_body(&[
                EventRecordSpec::TimeLine {
                    fid: 1,
                    line: 5,
                    ticks: 10
                },
                EventRecordSpec::TimeLine {
                    fid: 1,
                    line: 5,
                    ticks: 20
                },
                EventRecordSpec::TimeLine {
                    fid: 1,
                    line: 5,
                    ticks: 7
                },
                EventRecordSpec::TimeLine {
                    fid: 1,
                    line: 5,
                    ticks: 99
                },
            ])
        );
        let (recs, n) = decode_event_body(&enc).expect("decode TIME_LINE_RUN");
        assert_eq!(n, enc.len());
        assert_time_line_seq(&recs, &[(1, 5, 10), (1, 5, 20), (1, 5, 7), (1, 5, 99)]);
        // Dual decode stability.
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
        // Sum of ticks is not the recovered form — all four distinct values present.
        let sum: u64 = ticks.iter().sum();
        assert_ne!(recs.len(), 1);
        match &recs[0] {
            EventRecord::TimeLine { ticks: t, .. } => assert_ne!(*t, sum),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn time_line_run_multi_run_and_mixed_with_plain_time_line() {
        // Stream: plain TL | run@site A | plain TL@other | run@site B
        let run_a = [3u64, 5, 8];
        let run_b = [100u64, 200];
        let specs = [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 1,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 10,
                ticks: &run_a,
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 20,
                ticks: 42,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 11,
                ticks: &run_b,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"after-runs",
            },
        ];
        let enc = encode_event_body(&specs);
        let (recs, n) = decode_event_body(&enc).expect("mixed stream");
        assert_eq!(n, enc.len());
        // Expanded: 1 + 3 + 1 + 2 + 1 Mark = 8 logical records
        assert_eq!(recs.len(), 8);
        assert_time_line_seq(
            &recs[..7],
            &[
                (1, 1, 1),
                (2, 10, 3),
                (2, 10, 5),
                (2, 10, 8),
                (3, 20, 42),
                (2, 11, 100),
                (2, 11, 200),
            ],
        );
        match &recs[7] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"after-runs"),
            other => panic!("expected trailing Mark, got {other:?}"),
        }
        // Equivalence: expanded absolute TIME_LINE-only encode decodes to same sequence.
        let expanded_only = [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 1,
                ticks: 1,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 10,
                ticks: 3,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 10,
                ticks: 8,
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 20,
                ticks: 42,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 11,
                ticks: 100,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 11,
                ticks: 200,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"after-runs",
            },
        ];
        let abs = encode_event_body(&expanded_only);
        let (abs_recs, _) = decode_event_body(&abs).unwrap();
        assert_eq!(abs_recs, recs);
    }

    #[test]
    fn time_line_run_truncated_mid_run_err() {
        let ticks = [10u64, 20, 30, 40];
        let full = encode_event_body(&[EventRecordSpec::TimeLineRun {
            fid: 1,
            line: 5,
            ticks: &ticks,
        }]);
        assert!(full.len() > 4);
        // Drop last tick byte(s).
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid TIME_LINE_RUN, got {other:?}"),
        }
    }

    #[test]
    fn time_line_run_empty_count_err() {
        // Manually craft: opcode TIME_LINE_RUN, flags 0, fid=1, line=1, N=0
        let mut bad = encode_u64(opcode::TIME_LINE_RUN);
        bad.push(0);
        bad.extend_from_slice(&encode_u64(1));
        bad.extend_from_slice(&encode_u64(1));
        bad.extend_from_slice(&encode_u64(0)); // empty run
        match decode_event_body(&bad) {
            Err(EventBodyError::EmptyTimeLineRun) => {}
            other => panic!("expected EmptyTimeLineRun, got {other:?}"),
        }
    }

    #[test]
    fn time_line_run_oversize_count_err() {
        // Craft N = MAX+1 without emitting tick payloads (fail closed before expand).
        let n = (MAX_TIME_LINE_RUN_LEN as u64) + 1;
        let mut bad = encode_u64(opcode::TIME_LINE_RUN);
        bad.push(0);
        bad.extend_from_slice(&encode_u64(1));
        bad.extend_from_slice(&encode_u64(1));
        bad.extend_from_slice(&encode_u64(n));
        match decode_event_body(&bad) {
            Err(EventBodyError::OversizeTimeLineRun { len }) => {
                assert_eq!(len, MAX_TIME_LINE_RUN_LEN + 1);
            }
            other => panic!("expected OversizeTimeLineRun, got {other:?}"),
        }
    }

    #[test]
    fn time_line_run_as_codec_none_chunk_payload() {
        let ticks = [11u64, 22, 33];
        let body = encode_event_body(&[EventRecordSpec::TimeLineRun {
            fid: 4,
            line: 8,
            ticks: &ticks,
        }]);
        let frame = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            3,
            body.len() as u32,
            &body,
            0,
        );
        let parsed = parse_chunk_frame(&frame).expect("chunk");
        assert_eq!(parsed.codec, codec::NONE);
        assert_eq!(parsed.payload, body.as_slice());
        let (recs, n) = decode_event_body(parsed.payload).expect("body from chunk");
        assert_eq!(n, body.len());
        assert_time_line_seq(&recs, &[(4, 8, 11), (4, 8, 22), (4, 8, 33)]);
    }

    /// Assert recovered logical TIME_BLOCK sequence equals expected
    /// (fid, line, block_line, ticks).
    fn assert_time_block_seq(recs: &[EventRecord<'_>], expect: &[(u64, u64, u64, u64)]) {
        assert_eq!(recs.len(), expect.len(), "TIME_BLOCK count mismatch");
        for (i, exp) in expect.iter().enumerate() {
            match &recs[i] {
                EventRecord::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                } => {
                    assert_eq!(
                        (*fid, *line, *block_line, *ticks),
                        *exp,
                        "TIME_BLOCK[{i}] mismatch"
                    );
                }
                other => panic!("TIME_BLOCK[{i}] expected TimeBlock, got {other:?}"),
            }
        }
    }

    #[test]
    fn time_block_run_expands_to_ordered_same_site_time_blocks() {
        let ticks = [10u64, 20, 7, 99];
        let specs = [EventRecordSpec::TimeBlockRun {
            fid: 1,
            line: 5,
            block_line: 4,
            ticks: &ticks,
        }];
        let enc = encode_event_body(&specs);
        assert_ne!(
            enc,
            encode_event_body(&[
                EventRecordSpec::TimeBlock {
                    fid: 1,
                    line: 5,
                    block_line: 4,
                    ticks: 10
                },
                EventRecordSpec::TimeBlock {
                    fid: 1,
                    line: 5,
                    block_line: 4,
                    ticks: 20
                },
                EventRecordSpec::TimeBlock {
                    fid: 1,
                    line: 5,
                    block_line: 4,
                    ticks: 7
                },
                EventRecordSpec::TimeBlock {
                    fid: 1,
                    line: 5,
                    block_line: 4,
                    ticks: 99
                },
            ])
        );
        let (recs, n) = decode_event_body(&enc).expect("decode TIME_BLOCK_RUN");
        assert_eq!(n, enc.len());
        assert_time_block_seq(
            &recs,
            &[(1, 5, 4, 10), (1, 5, 4, 20), (1, 5, 4, 7), (1, 5, 4, 99)],
        );
        let (recs2, n2) = decode_event_body(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
        let sum: u64 = ticks.iter().sum();
        assert_ne!(recs.len(), 1);
        match &recs[0] {
            EventRecord::TimeBlock { ticks: t, .. } => assert_ne!(*t, sum),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn time_block_run_multi_run_mixed_with_plain_and_time_line_run() {
        let block_run_a = [3u64, 5, 8];
        let block_run_b = [100u64, 200];
        let line_run = [11u64, 22];
        let specs = [
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 1,
                block_line: 1,
                ticks: 1,
            },
            EventRecordSpec::TimeBlockRun {
                fid: 2,
                line: 10,
                block_line: 4,
                ticks: &block_run_a,
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 20,
                ticks: 42,
            },
            EventRecordSpec::TimeLineRun {
                fid: 3,
                line: 21,
                ticks: &line_run,
            },
            EventRecordSpec::TimeBlockRun {
                fid: 2,
                line: 11,
                block_line: 5,
                ticks: &block_run_b,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"after-block-runs",
            },
        ];
        let enc = encode_event_body(&specs);
        let (recs, n) = decode_event_body(&enc).expect("mixed stream");
        assert_eq!(n, enc.len());
        // 1 plain TB + 3 run A + 1 TL + 2 TL run + 2 run B + 1 Mark = 10
        assert_eq!(recs.len(), 10);
        match &recs[0] {
            EventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (1, 1, 1, 1)),
            other => panic!("[0] {other:?}"),
        }
        assert_time_block_seq(&recs[1..4], &[(2, 10, 4, 3), (2, 10, 4, 5), (2, 10, 4, 8)]);
        match &recs[4] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (3, 20, 42));
            }
            other => panic!("[4] {other:?}"),
        }
        assert_time_line_seq(&recs[5..7], &[(3, 21, 11), (3, 21, 22)]);
        assert_time_block_seq(&recs[7..9], &[(2, 11, 5, 100), (2, 11, 5, 200)]);
        match &recs[9] {
            EventRecord::Mark { label } => assert_eq!(label.data, b"after-block-runs"),
            other => panic!("expected trailing Mark, got {other:?}"),
        }

        // Expanded absolute-only encode yields same logical sequence.
        let expanded_only = [
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 1,
                block_line: 1,
                ticks: 1,
            },
            EventRecordSpec::TimeBlock {
                fid: 2,
                line: 10,
                block_line: 4,
                ticks: 3,
            },
            EventRecordSpec::TimeBlock {
                fid: 2,
                line: 10,
                block_line: 4,
                ticks: 5,
            },
            EventRecordSpec::TimeBlock {
                fid: 2,
                line: 10,
                block_line: 4,
                ticks: 8,
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 20,
                ticks: 42,
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 21,
                ticks: 11,
            },
            EventRecordSpec::TimeLine {
                fid: 3,
                line: 21,
                ticks: 22,
            },
            EventRecordSpec::TimeBlock {
                fid: 2,
                line: 11,
                block_line: 5,
                ticks: 100,
            },
            EventRecordSpec::TimeBlock {
                fid: 2,
                line: 11,
                block_line: 5,
                ticks: 200,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"after-block-runs",
            },
        ];
        let abs = encode_event_body(&expanded_only);
        let (abs_recs, _) = decode_event_body(&abs).unwrap();
        assert_eq!(abs_recs, recs);
    }

    #[test]
    fn time_block_run_truncated_mid_run_err() {
        let ticks = [10u64, 20, 30, 40];
        let full = encode_event_body(&[EventRecordSpec::TimeBlockRun {
            fid: 1,
            line: 5,
            block_line: 4,
            ticks: &ticks,
        }]);
        assert!(full.len() > 4);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid TIME_BLOCK_RUN, got {other:?}"),
        }
    }

    #[test]
    fn time_block_run_empty_count_err() {
        let mut bad = encode_u64(opcode::TIME_BLOCK_RUN);
        bad.push(0);
        bad.extend_from_slice(&encode_u64(1)); // fid
        bad.extend_from_slice(&encode_u64(1)); // line
        bad.extend_from_slice(&encode_u64(1)); // block_line
        bad.extend_from_slice(&encode_u64(0)); // N=0
        match decode_event_body(&bad) {
            Err(EventBodyError::EmptyTimeBlockRun) => {}
            other => panic!("expected EmptyTimeBlockRun, got {other:?}"),
        }
    }

    #[test]
    fn time_block_run_oversize_count_err() {
        let n = (MAX_TIME_BLOCK_RUN_LEN as u64) + 1;
        let mut bad = encode_u64(opcode::TIME_BLOCK_RUN);
        bad.push(0);
        bad.extend_from_slice(&encode_u64(1));
        bad.extend_from_slice(&encode_u64(1));
        bad.extend_from_slice(&encode_u64(1));
        bad.extend_from_slice(&encode_u64(n));
        match decode_event_body(&bad) {
            Err(EventBodyError::OversizeTimeBlockRun { len }) => {
                assert_eq!(len, MAX_TIME_BLOCK_RUN_LEN + 1);
            }
            other => panic!("expected OversizeTimeBlockRun, got {other:?}"),
        }
    }

    #[test]
    fn time_block_run_as_codec_none_chunk_payload() {
        let ticks = [11u64, 22, 33];
        let body = encode_event_body(&[EventRecordSpec::TimeBlockRun {
            fid: 4,
            line: 8,
            block_line: 3,
            ticks: &ticks,
        }]);
        let frame = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            3,
            body.len() as u32,
            &body,
            0,
        );
        let parsed = parse_chunk_frame(&frame).expect("chunk");
        assert_eq!(parsed.codec, codec::NONE);
        assert_eq!(parsed.payload, body.as_slice());
        let (recs, n) = decode_event_body(parsed.payload).expect("body from chunk");
        assert_eq!(n, body.len());
        assert_time_block_seq(&recs, &[(4, 8, 3, 11), (4, 8, 3, 22), (4, 8, 3, 33)]);
    }

    #[test]
    fn event_seq_dual_output_roundtrip_order_and_per_event_seq() {
        let specs = dual_output_sequence_specs();
        let enc = encode_event_body_with_seq(&specs);
        // Must differ from absolute no-seq encode.
        assert_ne!(enc, encode_event_body(&specs));
        assert!(enc.contains(&FLAG_HAS_SEQ));

        let (decoded, n) = decode_event_body_full(&enc).expect("full decode with seq");
        assert_eq!(n, enc.len());
        assert_eq!(decoded.records.len(), 9);
        assert_eq!(decoded.sequences.len(), 9);
        assert_dual_output_sequence_order(&decoded.records);
        // Monotonic 0..8 on each logical event (incl. VERSION and START_DEFLATE).
        for (i, s) in decoded.sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
        // Dual decode stability.
        let (d2, n2) = decode_event_body_full(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(d2.records, decoded.records);
        assert_eq!(d2.sequences, decoded.sequences);
        // decode_event_body drops sequences but keeps order/fields.
        let (recs_only, _) = decode_event_body(&enc).unwrap();
        assert_eq!(recs_only, decoded.records);
    }

    #[test]
    fn event_seq_with_time_line_run_expands_base_plus_offsets() {
        let ticks = [10u64, 20, 30];
        let specs = [
            EventRecordSpec::Version { major: 5, minor: 0 },
            EventRecordSpec::TimeLineRun {
                fid: 1,
                line: 5,
                ticks: &ticks,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"after",
            },
        ];
        let enc = encode_event_body_with_seq(&specs);
        let (decoded, n) = decode_event_body_full(&enc).unwrap();
        assert_eq!(n, enc.len());
        // VERSION + 3 expanded TL + Mark = 5
        assert_eq!(decoded.records.len(), 5);
        assert_eq!(
            decoded.sequences,
            vec![Some(0), Some(1), Some(2), Some(3), Some(4)]
        );
        match &decoded.records[1] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 5, 10));
            }
            other => panic!("{other:?}"),
        }
        match &decoded.records[3] {
            EventRecord::TimeLine { ticks, .. } => assert_eq!(*ticks, 30),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn event_seq_without_flag_is_none() {
        let specs = [EventRecordSpec::TimeLine {
            fid: 1,
            line: 2,
            ticks: 3,
        }];
        let enc = encode_event_body(&specs);
        assert!(!enc.contains(&FLAG_HAS_SEQ));
        let (decoded, _) = decode_event_body_full(&enc).unwrap();
        assert_eq!(decoded.sequences, vec![None]);
    }

    #[test]
    fn event_seq_truncated_mid_seq_field_err() {
        let full = encode_event_body_with_seq(&[EventRecordSpec::TimeLine {
            fid: 1,
            line: 2,
            ticks: 3,
        }]);
        // Craft: opcode TIME_LINE + FLAG_HAS_SEQ + truncated seq varint (continuation without end).
        let mut bad = encode_u64(opcode::TIME_LINE);
        bad.push(FLAG_HAS_SEQ);
        bad.push(0x80); // incomplete multi-byte ULEB
        match decode_event_body_full(&bad) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid-seq, got {other:?}"),
        }
        // Truncate a real encoded body mid-record (includes seq field).
        assert!(full.len() > 3);
        let trunc = &full[..full.len() - 1];
        match decode_event_body_full(trunc) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid-seq body, got {other:?}"),
        }
    }

    #[test]
    fn event_seq_as_codec_none_chunk_payload() {
        let specs = dual_output_sequence_specs();
        let body = encode_event_body_with_seq(&specs);
        let frame = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            9,
            body.len() as u32,
            &body,
            0,
        );
        let parsed = parse_chunk_frame(&frame).expect("chunk");
        assert_eq!(parsed.codec, codec::NONE);
        let (decoded, n) = decode_event_body_full(parsed.payload).unwrap();
        assert_eq!(n, body.len());
        assert_dual_output_sequence_order(&decoded.records);
        for (i, s) in decoded.sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64));
        }
    }

    /// Multi-record stream mixing dual-output meta, site-delta sites, and a run.
    fn site_delta_and_seq_compose_specs() -> Vec<EventRecordSpec<'static>> {
        vec![
            EventRecordSpec::Version { major: 5, minor: 0 },
            EventRecordSpec::Comment {
                string_id: 0,
                string_flags: 0,
                text: b"# compose packing",
            },
            EventRecordSpec::PidStart {
                pid: 1001,
                ppid: 1,
                start_time: 1_700_000_000,
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
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 50,
                ticks: &[7, 8],
            },
            EventRecordSpec::PidEnd {
                pid: 1001,
                end_time: 1_700_000_042,
            },
        ]
    }

    fn assert_site_delta_and_seq_compose(recs: &[EventRecord<'_>], sequences: &[Option<u64>]) {
        // VERSION, Comment, PidStart, TL, TL, TB, SubEntry, 2 expanded TL, PidEnd = 10
        assert_eq!(recs.len(), 10);
        assert_eq!(sequences.len(), 10);
        for (i, s) in sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
        match &recs[0] {
            EventRecord::Version { major, minor } => assert_eq!((*major, *minor), (5, 0)),
            other => panic!("[0] {other:?}"),
        }
        match &recs[3] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("[3] {other:?}"),
        }
        match &recs[4] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 11, 6));
            }
            other => panic!("[4] {other:?}"),
        }
        match &recs[5] {
            EventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (1, 12, 4, 20)),
            other => panic!("[5] {other:?}"),
        }
        match &recs[6] {
            EventRecord::SubEntry {
                caller_fid,
                caller_line,
            } => assert_eq!((*caller_fid, *caller_line), (1, 10)),
            other => panic!("[6] {other:?}"),
        }
        match &recs[7] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 50, 7));
            }
            other => panic!("[7] {other:?}"),
        }
        match &recs[8] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 50, 8));
            }
            other => panic!("[8] {other:?}"),
        }
        match &recs[9] {
            EventRecord::PidEnd { pid, end_time } => {
                assert_eq!((*pid, *end_time), (1001, 1_700_000_042));
            }
            other => panic!("[9] {other:?}"),
        }
    }

    #[test]
    fn site_delta_and_seq_compose_roundtrip_absolute_and_per_event_seq() {
        let specs = site_delta_and_seq_compose_specs();
        let enc = encode_event_body_with_site_deltas_and_seq(&specs).expect("compose encode");
        // Must differ from pure absolute, pure delta, and pure seq paths.
        assert_ne!(enc, encode_event_body(&specs));
        let delta_only = encode_event_body_with_site_deltas(&specs).unwrap();
        assert_ne!(enc, delta_only);
        let seq_only = encode_event_body_with_seq(&specs);
        assert_ne!(enc, seq_only);
        assert!(enc.contains(&(FLAG_SITE_DELTA | FLAG_HAS_SEQ)));
        assert!(enc.contains(&FLAG_HAS_SEQ));

        let (decoded, n) = decode_event_body_full(&enc).expect("compose decode");
        assert_eq!(n, enc.len());
        assert_site_delta_and_seq_compose(&decoded.records, &decoded.sequences);
        // Dual decode stability.
        let (d2, n2) = decode_event_body_full(&enc).unwrap();
        assert_eq!(n2, n);
        assert_eq!(d2.records, decoded.records);
        assert_eq!(d2.sequences, decoded.sequences);
        // decode_event_body drops sequences but keeps absolute order/fields.
        let (recs_only, _) = decode_event_body(&enc).unwrap();
        assert_eq!(recs_only, decoded.records);
    }

    #[test]
    fn site_delta_and_seq_compose_truncated_mid_field_err() {
        let full = encode_event_body_with_site_deltas_and_seq(&[
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
        ])
        .unwrap();
        assert!(full.len() > 4);
        let trunc = &full[..full.len() - 1];
        match decode_event_body_full(trunc) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid compose field, got {other:?}"),
        }
        // Incomplete multi-byte seq after combined flags.
        let mut bad = encode_u64(opcode::TIME_LINE);
        bad.push(FLAG_SITE_DELTA | FLAG_HAS_SEQ);
        bad.push(0x80); // truncated seq ULEB
        match decode_event_body_full(&bad) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid-seq compose, got {other:?}"),
        }
    }

    #[test]
    fn site_delta_and_seq_compose_as_codec_none_chunk_payload() {
        let specs = site_delta_and_seq_compose_specs();
        let body = encode_event_body_with_site_deltas_and_seq(&specs).unwrap();
        let frame = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            10,
            body.len() as u32,
            &body,
            0,
        );
        let parsed = parse_chunk_frame(&frame).expect("chunk");
        assert_eq!(parsed.codec, codec::NONE);
        let (decoded, n) = decode_event_body_full(parsed.payload).unwrap();
        assert_eq!(n, body.len());
        assert_site_delta_and_seq_compose(&decoded.records, &decoded.sequences);
    }

    /// Regression: after a TIME_LINE_RUN, encode SiteCursor must match decode so a
    /// following site-delta TIME_LINE at (2,51) reconstructs as (2,51) not (3,91).
    #[test]
    fn site_delta_after_time_line_run_reconstructs_absolute() {
        let ticks = [7u64, 8];
        let specs = [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 50,
                ticks: &ticks,
            },
            EventRecordSpec::TimeLine {
                fid: 2,
                line: 51,
                ticks: 9,
            },
        ];
        // Pure site-delta path (run is absolute wire, cursor must advance).
        let enc_delta = encode_event_body_with_site_deltas(&specs).expect("site-delta encode");
        let (recs_delta, n) = decode_event_body(&enc_delta).expect("site-delta decode");
        assert_eq!(n, enc_delta.len());
        assert_eq!(recs_delta.len(), 4); // 1 + 2 expanded + 1
        match &recs_delta[0] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("[0] {other:?}"),
        }
        match &recs_delta[1] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 50, 7));
            }
            other => panic!("[1] {other:?}"),
        }
        match &recs_delta[2] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 50, 8));
            }
            other => panic!("[2] {other:?}"),
        }
        match &recs_delta[3] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!(
                    (*fid, *line, *ticks),
                    (2, 51, 9),
                    "post-run site-delta must not double-apply pre-run base"
                );
            }
            other => panic!("[3] {other:?}"),
        }

        // Compose path: same absolute reconstruction + seq values.
        let enc_compose =
            encode_event_body_with_site_deltas_and_seq(&specs).expect("compose encode");
        let (decoded, n2) = decode_event_body_full(&enc_compose).expect("compose decode");
        assert_eq!(n2, enc_compose.len());
        assert_eq!(decoded.records.len(), 4);
        assert_eq!(decoded.sequences, vec![Some(0), Some(1), Some(2), Some(3)]);
        match &decoded.records[3] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 51, 9));
            }
            other => panic!("compose [3] {other:?}"),
        }
    }

    /// Regression: TIME_BLOCK_RUN advances block_line cursor for following site-delta.
    #[test]
    fn site_delta_after_time_block_run_reconstructs_absolute() {
        let ticks = [10u64, 20];
        let specs = [
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 5,
                block_line: 4,
                ticks: 1,
            },
            EventRecordSpec::TimeBlockRun {
                fid: 2,
                line: 8,
                block_line: 6,
                ticks: &ticks,
            },
            EventRecordSpec::TimeBlock {
                fid: 2,
                line: 9,
                block_line: 7,
                ticks: 3,
            },
        ];
        let enc = encode_event_body_with_site_deltas_and_seq(&specs).unwrap();
        let (decoded, _) = decode_event_body_full(&enc).unwrap();
        assert_eq!(decoded.records.len(), 4);
        match &decoded.records[3] {
            EventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (2, 9, 7, 3)),
            other => panic!("{other:?}"),
        }
        let enc_delta = encode_event_body_with_site_deltas(&specs).unwrap();
        let (recs, _) = decode_event_body(&enc_delta).unwrap();
        match &recs[3] {
            EventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (2, 9, 7, 3)),
            other => panic!("{other:?}"),
        }
    }

    /// Multi-chunk record-aligned packing: continued site/seq bases across partitions.
    #[test]
    fn multi_chunk_packing_plains_join_equals_single_chunk_compose() {
        let specs = [
            EventRecordSpec::Version { major: 5, minor: 0 },
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
        ];
        assert!(specs.len() >= 4);
        let single = encode_event_body_with_site_deltas_and_seq(&specs).expect("single");
        // max 2 records per chunk → ≥2 partitions
        let parts = crate::multi_chunk_event::partition_event_records(&specs, 2);
        assert!(
            parts.len() >= 2,
            "expected multi-chunk partition, got {}",
            parts.len()
        );
        let mut state = PackingEncodeState::new();
        let mut joined = Vec::new();
        for part in &parts {
            let plain =
                encode_event_body_with_site_deltas_and_seq_continuing(part, &mut state).unwrap();
            joined.extend_from_slice(&plain);
        }
        assert_eq!(
            joined, single,
            "multi-chunk continued packing plains must equal single-chunk compose wire"
        );
        // Naive per-chunk reset must differ (proves continuity is required).
        let mut naive = Vec::new();
        for part in &parts {
            naive.extend_from_slice(&encode_event_body_with_site_deltas_and_seq(part).unwrap());
        }
        assert_ne!(
            naive, single,
            "per-chunk packing reset must not match continuous packing"
        );

        let (decoded, n) = decode_event_body_full(&joined).unwrap();
        assert_eq!(n, joined.len());
        assert_eq!(decoded.records.len(), 6);
        for (i, s) in decoded.sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
        match &decoded.records[1] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 10, 5));
            }
            other => panic!("{other:?}"),
        }
        match &decoded.records[5] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (2, 1, 7));
            }
            other => panic!("{other:?}"),
        }
    }

    /// Multi-chunk packing with TIME_LINE_RUN / TIME_BLOCK_RUN: continued bases
    /// across partitions, including site-delta **after** a run in a later chunk.
    #[test]
    fn multi_chunk_packing_with_time_runs_plains_join_equals_single_and_post_run_site() {
        let tl_ticks = [7u64, 8];
        let tb_ticks = [10u64, 20];
        let specs = [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLineRun {
                fid: 2,
                line: 50,
                ticks: &tl_ticks,
            },
            // Chunk boundary (max=2) lands *after* the run; site-delta must use run cursor.
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
                ticks: &tb_ticks,
            },
            EventRecordSpec::TimeBlock {
                fid: 3,
                line: 9,
                block_line: 7,
                ticks: 3,
            },
        ];
        let single = encode_event_body_with_site_deltas_and_seq(&specs).expect("single");
        let parts = crate::multi_chunk_event::partition_event_records(&specs, 2);
        assert!(
            parts.len() >= 3,
            "expected ≥3 partitions, got {}",
            parts.len()
        );
        // Run in part0; post-run site-delta must be in a later partition.
        assert!(
            matches!(parts[0].last(), Some(EventRecordSpec::TimeLineRun { .. })),
            "part0 should end with TIME_LINE_RUN"
        );
        assert!(
            matches!(parts[1].first(), Some(EventRecordSpec::TimeLine { .. })),
            "part1 should start with post-run TIME_LINE"
        );

        let mut state = PackingEncodeState::new();
        let mut joined = Vec::new();
        for part in &parts {
            let plain =
                encode_event_body_with_site_deltas_and_seq_continuing(part, &mut state).unwrap();
            joined.extend_from_slice(&plain);
        }
        assert_eq!(
            joined, single,
            "multi-chunk+run continued packing must equal single-chunk compose wire"
        );

        let (decoded, n) = decode_event_body_full(&joined).unwrap();
        assert_eq!(n, joined.len());
        // 1 + 2 expanded + 1 + 1 + 2 expanded + 1 = 8 logical events
        assert_eq!(decoded.records.len(), 8);
        for (i, s) in decoded.sequences.iter().enumerate() {
            assert_eq!(*s, Some(i as u64), "seq[{i}]");
        }
        // Index 3 = first site-delta after TIME_LINE_RUN expand (indices 1,2).
        match &decoded.records[3] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!(
                    (*fid, *line, *ticks),
                    (2, 51, 9),
                    "post-run site-delta across chunk boundary"
                );
            }
            other => panic!("[3] {other:?}"),
        }
        // Last = site-delta after TIME_BLOCK_RUN (indices 5,6 expanded → index 7).
        match &decoded.records[7] {
            EventRecord::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => assert_eq!((*fid, *line, *block_line, *ticks), (3, 9, 7, 3)),
            other => panic!("[7] {other:?}"),
        }
    }
}

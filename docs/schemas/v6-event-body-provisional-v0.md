# Format v6 event-body opcode codec (provisional) — v0

**Status:** opcode/flag numeric IDs **frozen** for major=6 by [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md); detailed body layouts retained; not CLI v6 default  
**Board IDs:** `FMT-V6-EVENT-BODY-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-MVP` (shipped encode/decode + tests)  
**Depends on:** ULEB128 [`v6-varint-uleb128-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-varint-uleb128-provisional-v0.md); string/blob [`v6-string-blob-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-blob-provisional-v0.md); chunk frame (codec NONE payload role) [`v6-chunk-frame-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-chunk-frame-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** full event catalog / payload inflate / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** **event-body** byte layout used as a **codec NONE** chunk payload:

```text
event-body = record*
record     = ULEB128 opcode || u8 flags || typed-body
```

It is **not**:

- a permanent wire freeze or full logical-event catalog matching all v5 tags;
- permission to mark **COL-007** (C v6 writer) or **COL-008** done;
- payload inflate (zlib / zstd / LZ4);
- permanent global string pools / permanent location-delta or run packing freeze, or CRC verification freeze (local dictionary + site-delta + TIME_LINE_RUN + TIME_BLOCK_RUN preflights are siblings);
- default CLI report/dump of v6 profiles.

Opcodes and field layouts may change under future ADR + golden vectors.

---

## Record layout

| Field | Encoding | Notes |
|-------|----------|-------|
| opcode | strict ULEB128 | Provisional table below |
| flags | `u8` | `FLAG_OPCODE_REQUIRED = 0x01`; `FLAG_BODY_LENGTH = 0x02`; `FLAG_SITE_DELTA = 0x04`; `FLAG_HAS_SEQ = 0x08` (provisional seq-number preflight) |
| seq (optional) | ULEB128 | Present when `FLAG_HAS_SEQ`; see [`v6-event-body-seq-number-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-seq-number-provisional-v0.md) |
| typed-body | per opcode | Composed from existing primitives |

### Provisional opcodes

| Value | Name | Typed body |
|------:|------|------------|
| 0 | `RESERVED` | Invalid — **always Err** |
| 1 | `MARK` | length-prefixed string/blob (`encode_string_blob` / `decode_string_blob`) |
| 2 | `TIME_LINE` | three ULEB128 `u64`: `fid`, `line`, `ticks` |
| 3 | `TIME_BLOCK` | four ULEB128 `u64`: `fid`, `line`, `block_line`, `ticks` |
| 4 | `SUB_ENTRY` | two ULEB128 `u64`: `caller_fid`, `caller_line` |
| 5 | `SUB_RETURN` | three ULEB128 `u64` (`depth`, `incl`, `excl`) + string-blob `subname` |
| 6 | `SUB_INFO` | three ULEB128 `u64` (`fid`, `first_line`, `last_line`) + string-blob `name` |
| 7 | `SRC_LINE` | two ULEB128 `u64` (`fid`, `line`) + string-blob `text` |
| 8 | `NEW_FID` | one ULEB128 `u64` (`fid`) + string-blob `filename` |
| 9 | `PID_START` | three ULEB128 `u64`: `pid`, `ppid`, `start_time` |
| 10 | `PID_END` | two ULEB128 `u64`: `pid`, `end_time` |
| 11 | `SUB_CALLERS` | seven ULEB128 (`fid`, `line`, `count`, `incl`, `excl`, `reci`, `rec_depth`) + string-blob `called` + string-blob `caller` |
| 12 | `DISCOUNT` | empty typed body (opcode + flags only) |
| 13 | `ATTRIBUTE` | string-blob `key` + string-blob `value` |
| 14 | `OPTION` | string-blob `key` + string-blob `value` |
| 15 | `COMMENT` | string-blob `text` |
| 16 | `START_DEFLATE` | empty typed body (opcode + flags only; marker presence) |
| 17 | `VERSION` | two ULEB128 `u64`: `major`, `minor` |
| 18 | `TIME_LINE_RUN` | `fid`, `line`, `N`, then `N` × `ticks` (all ULEB128); expands to N logical `TIME_LINE` (see [`v6-event-body-time-line-run-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-time-line-run-provisional-v0.md)) |
| 19 | `TIME_BLOCK_RUN` | `fid`, `line`, `block_line`, `N`, then `N` × `ticks` (all ULEB128); expands to N logical `TIME_BLOCK` (see [`v6-event-body-time-block-run-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-time-block-run-provisional-v0.md)) |
| other | unknown | Required flag → `UnknownRequiredOpcode`; optional + `FLAG_BODY_LENGTH` → **skip** length-framed body (preflight); else `UnknownOpcode` |

Expanded opcodes (TIME_BLOCK through TIME_BLOCK_RUN) are **provisional preflight** layouts (v5-ish shapes; integer ULEB ticks/times; string projection for ATTRIBUTE/OPTION/COMMENT; START_DEFLATE is a marker only; VERSION is dump-aligned major/minor; TIME_LINE_RUN / TIME_BLOCK_RUN are packed same-site expansions) — **not** a full logical-event catalog freeze, dual-equality, or wire freeze.

### Fail-closed rules

| Condition | Result |
|-----------|--------|
| Empty body | **Ok** — zero records |
| Truncated mid-record (missing flags or mid-field) | **Err** |
| Opcode 0 | **Err** (`ReservedOpcode`) |
| Unknown opcode + `FLAG_OPCODE_REQUIRED` | **Err** (`UnknownRequiredOpcode`) |
| Unknown opcode without required flag, with `FLAG_BODY_LENGTH` | **Skip** length-framed body (not emitted; preflight) |
| Unknown opcode without required flag, without length frame | **Err** (`UnknownOpcode`) |
| Oversize body (> 64 MiB) | **Err** |
| Never panic on crafted bodies | Required |

---

## Role as codec NONE chunk payload

An EVENT chunk with `codec = NONE` may carry an event-body encoding as its payload bytes. Composition is optional smoke only in this MVP — no inflate, no default CLI path.

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::event_body::{
    encode_event_body, decode_event_body, EventRecordSpec, EventRecord,
};

let bytes = encode_event_body(&[
    EventRecordSpec::TimeLine { fid: 1, line: 5, ticks: 42 },
    EventRecordSpec::TimeBlock { fid: 1, line: 5, block_line: 4, ticks: 780 },
    EventRecordSpec::SubEntry { caller_fid: 1, caller_line: 12 },
    EventRecordSpec::SubReturn {
        depth: 1, incl: 900, excl: 50,
        string_id: 0, string_flags: 0, subname: b"main::leaf",
    },
    EventRecordSpec::SubInfo {
        fid: 1, first_line: 3, last_line: 7,
        string_id: 1, string_flags: 0, name: b"main::leaf",
    },
    EventRecordSpec::NewFid {
        fid: 1, string_id: 0, string_flags: 0, filename: b"workload.pl",
    },
    EventRecordSpec::SrcLine {
        fid: 1, line: 5, string_id: 1, string_flags: 0, text: b"  my $x = 1;",
    },
    EventRecordSpec::PidStart { pid: 1001, ppid: 1, start_time: 1_700_000_000 },
    EventRecordSpec::PidEnd { pid: 1001, end_time: 1_700_000_042 },
    EventRecordSpec::SubCallers {
        fid: 1, line: 10, count: 15, incl: 900, excl: 50, reci: 0, rec_depth: 0,
        called_string_id: 0, called_string_flags: 0, called: b"main::leaf",
        caller_string_id: 1, caller_string_flags: 0, caller: b"main::mid",
    },
    EventRecordSpec::Discount,
    EventRecordSpec::Attribute {
        key_string_id: 0, key_string_flags: 0, key: b"basetime",
        value_string_id: 1, value_string_flags: 0, value: b"1700000000",
    },
    EventRecordSpec::Option {
        key_string_id: 2, key_string_flags: 0, key: b"calls",
        value_string_id: 3, value_string_flags: 0, value: b"1",
    },
    EventRecordSpec::Comment {
        string_id: 0, string_flags: 0, text: b"# profiler note",
    },
    EventRecordSpec::StartDeflate,
    EventRecordSpec::Version { major: 5, minor: 0 },
    EventRecordSpec::Mark { string_id: 0, string_flags: 0, label: b"leaf" },
]);
let (recs, n) = decode_event_body(&bytes)?;
// n == bytes.len(); recs fields equal to specs
```

- Composes **shipped** `encode_u64` / `decode_u64` and `encode_string_blob` / `decode_string_blob`.
- Pure byte-slice / `Vec` APIs (no I/O). Not wired into default CLI.
- Always-inflate consumers recover these opcodes via `decode_event_body` after join (`FMT-V6-DECODED-EVENT-*` / `FMT-V6-DECODED-MIXED-*`).

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Varint / string / chunk / prefix+stream preflight | done |
| Event-body opcode codec (MARK…VERSION) | **done** (`FMT-V6-EVENT-BODY-*` + TIME_BLOCK/SUB_ENTRY + SUB_RETURN/SUB_INFO + SRC_LINE/NEW_FID + PID_START/PID_END + SUB_CALLERS/DISCOUNT + ATTRIBUTE/OPTION + COMMENT + START_DEFLATE + VERSION preflights) |
| Dual-output dump-aligned multi-record sequence preflight | **done** (`FMT-V6-EVENT-BODY-DUAL-OUTPUT-SEQUENCE-*`; not OI-001-03 seq-number freeze) |
| START_DEFLATE mid-stream chunk-codec switch preflight | **done** (`FMT-V6-START-DEFLATE-MID-STREAM-CODEC-SWITCH-*`; not v5 mid-payload stream deflate freeze) |
| Auto-emit VERSION from fixed-header preflight | **done** (`FMT-V6-AUTO-EMIT-VERSION-*`; not OI-001-03 freeze) |
| ATTRIBUTE/OPTION known-key vocabulary preflight | **done** (`FMT-V6-ATTR-OPTION-KNOWN-KEY-*`; not complete OI-002-03/04 freeze) |
| Unknown optional length-framed skip preflight | **done** (`FMT-V6-EVENT-BODY-UNKNOWN-OPTIONAL-SKIP-*`; not flag-bit freeze) |
| String-dictionary intern preflight | **done** (`FMT-V6-STRING-DICTIONARY-*`; not permanent global pool freeze) |
| Location / site-delta preflight (TIME_LINE/TIME_BLOCK/SUB_ENTRY) | **done** (`FMT-V6-EVENT-BODY-SITE-DELTA-*`; not permanent packing freeze) |
| Mini-profile composition using event-body | **done** separately (`FMT-V6-MINI-PROFILE-*`) |
| Always-inflate EVENT/mixed recovery of expanded opcodes | **done** (consumer paths) |
| Full v5-equivalent opcode catalog / deltas / dictionaries | residual |
| Default `parse_chunk_frame` inflate/CRC | residual (stays non-inflating) |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. ADR freeze of opcode space and permanent flag bits (length-framed optional skip is preflight only).
2. Remaining stream-control / catalog items: full dual-output **sequence-number** freeze (OI-001-03 / COL-003 — dump-aligned **order recovery**, chunk-framed mid-stream codec-switch, auto-emit VERSION, and unknown-optional skip preflights are done) + complete ATTRIBUTE/OPTION key vocabularies (OI-002-03/04 full inventory — known-key preflight is done) + **permanent** location-delta packing ADR (site-delta preflight is done) + **permanent** global string-pool freeze (local string-dictionary intern preflight is done); fork re-init / COL-015; exact DISCOUNT accounting vs BASE-003; COMPAT-002 comment volatile normalize.
3. Dual-equality vs C encoder + golden full-file corpus; float/NV exactness for times/ticks.
4. Default CLI v6 read path / wire freeze.

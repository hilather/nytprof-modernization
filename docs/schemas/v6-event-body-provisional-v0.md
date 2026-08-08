# Format v6 event-body opcode codec (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
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
- string dictionaries, location deltas, or CRC verification freeze;
- default CLI report/dump of v6 profiles.

Opcodes and field layouts may change under future ADR + golden vectors.

---

## Record layout

| Field | Encoding | Notes |
|-------|----------|-------|
| opcode | strict ULEB128 | Provisional table below |
| flags | `u8` | `FLAG_OPCODE_REQUIRED = 0x01` |
| typed-body | per opcode | Composed from existing primitives |

### Provisional opcodes

| Value | Name | Typed body |
|------:|------|------------|
| 0 | `RESERVED` | Invalid — **always Err** |
| 1 | `MARK` | length-prefixed string/blob (`encode_string_blob` / `decode_string_blob`) |
| 2 | `TIME_LINE` | three ULEB128 `u64`: `fid`, `line`, `ticks` |
| other | unknown | Fail closed (required flag → `UnknownRequiredOpcode`; else `UnknownOpcode` — MVP cannot skip unknown bodies) |

### Fail-closed rules

| Condition | Result |
|-----------|--------|
| Empty body | **Ok** — zero records |
| Truncated mid-record (missing flags or mid-field) | **Err** |
| Opcode 0 | **Err** (`ReservedOpcode`) |
| Unknown opcode + `FLAG_OPCODE_REQUIRED` | **Err** (`UnknownRequiredOpcode`) |
| Unknown opcode without required flag | **Err** (`UnknownOpcode`) in this MVP |
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
    EventRecordSpec::Mark { string_id: 0, string_flags: 0, label: b"leaf" },
]);
let (recs, n) = decode_event_body(&bytes)?;
// n == bytes.len(); recs fields equal to specs
```

- Composes **shipped** `encode_u64` / `decode_u64` and `encode_string_blob` / `decode_string_blob`.
- Pure byte-slice / `Vec` APIs (no I/O). Not wired into default CLI.

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Varint / string / chunk / prefix+stream preflight | done |
| Event-body opcode codec (codec NONE payload) | **done** (`FMT-V6-EVENT-BODY-*`) |
| Mini-profile composition using event-body | **done** separately (`FMT-V6-MINI-PROFILE-*`) |
| Full v5-equivalent opcode catalog / deltas / dictionaries | residual |
| Payload inflate (zlib/zstd/LZ4) | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. ADR freeze of opcode space and flag bits.
2. Full logical-event catalog + location deltas + dictionaries.
3. Optional skip of unknown non-required opcodes (needs length framing).
4. Payload inflate + dual-equality vs C encoder.
5. Golden full-file corpus and default CLI v6 read path.

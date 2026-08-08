# Format v6 event-body ATTRIBUTE + OPTION opcodes — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-ATTRIBUTE-OPTION-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-ATTRIBUTE-OPTION-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** full opcode catalog freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Extends the provisional event-body opcode set with:

| Opcode | Value | Typed body |
|--------|------:|------------|
| `ATTRIBUTE` | 13 | string-blob `key` + string-blob `value` |
| `OPTION` | 14 | string-blob `key` + string-blob `value` |

Same framing as other opcodes: `ULEB128 opcode || u8 flags || typed-body`.  
Keys and values are **string projections** only. A provisional **known-key** preflight is shipped separately (`FMT-V6-ATTR-OPTION-KNOWN-KEY-*` / [`v6-attr-option-known-key-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-attr-option-known-key-provisional-v0.md)) — **not** a complete ATTRIBUTE/OPTION key vocabulary freeze (OI-002-03 / OI-002-04).

It is **not**:

- a full v5 opcode catalog freeze (full dual-output sequence OI-001-03 if deferred, …; COMMENT/START_DEFLATE/VERSION are sibling preflights);
- complete key vocabulary freeze for ATTRIBUTE/OPTION (known-key preflight is a sibling);
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid ATTRIBUTE / OPTION | **Err** |
| Reserved opcode 0 / unknown opcodes | **Err** (unchanged MVP) |
| Never panic | Required |

---

## Always-inflate recovery

Shipped consumers recover mixed bodies that include these opcodes:

- `decode_decoded_event_profile` under NONE/ZLIB/ZSTD/LZ4
- `decode_decoded_mixed_profile` with co-present SOURCE under NONE/ZLIB/ZSTD/LZ4 EVENT

Evidence: `cargo test -p nytprof-format-v6`.

---

## Open residual

1. Remaining stream-control items (full dual-output sequence-number freeze OI-001-03 if deferred, …) via ADR + golden vectors (COMMENT/START_DEFLATE/VERSION + dual-output order recovery + auto-emit VERSION shipped as sibling preflights).
2. Complete ATTRIBUTE/OPTION key vocabularies (OI-002-03 / OI-002-04 full writer inventory — known-key preflight is shipped separately).
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality.

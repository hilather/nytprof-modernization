# Format v6 event-body SRC_LINE + NEW_FID opcodes — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-SRC-LINE-NEW-FID-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-SRC-LINE-NEW-FID-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** full opcode catalog freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Extends the provisional event-body opcode set with:

| Opcode | Value | Typed body |
|--------|------:|------------|
| `SRC_LINE` | 7 | ULEB `fid`, `line` + string-blob `text` |
| `NEW_FID` | 8 | ULEB `fid` + string-blob `filename` |

Same framing as other opcodes: `ULEB128 opcode || u8 flags || typed-body`.

It is **not**:

- a full v5 opcode catalog freeze (full dual-output sequence OI-001-03 if deferred, …; later sibling preflights include PID_START/PID_END through VERSION);
- full COMPAT-001 semantic freeze of field meanings;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid SRC_LINE / NEW_FID | **Err** |
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

1. Remaining stream-control items (full dual-output sequence-number freeze OI-001-03 if deferred, …) via ADR + golden vectors (PID_START/PID_END through VERSION + dual-output order recovery shipped as sibling preflights).
2. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality.

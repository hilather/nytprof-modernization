# Format v6 event-body SUB_CALLERS + DISCOUNT opcodes — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-SUB-CALLERS-DISCOUNT-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-SUB-CALLERS-DISCOUNT-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** full opcode catalog freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Extends the provisional event-body opcode set with:

| Opcode | Value | Typed body |
|--------|------:|------------|
| `SUB_CALLERS` | 11 | ULEB `fid`, `line`, `count`, `incl`, `excl`, `reci`, `rec_depth` + string-blob `called` + string-blob `caller` |
| `DISCOUNT` | 12 | empty (opcode + flags only) |

Same framing as other opcodes: `ULEB128 opcode || u8 flags || typed-body`.  
`incl` / `excl` / `reci` are **integer ULEB** (provisional) — not float/NV dual-equality; DISCOUNT is a marker only (not BASE-003 accounting freeze).

It is **not**:

- a full v5 opcode catalog freeze (full dual-output sequence OI-001-03 if deferred, …; ATTRIBUTE/OPTION/COMMENT/START_DEFLATE/VERSION are sibling preflights);
- exact DISCOUNT overhead accounting vs BASE-003;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid SUB_CALLERS | **Err** |
| DISCOUNT missing flags | **Err** |
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

1. Remaining stream-control items (full dual-output sequence-number freeze OI-001-03 if deferred, …) via ADR + golden vectors (ATTRIBUTE/OPTION/COMMENT/START_DEFLATE/VERSION + dual-output order recovery shipped as sibling preflights).
2. Exact DISCOUNT accounting; float/NV exactness for SUB_CALLERS times.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality.

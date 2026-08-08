# Format v6 event-body SUB_RETURN + SUB_INFO opcodes — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-SUB-RETURN-SUB-INFO-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-SUB-RETURN-SUB-INFO-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** full opcode catalog freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Extends the provisional event-body opcode set with:

| Opcode | Value | Typed body |
|--------|------:|------------|
| `SUB_RETURN` | 5 | ULEB `depth`, `incl`, `excl` + string-blob `subname` |
| `SUB_INFO` | 6 | ULEB `fid`, `first_line`, `last_line` + string-blob `name` |

Same framing as other opcodes: `ULEB128 opcode || u8 flags || typed-body`.  
`incl` / `excl` are **integer ULEB ticks** (provisional) — not float/NV dual-equality freeze.

It is **not**:

- a full v5 opcode catalog freeze (full dual-output sequence OI-001-03 if deferred, …; later sibling preflights include SRC_LINE/NEW_FID through VERSION);
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid SUB_RETURN / SUB_INFO | **Err** |
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

1. Remaining stream-control items (full dual-output sequence freeze OI-001-03 if deferred, …) via ADR + golden vectors (SRC_LINE/NEW_FID through VERSION shipped as sibling preflights).
2. Float/NV exactness for return times; dual-equality vs C.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008.

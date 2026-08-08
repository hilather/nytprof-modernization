# Format v6 event-body TIME_BLOCK + SUB_ENTRY opcodes — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-TIME-BLOCK-SUB-ENTRY-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-TIME-BLOCK-SUB-ENTRY-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** full opcode catalog freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Extends the provisional event-body opcode set with:

| Opcode | Value | Typed body (ULEB128) |
|--------|------:|----------------------|
| `TIME_BLOCK` | 3 | `fid`, `line`, `block_line`, `ticks` |
| `SUB_ENTRY` | 4 | `caller_fid`, `caller_line` |

Same framing as MARK/TIME_LINE: `ULEB128 opcode || u8 flags || typed-body`.

It is **not**:

- a full v5 opcode catalog freeze (SRC_LINE, PID_*, SUB_CALLERS, …; SUB_RETURN/SUB_INFO are sibling preflights);
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid TIME_BLOCK / SUB_ENTRY | **Err** |
| Reserved opcode 0 / unknown opcodes | **Err** (unchanged MVP) |
| Never panic | Required |

---

## Always-inflate recovery

Shipped consumers recover mixed bodies that include these opcodes:

- `decode_decoded_event_profile` (EVENT-only path) under NONE/ZLIB/ZSTD/LZ4
- `decode_decoded_mixed_profile` (multi-kind) with co-present SOURCE under NONE/ZLIB/ZSTD/LZ4 EVENT

Evidence: `cargo test -p nytprof-format-v6`.

---

## Open residual

1. Remaining opcodes and flag bits via ADR + golden vectors.
2. Default-parse inflate/CRC mutate.
3. Full COL-007 C writer / COL-008 / dual-equality.

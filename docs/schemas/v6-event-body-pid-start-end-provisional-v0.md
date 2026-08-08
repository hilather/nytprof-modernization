# Format v6 event-body PID_START + PID_END opcodes — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-PID-START-END-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-PID-START-END-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** full opcode catalog freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Extends the provisional event-body opcode set with:

| Opcode | Value | Typed body |
|--------|------:|------------|
| `PID_START` | 9 | ULEB `pid`, `ppid`, `start_time` |
| `PID_END` | 10 | ULEB `pid`, `end_time` |

Same framing as other opcodes: `ULEB128 opcode || u8 flags || typed-body`.  
Times are **integer ULEB** (provisional) — not float/NV dual-equality; not COL-015 fork re-init freeze.

It is **not**:

- a full v5 opcode catalog freeze (full dual-output sequence OI-001-03 if deferred, …; SUB_CALLERS through VERSION are sibling preflights);
- full PID lifecycle / fork re-init semantic freeze (COL-015);
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid PID_START / PID_END | **Err** |
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

1. Remaining stream-control items (full dual-output sequence-number freeze OI-001-03 if deferred, …) via ADR + golden vectors (SUB_CALLERS through VERSION + dual-output order recovery shipped as sibling preflights).
2. COL-015 fork re-init / full PID lifecycle freeze; float/NV exactness for times.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality.

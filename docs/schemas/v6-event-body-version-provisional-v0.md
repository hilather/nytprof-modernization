# Format v6 event-body VERSION opcode — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-VERSION-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-VERSION-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** full stream-control freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Extends the provisional event-body opcode set with:

| Opcode | Value | Typed body |
|--------|------:|------------|
| `VERSION` | 17 | ULEB `major`, `minor` |

Same framing as other opcodes: `ULEB128 opcode || u8 flags || typed-body`.  
Field shape is dump-aligned `args: [major, minor]` for preflight only.

It is **not**:

- full dual-output **sequence-number** freeze (OI-001-03) — dump-aligned multi-record **order recovery** is a sibling preflight (`FMT-V6-EVENT-BODY-DUAL-OUTPUT-SEQUENCE-*`);
- auto-emit VERSION freeze beyond the shipped preflight (`FMT-V6-AUTO-EMIT-VERSION-*`);
- mid-stream payload codec switch when START_DEFLATE is present;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid VERSION | **Err** |
| Reserved opcode 0 / unknown opcodes | **Err** (unchanged MVP) |
| Never panic | Required |

---

## Always-inflate recovery

Shipped consumers recover mixed bodies that include VERSION:

- `decode_decoded_event_profile` under NONE/ZLIB/ZSTD/LZ4
- `decode_decoded_mixed_profile` with co-present SOURCE under NONE/ZLIB/ZSTD/LZ4 EVENT
- dual-output multi-record sequence (VERSION-first order) under the same consumers

Evidence: `cargo test -p nytprof-format-v6`.

---

## Open residual

1. Full dual-output **sequence-number** freeze (OI-001-03 / COL-003) via ADR + golden vectors (order-recovery + auto-emit VERSION preflights are shipped separately).
2. Broader dual-equality / wire policy for header vs body VERSION beyond the preflight align helpers.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality.

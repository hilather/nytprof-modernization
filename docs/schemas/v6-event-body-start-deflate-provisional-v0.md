# Format v6 event-body START_DEFLATE opcode — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-START-DEFLATE-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-START-DEFLATE-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** full stream-control freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Extends the provisional event-body opcode set with:

| Opcode | Value | Typed body |
|--------|------:|------------|
| `START_DEFLATE` | 16 | empty (opcode + flags only) |

Same framing as other opcodes: `ULEB128 opcode || u8 flags || typed-body`.  
This preflight records **marker presence only** — it does **not** switch payload codec mid-stream on encode.

It is **not**:

- full dual-output **sequence-number** freeze for VERSION/START_DEFLATE (OI-001-03); VERSION body + dump-aligned multi-record order recovery + mid-stream chunk-codec switch preflight are sibling preflights;
- mid-stream automatic inflate/codec switch on the default `parse_chunk_frame` path;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| START_DEFLATE missing flags | **Err** |
| Reserved opcode 0 / unknown opcodes | **Err** (unchanged MVP) |
| Never panic | Required |

---

## Always-inflate recovery

Shipped consumers recover mixed bodies that include START_DEFLATE:

- `decode_decoded_event_profile` under NONE/ZLIB/ZSTD/LZ4
- `decode_decoded_mixed_profile` with co-present SOURCE under NONE/ZLIB/ZSTD/LZ4 EVENT

Evidence: `cargo test -p nytprof-format-v6`.

---

## Open residual

1. Full dual-output **sequence-number** freeze (OI-001-03) via ADR + golden vectors (VERSION body + dump-aligned order recovery + mid-stream chunk-codec switch preflight shipped as sibling preflights).
2. v5-style mid-payload stream deflate (if ever required) beyond the shipped **chunk-framed** post-marker codec switch preflight.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality.

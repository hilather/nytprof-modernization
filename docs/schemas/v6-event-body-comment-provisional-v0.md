# Format v6 event-body COMMENT opcode — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-COMMENT-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-COMMENT-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** full opcode catalog freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Extends the provisional event-body opcode set with:

| Opcode | Value | Typed body |
|--------|------:|------------|
| `COMMENT` | 15 | string-blob `text` |

Same framing as other opcodes: `ULEB128 opcode || u8 flags || typed-body`.  
Text is a **string projection** only — not COMPAT-002 volatile normalization freeze for comparators.

It is **not**:

- full dual-output sequence freeze OI-001-03 (START_DEFLATE/VERSION event records are sibling preflights; not mid-stream codec switch);
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid COMMENT | **Err** |
| Reserved opcode 0 / unknown opcodes | **Err** (unchanged MVP) |
| Never panic | Required |

---

## Always-inflate recovery

Shipped consumers recover mixed bodies that include COMMENT:

- `decode_decoded_event_profile` under NONE/ZLIB/ZSTD/LZ4
- `decode_decoded_mixed_profile` with co-present SOURCE under NONE/ZLIB/ZSTD/LZ4 EVENT

Evidence: `cargo test -p nytprof-format-v6`.

---

## Open residual

1. Remaining stream-control items (full dual-output sequence freeze OI-001-03 if deferred; START_DEFLATE/VERSION shipped as sibling preflights) via ADR + golden vectors.
2. COMPAT-002 volatile normalization of comment text for comparators.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality.

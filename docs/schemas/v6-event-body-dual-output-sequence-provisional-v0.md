# Format v6 event-body dual-output sequence — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-DUAL-OUTPUT-SEQUENCE-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-DUAL-OUTPUT-SEQUENCE-MVP` (shipped multi-record order recovery + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); VERSION / COMMENT / ATTRIBUTE / OPTION / START_DEFLATE / PID_* sibling preflights; always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** full OI-001-03 dual-output sequence-number freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Documents a **dump-aligned provisional multi-record EVENT body order** (COMPAT-001 illustrative stream shape) recovered by shipped encode/decode:

```text
VERSION
→ COMMENT* and/or ATTRIBUTE* and/or OPTION*
→ START_DEFLATE?
→ PID_START … (≥1 interior record) … PID_END
```

Shipped evidence uses a representative sequence: VERSION → COMMENT → ATTRIBUTE → OPTION → START_DEFLATE → PID_START → TIME_LINE → MARK → PID_END.

It is **not**:

- full **OI-001-03** freeze of whether VERSION / START_DEFLATE participate in dual-output **sequence numbers** (COL-003 / ADR + golden vectors);
- auto-emit VERSION freeze beyond shipped `FMT-V6-AUTO-EMIT-VERSION-*` preflight;
- mid-stream payload codec switch freeze beyond the shipped chunk-framed preflight (`FMT-V6-START-DEFLATE-MID-STREAM-CODEC-SWITCH-*`);
- complete ATTRIBUTE/OPTION key vocabulary freeze; COMPAT-002 comment normalize;
- wire freeze / dual-equality / CLI v6 default / FMT-012 golden corpus;
- permission to mark **COL-007** / **COL-008** done;
- mutating default `parse_chunk_frame` to always inflate or verify CRC.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid multi-record sequence | **Err** |
| Unknown / reserved opcodes | **Err** (unchanged MVP) |
| Never panic | Required |

---

## Always-inflate recovery

Shipped consumers recover the ordered dual-output sequence:

- `decode_event_body` — order + field round-trip
- `decode_decoded_event_profile` under NONE/ZLIB/ZSTD/LZ4 (default stream parse remains non-inflating)
- `decode_decoded_mixed_profile` with co-present SOURCE under NONE/ZLIB/ZSTD/LZ4 EVENT

Evidence: `cargo test -p nytprof-format-v6` (`dual_output_sequence_*`, `mixed_dual_output_sequence_*`).

---

## Open residual

1. Full dual-output **sequence-number** freeze (OI-001-03 / COL-003) via ADR + golden vectors.
2. Broader dual-equality policy for header vs body VERSION beyond auto-emit preflight.
3. v5-style mid-payload stream deflate (if required) beyond chunk-framed mid-stream codec-switch preflight.
4. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; key vocabularies.

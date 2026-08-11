# Format v6 event-body logical event sequence numbers — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-SEQ-NUMBER-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-SEQ-NUMBER-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); dual-output order [`v6-event-body-dual-output-sequence-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-dual-output-sequence-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway / **OI-001-03** preflight only — **before** full dual-output sequence-number freeze (COL-003 / ADR + golden vectors)

---

## Scope and non-claims

Allows each event-body wire record to carry an optional **monotonic logical event sequence number** recovered alongside the ordered logical events.

### Flag

| Flag | Value | Meaning |
|------|------:|---------|
| `FLAG_HAS_SEQ` | `0x08` | After flags: ULEB128 `seq`, then typed body |

Default [`encode_event_body`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6/src/event_body.rs) omits the flag (no seq field).  
[`encode_event_body_with_seq`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6/src/event_body.rs) assigns `0, 1, 2, …` in logical recovery order.

### Packed runs

`TIME_LINE_RUN` / `TIME_BLOCK_RUN` write a **base** sequence on the wire record. Decode expands to `base .. base+N-1` (one seq per logical TIME_LINE / TIME_BLOCK).

### Recovery

| API | Role |
|-----|------|
| `decode_event_body_full` | `(records, sequences)` via `EventBodyDecoded` |
| `decode_event_body` | records only (still parses seq field when present) |
| `DecodedEventProfile.sequences` | parallel to `records` after always-inflate |
| `DecodedMixedProfile.event_sequences` | parallel to `event_records` |

It is **not**:

- full **OI-001-03** / COL-003 freeze of whether VERSION / START_DEFLATE participate in dual-output sequence numbers (this preflight **does** assign seq to them when encoded via `encode_event_body_with_seq`, but that is **not** a permanent policy freeze);
- a permanent flag-bit ADR;
- permanent packing / string-pool freeze;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid-sequence ULEB | **Err** (varint / truncated) |
| Truncated mid typed body after seq | **Err** (unchanged) |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`event_seq_*`, `mixed_event_seq_*`).

- Dual-output-shaped multi-record order **and** per-event seq values
- EVENT NONE/ZLIB/ZSTD/LZ4; mixed + SOURCE co-kind
- Default stream parse / `parse_chunk_frame` non-inflating for compressed payloads
- Prior TIME_LINE_RUN / TIME_BLOCK_RUN / site-delta tests remain green

---

## Open residual

1. Full OI-001-03 / COL-003 dual-output sequence-number freeze (ADR + golden vectors; VERSION/START_DEFLATE participation policy).
2. Complete OI-002 inventory; permanent packing/string-pool ADRs (composed site-delta+seq preflight is a sibling — see [`v6-event-body-site-delta-seq-compose-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-site-delta-seq-compose-provisional-v0.md)).
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

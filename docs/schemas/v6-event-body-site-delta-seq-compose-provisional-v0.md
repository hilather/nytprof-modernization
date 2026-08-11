# Format v6 event-body composed site-delta + sequence-number packing — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-SITE-DELTA-SEQ-COMPOSE-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-SITE-DELTA-SEQ-COMPOSE-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** site-delta [`v6-event-body-site-delta-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-site-delta-provisional-v0.md); seq-number [`v6-event-body-seq-number-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-seq-number-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** permanent packing ADR / full OI-001-03 freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Allows **composed** provisional packing on the same wire record:

| Flag combination | Value | Meaning |
|------------------|------:|---------|
| `FLAG_SITE_DELTA \| FLAG_HAS_SEQ` | `0x0C` | ULEB seq after flags, then ZigZag site deltas (TIME_LINE / TIME_BLOCK / SUB_ENTRY) |

### Wire layout (site-bearing opcodes)

```text
opcode || u8 flags(SITE_DELTA|HAS_SEQ) || ULEB128 seq || i64 fid_delta || i64 line_delta || [i64 block_line_delta] || u64 ticks
```

### Shipped encode helper

| Item | Role |
|------|------|
| `encode_event_body_with_site_deltas_and_seq` | Absolute specs → composed wire |
| `decode_event_body_full` | Absolute sites + per-event sequences |

- Default `encode_event_body`: absolute, **no** seq (unchanged).
- Pure site-delta / pure seq paths remain available and green.
- Packed runs under compose use absolute body + `FLAG_HAS_SEQ` only (no site-delta on run form); expand still assigns base..base+N-1.

It is **not**:

- a permanent packing ADR / flag-bit freeze;
- full OI-001-03 / COL-003 dual-output sequence-number policy freeze;
- complete OI-002 inventory freeze;
- permanent global string-pool freeze;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid-seq ULEB | **Err** (varint / truncated) |
| Truncated mid-delta field | **Err** (varint / truncated) |
| Delta reconstruction outside `u64` | **Err** (`InvalidSiteDelta`) |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`site_delta_and_seq_compose_*`, `mixed_site_delta_and_seq_compose_*`).

- Multi-record absolute site reconstruction **and** monotonic seq values
- EVENT NONE/ZLIB/ZSTD/LZ4; mixed + SOURCE co-kind
- Default stream parse / `parse_chunk_frame` non-inflating for compressed payloads
- Prior pure site-delta, pure seq, TIME_LINE_RUN, TIME_BLOCK_RUN tests remain green

---

## Open residual

Permanent packing **intent** is proposed in [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md) (status proposed — not wire freeze; dual-equality readiness: [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md)).

1. Permanent ADR freeze of packing / flag bits / opcode alternate forms (dictionary+packing compose and multi-chunk packing continuity are siblings — see [`v6-string-dict-site-delta-seq-compose-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-dict-site-delta-seq-compose-provisional-v0.md), [`v6-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-site-delta-seq-packing-provisional-v0.md)).
2. Full OI-001-03 freeze; complete OI-002 freeze; permanent string pool.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

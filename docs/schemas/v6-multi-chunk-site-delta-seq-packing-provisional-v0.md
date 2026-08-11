# Format v6 multi-chunk record-aligned site-delta/seq packing continuity — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-PROVISIONAL` (contract), `FMT-V6-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** site-delta+seq compose [`v6-event-body-site-delta-seq-compose-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-site-delta-seq-compose-provisional-v0.md); multi-chunk partition [`v6-multi-chunk-event-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-event-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** permanent packing ADR / dual-equality / C v6 writer

---

## Scope and non-claims

Allows **multi-chunk record-aligned** EVENT streams whose bodies use provisional site-delta and logical sequence-number packing such that:

1. Site bases and sequence numbers **continue across chunk boundaries** (not reset per chunk).
2. Always-inflate join recovers the same ordered absolute logical events and per-event sequences as a **single-chunk** packing encode of the same specs.
3. Concatenating per-partition packing plains equals the single-chunk packing wire body.

### Shipped API

| Item | Role |
|------|------|
| `PackingEncodeState` | Continued site bases + next sequence |
| `encode_event_body_with_site_deltas_and_seq_continuing` | Encode one partition; updates state |
| `encode_decoded_event_profile_with_site_deltas_and_seq` | Multi-chunk packing profile (`max_events_per_chunk ≥ 1`) |

### Continuity rule

```text
state = PackingEncodeState::new()
for part in partition_event_records(events, max_per_chunk):
    plain_i = encode_event_body_with_site_deltas_and_seq_continuing(part, &mut state)
join(plain_i) == encode_event_body_with_site_deltas_and_seq(events)
```

Naive per-chunk `encode_event_body_with_site_deltas_and_seq(part)` without shared state is **incorrect** (seq/site reset).

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
| Truncated mid-seq / mid-delta | **Err** (varint / truncated) |
| Mid-record span residual | unchanged mid-record preflight (record-aligned partition only here) |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`multi_chunk_packing_*`, `multi_chunk_site_delta_seq_*`, `mixed_multi_chunk_site_delta_seq_*`).

- Multi-chunk packing equals single-chunk recovered records/seq under NONE/ZLIB/ZSTD/LZ4
- Mixed multi-chunk EVENT + SOURCE co-kind
- Default stream parse / `parse_chunk_frame` non-inflating for compressed EVENT payloads
- Prior single-chunk packing, dict compose, absolute multi-chunk tests remain green

---

## Open residual

Permanent packing **intent** is proposed in [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md) (status proposed — not wire freeze; dual-equality readiness: [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md)).

Mid-stream codec-switch + packing continuity is a sibling: [`v6-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md).

Auto-VERSION + multi-chunk packing compose is a sibling: [`v6-auto-version-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-auto-version-multi-chunk-site-delta-seq-packing-provisional-v0.md).

1. Permanent packing ADR / permanent string-pool ADR (TIME_*_RUN multi-chunk packing continuity is a sibling — see [`v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md)). Multi-chunk packing **with** FOOTER string-dictionary is a sibling: [`v6-string-dict-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-dict-multi-chunk-site-delta-seq-packing-provisional-v0.md).
2. Full OI-001-03 freeze; complete OI-002 freeze.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

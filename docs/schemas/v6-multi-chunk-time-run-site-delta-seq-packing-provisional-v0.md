# Format v6 multi-chunk packing continuity with TIME_LINE_RUN / TIME_BLOCK_RUN — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-MULTI-CHUNK-TIME-RUN-SITE-DELTA-SEQ-PACKING-PROVISIONAL` (contract), `FMT-V6-MULTI-CHUNK-TIME-RUN-SITE-DELTA-SEQ-PACKING-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** multi-chunk packing continuity [`v6-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-site-delta-seq-packing-provisional-v0.md); TIME_LINE_RUN [`v6-event-body-time-line-run-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-time-line-run-provisional-v0.md); TIME_BLOCK_RUN [`v6-event-body-time-block-run-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-time-block-run-provisional-v0.md); site-delta+seq compose [`v6-event-body-site-delta-seq-compose-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-site-delta-seq-compose-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** permanent packing ADR / dual-equality / C v6 writer

---

## Scope and non-claims

Extends multi-chunk record-aligned site-delta/seq packing so EVENT streams may include **TIME_LINE_RUN** and/or **TIME_BLOCK_RUN** coexisting with site-delta TIME_LINE / TIME_BLOCK / SUB_ENTRY:

1. Site bases and sequence numbers **continue across chunk boundaries** (`PackingEncodeState` + continuing encode).
2. Packed runs advance the encode **SiteCursor** (and seq by N) so a site-delta event **after** a run reconstructs the same absolute site whether the post-run record is in the same chunk or a **later** record-aligned partition.
3. Always-inflate join recovers the same ordered absolute logical events and per-event sequences as a single-chunk packing compose of the same specs.

Partitioning is **record-aligned** (`partition_event_records`): a whole run record stays in one chunk; mid-run body span is not claimed here.

### Continuity rule

```text
state = PackingEncodeState::new()
for part in partition_event_records(events, max_per_chunk):
    plain_i = encode_event_body_with_site_deltas_and_seq_continuing(part, &mut state)
join(plain_i) == encode_event_body_with_site_deltas_and_seq(events)
# runs update state.site (fid/line[/block_line]) and next_seq += N
```

### Shipped helpers

| Item | Role |
|------|------|
| `PackingEncodeState` + `encode_event_body_with_site_deltas_and_seq_continuing` | Continuous packing; TIME_*_RUN advances site + seq |
| `encode_decoded_event_profile_with_site_deltas_and_seq` | Multi-chunk packing profile encode (`max_events_per_chunk`) |
| Always-inflate `decode_decoded_event_profile` / `decode_decoded_mixed_profile` | Join + expand runs + absolute sites + sequences |

It is **not**:

- a permanent packing ADR / flag-bit freeze;
- full OI-001-03 / COL-003 dual-output sequence-number policy freeze;
- complete OI-002 inventory freeze;
- permanent global string-pool freeze;
- mid-record span of run bodies across chunks;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Empty / oversize TIME_*_RUN | **Err** (`EmptyTimeLineRun` / `OversizeTimeLineRun` / block siblings) |
| Truncated mid-run / mid-seq / mid-delta | **Err** |
| Record-aligned partition only | mid-run span residual unchanged |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`multi_chunk_packing_with_time_runs_*`, `mixed_multi_chunk_packing_with_time_runs_*`).

- Multi-chunk packing + TIME_*_RUN equals single-chunk recovered records/seq under NONE/ZLIB/ZSTD/LZ4
- Site-delta after run across a chunk boundary lands on correct absolute site
- Mixed multi-chunk EVENT + SOURCE co-kind
- Default stream parse / `parse_chunk_frame` non-inflating for compressed EVENT payloads
- Prior multi-chunk packing, string-dict multi-chunk packing, single-chunk TIME_*_RUN, and site-delta-after-run tests remain green

---

## Open residual

Auto-VERSION + multi-chunk packing compose is a sibling: [`v6-auto-version-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-auto-version-multi-chunk-site-delta-seq-packing-provisional-v0.md).

Triple compose (FOOTER dict + multi-chunk packing + TIME_*_RUN) is a sibling: [`v6-string-dict-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-dict-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md).

1. Permanent packing ADR / permanent string-pool ADR.
2. Full OI-001-03 freeze; complete OI-002 freeze.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

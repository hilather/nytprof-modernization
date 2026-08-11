# Format v6 event-body TIME_BLOCK_RUN packed-run — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-TIME-BLOCK-RUN-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-TIME-BLOCK-RUN-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); sibling TIME_LINE_RUN [`v6-event-body-time-line-run-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-time-line-run-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** permanent packing ADR / dual-equality / C v6 writer

---

## Scope and non-claims

Allows consecutive **same-site** `TIME_BLOCK` samples to be carried as a single packed wire record that **expands** on decode to the exact ordered list of logical `TIME_BLOCK` events, retaining **every per-event ticks value** (not sum/count-only aggregation).

### Opcode

| Value | Name | Typed body |
|------:|------|------------|
| 19 | `TIME_BLOCK_RUN` | `u64 fid`, `u64 line`, `u64 block_line`, `u64 N`, then `N` × `u64 ticks` (all ULEB128) |

### Expansion rule

```text
TIME_BLOCK_RUN(fid, line, block_line, N, ticks[0..N-1])
  →  TIME_BLOCK(fid, line, block_line, ticks[0])
  →  TIME_BLOCK(fid, line, block_line, ticks[1])
  →  …
  →  TIME_BLOCK(fid, line, block_line, ticks[N-1])
```

- All expanded events share the same absolute `(fid, line, block_line)`.
- There is **no** hidden logical event between expanded members of one run.
- Multi-run streams and mixed plain `TIME_BLOCK` + `TIME_BLOCK_RUN` streams (and coexistence with plain `TIME_LINE` / `TIME_LINE_RUN`) expand to one ordered absolute sequence.
- Encode via `EventRecordSpec::TimeBlockRun { fid, line, block_line, ticks }`; absolute path flags `0` (not `FLAG_SITE_DELTA`).

It is **not**:

- a permanent packing ADR / alternate opcode freeze;
- permanent site-delta packing freeze (sibling preflight remains);
- full OI-001-03 sequence-number freeze; complete OI-002 inventory;
- permanent global string-pool freeze;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| `N = 0` | **Err** (`EmptyTimeBlockRun`) |
| `N > MAX_TIME_BLOCK_RUN_LEN` (1_048_576) | **Err** (`OversizeTimeBlockRun`) before expand |
| Truncated mid-run (missing tick varint) | **Err** (varint / truncated) |
| Remaining bytes cannot hold N ticks (lower bound) | **Err** (truncated) before expand |

---

## Shipped API

| Item | Role |
|------|------|
| `opcode::TIME_BLOCK_RUN` (19) | Packed-run opcode |
| `MAX_TIME_BLOCK_RUN_LEN` | Fail-closed run-length cap |
| `EventRecordSpec::TimeBlockRun` | Encode path |
| `decode_event_body` | Expands to N logical `EventRecord::TimeBlock` |

Logical decode surface remains `TimeBlock` only (no `TimeBlockRun` owned record).

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`time_block_run_*`, `mixed_time_block_run_*`).

- Multi-run + mixed plain TIME_BLOCK / TIME_BLOCK_RUN + TIME_LINE_RUN coexistence → ordered absolute sequence
- Every per-event ticks retained (not sum/count)
- EVENT NONE/ZLIB/ZSTD/LZ4; mixed + SOURCE co-kind
- Fail-closed truncated mid-run / empty / oversize
- Default stream parse / `parse_chunk_frame` non-inflating for compressed payloads
- Prior TIME_LINE_RUN tests remain green

---

## Open residual

Multi-chunk packing continuity with this run form is a sibling preflight: [`v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md).

1. Permanent ADR freeze of run/delta packing / opcode alternate forms.
2. Full OI-001-03 freeze; full OI-002; permanent string pool (seq-number preflight is a sibling).
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

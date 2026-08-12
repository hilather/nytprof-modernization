# Format v6 event-body TIME_LINE_RUN packed-run — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-TIME-LINE-RUN-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-TIME-LINE-RUN-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** permanent packing ADR / dual-equality / C v6 writer

---

## Scope and non-claims

Allows consecutive **same-site** `TIME_LINE` samples to be carried as a single packed wire record that **expands** on decode to the exact ordered list of logical `TIME_LINE` events, retaining **every per-event ticks value** (not sum/count-only aggregation).

### Opcode

| Value | Name | Typed body |
|------:|------|------------|
| 18 | `TIME_LINE_RUN` | `u64 fid`, `u64 line`, `u64 N`, then `N` × `u64 ticks` (all ULEB128) |

### Expansion rule

```text
TIME_LINE_RUN(fid, line, N, ticks[0..N-1])
  →  TIME_LINE(fid, line, ticks[0])
  →  TIME_LINE(fid, line, ticks[1])
  →  …
  →  TIME_LINE(fid, line, ticks[N-1])
```

- All expanded events share the same absolute `(fid, line)`.
- There is **no** hidden logical event between expanded members of one run.
- Multi-run streams and mixed plain `TIME_LINE` + `TIME_LINE_RUN` streams expand to one ordered absolute `TIME_LINE` sequence.
- Encode via `EventRecordSpec::TimeLineRun { fid, line, ticks }`; absolute path flags `0` (not `FLAG_SITE_DELTA`).

It is **not**:

- a permanent packing ADR / alternate opcode freeze;
- permanent `TIME_BLOCK_RUN` packing freeze (sibling preflight — see [`v6-event-body-time-block-run-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-time-block-run-provisional-v0.md));
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
| `N = 0` | **Err** (`EmptyTimeLineRun`) |
| `N > MAX_TIME_LINE_RUN_LEN` (1_048_576) | **Err** (`OversizeTimeLineRun`) before expand |
| Truncated mid-run (missing tick varint) | **Err** (varint / truncated) |
| Remaining bytes cannot hold N ticks (lower bound) | **Err** (truncated) before expand |

---

## Shipped API

| Item | Role |
|------|------|
| `opcode::TIME_LINE_RUN` (18) | Packed-run opcode |
| `MAX_TIME_LINE_RUN_LEN` | Fail-closed run-length cap |
| `EventRecordSpec::TimeLineRun` | Encode path |
| `decode_event_body` | Expands to N logical `EventRecord::TimeLine` |

Logical decode surface remains `TimeLine` only (no `TimeLineRun` owned record).

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`time_line_run_*`, `mixed_time_line_run_*`).

- Multi-run + mixed plain TIME_LINE / TIME_LINE_RUN → ordered absolute TIME_LINE sequence
- Every per-event ticks retained (not sum/count)
- EVENT NONE/ZLIB/ZSTD/LZ4; mixed + SOURCE co-kind
- Fail-closed truncated mid-run / empty / oversize
- Default stream parse / `parse_chunk_frame` non-inflating for compressed payloads

---

## Open residual

Multi-chunk packing continuity with this run form is a sibling preflight: [`v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md).

1. Permanent ADR freeze of run/delta packing / opcode alternate forms.
2. Full OI-001-03 freeze; full OI-002; permanent string pool (TIME_BLOCK_RUN + seq-number preflights are siblings).
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

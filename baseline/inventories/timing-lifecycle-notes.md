# BASE-003 — Timing, call, numeric, and lifecycle freeze notes

**Status:** provisional freeze notes + open-item list (not a complete executable state machine yet)  
**Depends on:** BASE-001 pin, BASE-002 tag inventory  
**Date:** 2026-08-07  
**Oracle sources:** `NYTProf.xs` statement/call hooks, `FileHandle.xs` time writers, `ReadStream.pm`

## Binding principles (from COMPAT contract)

1. Statement attribution follows the legacy collector state machine — do not “improve” attribution.
2. Discount markers exclude profiler overhead exactly as 6.15 does.
3. v6 may store integer ticks; display/API float conversion happens only at historical FP boundaries.
4. Fake-clock / deterministic tests are required before collector refactors (COL-*).

## Numeric layout (v5 wire, provisional)

| Concept | Representation | Notes |
|---------|----------------|-------|
| Statement/block elapsed | `I32 elapsed` + `U32 overflow` in writers | See `NYTP_write_time_line/block` signatures in `FileHandle.h` |
| Process times | `NV` (native Perl double image) | `NYTP_write_process_start/end` |
| Sub callers times | `NV` incl/excl/reci | `NYTP_write_sub_callers` |
| Call return times | inclusive/exclusive ticks as written by call path | Confirm units in OI-003-02 |
| Header `nv_size` | attribute metadata | ReadStream exposes `nv_size` |
| Endian / pointer size | platform attributes in profile header | Cross-platform corpus later (TEST-017) |

**Open OI-003-01:** Exact composition of tick from elapsed+overflow and overflow threshold.  
**Open OI-003-02:** Whether call-return payload is NV seconds, integer ticks, or mixed — freeze from XS + fixtures.

## Statement timing (provisional model)

```text
On breakable statement entry (DB hooks / op hook path):
  read clock
  attribute (now - last) to previous statement/block
  update last = now
  record TIME_LINE or TIME_BLOCK with fid/line[/block_line/sub_line]
Discount:
  when profiler executes internal work, emit DISCOUNT and adjust counts/time
  per legacy placement relative to flushes
```

**Freeze rule:** No collector sink refactor (COL-001+) until a fake-clock suite proves this sequence for default options (TEST-003).

## Call modes (provisional)

| `calls` | Wire behavior | Required for Phase-0 |
|---------|---------------|----------------------|
| `0` | aggregates via SUB_INFO / SUB_CALLERS; no entry/return stream | yes |
| `1` | SUB_RETURN events | yes (basic) |
| `2` | SUB_ENTRY + SUB_RETURN | yes (basic) |

Depth, leave/correction, XSUB, `goto &sub`, exceptions: **open** (OI-003-03) — fixture matrix required; until then native paths must match oracle or fall back.

## Lifecycle / process

| Event | Tag | Notes |
|-------|-----|-------|
| Process start | `PID_START` | pid, ppid, start time NV |
| Process end | `PID_END` | pid, end time NV |
| Fork | re-init / new PID records | COL-015 later; stress in TEST-018 |
| Compression start | `START_DEFLATE` | after header attrs typically |
| Finalization | sub callers + sources + pid end | order must be fixture-captured |

**Open OI-003-04:** Exact finalization order for END/global destruction/`sigexit`/`posix_exit` modes.  
**Open OI-003-05:** File switch / restart / enable-disable interactions (`test50`/`test51` exist in oracle suite).

## Clock selection

ReadStream exposes `clock_id` and `ticks_per_sec`. Supported clocks remain those of 6.15 on each platform. Anomaly/monotonic behavior: **open OI-003-06** (platform matrix).

## What is frozen now

1. Tag set for timing/call/process events (BASE-002 table).  
2. Requirement that dual-output and native readers compare **canonical logical events**, not wall-clock variance.  
3. Requirement for fake-clock before collector changes.  
4. No precision-reducing aggregation on the storage path.

## What is explicitly not frozen

- Full state-machine diagrams for every exception path  
- Merge semantics across clock domains (ADR-Q023)  
- v6 tick signedness (ADR-Q002)  
- Exotic NV widths (ADR-Q013)

## Open items summary

| ID | Item | Unblock |
|----|------|---------|
| OI-003-01 | elapsed+overflow → logical ticks | XS read + golden vectors |
| OI-003-02 | call-return numeric units | ReadStream dump of calls=1/2 fixture |
| OI-003-03 | leave/XSUB/goto/exception matrix | expand fixtures |
| OI-003-04 | finalization order modes | lifecycle fixtures |
| OI-003-05 | enable/disable/restart | existing t/test50–51 + capture |
| OI-003-06 | clock anomaly matrix | platform later |

## Next executable steps

1. `tools/oracle/capture_fixture.sh` for `stmts+blocks+calls=1` and `calls=2`.  
2. Dump ReadStream JSONL; attach to `fixtures/v5/`.  
3. Promote OI-003-01/02 to frozen fields when dumps agree with XS.

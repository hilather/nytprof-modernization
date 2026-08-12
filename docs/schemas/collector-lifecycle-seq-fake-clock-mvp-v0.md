# Collector lifecycle + sequence + fake-clock — MVP v0 (COL-002 / COL-003 / TEST-003 scaffold)

**Status:** provisional scaffolding (not a wire freeze; not full M4 oracle gate)  
**Tasks:** COL-002, COL-003, TEST-003 slice (PR-B03)  
**Depends on:** COL-001 sink API ([`collector-sink-api-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-sink-api-mvp-v0.md))  
**Layout:** [ADR-0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md) B0-A `collector/`  
**Timing notes:** [BASE-003](https://github.com/hilather/nytprof-modernization/blob/main/baseline/inventories/timing-lifecycle-notes.md)

---

## Intent

1. **COL-002** — Freeze an explicit sink lifecycle with legal transitions and emit gates.  
2. **COL-003** — Assign gapless monotonic logical event sequence numbers at the public emit boundary (internal / dual-compare; **not** written on default v5 wire).  
3. **TEST-003 scaffold** — Deterministic fake-clock + BASE-003 statement-attribution driver + **M4 mini sample** harness so lifecycle/seq/attribution are regression-tested before COL-006/007.

---

## Lifecycle (COL-002)

```text
UNINITIALIZED -> OPEN -> ACTIVE -> STOPPED -> FINALIZING -> CLOSED
                       |          |
                       |          +-> ACTIVE (restart)
                       +-> FORK_SPLIT -> parent ACTIVE / child OPEN (seq reset)
                       +-> FAILED
```

| API | Transition |
|-----|------------|
| create backends | → `OPEN` |
| `nytp_sink_activate` | `OPEN` \| `STOPPED` → `ACTIVE` |
| `nytp_sink_stop` | `ACTIVE` → `STOPPED` |
| `nytp_sink_begin_finalize` | `OPEN` \| `ACTIVE` \| `STOPPED` → `FINALIZING` |
| `nytp_sink_begin_fork` / `end_fork_parent` / `end_fork_child` | `ACTIVE` ↔ `FORK_SPLIT`; child → `OPEN` + seq reset |
| `nytp_sink_mark_failed` | any non-`CLOSED` → `FAILED` |
| `nytp_sink_close` | any non-`CLOSED` → `CLOSED` (idempotent) |

### Emit gates

| State | Allowed emits |
|-------|----------------|
| `OPEN`, `ACTIVE` | all kinds (header + hot path) |
| `FINALIZING` | `SRC_LINE`, `SUB_INFO`, `SUB_CALLERS`, `PID_END`, meta (`ATTRIBUTE`/`OPTION`/`COMMENT`), `DISCOUNT` |
| `STOPPED`, `FORK_SPLIT`, `FAILED`, `CLOSED` | none |

Helpers: `nytp_sink_transition_allowed`, `nytp_sink_can_emit`, `nytp_sink_state_name`.

---

## Sequence numbers (COL-003)

| Rule | Detail |
|------|--------|
| Domain | `nytp_seq` (`uint64_t`), starts at **0** per process stream |
| Assignment | Public `nytp_emit_*` wrappers, on **successful** logical emit only |
| Logical kinds | All COMPAT-001 mapped kinds **except** `START_DEFLATE` (control) and `NONE` |
| Gapless | `next_seq` advances by 1; `nytp_seq_check_gapless` reports first mismatch |
| Child fork | `nytp_sink_end_fork_child` resets seq to 0 |
| v5 default | **Does not write** seq on wire (stub has no wire; COL-006 must preserve default) |
| Dual/test | Counting + stub v5 record `last_seq` + seq ring for comparators |

API: `nytp_sink_peek_seq`, `nytp_sink_last_seq`, `nytp_sink_logical_count`, `nytp_event_kind_is_logical`, `nytp_seq_check_gapless`.

---

## Fake-clock (TEST-003 scaffold)

| Piece | Role |
|-------|------|
| `nytp_fake_clock` | Scripted absolute tick reads; fail-closed on exhaust (`NYTP_ERR_EXHAUSTED`) |
| `nytp_stmt_driver` | BASE-003: on statement entry, attribute `(now - last)` to **previous** line |
| `nytp_m4_mini_sample_run` | Synthetic mini stream under counting / stub-v5 sink |

### M4 mini sample (not full corpus)

Synthetic order (logical events only; `START_DEFLATE` may appear as control without seq):

`ATTRIBUTE` → `OPTION` → activate → `START_DEFLATE` (control) → `PID_START` → `NEW_FID` →  
TIME_LINE(42@L1) → DISCOUNT → TIME_LINE(58@L2) → TIME_LINE(50@L3) →  
`SUB_RETURN` → finalize → `SRC_LINE` → `SUB_INFO` → `PID_END` → close  

Clock script: absolute ticks `1000, 1042, 1100, 1150`.

**Residual:** full fixture M4 (`fixtures/v5/*` oracle dumps) v5-via-sink equality is **not** claimed — needs **COL-006** real wire + complete TEST-003 corpus. See residual matrix.

---

## Acceptance for this MVP

| Check | Evidence |
|-------|----------|
| Lifecycle transitions + emit gates | `make -C collector test` → `test_lifecycle_seq` |
| Gapless seq; control excluded | same |
| Fake-clock + stmt driver + mini M4 | `test_fake_clock` |
| COL-001 surface still green | `test_sink_api` |
| Smoke + isolation | `scripts/packaging/collector_sink_smoke.sh` |
| Offline gate honest without CC | `scripts/ci/offline_gate.sh` step 10 |

---

## Residuals / non-claims

| Residual | Owner |
|----------|-------|
| Real v5 wire bytes | COL-006 |
| Full M4 oracle corpus under fake-clock | COL-006 + complete TEST-003 |
| C v6 writer | COL-007 |
| Full fork buffer ownership / signal-safe finalize | COL-015 |
| Live XS / opcode hooks | later COL / packaging |
| Production accidental fake-clock enable | keep test-only; no release default |

## Sources

- [`collector/include/nytp_sink.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytp_sink.h)  
- [`collector/include/nytp_clock.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytp_clock.h)  
- [`collector/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/collector/README.md)  

# Collector batch + statement fast path — MVP v0 (COL-004 / COL-005)

**Status:** provisional scaffolding (not a wire freeze; not production perf cert)  
**Tasks:** COL-004 (no-alloc statement fast path), COL-005 (bounded event batching)  
**PR:** PR-B04  
**Layout:** [ADR-0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md) B0-A `collector/`  
**Sources:** [`collector/include/nytp_batch.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytp_batch.h), [`collector/include/nytp_event.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytp_event.h), [`collector/src/nytp_batch.c`](https://github.com/hilather/nytprof-modernization/blob/main/collector/src/nytp_batch.c)  
**Depends on:** COL-001 sink, COL-002 lifecycle, COL-003 sequence (PR-B02/B03)

---

## Intent

1. **COL-004** — Common `TIME_LINE` / `TIME_BLOCK` capture appends a fixed POD event with **no general-heap allocation** after batch create.  
2. **COL-005** — Fixed event-header array + bounded side arena; high-water / full flush; oversized emergency path; exact order under forced capacities; **no borrowed Perl SV lifetime** beyond the emit call (bytes copied into arena).

## Surface (v0)

| Symbol | Role |
|--------|------|
| `nytp_event` | Fixed-size tagged event header (arena offsets for strings) |
| `nytp_batch` | Fixed capacity + arena; metrics; child drain |
| `nytp_batch_append_time_line` / `_time_block` | POD no-alloc appends |
| `nytp_batch_append_*` | Full kind set (strings → arena copy) |
| `nytp_batch_flush` | Replay to child ops; preserve COL-003 seq; reset only on full ack |
| `nytp_batch_sink_create` | Vtable sink facade over a batch |
| `nytp_fast_emit_time_line` / `_time_block` | Prebound batch fast path (seq commit inline) |
| `nytp_fast_bench_time_line` | Light engineering microbench (**not** BENCH certification) |

### Defaults / bounds

| Parameter | Default / limit |
|-----------|-----------------|
| Event capacity | default 64; max 4096; tests force 1..64 |
| Arena | default 4096 B; max 1 MiB |
| High-water | 0 ⇒ capacity; else `1..capacity` |

### Metrics (`nytp_batch_metrics`)

`appends`, `stmt_fast_appends`, `flushes`, `high_water_flushes`, `full_flushes`, `emergency_direct`, `arena_bytes_copied`, `heap_allocs` (create-time only; stmt path must not increment).

### Flush / order

- Flush drains events **in order** via child `ops->emit_*` (not public wrappers — avoids double COL-003 seq).  
- On success: `on_logical_committed(child, event.seq, kind)` + child seq state synced.  
- On **mid-batch failure**: already-acked prefix is **compacted out** (arena rebuilt for remaining); only unacked events remain — retry must not re-emit acked events.  
- Hard flush errors mark **child** failed and (via public `nytp_sink_flush` / emit path) **sticky-fail the batch sink**; further emits return `NYTP_ERR_STATE`.  
- Buffered event + high-water flush failure still advances COL-003 seq for the buffered event (`last_append_buffered`).  
- Reset of `count` / `arena_used` only after full successful drain.  
- Lifecycle: batch sink forwards stop/finalize/fork to child via optional `notify_*` ops.  
- **COL-015:** `notify_begin_fork` **preflushes** pending events; `notify_end_fork_child` **discards** residual (metrics `fork_preflush` / `fork_child_discard`). Prefer `nytp_fork_*` protocol wrapper.

### Oversized payload

If a string payload cannot fit the empty arena after flush → **emergency direct** emit to child (counts `emergency_direct`); still preserves seq and order relative to prior drained events.

## Acceptance for this MVP

| Check | Evidence |
|-------|----------|
| Compiles + unit tests | `make -C collector test` → `test_batch_fast` |
| Exact order caps 1..64 | dual: direct counting vs batch→counting kind/seq rings |
| SV lifetime | clobber caller buffers after append; flush still delivers original text |
| No stmt heap growth | `heap_allocs` stable; `arena_bytes_copied==0` on pure TIME_LINE/BLOCK |
| ASAN/UBSAN clean | sanitizer build of all collector test bins |
| Smoke | `scripts/packaging/collector_sink_smoke.sh` |

## Residuals / non-claims

| Residual | Owner |
|----------|-------|
| Real v5 wire encode | **COL-006 landed** — see collector-v5-wire-mvp-v0; full corpus residual |
| C v6 writer | COL-007 |
| Flush / compression **discount timing** vs BASE-003 oracle | dedicated timing ADR + complete TEST-003 |
| Production microbench certification | BENCH-003 / BENCH-006 (light bench here is **engineering only**) |
| Live Perl/XS opcode hooks | later COL |
| Dual-sink overhead product path | ARCH-007 |
| Full TEST-018 oracle forkdepth/addpid (beyond unit stress) | TEST-018 / COL-015 residual |

## Tests

- `collector/t/test_batch_fast.c` — capacity stress, SV lifetime, fast path, high-water, emergency, fail-flush retain, light microbench.

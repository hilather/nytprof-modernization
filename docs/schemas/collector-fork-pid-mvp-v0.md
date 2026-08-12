# Collector fork/PID protocol — MVP v0 (COL-015)

**Status:** provisional scaffolding (not a wire freeze; not full TEST-018 oracle matrix)  
**Tasks:** COL-015 (harden fork and PID transitions with buffered sinks)  
**PR:** PR-C02b  
**Layout:** [ADR-0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md) B0-A `collector/`  
**Sources:** [`collector/include/nytp_fork.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytp_fork.h), [`collector/src/nytp_fork.c`](https://github.com/hilather/nytprof-modernization/blob/main/collector/src/nytp_fork.c), batch/v5/v6/dual fork hooks  
**Depends on:** COL-002 lifecycle, COL-005 batching, COL-014 dual (test/dev)

---

## Intent

1. **No lost/dup events** across parent/child when sinks are buffered (COL-005 batch).  
2. **Seq domains** — parent keeps COL-003 continuity; child resets to 0 and emits a new `PID_START`.  
3. **Path ownership** — addpid-style child paths; detach/rebind; no shared-path truncate race.  
4. **Fail-closed preflush** — hard flush errors prevent entering `FORK_SPLIT`.  
5. **Honest residuals** — not full live Perl/XS hooks, not complete TEST-018 oracle forkdepth/addpid/merge corpus, not mid-deflate compressor inheritance into the child.

## Protocol surface

| Symbol | Role |
|--------|------|
| `nytp_fork_policy` | flush / require-empty / discard-residual / fail-if-residual |
| `nytp_fork_policy_default` | flush=1, discard_child=1, require_empty=0, fail_if_residual=0 |
| `nytp_fork_metrics` | prepare/preflush/parent/child/discard/rebind counters |
| `nytp_fork_prepare` | optional `nytp_sink_flush` + `begin_fork` → `FORK_SPLIT` |
| `nytp_fork_resume_parent` | `end_fork_parent` → `ACTIVE`, seq continues |
| `nytp_fork_resume_child` | discard residual batch + `end_fork_child` → `OPEN`, seq=0 |
| `nytp_fork_addpid_path` | `"<base>.<pid>"` formatter |
| `nytp_batch_discard_pending` / `pending` | residual ownership helpers |
| `nytp_v5_sink_fork_child_reinit` | detach/rebind path; abort zlib; rewrite header; clear stats |
| `nytp_v6_sink_fork_child_reinit` | rebind path; clear body/dict/packing; rewrite file prefix |
| `nytp_dual_sink_fork_child_reinit` | reinit v5/v6 children after child resume |

### Recommended call order

```text
ACTIVE
  → nytp_fork_prepare(root, &pol, &m)     # flush + FORK_SPLIT
  → OS fork()  (optional; tests also simulate)
  parent:
    → nytp_fork_resume_parent(root, &m)   # ACTIVE, seq continues
    → continue emits / finalize / close (keeps base path)
  child:
    → nytp_fork_resume_child(root, &pol, &m)  # OPEN, seq=0; discard residual
    → nytp_*_sink_fork_child_reinit(..., addpid_path)  # clean wire stream
    → activate → PID_START(child_pid, parent_pid, ...) → emits → close
```

### Batch hardening (even without protocol wrapper)

| Hook | Behavior |
|------|----------|
| `notify_begin_fork` | **Preflush** pending events to child before `FORK_SPLIT` (`fork_preflush++`) |
| `notify_end_fork_child` | **Discard** residual pending + arena (`fork_child_discard+=n`) |

### Wire child re-init policy

| Concern | Parent | Child |
|---------|--------|-------|
| COL-003 seq | continues | resets to 0 |
| Output path | keeps base | addpid `base.pid` (or detach) |
| zlib/zstd compressor | may continue | **not** inherited — child starts clean stream |
| FOOTER string dict (v6) | kept | cleared / new domain |
| Shared FD / double-write | sole owner of base path | must not write base path |

## Acceptance for this MVP

| Check | Evidence |
|-------|----------|
| Compiles + unit tests | `make -C collector test` → `test_fork_pid` |
| Near-full batch preflush | pending drained before `FORK_SPLIT`; no dups after child resume |
| Seq domains | parent peek continues; child first logical seq = 0 + `PID_START` |
| Nested forkdepth sim | 3-level parent/child alternate resume |
| Dual+batch | logical equality after preflush and child path |
| Fail-closed preflush | child `fail_next` → prepare ≠ OK; not stuck in `FORK_SPLIT` |
| addpid + reinit | v5/v6 clean header/prefix; child file ≠ parent path |
| POSIX fork | parent + child both exit 0; separate files; no shared-path write |
| ASAN/UBSAN | sanitizer build of collector tests |
| Smoke | `scripts/packaging/collector_sink_smoke.sh` includes fork suite |

## Residuals / non-claims

| Residual | Owner |
|----------|-------|
| Full TEST-018 oracle forkdepth/addpid/merge fixture corpus | TEST-018 |
| Live Perl/XS opcode hooks + signal-safe finalize | later COL / product collector |
| Mid-deflate compressor state **continued** in child matching 6.15 oracle | measure + ADR if product requires |
| Product option wiring for `forkdepth` / `addpid` | PERL-001 / product surface |
| File-switch / enable-disable mid-run (OI-003-05) | residual |
| Complete multi-OS CI fork stress | BUILD-006 |

## Tests

- `collector/t/test_fork_pid.c` — protocol, batch preflush/discard, nested depth, dual+batch, preflush fail-closed, v5/v6 reinit, POSIX fork addpid, dual wire reinit.

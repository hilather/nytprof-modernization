# Collector dual-sink (COL-014) — test/dev-only MVP v0

**Status:** test/dev-only scaffolding for E4/M6 same-run evidence (OQ-4)  
**Task:** COL-014 (PR-B10a)  
**Layout:** [ADR-0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md) B0-A `collector/`  
**Sources:** [`collector/include/nytp_sink_dual.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytp_sink_dual.h), [`collector/src/nytp_sink_dual.c`](https://github.com/hilather/nytprof-modernization/blob/main/collector/src/nytp_sink_dual.c)  
**Policy:** dual-sink is **not** product UX — product remains single-format (`format=v5` default; `format=v6` opt-in)

---

## Intent

Fan out each COMPAT-001 semantic emit to **two** child sinks (typically **v5 + v6**) in the **same run**, so collector CI can prove **logical equality** (multiplicities + COL-003 seq/kind rings) for E4/M6 evidence.

## OQ-4 (resolved)

| Item | Decision |
|------|----------|
| Dual-sink product UX? | **No** — test/dev-only |
| Operator `format=dual`? | **Not advertised** — optional test/dev env alias only |
| Purpose | E4/M6 same-run v5↔v6 logical equality evidence |

## Surface (v0)

| Symbol | Role |
|--------|------|
| `nytp_dual_sink_create(primary, secondary, owns_*)` | Fan-out wrapper over existing children |
| `nytp_dual_sink_create_v5_v6(path_v5, path_v6)` | Test/dev convenience: owned v5 + absolute v6 children |
| `nytp_dual_sink_is_dual` / `primary` / `secondary` | Identity + child access |
| `nytp_dual_sink_logical_equal` | Compare counting-compatible child stats (by_kind + seq/kind rings) |
| `nytp_dual_sink_meta` / `write_compare_meta` | Out-of-band comparison metadata (JSON sidecar; not on either wire) |
| `nytp_dual_env_enabled` | Test/dev probe: `NYTPROF_DUAL_SINK=1\|true\|yes\|on` or `NYTPROF_FORMAT=dual` |

### Fan-out rules

| Rule | Detail |
|------|--------|
| Order | Primary then secondary (deterministic finalization order) |
| Seq | Dual parent owns COL-003; children aligned via `on_logical_committed` (no double public-wrapper assign) |
| Lifecycle | activate/stop/finalize/fork/close forwarded to both children |
| Failure | Primary fail → secondary not called; secondary fail after primary OK → **fail-closed sticky for all secondary error codes** (IO/FAILED/OVERFLOW returned as-is; STATE/UNSUPPORTED/… mapped to `NYTP_ERR_FAILED` so `emit_commit` sticky-fails dual). Partial dual residual: no primary rollback (COL-018) |
| Control | `START_DEFLATE` fans out (no COL-003 seq) |

### Env flags (test/dev only)

```text
NYTPROF_DUAL_SINK=1          # enable probe (truthy: 1/true/yes/on)
NYTPROF_FORMAT=dual          # test/dev alias only — not product format UX
```

Explicit `nytp_dual_sink_create*` is already opt-in; env probe is for harness gating only.

## Acceptance for this MVP

| Check | Evidence |
|-------|----------|
| Unit dual fan-out + logical equality | `collector/t/test_dual_sink.c` |
| M4 mini under dual (v5+v6 wires) | same |
| Primary-fixture-shaped streams (default-calls1 / blocks-calls1 / calls2-default multiplicity patterns) | same (synthetic scaled mini — not full oracle counts) |
| Env probe + compare-meta JSON | same |
| Smoke includes `test_dual_sink` | `scripts/packaging/collector_sink_smoke.sh` |

## Residuals / non-claims

| Residual | Owner |
|----------|-------|
| Full `fixtures/v5/` oracle stream dual equality | complete TEST-003 + TEST-008 M6 suite + live hooks |
| Product dual-write UX / advertised `format=dual` | **rejected** by OQ-4 |
| Full TEST-018 oracle fork under dual (beyond unit stress) | TEST-018 / COL-015 residual (MVP protocol in collector-fork-pid-mvp-v0) |
| Secondary-fail after primary wire write (no rollback of primary bytes/stats) | COL-018 residual; dual parent **is** sticky-failed for all secondary non-OK |
| E4-v0 model aggregates on dual-sink scaled pairs | **done** (PR-B10; `fixtures/e4/dual-sink/` + `e4_v0_*`) |
| Full oracle dual aggregate pairs + E4 product CLI smoke | TEST-008 / PR-B12b residual |
| Wire freeze / CLI v6 default | after E3/E4 |

## Tests

- `collector/t/test_dual_sink.c` — counting dual, M4 dual v5+v6, primary-fixture shapes, env probe, secondary fail sticky (IO + STATE/UNSUPPORTED→FAILED hard asserts), finalize order
- Wired into `make -C collector test` and `collector_sink_smoke.sh`

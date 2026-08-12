# Collector semantic sink API — MVP v0 (COL-001)

**Status:** provisional scaffolding (not a wire freeze)  
**Task:** COL-001 (PR-B02)  
**Layout:** [ADR-0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md) B0-A `collector/`  
**Sources:** [`collector/include/nytp_sink.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytp_sink.h), [`collector/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/collector/README.md)  
**Logical contract:** [COMPAT-001](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md)

---

## Intent

Define a **stream-neutral** C API so Perl hooks emit **semantic** events once; backends (v5 stub today; real v5, dual, counting, later v6) implement a vtable. Common statement path must not require general-heap allocation in backends that honor the hot-path contract.

## Surface (v0)

| Symbol | Role |
|--------|------|
| `nytp_sink` / `nytp_sink_ops` | Opaque sink + vtable |
| `nytp_emit_*` | Public emit wrappers (null/state/ops checks) |
| `nytp_sink_activate` / `flush` / `close` / `destroy` | Minimal lifecycle (COL-002 expands) |
| `nytp_counting_sink_create` | Test sink |
| `nytp_v5_sink_create` | Stub v5 adapter (counts only — **not** wire encode) |

### Tick / string domains

| Type | Meaning |
|------|---------|
| `nytp_ticks` (`int64_t`) | Logical ticks (v5 I32+overflow composition open OI-003-01) |
| `nytp_string_view` | `{ptr, len, is_utf8}` — no ownership; caller lifetime |

### Mapped emits ↔ COMPAT-001

See mapping table in [`collector/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/collector/README.md). Control: `nytp_emit_start_deflate` (not a logical profile event).

## Acceptance for this MVP

| Check | Evidence |
|-------|----------|
| Headers + stub compile with `cc` | `make -C collector` |
| Unit tests pass | `make -C collector test` / `test_sink_api` |
| Smoke honest-skips without CC | `scripts/packaging/collector_sink_smoke.sh` |
| Isolation: `collector/` never on oracle `PERL5LIB` | same smoke |
| Offline gate remains green without hard CC dep | `scripts/ci/offline_gate.sh` step collector-sink |

## Residuals / non-claims

| Residual | Owner |
|----------|-------|
| Real v5 wire bytes | COL-006 |
| C v6 writer | COL-007 |
| Full lifecycle freeze | COL-002 |
| Sequence numbers | COL-003 |
| Fake-clock | TEST-003 / PR-B03 |
| Live XS hook integration | later COL / packaging |
| Dual-sink overhead prototype | ARCH-007 |
| Wire freeze / format defaults | out of scope |

## Tests

- `collector/t/test_sink_api.c` — null guards, counting hot path, stub v5 routing, kind names, OPEN emit.

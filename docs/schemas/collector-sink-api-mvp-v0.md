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
| `nytp_sink_activate` / `stop` / `begin_finalize` / `flush` / `close` / `destroy` | Lifecycle (COL-002; see lifecycle schema) |
| Sequence fields + `nytp_sink_peek_seq` / `last_seq` | COL-003 internal logical seq |
| `nytp_counting_sink_create` | Test sink |
| `nytp_v5_sink_create` | v5 wire sink (COL-006 — real FileHandle.xs protocol; see collector-v5-wire-mvp-v0) |
| `nytp_fake_clock` / `nytp_m4_mini_sample_run` | TEST-003 scaffold (PR-B03) |

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
| Real v5 wire bytes | **COL-006 landed (PR-B05)** — see [`collector-v5-wire-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v5-wire-mvp-v0.md); full oracle corpus residual |
| C v6 writer | COL-007 |
| Full lifecycle freeze | **COL-002 landed (PR-B03 scaffold)** — see [`collector-lifecycle-seq-fake-clock-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-lifecycle-seq-fake-clock-mvp-v0.md); COL-015 residual for full fork/signal matrix |
| Sequence numbers | **COL-003 landed (PR-B03)** — internal gapless seq; not on default v5 wire |
| Fake-clock full M4 corpus | TEST-003 complete — **mini sample only** (PR-B03/B05); full `fixtures/v5/*` residual |
| Batch + stmt fast path | **COL-004/005 landed (PR-B04)** — see [`collector-batch-fast-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-batch-fast-mvp-v0.md) |
| Live XS hook integration | later COL / packaging |
| Dual-sink overhead prototype | ARCH-007 |
| Wire freeze / format defaults | out of scope |

## Tests

- `collector/t/test_sink_api.c` — null guards, counting hot path, v5 routing, kind names, OPEN emit.
- `collector/t/test_lifecycle_seq.c` — COL-002 transitions + COL-003 gapless seq.
- `collector/t/test_fake_clock.c` — TEST-003 fake-clock + M4 mini sample (counting + v5 wire).
- `collector/t/test_batch_fast.c` — COL-004/005 batch + fast path + SV lifetime.
- `collector/t/test_v5_wire.c` — COL-006 real wire encode/decode + zlib M4 mini.

# Release notes — R2-preview (v6 opt-in) packaging cut (PR-B13)

**Date:** 2026-08-12  
**PLAN_ID:** `8c9b1a63`  
**Board ID:** `R2-PREVIEW-READINESS-CUT`  
**Horizon:** charter **R2-preview** (v6 collection/report **opt-in** only — **not** R2-stable, **not** R3/R4)  
**COL-009:** [ADR-0007 — production v6 writer backend: C baseline reaffirm](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0007-production-v6-writer-backend-c-baseline.md)  
**Dual-equality readiness:** [DUAL_EQUALITY_READINESS_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md)  
**Residual matrix:** [R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) (§ R2-preview)  
**Operator runbook:** [R1_PREVIEW_OPERATOR_RUNBOOK.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) (§ R2-preview honesty)

These notes freeze the **advertised R2-preview product scope** after Track B (COL-007 through CLI E5 + E4 product smoke). They are **not** a CPAN upload statement, performance certification, R2-stable claim, or permission to flip collection/engine defaults.

---

## Summary

| Theme | What ships under R2-preview | Honesty |
|-------|----------------------------|---------|
| **Collection default** | **v5** remains product default (`collection_default: v5`) | **No R4** format default flip |
| **v6 offline tools (E5)** | `dump` / `verify` / `report` / `html` / `csv` / `folded` / `callgrind` on v6 via magic auto-detect | **Opt-in read path** — not “v6 is default” |
| **Capability** | `v6_decode` / `v6_report` **true**; `convert` / `merge` **false** | Convert/merge residual (Phase C / PR-C01+ on other tracks) |
| **COL-007 C writer** | Product E3-EVENT with C (**done at PR-B09**, not at this packaging PR) | E3-mixed residual; live XS hooks residual |
| **COL-008** | — | **Deferred** non-baseline (ADR-0007) |
| **COL-009** | C backend **reaffirmed** (ADR-0007) | No COL-008 bake-off claim |
| **Wire freeze** | ADR-0006 major=6 IDs + golden vectors (**done PR-B11**) | Format IDs frozen; not “full dual-equality product freeze” |
| **E4** | E4-v0 model + E4 product CLI smoke (dual-sink scaled pairs; offline_gate step 12) | Full oracle dual residual (TEST-008) |
| **COL-014** | Dual-sink **test/dev-only** (OQ-4) | **Not** product UX / `format=dual` |
| **Dual-path legacy** | Unchanged: legacy-only without Cargo; oracle never uses `crates/` on `PERL5LIB` | v5 6.15 path remains |
| **Convert / merge / salvage** | — | **R2-preview residual** — do **not** advertise |
| **R2-stable / R3 / R4** | — | **Not claimed** |

---

## CLI / report (v6 opt-in)

- Native offline surfaces accept **v5 and v6** profiles by magic (`NYTProf 5…` vs `NYTPROF6`).
- Full product surfaces on v6 EVENT profiles (PR-B12): report/html/csv/folded/callgrind/dump/verify.
- Capability self-test honesty markers (stable):

```text
v6_decode: yes
v6_report: yes
convert: no
merge: no
collection_default: v5
```

- Schema: [`docs/schemas/cli-e5-v6-opt-in-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cli-e5-v6-opt-in-mvp-v0.md).
- E4 product smoke: [`docs/schemas/e4-product-cli-smoke-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-product-cli-smoke-mvp-v0.md); `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --full`.

---

## Collector / format

| Item | Status under R2-preview |
|------|-------------------------|
| COL-007 C v6 writer (EVENT) | **done** (PR-B09) — board flipped at E3-EVENT C green, **not** at packaging |
| Absolute + codecs/multi-chunk/CRC + packing + FOOTER dict + mid-stream | Product path for EVENT |
| Wire freeze ADR-0006 | **done** (PR-B11) |
| COL-009 / ADR-0007 | **C baseline reaffirmed**; COL-008 deferred |
| COL-014 dual-sink | test/dev harness only |
| COL-008 batched Rust writer | **deferred** |
| COL-015 fork/PID harden | **R2-stable residual** |
| E3-mixed (SOURCE/INDEX/SUMMARY C matrix) | **residual** |
| Live Perl/XS collection hooks shipping v6 by default | **not** claimed (overlay + harness path) |

---

## Dual-equality (E1–E5)

| Class | R2-preview status |
|-------|-------------------|
| E1 v5 surfaces | ready (R0/R1-preview stack) |
| E2 encode↔decode | ready + golden vectors |
| E3 C writer ↔ Rust decode | **ready (EVENT)**; E3-mixed residual |
| E4 v5↔v6 semantic | E4-v0 + E4 product CLI **ready** on dual-sink scaled pairs; full oracle residual |
| E5 CLI product path | **ready** opt-in; collection default remains v5 |

Authoritative checklist: [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md).

---

## Explicit non-claims (binding)

Do **not** advertise under this cut:

1. **R2-stable** (convert/merge/salvage, COL-015 fork suite, security/fuzz package, P1/P2 cert, full platform matrix as advertised for stable).
2. **Convert / merge / salvage** tooling or capability bits (`convert`/`merge` must stay **false** on this branch until those tools land and capability is updated with evidence).
3. **CLI v6 collection default** / R4 format default flip.
4. **R3** `engine=auto` product default flip.
5. **COL-008** batched Rust writer as baseline or measured superior backend.
6. **E3-mixed** multi-kind product C fixture matrix complete.
7. **Full oracle E4** dual pairs (TEST-003/TEST-008).
8. **Public performance SLOs** or certified BENCH package.
9. **CPAN upload** readiness.
10. That **this packaging PR** completed COL-007 — COL-007 was already **done** at PR-B09.

---

## Upgrade / operator notes

| Audience | Guidance |
|----------|----------|
| Preview operators | Keep using v5 profiles by default. Offline tools accept v6 when present. `./scripts/ci/offline_gate.sh` remains the primary gate. Never put `crates/` on oracle `PERL5LIB`. |
| Embedders / collectors | Production v6 writer backend is **C** (ADR-0007). Dual-path legacy without C/Cargo still required. |
| Tooling authors | Do not assume convert/merge until capability reports them true with green tools. Prefer model-level equality over re-encoding guesses. |
| Release engineers | This is an **opt-in R2-preview** honesty cut. R2-stable is a separate cut (Phase C / PR-C05 class). |

---

## Evidence map

| Item | Path |
|------|------|
| This cut board row | `R2-PREVIEW-READINESS-CUT` in [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md) |
| COL-009 ADR | [`docs/adrs/0007-production-v6-writer-backend-c-baseline.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0007-production-v6-writer-backend-c-baseline.md) |
| Dual-equality | [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md) |
| Residual matrix | [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) |
| Runbook | [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) |
| Wire freeze | [`docs/adrs/0006-v6-wire-freeze.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md) |
| Offline gate | [`scripts/ci/offline_gate.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/offline_gate.sh) (steps 10–12 collector / E3 / E4) |

---

## Track B PR index (path to this cut)

| PR | Role | Status in this cut |
|----|------|--------------------|
| PR-B00 | Collector packaging ADR-0004 | done |
| PR-B01 | ADR-0001/0002 accept + ID lockfile | done |
| PR-B02..B05 | Sink + lifecycle + batch + v5 wire | done (scaffold / wire) |
| PR-B06..B08 | COL-007 absolute / codecs / packing | done (scaffold → product stack) |
| PR-B08.5 | E3 stand-in harness extensions | done (engineering only) |
| PR-B09 | E3-EVENT with C + **board COL-007 done** | done |
| PR-B10a | COL-014 dual-sink test/dev | done |
| PR-B10 | E4-v0 model semantic | done |
| PR-B11 | Wire freeze ADR-0006 + golden vectors | done |
| PR-B11a | Product v6→ProfileModel ingest | done |
| PR-B12 | CLI E5 v6 opt-in + capability honesty | done |
| PR-B12b | E4 product smoke in offline_gate | done |
| **PR-B13** | **This R2-preview packaging + honesty + COL-009** | **done** |

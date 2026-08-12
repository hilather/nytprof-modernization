# Architecture Decision Records

Process: [`docs/governance/ARCH-008_ADR_PROCESS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/ARCH-008_ADR_PROCESS.md)  
Template: [`docs/plan/templates/ADR_TEMPLATE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/templates/ADR_TEMPLATE.md)  
Queue: [`docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md)

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md) | Format v6 event-body packing candidate design | **accepted** (OQ-1 as-is; packing intent; not COL-007 alone) |
| [0002](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md) | Format v6 FOOTER string-pool / dictionary candidate | **accepted** (OQ-1 as-is; FOOTER-local; not global pool) |
| [0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md) | Collector packaging / source-tree layout (B0-A overlay) | **accepted** — required before COL-001 / PR-B02 merge |
| [0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md) | Format v6 wire freeze (numeric IDs + core layouts) | **accepted** — after E3-EVENT(C) + E4-v0; golden vectors |

**Numbering map (coordinate across parallel PRs):**

| Number | Topic | Track / PR |
|--------|-------|------------|
| **0001** | Format v6 event-body packing candidate | R2 format — PR-B01 |
| **0002** | Format v6 FOOTER string-pool / dictionary candidate | R2 format — PR-B01 |
| **0003** | Full R1 residual policy (CLOSE / WAIVE / OUT-OF-R1) | Track A — PR-A04 (may land as `0003-r1-full-residual-policy.md` on that branch) |
| **0004** | Collector packaging / source-tree (B0-A) | Track B — PR-B00 |
| **0005** | R3 `engine=auto` default promotion | Track D — PR-D02 (when present) |
| **0006** | Format v6 wire freeze (IDs + golden vectors) | Track B — PR-B11 |

### Related (not ADRs)

| Doc | Role |
|-----|------|
| [`V6_PROVISIONAL_ID_LOCKFILE_v0`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md) | ID lockfile (path historical; **status frozen** by ADR-0006) |
| [`v6-wire-ids-frozen-v1`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md) | Authoritative frozen major=6 ID catalog |
| [`DUAL_EQUALITY_READINESS_v0`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md) | Dual-equality readiness checklist (E1–E5) |
| C header | [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h) — mirrors frozen constants |
| Golden vectors | [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/) |

Governance ratifications (not format ADRs):

- [`COMPAT-000`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/COMPAT-000_RATIFICATION.md) — compatibility contract binding
- [`ARCH-008`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/ARCH-008_ADR_PROCESS.md) — ADR process established

When further ADR-Q items are decided, add files here as `NNNN-short-title.md`.

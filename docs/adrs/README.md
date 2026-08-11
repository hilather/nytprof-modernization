# Architecture Decision Records

Process: [`docs/governance/ARCH-008_ADR_PROCESS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/ARCH-008_ADR_PROCESS.md)  
Template: [`docs/plan/templates/ADR_TEMPLATE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/templates/ADR_TEMPLATE.md)  
Queue: [`docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md)

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md) | Format v6 event-body packing candidate design | **accepted** (OQ-1 as-is; intent freeze — not wire freeze; not COL-007) |
| [0002](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md) | Format v6 FOOTER string-pool / dictionary candidate | **accepted** (OQ-1 as-is; FOOTER-local; not global pool; not COL-007) |

### Related (not ADRs)

| Doc | Role |
|-----|------|
| [`V6_PROVISIONAL_ID_LOCKFILE_v0`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md) | Shared provisional numeric IDs for COL-007 (not wire freeze); plan FMT-002..010 deviation |
| [`DUAL_EQUALITY_READINESS_v0`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md) | Dual-equality readiness checklist (E1–E5) |
| C header stub | [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h) — mirrors lockfile constants |

Governance ratifications (not format ADRs):

- [`COMPAT-000`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/COMPAT-000_RATIFICATION.md) — compatibility contract binding
- [`ARCH-008`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/ARCH-008_ADR_PROCESS.md) — ADR process established

When further ADR-Q items are decided, add files here as `NNNN-short-title.md`.

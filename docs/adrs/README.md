# Architecture Decision Records

Process: [`docs/governance/ARCH-008_ADR_PROCESS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/ARCH-008_ADR_PROCESS.md)  
Template: [`docs/plan/templates/ADR_TEMPLATE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/templates/ADR_TEMPLATE.md)  
Queue: [`docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md)

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md) | Collector packaging / source-tree layout (B0-A overlay) | **accepted** — required before COL-001 / PR-B02 merge; not COL-007 product |

**Numbering map (coordinate across parallel PRs):**

| Number | Topic | Track / PR |
|--------|-------|------------|
| **0001** | Format v6 event-body packing candidate | R2 format — PR-B01 |
| **0002** | Format v6 FOOTER string-pool / dictionary candidate | R2 format — PR-B01 |
| **0003** | Full R1 residual policy (CLOSE / WAIVE / OUT-OF-R1) | Track A — PR-A04 |
| **0004** | Collector packaging / source-tree (B0-A) | Track B — PR-B00 (this index row) |

**Merge handoff:** rebase or merge-resolve this file with PR-B01 and PR-A04 so **all** rows remain listed (do not replace a multi-row table with a single-row index). B01 collector stubs must match B0-A `collector/` layout from ADR-0004.

Governance ratifications (not format ADRs):

- [`COMPAT-000`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/COMPAT-000_RATIFICATION.md) — compatibility contract binding
- [`ARCH-008`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/ARCH-008_ADR_PROCESS.md) — ADR process established

When further ADR-Q items are decided, add files here as `NNNN-short-title.md`.

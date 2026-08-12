# Architecture Decision Records

Process: [`docs/governance/ARCH-008_ADR_PROCESS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/ARCH-008_ADR_PROCESS.md)  
Template: [`docs/plan/templates/ADR_TEMPLATE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/templates/ADR_TEMPLATE.md)  
Queue: [`docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md)

## Index

| ADR | Title | Status |
|-----|-------|--------|
| 0001 | *(reserved)* Format v6 event-body packing candidate | reserved for R2 runway (land/accept via packing work + **PR-B01** / OQ-1) |
| 0002 | *(reserved)* Format v6 FOOTER string-pool / dictionary candidate | reserved for R2 runway (land/accept via packing work + **PR-B01** / OQ-1) |
| [0003](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) | Full R1 residual close-or-waive policy (HTML map + OQ-2) | **accepted** (**PR-A04**) |
| 0004 | *(reserved)* Collector packaging / source-tree layout | reserved for **PR-B00** (`0004-collector-packaging-source-tree.md` when that PR lands; do **not** reuse 0003) |
| [0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) | R3 `engine=auto` product default promotion (gated) | **accepted (policy)**; **flip not executed** (**PR-D02** / ADR-Q024) |

**Numbering coordination (PLAN `8c9b1a63`):** 0001–0002 = format packing track (B01); **0003 = residual policy (A04)**; **0004 = collector packaging (B00)**; **0005 = R3 default promotion (D02)**. Later ADRs start at **0006+**. Do not steal 0001–0005. R4 format default is a separate ADR (not 0005).

Governance ratifications (not format ADRs):

- [`COMPAT-000`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/COMPAT-000_RATIFICATION.md) — compatibility contract binding
- [`ARCH-008`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/ARCH-008_ADR_PROCESS.md) — ADR process established

When further ADR-Q items are decided, add files here as `NNNN-short-title.md`.

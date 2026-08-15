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
| [0007](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0007-production-v6-writer-backend-c-baseline.md) | Production v6 writer backend: reaffirm C baseline (COL-009) | **accepted** — R2-preview cut PR-B13; COL-008 remains deferred |
| [0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) | R4 `format=v6` product collection/output default promotion (gated) | **accepted (policy)**; **flip not executed** (**PR-E02** / ADR-Q025) |
| [0009](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0009-r5-legacy-retirement-governance.md) | R5 legacy retirement governance (per-component; never automatic) | **accepted (policy)**; **no component retired** (**PR-F01** / ADR-Q026) |
| [0010](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0010-signed-ci-prebuilt-native-cli.md) | Signed CI prebuilt `nytprof-cli` for EL8 tools RPM (KD-13) | **accepted (policy)**; pipeline / K02 **not implemented** (**PR-K03** / ADR-Q016 EL8 slice) |
| [0011](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0011-native-operator-html-v1.md) | Native operator HTML v1 (heat / links / vanilla sort) | **accepted (policy)**; does **not** un-waive M01 jquery/tablesorter; implementation PR-2..PR-6 |
| [0012](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0012-native-operator-html-v2.md) | Native operator HTML v2 (oracle look/feel/nav; modern JS/CSS) | **accepted (policy)**; does **not** un-waive M01; design [`OPERATOR_HTML_V2_DESIGN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/OPERATOR_HTML_V2_DESIGN_v0.md) |
| [0013](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0013-v5-coalesced-checkpoints.md) | In-memory v5 coalesced checkpoints (`aggregate=1`) | **proposed** (charter / plan-01 exception; ADR-Q027; **not accepted**; no C1/C2 until owner sign-off) |

**Numbering map (coordinate across parallel PRs):**

| Number | Topic | Track / PR |
|--------|-------|------------|
| **0001** | Format v6 event-body packing candidate | R2 format — PR-B01 |
| **0002** | Format v6 FOOTER string-pool / dictionary candidate | R2 format — PR-B01 |
| **0003** | Full R1 residual policy (CLOSE / WAIVE / OUT-OF-R1) | Track A — PR-A04 (may land as `0003-r1-full-residual-policy.md` on that branch) |
| **0004** | Collector packaging / source-tree (B0-A) | Track B — PR-B00 |
| **0005** | R3 `engine=auto` default promotion | Track D — PR-D02 (when present) |
| **0006** | Format v6 wire freeze (IDs + golden vectors) | Track B — PR-B11 |
| **0007** | Production v6 writer backend (C baseline / COL-009) | Track B — PR-B13 |
| **0008** | R4 `format=v6` collection/output default promotion | Track E — PR-E02 |
| **0009** | R5 legacy retirement governance (umbrella) | Track F — PR-F01 |
| **0010** | Signed CI prebuilt `nytprof-cli` (EL8 tools) | Product-completion K-track — **PR-K03** (hard-gates K02) |
| **0011** | Native operator HTML v1 | Live-metrics / HTML program — **PR-0** |
| **0012** | Native operator HTML v2 | HTML-OP-V2 chrome / IA / dual-docker lab |
| **0013** | In-memory v5 coalesced checkpoints | Profile size item 3 — **PR-A13** (`proposed` only) |

### Related (not ADRs)

| Doc | Role |
|-----|------|
| [`V6_PROVISIONAL_ID_LOCKFILE_v0`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md) | ID lockfile (path historical; **status frozen** by ADR-0006) |
| [`v6-wire-ids-frozen-v1`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md) | Authoritative frozen major=6 ID catalog |
| [`DUAL_EQUALITY_READINESS_v0`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md) | Dual-equality readiness checklist (E1–E5) |
| C header | [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h) — mirrors frozen constants |
| Golden vectors | [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/) |
| 0001 | *(reserved)* Format v6 event-body packing candidate | reserved for R2 runway (land/accept via packing work + **PR-B01** / OQ-1) |
| 0002 | *(reserved)* Format v6 FOOTER string-pool / dictionary candidate | reserved for R2 runway (land/accept via packing work + **PR-B01** / OQ-1) |
| [0003](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) | Full R1 residual close-or-waive policy (HTML map + OQ-2) | **accepted** (**PR-A04**) |
| 0004 | *(reserved)* Collector packaging / source-tree layout | reserved for **PR-B00** (`0004-collector-packaging-source-tree.md` when that PR lands; do **not** reuse 0003) |
| [0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) | R3 `engine=auto` product default promotion (gated) | **accepted (policy)**; **flip not executed** (**PR-D02** / ADR-Q024) |

**Numbering coordination (PLAN `8c9b1a63`):** 0001–0002 = format packing track (B01); **0003 = residual policy (A04)**; **0004 = collector packaging (B00)**; **0005 = R3 default promotion (D02)**; **0006–0007 = wire freeze / COL-009**; **0008 = R4 default (E02)**; **0009 = R5 retirement governance (F01)**. Do not steal 0001–0009. Component-specific R5 retirements are later ADRs under 0009.

Governance ratifications (not format ADRs):

- [`COMPAT-000`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/COMPAT-000_RATIFICATION.md) — compatibility contract binding
- [`ARCH-008`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/ARCH-008_ADR_PROCESS.md) — ADR process established

When further ADR-Q items are decided, add files here as `NNNN-short-title.md`.

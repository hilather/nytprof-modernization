# ADR-0005 — R3 `engine=auto` product default promotion (gated)

- **Status:** **accepted (policy)** — product flip state: **not executed**
- **Date:** 2026-08-11
- **Owners/approvers:** release review group (REL-006); program completion plan (PLAN_ID `8c9b1a63`)
- **Related ADR-Q:** [ADR-Q024](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md) (native report default promotion criteria / field window)
- **Related tasks/risks/gates:** REL-005, REL-006, REL-009, COMPAT-014, TEST-020, BENCH-013 (engineering only; not public SLO), charter **R3**, Phase D **PR-D01** / **PR-D02**
- **Decision scope/version:** product **default engine selection** for dual-path reporting surfaces only (not R4 format default; not COL-007 / v6 wire freeze; not legacy retirement)

---

## Context

Charter level **R3** ([`docs/PROGRAM_CHARTER.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md)):

> `engine=auto` prefers native reports — **Yes, only after field window + ADR**

Today (full R1 MVP / post-A10):

| Surface | When `--engine` / `NYTPROF_ENGINE` omitted | `auto` behavior |
|---------|--------------------------------------------|-----------------|
| Perl facade (`nytprof-engine`) | **`native`** (fail if native CLI missing) | Prefer native; fall back to legacy + STDERR note (**ENGINE-AUTO-FALLBACK**) |
| Pure-Rust `nytprof-cli` | **`native`** | Maps `auto` → `native` (no in-process legacy residual) |

Facade prefer-native/fallback under **explicit** `engine=auto` is already shipped and smoke-tested. That is **not** the R3 product default flip. R3 means the **product default** (omitted flag/env on dual-path surfaces) becomes **`auto`** so operators get prefer-native with honest legacy fallback **without** opting in.

Promotion without field evidence risks silent correctness regressions, support spikes, and hard-to-roll-back ecosystem defaults. Plan tasks [REL-006](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) and acceptance [Level R3](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md) require a field window, default-change ADR, force-legacy escape, and rollback.

**Residual honesty for this PR (PR-D02):** no accepted multi-site field report exists in-tree that recommends **promote**. Therefore this ADR **binds policy and procedure only**. It does **not** change runtime defaults, packaging defaults, or offline_gate expectations.

Numbering coordination (PLAN `8c9b1a63`):

| Number | Owner | Topic |
|--------|-------|--------|
| 0001 / 0002 | PR-B01 | v6 packing / FOOTER string-pool candidates |
| 0003 | PR-A04 | Full R1 residual close-or-waive policy |
| 0004 | PR-B00 | Collector packaging / source-tree |
| **0005** | **PR-D02 (this ADR)** | R3 `engine=auto` product default promotion |

Do not reuse 0005 for R4 format default (separate ADR / ADR-Q025).

---

## Evidence

| Source | Role |
|--------|------|
| Field-window pack (PR-D01) | Local evidence collection — [`docs/R3_FIELD_WINDOW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md), [`scripts/field/r3_field_window_collect.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/r3_field_window_collect.sh), [`docs/templates/R3_FIELD_WINDOW_REPORT.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R3_FIELD_WINDOW_REPORT.md) |
| Packaging smokes | `engine_auto_smoke.sh`, `engine_auto_fallback_smoke.sh` — lab dual-path auto, not field window |
| Residual matrix | R3 row remains **not flipped** until flip execution checklist completes |
| Plan REL-005 / REL-006 / ADR-Q024 | Opt-in cycle → field window → default-change ADR → promote |
| This PR | **No** field packs claiming multi-site promote; **no** runtime flip |

**Gate evidence (required before flip execution, not claimed present by this ADR):**

1. One or more filled field-window reports with status **accepted** and recommendation **Promote**.
2. Coverage: ≥1 production-like site per **advertised eligible OS tier** listed at flip time.
3. No open **critical/high** native-path correctness, data-loss, or security issues attributable to the defaulted path.
4. Force-legacy escape verified on each eligible tier.
5. Fallback-when-native-missing documented; fallback must not hide corruption as complete success.
6. Release review + compatibility/QA sign-off on the report(s).
7. Offline gate green on the flip candidate tree: `./scripts/ci/offline_gate.sh`.

If any gate item is missing → **do not flip**. Extend the field window or record **Do not promote**.

---

## Decision

### 1. Binding product meaning of R3 (when flip is executed)

On **dual-path product surfaces** (primarily Perl `nytprof-engine` / EngineDispatch and any facade that documents product engine selection):

| Control | Pre-R3 (current / this ADR until flip) | Post-R3 (after flip execution only) |
|---------|----------------------------------------|-------------------------------------|
| Omitted `--engine` and unset `NYTPROF_ENGINE` | **`native`** (fail closed if native CLI missing) | **`auto`** (prefer native; fall back to legacy + STDERR note when native not discoverable) |
| Explicit `native` | Unchanged — fail if native missing | Unchanged |
| Explicit `legacy` | Unchanged — oracle path | Unchanged — **one-step operator rollback / escape hatch** |
| Explicit `auto` | Prefer-native / fall-back-legacy | Same semantics (now matches product default) |

Precedence remains: **CLI flag overrides env**; both override the product default.

### 2. Eligible operations and formats

When flipped, `auto` (including as product default) applies to the same actions that already honor engine selection under the engine-selection MVP:

- At minimum: `report`, `summary`, `html`, `csv`, `verify`, `inspect`, `query` (and other facade passthroughs documented in [`engine-selection-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/engine-selection-mvp-v0.md) / [`perl-engine-dispatch-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-engine-dispatch-mvp-v0.md)).
- Profile format scope for R3: **supported v5** (and later **supported v6 read** when product v6 read is advertised). R3 does **not** change collector **output** format default (that is charter **R4** / ADR-Q025).

### 3. Eligible tiers

Eligible OS/arch tiers for the product default flip are **only** those listed in the accepted field-window report(s) and reaffirmed in release notes at flip time. Tiers without accepted field evidence remain on the pre-R3 default policy (or document explicit non-eligibility). Multi-OS CI MVP (BUILD-006-MVP) is **not** by itself field-window evidence.

### 4. Fallback policy (binding)

| Situation | Required behavior |
|-----------|-------------------|
| Native CLI discoverable | `auto` uses native; semantic samples on default-calls1 remain leaf **15** / mid **3** when that fixture applies |
| Native CLI not discoverable | `auto` falls back to legacy with a clear **STDERR** note; must not claim native success |
| Explicit `native` + missing CLI | **Fail closed** — no silent legacy |
| Corrupt / incomplete profile | Fail closed per existing COMPAT-010 / incomplete-stream policy — **fallback must not hide corruption** |
| `NYTPROF_FORCE_NO_NATIVE=1` | Test/field hook only (same as ENGINE-AUTO-FALLBACK); not a product operator API |

### 5. Pure-Rust `nytprof-cli` residual

Pure-Rust `nytprof-cli` has **no in-process legacy path**. Mapping `auto` → `native` may remain. Charter R3 product dual-path default is owned by the **Perl facade** (and any future dual-path wrapper). Do not invent a fake legacy engine inside `nytprof-cli` as part of R3.

### 6. Flip is gated — not executed by accepting this ADR

| State token | Meaning |
|-------------|---------|
| **policy accepted** | This ADR is binding for criteria, procedure, rollback design |
| **flip not executed** | Runtime product default remains pre-R3 (`native` when omitted on facade) |
| **flip executed** | Only after gate evidence (§ Evidence) + flip checklist in [`docs/R3_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) + release notes + honesty doc sync |

**Acceptance of this ADR alone must never be cited as proof that R3 product defaults already changed.**

### 7. Rollback path (binding design)

| Layer | Mechanism |
|-------|-----------|
| **Operator one-step** | `--engine=legacy` or `NYTPROF_ENGINE=legacy` on dual-path surfaces (always retained after flip) |
| **Operator prefer fail-closed native** | `--engine=native` / `NYTPROF_ENGINE=native` |
| **Product default rollback** | Revert the flip change set (default string / resolve path) so omitted flag/env resolves to pre-R3 **`native`** again; ship as patch/release with release notes |
| **Monitoring triggers** | Open high/critical native-path correctness issue; elevated fallback frequency with unexplained failures; security issue on defaulted path; field incident process (REL-009) |
| **Rollback owner** | Named in the accepted field report and release notes at flip time |

Rollback of the **product default** does **not** remove `engine=auto` prefer-native capability, does **not** remove v5 read, and does **not** imply R5 legacy retirement.

---

## Exactness and compatibility consequences

| Area | Effect |
|------|--------|
| Event order / counts / ticks | Unchanged — same engines; only default selection changes when flipped |
| v5 read/write | Unchanged; R3 is report-time engine default, not collector format |
| Explicit engine flags | Remain supported; force-legacy is the user escape hatch |
| Offline R0 / R1-preview / full R1 MVP | **Unchanged** until flip execution — native remains opt-in as product default |
| R4 | Independent; format default never coupled to this ADR |
| Performance claims | Field wall-times / light_bench are **not** public SLOs; certified claims still require BENCH package |

---

## Alternatives considered

| Alternative | Correctness/compatibility | Performance/storage | Security/reliability | Build/portability | Reason accepted/rejected |
|-------------|---------------------------|---------------------|----------------------|-------------------|--------------------------|
| Flip product default in PR-D02 without accepted field report | Weak field confidence | Unchanged engines | High incident risk | Same | **Rejected** — residual honesty / charter R3 |
| Keep ADR proposed until field promote | Weaker procedural freeze | — | — | — | **Rejected** — policy needs to be binding now so operators collect against a known bar |
| Default to `legacy` forever | Safest compat | Misses native report gains | Low | — | **Rejected** as terminal policy; R3 exists to promote after evidence |
| Default pure-Rust CLI to dual-path legacy | Requires oracle in-process | — | Complex | Breaks pure-Rust packaging | **Rejected** — residual remains; facade owns dual-path auto |
| Couple R3 with R4 v6 default | Larger blast radius | — | Harder rollback | — | **Rejected** — charter separately promotable outcomes |

---

## Implementation and testing requirements

### A. This PR (PR-D02) — policy only

1. Land this ADR; index in [`docs/adrs/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/README.md).
2. Land flip procedure + rollback checklist: [`docs/R3_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md).
3. Sync residual matrix, board, runbook, engine-selection schema, field-window guide to: **ADR accepted; flip not executed**.
4. Update ADR-Q024 status to point at this ADR (criteria frozen; flip gated).
5. **Do not** change `EngineDispatch` / `nytprof-cli` omitted-engine default in this PR.
6. **Do not** set field pack `no_default_flip` false; collectors remain evidence-only.

### B. Flip execution (later change set; not this PR)

Follow [`docs/R3_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md). Minimum technical delta:

1. Perl facade: omitted flag/env → resolve as **`auto`** (not `native`).
2. Docs/help strings: default engine = `auto` on dual-path surfaces.
3. Regression: omitted-engine report on default-calls1 → leaf **15** / mid **3** when native discoverable; force-no-native auto still falls back with STDERR note; explicit native still fail-closed under force-no-native.
4. Honesty: residual matrix R3 row → flipped for eligible tiers; release notes; runbook.
5. Smoke/gate: extend packaging smokes if needed so **default** (no flag) is covered, not only explicit `--engine=auto`.
6. Never put `crates/` on oracle `PERL5LIB`.

### C. Tests for any future flip PR

| Case | Expect |
|------|--------|
| No flag, native present | Prefer native; leaf **15** / mid **3** on default-calls1 |
| No flag, native absent | Legacy fallback + STDERR note (not false native) |
| `--engine=legacy` | Legacy path |
| `--engine=native` + no CLI | Fail closed |
| Invalid engine | Fail closed |

---

## Migration, rollout, and rollback

| Topic | Policy |
|-------|--------|
| Pre-flip operators | No behavior change from this ADR alone |
| Opt-in today | Use `--engine=auto` / `NYTPROF_ENGINE=auto` (already shipped) |
| Flip rollout | Single release notes entry; eligible tiers listed; link accepted field report ID(s) |
| Compatibility window | Force-legacy retained for the full compatibility window; no silent removal |
| Rollback triggers | High/critical native correctness or security; systemic fallback failure; release-owner call |
| Files already produced | Unaffected (report-time selection only) |

Detailed steps: [`docs/R3_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md).

---

## Revisit triggers

- Accepted field report recommends **extend window** or **do not promote** → keep flip not executed; update report archive; do not weaken gates.
- New high-severity native-path issue after flip → execute product default rollback; keep explicit `auto` available.
- Multi-OS / tier expansion → require additional accepted field evidence for new tiers before advertising them as R3-eligible defaults.
- Desire to change pure-Rust `auto`→`native` residual → separate ADR (dual-path inside CLI is out of scope here).
- R4 format default → **ADR-Q025** / separate ADR only; never piggyback on this ADR.
- Superseding evidence that product default should remain `native` indefinitely → superseding ADR; this ADR’s flip checklist stays unused.

---

## Normative doc pointers

| Doc | Role |
|-----|------|
| [`docs/R3_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) | Flip execution + rollback checklist |
| [`docs/R3_FIELD_WINDOW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md) | Field evidence collection (PR-D01) |
| [`docs/templates/R3_FIELD_WINDOW_REPORT.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R3_FIELD_WINDOW_REPORT.md) | Human multi-site report |
| [`docs/schemas/engine-selection-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/engine-selection-mvp-v0.md) | Engine names / precedence |
| [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) | R3 residual honesty row |
| [`docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md) | Level R3 criteria |
| [`docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) | REL-005 / REL-006 |

---

## Board / plan placement

| ID | Status | Evidence |
|----|--------|----------|
| `R3-DEFAULT-CHANGE-ADR` (PR-D02) | **done (policy; flip not executed)** | this ADR + `docs/R3_DEFAULT_FLIP.md` |
| `R3-FIELD-WINDOW-PACK` (PR-D01) | **done** (instrumentation) | field pack tools |
| Charter R3 product default | **not complete** until flip execution checklist | residual matrix |
| ADR-Q024 | criteria answered by this ADR; flip still gated | plan queue update |

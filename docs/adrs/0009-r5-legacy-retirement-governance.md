# ADR-0009 — R5 legacy retirement governance (per-component; never automatic)

- **Status:** **accepted (policy)** — component retirement state: **none executed**
- **Date:** 2026-08-12
- **Owners/approvers:** ecosystem / release review group (REL-012); program completion plan (PLAN_ID `8c9b1a63`)
- **Related ADR-Q:** [ADR-Q026](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md) (legacy code retirement policy)
- **Related tasks/risks/gates:** REL-012, REL-013, charter **R5**, Phase F **PR-F01**, plan Phase 7
- **Decision scope/version:** **governance** for whether/when any legacy component may be deprecated or removed (not R3 engine default; not R4 format default; not a removal of any component by itself)

---

## Context

Charter level **R5** ([`docs/PROGRAM_CHARTER.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md)):

> Legacy retirement consideration — **Separate ADRs only; never automatic**

Acceptance Level **R5** ([`docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md)):

> Not implied by R4. Each legacy component requires separate deprecation/removal decision, usage evidence, support window, and migration path.

Modernization success (R1–R4 path, dual-path install, v6 opt-in, field-gated defaults) **must not** be misread as authorization to delete legacy report engines, v5 readers/writers, old Perl tiers, or oracle-shaped CLIs. Plan task [REL-012](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) and Phase 7 require **component-independent** review.

Risks of silent or bulk retirement:

- ecosystem breakage (CPAN consumers, old CI images, air-gapped tools that only read v5);
- loss of archival readability for profiles already on disk;
- coupling default flips (R3/R4) with path removal, making rollback impossible;
- security/maintenance pressure used as a substitute for migration coverage.

**Residual honesty for this PR (PR-F01):** no sustained multi-release R4 field record, no ecosystem usage study, and **no component deprecation or removal** lands in this change set. This ADR **binds retirement governance only**. **Absence of any retirement is a valid program outcome.**

Numbering coordination (PLAN `8c9b1a63`):

| Number | Owner | Topic |
|--------|-------|--------|
| 0001 / 0002 | PR-B01 | v6 packing / FOOTER string-pool candidates |
| 0003 | PR-A04 | Full R1 residual close-or-waive policy |
| 0004 | PR-B00 | Collector packaging / source-tree |
| 0005 | PR-D02 | R3 `engine=auto` product default promotion |
| 0006 | PR-B11 | Format v6 wire freeze |
| 0007 | PR-B13 | Production v6 writer backend (C baseline) |
| 0008 | PR-E02 | R4 `format=v6` product collection/output default promotion |
| **0009** | **PR-F01 (this ADR)** | R5 legacy retirement **governance** (umbrella) |

Component-specific retirement decisions (if any ever land) are **later ADRs** under this umbrella — never silent code deletion, and never reusing 0009 as a bulk-removal act.

---

## Evidence

| Source | Role |
|--------|------|
| Charter R5 row | Separate ADRs only; never automatic |
| Plan Phase 7 / REL-012 / ADR-Q026 | Per-component review; no automatic removal |
| ADR-0005 / ADR-0008 | Default flips **retain** force-legacy and `format=v5`; explicitly **not** R5 |
| Residual matrix / board | Legacy retirement listed residual / out of first slice |
| This PR | **No** component retired; **no** deprecation timer started; **no** runtime/package change |

**Gate evidence (required before any component retirement ADR may be accepted and executed):**

1. **Sustained field use** of the **replacement** path (native report engine, v6 collection/read, etc.) on every tier where retirement would remove a capability.
2. **Usage / platform evidence** for the component under review (who still needs it; which OS/Perl tiers; archival vs live profiling).
3. **Migration coverage:** documented operator path (convert, force flags, dual-install) and regression tests that drive the **real** remaining entry points.
4. **Deprecation period** advertised in release notes before any hard removal.
5. **Support end date** and (where file formats are involved) **archival reader longevity** policy.
6. **Security/maintenance rationale** that does **not** substitute for items 1–5.
7. **Independent release / ecosystem review** sign-off (REL-012 owners).
8. Offline gate green on the retirement candidate tree: `./scripts/ci/offline_gate.sh`.

If any gate item is missing → **do not retire**. Record **retain**, **narrow install**, or **extend deprecation** instead.

---

## Decision

### 1. Binding meaning of R5 (governance)

| Token | Meaning |
|-------|---------|
| **R5 governance accepted** | This ADR is binding: process, component catalog, non-claims, and residual honesty for retirement |
| **No component retired** | No legacy path is deprecated or removed by accepting this ADR |
| **Component retirement executed** | Only after a **separate, component-scoped ADR** + checklist in [`docs/R5_RETIREMENT_REVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R5_RETIREMENT_REVIEW.md) |

**Acceptance of this ADR alone must never be cited as proof that any legacy component was removed, deprecated, or scheduled for removal.**

### 2. Never automatic (binding)

| Prohibited | Why |
|------------|-----|
| Auto-retirement because R3 or R4 flipped | Defaults ≠ path removal; rollback of defaults requires retained escapes |
| Bulk “legacy removal” mega-PR without per-component ADRs | Violates charter R5 and REL-012 |
| Silent deletion of install paths, CLI tools, or readers | Breaks archival and dual-path promises |
| Treating modernization program completion as retirement authorization | REL-013 may close the program while deferring retirement indefinitely |

### 3. Per-component catalog (independent decisions)

Each row is a **potential** future decision unit. **Default outcome until a component ADR says otherwise: retain.**

| Component ID | Surface (examples) | Typical replacement | Notes |
|--------------|--------------------|---------------------|-------|
| **R5-C-LEGACY-REPORT** | Oracle `nytprofhtml` / legacy report engine on dual-path facade | Native report / HTML | Distinct from R3 product **default**; force-legacy may remain after R3 |
| **R5-C-LEGACY-READER** | Legacy-only decode path used when native CLI absent | Native `nytprof-cli` / model | Dual-path install may still need legacy-only tier |
| **R5-C-V5-WRITER** | Collector / sink **write** of NYTProf 5 | v6 writer (COL-007+) | Coupled to R4 window; **not** authorized by R4 flip alone |
| **R5-C-V5-READER** | Product ability to **read** NYTProf 5 | Convert-only workflows | Archival policy usually **retain** long after write retirement |
| **R5-C-LEGACY-CLI** | Oracle CLI tools in packaging | Native CLI surface | Narrow install is a softer alternative to full removal |
| **R5-C-MIN-PERL** | Minimum supported Perl / toolchain floors | Newer baseline | Ecosystem evidence mandatory; separate from format retirement |
| **R5-C-OTHER** | Any other legacy surface not listed | — | Still needs its own ADR; do not piggyback |

Optional softer outcomes (also require explicit decision docs, not silent change):

| Outcome | Meaning |
|---------|---------|
| **Retain** | No change (default) |
| **Narrow install** | Stop shipping on some tiers; document remaining install path |
| **Deprecate** | Warn + migration docs; hard removal still needs a later ADR or dated step in the same ADR |
| **Retire** | Hard removal after deprecation window and migration coverage |

### 4. Required shape of every component retirement ADR

A future component ADR **must** specify:

1. **Component ID** from the catalog (or justified new ID).
2. **Decision:** retain / narrow / deprecate / retire (and exact technical delta).
3. **Evidence pack** (usage, field longevity, security/maintenance cost).
4. **Deprecation period** and communication (release notes, warnings).
5. **Migration path** and tests on real entry points.
6. **Support end** and, for formats, **archival reader** guarantees.
7. **Rollback / re-ship** policy if removal causes critical field breakage.
8. **Explicit non-coupling** to other components (e.g. retiring writer does not retire reader).

### 5. Relationship to R3 / R4 and dual-path

| Topic | Coupling |
|-------|----------|
| R3 engine default (ADR-0005) | **Independent** — force-legacy retention is **not** retirement |
| R4 format default (ADR-0008) | **Independent** — retaining `format=v5` is **not** permanent forever; dropping v5 write/read needs this R5 process |
| Dual-path legacy-only install | **Not** removed by R3/R4; only by an R5 component ADR |
| COL-008 / wire freeze / packing | Unrelated to retirement |

### 6. Valid success without retirement

Program and charter success **do not** require any R5 component retirement. Completing modernization with all legacy paths retained is **success**. REL-013 may close the modernization program while listing deferred retirement as ongoing maintenance backlog.

---

## Exactness and compatibility consequences

| Area | Effect of **this** ADR |
|------|------------------------|
| Event order / counts / ticks | **None** — no runtime change |
| v5 read/write | **Retained** until a component ADR changes them |
| engine=legacy / format=v5 | **Retained** as product escape hatches |
| Packaging / dual-path | **Unchanged** |
| R3 / R4 flip state | **Unchanged** (still gated / not executed unless those checklists run) |
| Performance claims | Unchanged |

---

## Alternatives considered

| Alternative | Correctness/compatibility | Performance/storage | Security/reliability | Build/portability | Reason accepted/rejected |
|-------------|---------------------------|---------------------|----------------------|-------------------|--------------------------|
| Auto-retire legacy after R4 success | High ecosystem break risk | Possible maintenance win | Loss of rollback | Breaks dual-path | **Rejected** — charter R5 |
| Single mega-ADR removing all legacy | Opaque blast radius | — | Hard recovery | — | **Rejected** — per-component only |
| Keep ADR-Q026 deferred forever with no governance text | Weaker residual honesty | — | — | — | **Rejected** — operators need a binding “never automatic” bar now |
| Start deprecation timers in PR-F01 without evidence | Premature | — | Support spikes | — | **Rejected** — residual honesty |
| Retire v5 **read** with v5 **write** in one step | Archival loss | — | — | — | **Rejected** as default; may be considered only with separate IDs and evidence |

---

## Implementation and testing requirements

### A. This PR (PR-F01) — governance only

1. Land this ADR; index in [`docs/adrs/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/README.md).
2. Land review procedure: [`docs/R5_RETIREMENT_REVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R5_RETIREMENT_REVIEW.md).
3. Sync residual matrix, board, runbook, dual-equality honesty, ADR-Q026: **governance accepted; no component retired**.
4. **Do not** remove packages, CLIs, readers, writers, or dual-path install paths.
5. **Do not** start product deprecation warnings or timers.
6. **Do not** change R3/R4 runtime defaults.

### B. Future component retirement PR (after component ADR)

1. One primary component ID per change set unless a tightly coupled pair is justified in the component ADR.
2. Regression tests must fail before removal and pass after on **remaining** entry points; archival guarantees need explicit tests where claimed.
3. Honesty docs + release notes in the **same** change set.
4. Never put `crates/` on oracle `PERL5LIB`.

---

## Migration, rollout, and rollback

| Phase | Action |
|-------|--------|
| Now (PR-F01) | Governance + checklist only; all legacy paths retained |
| Component review (REL-012) | Fill [`docs/R5_RETIREMENT_REVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R5_RETIREMENT_REVIEW.md) per component; prefer **retain** or **narrow** when evidence is thin |
| Deprecate | Release notes + operator warnings; migration path live; no hard delete yet |
| Retire | Hard removal only after deprecation window; archival reader policy explicit |
| Incident after retirement | Re-ship removed path or convert-based recovery per component ADR rollback section |

---

## Revisit triggers

- Sustained R4 (or stronger) field use with measured legacy usage decline.
- Security finding **only** on a legacy path with no practical patch — still requires component ADR + migration.
- Platform/vendor EOL for a supported Perl tier (R5-C-MIN-PERL).
- Desire to drop `format=v5` write or read after R4 — **must** use this process (not ADR-0008 alone).
- Superseding program charter that changes R5 — new ADR; this ADR remains until superseded.

---

## Non-claims

- **Not** any component deprecation or removal.
- **Not** a schedule or end-of-life date for v5, legacy report engine, or dual-path install.
- **Not** R3 or R4 product default flips.
- **Not** permission to claim “legacy retired” in release notes or residual matrix.
- **Not** REL-013 final program close.
- **Not** public performance or CPAN claims.

---

## Normative doc pointers

| Doc | Role |
|-----|------|
| [`docs/R5_RETIREMENT_REVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R5_RETIREMENT_REVIEW.md) | Per-component review + retirement checklist |
| [`docs/PROGRAM_CHARTER.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md) | R5 row |
| [`docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) | REL-012 / REL-013 |
| [`docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md) | Level R5 |
| [`docs/adrs/0005-r3-engine-auto-default-promotion.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) | Defaults ≠ retirement |
| [`docs/adrs/0008-r4-v6-output-default-promotion.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) | Format default ≠ retirement |
| [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) | R5 residual honesty |

---

## Board / plan placement

| ID | Status | Evidence |
|----|--------|----------|
| `R5-RETIREMENT-GOVERNANCE-ADR` (PR-F01) | **done (governance; no component retired)** | this ADR + `docs/R5_RETIREMENT_REVIEW.md` |
| Charter R5 product retirement | **not complete** until component ADRs (if any) execute — **optional** | residual matrix |
| ADR-Q026 | governance answered by this ADR; component decisions still open / deferred | plan queue update |
| REL-012 | process ready; review runs still deferred on evidence | plan task remains deferred for execution |

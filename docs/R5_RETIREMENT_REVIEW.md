# R5 legacy retirement — per-component review procedure

**Status:** procedure only — **no component retired**  
**Board ID:** `R5-RETIREMENT-GOVERNANCE-ADR`  
**Binding policy:** [ADR-0009](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0009-r5-legacy-retirement-governance.md)  
**Plan task:** [REL-012](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md)  
**Does not:** remove or deprecate any path by existing in the tree; claim charter R5 retirement complete; flip R3/R4 defaults

---

## 1. Purpose

Define the **only** allowed path to deprecate, narrow, or remove a **single** legacy component under charter **R5**.

This document is the operational checklist for future **component-scoped** ADRs. **PR-F01 lands ADR-0009 + this procedure without retiring anything.**

**Absence of retirement is valid success.** Prefer **retain** when evidence is incomplete.

---

## 2. Non-negotiables

1. **Never automatic** — R3/R4 success, program completion, or maintainer fatigue alone do **not** authorize removal.
2. **Per-component only** — one primary component ID per decision (see ADR-0009 catalog).
3. **No silent removal** — code/docs/release notes and residual honesty move together.
4. **Migration first** — operators must have a documented path before hard delete.
5. **Archival honesty** — retiring write ≠ retiring read; state each explicitly.
6. **Defaults ≠ retirement** — force-legacy and `format=v5` escapes may outlive product default flips.

---

## 3. Component IDs (catalog)

| Component ID | Surface |
|--------------|---------|
| `R5-C-LEGACY-REPORT` | Legacy / oracle report engine |
| `R5-C-LEGACY-READER` | Legacy-only decode when native absent |
| `R5-C-V5-WRITER` | NYTProf 5 collection/write |
| `R5-C-V5-READER` | NYTProf 5 product read |
| `R5-C-LEGACY-CLI` | Oracle CLI packaging |
| `R5-C-MIN-PERL` | Minimum Perl / toolchain floor |
| `R5-C-OTHER` | Justified new ID in the component ADR |

---

## 4. Outcomes

| Outcome | Code impact | Min evidence |
|---------|-------------|--------------|
| **Retain** | None | Optional short note why review closed with no change |
| **Narrow install** | Stop shipping on listed tiers only | Tier usage data + dual-path docs |
| **Deprecate** | Warnings + docs; path still works | Migration path + release-note period |
| **Retire** | Hard removal after deprecation window | All gate rows in §5 |

Default if any gate fails: **Retain** or **Extend deprecation** — never partial silent delete.

---

## 5. Preconditions (all required for Deprecate/Retire)

| # | Gate | Evidence |
|---|------|----------|
| 1 | [ADR-0009](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0009-r5-legacy-retirement-governance.md) accepted (governance) | this tree |
| 2 | Component ADR drafted with exact ID + decision | `docs/adrs/NNNN-…md` |
| 3 | Replacement path sustained in field on every tier losing the component | field notes / support data |
| 4 | Usage / platform / maintenance-cost evidence recorded | review packet |
| 5 | Migration tooling and docs live | convert, force flags, dual-install as applicable |
| 6 | Deprecation period length + start/end named | release notes draft |
| 7 | Archival reader policy if formats involved | component ADR § archival |
| 8 | No open critical/high issues that the legacy path uniquely mitigates without migration | issue log |
| 9 | Ecosystem / release review sign-off (REL-012) | named approvers |
| 10 | Offline gate green on candidate tree | `./scripts/ci/offline_gate.sh` |
| 11 | Rollback / re-ship plan | component ADR |

If any gate fails → **stop**. Choose **retain**, **narrow**, or **extend deprecation**.

---

## 6. Review packet template (fill per component)

```markdown
# R5 review — <COMPONENT_ID>

- Date:
- Reviewers:
- Recommended outcome: retain | narrow | deprecate | retire
- Replacement path:
- Tiers affected:
- Usage evidence (summary + links):
- Migration path (commands / docs):
- Deprecation window (if any):
- Archival policy (if any):
- Security/maintenance rationale:
- Open issues that block retirement:
- Sign-off:
```

Store accepted packets under an agreed archive path (e.g. release evidence tree) and link them from the component ADR.

---

## 7. Execution checklist (future component PR only)

### 7.1 Decision docs

- [ ] Component ADR accepted (not only proposed).
- [ ] Residual matrix row updated for that component only.
- [ ] Operator runbook: migration + what was **not** removed.
- [ ] Release notes: theme-grouped; absolute HTTPS links; no “misc cleanup” hide.

### 7.2 Code / packaging

- [ ] Remove or narrow **only** the scoped surface.
- [ ] Do **not** couple unrelated catalog IDs without explicit multi-ID justification.
- [ ] Keep oracle `PERL5LIB` isolation: never put `crates/` on oracle load path.

### 7.3 Tests (real entry points)

- [ ] Regression that **failed** when the component was incorrectly absent (pre-fix) and **passes** after the intended change.
- [ ] Migration path tests (convert / force-v5 / force-legacy as claimed).
- [ ] Archival read tests if reader longevity is claimed.
- [ ] Dual-path install smoke still honest for remaining legacy-only tiers.

### 7.4 Forbidden in the same PR without separate ADRs

- [ ] R3 product default flip
- [ ] R4 collection default flip
- [ ] Wire ID renumber
- [ ] Lossy convert
- [ ] COL-008 baseline promotion

---

## 8. Rollback after mistaken retirement

| Layer | Action |
|-------|--------|
| **Emergency** | Re-ship the removed path in a patch release; document in release notes |
| **Operator** | Use convert / archived install / prior version pins per migration doc |
| **Process** | Open superseding ADR; record light row in `docs/agent-notes/failed-attempts.md` if an approach is abandoned |

Rollback of retirement **must not** invent false “never shipped” history — residual honesty stays explicit.

---

## 9. Relationship to program close (REL-013)

Final modernization program review may complete while:

- R5 governance (this doc + ADR-0009) is accepted, and
- **zero** component retirements have executed.

Deferred retirement remains an optional maintenance backlog, not a blocker for advertised R1/R2 (or R3/R4 when those flips execute).

---

## 10. Normative pointers

| Doc | Role |
|-----|------|
| [ADR-0009](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0009-r5-legacy-retirement-governance.md) | Binding governance |
| [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) | R3 defaults ≠ retirement |
| [ADR-0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) | R4 defaults ≠ retirement |
| [PROGRAM_CHARTER.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md) | R5 row |
| [R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) | Residual honesty |
| [19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) | REL-012 / REL-013 |

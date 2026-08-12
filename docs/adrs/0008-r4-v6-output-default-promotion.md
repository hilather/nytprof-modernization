# ADR-0008 — R4 `format=v6` product collection/output default promotion (gated)

- **Status:** **accepted (policy)** — product flip state: **not executed**
- **Date:** 2026-08-12
- **Owners/approvers:** independent release review group (REL-008); program completion plan (PLAN_ID `8c9b1a63`)
- **Related ADR-Q:** [ADR-Q025](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md) (v6 output default promotion criteria / field window)
- **Related tasks/risks/gates:** REL-007, REL-008, REL-009, COMPAT-014, TEST-020, BENCH-013 / SEC-012 (engineering gates; not public SLO claims), charter **R4**, Phase E **PR-E01** / **PR-E02**
- **Decision scope/version:** product **collection / output format default** on eligible tiers only (not R3 engine default; not wire redesign; not legacy retirement; not COL-008)

---

## Context

Charter level **R4** ([`docs/PROGRAM_CHARTER.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md)):

> v6 output default on eligible tiers — **Yes, only after field window + ADR**

Today (R2-stable / post-C05 + PR-E01 field pack):

| Surface | Default / policy |
|---------|------------------|
| Product collection default | **v5** (`capability.collection_default: "v5"`) |
| Opt-in collection / writer | `format=v6` (and dual-sink **test/dev only**) — not product default |
| Offline tools (dump / report / verify / convert / …) | Magic auto-detect `NYTPROF6` vs `NYTProf 5 …` — no format flag required for **read** |
| v5 escape hatch | Always retained: force `format=v5` / convert `--to=v5` / keep v5 profiles |

R2-stable shipped **opt-in** v6 collection/read/report plus convert/merge/repack/salvage with honesty that the **collection default remains v5**. That is **not** the R4 product default flip. R4 means the product default for **new collection/output** on **eligible tiers** becomes **v6**, while **`format=v5` remains supported** for the compatibility window.

Promotion without field evidence risks ecosystem breakage (old tools that only read v5, mixed-team friction, convert/size surprises, fork/long-run issues) and hard-to-roll-back defaults. Plan tasks [REL-008](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) and acceptance Level **R4** require a field window, default-format ADR, `format=v5` escape, and rollback independent of legacy removal.

**Residual honesty for this PR (PR-E02):** no accepted multi-site field report exists in-tree that recommends **promote**. Therefore this ADR **binds policy and procedure only**. It does **not** change runtime `collection_default`, packaging defaults, capability self-test expectations, or offline_gate defaults.

Numbering coordination (PLAN `8c9b1a63`):

| Number | Owner | Topic |
|--------|-------|--------|
| 0001 / 0002 | PR-B01 | v6 packing / FOOTER string-pool candidates |
| 0003 | PR-A04 | Full R1 residual close-or-waive policy |
| 0004 | PR-B00 | Collector packaging / source-tree |
| 0005 | PR-D02 | R3 `engine=auto` product default promotion (when present) |
| 0006 | PR-B11 | Format v6 wire freeze |
| 0007 | PR-B13 | Production v6 writer backend (C baseline) |
| **0008** | **PR-E02 (this ADR)** | R4 `format=v6` product collection/output default promotion |

Do **not** reuse 0005 for R4 (R3 engine default is separate). Do not couple this ADR to R3 engine promotion.

---

## Evidence

| Source | Role |
|--------|------|
| Field-window pack (PR-E01) | Local evidence collection — [`docs/R4_FIELD_WINDOW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_FIELD_WINDOW.md), [`scripts/field/r4_field_window_collect.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/r4_field_window_collect.sh), [`docs/templates/R4_FIELD_WINDOW_REPORT.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R4_FIELD_WINDOW_REPORT.md) |
| R2-stable base | Convert/merge/repack/salvage + E5 v6 opt-in — [`docs/RELEASE_NOTES_R2_STABLE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_STABLE.md) |
| Capability honesty | `collection_default: v5` until flip execution |
| Residual matrix | R4 row remains **not flipped** until flip checklist completes |
| Plan REL-007 / REL-008 / ADR-Q025 | Opt-in cycle → field window → default-format ADR → promote |
| This PR | **No** field packs claiming multi-site promote; **no** runtime flip |

**Gate evidence (required before flip execution, not claimed present by this ADR):**

1. One or more filled field-window reports with status **accepted** and recommendation **Promote**.
2. Coverage: ≥1 production-like site per **advertised eligible OS tier** listed at flip time.
3. No open **critical/high** v6-path correctness, corruption, convert, data-loss, or security issues attributable to the format / tooling path intended as default.
4. **v5 escape hatch** verified on each eligible tier (`format=v5` and/or convert `--to=v5` / old-tool shape after successful v5 convert).
5. Tools auto-detect and **read** v6; capability honesty remains consistent with advertised R2-stable+ surfaces (convert/merge when claimed).
6. Release review + compatibility/QA sign-off on the report(s).
7. Offline gate green on the flip candidate tree: `./scripts/ci/offline_gate.sh`.
8. Compatibility-window reaffirmation: **`format=v5` retained** (no silent deprecation as part of the flip).

If any gate item is missing → **do not flip**. Extend the field window or record **Do not promote**.

---

## Decision

### 1. Binding product meaning of R4 (when flip is executed)

On **product collection / output surfaces** that advertise a default profile format (collector options, env, capability `collection_default`, and any facade that documents collection format):

| Control | Pre-R4 (current / this ADR until flip) | Post-R4 (after flip execution only, **eligible tiers**) |
|---------|----------------------------------------|----------------------------------------------------------|
| Product collection / output default | **v5** (`collection_default: "v5"`) | **v6** (`collection_default: "v6"`) |
| Explicit `format=v5` / force-v5 | Supported | **Retained** — operator escape hatch and compatibility window |
| Explicit `format=v6` | Opt-in | Same semantics (now matches product default on eligible tiers) |
| Offline **read** (dump/report/verify/…) | Magic auto-detect | Unchanged — continues to read both majors |
| Convert | Opt-in tooling | Unchanged contract; v5↔v6 strict path remains available |

**Ineligible / non-advertised tiers** keep the pre-R4 default (**v5**) unless a superseding release explicitly extends eligibility with new field evidence.

### 2. Eligible tiers

Eligible OS/arch (and any other advertised support tiers) for the product format default flip are **only** those listed in the accepted field-window report(s) and reaffirmed in release notes at flip time. Tiers without accepted field evidence remain on **v5** default (or document explicit non-eligibility). Multi-OS CI MVP (BUILD-006-MVP) is **not** by itself field-window evidence.

### 3. Compatibility window (binding)

| Item | Policy |
|------|--------|
| **`format=v5` retained** | Always after flip for at least the advertised compatibility window; flip must **not** remove v5 write/collection |
| Old tools that only read v5 | Operators use convert `--to=v5` or force v5 collection; flip release notes must document the workflow |
| v6 read after rollback | Product default may roll back to v5 while **retaining** v6 read/report/convert support |
| Dual-sink `format=dual` | Remains **test/dev only** (OQ-4) — never product default under R4 |

### 4. Escape hatch and rollback design (binding)

| Layer | Mechanism |
|-------|-----------|
| **Operator one-step (force v5)** | Explicit `format=v5` / documented env or collector option that forces v5 collection (always retained after flip) |
| **Operator convert escape** | `nytprof-cli convert --to=v5 IN -o OUT` for old-tool shape |
| **Product default rollback** | Revert capability + collector/facade default so `collection_default` is **`v5`** again; ship as patch/release with release notes |
| **Monitoring triggers** | Open high/critical v6-path correctness / corruption / convert issues on the defaulted path; security issue; field incident process (REL-009) |
| **Rollback owner** | Named in the accepted field report and release notes at flip time |

Rollback of the **product default** does **not** remove v6 read/report, does **not** un-freeze wire IDs (ADR-0006), and does **not** imply R5 legacy retirement or COL-008 promotion.

### 5. Flip is gated — not executed by accepting this ADR

| State token | Meaning |
|-------------|---------|
| **policy accepted** | This ADR is binding for criteria, eligible-tier policy, procedure, rollback design |
| **flip not executed** | Runtime product collection default remains **v5** |
| **flip executed** | Only after gate evidence (§ Evidence) + flip checklist in [`docs/R4_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md) + release notes + honesty doc sync |

**Acceptance of this ADR alone must never be cited as proof that R4 product defaults already changed**, that capability `collection_default` is `"v6"`, or that charter R4 is complete.

### 6. Decoupling from R3 and other tracks

| Topic | Coupling |
|-------|----------|
| R3 `engine=auto` (ADR-0005 when present) | **Independent** — never piggyback format default on engine default or vice versa |
| Wire freeze (ADR-0006) | Prerequisite for stable v6; flip does not change IDs |
| C writer backend (ADR-0007) | Unchanged by default flip |
| COL-008 | Remains deferred / non-baseline |
| Public perf SLOs | Field size/wall samples are **not** certification |

---

## Exactness and compatibility consequences

| Area | Effect |
|------|--------|
| Wire bytes / event semantics | Unchanged by default selection; same v5/v6 codecs |
| Offline tools | Continue magic detect; no requirement to drop v5 read |
| Capability `collection_default` | Remains **`"v5"`** until flip execution; becomes **`"v6"`** only after flip on eligible product builds |
| Convert / merge / salvage | Contracts unchanged; flip does not authorize lossy convert |
| Old v6.15 tools | Still cannot read v6 directly — convert / force-v5 remains the documented path |
| R3 engine default | Unchanged by this ADR |
| Performance claims | Field size/overhead samples are **not** public SLOs |

---

## Alternatives considered

| Alternative | Correctness/compatibility | Performance/storage | Security/reliability | Build/portability | Reason accepted/rejected |
|-------------|---------------------------|---------------------|----------------------|-------------------|--------------------------|
| Flip `collection_default` to v6 in PR-E02 without accepted field report | Weak field confidence; old-tool risk | Possible size wins unvalidated | High incident risk | Same | **Rejected** — residual honesty / charter R4 |
| Keep ADR proposed until field promote | Weaker procedural freeze | — | — | — | **Rejected** — policy needs to be binding so operators collect against a known bar |
| Default to v5 forever | Safest compat | Misses compact/exact v6 goals | Low | — | **Rejected** as terminal policy; R4 exists to promote after evidence |
| Flip globally on all OS without per-tier evidence | Overclaims portability | — | Uneven field risk | Conflicts with eligible-tier charter | **Rejected** — eligible tiers only |
| Couple R4 with R3 engine flip | Larger blast radius | — | Harder rollback | — | **Rejected** — charter separately promotable outcomes |
| Drop `format=v5` at flip | Breaks compatibility promise | — | Support spikes | — | **Rejected** — REL-008 requires v5 retained |

---

## Implementation and testing requirements

### A. This PR (PR-E02) — policy only

1. Land this ADR; index in [`docs/adrs/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/README.md).
2. Land flip/rollback procedure [`docs/R4_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md).
3. Update residual matrix, board, field-window guide/template/schema, runbook, R2-stable residual notes, ADR-Q025, README — all stating **policy accepted; flip not executed**.
4. **Do not** change capability self-test, collector default, CLI defaults, or offline_gate.

### B. Future flip PR (after accepted Promote report)

1. Change product collection default resolution so new profiles on eligible tiers write **v6** when format is omitted.
2. Set capability `collection_default` to **`"v6"`** with tests that fail if still `"v5"` on flipped builds.
3. Keep force-`format=v5` and convert `--to=v5` green; dual-sink remains non-product.
4. Regression tests must drive **real entry points** (capability CLI, collector option surface, facade if any) — no stubbed “always pass.”
5. Update honesty docs in the **same** change set as the runtime flip.
6. Release notes: eligible tiers, rollback owner, absolute HTTPS links to this ADR + field report archive.

---

## Migration, rollout, and rollback

| Phase | Action |
|-------|--------|
| Now (PR-E02) | Policy + checklist only; operators run PR-E01 field packs with `no_default_flip=true` / `collection_default=v5` |
| After accepted **Promote** report | Execute [`docs/R4_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md) as a dedicated flip PR |
| Post-flip incident | Operator force-v5 immediately; product default rollback via patch if needed |
| Files already produced | Existing v5 and v6 profiles remain valid; tools keep dual magic detect |

---

## Revisit triggers

- Accepted field report recommends **Do not promote** or **Extend window**.
- Wire or codec security finding on the defaulted path.
- Convert / fork / long-run regression class that blocks eligible tiers.
- Superseding major format beyond 6 — new ADR; this ADR does not auto-extend.
- Desire to drop `format=v5` — **separate** retirement ADR (ADR-Q026 / REL-012 class); not this promotion.

---

## Non-claims

- **Not** charter R4 product **runtime** format default flip (flip **not executed**).
- **Not** an accepted multi-site promote field report in-tree.
- **Not** R3 engine default flip (ADR-Q024 / ADR-0005 when present).
- **Not** COL-008 baseline; **not** lossy convert; **not** E3-mixed complete.
- **Not** public performance certification or CPAN upload.
- **Not** permission to claim `collection_default: v6` in capability until flip execution.
- **Not** a wire-ID or packing change (see ADR-0006 / ADR-0001 / ADR-0002).

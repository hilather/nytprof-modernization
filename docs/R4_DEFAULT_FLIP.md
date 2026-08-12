# R4 product format default flip — procedure and rollback

**Status:** procedure only — **flip not executed**  
**Board ID:** `R4-DEFAULT-CHANGE-ADR`  
**Binding policy:** [ADR-0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md)  
**Field evidence pack:** [docs/R4_FIELD_WINDOW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_FIELD_WINDOW.md) (PR-E01)  
**Does not:** change runtime defaults by existing in the tree; claim charter R4 complete; flip R3 engine default

---

## 1. Purpose

Define the **only** allowed path to promote product policy so that, on **eligible tiers**, the product **collection / output default** becomes **`format=v6`** (`collection_default: "v6"`), while **`format=v5` remains supported**, per charter **R4**.

This document is the operational checklist for a future flip PR. **PR-E02 lands the ADR + this procedure without flipping.**

---

## 2. Current vs target default

| Surface | Current default | Target after flip (eligible tiers only) |
|---------|-----------------|-----------------------------------------|
| Product collection / output | **v5** (`collection_default: "v5"`) | **v6** (`collection_default: "v6"`) |
| Explicit `format=v5` / force-v5 | Supported | **Retained** — operator one-step escape / compatibility window |
| Explicit `format=v6` | Opt-in | Same semantics (now matches product default on eligible tiers) |
| Offline read tools | Magic auto-detect both majors | Unchanged |
| Convert `--to=v5` / `--to=v6` | Available (R2-stable) | Unchanged contracts |
| Ineligible / non-advertised tiers | v5 default | **Remain v5** until separate evidence + release note extension |
| Dual-sink `format=dual` | Test/dev only | **Never** product default |

---

## 3. Preconditions (all required)

Do **not** start the flip PR until every item is true:

| # | Gate | Evidence |
|---|------|----------|
| 1 | [ADR-0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) accepted (policy) | this tree |
| 2 | Field-window report(s) status **accepted**, recommendation **Promote** | filled [R4_FIELD_WINDOW_REPORT](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R4_FIELD_WINDOW_REPORT.md) |
| 3 | ≥1 site pack per **eligible tier** intended for the flip | pack roots + `summary.json` |
| 4 | No open critical/high v6-path correctness / corruption / convert / data-loss / security issues | report issues log |
| 5 | v5 escape hatch verified on each eligible tier | report § Escape hatch / convert |
| 6 | Tools auto-detect and read v6; convert both ways exercised where claimed | pack runs + capability |
| 7 | Release review + compatibility/QA sign-off | report § Sign-off |
| 8 | Offline gate green on flip candidate | `./scripts/ci/offline_gate.sh` |
| 9 | Rollback owner named | report + release notes draft |
| 10 | Compatibility window reaffirmed: **`format=v5` retained** | release notes draft |

If any gate fails → **stop**. Choose **extend window** or **do not promote**. Incomplete evidence is not a partial flip.

---

## 4. Flip execution checklist (future PR)

Use a dedicated branch and PR. Title should state **R4 product format default flip executed** (not merely ADR).

### 4.1 Code (collection default surfaces)

- [ ] Change omitted-format resolution so product collection/output default is **v6** on **eligible-tier** product builds.
- [ ] Set capability honesty `collection_default` to **`"v6"`** (human `collection_default: v6`).
- [ ] Keep force-**v5** path(s) and convert `--to=v5` fully usable.
- [ ] Do **not** make dual-sink product default.
- [ ] Do **not** remove v5 write/read or break magic detect for offline tools.
- [ ] Document ineligible tiers still defaulting to v5 (if any).

### 4.2 Tests (must drive real entry points)

- [ ] Capability self-test / `capability --json`: `collection_default` is **`"v6"`** on flipped builds (and regression fails if still `"v5"`).
- [ ] Omitted format on eligible collection path produces **v6** magic (`NYTPROF6`).
- [ ] Explicit force-v5 still produces v5 (`NYTProf 5` / oracle-readable as advertised).
- [ ] Offline report/verify on both majors still green (default-calls1 / dual-sink samples: leaf **15** / mid **3** where fixture applies).
- [ ] Convert `--to=v5` and `--to=v6` still pass on dual-sink lab pair.
- [ ] Update packaging / field smokes that assert `collection_default: v5` or `no_default_flip=true` as product truth for post-flip trees (schema revision or post-flip collector honesty).

### 4.3 Docs / honesty (same change set)

- [ ] Residual matrix R4 row: product format default **flipped** for listed eligible tiers; link this checklist completion + report IDs.
- [ ] [cli-e5-v6-opt-in-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cli-e5-v6-opt-in-mvp-v0.md) / capability schema: `collection_default` **v6** post-flip.
- [ ] Operator runbook: R4 default live on eligible tiers; force-v5 escape documented first.
- [ ] Board / charter pointers: R4 product default complete **only** for eligible tiers.
- [ ] Release notes: delta, eligible tiers, compatibility window, rollback owner, absolute HTTPS links to ADR-0008 + field report archive.
- [ ] Field collector: if packs are still produced post-flip, stop requiring `no_default_flip=true` / `collection_default=v5` as a product claim (or document post-flip schema revision) — do not leave contradictory honesty flags.

### 4.4 Release mechanics

- [ ] Version / tag decision recorded.
- [ ] Post-release monitoring owner (REL-009 spirit).
- [ ] One-step operator rollback published in release notes:

```sh
# Force v5 collection (operator rollback / escape hatch) — exact flag/env as shipped at flip time
# Example shapes (replace with the product option names from the flip PR):
#   format=v5 in NYTPROF / collector options
#   convert escape for already-written v6:
nytprof-cli convert --to=v5 path/to/v6.nytprof -o path/to/v5.out
```

### 4.5 Explicit non-goals of the flip PR

- No R3 engine default flip (separate ADR-0005 / R3_DEFAULT_FLIP when present).
- No COL-008 baseline; no lossy convert; no wire-ID change.
- No public performance SLOs from field wall times / size samples alone.
- No R5 / legacy retirement; no drop of `format=v5`.
- No CPAN upload claim unless packaging track separately certifies.

---

## 5. Rollback procedure

### 5.1 Immediate operator mitigation (no release required)

| Goal | Action |
|------|--------|
| New profiles as v5 | Force `format=v5` (product option / env as documented at flip time) |
| Old tools need v5 shape | `nytprof-cli convert --to=v5 IN -o OUT` |
| Site-specific pin | Document format pin in deploy config |

### 5.2 Product default rollback (release / patch)

1. Revert the default-resolution change so product collection default is **v5** again (`collection_default: "v5"`).
2. Keep v6 **read**/report/convert available.
3. Keep force-v5 and force-v6 (opt-in) available.
4. Update residual matrix honesty: default **rolled back**; link incident / report.
5. Release notes: why, who owns follow-up, whether a new field window is required before re-flip.
6. Regression tests: confirm capability `collection_default` is `"v5"` again; v6 opt-in still works.

### 5.3 Rollback triggers (from ADR-0008)

- Open high/critical v6-path correctness, corruption, convert, data-loss, or security issue on the defaulted path.
- Systemic convert/old-tool failures that flood support or hide data loss.
- Release-owner judgment under post-release field validation (REL-009).

---

## 6. Residual honesty (this document alone)

| Claim | Status |
|-------|--------|
| ADR-0008 policy for R4 promotion | **accepted** |
| Flip procedure + rollback documented | **yes** (this file) |
| Product format default flipped in tree | **no** |
| Charter R4 complete | **no** |
| Accepted multi-site promote report in-tree | **not claimed** — operators must supply |
| R3 engine default | **out of scope** |
| `collection_default` | remains **v5** until flip execution |

---

## 7. Commands (pre-flip evidence; still valid)

```sh
# Collect local field pack (does not flip defaults)
./scripts/field/r4_field_window_collect.sh --out /tmp/r4-field-pack

# Layout + honesty smoke (expects collection_default=v5, no_default_flip=true)
./scripts/field/r4_field_window_smoke.sh

# Capability honesty (R2-stable+ tools)
./scripts/packaging/capability_selftest_smoke.sh
# Expect: collection_default: v5
```

Fill: [docs/templates/R4_FIELD_WINDOW_REPORT.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R4_FIELD_WINDOW_REPORT.md).

---

## 8. Links

| Doc | URL |
|-----|-----|
| ADR-0008 | https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md |
| Field window guide | https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_FIELD_WINDOW.md |
| Program charter R4 | https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md |
| Residual matrix | https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md |
| REL-007 / REL-008 | https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md |
| Level R4 acceptance | https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md |
| ADR-Q025 | https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md |
| R2-stable notes | https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_STABLE.md |

# R3 product default flip — procedure and rollback

**Status:** procedure only — **flip not executed**  
**Board ID:** `R3-DEFAULT-CHANGE-ADR`  
**Binding policy:** [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md)  
**Field evidence pack:** [docs/R3_FIELD_WINDOW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md) (PR-D01)  
**Does not:** change runtime defaults by existing in the tree; claim charter R3 complete; flip R4 format default

---

## 1. Purpose

Define the **only** allowed path to promote product policy so that, on dual-path surfaces, **omitted** `--engine` / `NYTPROF_ENGINE` resolves to **`auto`** (prefer native; fall back to legacy), per charter **R3**.

This document is the operational checklist for a future flip PR. **PR-D02 lands the ADR + this procedure without flipping.**

---

## 2. Current vs target default

| Surface | Current default (omit flag/env) | Target after flip |
|---------|----------------------------------|-------------------|
| Perl `nytprof-engine` / EngineDispatch | `native` (fail if CLI missing) | `auto` (prefer native; legacy fallback + STDERR note) |
| Explicit `--engine=auto` | Prefer-native / fall-back-legacy (already shipped) | Unchanged semantics |
| Explicit `--engine=legacy` | Oracle legacy | **Retained** — operator one-step escape / rollback |
| Explicit `--engine=native` | Fail closed if missing | Unchanged |
| Pure-Rust `nytprof-cli` | `native`; `auto`→`native` residual | **No dual-path invent**; residual may remain |

---

## 3. Preconditions (all required)

Do **not** start the flip PR until every item is true:

| # | Gate | Evidence |
|---|------|----------|
| 1 | [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) accepted (policy) | this tree |
| 2 | Field-window report(s) status **accepted**, recommendation **Promote** | filled [R3_FIELD_WINDOW_REPORT](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R3_FIELD_WINDOW_REPORT.md) |
| 3 | ≥1 site pack per **eligible tier** intended for the flip | pack roots + `summary.json` |
| 4 | No open critical/high native-path correctness / data-loss / security issues | report issues log |
| 5 | Force-legacy verified on each eligible tier | report § Fallback |
| 6 | Fallback does not hide corruption | report + issue log |
| 7 | Release review + compatibility/QA sign-off | report § Sign-off |
| 8 | Offline gate green on flip candidate | `./scripts/ci/offline_gate.sh` |
| 9 | Rollback owner named | report + release notes draft |

If any gate fails → **stop**. Choose **extend window** or **do not promote**. Incomplete evidence is not a partial flip.

---

## 4. Flip execution checklist (future PR)

Use a dedicated branch and PR. Title should state **R3 product default flip executed** (not merely ADR).

### 4.1 Code (dual-path facade)

- [ ] Change omitted-engine resolution so default requested engine is **`auto`** (e.g. `resolve_engine(undef, undef)` / help default string in `EngineDispatch` / `nytprof-engine`).
- [ ] Keep precedence: CLI flag **overrides** env **overrides** product default.
- [ ] Do **not** weaken fail-closed behavior for explicit `native`.
- [ ] Do **not** invent pure-Rust in-process legacy for `nytprof-cli` unless a separate ADR says so.

### 4.2 Tests (must drive real entry points)

- [ ] Omitted engine + native discoverable: default-calls1 report leaf **15** / mid **3**.
- [ ] Omitted engine + `NYTPROF_FORCE_NO_NATIVE=1` (or native absent): STDERR auto-fallback note; legacy success only when oracle path available — honest non-zero otherwise.
- [ ] Explicit `--engine=legacy` still works.
- [ ] Explicit `--engine=native` + force-no-native fails closed (no silent legacy).
- [ ] Update packaging smokes if they assume default is `native` without a flag.

### 4.3 Docs / honesty (same change set)

- [ ] Residual matrix R3 row: product default **flipped** for listed eligible tiers; link this checklist completion + report IDs.
- [ ] [engine-selection-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/engine-selection-mvp-v0.md): product default = `auto` on facade.
- [ ] Operator runbook: R3 default live; force-legacy escape documented first.
- [ ] Board / charter pointers: R3 product default complete **only** for eligible tiers.
- [ ] Release notes: delta, eligible tiers, rollback owner, absolute HTTPS links to ADR-0005 + field report archive.
- [ ] Field collector: if packs are still produced post-flip, stop requiring `no_default_flip=true` as a product claim (or document post-flip schema revision) — do not leave contradictory honesty flags.

### 4.4 Release mechanics

- [ ] Version / tag decision recorded.
- [ ] Post-release monitoring owner (REL-009 spirit).
- [ ] One-step operator rollback published in release notes:

```sh
# Force legacy (operator rollback / escape hatch)
perl -Iperl/lib perl/bin/nytprof-engine --engine=legacy report path/to/nytprof.out
# or:
NYTPROF_ENGINE=legacy perl -Iperl/lib perl/bin/nytprof-engine report path/to/nytprof.out
```

### 4.5 Explicit non-goals of the flip PR

- No COL-007 / v6 wire freeze / CLI v6 default (R4 is separate).
- No public performance SLOs from field wall times alone.
- No R5 legacy retirement.
- No CPAN upload claim unless packaging track separately certifies.

---

## 5. Rollback procedure

### 5.1 Immediate operator mitigation (no release required)

| Goal | Action |
|------|--------|
| All reports on legacy | `--engine=legacy` or `NYTPROF_ENGINE=legacy` |
| Fail closed without fallback | `--engine=native` (will error if CLI missing) |
| Site-specific pin | Document env in deploy config |

### 5.2 Product default rollback (release / patch)

1. Revert the default-resolution change so omitted flag/env is **`native`** again (pre-R3).
2. Keep `engine=auto` prefer-native available for opt-in.
3. Keep force-legacy available.
4. Update residual matrix honesty: default **rolled back**; link incident / report.
5. Release notes: why, who owns follow-up, whether a new field window is required before re-flip.
6. Regression tests: confirm omitted engine is `native` again; auto still works when explicit.

### 5.3 Rollback triggers (from ADR-0005)

- Open high/critical native-path correctness, data-loss, or security issue on the defaulted path.
- Systemic fallback failures that hide corruption or flood support.
- Release-owner judgment under post-release field validation (REL-009).

---

## 6. Residual honesty (this document alone)

| Claim | Status |
|-------|--------|
| ADR-0005 policy for R3 promotion | **accepted** |
| Flip procedure + rollback documented | **yes** (this file) |
| Product default flipped in tree | **no** |
| Charter R3 complete | **no** |
| Accepted multi-site promote report in-tree | **not claimed** — operators must supply |
| R4 format default | **out of scope** |

---

## 7. Commands (pre-flip evidence; still valid)

```sh
# Collect local field pack (does not flip defaults)
./scripts/field/r3_field_window_collect.sh --out /tmp/r3-field-pack

# Layout + honesty smoke
./scripts/field/r3_field_window_smoke.sh

# Existing dual-path auto smokes (explicit auto, not product default)
./scripts/packaging/engine_auto_smoke.sh
./scripts/packaging/engine_auto_fallback_smoke.sh
```

Fill: [docs/templates/R3_FIELD_WINDOW_REPORT.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R3_FIELD_WINDOW_REPORT.md).

---

## 8. Links

| Doc | URL |
|-----|-----|
| ADR-0005 | https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md |
| Field window guide | https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md |
| Program charter R3 | https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md |
| Residual matrix | https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md |
| REL-006 | https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md |
| Level R3 acceptance | https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md |
| ADR-Q024 | https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md |

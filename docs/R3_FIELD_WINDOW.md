# R3 field window — `engine=auto` evidence (no default flip)

**Status:** instrumentation + report package only (**PR-D01**)  
**Board ID:** `R3-FIELD-WINDOW-PACK`  
**Charter level:** [R3](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md) — *`engine=auto` prefers native reports* — **only after field window + ADR**  
**Does not:** flip product defaults, ship PR-D02 default-change ADR, claim R3 complete, transmit telemetry, or change offline_gate defaults

---

## 1. Purpose

Collect **local, operator-controlled** field evidence that native report paths are safe enough on real workloads to consider promoting product policy so that **`engine=auto` prefers native** (charter **R3**).

This package is the **field-window half** of Phase D:

| Piece | Role | This PR |
|-------|------|---------|
| Evidence collector | Local pack under an output directory | **yes** — [`scripts/field/r3_field_window_collect.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/r3_field_window_collect.sh) |
| Smoke | Fixture-backed check that the collector works | **yes** — [`scripts/field/r3_field_window_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/r3_field_window_smoke.sh) |
| Report template | Human pack for multi-site review | **yes** — [`docs/templates/R3_FIELD_WINDOW_REPORT.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R3_FIELD_WINDOW_REPORT.md) |
| Pack schema | Layout + machine-readable summary | **yes** — [`docs/schemas/r3-field-window-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r3-field-window-mvp-v0.md) |
| Default-change ADR + flip procedure | PR-D02 / ADR-Q024 / [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) | **policy landed** — flip **not** executed; see [`docs/R3_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) |

**Binding non-claims**

- Product default remains **opt-in native** / facade `engine=auto` behavior as shipped under full R1 MVP — **not** “auto is the product default.”
- Pure-Rust `nytprof-cli` still maps `auto` → `native` (no in-process legacy). Dual-path auto evidence is the **Perl facade** (`nytprof-engine`).
- No COL-007 / v6 wire freeze / CLI v6 default / R4 format default.
- Light wall-time samples in a pack are **not** public perf certification.

---

## 2. Preconditions

1. Full R1 MVP product cut honesty accepted ([`docs/RELEASE_NOTES_R1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R1.md), residual matrix § Full R1 ready).
2. Offline gate green on the candidate tree when claiming lab readiness: `./scripts/ci/offline_gate.sh`.
3. Operators can run `perl -Iperl/lib perl/bin/nytprof-engine` and (when measuring native prefer) a discoverable native CLI (`prefix/bin`, `target/*/nytprof-dump`, `$NYTPROF_NATIVE_CLI`, or `cargo`).
4. Never put `crates/` on oracle `PERL5LIB`.

---

## 3. What to collect (minimum evidence set)

Aligned with plan [REL-005](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) / [REL-006](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) and [ADR-Q024](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md):

| Evidence class | Local pack artifact | Notes |
|----------------|---------------------|-------|
| Provenance | `env/provenance.txt`, `summary.json` | OS, uname, Perl `-V` summary, git commit, tool paths |
| Capability | `capability/capability.json` (when native present) | `decode`/`report`/`verify` true; optional `profile_ok` |
| Prefer-native under auto | `runs/engine_auto_*` | Perl facade `--engine=auto report` (and query when useful) |
| Explicit native | `runs/engine_native_*` | Same profile; semantic samples when fixture-like |
| Explicit legacy escape | `runs/engine_legacy_*` | Force path remains usable |
| Fallback when native missing | `runs/engine_auto_force_no_native_*` | `NYTPROF_FORCE_NO_NATIVE=1` + **auto** (test hook) **or** field note when CLI absent. STDERR fallback note required; **`rc==0` only if** `baseline/6.15/install` present — honest non-zero otherwise |
| Explicit native fail-closed under force | `runs/engine_native_force_no_native_*` | `NYTPROF_FORCE_NO_NATIVE=1` + **native** must **not** silent-legacy; non-zero `rc` |
| Issues / severity | report template § Issues | High-severity correctness → window fail / no promotion |
| Platform tier | report template + provenance | Eligible tiers for a future ADR |
| Duration / volume | report template header | Suggested window in §4 — not frozen by this pack |

Operator profiles may be supplied as extra paths. **Do not** paste secrets or full proprietary source into public packs; see redaction in the schema.

---

## 4. Suggested field-window parameters (provisional)

These are **engineering defaults for the report**, not a ratified REL-001 policy freeze (open question OQ-7 / ADR-Q024):

| Parameter | Provisional guidance |
|-----------|----------------------|
| Duration | At least **one** stable opt-in cycle on the full R1 MVP cut (calendar length set by release lead) |
| Sites / tiers | ≥1 production-like site per **advertised** OS tier intended for R3 eligibility |
| Workloads | Mix of long-running and short CLI; include multi-file and blocks-style profiles when available |
| Correctness bar | No unresolved **high** severity event/count/source/call mismatch attributable to native path |
| Fallback bar | Document fallback frequency; fallback must not hide corruption |
| Rollback bar | Document one-step force-legacy (`--engine=legacy` / `NYTPROF_ENGINE=legacy`) |

Promotion policy is **[ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md)** (**PR-D02**); runtime flip is a later checklist in [`docs/R3_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md). **This package never flips defaults.**

---

## 5. Commands (copy-paste)

From repo root (or any cwd — scripts resolve the tree):

```sh
# Lab / fixture-backed evidence pack (default-calls1)
./scripts/field/r3_field_window_collect.sh \
  --out /tmp/r3-field-pack-lab

# Operator profile(s) in addition to golden fixtures
./scripts/field/r3_field_window_collect.sh \
  --out /tmp/r3-field-pack-site-a \
  --profile /path/to/redacted-or-local/nytprof.out

# Optional labels for multi-site aggregation
./scripts/field/r3_field_window_collect.sh \
  --out /tmp/r3-field-pack-site-a \
  --site site-a \
  --note "staging API workers; engine=auto opt-in only"

# Smoke that the collector layout is intact (no default flip)
./scripts/field/r3_field_window_smoke.sh
```

Fill the human report from pack contents:

- Template: [`docs/templates/R3_FIELD_WINDOW_REPORT.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R3_FIELD_WINDOW_REPORT.md)
- Generic evidence bundle (release-scale): [`docs/plan/templates/EVIDENCE_BUNDLE_TEMPLATE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/templates/EVIDENCE_BUNDLE_TEMPLATE.md)

Related facade smokes (already in offline gate; not substitutes for field packs):

```sh
./scripts/packaging/engine_auto_smoke.sh
./scripts/packaging/engine_auto_fallback_smoke.sh
./scripts/packaging/capability_selftest_smoke.sh
```

---

## 6. Residual honesty

| Claim | Status under PR-D01 + PR-D02 |
|-------|------------------------------|
| Field evidence **collection** tools + report template | **ready** (PR-D01) |
| Default-change **policy** ADR (ADR-Q024 / ADR-0005) | **accepted (policy)** (PR-D02) |
| R3 product default **runtime** flip | **not** executed — remains residual until flip checklist + accepted promote report |
| R4 format default field window | separate PR-E01 package |
| Public performance SLOs from field packs | **not** claimed |

Residual matrix row: `engine=auto` product default flip remains **OUT-OF-R1** / flip-not-executed; this pack is **instrumentation only**.

---

## 7. Exit criteria for the *window* (feeds PR-D02 — not auto-promoted)

The field window report is **accepted** when maintainers can answer yes to:

1. Packs from eligible tiers exist with `summary.json` `no_default_flip=true` and honest `native_discoverable` / fallback notes.
2. No open high-severity native-path correctness issues for the window.
3. Force-legacy escape hatch verified on each tier.
4. Fallback when native is missing is documented and does not claim false native success.
5. Report template completed with site list, duration, issue log, and recommendation (**promote** / **extend window** / **do not promote**).

Only then may maintainers run the flip checklist in [`docs/R3_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) under [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md). **This package never flips defaults.** Incomplete evidence → **do not flip**.

---

## 8. Links

| Doc | URL |
|-----|-----|
| Program charter R3 | https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md |
| R1 residual matrix | https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md |
| Operator runbook | https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md |
| Engine selection MVP | https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/engine-selection-mvp-v0.md |
| Acceptance R3 criteria | https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md |
| Rollout REL-005/006 | https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md |
| ADR-Q024 | https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md |
| ADR-0005 (R3 promotion policy) | https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md |
| R3 flip / rollback procedure | https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md |

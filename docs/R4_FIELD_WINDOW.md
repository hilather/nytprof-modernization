# R4 field window — `format=v6` evidence (runtime flip gated)

**Status:** instrumentation + report package (**PR-E01**); promotion **policy** landed in **PR-E02** / [ADR-0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) (**flip not executed**)  
**Board ID:** `R4-FIELD-WINDOW-PACK`  
**Charter level:** [R4](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md) — *v6 output default on eligible tiers* — **only after field window + ADR**  
**Does not:** flip product **runtime** defaults, claim R4 complete, transmit telemetry, or change offline_gate defaults

---

## 1. Purpose

Collect **local, operator-controlled** field evidence that opt-in **v6** profile output and the R2-stable offline toolchain (decode / report / verify / convert / merge / repack / salvage) are safe enough on real workloads to consider promoting product policy so that **v6 is the collection/output default on eligible tiers** (charter **R4**).

This package is the **field-window half** of Track E (default-format evaluation). It **mirrors** the R3 field-window package structure ([`docs/R3_FIELD_WINDOW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md) when present on a branch that includes PR-D01) but targets **format**, not engine selection. Promotion **policy** and flip checklist live in PR-E02 (ADR-0008 + [`docs/R4_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md)); this guide does **not** execute the flip.

| Piece | Role | Status |
|-------|------|--------|
| Evidence collector | Local pack under an output directory | **ready** (PR-E01) — [`scripts/field/r4_field_window_collect.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/r4_field_window_collect.sh) |
| Smoke | Fixture-backed check that the collector works | **ready** (PR-E01) — [`scripts/field/r4_field_window_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/r4_field_window_smoke.sh) |
| Report template | Human pack for multi-site review | **ready** (PR-E01) — [`docs/templates/R4_FIELD_WINDOW_REPORT.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R4_FIELD_WINDOW_REPORT.md) |
| Pack schema | Layout + machine-readable summary | **ready** (PR-E01) — [`docs/schemas/r4-field-window-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r4-field-window-mvp-v0.md) |
| Default-change ADR (policy) | REL-008 / ADR-Q025 criteria | **accepted (policy)** — [ADR-0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) (PR-E02); **flip not executed** |
| Flip + rollback procedure | Runtime change checklist | **documented** — [`docs/R4_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md); execute only after accepted **Promote** report |

**Binding non-claims**

- Product **collection default remains v5** (`capability.collection_default: "v5"`) until a future flip PR completes the checklist. Packs must record `no_default_flip: true`.
- v6 remains **opt-in** collection / conversion; tools auto-detect magic for read/report.
- No R3 engine default flip from this package (engine selection is a separate field window).
- No lossy convert, COL-008 baseline promotion, or public perf certification from field size samples.
- Light wall-time / size samples in a pack are **not** public perf certification.
- **ADR-0008 acceptance alone is not R4 completion** and must not be cited as a runtime default change.

**Base cut:** R2-stable ([`docs/RELEASE_NOTES_R2_STABLE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_STABLE.md), PR-C05) — convert/merge/repack/salvage and E5 v6 opt-in must be available for a meaningful R4 pack.

---

## 2. Preconditions

1. R2-stable honesty accepted on the candidate tree (convert / merge / repack / salvage capability **true**; collection default still **v5**).
2. Offline gate green when claiming lab readiness: `./scripts/ci/offline_gate.sh`.
3. Discoverable native CLI (`prefix/bin`, `target/*/nytprof-dump`, `$NYTPROF_NATIVE_CLI`, or `cargo`).
4. Never put `crates/` on oracle `PERL5LIB`.
5. Prefer dual-sink fixtures for convert round-trips (`fixtures/e4/dual-sink/*`); golden `fixtures/v5/default-calls1/nytprof.out` may refuse strict convert when timestamps are unrepresentable — record honest non-zero `rc` if exercised as an extra profile.

---

## 3. What to collect (minimum evidence set)

Aligned with plan [REL-007](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) / [REL-008](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) and [ADR-Q025](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md):

| Evidence class | Local pack artifact | Notes |
|----------------|---------------------|-------|
| Provenance | `env/provenance.txt`, `summary.json` | OS, uname, Perl `-V` summary, git commit, tool paths |
| Capability honesty | `capability/capability.json` | `v6_decode`/`v6_report`/`convert` true; **`collection_default` must be `"v5"`** |
| v5 escape hatch | `runs/v5_report_*` | Original v5 profile still reports correctly |
| v6 opt-in read/report | `runs/v6_report_*`, `runs/v6_verify_*` | Dual-sink or operator v6 profiles |
| Convert v5→v6 | `runs/convert_to_v6_*` + `artifacts/` | Strict convert; record size bytes |
| Convert v6→v5 | `runs/convert_to_v5_*` + `artifacts/` | Escape hatch for old-tool shape |
| Post-convert semantics | `runs/report_after_convert_*` | Leaf/mid samples when fixture-like |
| Size / overhead | `summary.json` size fields | Engineering only — not public SLOs |
| Issues / severity | report template § Issues | High-severity format/tool issues → no promotion |
| Platform tier | report template + provenance | Eligible tiers for a future ADR |
| Duration / volume | report template header | Suggested window in §4 — not frozen by this pack |

Operator profiles may be supplied as extra paths. **Do not** paste secrets or full proprietary source into public packs; see redaction in the schema.

---

## 4. Suggested field-window parameters (provisional)

These are **engineering defaults for the report**. Promotion **criteria** are binding in [ADR-0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md); calendar duration may still be set by the release lead:

| Parameter | Provisional guidance |
|-----------|----------------------|
| Duration | At least **one** stable R2-stable opt-in cycle after convert/tools ship (calendar length set by release lead) |
| Sites / tiers | ≥1 production-like site per **advertised** OS tier intended for R4 eligibility |
| Workloads | Mix of short CLI and long-running; include fork/recovery cases when COL-015 surfaces are exercised in the field |
| Correctness bar | No unresolved **high** severity event/count/source mismatch between v5 and v6 paths attributable to format tools |
| Convert bar | Document convert usage/failures; strict path must not claim success on refuse cases |
| Escape hatch | `format=v5` / convert `--to=v5` remains usable; old-tool shape after successful v5 convert |
| Rollback bar | One-step force-v5 + product default rollback per [`docs/R4_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md) (`collection_default` back to v5) while retaining v6 **read** |

**Runtime** promotion is gated: accepted field report recommendation **Promote** + flip checklist. Incomplete evidence → **do not flip**.

---

## 5. Commands (copy-paste)

From repo root (or any cwd — scripts resolve the tree):

```sh
# Lab / fixture-backed evidence pack (dual-sink default_calls1 v5+v6)
./scripts/field/r4_field_window_collect.sh \
  --out /tmp/r4-field-pack-lab

# Operator profile(s) in addition to default dual-sink fixtures
./scripts/field/r4_field_window_collect.sh \
  --out /tmp/r4-field-pack-site-a \
  --profile /path/to/redacted-or-local/nytprof.out

# Optional labels for multi-site aggregation
./scripts/field/r4_field_window_collect.sh \
  --out /tmp/r4-field-pack-site-a \
  --site site-a \
  --note "staging workers; format=v6 opt-in only; collection_default still v5"

# Smoke that the collector layout is intact (no default flip)
./scripts/field/r4_field_window_smoke.sh
```

Fill the human report from pack contents:

- Template: [`docs/templates/R4_FIELD_WINDOW_REPORT.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R4_FIELD_WINDOW_REPORT.md)
- Generic evidence bundle (release-scale): [`docs/plan/templates/EVIDENCE_BUNDLE_TEMPLATE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/templates/EVIDENCE_BUNDLE_TEMPLATE.md)

Related lab smokes (already in offline gate; not substitutes for field packs):

```sh
./scripts/packaging/capability_selftest_smoke.sh
# convert / E4 dual-sink product smoke via offline_gate step when native present
./scripts/ci/offline_gate.sh
```

---

## 6. Residual honesty

| Claim | Status |
|-------|--------|
| Field evidence **collection** tools + report template | **ready** (PR-E01) |
| Default-change ADR (ADR-Q025 / REL-008) | **accepted (policy)** — [ADR-0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) (PR-E02) |
| Flip + rollback procedure | **documented** — [`docs/R4_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md) |
| R4 product format **runtime** default flip | **not** done — `collection_default` remains **v5** |
| Charter R4 complete | **not** claimed |
| R3 engine default field window | separate package (PR-D01 when present) |
| Public performance SLOs from field packs | **not** claimed |
| Lossy convert / packing-target convert | **not** claimed |

Residual matrix: collection / format default **runtime** flip remains **not claimed**; policy ADR + field instrumentation are ready; flip gated on accepted **Promote** report.

---

## 7. Exit criteria for the *window* (feeds flip checklist — not auto-promoted)

The field window report is **accepted** when maintainers can answer yes to:

1. Packs from eligible tiers exist with `summary.json` `no_default_flip=true` and `collection_default="v5"`.
2. No open high-severity v6-path correctness / corruption / convert issues for the window.
3. v5 escape hatch and convert `--to=v5` verified on each tier.
4. Tools auto-detect and read v6; capability honesty stays consistent with R2-stable.
5. Report template completed with site list, duration, issue log, size/overhead notes, and recommendation (**promote** / **extend window** / **do not promote**).

Only then may a future PR **execute** the flip per [`docs/R4_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md) under ADR-0008. **This field package never flips defaults.** Incomplete evidence → do not flip.

---

## 8. Links

| Doc | URL |
|-----|-----|
| Program charter R4 | https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md |
| ADR-0008 (promotion policy) | https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md |
| Flip + rollback checklist | https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md |
| R2-stable release notes | https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_STABLE.md |
| R1 residual matrix | https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md |
| Operator runbook | https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md |
| Convert strict MVP | https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/convert-strict-mvp-v0.md |
| CLI E5 v6 opt-in | https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cli-e5-v6-opt-in-mvp-v0.md |
| Acceptance criteria | https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md |
| Rollout REL-007/008 | https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md |
| ADR-Q025 | https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md |
| R3 field window (sibling) | https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md |

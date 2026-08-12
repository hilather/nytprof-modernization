# R3 field-window report — `engine=auto` (template)

**Status to fill:** draft | in-window | accepted | rejected  
**Template version:** v0 (PR-D01)  
**Does not flip defaults.** Promotion policy: [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) (PR-D02). Runtime flip only via [docs/R3_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) after this report is **accepted** with recommendation **Promote**. Incomplete evidence → do not flip.

Guide: [docs/R3_FIELD_WINDOW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md)  
Pack schema: [docs/schemas/r3-field-window-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r3-field-window-mvp-v0.md)  
Flip procedure: [docs/R3_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md)  
Generic evidence bundle: [docs/plan/templates/EVIDENCE_BUNDLE_TEMPLATE.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/templates/EVIDENCE_BUNDLE_TEMPLATE.md)

---

## Identity

| Field | Value |
|-------|-------|
| Report ID | |
| Window start (UTC) | |
| Window end (UTC) | |
| Candidate tree / git commit(s) | |
| Oracle pin (if used) | `baseline/6.15` + manifest hash: |
| Full R1 cut reference | [RELEASE_NOTES_R1.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R1.md) |
| Operator / sites lead | |
| Reviewers | |

**Binding statement:** This report evaluates **opt-in** native reporting and Perl `engine=auto` prefer-native / fall-back-legacy. It does **not** authorize changing the product default engine or format.

---

## Sites and packs

| Site ID | Tier (OS/arch) | Workload class | Pack path | `summary.json` commit | Native discoverable? | Notes |
|---------|----------------|----------------|-----------|------------------------|----------------------|-------|
| | | | | | yes/no | |

Attach or link one pack root per site (output of `scripts/field/r3_field_window_collect.sh`). Prefer `SHA256SUMS` under each pack when publishing.

---

## Environment roll-up

Summarize from each pack’s `env/provenance.txt` / `summary.json`:

| Dimension | Observed values |
|-----------|-----------------|
| OS / arch / libc | |
| Perl version(s) | |
| Rust/Cargo (if native built on-site) | |
| How native CLI was discovered | path / cargo / absent |
| Relevant env (`NYTPROF_*`) — redacted | |

Confirm: no pack used `crates/` on oracle `PERL5LIB`.

---

## Correctness and engine selection

### Lab / fixture baseline (required for lab packs)

On `fixtures/v5/default-calls1/nytprof.out` when native is present. Fill from pack `runs/*` / `summary.json` (collector run ids below).

| Check | Pack run id (when collected) | Expected | Result |
|-------|------------------------------|----------|--------|
| `--engine=auto report` leaf / mid | `engine_auto_report_default-calls1` | **15** / **3**, `rc==0` | |
| `--engine=native report` leaf / mid | `engine_native_report_default-calls1` | **15** / **3**, `rc==0` | |
| `capability --json` | `capability/capability.json` | `ok`/`decode`/`report`/`verify` true | |
| `NYTPROF_FORCE_NO_NATIVE=1` + **auto** report | `engine_auto_force_no_native_report_default-calls1` | **STDERR auto-fallback note** required; **`rc==0` only when** `baseline/6.15/install` (oracle pin install) is present — **honest non-zero** if pin install is absent (same residual as packaging legacy smokes) | |
| Explicit `--engine=native` + force-no-native | `engine_native_force_no_native_report_default-calls1` | **Fail closed:** non-zero `rc`; **no** silent legacy success (no leaf/mid **15**/**3** as a false native win). Matches ENGINE-AUTO-FALLBACK packaging smoke case | |

**Notes (force-no-native honesty)**

- Auto + force-no-native exercises prefer-native **fallback**; it is **not** a hard fail if legacy cannot run because the oracle install tree is missing — record `rc`, `stderr_fallback_note`, and whether `baseline/6.15/install` existed.
- Native + force-no-native must **not** fall back; a non-zero exit is the success contract for this row.
- Optional cross-check (not required from pack alone): `./scripts/packaging/engine_auto_fallback_smoke.sh` when a full oracle pin is available.

Optional extra fixtures:

| Fixture | Check | Expected | Result |
|---------|-------|----------|--------|
| `calls2-default` | `sub_entry_events` (when JSON path used) | **27** | |
| `blocks-calls1` | line5 / block 1:4 (when JSON path used) | **780** / **810** | |

### Operator workloads

| Site | Profile (redacted id) | auto outcome (native/legacy) | Semantic samples checked | Mismatch? | Severity |
|------|------------------------|------------------------------|---------------------------|-----------|----------|
| | | | | yes/no | |

---

## Fallback and escape hatch

| Question | Answer |
|----------|--------|
| Fallback frequency (auto → legacy) | |
| Fallback reasons (missing CLI / install / other) | |
| Did fallback ever hide a corrupt profile as complete? | yes/no — evidence |
| Force-legacy path verified (`--engine=legacy` / `NYTPROF_ENGINE=legacy`) | yes/no |
| One-step rollback for a future default flip documented? | yes/no — see [R3_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) (`--engine=legacy` / `NYTPROF_ENGINE=legacy`) |

---

## Issues log

| ID | Date | Site | Severity (crit/high/med/low) | Surface (report/query/html/…) | Engine path | Summary | Status | Fixture follow-up |
|----|------|------|------------------------------|-------------------------------|-------------|---------|--------|-------------------|
| | | | | | | | open/fixed | |

**Promotion blocker rule:** any **critical/high** correctness, data-loss, or security issue attributable to the native path that remains open at window end → **do not promote**.

---

## Support load and adoption (qualitative)

| Metric (best-effort) | Value / note |
|----------------------|--------------|
| Opt-in sites using `engine=auto` or `engine=native` | |
| Sites remaining on explicit legacy | |
| Support tickets related to engine selection | |
| Downstream report consumers (HTML/CSV/tools) issues | |

---

## Performance / size (optional, non-certifying)

Record only equal-feature comparisons. **Do not** treat these as public SLOs.

| Workload | Metric | Native | Legacy | Notes |
|----------|--------|--------|--------|-------|
| | wall / peak RSS / report bytes | | | light harness only |

Light harness (engineering only): [docs/BENCH_NOTES.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md), `tools/bench/light_bench.sh`.

---

## Residual honesty checklist

Confirm each remains **true** for this report:

- [ ] No claim of R3 product default flip from this report alone  
- [ ] No COL-007 / v6 wire freeze / CLI v6 default claim  
- [ ] No full oracle `nytprofhtml` DOM claim  
- [ ] No public performance certification claim  
- [ ] Rust CLI `auto`→`native` residual acknowledged (Perl facade is dual-path auto surface)  
- [ ] Absolute HTTPS links used if this report is published outside the tree  

---

## Recommendation

| Option | Select one |
|--------|------------|
| **Promote** — run flip checklist in [R3_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) under [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) (policy already landed; runtime flip is a separate change set) | |
| **Extend window** — more sites / duration / fixes required | |
| **Do not promote** — blockers listed above; **do not** execute flip | |

**Rationale (short):**

**Eligible tiers proposed for a future ADR (if promote):**

**Rollback owner (if promote):**

---

## Sign-off

| Role | Name | Date | Signature / ack |
|------|------|------|-----------------|
| Sites lead | | | |
| Release review | | | |
| Compatibility / QA | | | |

---

## Attachments

- Pack directories (paths or archive URLs)  
- `SHA256SUMS` per pack  
- Redacted issue links  
- Optional: filled [EVIDENCE_BUNDLE_TEMPLATE](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/templates/EVIDENCE_BUNDLE_TEMPLATE.md) for release-candidate scale  

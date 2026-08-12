# R3 field-window report — `engine=auto` (template)

**Status to fill:** draft | in-window | accepted | rejected  
**Template version:** v0 (PR-D01)  
**Does not flip defaults.** Promotion requires a separate default-change ADR (PR-D02 / ADR-Q024).

Guide: [docs/R3_FIELD_WINDOW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md)  
Pack schema: [docs/schemas/r3-field-window-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r3-field-window-mvp-v0.md)  
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

On `fixtures/v5/default-calls1/nytprof.out` when native is present:

| Check | Expected | Result |
|-------|----------|--------|
| `--engine=auto report` leaf / mid | **15** / **3** | |
| `--engine=native report` leaf / mid | **15** / **3** | |
| `capability --json` | `ok`/`decode`/`report`/`verify` true | |
| `NYTPROF_FORCE_NO_NATIVE=1` + auto report | exit 0 via legacy; STDERR fallback note | |
| Explicit `--engine=native` + force-no-native | fail closed (no silent legacy) | |

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
| One-step rollback for a future default flip documented? | yes/no (plan only; not implemented here) |

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
| **Promote** — draft PR-D02 default-change ADR | |
| **Extend window** — more sites / duration / fixes required | |
| **Do not promote** — blockers listed above | |

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

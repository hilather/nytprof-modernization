# R4 field-window report — `format=v6` (template)

**Status to fill:** draft | in-window | accepted | rejected  
**Template version:** v0 (PR-E01)  
**Does not flip defaults.** Promotion requires a separate default-change ADR (REL-008 / ADR-Q025).

Guide: [docs/R4_FIELD_WINDOW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_FIELD_WINDOW.md)  
Pack schema: [docs/schemas/r4-field-window-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r4-field-window-mvp-v0.md)  
Generic evidence bundle: [docs/plan/templates/EVIDENCE_BUNDLE_TEMPLATE.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/templates/EVIDENCE_BUNDLE_TEMPLATE.md)  
R2-stable baseline: [docs/RELEASE_NOTES_R2_STABLE.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_STABLE.md)

---

## Identity

| Field | Value |
|-------|-------|
| Report ID | |
| Window start (UTC) | |
| Window end (UTC) | |
| Candidate tree / git commit(s) | |
| R2-stable cut reference | [RELEASE_NOTES_R2_STABLE.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_STABLE.md) |
| Operator / sites lead | |
| Reviewers | |

**Binding statement:** This report evaluates **opt-in** v6 collection/output and R2-stable offline tooling (decode / report / verify / convert). It does **not** authorize changing the product **collection default** from v5 to v6, nor engine defaults (R3).

---

## Sites and packs

| Site ID | Tier (OS/arch) | Workload class | Pack path | `summary.json` commit | `collection_default` | Notes |
|---------|----------------|----------------|-----------|------------------------|----------------------|-------|
| | | | | | must be **v5** | |

Attach or link one pack root per site (output of `scripts/field/r4_field_window_collect.sh`). Prefer `SHA256SUMS` under each pack when publishing.

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
| Capability `collection_default` | must remain **v5** |

Confirm: no pack used `crates/` on oracle `PERL5LIB`. Confirm no pack claims a product default flip.

---

## Correctness and format tooling

### Lab / fixture baseline (required for lab packs)

On dual-sink `fixtures/e4/dual-sink/default_calls1_{v5,v6}.nytprof` when native is present. Fill from pack `runs/*` / `summary.json`.

| Check | Pack run id (when collected) | Expected | Result |
|-------|------------------------------|----------|--------|
| `capability --json` | `capability/capability.json` | `ok`/`v6_decode`/`v6_report`/`convert` true; **`collection_default: "v5"`** | |
| v5 `report` leaf / mid | `v5_report_default_calls1_v5` | **15** / **3**, `rc==0` | |
| v6 `report` leaf / mid | `v6_report_default_calls1_v6` | **15** / **3**, `rc==0` | |
| v6 `verify` | `v6_verify_default_calls1_v6` | `rc==0` | |
| `convert --to=v6` (dual-sink v5) | `convert_to_v6_default_calls1_v5` | `rc==0` | |
| `report` after convert→v6 | `report_after_convert_to_v6_default_calls1_v5` | **15** / **3**, `rc==0` | |
| `convert --to=v5` (dual-sink v6) | `convert_to_v5_default_calls1_v6` | `rc==0` | |
| `report` after convert→v5 | `report_after_convert_to_v5_default_calls1_v6` | **15** / **3**, `rc==0` | |

**Notes (convert honesty)**

- Strict convert may **refuse** some real/golden v5 profiles (e.g. fractional timestamps) — record `rc` and stderr; do not treat refuse as silent success.
- Dual-sink pairs are the lab convert contract for this pack MVP.
- Optional cross-check: offline_gate E4 product smoke / convert packaging paths when native present.

Optional extra fixtures:

| Fixture | Check | Expected | Result |
|---------|-------|----------|--------|
| `calls2_default` dual-sink | `sub_entry` / report when used | per E4 meta | |
| `blocks_calls1` dual-sink | line/block samples when used | per E4 meta | |
| Operator v6 profile | verify + report | complete; no high-severity mismatch | |

### Operator workloads

| Site | Profile (redacted id) | Format (v5/v6) | Tools used (report/convert/…) | Semantic samples checked | Mismatch? | Severity |
|------|------------------------|----------------|-------------------------------|---------------------------|-----------|----------|
| | | | | | yes/no | |

---

## Convert, escape hatch, and size

| Question | Answer |
|----------|--------|
| Convert v5→v6 usage / failure rate | |
| Convert v6→v5 (old-tool escape) verified? | yes/no |
| Did convert ever produce a “success” that failed verify/report? | yes/no — evidence |
| Profile size trend (v6 vs v5) — engineering only | see pack `sizes` |
| One-step rollback for a future default flip documented? | yes/no (plan only; not implemented here) |
| Does capability still report `collection_default: v5`? | **must yes** for this window |

---

## Issues log

| ID | Date | Site | Severity (crit/high/med/low) | Surface (report/convert/verify/…) | Format path | Summary | Status | Fixture follow-up |
|----|------|------|------------------------------|-----------------------------------|-------------|---------|--------|-------------------|
| | | | | | | | open/fixed | |

**Promotion blocker rule:** any **critical/high** correctness, data-loss, corruption, or security issue attributable to the v6 path / convert tooling that remains open at window end → **do not promote**.

---

## Support load and adoption (qualitative)

| Metric (best-effort) | Value / note |
|----------------------|--------------|
| Opt-in sites using `format=v6` or convert→v6 | |
| Sites remaining on v5-only | |
| Support tickets related to format / convert | |
| Mixed-team friction (old tools vs v6) | |
| Downstream consumers of converted v5 | |

---

## Performance / size (optional, non-certifying)

Record only equal-feature comparisons. **Do not** treat these as public SLOs.

| Workload | Metric | v5 | v6 | Notes |
|----------|--------|----|----|-------|
| | profile bytes / wall / peak RSS | | | light harness only |

Light harness (engineering only): [docs/BENCH_NOTES.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md), `tools/bench/light_bench.sh`.

---

## Residual honesty checklist

Confirm each remains **true** for this report:

- [ ] No claim of R4 product format default flip from this report alone  
- [ ] `collection_default` remains **v5** in capability on every pack  
- [ ] No R3 engine product default flip claim  
- [ ] No lossy convert / packing-target convert claim  
- [ ] No COL-008 baseline promotion claim  
- [ ] No public performance certification claim  
- [ ] Absolute HTTPS links used if this report is published outside the tree  

---

## Recommendation

| Option | Select one |
|--------|------------|
| **Promote** — draft default-format ADR (ADR-Q025 / REL-008) | |
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

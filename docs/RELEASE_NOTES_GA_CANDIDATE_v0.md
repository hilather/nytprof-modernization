# Release notes — GA-candidate drop-in honesty cut (PR-P01)

**Status:** **GA-candidate MVP** — **not** final GA marketing  
**Date:** 2026-08-13  
**Board ID:** `P01-GA-CANDIDATE`  
**Identity:** **`Devel::NYTProf` ≥ 7.00** (J01 `VERSION_FROM`)  
**Does not supersede:** [PROGRAM_CHARTER.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md), ADRs 0001–0010, [DROP_IN_DOD_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md)

This cut advertises **collection drop-in preview** on **advertised flavors** only. It is not SEC-012 complete, not uploaded to PAUSE, and does not flip R3/R4 defaults.

---

## Advertised flavors

| Flavor | Claim | Evidence |
|--------|-------|----------|
| **CPAN / source D1-A** | Attach-preview with optional `format=v6` when built `xs-nytprof-v6` / `NYTPROF_V6_COLLECT` (EVENT path). Multi-kind v6 collection is **EVENT-only honesty** unless E3-mixed ships. | G05 D1-A `NYTPROF6`; `collection_default` remains **v5** |
| **Rocky / EL8 default RPM = D1-B only** | K01 `perl-Devel-NYTProf` is **v5-only** (`-lz`). K01 ≠ full D1-A. `PRODUCT-V6-COLLECT-EL8` stays **residual**. | [perl-Devel-NYTProf.spec](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/perl-Devel-NYTProf.spec) |
| **Tools companion** | `nytprof-cli` RPM is **not** drop-in collection (K02 / ADR-0010). | [nytprof-cli.spec](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/nytprof-cli.spec) |

Live `-d:NYTProf` with `file=` (D1-B) still yields shipped report leaf **15** / mid **3** / mid→leaf **15** on the default-calls1-shaped workload.

---

## Residuals operators hit

| Residual | Honesty |
|----------|---------|
| tablesorter / shared JS **WAIVE** | M01 — jquery **not** shipped; native HTML is MVP |
| Not full 6.15 opcode / `entersub` | Attach-MVP is `DB::sub` / `DB::DB` |
| `collection_default` **v5** | No R4 flip |
| COMPAT-007 bless-array Data | Residual |
| Merge aggregate-sum vs `nytprofmerge` | Residual (stream-concat MVP only) |
| Signed CI prebuilt **pipeline** | Residual (K02 spec only) |
| `PRODUCT-V6-COLLECT-EL8` | Residual — Rocky default is D1-B |
| `BUILD-003-FULL` | Residual |
| S2 dual-path rewrite | Residual — `dual_path_smoke` stays oracle-primary |
| **SEC-012** complete GA marketing | **Not claimed** (P02 is checklist / job MVP only; not independent sign-off) |
| R3 / R4 runtime defaults | **Not flipped** |
| PAUSE upload | **Not uploaded to PAUSE** |

Migration: [MIGRATION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/MIGRATION_DROP_IN_v0.md). TRIAL notes: [RELEASE_NOTES_CPAN_TRIAL_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_CPAN_TRIAL_v0.md).

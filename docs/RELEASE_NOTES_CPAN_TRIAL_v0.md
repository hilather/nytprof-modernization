# Release notes — NYTProfM 6.15 CPAN TRIAL (attach-preview)

**Status:** **notes-ready MVP** (PR-J02) — **not** uploaded to PAUSE  
**Date:** 2026-08-12  
**Board ID:** `CPAN-TRIAL-READY` (notes-ready / MVP)  
**Identity:** CPAN **`NYTProfM`**, module **`Devel::NYTProfM`**, product `$VERSION` **6.15** (KD-16/17 Option B; J01 `VERSION_FROM`)  
**Does not supersede:** [PROGRAM_CHARTER.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md), ADRs 0001–0010, [DROP_IN_DOD_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md)

These notes describe a **collection preview** (attach-preview) for operators evaluating product `-d:NYTProfM` from this tree. Operators switch from stock `-d:NYTProf`. They are not uploaded to PAUSE (no `cpan-upload`, no CPAN index). This is not a full GA drop-in claim.

---

## What this preview is

| Claim | Meaning |
|-------|---------|
| **Attach-preview** | Product `perl -d:NYTProfM` with `NYTPROF file=` writes `NYTProf 5`; shipped dump/report of those bytes is leaf **15** / mid **3** / mid→leaf **15** on the default-calls1-shaped workload |
| **CPAN identity** | `NYTProfM` / `Devel::NYTProfM` **6.15** from root `Makefile.PL` `DISTNAME` + `VERSION_FROM` [`collector/xs/Devel/NYTProfM.pm`](https://github.com/hilather/nytprof-modernization/blob/main/collector/xs/Devel/NYTProfM.pm) |
| **`collection_default`** | **v5** (capability JSON; no R4 flip) |
| **Dist listing** | `make manifest` excludes `baseline/`, `target/`, `prefix/` (J01) |

Familiar script names after I03: `nytprof-engine` / `nytprofhtml` / `nytprofcsv` in the product prefix. Native CLI remains optional (`NYTPROF_NATIVE`).

---

## Residuals operators hit (day-one honesty)

| Residual | Operator impact |
|----------|-----------------|
| **Not full 6.15 opcode / `entersub`** | Attach-MVP is Perl `DB::sub` / `DB::DB`, not the 6.15 opcode profiler |
| **Name switch** | Product is **`NYTProfM` / `-d:NYTProfM`**. Stock/oracle remains **`Devel::NYTProf` / `-d:NYTProf`** |
| **tablesorter / shared JS `WAIVE`** | Native HTML is MVP (CSS + excl). GA-candidate does **not** ship jquery/tablesorter (M01) |
| **`collection_default` v5** | `format=v6` is flavor-gated; D1-B fail-closes with the `v6_collect` rebuild string |
| **Rocky / EL8 default RPM** | **Residual** (`EL8-RPM-MODULE`). When it exists it is **D1-B** (v5-only) unless `--with v6_collect` |
| **Not PAUSE uploaded** | No `cpan-upload`; no indexed TRIAL tarball from this cut |
| **Not `BUILD-003-FULL`** | MakeMaker is still a packaging entry (`full_build003=0`) |
| **S2 dual-path** | `dual_path_smoke.sh` stays oracle-primary (`legacy_only` first half) |
| Mid-deflate-in-child, full TEST-018, blocks-calls1 line5 **780** | Still residual |

Migration: [MIGRATION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/MIGRATION_DROP_IN_v0.md). Operator runbook: [R1_PREVIEW_OPERATOR_RUNBOOK.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md).

---

## Upgrade / rollback (preview)

- Keep `format=v5` unless you have a D1-A / `v6_collect` build.
- Rollback collection: uninstall the product prefix / `NYTPROF=` unset; oracle 6.15 pin remains under `baseline/6.15/install`.
- Never put `crates/` on oracle `PERL5LIB`.

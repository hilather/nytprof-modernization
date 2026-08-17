# NYTProfM v0.2.13 — fail-closed `nodebug_stash` (no attach SEGV)

**Tag:** `v0.2.13`  
**Date:** 2026-08-17  
**Since:** [`v0.2.12`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.12)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-7** (upgrades `6.15-6`). **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.13/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.13/scripts/packaging/build_el8_module_rpm.sh).

```text
sudo rpm -Uvh perl-NYTProfM-6.15-7.el8.x86_64.rpm
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM script.pl
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
```

## The change

v0.2.12 `DB::nodebug_stash` (hint-magic `CvNODEBUG` walk) called `GvCV` after `isGV` only. On Perl 5.26 and 5.38, `GvCV` is `GvGP(gv)->gp_cv`. A GP-less stash glob (`isGV` true — `Package::Stash` / `namespace::clean` leave those) is a **NULL deref**: `Segmentation fault (core dumped)` on `file=` attach. This cut walks via `isGV_with_GP`, skips XSUB CVs before treating `CvROOT` as an `OP*`, and plants that slot in a regression smoke.

## Changes since v0.2.12 (grouped)

**Perl attach (collector)**

- `product_stash_val_cv`: `isGV_with_GP` before `GvCV`; also accept a CV (or RV to a CV) stored directly in the stash.
- `product_nodebug_stash` and `product_rebind_stash_slowops` both use that helper.
- `product_rebind_cv` returns on `CvISXSUB` (`CvROOT`/`CvXSUB` share a union).
- `product_fill_cv_name` ignores a `CvGV` without a GP.
- Missing `$DB::sub` dies instead of a silent `return` from `DB::sub`.
- Smoke: [`g11_nodebug_stash_nogp_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.13/scripts/packaging/g11_nodebug_stash_nogp_smoke.sh). Test: [`t/nodebug_stash_nogp.t`](https://github.com/hilather/nytprof-modernization/blob/v0.2.13/t/nodebug_stash_nogp.t).

**Packaging**

- Module RPM **6.15-7**. Prebuilt `nytprof-cli` ELF unchanged (source hash still matches; the fix is `.pm` + XS).

## Known residuals (unchanged)

- Exclusive seconds vs 6.15 are **not** a gate (`AGENTS.md` §5). Native HTML is MVP (no tablesorter / full DOM).
- FFI, XS Data, COL-007 C v6 writer, multi-OS product certification: [residual matrix](https://github.com/hilather/nytprof-modernization/blob/v0.2.13/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md).
- Full opcode / `entersub` / XSUB attach is still residual.

## Upgrade notes

- `dnf upgrade` / `rpm -Uvh` to **6.15-7** to pick up the attach `.so`. Scripts that already set `NYTPROF=file=…` need no flag change.
- If v0.2.12 / 6.15-6 core-dumped on a real app (DateTime / namespace::clean / Package::Stash graphs), re-profile with this RPM (or in-tree `xs-nytprof`).
- Field lab default HTML remains `--no-flame`; operator `nytprofm-cli html` still defaults flame **on** (v0.2.11).

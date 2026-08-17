# NYTProfM v0.2.12 — attach survival on DateTime / Rex / CPAN graphs

**Tag:** `v0.2.12`  
**Date:** 2026-08-17  
**Since:** [`v0.2.11`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.11)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-6** (upgrades `6.15-5`). **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.12/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.12/scripts/packaging/build_el8_module_rpm.sh).

```text
sudo rpm -Uvh perl-NYTProfM-6.15-6.el8.x86_64.rpm
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM script.pl
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
```

## The change

Live `perl -d:NYTProfM` now survives real CPAN graphs that used to die at compile (DateTime / `namespace::autoclean` / `B::Hooks::EndOfScope::XS`, Rex `Shared::Var`, `XSLoader::load` looking for `DB.so`). The field lab catalogs **20** apps and ships bounded drivers for a **dependency-diverse top 10**, with optional `--engine both` vs pinned 6.15 as an **attach-survival** check (not exclusive-time match).

## Changes since v0.2.11 (grouped)

**Perl attach (collector)**

- Do **not** wrap `CORE::require` in `CORE::GLOBAL::require` (extra frame broke compile-time `%^H` / Variable::Magic).
- Preload `B::Hooks::EndOfScope` / `Variable::Magic` / `namespace::*` and `CvNODEBUG` their CVs **before** `$^P` 0x01. Do **not** defer 0x01 to `INIT` (subs compiled without `PERLDBf_SUB` never call `DB::sub` — g04 15/3/15 went to 0).
- `goto &$raw` for every `::import` / `::unimport` (inherited `Exporter::import` is named `Child::import` in `$DB::sub` — Rex `share qw(@SUMMARY)`).
- `goto` `XSLoader::` / `DynaLoader::` (`XSLoader::load()` with no args uses `caller` as the module name; wrap looked for `DB.so` on `use Fcntl`).
- Resolve imported aliases (`main::task` → defining package) before the goto list.
- Also goto `Moo::` / `Moose::` / `Class::` / `Rex::` / `DateTime::`.
- Smoke: [`g10_datetime_hints_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.12/scripts/packaging/g10_datetime_hints_smoke.sh).

**Field / catalog**

- 20-app catalog + 10 distinct primary families: [`complex-app-catalog-v0.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.12/docs/schemas/complex-app-catalog-v0.md).
- Rocky lab [`complex_app_docker_profile.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.12/scripts/field/complex_app_docker_profile.sh) `--app ID` / `--engine native|oracle|both`. Gate is success token + `NYTProf 5`. HTML `--no-flame` (DateTime loops emit millions of `TIME_LINE`s).
- Fail-closed helper [`attach_survival.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.12/scripts/field/lib/attach_survival.sh) (`as an ARRAY ref`, `EndOfScope/XS.pm`, `loadable object for module DB`).
- Findings: [`complex-app-findings-v0.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.12/docs/schemas/complex-app-findings-v0.md). All ten native attaches survived this cut; Rex also survived 6.15 `--engine both`. Residual: DBI `disconnect` warns about an active statement handle (not an attach-kill).

**Tests / CI**

- `t/complex_app_catalog.t` (20 rows, 10 unique families, top-10 drivers).
- `t/attach_survival_failclosed.t` drives the shipped helper.
- GHA job **Complex-app Docker lab (Rex)** (`--engine both`, timeout 75m). **Not** `offline_gate`.

**Packaging**

- Module RPM **6.15-6**. Prebuilt `nytprof-cli` ELF unchanged (source hash still matches; attach fixes are `.pm` + XS).

## Known residuals (unchanged)

- Exclusive seconds vs 6.15 are **not** a gate (`AGENTS.md` §5). Native HTML is MVP (no tablesorter / full DOM).
- FFI, XS Data, COL-007 C v6 writer, multi-OS product certification: [residual matrix](https://github.com/hilather/nytprof-modernization/blob/v0.2.12/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md).

## Upgrade notes

- `dnf upgrade` / `rpm -Uvh` to **6.15-6** to pick up the attach `.pm`+`.so`. Scripts that already set `NYTPROF=file=…` need no flag change.
- DateTime / Rex / Moose-style apps that died at compile under 6.15-5 should be re-profiled with this RPM (or in-tree `xs-nytprof`).
- Field lab default HTML is `--no-flame`; operator `nytprofm-cli html` still defaults flame **on** (v0.2.11).

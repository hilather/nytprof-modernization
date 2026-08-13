# CPAN dist hygiene MVP (v0)

**Board ID:** `J01-CPAN-HYGIENE`  
**Status:** **done (MVP)** — shipped MakeMaker identity is **`NYTProfM` / `Devel::NYTProfM` 6.15**; MANIFEST excludes `baseline/` / `target/` / `prefix/`.  
**Not:** `CPAN-TRIAL-READY` / PAUSE upload / J02 TRIAL notes; not `BUILD-003-FULL`; not S2; not EL8 RPM.

**Configure:** root [`Makefile.PL`](https://github.com/hilather/nytprof-modernization/blob/main/Makefile.PL)  
**Version source:** [`collector/xs/Devel/NYTProfM.pm`](https://github.com/hilather/nytprof-modernization/blob/main/collector/xs/Devel/NYTProfM.pm) (`VERSION_FROM`; KD-16)  
**Skip file:** [`MANIFEST.SKIP`](https://github.com/hilather/nytprof-modernization/blob/main/MANIFEST.SKIP)  
**Smoke:** [`scripts/packaging/j01_cpan_hygiene_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/j01_cpan_hygiene_smoke.sh)

## Identity

| Field | Value | Source |
|-------|--------|--------|
| CPAN name | `NYTProfM` (META; module `Devel::NYTProfM`) | `WriteMakefile(NAME => 'Devel::NYTProfM', DISTNAME => 'NYTProfM')` |
| Version | **6.15** (match `baseline/6.15` pin) | `VERSION_FROM` the debugger `.pm` — not a smoke-only string, not `0.001` / not `7.00` |
| Old facade | `NYTProf::Modernization::PackagingEntry` / `0.001` | **retired as advertised NAME/VERSION** |

`x_nytprof_modernization.cpan_trial_ready` stays **0**. Stamp `packaging_j01=1`, `full_build003=0`.

## Dist listing

`make manifest` (ExtUtils::Manifest + `MANIFEST.SKIP`) must **not** list:

- `baseline/`
- `target/`
- `prefix/`
- `crates/` (never on oracle `PERL5LIB`; also not a dist payload)

Must list `collector/xs/Devel/NYTProfM.pm` (the `VERSION_FROM` file).

Cargo is not required. No PAUSE upload.

## Residuals

| Residual | Status |
|----------|--------|
| `CPAN-TRIAL-READY` | **residual** (J02) |
| `BUILD-003-FULL` | **residual** |
| S2 dual_path rewrite | **residual** |
| EL8 RPM | **residual** |

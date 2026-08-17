# NYTProfM v0.2.16 — exclusive nest split, `stmts=0`, “Profile of $0”

**Tag:** `v0.2.16`  
**Date:** 2026-08-17  
**Since:** [`v0.2.15`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.15)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-10** (upgrades `6.15-9`). **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.16/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.16/scripts/packaging/build_el8_module_rpm.sh).

```text
sudo rpm -Uvh perl-NYTProfM-6.15-10.el8.x86_64.rpm
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM script.pl
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
```

## The change

Three attach/report fixes from field use of v0.2.15:

1. **Exclusive times** were `incl − Σ child exclusive`, so grandchild time leaked into the parent (`lab_run` excl ≈ YAML). Exclusive is now **`incl − Σ child inclusive`**.
2. **`nytprof.out` size** — default is still a `TIME_LINE` per statement (not automatically smaller than 6.15). **`stmts=0`** now skips `TIME_LINE` (sub times remain). `savesrc=0` still drops source copies; omitted `compress` is zlib-6.
3. **HTML “Profile of Config_heavy.pl”** — product did not write `ATTRIBUTE application=$0`, and the primary-fid heuristic treated EL8 `/usr/lib64/perl5/…/Config_heavy.pl` as a user `.pl`. We emit `$0` (6.15) and treat `/lib64/perl` + the `Config_heavy.pl` basename as INC.

## Changes since v0.2.15 (grouped)

**Perl attach (collector)**

- Parent stack credits child **inclusive**. Smoke: [`g14_nested_excl_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.16/scripts/packaging/g14_nested_excl_smoke.sh).
- `stmts=0` skips `DB::DB` `TIME_LINE`. Default remains `stmts=1`.
- `ATTRIBUTE application=$0` at `file=` enable.

**HTML / CLI**

- `is_inc_ish_path` includes `/lib64/perl` and `Config_heavy.pl`. Re-render an old profile to drop a wrong `Config_heavy.pl` title; a **new** attach is needed for `$0` in the header.
- EL8 prebuilt `nytprof-cli` refreshed (report crate changed).

**Packaging**

- Module RPM **6.15-10**.

## Known residuals (unchanged)

- Exclusive seconds vs 6.15 are **not** a gate (`AGENTS.md` §5). Native HTML is MVP (no tablesorter / full DOM).
- `DateTime::` / `Moo::` / `Moose::` / `Class::` / `Rex::` stay `goto` (attach survival) — those CVs still have no own `SUB_RETURN` rows.
- FFI, XS Data, COL-007 C v6 writer, multi-OS product certification: [residual matrix](https://github.com/hilather/nytprof-modernization/blob/v0.2.16/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md).

## Upgrade notes

- `dnf upgrade` / `rpm -Uvh` to **6.15-10** for the attach `.pm`+`.so` and the refreshed `nytprofm-cli`.
- Re-profile for exclusive-split + `application=$0`. Re-html an old `nytprof.out` is enough to stop titling `Config_heavy.pl`.
- Smaller files: `NYTPROF=file=…:savesrc=0:stmts=0`.
- Field lab default HTML remains `--no-flame`; operator `nytprofm-cli html` still defaults flame **on** (v0.2.11).

# NYTProfM v0.2.15 — logger `caller` is the app, not `NYTProfM.pm`

**Tag:** `v0.2.15`  
**Date:** 2026-08-17  
**Since:** [`v0.2.14`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.14)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-9** (upgrades `6.15-8`). **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.15/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.15/scripts/packaging/build_el8_module_rpm.sh).

```text
sudo rpm -Uvh perl-NYTProfM-6.15-9.el8.x86_64.rpm
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM script.pl
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
```

## The change

Workload wrap used `eval { &$raw }` so a die still emitted `SUB_RETURN`. `caller()` skips **package-DB sub** frames but **not** `CXt_EVAL`, so every logger that does `caller(0)` reported **`Devel/NYTProfM.pm:308`** instead of the app file. This cut calls `&$raw` directly; a `DESTROY` guard still emits if the callee dies.

## Changes since v0.2.14 (grouped)

**Perl attach (collector)**

- No `eval` around `&$raw`. `DB::ProductWrapGuard` finishes the frame on die.
- Smoke: [`g13_logger_caller_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.15/scripts/packaging/g13_logger_caller_smoke.sh). Test: [`t/logger_caller_attach.t`](https://github.com/hilather/nytprof-modernization/blob/v0.2.15/t/logger_caller_attach.t).

**Packaging**

- Module RPM **6.15-9**. Prebuilt `nytprof-cli` ELF unchanged (source hash still matches; the fix is `.pm`).

## Known residuals (unchanged)

- Exclusive seconds vs 6.15 are **not** a gate (`AGENTS.md` §5). Native HTML is MVP (no tablesorter / full DOM).
- FFI, XS Data, COL-007 C v6 writer, multi-OS product certification: [residual matrix](https://github.com/hilather/nytprof-modernization/blob/v0.2.15/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md).
- Full opcode / `entersub` / XSUB attach is still residual.

## Upgrade notes

- `dnf upgrade` / `rpm -Uvh` to **6.15-9** to pick up the attach `.pm`. Scripts that already set `NYTPROF=file=…` need no flag change.
- If v0.2.14 / 6.15-8 made every log line show `NYTProfM.pm`, re-profile with this RPM (or in-tree `xs-nytprof`).
- Field lab default HTML remains `--no-flame`; operator `nytprofm-cli html` still defaults flame **on** (v0.2.11).

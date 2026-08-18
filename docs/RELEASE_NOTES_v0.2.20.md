# NYTProfM v0.2.20 — default `slowops=2` is the 6.15 full table

**Tag:** `v0.2.20`  
**Date:** 2026-08-18  
**Since:** [`v0.2.19`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.19)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-14**. **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA. **Not** a certified perf claim.

Default `perl -d:NYTProfM` now profiles the same slow-opcode set as Devel::NYTProf **6.15** (`open`, `readline`, `subst`, `stat`, `sleep`, `printf`, … as `pkg::CORE:op`). Side-by-side HTML no longer hides those rows in the parent exclusive. Exclusive time is still **thin** (not 6.15 savestack).

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.20/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.20/scripts/packaging/build_el8_module_rpm.sh).

```text
sudo rpm -Uvh perl-NYTProfM-6.15-14.el8.x86_64.rpm
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM script.pl
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
```

## Changes since v0.2.19

**CLI / attach**

- Default omit / `slowops=2` installs the copied 6.15 [`slowops.h`](https://github.com/hilather/nytprof-modernization/blob/v0.2.20/collector/xs/slowops.h) table (`pkg::CORE:op`). `slowops=full` / `=3` are aliases. `slowops=0` still off; `slowops=1` still fail-closed.
- XS BOOT installs the full table so compile-time PRINT/MATCH/open/… copy `pp_slowop_profiler`. Emit stays off until `file=` + INIT.
- Smoke: [`g19_slowops_full_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.20/scripts/packaging/g19_slowops_full_smoke.sh) (default must emit `CORE:stat` / `sleep` / `prtf`); [`t/slowops_full_attach.t`](https://github.com/hilather/nytprof-modernization/blob/v0.2.20/t/slowops_full_attach.t); g08 / g09 still green.

**CI**

- Linux matrix `apt-get update` has a 2-minute / 5-minute step cap so `offline_gate` cannot hang the 90-minute job ([`ci-matrix.yml`](https://github.com/hilather/nytprof-modernization/blob/v0.2.20/.github/workflows/ci-matrix.yml)).

**Packaging**

- Module RPM **6.15-14**. Bundled EL8 `nytprofm-cli` unchanged (collector-only cut; source-sha256 matches).

## Known residuals

- Exclusive on the full table is **thin** (not 6.15 savestack). A slowop that re-enters Perl (`sort`, backtick, regex that calls methods) can still look too exclusive. Do not claim 6.15 exclusive seconds.
- Live `calls=2` `sub_entry_events` is **21** (emit after INIT); oracle golden **27** is `start=begin`.
- Product `leave` default stays **0** (not 6.15 `leave=1`).
- `collection_default` stays **v5**.
- Exclusive seconds vs 6.15 HTML are **not** a gate ([`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.20/AGENTS.md) §5).
- FFI, XS Data, COL-007 C v6 writer, multi-OS product certification: [residual matrix](https://github.com/hilather/nytprof-modernization/blob/v0.2.20/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md).

## Upgrade notes

- `dnf upgrade` / `rpm -Uvh` to **6.15-14**.
- Re-profile. Default HTML now lists `CORE:open` / `readline` / `subst` / … instead of folding those seconds into the parent.
- To disable slowops: `NYTPROF=file=…:slowops=0`.
- Rollback attach: `NYTPROF=file=…:wrap=1`.

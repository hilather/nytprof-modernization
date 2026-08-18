# NYTProfM v0.2.18 — EL8 dist includes opcode graft sources

**Tag:** `v0.2.18`  
**Date:** 2026-08-18  
**Since:** [`v0.2.17`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.17)

**EL8 RPM:** 6.15-12 never attached (`%check` counted `SUB_CALLERS` tags, not `count`). Operators install [`v0.2.19`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.19) / **6.15-13**.

Unsigned **Rocky 8 / EL8** testdrive was intended as RPM **6.15-12**. **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA.

[`v0.2.17`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.17) shipped the opcode attach + `SUB_CALLERS` aggregator on `main`, but the EL8 RPM job failed: staged `NYTProfM-6.15.tar.gz` listed only `NYTProf.xs` + `Devel/` and omitted `nytprof_pp.h` / `pp_entersub.c` / `pp_leave.c` / `product_callers.c` / `slowops.h`. **Use this tag for the RPM.** Attach behavior is the same as v0.2.17.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.18/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.18/scripts/packaging/build_el8_module_rpm.sh).

```text
sudo rpm -Uvh perl-NYTProfM-6.15-12.el8.x86_64.rpm
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM script.pl
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
```

## Changes since v0.2.17

**Packaging**

- [`make_nytprofm_dist.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.18/scripts/packaging/make_nytprofm_dist.sh) copies the whole `collector/xs/` tree.
- [`rpm01_make_dist_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.18/scripts/packaging/rpm01_make_dist_smoke.sh) asserts the graft members exist in the staged tarball and that the tag workflow NEVRA matches spec `Release`.
- [`.github/workflows/release-el8-rpm.yml`](https://github.com/hilather/nytprof-modernization/blob/v0.2.18/.github/workflows/release-el8-rpm.yml) derives the module RPM filename from spec `Release` (no leftover `6.15-10` pin) and uses `docs/RELEASE_NOTES_${TAG}.md`.
- Module RPM **6.15-12** (job failed; use **6.15-13**).

**Attach (unchanged from v0.2.17)**

See [`docs/RELEASE_NOTES_v0.2.17.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.18/docs/RELEASE_NOTES_v0.2.17.md): default C `OP_ENTERSUB` + `OP_GOTO`; `wrap=1` escape; `SUB_CALLERS` aggregated at finish; `leave=1` / `slowops=full` opt-in.

## Known residuals

Same as v0.2.17. `collection_default` stays **v5**. No certified perf claim.

## Upgrade notes

- `dnf upgrade` / `rpm -Uvh` to **6.15-13** (6.15-11 and 6.15-12 never attached).
- Rollback attach: `NYTPROF=file=…:wrap=1`.

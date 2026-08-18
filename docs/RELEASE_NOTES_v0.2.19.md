# NYTProfM v0.2.19 — EL8 `%check` sums `SUB_CALLERS.count`

**Tag:** `v0.2.19`  
**Date:** 2026-08-18  
**Since:** [`v0.2.18`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.18)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-13**. **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA.

[`v0.2.18`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.18) staged the opcode graft sources, but EL8 `%check` died: `t/installed_attach.t` incremented mid→leaf once per `c` tag and skipped `count`, so finish-flush aggregation looked like `CALLERS=1 want 15`. **Use this tag for the RPM.** Attach behavior is the same as v0.2.17.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.19/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.19/scripts/packaging/build_el8_module_rpm.sh).

```text
sudo rpm -Uvh perl-NYTProfM-6.15-13.el8.x86_64.rpm
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM script.pl
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
```

## Changes since v0.2.18

**Packaging / `%check`**

- [`t/installed_attach.t`](https://github.com/hilather/nytprof-modernization/blob/v0.2.19/t/installed_attach.t) sums `SUB_CALLERS.count` for the 15/3/15 mid→leaf bar, and finds `START_DEFLATE` by walking tags (not `index(..., 'z')`, which matches fid paths such as `modernization`).
- [`rpm01_make_dist_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.19/scripts/packaging/rpm01_make_dist_smoke.sh) runs staged `installed_attach.t` against a fake prefix (same layout as spec `%install`).
- Module RPM **6.15-13**.

**Attach (unchanged from v0.2.17)**

See [`docs/RELEASE_NOTES_v0.2.17.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.19/docs/RELEASE_NOTES_v0.2.17.md): default C `OP_ENTERSUB` + `OP_GOTO`; `wrap=1` escape; `SUB_CALLERS` aggregated at finish; `leave=1` / `slowops=full` opt-in.

## Known residuals

Same as v0.2.17. `collection_default` stays **v5**. No certified perf claim.

## Upgrade notes

- `dnf upgrade` / `rpm -Uvh` to **6.15-13** (6.15-11 and 6.15-12 never attached).
- Rollback attach: `NYTPROF=file=…:wrap=1`.

# NYTProfM v0.2.6 — Rocky 8 testdrive RPM

**Tag:** `v0.2.6`  
**Date:** 2026-08-13  
**Since:** [`v0.2.5`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.5)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` package for operator test-drive. **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.6/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.6/scripts/packaging/build_el8_module_rpm.sh).

Download `perl-NYTProfM-6.15-1.el8.x86_64.rpm` from this release. Install:

```text
sudo rpm -Uvh --replacefiles perl-NYTProfM-6.15-1.el8.x86_64.rpm
# --replacefiles overwrites stock /usr/bin/nytprofhtml if perl-Devel-NYTProf is present
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM your_script.pl
nytprofhtml /tmp/nytprof.out --out-dir /tmp/nytprof-html
```

## What is in the RPM

| Piece | Notes |
|-------|--------|
| `perl -d:NYTProfM` | Product debugger, `$VERSION` **6.15**, Option B (operators switch) |
| `nytprofhtml` / `nytprofcsv` / `nytprofcg` | Product wrappers → `nytprof-engine` |
| `nytprof-cli` | Unsigned Rocky 8 ELF (sibling of the wrappers) |
| D1-B | v5-only, `-lz`; `format=v6` fail-closed |

Does **not** `Provides: perl(Devel::NYTProf)`.

## Changes since v0.2.5 (grouped)

**Collection (live `perl -d:NYTProfM`)**

- `blocks=1` TIME_BLOCK + resolved-fid line5 **780** / block_line **810**
- `calls=2` **27** `SUB_ENTRY` + `CORE:print` / `CORE:match`
- Product M4-mini projected kinds (not full `compare_jsonl`)
- `sigexit=1` TERM flush (`_exit` still residual)
- `compress=1` START_DEFLATE; `slowops=0/1/2` policy

**Identity / docs**

- Option B: `NYTProfM` / `Devel::NYTProfM` **6.15** / `-d:NYTProfM` / `perl-NYTProfM`
- Operator migration + living DoD header updated; frozen rev-4 KD body stays historical

**Packaging**

- Staged `NYTProfM-6.15.tar.gz`; `%check` on installed attach + scripts
- I03 wrappers + bundled Rocky 8 `nytprof-cli` in the **module** RPM
- Unsigned internal yum bootstrap docs (`gpgcheck=0` is not production policy)
- Mock runner SKIP when mock is absent (this testdrive RPM is `rpmbuild` in `rockylinux:8`)

## Residuals (do not claim)

Full 6.15 opcode / DOM/jquery/tablesorter (WAIVE) / COMPAT-007 Data / `_exit` flush / mid-deflate-in-child / S2 / `BUILD-003-FULL` / PAUSE / signed COPR / default Rocky `format=v6`.

## Docs

- [MIGRATION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/v0.2.6/docs/MIGRATION_DROP_IN_v0.md)
- [ROCKY8_DEPLOYMENT_REMAINING_v0.md](https://github.com/hilather/nytprof-modernization/blob/v0.2.6/docs/ROCKY8_DEPLOYMENT_REMAINING_v0.md)
- [perl-NYTProfM.spec](https://github.com/hilather/nytprof-modernization/blob/v0.2.6/packaging/rpm/perl-NYTProfM.spec)

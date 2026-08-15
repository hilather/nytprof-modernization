# Rocky 8 Docker profile lab MVP (v0)

**Board ID:** `ROCKY8-DOCKER-PROFILE-LAB` / `ROCKY8-DUAL-DOCKER-LAB`  
**Status:** implemented (integration smoke + operator demo + dual-engine)  
**Not:** mock-certified EL8 RPM, COPR, public perf claim, oracle DOM clone, product `DB::sub` goto / Getopt::Long attach, M01 jquery

## Goal

Keep a **repeatable Rocky 8 container lab** in-tree that installs the unsigned testdrive `perl-NYTProfM` RPM, runs live `perl -d:NYTProfM` on a core-only analyzer, and emits native `nytprofhtml`. The **integration smoke** drives that same entry point with a short `--lab` run and asserts artifacts.

Demo: [`scripts/field/rocky8_docker_profile_demo.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/rocky8_docker_profile_demo.sh)  
Smoke: [`scripts/field/rocky8_docker_profile_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/rocky8_docker_profile_smoke.sh)  
Workload: [`scripts/field/workloads/minute_text_scanner.pl`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/workloads/minute_text_scanner.pl)  
Guide: [R1_PREVIEW_OPERATOR_RUNBOOK.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) § 7c.3

## Two entry points

| Entry | Command | Duration | Network besides yum / image |
|-------|---------|----------|------------------------------|
| Operator demo | `./scripts/field/rocky8_docker_profile_demo.sh --out ~/Downloads/nytprof-rocky8-demo` | ~60s profile | ack + Gutenberg (seed fallback) |
| Dual reports | `… --engine both --out ~/Downloads/nytprof-rocky8-demo` | native + 6.15 oracle | **same** `--seconds` and corpus on both sides (apples-to-apples; `AGENTS.md` §5) |
| Host paired reports | `./scripts/field/compare_oracle_native_reports.sh --seconds 25` | 25s + 25s | Host pin + in-tree XS; `~/Downloads/nytprof-compare-apples/{oracle,native}/` |
| Integration lab | `./scripts/field/rocky8_docker_profile_smoke.sh` | host 1s + `--engine both` ~3s | none (generated seed). RPM from `dist/el8/` or GitHub Release |

`--lab` is the real demo script with `NYTPROF_DEMO_LAB=1`: generated seed, two corpus files, no ack download. Lab attach builds **in-tree** `xs-nytprof` inside the container (testdrive RPM `.so` lags live metrics). After the container returns, the smoke re-renders HTML with the **in-tree** `nytprof-cli` (heat / sort / seconds).

## Smoke contract

**Always (even without docker):**

| Check | Pass |
|-------|------|
| `bash -n` + `--help` on the demo | exit 0 |
| `perl -c` scanner | syntax OK |
| host `minute_text_scanner.pl DIR 1` | stdout matches `^passes=` |

**When docker is usable** (binary + `docker info`) **and** a testdrive RPM is on `dist/el8/`, `NYTPROF_DEMO_RPM`, or the GitHub Release URL: run `rocky8_docker_profile_demo.sh --lab --engine both --seconds 3` into a temp dir and require:

Honest **SKIP** of the docker half when the RPM is not on disk and not downloadable yet (same tag push as `Release EL8 RPM` — do not fail the CI matrix on that 404).

| Artifact | Assertion |
|----------|-----------|
| `nytprof.out` | non-empty; prefix `NYTProf 5` |
| `html/index.html` | **symlink** to `native/html`; contains `time_line_events` and `main::tokenize`; references `nytprof-sort.js` |
| `native/html/` | native site (same contract as `html/` via the symlink) |
| `oracle/html/index.html` | 6.15 site when compile succeeds; else `oracle/meta/oracle-skip.txt` + honest SKIP |
| `html/style.css` | contains `heat-hot` |
| `html/nytprof-sort.js` | present; no `jquery` / `tablesorter` |
| `html/file-*.html` | source `<tr` rows and `id="L` anchors |
| `meta/report.txt` | `main::tokenize` incl and excl **not** `0` (workload sub; not `CORE::match`) |
| `meta/timings.txt` | `lab=1`, `profiled_scanner_rc=0`, `primary_profile=minute_text_scanner` |
| `meta/scanner-profiled.txt` | `^passes=` |
| `meta/verify.txt` | `^OK:` when written |
| `meta/nytprofm-version.txt` | contains `6.15` when written |

**Honest SKIP** of the container half when `docker` is missing or the daemon is unreachable. Host checks still run. Exit 0 after `SKIP:`.

## Non-goals

| Non-goal | Notes |
|----------|-------|
| Join `offline_gate.sh` | Needs docker + yum + `rockylinux:8` pull — not an offline R1 step |
| Mock / COPR / signed RPM | Testdrive `rpm -Uvh --replacefiles` only |
| Profile ack / Getopt::Long | Product `DB::sub` residual (`&$raw`, not `goto`) |
| Public perf / 60s in CI | Lab uses 3s; operator demo stays ~60s |
| Oracle DOM / tablesorter | Native MVP HTML only |

## Make / CI

```sh
./scripts/field/rocky8_docker_profile_smoke.sh
# after perl Makefile.PL:
# make rocky8-docker-lab-smoke
```

GHA job **Rocky 8 Docker lab** in [`.github/workflows/ci-matrix.yml`](https://github.com/hilather/nytprof-modernization/blob/main/.github/workflows/ci-matrix.yml) (Linux, parallel with `rust-smoke`). **Not** a matrix row; **not** part of `offline_gate`.

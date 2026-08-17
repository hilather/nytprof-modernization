# Complex-app Docker profile lab MVP (v0)

**Board ID:** `COMPLEX-APP-DOCKER-PROFILE-LAB`  
**Status:** implemented (Rex local lab + integration smoke)  
**Not:** mock-certified EL8 RPM, COPR, public perf claim, oracle DOM, `offline_gate`

## Goal

Keep a **repeatable Rocky 8 container lab** that profiles a **real CPAN application** (not the core-only scanner) under **in-tree** `perl -d:NYTProfM` and emits native HTML. The point is to fail closed on attach crashes that only show up in large module graphs (`DateTime` / `namespace::autoclean` / `B::Hooks::EndOfScope::XS`, Getopt, Rex).

Demo: [`scripts/field/complex_app_docker_profile.sh --app ID`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/complex_app_docker_profile.sh)  
Smoke: [`scripts/field/complex_app_docker_profile_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/complex_app_docker_profile_smoke.sh) (`--app rex --engine both`)  
Catalog: [`complex-app-catalog-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/complex-app-catalog-v0.md)  
Fail-closed helper: [`scripts/field/lib/attach_survival.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/lib/attach_survival.sh)

## Why Rex

| App | Why not / why |
|-----|----------------|
| **Rex** (chosen) | Real operator CLI. Loads Getopt, Moo, YAML; this Rexfile also `use`s **DateTime** + **DateTime::Duration**. Local connection only (no SSH). Bounded `rex lab` task. |
| Munin | Daemon + plugin tree; hard to bound a single-process run. |
| Core-only scanner | Already covered by [`rocky8_docker_profile_demo.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/rocky8_docker_profile_demo.sh). Does **not** load DateTime/BHES. |

## Two entry points

| Entry | Command | Duration | Network |
|-------|---------|----------|---------|
| Operator demo | `./scripts/field/complex_app_docker_profile.sh --out ~/Downloads/nytprof-rex-demo` | ~5s native | yum + CPAN (`Rex`, `DateTime`, `YAML`) |
| Dual attach | `… --engine both` | same `--seconds` on native + 6.15 | 6.15 archive + File::Which; oracle **SKIP** if missing |
| Integration lab | `./scripts/field/complex_app_docker_profile_smoke.sh` | `--lab --engine both` ~3s | same when docker is usable |

`--engine both` is an **attach-survival** comparison: both must print `rex_lab_ok` and write `NYTProf 5`. It is **not** an exclusive-time or HTML-DOM gate. Oracle container mounts the 6.15 archive + `run_lab.pl` + File::Which only — never `crates/` on `PERL5LIB`.

HTML is `nytprof-cli html --no-flame` (DateTime loops emit millions of `TIME_LINE`s; flame on a 15s run did not finish in 15 minutes).

Attach is always **in-tree** `make -C collector xs-nytprof` inside `rockylinux:8` (not the testdrive RPM `.so`). HTML is rendered on the **host** with in-tree `nytprof-cli` after the container returns.

## Smoke contract

**Always (even without docker):**

| Check | Pass |
|-------|------|
| `bash -n` + `--help` on the demo | exit 0 |
| Rexfile contains `use DateTime`, `DateTime::Duration`, `use YAML`, `connection => 'Local'` | present |

**When docker is usable** (binary + `docker info`): run `complex_app_docker_profile.sh --lab --seconds 5` into a temp dir and require:

| Artifact | Assertion |
|----------|-----------|
| `nytprof.out` | non-empty; prefix `NYTProf 5` |
| `html/index.html` | contains `time_line_events` |
| `meta/rex-profiled.txt` | contains `rex_lab_ok` |
| `meta/rex-profiled.err` | must **not** contain `as an ARRAY ref` or `EndOfScope/XS.pm` |
| `meta/report.txt` | contains `Rex::` or `DateTime::` when written |
| `meta/verify.txt` | `^OK` when written |

Honest **SKIP** of the container half when `docker` is missing or the daemon is unreachable. Host checks still run. Exit 0 after `SKIP:`.

## Non-goals

| Non-goal | Notes |
|----------|-------|
| Join `offline_gate.sh` | Needs docker + yum + CPAN + `rockylinux:8` pull |
| Exclusive-time match vs 6.15 | Survival only (`rex_lab_ok` + NYTProf 5). Same wall ≠ same exclusive seconds (`AGENTS.md` §5). |
| SSH / remote Rex | Local connection only |
| Public perf claim | Engineering attach net only |

## Make / CI

```sh
./scripts/field/complex_app_docker_profile_smoke.sh
# after perl Makefile.PL:
# make complex-app-docker-lab-smoke
```

GHA job **Complex-app Docker lab (Rex)** in [`.github/workflows/ci-matrix.yml`](https://github.com/hilather/nytprof-modernization/blob/main/.github/workflows/ci-matrix.yml) (Linux, timeout ≥60). **Not** a matrix row; **not** part of `offline_gate`.

# EL8 / Rocky RPM sources

**Board:** `EL8-RPM-MODULE` (K01 spec/D1-B MVP) + `EL8-RPM-TOOLS` (K02 spec/MVP)  
**Not:** signed CI publish pipeline complete, mock-certified multi-stream, D1-A as default Rocky, tools-alone drop-in.

**Completion design:** [docs/DROP_IN_RPM_COMPLETION_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/DROP_IN_RPM_COMPLETION_v0.md) (PR-A0/A1 started). Source0 is produced by [`scripts/packaging/make_nytprofm_dist.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/make_nytprofm_dist.sh) (`NYTProfM-6.15.tar.gz`). Smoke: [`rpm01_make_dist_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/rpm01_make_dist_smoke.sh). **Not** BUILD-003-FULL.

## Advertised stream

First module RPM targets **Rocky 8 / EL8 base Perl 5.26**. AppStream Perl 5.32 is a **multi-stream residual**.

## Module package (`perl-NYTProfM.spec`)

| Field | Value |
|-------|--------|
| Name | `perl-NYTProfM` |
| Version | **6.15** (`Devel::NYTProfM`; match pin) |
| Default flavor | **D1-B** (v5-only, `-lz` / `zlib-devel`, **no cargo**) |
| Provides | `perl(Devel::NYTProfM) = %{version}` (not stock `perl(Devel::NYTProf)`) |
| Self-Obsoletes | **forbidden** |
| `format=v6` | fail-closed: `format=v6 requires v6-enabled build (install v6_collect package or rebuild with --with v6_collect)` |
| D1-A | `rpmbuild --with v6_collect` (optional; not default `%check` green) |

`%check` reuses shipped `g05_options_format_smoke.sh` / `product_legacy_smoke.sh` (D1-B attach **15/3/15**; no `NYTPROF6` required).

## Tools package (`nytprof-cli.spec`)

| Field | Value |
|-------|--------|
| Name | `nytprof-cli` |
| Version | **6.15** |
| Role | **Companion** — dump/report/html/convert; **not** `perl -d:NYTProfM` |
| Recommends | `perl-NYTProfM` (weak; not a substitute for the module) |
| Ingest | [ADR-0010](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0010-signed-ci-prebuilt-native-cli.md) signed CI prebuilt `linux-x86_64` |
| mock `%build` | Unpack + verify only — **no** rustup / cargo / rustc |
| Pipeline | Residual (no live signed tarball required for this MVP) |

Smoke: [`k02_el8_tools_rpm_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/k02_el8_tools_rpm_smoke.sh).

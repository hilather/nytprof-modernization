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

`%check` is [`t/installed_attach.t`](https://github.com/hilather/nytprof-modernization/blob/main/t/installed_attach.t) plus [`t/installed_scripts.t`](https://github.com/hilather/nytprof-modernization/blob/main/t/installed_scripts.t) with `PERL5LIB` = buildroot **vendorarch:vendorlib** (`.so` under `%{perl_vendorarch}`). D1-B attach **15/3/15**; `format=v6` fail-closed; `readelf` must not `NEEDED` libzstd/liblz4. **Scripts:** `%{_bindir}/nytprofhtml` / `nytprofcsv` / `nytprofcg` / `nytprof-engine` plus bundled unsigned Rocky 8 `%{_bindir}/nytprof-cli` (and `nytprof-dump` symlink). Overwrite stock `/usr/bin` names on clash (`rpm -Uvh --replacefiles`). Rebuild the ELF with [`scripts/packaging/build_el8_nytprof_cli.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/build_el8_nytprof_cli.sh) (`rockylinux:8`). `%build` stays cargo-free. Mock: [`a3_el8_mock_module.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/a3_el8_mock_module.sh) on `rocky+epel-8-x86_64` when usable; **SKIP** if mock is absent or unusable. Signing/COPR not required for test-drive.

**Testdrive RPM (unsigned, not mock-certified):** [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/build_el8_module_rpm.sh) runs `rpmbuild -ba` in `rockylinux:8` (docker on this host, native on Rocky 8). Tag workflow [`.github/workflows/release-el8-rpm.yml`](https://github.com/hilather/nytprof-modernization/blob/main/.github/workflows/release-el8-rpm.yml) attaches `perl-NYTProfM-6.15-2.el8.x86_64.rpm` to the GitHub Release.

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

## Unsigned internal yum bootstrap (A5a — not a production policy)

`gpgcheck=0` is **temporary**. Do not copy this as a fleet default. Live key + `gpgcheck=1` is A5b (holder: **hilather**). Stub file: [`RPM-GPG-KEY-nytprofm`](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/RPM-GPG-KEY-nytprofm) (`NYTPROFM-GPG-STUB`). Do **not** `rpm --import` the stub as a real key.

```text
# /etc/yum.repos.d/nytprofm-internal.repo
[nytprofm-internal]
name=NYTProfM internal (unsigned bootstrap)
baseurl=https://example.invalid/nytprofm/el8/
enabled=1
gpgcheck=0
```

No COPR project. No live `rpmsign`. Public COPR enable is A5b.

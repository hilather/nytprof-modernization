# EL8 module RPM MVP (v0)

**Board ID:** `EL8-RPM-MODULE`  
**Status:** **done (MVP)** — spec + smoke for D1-B `perl-NYTProfM` **6.15**.  
**Not:** mock-certified (k01 SKIP if mock absent/unusable); D1-A as default Rocky; I03 scripts in the RPM; `EL8-RPM-TOOLS` pipeline; `BUILD-003-FULL`; S2.

**Spec:** [`packaging/rpm/perl-NYTProfM.spec`](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/perl-NYTProfM.spec)  
**Smoke:** [`scripts/packaging/k01_el8_module_rpm_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/k01_el8_module_rpm_smoke.sh)  
**Mock runner:** [`scripts/packaging/a3_el8_mock_module.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/a3_el8_mock_module.sh)

## Default flavor (D1-B)

- Version **6.15**; Provides `perl(Devel::NYTProfM)`
- Do **not** Provides stock `perl(Devel::NYTProf)` (Option B; parallel to distro 6.15)
- BuildRequires: `gcc`, `make`, `perl-devel`, `perl-generators`, `perl(ExtUtils::ParseXS)`, `perl(ExtUtils::Embed)`, `zlib-devel`, `binutils` — **no cargo / rustc / rustup**
- No self-Obsoletes
- Advertised stream: Rocky 8 **base Perl 5.26**
- `format=v6` fail-closed with the `v6_collect` rebuild string
- Live attach (when CC/XS present) leaf **15** / mid **3** / mid→leaf **15** via shipped G05 / `t/installed_attach.t`
- **Collection-only bindir:** `%{_bindir}/nytprofm-cli` + `nytprofm-dump` (unsigned Rocky 8 ELF). Does **not** install `nytprofhtml` / `nytprofcsv` / `nytprofcg` / `nytprof-engine` or `Devel::NYTProf::*` report facades. Sits beside stock `/usr/bin/nytprofhtml`. `%build` does not run cargo. Signing not required for test-drive.

`--with v6_collect` is documented D1-A, not default K01 green.

## Install layout (KD-R12)

| Macro | EL8 typical | Files |
|-------|-------------|-------|
| `%{perl_vendorlib}` | `/usr/share/perl5/vendor_perl` | `Devel/NYTProfM.pm`, `Devel/NYTProfM/Core.pm` |
| `%{perl_vendorarch}` | `/usr/lib64/perl5/vendor_perl` | `auto/Devel/NYTProfM/NYTProfM.so` |

`%check` `PERL5LIB` is buildroot **vendorarch:vendorlib**. `readelf -d` on the vendorarch `.so` must not `NEEDED` `libzstd` / `liblz4`.

## Mock (A3)

Chroot: `rocky+epel-8-x86_64` (fallback `rocky-8-x86_64`). First `--rebuild` is **online** (BaseOS+AppStream `builddep`). SKIP when mock is absent or unusable. Not CI-mock certified. Not a GHA mock job.

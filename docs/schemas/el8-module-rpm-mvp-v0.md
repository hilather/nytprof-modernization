# EL8 module RPM MVP (v0)

**Board ID:** `EL8-RPM-MODULE`  
**Status:** **done (MVP)** — spec + smoke for D1-B `perl-NYTProfM` **6.15**.  
**Not:** mock-certified multi-stream; D1-A as default Rocky; `EL8-RPM-TOOLS` pipeline; `BUILD-003-FULL`; S2.

**Spec:** [`packaging/rpm/perl-NYTProfM.spec`](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/perl-NYTProfM.spec)  
**Smoke:** [`scripts/packaging/k01_el8_module_rpm_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/k01_el8_module_rpm_smoke.sh)

## Default flavor (D1-B)

- Version **6.15**; Provides `perl(Devel::NYTProfM)`
- Do **not** Provides stock `perl(Devel::NYTProf)` (Option B; parallel to distro 6.15)
- BuildRequires: `gcc`, `make`, `perl-devel`, `zlib-devel` — **no cargo / rustc / rustup**
- No self-Obsoletes
- Advertised stream: Rocky 8 **base Perl 5.26**
- `format=v6` fail-closed with the `v6_collect` rebuild string
- Live attach (when CC/XS present) leaf **15** / mid **3** / mid→leaf **15** via shipped G05 / I01 paths

`--with v6_collect` is documented D1-A, not default K01 green.

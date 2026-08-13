# EL8 module RPM MVP (v0)

**Board ID:** `EL8-RPM-MODULE`  
**Status:** **done (MVP)** — spec + smoke for D1-B `perl-Devel-NYTProf` ≥ 7.00.  
**Not:** mock-certified multi-stream; D1-A as default Rocky; `EL8-RPM-TOOLS`; `BUILD-003-FULL`; S2.

**Spec:** [`packaging/rpm/perl-Devel-NYTProf.spec`](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/perl-Devel-NYTProf.spec)  
**Smoke:** [`scripts/packaging/k01_el8_module_rpm_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/k01_el8_module_rpm_smoke.sh)

## Default flavor (D1-B)

- Version **≥ 7.00**; Provides `perl(Devel::NYTProf)`
- BuildRequires: `gcc`, `make`, `perl-devel`, `zlib-devel` — **no cargo / rustc / rustup**
- No self-Obsoletes
- Advertised stream: Rocky 8 **base Perl 5.26**
- `format=v6` fail-closed with the `v6_collect` rebuild string
- Live attach (when CC/XS present) leaf **15** / mid **3** / mid→leaf **15** via shipped G05 / I01 paths

`--with v6_collect` is documented D1-A, not default K01 green.

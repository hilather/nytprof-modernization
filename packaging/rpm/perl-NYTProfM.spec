# EL8 / Rocky module RPM — D1-B default (v5-only, zlib, no cargo).
# Board: EL8-RPM-MODULE (K01 spec/D1-B MVP). Not EL8-RPM-TOOLS. Not BUILD-003-FULL.
# Advertised stream (first cut): Rocky 8 / EL8 **base Perl 5.26**.
# Multi-stream (AppStream 5.32) is residual.
# Identity: perl-NYTProfM 6.15 / perl(Devel::NYTProfM). Does NOT Provides
# perl(Devel::NYTProf) (Option B — operators switch to -d:NYTProfM).

%bcond_with v6_collect

Name:           perl-NYTProfM
Version:        6.15
Release:        6%{?dist}
Summary:        NYTProfM 6.15 collection (D1-B v5-only default; -d:NYTProfM)
License:        GPL+ or Artistic
URL:            https://github.com/hilather/nytprof-modernization
Source0:        NYTProfM-%{version}.tar.gz

# Parallel package — does not displace stock perl-Devel-NYTProf.
Provides:       perl(Devel::NYTProfM) = %{version}

BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  perl-devel
BuildRequires:  perl-generators
BuildRequires:  perl(ExtUtils::ParseXS)
BuildRequires:  perl(ExtUtils::Embed)
BuildRequires:  perl(Compress::Raw::Zlib)
BuildRequires:  zlib-devel
BuildRequires:  binutils
# Module path is cargo-free. Do not BuildRequire cargo, rustc, or rustup.
%if %{with v6_collect}
BuildRequires:  libzstd-devel
BuildRequires:  lz4-devel
%endif

Requires:       perl(:MODULE_COMPAT_%(eval "`%{__perl} -V:version`"; echo $version))
Requires:       zlib
%if %{with v6_collect}
Requires:       libzstd
Requires:       lz4
%endif

# Collection-only: Devel::NYTProfM + XS + optional nytprofm-cli ELF.
# Does NOT install stock-named nytprofhtml/nytprofcsv/nytprofcg/nytprof-engine
# and does NOT package Devel::NYTProf::* report facades (I03 stays prefix/dev).
# Sits beside perl-Devel-NYTProf; no stock /usr/bin name clash.
# %build stays cargo-free. Signing / COPR not required (test-drive).

%description
NYTProfM 6.15 (Devel::NYTProfM) collection module for Rocky 8 / EL8 (advertised
stream: base Perl 5.26). Operators use perl -d:NYTProfM (not -d:NYTProf).
This RPM is collection-only: the debugger + XS. It does not install
nytprofhtml / nytprofcsv / nytprofcg / nytprof-engine and does not overwrite
stock /usr/bin names. Native report/html is nytprofm-cli (unsigned Rocky 8
ELF) or stock nytprofhtml from perl-Devel-NYTProf. Default build is D1-B:
v5-only collector linked with -lz. No cargo in %%build.

format=v6 on this default flavor fail-closes with:
  format=v6 requires v6-enabled build (install v6_collect package or rebuild with --with v6_collect)
Rebuild with --with v6_collect for D1-A (NYTPROF6). Default %%check must not
require producing NYTPROF6.

%prep
%setup -q -n NYTProfM-%{version}

%build
# D1-B default: v5-only XS. Never invoke cargo/rustc.
%if %{with v6_collect}
make -C collector xs-nytprof-v6
%else
make -C collector xs-nytprof
%endif

%install
rm -rf %{buildroot}
instlib=%{buildroot}%{perl_vendorlib}
instarch=%{buildroot}%{perl_vendorarch}
%if %{with v6_collect}
src=collector/build/xs-nytprof-v6
%else
src=collector/build/xs-nytprof
%endif
mkdir -p ${instlib}/Devel/NYTProfM ${instarch}/auto/Devel/NYTProfM
install -m 644 ${src}/Devel/NYTProfM.pm ${instlib}/Devel/NYTProfM.pm
install -m 644 ${src}/Devel/NYTProfM/Core.pm ${instlib}/Devel/NYTProfM/Core.pm
install -m 755 ${src}/auto/Devel/NYTProfM/NYTProfM.so \
  ${instarch}/auto/Devel/NYTProfM/NYTProfM.so
# Native CLI under nytprofm* only (sits beside stock nytprofhtml).
mkdir -p %{buildroot}%{_bindir}
test -x prebuilt/el8-x86_64/nytprof-cli
install -m 755 prebuilt/el8-x86_64/nytprof-cli \
  %{buildroot}%{_bindir}/nytprofm-cli
ln -sf nytprofm-cli %{buildroot}%{_bindir}/nytprofm-dump

%check
# mock/EL8 %%check (default = D1-B): no cargo. Collection attach only.
# Dual PERL5LIB so DynaLoader finds vendorarch .so (KD-R12).
export PERL5LIB=%{buildroot}%{perl_vendorarch}:%{buildroot}%{perl_vendorlib}
%{__perl} t/installed_attach.t
test -x %{buildroot}%{_bindir}/nytprofm-cli
%{buildroot}%{_bindir}/nytprofm-cli capability --json | grep -F '"collection_default":"v5"'
# D1-B: no libzstd / liblz4
if readelf -d %{buildroot}%{perl_vendorarch}/auto/Devel/NYTProfM/NYTProfM.so \
    | grep -E -q 'NEEDED.*lib(zstd|lz4)'; then
  echo "ERROR: D1-B NYTProfM.so NEEDED libzstd or liblz4" >&2
  exit 1
fi

%files
%license Changes
%{perl_vendorlib}/Devel/NYTProfM.pm
%{perl_vendorlib}/Devel/NYTProfM/
%{perl_vendorarch}/auto/Devel/NYTProfM/
%{_bindir}/nytprofm-cli
%{_bindir}/nytprofm-dump

%changelog
* Mon Aug 17 2026 nytprof-modernization <devnull@example.com> - 6.15-6
- Attach: no CORE::GLOBAL::require wrap; CvNODEBUG hint-magic CVs before
  $^P 0x01; goto inherited ::import / XSLoader / DynaLoader (DateTime
  namespace::autoclean, Rex Shared::Var, Fcntl bootstrap)
- Field: 20-app catalog + 10 diverse top-10 drivers; --app / --engine both
* Sat Aug 15 2026 nytprof-modernization <devnull@example.com> - 6.15-5
- Refresh bundled nytprof-cli: html flame graph now default-on (oracle
  nytprofhtml parity: its flame! option defaults to 1); --no-flame opts out;
  flame artifacts skipped when the profile has no call edges
* Sat Aug 15 2026 nytprof-modernization <devnull@example.com> - 6.15-4
- Refresh bundled nytprof-cli: operator HTML v2 visual refresh (carded tables,
  sticky thead, prefers-color-scheme dark) + flame frame polish actually ship
  (6.15-3 RPM packaged a prebuilt CLI from before the v0.2.8 styling commit)
- Freshness contract: nytprof-cli.source-sha256 marker; release workflow fails
  closed on a stale prebuilt (ADR-0010 test-drive gate)
* Sat Aug 15 2026 nytprof-modernization <devnull@example.com> - 6.15-3
- Collection-only module RPM: no I03 Perl wrappers or Devel::NYTProf::*
- Native CLI as nytprofm-cli / nytprofm-dump (no stock nytprofhtml clash)
* Sat Aug 15 2026 nytprof-modernization <devnull@example.com> - 6.15-2
- Operator HTML v2 + opt-in call-tree --flame (hover/click)
- Live attach incl/excl (clock + pending child excl)
- Getopt/Exporter compile under -d:NYTProfM (INIT + goto)
* Thu Aug 13 2026 nytprof-modernization <devnull@example.com> - 6.15-1
- K01: identity NYTProfM / Devel::NYTProfM 6.15; -d:NYTProfM
- D1-B v5-only module RPM (zlib, no cargo)
- I03 nytprofhtml/csv/cg + nytprof-engine (overwrite stock /usr/bin names)
- Bundle unsigned Rocky 8 nytprof-cli ELF (no cargo in %%build)
- Does not Provides perl(Devel::NYTProf) (operator switch)

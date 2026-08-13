# EL8 / Rocky module RPM — D1-B default (v5-only, zlib, no cargo).
# Board: EL8-RPM-MODULE (K01 spec/D1-B MVP). Not EL8-RPM-TOOLS. Not BUILD-003-FULL.
# Advertised stream (first cut): Rocky 8 / EL8 **base Perl 5.26**.
# Multi-stream (AppStream 5.32) is residual.
# Identity: perl-NYTProfM 6.15 / perl(Devel::NYTProfM). Does NOT Provides
# perl(Devel::NYTProf) (Option B — operators switch to -d:NYTProfM).

%bcond_with v6_collect

Name:           perl-NYTProfM
Version:        6.15
Release:        1%{?dist}
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

# I03 wrappers + unsigned Rocky 8 nytprof-cli ELF (prebuilt/el8-x86_64/).
# %build stays cargo-free. Sibling /usr/bin/nytprof-cli makes nytprofhtml work.
# Overwrites stock /usr/bin/nytprofhtml on clash (rpm -Uvh --replacefiles).
# Does not Obsoletes perl-Devel-NYTProf. Signing / COPR not required (test-drive).

%description
NYTProfM 6.15 (Devel::NYTProfM) product module for Rocky 8 / EL8 (advertised
stream: base Perl 5.26). Operators use perl -d:NYTProfM (not -d:NYTProf).
Also installs nytprof-engine, familiar nytprofhtml/nytprofcsv/nytprofcg
wrappers, and an unsigned Rocky 8 nytprof-cli next to them so html/csv work
without a second RPM. Overwrites stock /usr/bin names if they clash. Default
build is D1-B: v5-only collector linked with -lz. No cargo in %%build.

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
# I03 report wrappers (product, not oracle 6.15 nytprofhtml).
mkdir -p ${instlib}/Devel/NYTProf %{buildroot}%{_bindir}
install -m 644 perl/lib/Devel/NYTProf/EngineDispatch.pm \
  ${instlib}/Devel/NYTProf/EngineDispatch.pm
install -m 644 perl/lib/Devel/NYTProf/JsonlData.pm \
  ${instlib}/Devel/NYTProf/JsonlData.pm
install -m 644 perl/lib/Devel/NYTProf/JsonlReadStream.pm \
  ${instlib}/Devel/NYTProf/JsonlReadStream.pm
install -m 644 perl/lib/Devel/NYTProf/LegacyBridge.pm \
  ${instlib}/Devel/NYTProf/LegacyBridge.pm
install -m 644 perl/lib/Devel/NYTProf/Data.pm \
  ${instlib}/Devel/NYTProf/Data.pm
install -m 644 perl/lib/Devel/NYTProf/ReadStream.pm \
  ${instlib}/Devel/NYTProf/ReadStream.pm
install -m 755 perl/bin/nytprof-engine %{buildroot}%{_bindir}/nytprof-engine
install -m 755 perl/bin/nytprofhtml %{buildroot}%{_bindir}/nytprofhtml
install -m 755 perl/bin/nytprofcsv %{buildroot}%{_bindir}/nytprofcsv
install -m 755 perl/bin/nytprofcg %{buildroot}%{_bindir}/nytprofcg
# Unsigned Rocky 8 prebuilt (not compiled in mock).
test -x prebuilt/el8-x86_64/nytprof-cli
install -m 755 prebuilt/el8-x86_64/nytprof-cli %{buildroot}%{_bindir}/nytprof-cli
ln -sf nytprof-cli %{buildroot}%{_bindir}/nytprof-dump

%check
# mock/EL8 %%check (default = D1-B): no cargo. Drive installed files only.
# Dual PERL5LIB so DynaLoader finds vendorarch .so (KD-R12).
export PERL5LIB=%{buildroot}%{perl_vendorarch}:%{buildroot}%{perl_vendorlib}
%{__perl} t/installed_attach.t
export NYTPROF_BINDDIR=%{buildroot}%{_bindir}
%{__perl} t/installed_scripts.t
test -x %{buildroot}%{_bindir}/nytprof-cli
%{buildroot}%{_bindir}/nytprof-cli capability --json | grep -F '"collection_default":"v5"'
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
%{perl_vendorlib}/Devel/NYTProf/
%{perl_vendorarch}/auto/Devel/NYTProfM/
%{_bindir}/nytprof-engine
%{_bindir}/nytprofhtml
%{_bindir}/nytprofcsv
%{_bindir}/nytprofcg
%{_bindir}/nytprof-cli
%{_bindir}/nytprof-dump

%changelog
* Thu Aug 13 2026 nytprof-modernization <devnull@example.com> - 6.15-1
- K01: identity NYTProfM / Devel::NYTProfM 6.15; -d:NYTProfM
- D1-B v5-only module RPM (zlib, no cargo)
- I03 nytprofhtml/csv/cg + nytprof-engine (overwrite stock /usr/bin names)
- Bundle unsigned Rocky 8 nytprof-cli ELF (no cargo in %%build)
- Does not Provides perl(Devel::NYTProf) (operator switch)

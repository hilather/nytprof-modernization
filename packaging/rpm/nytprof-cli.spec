# EL8 / Rocky tools RPM — nytprof-cli companion (not drop-in collection).
# Board: EL8-RPM-TOOLS (K02 spec/MVP). Consumes ADR-0010 signed CI prebuilts.
# Pipeline / live signed tarball: residual. No rustup/cargo/rustc in mock %build.

Name:           nytprof-cli
Version:        6.15
Release:        1%{?dist}
Summary:        Native NYTProf tools companion (not collection drop-in)
License:        GPL+ or Artistic
URL:            https://github.com/hilather/nytprof-modernization

# Official ingest: signed CI prebuilt (ADR-0010). linux-x86_64 only for this RPM.
# Pipeline residual: these Source* names are the contract; unsigned fallback forbidden.
Source0:        nytprof-cli-%{version}-linux-x86_64.tar.gz
Source1:        SHA256SUMS
Source2:        SHA256SUMS.sig
Source3:        manifest.json

# Weak dep: tools never replace the module / perl -d:NYTProfM.
Recommends:     perl-NYTProfM

# No cargo, rustc, or rustup. Do not compile crates/ in mock.
# BuildRequires are only unpack/verify helpers (coreutils, tar).
BuildRequires:  tar
BuildRequires:  gzip
BuildRequires:  coreutils

%description
nytprof-cli native tools (dump / report / html / convert) for Rocky 8 / EL8.

This package is a tools companion. It is NOT drop-in collection and does
NOT provide perl -d:NYTProfM attach. Collection is perl-NYTProfM (K01).

Ingest is ADR-0010 signed CI prebuilts (linux-x86_64). mock %%build must
NOT run rustup, cargo, or rustc. Missing sums/signature fail-closed.
collection_default stays v5.

%prep
# Fail closed: require signed sums (ADR-0010). No unsigned fallback. No rustup.
test -f %{SOURCE0}
test -f %{SOURCE1}
test -f %{SOURCE2}
# Identity + integrity (pipeline residual until artifacts exist):
#   sha256sum -c SHA256SUMS
#   gpg --verify SHA256SUMS.sig SHA256SUMS   # or cosign; named when pipeline lands
%setup -q -c -n nytprof-cli-%{version}

%build
# Unpack-only. Forbidden: rustup, cargo, rustc, cargo build -p nytprof-cli.
:

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}%{_bindir}
# Payload contract: nytprof-cli (+ optional nytprof-dump) at archive root or bin/.
if [ -x nytprof-cli ]; then
  install -m 755 nytprof-cli %{buildroot}%{_bindir}/nytprof-cli
elif [ -x bin/nytprof-cli ]; then
  install -m 755 bin/nytprof-cli %{buildroot}%{_bindir}/nytprof-cli
else
  echo "ERROR: signed prebuilt payload missing nytprof-cli" >&2
  exit 1
fi
if [ -x nytprof-dump ]; then
  install -m 755 nytprof-dump %{buildroot}%{_bindir}/nytprof-dump
elif [ -x bin/nytprof-dump ]; then
  install -m 755 bin/nytprof-dump %{buildroot}%{_bindir}/nytprof-dump
fi

%check
# Tools-only: report on a bundled fixture when present. Not -d:NYTProf.
if [ -x %{buildroot}%{_bindir}/nytprof-cli ] && [ -f fixtures/v5/default-calls1/nytprof.out ]; then
  %{buildroot}%{_bindir}/nytprof-cli report --json fixtures/v5/default-calls1/nytprof.out
fi

%files
%{_bindir}/nytprof-cli
# optional nytprof-dump installed when present in the prebuilt payload

%changelog
* Thu Aug 13 2026 nytprof-modernization <devnull@example.com> - 6.15-1
- K02: tools companion RPM; Recommends perl-NYTProfM
- ADR-0010 signed CI prebuilt ingest; no rustup-in-mock
- Not drop-in collection / not perl -d:NYTProfM

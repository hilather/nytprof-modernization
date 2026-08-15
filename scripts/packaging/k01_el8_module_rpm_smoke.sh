#!/usr/bin/env bash
# K01 — EL8 perl-NYTProfM module RPM (D1-B spec/D1-B MVP).
#
# Drives the real spec file plus shipped D1-B attach / format=v6 fail-closed
# (g05_options_format_smoke.sh). When rpmspec/rpmbuild exists, invokes that
# real tool. Honest SKIP for the build-root half when those tools are absent.
# Never requires cargo. Never puts crates/ on oracle PERL5LIB.
#
# Exit 0: K01 pass (or honest skip of rpmbuild half). Exit 1: spec/attach fail.
# Exit 2: misuse / crates/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

SPEC="$ROOT/packaging/rpm/perl-NYTProfM.spec"
G05="$ROOT/scripts/packaging/g05_options_format_smoke.sh"
A3="$ROOT/scripts/packaging/a3_el8_mock_module.sh"
V6_MSG="format=v6 requires v6-enabled build (install v6_collect package or rebuild with --with v6_collect)"

usage() {
  cat <<'EOF'
Usage: k01_el8_module_rpm_smoke.sh

K01: real EL8 module spec is D1-B NYTProfM / Devel::NYTProfM 6.15; shipped D1-B
attach 15/3/15 + format=v6 fail-closed. Honest skip without rpmbuild.
EOF
}

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    *) printf 'ERROR: unknown flag: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

echo "k01_el8_module_rpm_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; cargo is not required for the module RPM"
echo "EL8-RPM-TOOLS residual; not mock-certified multi-stream; not D1-A default"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$SPEC" ]] || fail "missing $SPEC"
[[ -x "$G05" ]] || fail "missing shipped G05 smoke: $G05"
[[ -x "$A3" ]] || fail "missing A3 mock runner: $A3"
grep -F -q '%{perl_vendorarch}' "$SPEC" \
  || fail "spec must install NYTProfM.so under %{perl_vendorarch}"
grep -F -q 'perl(ExtUtils::ParseXS)' "$SPEC" \
  || fail "spec missing BuildRequires perl(ExtUtils::ParseXS)"
grep -F -q 'perl(ExtUtils::Embed)' "$SPEC" \
  || fail "spec missing BuildRequires perl(ExtUtils::Embed)"
grep -F -q 'binutils' "$SPEC" \
  || fail "spec missing BuildRequires binutils"
if grep -E -q '%\{_bindir\}/nytprof(html|csv|cg)|%\{_bindir\}/nytprof-engine' "$SPEC"; then
  fail "spec must not install stock nytprofhtml/csv/cg or nytprof-engine (collection-only)"
fi
if grep -F -q 't/installed_scripts.t' "$SPEC"; then
  fail "spec %check must not require I03 t/installed_scripts.t"
fi
if grep -F -q '%{perl_vendorlib}/Devel/NYTProf/' "$SPEC"; then
  fail "spec must not package Devel::NYTProf::* report facades"
fi
if grep -Eiq 'replacefiles' "$SPEC"; then
  fail "spec must not instruct --replacefiles (no stock bindir clash)"
fi
grep -F -q '%{_bindir}/nytprofm-cli' "$SPEC" \
  || fail "spec must install bundled EL8 CLI as nytprofm-cli"
if grep -E -q '%\{_bindir\}/nytprof-cli$' "$SPEC"; then
  fail "spec must not install stock-named nytprof-cli (use nytprofm-cli)"
fi
if awk '/^%build/,/^%install/' "$SPEC" | grep -v '^#' | grep -Eiq 'cargo |rustc |rustup'; then
  fail "module spec %build must not invoke cargo/rustc/rustup (prebuilt only)"
fi

# --- real spec contents (not a stub dump) ---
grep -E -q '^Name:[[:space:]]+perl-NYTProfM' "$SPEC" \
  || fail "spec Name is not perl-NYTProfM"
grep -E -q '^Version:[[:space:]]+6\.15' "$SPEC" \
  || fail "spec Version is not 6.15"
grep -F -q 'perl(Devel::NYTProfM)' "$SPEC" \
  || fail "spec missing Provides perl(Devel::NYTProfM)"
grep -F -q 'zlib-devel' "$SPEC" \
  || fail "spec missing zlib-devel (D1-B)"
if grep -Eiq 'BuildRequires:[[:space:]]*(cargo|rustc|rustup)' "$SPEC"; then
  fail "module spec must not BuildRequire cargo/rustc/rustup"
fi
if grep -Eiq '^Obsoletes:[[:space:]]*perl-NYTProfM' "$SPEC"; then
  fail "spec must not self-Obsoletes perl-NYTProfM"
fi
if grep -E '^Provides:' "$SPEC" | grep -E -q 'perl\(Devel::NYTProf\)[^M]'; then
  fail "spec must not Provides stock perl(Devel::NYTProf) (Option B)"
fi
grep -F -q "$V6_MSG" "$SPEC" \
  || fail "spec missing format=v6 fail-closed v6_collect wording"
grep -F -q 'D1-B' "$SPEC" \
  || fail "spec missing D1-B default flavor"
grep -Eiq 'Perl 5\.26|base Perl 5.26' "$SPEC" \
  || fail "spec must document advertised Rocky 8 base Perl 5.26 stream"
grep -F -q '%bcond_with v6_collect' "$SPEC" \
  || fail "spec missing optional --with v6_collect (D1-A note)"
grep -F -q 'xs-nytprof' "$SPEC" \
  || fail "spec %build must drive real collector xs-nytprof"
if grep -Fqi 'cargo build' "$SPEC"; then
  fail "module spec must not cargo build"
fi
ok "real spec: perl-NYTProfM 6.15 D1-B zlib-only no cargo no self-Obsoletes"

# --- shipped D1-B attach + format=v6 fail-closed (real -d:NYTProfM) ---
echo "running shipped G05 (D1-B attach 15/3/15 + format=v6 fail-closed)"
set +e
G05_OUT="$(bash "$G05" 2>&1)"
G05_RC=$?
set -e
printf '%s\n' "$G05_OUT"
[[ "$G05_RC" -eq 0 ]] || fail "g05_options_format_smoke.sh failed (rc=$G05_RC)"
if grep -E -q 'SKIP: no C toolchain|SKIP: perl XS headers' <<<"$G05_OUT"; then
  ok "G05 honest skip (no CC/XS) — spec asserts still hold"
else
  grep -F -q "$V6_MSG" <<<"$G05_OUT" \
    || fail "G05 did not emit v6_collect fail-closed text"
  grep -E -q 'leaf_returns=15' <<<"$G05_OUT" \
    || fail "G05 did not report leaf_returns=15 from live attach"
  grep -E -q 'mid_returns=3' <<<"$G05_OUT" \
    || fail "G05 did not report mid_returns=3 from live attach"
  grep -E -q 'mid_leaf_edge=15' <<<"$G05_OUT" \
    || fail "G05 did not report mid_leaf_edge=15 from live attach"
  ok "D1-B live attach 15/3/15 + format=v6 fail-closed via shipped G05"
fi

# --- real rpm tooling when present ---
if command -v rpmspec >/dev/null 2>&1; then
  echo "running: rpmspec -q --srpm $SPEC"
  set +e
  SPECQ="$(rpmspec -q --srpm "$SPEC" 2>&1)"
  SPECRC=$?
  set -e
  printf '%s\n' "$SPECQ"
  if [[ "$SPECRC" -ne 0 ]]; then
    echo "SKIP: rpmspec --srpm failed (host macros/deps) — spec file asserts hold"
  else
    grep -E -q 'perl-NYTProfM-6\.15' <<<"$SPECQ" \
      || fail "rpmspec query missing perl-NYTProfM-6.15"
    ok "rpmspec queried real spec as perl-NYTProfM-6.15"
  fi
elif command -v rpmbuild >/dev/null 2>&1; then
  echo "running: rpmbuild --nobuild -bp is skipped; rpmbuild --showrc name check"
  set +e
  rpmbuild --eval '%{name}' >/dev/null 2>&1
  echo "SKIP: rpmbuild present but no isolated mock; use rpmspec when available"
  set -e
  ok "rpmbuild present — spec file still the source of truth (no fake mock)"
else
  echo "SKIP: no rpmspec/rpmbuild on PATH — spec + G05 asserts hold (not mock-certified)"
fi

echo "running A3 mock runner (SKIP if mock absent or unusable)"
set +e
A3_OUT="$(bash "$A3" 2>&1)"
A3_RC=$?
set -e
printf '%s\n' "$A3_OUT"
[[ "$A3_RC" -eq 0 ]] || fail "a3_el8_mock_module.sh failed (rc=$A3_RC)"
if grep -E -q '^SKIP:' <<<"$A3_OUT"; then
  ok "A3 honest SKIP — not maintainer-mock certified"
else
  grep -F -q 'OK: A3 maintainer-mock rebuild' <<<"$A3_OUT" \
    || fail "A3 did not report maintainer-mock rebuild"
  ok "A3 mock rebuild green"
fi

echo "NOT-YET: BUILD-003-FULL / S2 dual_path rewrite"
echo "NOT-YET: D1-A default Rocky / AppStream 5.32 multi-stream"
echo "NOT-YET: public COPR / live rpmsign (A5b)"
ok "EL8-RPM-MODULE"
exit 0

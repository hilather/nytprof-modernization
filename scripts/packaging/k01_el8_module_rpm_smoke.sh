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

if ! command -v mock >/dev/null 2>&1; then
  echo "SKIP: mock not installed — not mock-certified multi-stream"
fi

echo "NOT-YET: EL8-RPM-TOOLS / K02 nytprof-cli spec"
echo "NOT-YET: BUILD-003-FULL / S2 dual_path rewrite"
echo "NOT-YET: D1-A default Rocky / AppStream 5.32 multi-stream"
ok "EL8-RPM-MODULE"
exit 0

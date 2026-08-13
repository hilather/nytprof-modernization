#!/usr/bin/env bash
# J01 — CPAN dist hygiene: NYTProfM / Devel::NYTProfM 6.15 from the real Makefile.PL.
#
# Drives root perl Makefile.PL (not a reimplemented META writer). Asserts
# generated MYMETA name/version, then make manifest / MANIFEST listing
# excludes baseline/ target/ prefix/. Cargo-free (PATH-hidden cargo).
#
# Does not flip CPAN-TRIAL-READY / BUILD-003-FULL / S2.
# Never puts crates/ on oracle PERL5LIB.
#
# Exit 0: J01 pass. Exit 1: configure/META/MANIFEST failure. Exit 2: misuse.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

MAKEFILE_PL="$ROOT/Makefile.PL"
PRODUCT_PM="$ROOT/collector/xs/Devel/NYTProfM.pm"
SKIP="$ROOT/MANIFEST.SKIP"

usage() {
  cat <<'EOF'
Usage: j01_cpan_hygiene_smoke.sh

J01: real Makefile.PL emits NYTProfM / Devel::NYTProfM 6.15; MANIFEST excludes
baseline/ target/ prefix/. Cargo-free. CPAN-TRIAL-READY stays residual.
EOF
}

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'ERROR: unknown flag: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

echo "j01_cpan_hygiene_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; cargo is not required"
echo "full_build003=0; CPAN-TRIAL-READY residual"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*)
      fail2 "PERL5LIB must not contain /crates/: $PERL5LIB"
      ;;
  esac
fi

[[ -f "$MAKEFILE_PL" ]] || fail "missing $MAKEFILE_PL"
[[ -f "$PRODUCT_PM" ]] || fail "missing product module $PRODUCT_PM"
[[ -f "$SKIP" ]] || fail "missing $SKIP"
grep -q "NAME             => 'Devel::NYTProfM'" "$MAKEFILE_PL" \
  || fail "Makefile.PL NAME is not Devel::NYTProf"
grep -q 'VERSION_FROM' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing VERSION_FROM (must read product \$VERSION)"
grep -F -q "collector/xs/Devel/NYTProfM.pm" "$MAKEFILE_PL" \
  || fail "Makefile.PL VERSION_FROM must point at collector/xs/Devel/NYTProfM.pm"
grep -q 'packaging_j01=1' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing packaging_j01=1 stamp"
grep -q 'cpan_trial_ready=0' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing cpan_trial_ready=0"
grep -q 'packaging_i02=1' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing packaging_i02=1"
grep -q 'full_build003=0' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing full_build003=0"
if grep -q "NAME             => 'NYTProf::Modernization::PackagingEntry'" "$MAKEFILE_PL"; then
  fail "Makefile.PL still advertises PackagingEntry as NAME"
fi
grep -q "our \$VERSION = '6.15'" "$PRODUCT_PM" \
  || fail "product Devel::NYTProfM.pm missing \$VERSION = '6.15'"
grep -F -q '^baseline/' "$SKIP" || fail "MANIFEST.SKIP missing ^baseline/"
grep -F -q '^target/' "$SKIP" || fail "MANIFEST.SKIP missing ^target/"
grep -F -q '^prefix/' "$SKIP" || fail "MANIFEST.SKIP missing ^prefix/"
ok "Makefile.PL + product \$VERSION + MANIFEST.SKIP present"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-j01-XXXXXX")"
HAD_MAKEFILE=0
[[ -f "$ROOT/Makefile" ]] && HAD_MAKEFILE=1
HAD_MANIFEST=0
[[ -f "$ROOT/MANIFEST" ]] && HAD_MANIFEST=1
SMOKE_OWNED_MAKEFILE=0
cleanup() {
  if [[ "${SMOKE_OWNED_MAKEFILE}" -eq 1 ]]; then
    rm -f "$ROOT/Makefile" "$ROOT/Makefile.old" \
      "$ROOT/MYMETA.json" "$ROOT/MYMETA.yml" \
      "$ROOT/nytprof-packaging.mode" \
      "$ROOT/pm_to_blib" 2>/dev/null || true
    rm -rf "$ROOT/blib" 2>/dev/null || true
    if [[ "$HAD_MANIFEST" -eq 0 ]]; then
      rm -f "$ROOT/MANIFEST" 2>/dev/null || true
    fi
    rm -rf "$ROOT"/NYTProfM-* 2>/dev/null || true
  fi
  rm -rf "$WORKDIR"
}
trap cleanup EXIT

NOCARGO_BIN="$WORKDIR/nocargo-bin"
mkdir -p "$NOCARGO_BIN"
PERL_BIN="$(command -v perl)"
[[ -n "$PERL_BIN" && -x "$PERL_BIN" ]] || fail "perl not on PATH"
ln -sf "$PERL_BIN" "$NOCARGO_BIN/perl"
printf '#!/bin/sh\necho "ERROR: cargo hidden by j01_cpan_hygiene_smoke" >&2\nexit 127\n' \
  >"$NOCARGO_BIN/cargo"
chmod +x "$NOCARGO_BIN/cargo"
NOCARGO_PATH="$NOCARGO_BIN:/usr/bin:/bin"

print_residuals() {
  echo "NOT-YET: CPAN-TRIAL-READY / PAUSE upload / J02 TRIAL notes"
  echo "NOT-YET: BUILD-003-FULL / full_build003=1"
  echo "NOT-YET: EL8 RPM / S2 dual_path primary → P-PRODUCT-LEGACY"
}

if [[ "$HAD_MAKEFILE" -eq 1 ]]; then
  fail "Makefile already present — refuse to reconfigure (would clobber); rerun without a leftover Makefile"
fi

echo "running: PATH=<cargo-hidden> NYTPROF_NATIVE=0 perl Makefile.PL"
set +e
OUTCFG="$(
  cd "$ROOT" && PATH="$NOCARGO_PATH" NYTPROF_NATIVE=0 perl Makefile.PL 2>&1
)"
RCCFG=$?
set -e
printf '%s\n' "$OUTCFG"
[[ "$RCCFG" -eq 0 ]] || fail "NYTPROF_NATIVE=0 perl Makefile.PL failed without cargo (rc=$RCCFG)"
SMOKE_OWNED_MAKEFILE=1
[[ -f "$ROOT/MYMETA.json" ]] || fail "configure did not write MYMETA.json"
[[ -f "$ROOT/nytprof-packaging.mode" ]] || fail "missing nytprof-packaging.mode"
grep -q 'packaging_j01=1' "$ROOT/nytprof-packaging.mode" \
  || fail "expected packaging_j01=1 stamp"
grep -q 'cpan_trial_ready=0' "$ROOT/nytprof-packaging.mode" \
  || fail "expected cpan_trial_ready=0"
grep -q 'full_build003=0' "$ROOT/nytprof-packaging.mode" \
  || fail "expected full_build003=0"
ok "cargo-free configure wrote MYMETA.json"

# Parse identity from the real MakeMaker META (not a stub).
META_NAME="$(perl -ne 'print $1 if /"name"\s*:\s*"([^"]+)"/' "$ROOT/MYMETA.json")"
META_VER="$(perl -ne 'print $1 if /"version"\s*:\s*"([^"]+)"/' "$ROOT/MYMETA.json")"
echo "MYMETA.json name=${META_NAME:-?} version=${META_VER:-?}"
[[ "$META_NAME" == "NYTProfM" || "$META_NAME" == "Devel-NYTProfM" || "$META_NAME" == "Devel::NYTProfM" ]] \
  || fail "MYMETA name=$META_NAME (want NYTProfM or Devel-NYTProfM)"
if [[ "$META_NAME" == *"PackagingEntry"* ]]; then
  fail "MYMETA still uses PackagingEntry"
fi
[[ -n "$META_VER" ]] || fail "MYMETA missing version"
perl -e 'my $v = shift; die "version $v < 7.00\n" unless $v eq "6.15"' "$META_VER" \
  || fail "MYMETA version=$META_VER (want 6.15)"
if [[ "$META_VER" == "0.001" ]]; then
  fail "MYMETA still uses facade version 0.001"
fi
# Product module $VERSION must be the same source (VERSION_FROM).
PM_VER="$(perl -ne "print \$1 if /our \\\$VERSION = '([^']+)'/" "$PRODUCT_PM")"
[[ "$PM_VER" == "$META_VER" ]] \
  || fail "MYMETA version=$META_VER != product module \$VERSION=$PM_VER"
ok "real MYMETA identity: $META_NAME $META_VER (from $PRODUCT_PM)"

echo "running: PATH=<cargo-hidden> make manifest"
set +e
OUTMAN="$(
  cd "$ROOT" && PATH="$NOCARGO_PATH" make manifest 2>&1
)"
RCMAN=$?
set -e
printf '%s\n' "$OUTMAN"
[[ "$RCMAN" -eq 0 ]] || fail "make manifest failed (rc=$RCMAN)"
[[ -f "$ROOT/MANIFEST" ]] || fail "make manifest did not write MANIFEST"

# Dist listing must not include excluded trees (path prefix, not substring).
bad="$(perl -ne '
  chomp;
  s/^\s+//;
  next if $_ eq "" || /^#/;
  s/\s+#.*$//;
  if (m{^(baseline|target|prefix|crates)/} || m{^(baseline|target|prefix|crates)$}) {
    print "$_\n";
  }
' "$ROOT/MANIFEST")"
if [[ -n "$bad" ]]; then
  printf '%s\n' "$bad" >&2
  fail "MANIFEST includes excluded baseline/ target/ prefix/ crates/ paths"
fi
# Positive: product module that VERSION_FROM reads must be listable.
if ! grep -E -q 'collector/xs/Devel/NYTProfM\.pm' "$ROOT/MANIFEST"; then
  fail "MANIFEST missing collector/xs/Devel/NYTProfM.pm (VERSION_FROM source)"
fi
ok "MANIFEST excludes baseline/ target/ prefix/ crates/; includes product .pm"

print_residuals
ok "J01-CPAN-HYGIENE"
exit 0

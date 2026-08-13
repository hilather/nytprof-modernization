#!/usr/bin/env bash
# J02 — CPAN TRIAL notes-ready (attach-preview). Not a PAUSE upload.
#
# Reads the real release notes + Changes, and the real Makefile.PL /
# VERSION_FROM product .pm / generated MYMETA. Cargo-free.
#
# Exit 0: J02 pass. Exit 1: notes/identity failure. Exit 2: misuse / crates/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

NOTES="$ROOT/docs/RELEASE_NOTES_CPAN_TRIAL_v0.md"
CHANGES="$ROOT/Changes"
MAKEFILE_PL="$ROOT/Makefile.PL"
PRODUCT_PM="$ROOT/collector/xs/Devel/NYTProfM.pm"

usage() {
  cat <<'EOF'
Usage: j02_cpan_trial_notes_smoke.sh

J02: real TRIAL attach-preview notes + real Makefile.PL identity
NYTProfM / Devel::NYTProfM 6.15. Not PAUSE uploaded.
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

echo "j02_cpan_trial_notes_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; cargo is not required; not PAUSE uploaded"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$NOTES" ]] || fail "missing $NOTES"
[[ -f "$CHANGES" ]] || fail "missing $CHANGES"
[[ -f "$MAKEFILE_PL" ]] || fail "missing $MAKEFILE_PL"
[[ -f "$PRODUCT_PM" ]] || fail "missing $PRODUCT_PM"
[[ -x "$0" ]] || true

NOTES_BLOB="$(cat "$NOTES" "$CHANGES")"
for needle in \
  'NYTProfM' \
  'Devel::NYTProfM' \
  '6.15' \
  'attach-preview' \
  'WAIVE' \
  'tablesorter' \
  'entersub' \
  'collection_default' \
  'v5' \
  'PAUSE'
do
  grep -F -q -- "$needle" <<<"$NOTES_BLOB" \
    || fail "TRIAL notes/Changes missing required string: $needle"
done
grep -Eiq 'not uploaded to PAUSE|not a PAUSE upload|Not PAUSE uploaded' <<<"$NOTES_BLOB" \
  || fail "notes must say TRIAL is not uploaded to PAUSE"
if grep -Eiq 'cpan-upload succeeded|uploaded the TRIAL to PAUSE' "$NOTES"; then
  fail "notes must not claim a PAUSE upload succeeded"
fi
ok "real TRIAL notes + Changes have attach-preview identity and residual honesty"

grep -q "NAME             => 'Devel::NYTProfM'" "$MAKEFILE_PL" \
  || fail "Makefile.PL NAME is not Devel::NYTProfM"
grep -F -q 'collector/xs/Devel/NYTProfM.pm' "$MAKEFILE_PL" \
  || fail "Makefile.PL VERSION_FROM must point at product .pm"
grep -q 'packaging_j02=1' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing packaging_j02=1"
grep -q 'cpan_trial_notes=1' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing cpan_trial_notes=1"
grep -q 'cpan_trial_uploaded=0' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing cpan_trial_uploaded=0"
grep -q 'cpan_trial_ready=0' "$MAKEFILE_PL" \
  || fail "Makefile.PL must keep cpan_trial_ready=0 (not PAUSE uploaded)"
grep -q "our \$VERSION = '6.15'" "$PRODUCT_PM" \
  || fail "product module missing \$VERSION = '6.15'"
ok "shipped configure/module identity is NYTProfM / Devel::NYTProfM 6.15 (not a stub)"

WORKDIR=$(mktemp -d /tmp/nytprof-j02-XXXXXX)
HAD_MAKEFILE=0
[[ -f "$ROOT/Makefile" ]] && HAD_MAKEFILE=1
SMOKE_OWNED_MAKEFILE=0
cleanup() {
  if [[ "${SMOKE_OWNED_MAKEFILE}" -eq 1 ]]; then
    rm -f "$ROOT/Makefile" "$ROOT/Makefile.old" \
      "$ROOT/MYMETA.json" "$ROOT/MYMETA.yml" \
      "$ROOT/nytprof-packaging.mode" 2>/dev/null || true
  fi
  rm -rf "$WORKDIR"
}
trap cleanup EXIT

NOCARGO_BIN="$WORKDIR/nocargo-bin"
mkdir -p "$NOCARGO_BIN"
PERL_BIN="$(command -v perl)"
[[ -n "$PERL_BIN" && -x "$PERL_BIN" ]] || fail "perl not on PATH"
ln -sf "$PERL_BIN" "$NOCARGO_BIN/perl"
printf '#!/bin/sh\necho "ERROR: cargo hidden by j02_cpan_trial_notes_smoke" >&2\nexit 127\n' \
  >"$NOCARGO_BIN/cargo"
chmod +x "$NOCARGO_BIN/cargo"
NOCARGO_PATH="$NOCARGO_BIN:/usr/bin:/bin"

if [[ "$HAD_MAKEFILE" -eq 1 ]]; then
  echo "SKIP: Makefile already present — asserting identity from Makefile.PL + product .pm only"
else
  echo "running: PATH=<cargo-hidden> NYTPROF_NATIVE=0 perl Makefile.PL"
  set +e
  OUTCFG="$(
    cd "$ROOT" && PATH="$NOCARGO_PATH" NYTPROF_NATIVE=0 perl Makefile.PL 2>&1
  )"
  RCCFG=$?
  set -e
  printf '%s\n' "$OUTCFG"
  [[ "$RCCFG" -eq 0 ]] || fail "perl Makefile.PL failed without cargo (rc=$RCCFG)"
  SMOKE_OWNED_MAKEFILE=1
  [[ -f "$ROOT/MYMETA.json" ]] || fail "configure did not write MYMETA.json"
  META_NAME="$(perl -ne 'print $1 if /"name"\s*:\s*"([^"]+)"/' "$ROOT/MYMETA.json")"
  META_VER="$(perl -ne 'print $1 if /"version"\s*:\s*"([^"]+)"/' "$ROOT/MYMETA.json")"
  echo "MYMETA.json name=${META_NAME:-?} version=${META_VER:-?}"
  [[ "$META_NAME" == "NYTProfM" || "$META_NAME" == "Devel-NYTProfM" || "$META_NAME" == "Devel::NYTProfM" ]] \
    || fail "MYMETA name=$META_NAME (want NYTProfM)"
  perl -e 'my $v = shift; die "version $v < 7.00\n" unless $v eq "6.15"' "$META_VER" \
    || fail "MYMETA version=$META_VER (want 6.15)"
  grep -q 'cpan_trial_uploaded=0' "$ROOT/nytprof-packaging.mode" \
    || fail "stamp missing cpan_trial_uploaded=0"
  grep -q 'cpan_trial_notes=1' "$ROOT/nytprof-packaging.mode" \
    || fail "stamp missing cpan_trial_notes=1"
  grep -q 'cpan_trial_ready=0' "$ROOT/nytprof-packaging.mode" \
    || fail "stamp missing cpan_trial_ready=0 (not PAUSE)"
  ok "real MYMETA identity: $META_NAME $META_VER (notes-ready; not uploaded)"
fi

echo "NOT-YET: PAUSE upload / cpan-upload / CPAN index"
echo "NOT-YET: BUILD-003-FULL / EL8 RPM / S2 dual_path rewrite"
ok "J02-CPAN-TRIAL-NOTES"
exit 0

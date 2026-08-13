#!/usr/bin/env bash
# I02 — MakeMaker NYTPROF_NATIVE install of nytprof-cli.
#
# Drives the real root Makefile.PL / make native-install / install_native.sh:
#   - NYTPROF_NATIVE=1 without cargo → fail-closed (non-zero + cargo-required)
#   - NYTPROF_NATIVE=0 without cargo → configure succeeds (cargo-free)
#   - NYTPROF_NATIVE=auto without cargo → configure succeeds (no hard fail)
#   - With cargo: =1 (and auto) install prefix nytprof-cli; report --json of a
#     real v5 fixture is leaf 15 / mid 3 / mid→leaf 15
#
# Cargo-absent branches hide cargo via a PATH that only has perl (real
# Makefile.PL entry, not a stub). Never puts crates/ on oracle PERL5LIB.
#
# Exit 0: I02 pass. Exit 1: configure/install/report failure. Exit 2: misuse.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

MAKEFILE_PL="$ROOT/Makefile.PL"
INSTALL_NATIVE="$ROOT/scripts/packaging/install_native.sh"
FIXTURE="$ROOT/fixtures/v5/default-calls1/nytprof.out"

usage() {
  cat <<'EOF'
Usage: i02_makemaker_native_smoke.sh

I02 MakeMaker native: NYTPROF_NATIVE=1 fail-closed without cargo; =0/auto
cargo-free; with cargo install nytprof-cli and report 15/3/15.
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

echo "i02_makemaker_native_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on oracle PERL5LIB; full_build003=0"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*)
      fail2 "PERL5LIB must not contain /crates/: $PERL5LIB"
      ;;
  esac
fi

[[ -f "$MAKEFILE_PL" ]] || fail "missing $MAKEFILE_PL"
[[ -f "$INSTALL_NATIVE" ]] || fail "missing $INSTALL_NATIVE"
[[ -f "$FIXTURE" ]] || fail "missing fixture $FIXTURE"
grep -q 'NYTPROF_NATIVE=1 requires cargo' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing NYTPROF_NATIVE=1 requires cargo fail-closed text"
grep -q 'packaging_i02=1' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing packaging_i02=1 stamp"
grep -q 'install_native.sh' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing install_native.sh"
ok "Makefile.PL + install_native.sh + fixture present"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-i02-XXXXXX")"
SMOKE_OWNED_MAKEFILE=0
cleanup() {
  if [[ "${SMOKE_OWNED_MAKEFILE}" -eq 1 ]]; then
    rm -f "$ROOT/Makefile" "$ROOT/Makefile.old" \
      "$ROOT/MYMETA.json" "$ROOT/MYMETA.yml" \
      "$ROOT/nytprof-packaging.mode" \
      "$ROOT/pm_to_blib" 2>/dev/null || true
    rm -rf "$ROOT/blib" 2>/dev/null || true
  fi
  rm -rf "$WORKDIR"
}
trap cleanup EXIT

NOCARGO_BIN="$WORKDIR/nocargo-bin"
mkdir -p "$NOCARGO_BIN"
PERL_BIN="$(command -v perl)"
[[ -n "$PERL_BIN" && -x "$PERL_BIN" ]] || fail "perl not on PATH"
ln -sf "$PERL_BIN" "$NOCARGO_BIN/perl"
NOCARGO_PATH="$NOCARGO_BIN"

# --- NYTPROF_NATIVE=1 without cargo: real configure fail-closed ---
echo "running: PATH=<perl-only> NYTPROF_NATIVE=1 perl Makefile.PL"
set +e
OUT1="$(
  cd "$ROOT" && PATH="$NOCARGO_PATH" NYTPROF_NATIVE=1 perl Makefile.PL 2>&1
)"
RC1=$?
set -e
printf '%s\n' "$OUT1"
[[ "$RC1" -ne 0 ]] || fail "NYTPROF_NATIVE=1 perl Makefile.PL must fail-closed without cargo"
grep -F -q 'NYTPROF_NATIVE=1 requires cargo' <<<"$OUT1" \
  || fail "NYTPROF_NATIVE=1 fail-closed missing greppable cargo-required text"
ok "NYTPROF_NATIVE=1 fail-closed without cargo (real Makefile.PL)"

# --- =0 without cargo: cargo-free configure ---
echo "running: PATH=<perl-only> NYTPROF_NATIVE=0 perl Makefile.PL"
set +e
OUT0="$(
  cd "$ROOT" && PATH="$NOCARGO_PATH" NYTPROF_NATIVE=0 perl Makefile.PL 2>&1
)"
RC0=$?
set -e
printf '%s\n' "$OUT0"
[[ "$RC0" -eq 0 ]] || fail "NYTPROF_NATIVE=0 perl Makefile.PL must succeed without cargo (rc=$RC0)"
SMOKE_OWNED_MAKEFILE=1
[[ -f "$ROOT/nytprof-packaging.mode" ]] || fail "missing nytprof-packaging.mode after =0"
grep -q 'native_mode=off' "$ROOT/nytprof-packaging.mode" \
  || fail "expected native_mode=off for NYTPROF_NATIVE=0"
grep -q 'packaging_i02=1' "$ROOT/nytprof-packaging.mode" \
  || fail "expected packaging_i02=1 stamp"
grep -q 'full_build003=0' "$ROOT/nytprof-packaging.mode" \
  || fail "expected full_build003=0"
ok "NYTPROF_NATIVE=0 configure is cargo-free"

# --- auto without cargo: no hard fail ---
echo "running: PATH=<perl-only> NYTPROF_NATIVE=auto perl Makefile.PL"
set +e
OUTA="$(
  cd "$ROOT" && PATH="$NOCARGO_PATH" NYTPROF_NATIVE=auto perl Makefile.PL 2>&1
)"
RCA=$?
set -e
printf '%s\n' "$OUTA"
[[ "$RCA" -eq 0 ]] || fail "NYTPROF_NATIVE=auto perl Makefile.PL must not fail without cargo (rc=$RCA)"
grep -q 'native_mode=off' "$ROOT/nytprof-packaging.mode" \
  || fail "expected native_mode=off for auto without cargo"
ok "NYTPROF_NATIVE=auto stays cargo-free when cargo is absent"

print_residuals() {
  echo "NOT-YET: BUILD-003-FULL / full_build003=1"
  echo "NOT-YET: CPAN-TRIAL-READY / EL8 RPM"
  echo "NOT-YET: S2 dual_path primary → P-PRODUCT-LEGACY"
}

if ! command -v cargo >/dev/null 2>&1; then
  echo "SKIP: cargo not on PATH — native-install half not exercised"
  echo "  (fail-closed =1 and cargo-free =0/auto already proven via real Makefile.PL)"
  print_residuals
  ok "i02_makemaker_native_smoke completed (skip native install — no cargo)"
  exit 0
fi
ok "cargo present: $(cargo --version 2>/dev/null || echo unknown)"

# --- =1 with cargo: configure + install into isolated prefix ---
echo "running: NYTPROF_NATIVE=1 perl Makefile.PL (cargo present)"
set +e
OUT1C="$(
  cd "$ROOT" && NYTPROF_NATIVE=1 perl Makefile.PL 2>&1
)"
RC1C=$?
set -e
printf '%s\n' "$OUT1C"
[[ "$RC1C" -eq 0 ]] || fail "NYTPROF_NATIVE=1 perl Makefile.PL failed with cargo (rc=$RC1C)"
grep -q 'native_mode=on' "$ROOT/nytprof-packaging.mode" \
  || fail "expected native_mode=on after NYTPROF_NATIVE=1 + cargo"
ok "NYTPROF_NATIVE=1 configure with cargo (native_mode=on)"

PREFIX="$WORKDIR/prefix"
echo "NYTPROF_PREFIX=$PREFIX make native-install"
NYTPROF_PREFIX="$PREFIX" make native-install
CLI="$PREFIX/bin/nytprof-cli"
[[ -x "$CLI" ]] || fail "native-install did not produce $CLI"
ok "installed nytprof-cli → $CLI"

set +e
"$CLI" report --json "$FIXTURE" >"$WORKDIR/report.json" 2>"$WORKDIR/report.json.err"
JRC=$?
set -e
if [[ "$JRC" -ne 0 ]]; then
  cat "$WORKDIR/report.json.err" >&2 || true
  fail "installed nytprof-cli report --json failed (rc=$JRC)"
fi
LEAF="$(perl -ne 'print $1 if /"leaf_returns"\s*:\s*(\d+)/' "$WORKDIR/report.json")"
MID="$(perl -ne 'print $1 if /"mid_returns"\s*:\s*(\d+)/' "$WORKDIR/report.json")"
EDGE="$(perl -ne 'print $1 if /"mid_leaf_edge"\s*:\s*(\d+)/' "$WORKDIR/report.json")"
echo "report --json: leaf_returns=${LEAF:-?} mid_returns=${MID:-?} mid_leaf_edge=${EDGE:-?}"
[[ "$LEAF" == "15" ]] || fail "leaf_returns=$LEAF (want 15) from installed CLI"
[[ "$MID" == "3" ]] || fail "mid_returns=$MID (want 3) from installed CLI"
[[ "$EDGE" == "15" ]] || fail "mid_leaf_edge=$EDGE (want 15) from installed CLI"
ok "installed CLI report of real v5 profile: leaf 15 / mid 3 / mid→leaf 15"

# --- auto + cargo: configure optional and make all installs CLI ---
echo "running: NYTPROF_NATIVE=auto perl Makefile.PL && make all (cargo present)"
set +e
OUTAC="$(
  cd "$ROOT" && NYTPROF_NATIVE=auto perl Makefile.PL 2>&1
)"
RCAC=$?
set -e
printf '%s\n' "$OUTAC"
[[ "$RCAC" -eq 0 ]] || fail "NYTPROF_NATIVE=auto perl Makefile.PL failed with cargo"
grep -q 'native_mode=optional' "$ROOT/nytprof-packaging.mode" \
  || fail "expected native_mode=optional for auto + cargo"
PREFIX2="$WORKDIR/prefix-auto"
NYTPROF_PREFIX="$PREFIX2" make all
[[ -x "$PREFIX2/bin/nytprof-cli" ]] || fail "auto make all did not install $PREFIX2/bin/nytprof-cli"
ok "NYTPROF_NATIVE=auto + cargo: make all installed nytprof-cli"

# Restore cargo-free default stamp
NYTPROF_NATIVE=0 perl Makefile.PL >/dev/null

print_residuals
ok "I02 MakeMaker native"
exit 0

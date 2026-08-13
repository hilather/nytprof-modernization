#!/usr/bin/env bash
# I03 — cargo-free product report scripts + EngineDispatch prefix install.
#
# Drives the real install_product_scripts.sh (and optionally MakeMaker
# install-product-scripts) into an isolated NYTPROF_PREFIX. Asserts
# EngineDispatch + nytprof-engine + wrappers, then runs the INSTALLED
# nytprof-engine query --json --jsonl on golden default-calls1 JSONL and
# parses leaf_returns 15 / mid_returns 3 / mid_leaf_edge 15.
#
# Cargo is hidden on PATH. Never puts crates/ on PERL5LIB.
# Not wired into dual_path_smoke.sh (S2 not claimed).
#
# Exit 0: I03 pass. Exit 1: install/query failure. Exit 2: misuse / crates/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

INSTALL_SH="$ROOT/scripts/packaging/install_product_scripts.sh"
MAKEFILE_PL="$ROOT/Makefile.PL"
JSONL="$ROOT/fixtures/v5/default-calls1/readstream.jsonl"
FIXTURE="$ROOT/fixtures/v5/default-calls1/nytprof.out"

usage() {
  cat <<'EOF'
Usage: i03_dist_scripts_smoke.sh

I03 product scripts: cargo-free install of EngineDispatch + nytprof-engine
+ nytprofhtml/csv wrappers; installed query --json --jsonl is 15/3/15.
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

echo "i03_dist_scripts_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; cargo is not required"
echo "full_build003=0; not CPAN-TRIAL; not COMPAT-007"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*)
      fail2 "PERL5LIB must not contain /crates/: $PERL5LIB"
      ;;
  esac
fi

[[ -f "$INSTALL_SH" ]] || fail "missing $INSTALL_SH"
[[ -x "$INSTALL_SH" ]] || fail "not executable: $INSTALL_SH"
[[ -f "$MAKEFILE_PL" ]] || fail "missing $MAKEFILE_PL"
[[ -f "$JSONL" ]] || fail "missing golden JSONL $JSONL"
[[ -f "$FIXTURE" ]] || fail "missing fixture $FIXTURE"
grep -q 'install-product-scripts' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing install-product-scripts target"
grep -q 'i03-dist-scripts-smoke' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing i03-dist-scripts-smoke target"
grep -q 'packaging_i03=1' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing packaging_i03=1 stamp"
grep -q 'packaging_i02=1' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing packaging_i02=1 stamp"
grep -q 'full_build003=0' "$MAKEFILE_PL" \
  || fail "Makefile.PL missing full_build003=0"
ok "installer + Makefile.PL + golden JSONL present"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-i03-XXXXXX")"
HAD_MAKEFILE=0
[[ -f "$ROOT/Makefile" ]] && HAD_MAKEFILE=1
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

# Hide cargo while keeping perl + unix tools (installer needs mkdir/cp/chmod).
NOCARGO_BIN="$WORKDIR/nocargo-bin"
mkdir -p "$NOCARGO_BIN"
PERL_BIN="$(command -v perl)"
[[ -n "$PERL_BIN" && -x "$PERL_BIN" ]] || fail "perl not on PATH"
ln -sf "$PERL_BIN" "$NOCARGO_BIN/perl"
printf '#!/bin/sh\necho "ERROR: cargo hidden by i03_dist_scripts_smoke" >&2\nexit 127\n' \
  >"$NOCARGO_BIN/cargo"
chmod +x "$NOCARGO_BIN/cargo"
NOCARGO_PATH="$NOCARGO_BIN:/usr/bin:/bin"

print_residuals() {
  echo "NOT-YET: BUILD-003-FULL / full_build003=1"
  echo "NOT-YET: CPAN-TRIAL-READY / EL8 RPM"
  echo "NOT-YET: S2 dual_path primary → P-PRODUCT-LEGACY"
  echo "NOT-YET: full 6.15 nytprofhtml DOM / COMPAT-007 Data drop-in"
}

PREFIX="$WORKDIR/prefix"
echo "NYTPROF_PREFIX=$PREFIX"
echo "running: PATH=<cargo-hidden> install_product_scripts.sh (no cargo)"
NYTPROF_PREFIX="$PREFIX" PATH="$NOCARGO_PATH" bash "$INSTALL_SH"

LIB="$PREFIX/lib/perl5"
ENGINE="$PREFIX/bin/nytprof-engine"
HTML="$PREFIX/bin/nytprofhtml"
CSV="$PREFIX/bin/nytprofcsv"
CG="$PREFIX/bin/nytprofcg"
ED="$LIB/Devel/NYTProf/EngineDispatch.pm"

[[ -f "$ED" ]] || fail "install did not write $ED"
[[ -x "$ENGINE" ]] || fail "install did not write executable $ENGINE"
[[ -x "$HTML" ]] || fail "install did not write executable $HTML"
[[ -x "$CSV" ]] || fail "install did not write executable $CSV"
[[ -x "$CG" ]] || fail "install did not write executable $CG"
[[ -f "$LIB/Devel/NYTProf/JsonlData.pm" ]] || fail "missing installed JsonlData.pm"
[[ -f "$LIB/Devel/NYTProf/JsonlReadStream.pm" ]] || fail "missing installed JsonlReadStream.pm"
[[ -f "$LIB/Devel/NYTProf/LegacyBridge.pm" ]] || fail "missing installed LegacyBridge.pm"
[[ -f "$PREFIX/nytprof-product-scripts.install" ]] || fail "missing install stamp"
grep -F -q 'packaging_i03=1' "$PREFIX/nytprof-product-scripts.install" \
  || fail "install stamp missing packaging_i03=1"
grep -F -q 'full_build003=0' "$PREFIX/nytprof-product-scripts.install" \
  || fail "install stamp missing full_build003=0"
grep -F -q 'cargo_required=0' "$PREFIX/nytprof-product-scripts.install" \
  || fail "install stamp missing cargo_required=0"
# Must not overwrite I01 debugger files (this installer never writes them).
if [[ -f "$LIB/Devel/NYTProf.pm" ]]; then
  fail "I03 installer must not write I01 debugger $LIB/Devel/NYTProf.pm"
fi
ok "cargo-free product scripts installed under isolated prefix"

# Installed engine query must consume real golden JSONL (not a stub).
unset PERL5LIB || true
echo "running: installed nytprof-engine query --json --jsonl (golden default-calls1)"
set +e
PATH="$NOCARGO_PATH" "$ENGINE" query --json --jsonl "$JSONL" \
  >"$WORKDIR/query.json" 2>"$WORKDIR/query.json.err"
QRC=$?
set -e
if [[ "$QRC" -ne 0 ]]; then
  cat "$WORKDIR/query.json.err" >&2 || true
  fail "installed nytprof-engine query --json --jsonl failed (rc=$QRC)"
fi
LEAF="$(perl -ne 'print $1 if /"leaf_returns"\s*:\s*(\d+)/' "$WORKDIR/query.json")"
MID="$(perl -ne 'print $1 if /"mid_returns"\s*:\s*(\d+)/' "$WORKDIR/query.json")"
EDGE="$(perl -ne 'print $1 if /"mid_leaf_edge"\s*:\s*(\d+)/' "$WORKDIR/query.json")"
echo "query --json --jsonl: leaf_returns=${LEAF:-?} mid_returns=${MID:-?} mid_leaf_edge=${EDGE:-?}"
[[ "$LEAF" == "15" ]] || fail "leaf_returns=$LEAF (want 15) from installed engine + golden JSONL"
[[ "$MID" == "3" ]] || fail "mid_returns=$MID (want 3) from installed engine + golden JSONL"
[[ "$EDGE" == "15" ]] || fail "mid_leaf_edge=$EDGE (want 15) from installed engine + golden JSONL"
ok "installed nytprof-engine query of real golden JSONL: leaf 15 / mid 3 / mid→leaf 15"

# Wrappers must be product sibling-engine execs, not 6.15 nytprofhtml.
grep -F -q 'nytprof-engine' "$HTML" \
  || fail "installed nytprofhtml is not a nytprof-engine wrapper"
grep -E -q 'Devel::NYTProf::Reader|nytprofhtml -' "$HTML" \
  && fail "installed nytprofhtml looks like oracle 6.15 nytprofhtml"
set +e
PATH="$NOCARGO_PATH" "$HTML" -h >"$WORKDIR/html.help" 2>&1
HRC=$?
set -e
grep -F -q 'nytprof-engine' "$WORKDIR/html.help" \
  || fail "installed nytprofhtml -h did not reach nytprof-engine"
ok "installed nytprofhtml wrapper is executable and reaches nytprof-engine (rc=$HRC)"

# Optional MakeMaker target into a second isolated prefix (cargo-free).
PREFIX2="$WORKDIR/prefix-make"
if [[ "$HAD_MAKEFILE" -eq 0 ]]; then
  echo "running: PATH=<cargo-hidden> NYTPROF_NATIVE=0 perl Makefile.PL && make install-product-scripts"
  set +e
  OUTMM="$(
    cd "$ROOT" && PATH="$NOCARGO_PATH" NYTPROF_NATIVE=0 perl Makefile.PL 2>&1
  )"
  RCMM=$?
  set -e
  printf '%s\n' "$OUTMM"
  [[ "$RCMM" -eq 0 ]] || fail "NYTPROF_NATIVE=0 perl Makefile.PL failed without cargo (rc=$RCMM)"
  SMOKE_OWNED_MAKEFILE=1
  [[ -f "$ROOT/nytprof-packaging.mode" ]] || fail "missing nytprof-packaging.mode"
  grep -q 'packaging_i02=1' "$ROOT/nytprof-packaging.mode" \
    || fail "expected packaging_i02=1 stamp"
  grep -q 'packaging_i03=1' "$ROOT/nytprof-packaging.mode" \
    || fail "expected packaging_i03=1 stamp"
  grep -q 'full_build003=0' "$ROOT/nytprof-packaging.mode" \
    || fail "expected full_build003=0"
  NYTPROF_PREFIX="$PREFIX2" PATH="$NOCARGO_PATH" make install-product-scripts
  [[ -x "$PREFIX2/bin/nytprof-engine" ]] || fail "make install-product-scripts missing engine"
  [[ -f "$PREFIX2/lib/perl5/Devel/NYTProf/EngineDispatch.pm" ]] \
    || fail "make install-product-scripts missing EngineDispatch.pm"
  ok "make install-product-scripts is cargo-free"
else
  echo "SKIP: Makefile already present — not reconfiguring; installer path already proven"
fi

# Optional native csv/html/query on real nytprof.out when CLI is discoverable
# without putting crates/ on PERL5LIB.
NATIVE=""
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  NATIVE="${NYTPROF_NATIVE_CLI}"
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  NATIVE="$ROOT/prefix/bin/nytprof-cli"
elif [[ -x "$ROOT/target/debug/nytprof-cli" ]]; then
  NATIVE="$ROOT/target/debug/nytprof-cli"
elif [[ -x "$ROOT/target/release/nytprof-cli" ]]; then
  NATIVE="$ROOT/target/release/nytprof-cli"
elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
  NATIVE="$ROOT/target/debug/nytprof-dump"
elif [[ -x "$ROOT/target/release/nytprof-dump" ]]; then
  NATIVE="$ROOT/target/release/nytprof-dump"
fi

if [[ -n "$NATIVE" ]]; then
  echo "optional native: NYTPROF_NATIVE_CLI=$NATIVE"
  set +e
  PATH="$NOCARGO_PATH" NYTPROF_NATIVE_CLI="$NATIVE" \
    "$ENGINE" query --json "$FIXTURE" \
    >"$WORKDIR/query-native.json" 2>"$WORKDIR/query-native.json.err"
  NRC=$?
  set -e
  if [[ "$NRC" -ne 0 ]]; then
    cat "$WORKDIR/query-native.json.err" >&2 || true
    fail "installed engine query of real nytprof.out failed (rc=$NRC)"
  fi
  NLEAF="$(perl -ne 'print $1 if /"leaf_returns"\s*:\s*(\d+)/' "$WORKDIR/query-native.json")"
  NMID="$(perl -ne 'print $1 if /"mid_returns"\s*:\s*(\d+)/' "$WORKDIR/query-native.json")"
  NEDGE="$(perl -ne 'print $1 if /"mid_leaf_edge"\s*:\s*(\d+)/' "$WORKDIR/query-native.json")"
  echo "query --json (native dump): leaf_returns=${NLEAF:-?} mid_returns=${NMID:-?} mid_leaf_edge=${NEDGE:-?}"
  [[ "$NLEAF" == "15" ]] || fail "native-path leaf_returns=$NLEAF (want 15)"
  [[ "$NMID" == "3" ]] || fail "native-path mid_returns=$NMID (want 3)"
  [[ "$NEDGE" == "15" ]] || fail "native-path mid_leaf_edge=$NEDGE (want 15)"
  ok "installed engine + discoverable native CLI: fixture nytprof.out 15/3/15"
else
  echo "SKIP: no native CLI discoverable without crates/ on PERL5LIB — jsonl 15/3/15 already proven"
fi

print_residuals
ok "I03-DIST-SCRIPTS"
exit 0

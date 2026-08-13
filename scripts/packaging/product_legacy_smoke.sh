#!/usr/bin/env bash
# Product legacy-only smoke (P-PRODUCT-LEGACY) — I01 install+attach.
#
# Cargo-free MakeMaker/prefix install of product Devel::NYTProf, then live
# perl -d:NYTProfM with NYTPROF file= on default-calls1-shaped work.
# Shipped dump/report of those bytes: leaf 15 / mid 3 / mid→leaf 15.
#
# When CC + Perl XS headers exist: install_product_xs.sh into an isolated
# prefix and attach. When missing: honest SKIP: (exit 0).
#
# Flavor stub only (I01 installs D1-B / -lz only):
#   PRODUCT_D1_FLAVOR=A|B          (default B)
#   --flavor=d1-a|--flavor=d1-b|--flavor=A|--flavor=B
#
# Exit 0: install+attach pass, or honest skip (no CC / no XS headers).
# Exit 1: install / attach / parity failure.
# Exit 2: wrapper misuse (unknown flag) or crates/ on PERL5LIB.
#
# Never required cargo. Never puts crates/ on PERL5LIB.
# Not wired into dual_path_smoke.sh or offline_gate.sh (S2 not claimed).
# legacy_only_smoke.sh remains P-ORACLE forever — this script is not that path.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

INSTALL_SH="$ROOT/scripts/packaging/install_product_xs.sh"
WORKLOAD="$ROOT/fixtures/v5/default-calls1/workload.pl"

usage() {
  cat <<'EOF'
Usage: product_legacy_smoke.sh [--flavor=d1-a|d1-b|A|B]

I01 P-PRODUCT-LEGACY: cargo-free product XS install + live -d:NYTProfM attach.
Honest skip when CC / Perl XS headers are absent.
EOF
}

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

normalize_flavor() {
  local raw="$1"
  case "$raw" in
    A|a|d1-a|D1-A|D1-a)
      printf '%s\n' 'd1-a'
      ;;
    B|b|d1-b|D1-B|D1-b|'')
      printf '%s\n' 'd1-b'
      ;;
    *)
      printf 'ERROR: unknown PRODUCT_D1_FLAVOR / --flavor value: %s (want A|B|d1-a|d1-b)\n' "$raw" >&2
      exit 2
      ;;
  esac
}

FLAVOR_IN="${PRODUCT_D1_FLAVOR:-B}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --flavor=*)
      FLAVOR_IN="${1#--flavor=}"
      shift
      ;;
    --flavor)
      if [[ $# -lt 2 ]]; then
        printf 'ERROR: --flavor requires a value (d1-a|d1-b|A|B)\n' >&2
        exit 2
      fi
      FLAVOR_IN="$2"
      shift 2
      ;;
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

FLAVOR_STUB="$(normalize_flavor "$FLAVOR_IN")"

echo "product_legacy_smoke: repo root $ROOT"
echo "phase: I01"
echo "flavor_stub: ${FLAVOR_STUB}"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; cargo is not required for install"
echo "P-ORACLE remains scripts/packaging/legacy_only_smoke.sh (forever)"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*)
      fail2 "PERL5LIB must not contain /crates/: $PERL5LIB"
      ;;
  esac
fi

[[ -f "$INSTALL_SH" ]] || fail "missing $INSTALL_SH"
[[ -x "$INSTALL_SH" ]] || fail "not executable: $INSTALL_SH"
[[ -f "$WORKLOAD" ]] || fail "missing default-calls1 workload $WORKLOAD"
ok "install_product_xs.sh and fixture workload present"

resolve_cc() {
  if [[ -n "${CC-}" ]] && command -v "$CC" >/dev/null 2>&1; then
    printf '%s\n' "$CC"
    return 0
  fi
  for c in cc gcc clang; do
    if command -v "$c" >/dev/null 2>&1; then
      printf '%s\n' "$c"
      return 0
    fi
  done
  return 1
}

print_residuals() {
  echo "NOT-YET: BUILD-003-FULL / full_build003=1"
  echo "NOT-YET: CPAN-TRIAL-READY / EL8 RPM"
  echo "NOT-YET: S2 dual_path primary → P-PRODUCT-LEGACY"
  echo "NOT-YET: mid-deflate continue-in-child / TEST-018 / blocks-780"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — product XS not installed"
  echo "SKIP: P-PRODUCT-LEGACY (honest skip — no CC)"
  echo "product_xs_attach: no"
  print_residuals
  ok "product_legacy_smoke completed (skip — no CC)"
  exit 0
fi
ok "C toolchain: $CC_BIN"

have_xs_headers=0
if command -v perl >/dev/null 2>&1; then
  if perl -MConfig -e 'exit((-f "$Config{archlibexp}/CORE/EXTERN.h") ? 0 : 1)'; then
    have_xs_headers=1
  fi
fi

if [[ "$have_xs_headers" -ne 1 ]]; then
  echo "SKIP: perl XS headers (EXTERN.h) not present — product XS not installed"
  echo "SKIP: P-PRODUCT-LEGACY (honest skip — no XS headers)"
  echo "product_xs_attach: no"
  print_residuals
  ok "product_legacy_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "perl + EXTERN.h present"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-i01-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

PREFIX="$WORKDIR/prefix"
echo "NYTPROF_PREFIX=$PREFIX"
echo "running: install_product_xs.sh (no cargo)"
NYTPROF_PREFIX="$PREFIX" bash "$INSTALL_SH"
LIB="$PREFIX/lib/perl5"
[[ -f "$LIB/Devel/NYTProfM.pm" ]] || fail "install did not write $LIB/Devel/NYTProfM.pm"
[[ -f "$LIB/Devel/NYTProfM/Core.pm" ]] || fail "install did not write Core.pm"
[[ -f "$LIB/auto/Devel/NYTProfM/NYTProfM.so" ]] || fail "install did not write NYTProfM.so"
[[ -f "$PREFIX/nytprof-product-xs.install" ]] || fail "missing install stamp"
grep -F -q 'full_build003=0' "$PREFIX/nytprof-product-xs.install" \
  || fail "install stamp missing full_build003=0"
grep -F -q 'cargo_required=0' "$PREFIX/nytprof-product-xs.install" \
  || fail "install stamp missing cargo_required=0"
ok "cargo-free product XS installed under isolated prefix"

CLI_CMD=()
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI_CMD=("${NYTPROF_NATIVE_CLI}")
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/prefix/bin/nytprof-cli")
elif [[ -x "$ROOT/target/debug/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-cli")
elif [[ -x "$ROOT/target/release/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/target/release/nytprof-cli")
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  CLI_CMD=(cargo run -q -p nytprof-cli --)
else
  fail "no shipped dump/report (looked for prefix/bin/nytprof-cli, target/{debug,release}/nytprof-cli, cargo)"
fi
echo "dump/report CLI: ${CLI_CMD[*]}"

unset PERL5OPT || true
export PERL5LIB="$LIB"
echo "PERL5LIB=$PERL5LIB"

# Identity: product prefix, not oracle, not crates, not collector/build dest.
set +e
STAMP_OUT="$(
  cd "$WORKDIR" && env -u NYTPROF perl -I"$LIB" -d:NYTProfM -e '
    my $inc = $INC{"Devel/NYTProfM.pm"} // "";
    print "INC=", $inc, "\n";
    my $load = ($Devel::NYTProfM::PRODUCT_XS_LOAD ? 1 : 0);
    print "PRODUCT_XS_LOAD=", $load, "\n";
    die "PRODUCT_XS_LOAD stamp missing\n" unless $load;
    print "I01_STAMP_OK\n";
  ' 2>&1
)"
STAMP_RC=$?
set -e
printf '%s\n' "$STAMP_OUT"
[[ "$STAMP_RC" -eq 0 ]] || fail "prefix load probe exited $STAMP_RC"
INC_LINE="$(printf '%s\n' "$STAMP_OUT" | grep -E '^INC=' | tail -n1 || true)"
[[ -n "$INC_LINE" ]] || fail "prefix load did not print INC="
if grep -F -q 'baseline/6.15/install' <<<"$INC_LINE"; then
  fail "loaded Devel/NYTProfM.pm is the 6.15 oracle pin: $INC_LINE"
fi
if grep -F -q '/crates/' <<<"$INC_LINE"; then
  fail "loaded module path contains /crates/: $INC_LINE"
fi
if ! grep -F -q "$LIB/Devel/NYTProfM.pm" <<<"$INC_LINE"; then
  fail "loaded Devel/NYTProfM.pm is not the product prefix ($LIB): $INC_LINE"
fi
ok "product prefix module path (not oracle pin, not crates/)"

PROFILE="$WORKDIR/nytprof.out"
set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}" perl -I"$LIB" -d:NYTProfM "$WORKLOAD" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"
[[ "$RUN_RC" -eq 0 ]] || fail "perl -d:NYTProfM from product prefix exited $RUN_RC"
[[ -f "$PROFILE" ]] || fail "NYTPROF file= did not write $PROFILE"
magic="$(head -c 9 "$PROFILE" || true)"
[[ "$magic" == "NYTProf 5" ]] || fail "produced bytes must start with NYTProf 5 (got $(printf %q "$magic"))"
ok "prefix -d:NYTProfM wrote NYTProf 5"

set +e
"${CLI_CMD[@]}" report --json "$PROFILE" >"$WORKDIR/report.json" 2>"$WORKDIR/report.json.err"
JSON_RC=$?
set -e
if [[ "$JSON_RC" -ne 0 ]]; then
  cat "$WORKDIR/report.json.err" >&2 || true
  fail "nytprof-cli report --json failed on prefix-produced profile (rc=$JSON_RC)"
fi
LEAF="$(perl -ne 'print $1 if /"leaf_returns"\s*:\s*(\d+)/' "$WORKDIR/report.json")"
MID="$(perl -ne 'print $1 if /"mid_returns"\s*:\s*(\d+)/' "$WORKDIR/report.json")"
EDGE="$(perl -ne 'print $1 if /"mid_leaf_edge"\s*:\s*(\d+)/' "$WORKDIR/report.json")"
echo "report --json: leaf_returns=${LEAF:-?} mid_returns=${MID:-?} mid_leaf_edge=${EDGE:-?}"
[[ "$LEAF" == "15" ]] || fail "leaf_returns=$LEAF (want 15) from prefix-produced profile"
[[ "$MID" == "3" ]] || fail "mid_returns=$MID (want 3) from prefix-produced profile"
[[ "$EDGE" == "15" ]] || fail "mid_leaf_edge=$EDGE (want 15) from prefix-produced profile"
ok "prefix-produced profile: leaf 15 / mid 3 / mid→leaf 15"

# G03a: no file= still no nytprof.out from the installed module
LOAD_CWD="$(mktemp -d "$WORKDIR/g03a-load-XXXXXX")"
set +e
LOAD_OUT="$(
  cd "$LOAD_CWD" && env -u NYTPROF perl -I"$LIB" -d:NYTProfM -e 'print "ok\n"' 2>&1
)"
LOAD_RC=$?
set -e
[[ "$LOAD_RC" -eq 0 ]] || fail "G03a trivial -e from prefix exited $LOAD_RC"
grep -F -q 'ok' <<<"$LOAD_OUT" || fail "G03a trivial -e missing stdout ok"
if [[ -e "$LOAD_CWD/nytprof.out" ]]; then
  fail "G03a must not write nytprof.out from product prefix"
fi
ok "G03a trivial -e from prefix still writes no nytprof.out"

echo "product_xs_attach: yes"
echo "PRODUCT-LEGACY-SMOKE"
print_residuals
ok "P-PRODUCT-LEGACY install+attach"
exit 0

#!/usr/bin/env bash
# Product attach smoke — G03a load via real `perl -d:NYTProfM`.
#
# Proves product Devel::NYTProf loads and a trivial -e exits 0.
# G03a load-only: no NYTPROF file=, PRODUCT_XS_ATTACH stays 0, no nytprof.out.
# Live attach parity is G04 / g04_v5_parity_smoke.sh. Flavor is still a stub.
#
# When CC + Perl XS headers exist:
#   make -C collector xs-nytprof, then isolated perl -d:NYTProfM in a temp cwd.
# When missing: honest SKIP: after source-file asserts (exit 0).
#
# Exit 0: G03a load pass, or honest skip (no CC / no XS headers).
# Exit 1: load / identity / stamp / stray profile-file failure.
# Exit 2: wrapper misuse (unknown flag) or crates/ on PERL5LIB.
#
# Never required cargo. Never puts crates/ on PERL5LIB.
# Not wired into dual_path_smoke.sh or offline_gate.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

COLLECTOR="$ROOT/collector"
MAKEFILE="$COLLECTOR/Makefile"
NYTP_DEST="$COLLECTOR/build/xs-nytprof"
NYTP_SO="$NYTP_DEST/auto/Devel/NYTProfM/NYTProfM.so"
NYTP_PM_SRC="$COLLECTOR/xs/Devel/NYTProfM.pm"
NYTP_CORE_SRC="$COLLECTOR/xs/Devel/NYTProfM/Core.pm"
NYTP_XS="$COLLECTOR/xs/NYTProf.xs"

usage() {
  cat <<'EOF'
Usage: product_attach_smoke.sh [--flavor=d1-a|d1-b|A|B]

G03a load smoke: real perl -d:NYTProfM (product tree) without file=.
Live attach parity is g04_v5_parity_smoke.sh. Flavor is a stub only.
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

echo "product_attach_smoke: repo root $ROOT"
echo "phase: S0/S1"
echo "flavor_stub: ${FLAVOR_STUB}"
echo "product_xs_attach: no"
echo "product_xs_attach: not-ready"
echo "collection_default remains v5; dual_path stays oracle-primary until S2"
echo "never crates/ on PERL5LIB; cargo is not required"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*)
      fail2 "PERL5LIB must not contain /crates/: $PERL5LIB"
      ;;
  esac
fi

# Sources + Makefile target must exist even on honest skip.
[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$NYTP_XS" ]] || fail "missing $NYTP_XS"
[[ -f "$NYTP_PM_SRC" ]] || fail "missing $NYTP_PM_SRC"
[[ -f "$NYTP_CORE_SRC" ]] || fail "missing $NYTP_CORE_SRC"
grep -q 'xs-nytprof' "$MAKEFILE" || fail "Makefile missing xs-nytprof target"
grep -q 'libnytp_sink_v5.a' "$MAKEFILE" || fail "Makefile missing libnytp_sink_v5.a (D1-B link)"
ok "G03a debugger sources and Makefile target present"

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

print_attach_residuals() {
  echo "G03a load-only (no file=); live attach: g04_v5_parity_smoke.sh"
  echo "G05 options/format: g05_options_format_smoke.sh"
  echo "G06 fork/addpid: g06_fork_addpid_smoke.sh"
  echo "NOT-YET: mid-deflate continue-in-child / TEST-018 / full opcode"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — G03a debugger .so not built"
  echo "  (honest skip; G03a load requires xs-nytprof)"
  print_attach_residuals
  ok "product_attach_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present — G03a debugger .so not built"
  echo "  (honest skip; G03a load requires xs-nytprof)"
  print_attach_residuals
  ok "product_attach_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "perl + EXTERN.h present"

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
[[ -f "$NYTP_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM.pm"
[[ -f "$NYTP_DEST/Devel/NYTProfM/Core.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM/Core.pm"
ok "xs-nytprof produced .so + .pm under collector/build/xs-nytprof"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g03a-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

# Isolated product @INC only. Never baseline/6.15/install, never crates/.
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

echo "workdir: $WORKDIR"
echo "PERL5LIB=$PERL5LIB"
echo "running: perl -I${NYTP_DEST} -d:NYTProfM -e '(G03a load probe)'"

set +e
LOAD_OUT="$(
  cd "$WORKDIR" && perl -I"$NYTP_DEST" -d:NYTProfM -e '
    my $inc = $INC{"Devel/NYTProfM.pm"} // "";
    print "INC=", $inc, "\n";
    my $load = ($Devel::NYTProfM::PRODUCT_XS_LOAD ? 1 : 0);
    my $attach = (defined $Devel::NYTProfM::PRODUCT_XS_ATTACH && $Devel::NYTProfM::PRODUCT_XS_ATTACH) ? 1 : 0;
    print "PRODUCT_XS_LOAD=", $load, "\n";
    print "PRODUCT_XS_ATTACH=", $attach, "\n";
    die "PRODUCT_XS_LOAD stamp missing\n" unless $load;
    die "PRODUCT_XS_ATTACH must stay false\n" if $attach;
    print "G03A_LOAD_STAMP\n";
  ' 2>&1
)"
LOAD_RC=$?
set -e
printf '%s\n' "$LOAD_OUT"

[[ "$LOAD_RC" -eq 0 ]] || fail "perl -d:NYTProfM exited $LOAD_RC (want 0)"

INC_LINE="$(printf '%s\n' "$LOAD_OUT" | grep -E '^INC=' | tail -n1 || true)"
[[ -n "$INC_LINE" ]] || fail "perl -d:NYTProfM did not print INC="
if grep -F -q 'baseline/6.15/install' <<<"$INC_LINE"; then
  fail "loaded Devel/NYTProfM.pm is the 6.15 oracle pin: $INC_LINE"
fi
if ! grep -F -q 'collector/build/xs-nytprof' <<<"$INC_LINE"; then
  fail "loaded Devel/NYTProfM.pm is not the product dest (want collector/build/xs-nytprof): $INC_LINE"
fi
ok "product module path (not baseline/6.15/install)"

grep -F -q 'PRODUCT_XS_LOAD=1' <<<"$LOAD_OUT" \
  || fail "missing PRODUCT_XS_LOAD=1 stamp"
grep -F -q 'G03A_LOAD_STAMP' <<<"$LOAD_OUT" \
  || fail "missing G03A_LOAD_STAMP"
if grep -F -q 'PRODUCT_XS_ATTACH=1' <<<"$LOAD_OUT"; then
  fail "PRODUCT_XS_ATTACH must stay 0 (G03a is load-only)"
fi
ok "G03a load stamp; PRODUCT_XS_ATTACH stays false"

if [[ -e "$WORKDIR/nytprof.out" ]]; then
  fail "G03a must not write nytprof.out (found $WORKDIR/nytprof.out)"
fi
if compgen -G "$WORKDIR/nytprof*" > /dev/null; then
  fail "G03a must not write profile files under workdir: $(compgen -G "$WORKDIR/nytprof*" | tr '\n' ' ')"
fi
ok "no profile file written (nytprof.out absent)"

if grep -F -q 'OK: attach works' <<<"$LOAD_OUT"; then
  fail "perl -d:NYTProfM output must not contain OK: attach works"
fi

print_attach_residuals
ok "G03a load"
exit 0

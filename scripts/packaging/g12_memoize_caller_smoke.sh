#!/usr/bin/env bash
# PR-12 — Memoize::memoize uses caller() as the package of the named
# function. Wrapping it with &$raw makes caller DB, so
#   memoize('expensive')
# croaks: Cannot operate on nonexistent function `expensive'
# even though main::expensive exists (works without -d:NYTProfM).
#
# Drives real `perl -d:NYTProfM` (product tree) with NYTPROF file=.
# Isolated product @INC. Never crates/. collection_default stays v5.
# Not in dual_path / offline_gate.
#
# Exit 0: pass, or honest skip (no CC / no XS headers).
# Exit 1: compile / attach / Memoize failure.
# Exit 2: wrapper misuse or crates/ on PERL5LIB.
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
Usage: g12_memoize_caller_smoke.sh

PR-12: Memoize::memoize under live perl -d:NYTProfM (caller is not DB).
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

echo "g12_memoize_caller_smoke: repo root $ROOT"
echo "collection_default remains v5; never crates/ on PERL5LIB"
echo "PR-12: goto Memoize:: so memoize('fn') does not look up DB::fn"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*)
      fail2 "PERL5LIB must not contain /crates/: $PERL5LIB"
      ;;
  esac
fi

[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$NYTP_XS" ]] || fail "missing $NYTP_XS"
[[ -f "$NYTP_PM_SRC" ]] || fail "missing $NYTP_PM_SRC"
[[ -f "$NYTP_CORE_SRC" ]] || fail "missing $NYTP_CORE_SRC"

grep -F -q 'Memoize(?:::|\z)' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing Memoize(?:::|\\z) on _product_needs_goto"
ok "PR-12 debugger sources: Memoize on the goto list"

print_residuals() {
  echo "G07 Getopt/Exporter compile-safe: g07_getopt_compile_smoke.sh"
  echo "G11 GP-less stash: g11_nodebug_stash_nogp_smoke.sh"
  echo "NOT-YET: full 6.15 opcode/entersub / XSUB / leavesub"
}

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

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — G12 debugger .so not built"
  print_residuals
  ok "g12_memoize_caller_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present — G12 debugger .so not built"
  print_residuals
  ok "g12_memoize_caller_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "perl + EXTERN.h present"

if ! perl -e 'require Memoize; 1' >/dev/null 2>&1; then
  echo "SKIP: Memoize not installed — cannot drive live memoize() attach"
  print_residuals
  ok "g12_memoize_caller_smoke completed (skip — no Memoize)"
  exit 0
fi
ok "Memoize loadable"

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
[[ -f "$NYTP_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM.pm"
ok "xs-nytprof produced .so + .pm under collector/build/xs-nytprof"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g12-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

PROFILE="$WORKDIR/nytprof.out"
WORKLOAD="$WORKDIR/memoize.pl"
cat >"$WORKLOAD" <<'PL'
use strict;
use warnings;
use Memoize;

sub expensive {
    my ($n) = @_;
    return $n + 1;
}

memoize('expensive');
my $a = expensive(1);
Memoize::flush_cache('expensive');
my $b = expensive(1);
die "sum" unless $a == 2 && $b == 2;

Memoize::unmemoize('expensive');
my $c = expensive(3);
die "after-unmemoize" unless $c == 4;

Memoize::memoize('expensive');
die "qualified" unless expensive(4) == 5;

print "ok-memoize\n";
PL

echo "workdir: $WORKDIR"
echo "running: NYTPROF=file=${PROFILE} perl -d:NYTProfM <Memoize::memoize>"

set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}" perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"

if grep -F -q 'Cannot operate on nonexistent function' <<<"$RUN_OUT"; then
  fail "Memoize still sees caller=DB (Cannot operate on nonexistent function)"
fi
[[ "$RUN_RC" -eq 0 ]] || fail "perl -d:NYTProfM Memoize exited $RUN_RC (want 0)"
grep -F -q 'ok-memoize' <<<"$RUN_OUT" || fail "workload did not print ok-memoize"
ok "live perl -d:NYTProfM memoize('expensive') used the real package"

[[ -f "$PROFILE" ]] || fail "NYTPROF file= did not write $PROFILE"
magic="$(head -c 9 "$PROFILE" || true)"
[[ "$magic" == "NYTProf 5" ]] || fail "produced bytes must start with NYTProf 5 (got $(printf %q "$magic"))"
ok "produced bytes start with NYTProf 5"

print_residuals
ok "G12 Memoize caller-safe attach"
exit 0

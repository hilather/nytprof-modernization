#!/usr/bin/env bash
# PR-13 — Workload wrap must not insert an eval frame around &$raw.
# caller() skips package-DB sub frames but not CXt_EVAL, so loggers
# that do caller(0) reported Devel/NYTProfM.pm:308 (the &$raw line).
#
# Drives real `perl -d:NYTProfM` with NYTPROF file=. Isolated product
# @INC. Never crates/. collection_default stays v5.
# Not in dual_path / offline_gate.
#
# Exit 0: pass, or honest skip (no CC / no XS headers).
# Exit 1: attach / caller / die-path failure.
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
Usage: g13_logger_caller_smoke.sh

PR-13: logger caller() under live -d:NYTProfM is the app, not NYTProfM.pm.
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

echo "g13_logger_caller_smoke: repo root $ROOT"
echo "collection_default remains v5; never crates/ on PERL5LIB"
echo 'PR-13: no eval around &$raw (caller-visible logger frame)'

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

if grep -n '^[[:space:]]*\$ok = eval {' "$NYTP_PM_SRC" | grep -q .; then
  fail "NYTProfM.pm still eval-wraps the callee (caller-visible NYTProfM.pm frame)"
fi
grep -q 'ProductWrapGuard' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing ProductWrapGuard (die-path SUB_RETURN)"
ok 'PR-13 sources: no eval around callee; DESTROY guard present'

print_residuals() {
  echo "G04 attach 15/3/15: g04_v5_parity_smoke.sh"
  echo "G12 Memoize caller: g12_memoize_caller_smoke.sh"
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
  echo "SKIP: no C toolchain — G13 debugger .so not built"
  print_residuals
  ok "g13_logger_caller_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers not present — G13 debugger .so not built"
  print_residuals
  ok "g13_logger_caller_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "perl + EXTERN.h present"

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
[[ -f "$NYTP_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM.pm"
ok "xs-nytprof produced .so + .pm"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g13-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

PROFILE="$WORKDIR/nytprof.out"
WORKLOAD="$WORKDIR/logger.pl"
cat >"$WORKLOAD" <<'END_LOGGER'
use strict;
use warnings;

package MyLog;
sub info {
    my ( $self, $msg ) = @_;
    my ( $pkg, $file, $line ) = caller(0);
    print "LOG $pkg $file:$line $msg\n";
    if ( defined $file && $file =~ /NYTProfM\.pm/ ) {
        die "logger caller is NYTProfM.pm:$line (eval wrap still visible)\n";
    }
    if ( !defined $pkg || $pkg eq 'DB' ) {
        die "logger caller package is DB (want the app)\n";
    }
}

package main;
sub do_work {
    MyLog->info("hello");
}
do_work();
print "ok-logger\n";
END_LOGGER

echo "running: NYTPROF=file=${PROFILE} perl -d:NYTProfM <logger caller>"

set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}" perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"

if grep -F -q 'NYTProfM.pm' <<<"$RUN_OUT"; then
  fail "output still mentions NYTProfM.pm (caller leak)"
fi
[[ "$RUN_RC" -eq 0 ]] || fail "logger attach exited $RUN_RC"
grep -F -q 'ok-logger' <<<"$RUN_OUT" || fail "did not print ok-logger"
grep -E -q 'LOG main .*logger\.pl:' <<<"$RUN_OUT" \
  || fail "logger caller was not main / logger.pl"
ok "logger caller is the app file, not NYTProfM.pm"

[[ -f "$PROFILE" ]] || fail "did not write $PROFILE"
magic="$(head -c 9 "$PROFILE" || true)"
[[ "$magic" == "NYTProf 5" ]] || fail "want NYTProf 5 (got $(printf %q "$magic"))"
ok "produced bytes start with NYTProf 5"

# Die from a wrapped sub must still seal the profile (DESTROY guard).
DIE_PROFILE="$WORKDIR/nytprof-die.out"
DIE_WORKLOAD="$WORKDIR/die.pl"
cat >"$DIE_WORKLOAD" <<'END_DIE'
use strict;
use warnings;
sub boom { die "boom-from-leaf\n" }
sub mid { boom() }
eval { mid() };
print "ok-die\n";
END_DIE

set +e
DIE_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${DIE_PROFILE}" perl -I"$NYTP_DEST" -d:NYTProfM "$DIE_WORKLOAD" 2>&1
)"
DIE_RC=$?
set -e
printf '%s\n' "$DIE_OUT"
[[ "$DIE_RC" -eq 0 ]] || fail "die-path attach exited $DIE_RC"
grep -F -q 'ok-die' <<<"$DIE_OUT" || fail "die-path did not print ok-die"
[[ -f "$DIE_PROFILE" ]] || fail "die-path did not write profile"
die_magic="$(head -c 9 "$DIE_PROFILE" || true)"
[[ "$die_magic" == "NYTProf 5" ]] || fail "die-path want NYTProf 5"
ok "die from wrapped sub still writes NYTProf 5 (DESTROY finish)"

print_residuals
ok "G13 logger caller is the app (no eval wrap frame)"
exit 0

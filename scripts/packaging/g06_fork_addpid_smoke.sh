#!/usr/bin/env bash
# PR-G06 — Live product fork + addpid=1 via shipped nytp_fork_* (COL-015).
#
# Drives real `perl -d:NYTProfM` (product tree, not baseline/6.15/install)
# with NYTPROF file=<path>:addpid=1 on a program that actually fork()s.
# Parent profile at <file> and child at <file>.<childpid> must both start
# with NYTProf 5. Paths/magic come from those produced files.
#
# Does NOT skip fork via DB::emit_* probes. collection_default stays v5.
# dual_path stays oracle-primary. Mid-deflate-in-child / TEST-018 residual.
#
# Exit 0: G06 pass, or honest skip (no CC / no XS headers).
# Exit 1: fork / addpid / magic failure.
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
Usage: g06_fork_addpid_smoke.sh

G06 live fork+addpid: real perl -d:NYTProfM with file= + addpid=1, parent
and <file>.<childpid> both NYTProf 5.
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

echo "g06_fork_addpid_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary until S2"
echo "never crates/ on PERL5LIB"
echo "G06 live fork+addpid via nytp_fork_*; not mid-deflate-in-child / TEST-018"

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
grep -q 'xs-nytprof' "$MAKEFILE" || fail "Makefile missing xs-nytprof target"
grep -q 'libnytp_sink_v5.a' "$MAKEFILE" || fail "Makefile missing libnytp_sink_v5.a"
grep -q 'nytp_fork_prepare' "$NYTP_XS" || fail "NYTProf.xs missing nytp_fork_prepare"
grep -q 'nytp_fork_resume_parent' "$NYTP_XS" || fail "NYTProf.xs missing nytp_fork_resume_parent"
grep -q 'nytp_fork_resume_child' "$NYTP_XS" || fail "NYTProf.xs missing nytp_fork_resume_child"
grep -q 'nytp_fork_addpid_path' "$NYTP_XS" || fail "NYTProf.xs missing nytp_fork_addpid_path"
grep -q 'nytp_v5_sink_fork_child_reinit' "$NYTP_XS" || fail "NYTProf.xs missing v5 child reinit"
grep -q 'CORE::GLOBAL::fork' "$NYTP_PM_SRC" || fail "NYTProf.pm missing CORE::GLOBAL::fork hook"
grep -q 'PRODUCT_ADDPID' "$NYTP_PM_SRC" || fail "NYTProf.pm missing PRODUCT_ADDPID"
ok "G06 sources, nytp_fork_* wrappers, CORE::GLOBAL::fork hook present"

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
  echo "NOT-YET: mid-deflate compressor continue-in-child"
  echo "NOT-YET: full TEST-018 forkdepth/addpid/merge corpus"
  echo "NOT-YET: sigexit / POSIX _exit / signal-end matrix"
  echo "NOT-YET: blocks-calls1 line5 780 / full opcode"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — G06 debugger .so not built"
  echo "  (honest skip; live fork-attach requires xs-nytprof)"
  print_residuals
  ok "g06_fork_addpid_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present — G06 debugger .so not built"
  echo "  (honest skip; live fork-attach requires xs-nytprof)"
  print_residuals
  ok "g06_fork_addpid_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "perl + EXTERN.h present"

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
[[ -f "$NYTP_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM.pm"
ok "xs-nytprof produced .so + .pm under collector/build/xs-nytprof"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g06-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

PROFILE="$WORKDIR/nytprof.out"
WORKLOAD="$WORKDIR/fork_addpid.pl"
cat >"$WORKLOAD" <<'PL'
use strict;
use warnings;
my $pid = fork();
if (!defined $pid) {
    die "fork failed: $!\n";
}
if ($pid) {
    waitpid($pid, 0);
    print "parent=$$\n";
    print "child=$pid\n";
}
else {
    print "in_child=$$\n";
    exit 0;
}
PL

echo "workdir: $WORKDIR"
echo "PERL5LIB=$PERL5LIB"
echo "running: NYTPROF=file=${PROFILE}:addpid=1 perl -I${NYTP_DEST} -d:NYTProfM <fork workload>"

set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}:addpid=1" \
    perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"
[[ "$RUN_RC" -eq 0 ]] || fail "perl -d:NYTProfM fork workload exited $RUN_RC (want 0)"

INC_LINE="$(printf '%s\n' "$RUN_OUT" | grep -E '^INC=' | tail -n1 || true)"
# workload does not print INC; identity via PERL5LIB dest
if grep -F -q 'baseline/6.15/install' <<<"$RUN_OUT"; then
  fail "output mentions 6.15 oracle pin"
fi
CHILD="$(printf '%s\n' "$RUN_OUT" | sed -n 's/^child=//p' | tail -n1)"
[[ -n "$CHILD" ]] || fail "workload did not print child=<pid>"
[[ "$CHILD" =~ ^[0-9]+$ ]] || fail "child pid not numeric: $CHILD"

[[ -f "$PROFILE" ]] || fail "parent profile missing: $PROFILE"
PARENT_MAGIC="$(head -c 9 "$PROFILE" || true)"
[[ "$PARENT_MAGIC" == "NYTProf 5" ]] || fail "parent bytes must start with NYTProf 5 (got $(printf %q "$PARENT_MAGIC"))"
ok "parent $PROFILE starts with NYTProf 5"

CHILD_PATH="${PROFILE}.${CHILD}"
[[ -f "$CHILD_PATH" ]] || fail "child addpid profile missing: $CHILD_PATH"
CHILD_MAGIC="$(head -c 9 "$CHILD_PATH" || true)"
[[ "$CHILD_MAGIC" == "NYTProf 5" ]] || fail "child bytes must start with NYTProf 5 (got $(printf %q "$CHILD_MAGIC"))"
ok "child $CHILD_PATH starts with NYTProf 5"

if [[ "$PROFILE" == "$CHILD_PATH" ]]; then
  fail "parent and child paths must differ"
fi
ok "addpid child path is <file>.<childpid> (not the parent path)"

# Stamp: addpid session installed hook
set +e
STAMP_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${WORKDIR}/stamp.out:addpid=1" \
    perl -I"$NYTP_DEST" -d:NYTProfM -e '
      my $inc = $INC{"Devel/NYTProfM.pm"} // "";
      print "INC=", $inc, "\n";
      my $add = ($Devel::NYTProfM::PRODUCT_ADDPID ? 1 : 0);
      my $hook = ($Devel::NYTProfM::PRODUCT_FORK_HOOK ? 1 : 0);
      print "PRODUCT_ADDPID=", $add, "\n";
      print "PRODUCT_FORK_HOOK=", $hook, "\n";
      die "PRODUCT_ADDPID must be 1\n" unless $add;
      die "PRODUCT_FORK_HOOK must be 1\n" unless $hook;
      print "G06_STAMP_OK\n";
    ' 2>&1
)"
STAMP_RC=$?
set -e
printf '%s\n' "$STAMP_OUT"
[[ "$STAMP_RC" -eq 0 ]] || fail "G06 stamp probe exited $STAMP_RC"
if grep -F -q 'baseline/6.15/install' <<<"$STAMP_OUT"; then
  fail "stamp loaded 6.15 oracle pin"
fi
if ! grep -F -q 'collector/build/xs-nytprof' <<<"$STAMP_OUT"; then
  fail "stamp INC is not product dest"
fi
ok "product dest; addpid=1 installs CORE::GLOBAL::fork hook"

# G03a: no file= still no nytprof.out
LOAD_CWD="$(mktemp -d "$WORKDIR/g03a-load-XXXXXX")"
set +e
LOAD_OUT="$(
  cd "$LOAD_CWD" && env -u NYTPROF perl -I"$NYTP_DEST" -d:NYTProfM -e 'print "ok\n"' 2>&1
)"
LOAD_RC=$?
set -e
[[ "$LOAD_RC" -eq 0 ]] || fail "G03a trivial -e exited $LOAD_RC"
grep -F -q 'ok' <<<"$LOAD_OUT" || fail "G03a trivial -e missing stdout ok"
if [[ -e "$LOAD_CWD/nytprof.out" ]]; then
  fail "G03a must not write nytprof.out"
fi
ok "G03a trivial -e still writes no nytprof.out"

print_residuals
ok "G06 fork+addpid"
exit 0

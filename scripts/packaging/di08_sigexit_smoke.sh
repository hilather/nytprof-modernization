#!/usr/bin/env bash
# PR-B4 / DI-08 — Live sigexit=1 flushes a valid NYTProf 5 on SIGTERM.
#
# Drives real perl -d:NYTProfM (product xs-nytprof dest). Child sleeps after
# a ready handshake; parent sends SIGTERM. With sigexit=1 the handler must
# call DB::finish_profiler so dump/verify succeed. A silent truncated file
# (magic present but dump/verify fail, or empty file) is a fail.
#
# POSIX::_exit is residual: END does not run; verify must fail-closed
# (never silent OK on an incomplete stream).
#
# Dual_path stays oracle-primary. collection_default stays v5. Not opcode.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

NYTP_DEST="$ROOT/collector/build/xs-nytprof"
NYTP_SO="$NYTP_DEST/auto/Devel/NYTProfM/NYTProfM.so"
NYTP_PM="$ROOT/collector/xs/Devel/NYTProfM.pm"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "di08_sigexit_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary until S2"
echo "never crates/ on PERL5LIB"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$NYTP_PM" ]] || fail "missing $NYTP_PM"
grep -F -q 'sigexit' "$NYTP_PM" || fail "NYTProfM.pm must parse sigexit"
grep -F -q 'PRODUCT_SIGEXIT' "$NYTP_PM" \
  || fail "NYTProfM.pm missing PRODUCT_SIGEXIT stamp / handler (DI-08)"

if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1; then
  echo "SKIP: no C compiler"
  ok "di08 layout (compile skipped)"
  exit 0
fi

echo "make -C collector xs-nytprof"
make -C "$ROOT/collector" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof missing .so"
ok "xs-nytprof ready"

CLI_CMD=()
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI_CMD=("${NYTPROF_NATIVE_CLI}")
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/prefix/bin/nytprof-cli")
elif [[ -x "$ROOT/target/debug/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-cli")
elif command -v cargo >/dev/null 2>&1; then
  CLI_CMD=(cargo run -q -p nytprof-cli --)
else
  fail "no shipped dump/verify CLI"
fi

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-di08-XXXXXX")"
trap 'rm -rf "$WORKDIR"' EXIT

CHILD="$WORKDIR/sleeper.pl"
READY="$WORKDIR/ready"
PROFILE="$WORKDIR/nytprof.out"
cat >"$CHILD" <<'PL'
use strict;
my $ready = $ENV{DI08_READY} or die "DI08_READY";
open my $fh, ">", $ready or die $!;
print {$fh} "ready\n";
close $fh;
sleep 30;
print "slept\n";
PL

unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"
export DI08_READY="$READY"

echo "starting child: NYTPROF=file=…:sigexit=1 perl -d:NYTProfM sleeper"
set +e
NYTPROF="file=${PROFILE}:sigexit=1" perl -I"$NYTP_DEST" -d:NYTProfM "$CHILD" \
  >"$WORKDIR/child.out" 2>"$WORKDIR/child.err" &
CPID=$!
set -e

# Wait for ready handshake (attach + first statement).
for _i in $(seq 1 50); do
  if [[ -f "$READY" ]]; then
    break
  fi
  if ! kill -0 "$CPID" 2>/dev/null; then
    wait "$CPID" || true
    fail "child exited before ready: $(cat "$WORKDIR/child.err" 2>/dev/null || true)"
  fi
  sleep 0.1
done
[[ -f "$READY" ]] || fail "child never wrote ready file"

kill -TERM "$CPID" 2>/dev/null || true
set +e
wait "$CPID"
WAIT_RC=$?
set -e
echo "child wait rc=$WAIT_RC (expect non-zero after SIGTERM+sigexit)"

[[ -f "$PROFILE" ]] || fail "sigexit=1 SIGTERM did not leave a profile file"
magic="$(head -c 9 "$PROFILE" || true)"
[[ "$magic" == "NYTProf 5" ]] || fail "profile magic not NYTProf 5 (got $(printf %q "$magic"))"

set +e
"${CLI_CMD[@]}" dump "$PROFILE" >"$WORKDIR/dump.jsonl" 2>"$WORKDIR/dump.err"
DUMP_RC=$?
"${CLI_CMD[@]}" verify "$PROFILE" >"$WORKDIR/verify.out" 2>"$WORKDIR/verify.err"
VER_RC=$?
set -e
if [[ "$DUMP_RC" -ne 0 ]]; then
  cat "$WORKDIR/dump.err" >&2 || true
  fail "dump failed after sigexit SIGTERM (silent/truncated file is a fail)"
fi
if [[ "$VER_RC" -ne 0 ]]; then
  cat "$WORKDIR/verify.err" >&2 || true
  fail "verify failed after sigexit SIGTERM (incomplete stream is a fail)"
fi
grep -E -q '"tag":[[:space:]]*"SUB_RETURN"|"tag":[[:space:]]*"TIME_LINE"|"tag":[[:space:]]*"PID_' \
  "$WORKDIR/dump.jsonl" \
  || fail "dump after sigexit has no expected tags (empty/truncated body)"
ok "sigexit=1 SIGTERM flushed dumpable NYTProf 5"

# _exit residual: END does not run. Drive a raw exit syscall (not POSIX.pm —
# loading POSIX under -d:NYTProfM hits DynaLoader via our require hook).
# Must not verify-OK an incomplete file.
EXIT_PL="$WORKDIR/uexit.pl"
EXIT_OUT="$WORKDIR/uexit.out"
if [[ "$(uname -s)" != "Linux" ]]; then
  echo "SKIP: raw SYS_exit residual check is Linux-only"
else
  cat >"$EXIT_PL" <<'PL'
# x86_64 SYS_exit=60; i386 SYS_exit=1. Skip END/finish_profiler.
my $nr = ($^O eq 'linux' && pack('P', 0) eq pack('P', 0)) ? 60 : 60;
if (eval { require Config; 1 } && $Config::Config{archname} =~ /64/) {
    $nr = 60;
}
syscall( $nr, 0 );
PL
  set +e
  NYTPROF="file=${EXIT_OUT}:sigexit=1" perl -I"$NYTP_DEST" -d:NYTProfM "$EXIT_PL" \
    >"$WORKDIR/uexit.stdout" 2>"$WORKDIR/uexit.stderr"
  set -e
  if [[ -s "$EXIT_OUT" ]]; then
    set +e
    "${CLI_CMD[@]}" verify "$EXIT_OUT" >"$WORKDIR/uexit.verify" 2>"$WORKDIR/uexit.verify.err"
    UV_RC=$?
    set -e
    if [[ "$UV_RC" -eq 0 ]]; then
      fail "_exit syscall left a verify-OK file (must fail-closed or be empty; residual)"
    fi
    ok "_exit residual: verify fail-closed (not silent OK)"
  else
    ok "_exit residual: no/empty profile (END did not flush)"
  fi
fi

echo "NOT-YET: POSIX::_exit flush / full TEST-018 / mid-deflate-in-child / S2"
ok "DI-08 sigexit SIGTERM flush"

#!/usr/bin/env bash
# INCOMPLETE-STREAM: fail-closed on record-aligned short prefixes.
#
# Contract: docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md
# Board: INCOMPLETE-STREAM
#
# Cases:
#   a) first 500 bytes of fixtures/v5/default-calls1/nytprof.out
#      → verify exit != 0 (and no OK: line)
#      → report exit != 0
#   b) golden default-calls1 → verify exit 0 with OK:
#   c) optional salvage: NYTPROF_ALLOW_INCOMPLETE=1 on prefix
#      → verify exit 0 with INCOMPLETE: (not bare OK: only)
#   d) dump on prefix is allowed to succeed or fail (lenient salvage surface)
#
# Does not require oracle Perl / PERL5LIB.
#
# Usage:
#   bash tools/oracle/selftest_incomplete_stream.sh
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
cd "$ROOT"

FIXTURE="$ROOT/fixtures/v5/default-calls1/nytprof.out"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

need_file() {
  [[ -f "$1" ]] || fail "missing $1"
}

need_file "$FIXTURE"

run_cli() {
  if command -v cargo >/dev/null 2>&1; then
    cargo run -q -p nytprof-cli -- "$@"
  elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
    "$ROOT/prefix/bin/nytprof-cli" "$@"
  elif [[ -x "$ROOT/prefix/bin/nytprof-dump" ]]; then
    "$ROOT/prefix/bin/nytprof-dump" "$@"
  elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
    "$ROOT/target/debug/nytprof-dump" "$@"
  elif [[ -x "$ROOT/target/release/nytprof-dump" ]]; then
    "$ROOT/target/release/nytprof-dump" "$@"
  else
    fail "no cargo and no prefix/target nytprof-cli/nytprof-dump binary found"
  fi
}

expect_fail() {
  local label="$1"
  shift
  local out rc
  set +e
  out=$(run_cli "$@" 2>&1)
  rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then
    fail "$label: expected non-zero exit, got 0; output:
$out"
  fi
  if printf '%s\n' "$out" | grep -q '^OK:'; then
    fail "$label: must not print OK: on incomplete input; output:
$out"
  fi
  ok "$label (exit $rc)"
}

expect_verify_ok() {
  local label="$1"
  local path="$2"
  local out rc
  set +e
  out=$(run_cli verify "$path" 2>&1)
  rc=$?
  set -e
  if [[ "$rc" -ne 0 ]]; then
    fail "$label: expected exit 0, got $rc; output:
$out"
  fi
  if ! printf '%s\n' "$out" | grep -q '^OK:'; then
    fail "$label: expected OK: line; output:
$out"
  fi
  ok "$label"
}

TMP="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-incomplete.XXXXXX")"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

PREFIX="$TMP/prefix-500.out"
python3 - "$FIXTURE" "$PREFIX" <<'PY'
import sys
src, dest = sys.argv[1], sys.argv[2]
data = open(src, "rb").read()
assert len(data) > 500, len(data)
open(dest, "wb").write(data[:500])
PY

log "selftest_incomplete_stream: workdir=$TMP"

# a) incomplete prefix → verify/report fail closed
# Ensure salvage env is unset for default policy.
unset NYTPROF_ALLOW_INCOMPLETE || true
expect_fail "verify incomplete 500-byte prefix" verify "$PREFIX"
expect_fail "report incomplete 500-byte prefix" report "$PREFIX"

# d) dump is lenient — success or failure both acceptable; just run it.
set +e
dump_out=$(run_cli dump "$PREFIX" 2>&1)
dump_rc=$?
set -e
ok "dump incomplete prefix (exit $dump_rc; lenient, not asserted)"

# b) golden still OK
expect_verify_ok "verify golden default-calls1" "$FIXTURE"

# c) salvage env accepts incomplete with INCOMPLETE: note
export NYTPROF_ALLOW_INCOMPLETE=1
set +e
salvage_out=$(run_cli verify "$PREFIX" 2>&1)
salvage_rc=$?
set -e
unset NYTPROF_ALLOW_INCOMPLETE
if [[ "$salvage_rc" -ne 0 ]]; then
  fail "salvage verify expected exit 0, got $salvage_rc; output:
$salvage_out"
fi
if ! printf '%s\n' "$salvage_out" | grep -q 'INCOMPLETE'; then
  fail "salvage verify must mention INCOMPLETE; output:
$salvage_out"
fi
# Must not present as a normal bare OK without incompleteness signal.
if printf '%s\n' "$salvage_out" | grep -q '^OK:' && ! printf '%s\n' "$salvage_out" | grep -qi incomplete; then
  fail "salvage must not look like a normal complete OK only; output:
$salvage_out"
fi
ok "salvage verify incomplete prefix (INCOMPLETE note)"

log "selftest_incomplete_stream: PASS"
exit 0

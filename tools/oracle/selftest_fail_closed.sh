#!/usr/bin/env bash
# COMPAT-010-ERR: fail-closed smoke on corrupt inputs via shipped CLI.
#
# Contract: docs/contracts/COMPAT-010_ERROR_FAIL_CLOSED.md
# Board: COMPAT-010-ERR
#
# Runs `verify` (and dump/report when cheap) on:
#   a) empty tempfile
#   b) half of fixtures/v5/default-calls1/nytprof.out
#   c) bad magic ("NOTPROF 5 0\n")
# Expects exit != 0 for each. Does not require oracle Perl / PERL5LIB.
#
# Usage:
#   bash tools/oracle/selftest_fail_closed.sh
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

# Resolve shipped CLI (prefer cargo run of nytprof-cli package)
run_cli() {
  # args: subcommand path
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

# Expect non-zero exit; capture stdout/stderr so we can assert no OK: line.
expect_fail() {
  local label="$1"
  shift
  local out rc
  set +e
  out=$(run_cli "$@" 2>&1)
  rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then
    fail "$label: expected non-zero exit, got 0; output:\n$out"
  fi
  if printf '%s\n' "$out" | grep -q '^OK:'; then
    fail "$label: must not print OK: on corrupt input; output:\n$out"
  fi
  ok "$label (exit $rc)"
}

TMP="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-fail-closed.XXXXXX")"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

EMPTY="$TMP/empty.out"
: >"$EMPTY"

TRUNC="$TMP/trunc.out"
# half bytes of golden default-calls1
python3 - "$FIXTURE" "$TRUNC" <<'PY'
import sys
src, dest = sys.argv[1], sys.argv[2]
data = open(src, "rb").read()
assert len(data) > 2
open(dest, "wb").write(data[: len(data) // 2])
PY

BAD="$TMP/bad-magic.out"
printf 'NOTPROF 5 0\n' >"$BAD"

log "selftest_fail_closed: workdir=$TMP"

# Primary surface: verify (inspect alias is the same code path)
expect_fail "verify empty" verify "$EMPTY"
expect_fail "verify truncated default-calls1" verify "$TRUNC"
expect_fail "verify bad magic" verify "$BAD"

# Also dump + report (same fail-closed policy)
expect_fail "dump empty" dump "$EMPTY"
expect_fail "dump truncated default-calls1" dump "$TRUNC"
expect_fail "dump bad magic" dump "$BAD"

expect_fail "report empty" report "$EMPTY"
expect_fail "report truncated default-calls1" report "$TRUNC"
expect_fail "report bad magic" report "$BAD"

log "selftest_fail_closed: PASS"
exit 0

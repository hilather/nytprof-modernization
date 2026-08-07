#!/usr/bin/env bash
# Smoke the stable prefix install of nytprof-cli / nytprof-dump.
#
# Spec: docs/schemas/native-install-mvp-v0.md
#
# Expects install_native.sh to have populated $PREFIX/bin (default: $REPO/prefix).
# Prefer the prefix binary via NYTPROF_NATIVE_CLI / find order — does not put
# crates/ on oracle PERL5LIB.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/install_native.sh
#   ./scripts/packaging/native_install_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE="fixtures/v5/default-calls1/nytprof.out"
PREFIX="${PREFIX:-$ROOT/prefix}"
CLI_CLI="$PREFIX/bin/nytprof-cli"
CLI_DUMP="$PREFIX/bin/nytprof-dump"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/$FIXTURE" ]] || fail "missing fixture $FIXTURE"

if [[ -x "$CLI_CLI" ]]; then
  CLI="$CLI_CLI"
elif [[ -x "$CLI_DUMP" ]]; then
  CLI="$CLI_DUMP"
else
  fail "prefix CLI missing; run ./scripts/packaging/install_native.sh first
  looked for: $CLI_CLI and $CLI_DUMP"
fi

ok "using prefix binary: $CLI"

# Isolate discovery: pin env so Perl dispatch / smoke path hits prefix first.
export NYTPROF_NATIVE_CLI="$CLI"
# Do not touch PERL5LIB — this smoke is native-binary only.

REPORT_OUT="$(mktemp)"
VERIFY_OUT="$(mktemp)"
trap 'rm -f "$REPORT_OUT" "$VERIFY_OUT"' EXIT

if ! "$CLI" --engine=native report "$FIXTURE" >"$REPORT_OUT" 2>/tmp/native_install_report.err; then
  cat /tmp/native_install_report.err >&2 || true
  fail "prefix --engine=native report failed"
fi
grep -q 'main::leaf' "$REPORT_OUT" || fail "report missing main::leaf:\n$(cat "$REPORT_OUT")"
grep -q 'returns=15' "$REPORT_OUT" || fail "report missing returns=15:\n$(cat "$REPORT_OUT")"
if grep -q 'main::mid' "$REPORT_OUT"; then
  grep -qE 'main::mid[[:space:]]+returns=3\b' "$REPORT_OUT" \
    || fail "report has main::mid but missing returns=3:\n$(cat "$REPORT_OUT")"
  ok "prefix report: main::leaf returns=15 and main::mid returns=3"
else
  ok "prefix report: main::leaf and returns=15"
fi

if ! "$CLI" --engine=native verify "$FIXTURE" >"$VERIFY_OUT" 2>/tmp/native_install_verify.err; then
  cat /tmp/native_install_verify.err >&2 || true
  fail "prefix --engine=native verify failed"
fi
grep -q 'OK:' "$VERIFY_OUT" || fail "verify missing OK: line:\n$(cat "$VERIFY_OUT")"
ok "prefix verify → OK"

# Optional: Perl facade discovery via prefix (when facade present)
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"
if [[ -f "$ROOT/$ENGINE_BIN" && -d "$ROOT/$ENGINE_LIB" ]]; then
  FACADE_OUT="$(mktemp)"
  trap 'rm -f "$REPORT_OUT" "$VERIFY_OUT" "$FACADE_OUT"' EXIT
  if ! perl -I"$ENGINE_LIB" "$ENGINE_BIN" --engine=native report "$FIXTURE" \
    >"$FACADE_OUT" 2>/tmp/native_install_facade.err; then
    cat /tmp/native_install_facade.err >&2 || true
    fail "nytprof-engine via NYTPROF_NATIVE_CLI (prefix) failed"
  fi
  grep -q 'main::leaf' "$FACADE_OUT" || fail "facade report missing main::leaf"
  grep -q 'returns=15' "$FACADE_OUT" || fail "facade report missing returns=15"
  ok "Perl facade finds prefix CLI via NYTPROF_NATIVE_CLI"
fi

ok "native-install packaging smoke passed"
exit 0

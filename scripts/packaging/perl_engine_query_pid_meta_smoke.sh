#!/usr/bin/env bash
# Perl engine query PID lifecycle + ATTRIBUTE/OPTION packaging smoke
# (PERL-QUERY-PID-META).
#
# Spec: docs/schemas/perl-engine-dispatch-mvp-v0.md
# Data: docs/schemas/perl-jsonl-data-mvp-v0.md
#
# Expands nytprof-engine query default output to also surface dump-derived
# PID lifecycle and ATTRIBUTE/OPTION via JsonlData APIs only
# (pid_start_count, pid_starts, pid_ends, attribute, option, attributes,
# options — no reimplementation; no crates/ on oracle PERL5LIB).
#
# 1. default-calls1 via --jsonl:
#      leaf returns=15, mid returns=3, mid→leaf count=15
#      pid_start_count>=1, pid_end_count>=1
#      matching pid (golden: pid_start 2975381 / pid_end same)
#      at least one attribute (ticks_per_sec) and one option (calls)
# 2. Optional prove: perl/t/engine_query_default_calls1.t
# 3. Optional: default-calls1 native profile path when CLI available
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_engine_query_pid_meta_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

DEFAULT_DIR="fixtures/v5/default-calls1"
DEFAULT_GOLDEN="$DEFAULT_DIR/readstream.jsonl"
DEFAULT_PROFILE="$DEFAULT_DIR/nytprof.out"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"
DISPATCH_PM="$ENGINE_LIB/Devel/NYTProf/EngineDispatch.pm"
DATA_PM="$ENGINE_LIB/Devel/NYTProf/JsonlData.pm"
T="perl/t/engine_query_default_calls1.t"

# Golden dump-derived pid (from committed readstream.jsonl; not invented)
EXPECTED_PID=2975381

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$DEFAULT_GOLDEN" ]] || fail "missing golden dump $DEFAULT_GOLDEN"
[[ -f "$ROOT/$ENGINE_BIN" ]] || fail "missing $ENGINE_BIN"
[[ -f "$ROOT/$DISPATCH_PM" ]] || fail "missing $DISPATCH_PM"
[[ -f "$ROOT/$DATA_PM" ]] || fail "missing $DATA_PM"

ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

# ---------------------------------------------------------------------------
# 1. default-calls1 golden JSONL (returns/edges + PID + attribute/option)
# ---------------------------------------------------------------------------
echo "=== engine query pid/meta: default-calls1 --jsonl ==="
DEF_OUT="$TMPDIR_SMOKE/default_jsonl.out"
DEF_ERR="$TMPDIR_SMOKE/default_jsonl.err"
if ! "${ENGINE[@]}" query --jsonl "$DEFAULT_GOLDEN" >"$DEF_OUT" 2>"$DEF_ERR"; then
  cat "$DEF_ERR" >&2 || true
  cat "$DEF_OUT" >&2 || true
  fail "query --jsonl default-calls1 failed"
fi
cat "$DEF_OUT"

grep -qE 'main::leaf returns=15' "$DEF_OUT" \
  || fail "missing main::leaf returns=15"
grep -qE 'main::mid returns=3' "$DEF_OUT" \
  || fail "missing main::mid returns=3"
grep -qE 'main::mid -> main::leaf count=15' "$DEF_OUT" \
  || fail "missing main::mid -> main::leaf count=15"

# PID lifecycle
grep -qE '^pid_start_count=[1-9][0-9]*$' "$DEF_OUT" \
  || fail "missing pid_start_count>=1"
grep -qE '^pid_end_count=[1-9][0-9]*$' "$DEF_OUT" \
  || fail "missing pid_end_count>=1"
grep -qE "^pid_start pid=${EXPECTED_PID}( |\$)" "$DEF_OUT" \
  || fail "missing pid_start pid=${EXPECTED_PID}"
grep -qE "^pid_end pid=${EXPECTED_PID}( |\$)" "$DEF_OUT" \
  || fail "missing pid_end pid=${EXPECTED_PID}"

# ATTRIBUTE / OPTION (at least key ones from golden)
grep -qE '^attribute ticks_per_sec=' "$DEF_OUT" \
  || fail "missing attribute ticks_per_sec=..."
grep -qE '^option calls=' "$DEF_OUT" \
  || fail "missing option calls=..."

ok "default-calls1 --jsonl: 15/3/15 + pid_start/end ${EXPECTED_PID} + attribute ticks_per_sec + option calls"

# ---------------------------------------------------------------------------
# 2. Optional prove unit/integration test
# ---------------------------------------------------------------------------
if [[ -f "$ROOT/$T" ]]; then
  echo "=== prove $T ==="
  prove -I"$ENGINE_LIB" "$T" || fail "prove $T failed"
  ok "prove $T"
else
  echo "NOTE: $T not present; skipping prove"
fi

# ---------------------------------------------------------------------------
# 3. Optional native profile path (default-calls1) when CLI available
# ---------------------------------------------------------------------------
find_cli() {
  if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
    echo "path:${NYTPROF_NATIVE_CLI}"
    return 0
  fi
  for p in \
    prefix/bin/nytprof-cli \
    prefix/bin/nytprof-dump \
    target/release/nytprof-dump \
    target/debug/nytprof-dump
  do
    if [[ -x "$ROOT/$p" || -f "$ROOT/$p" ]]; then
      echo "path:$ROOT/$p"
      return 0
    fi
  done
  if command -v cargo >/dev/null 2>&1; then
    echo "cargo"
    return 0
  fi
  return 1
}

if CLI_SPEC="$(find_cli)"; then
  echo "=== engine query pid/meta: native profile ($CLI_SPEC) ==="
  [[ -f "$ROOT/$DEFAULT_PROFILE" ]] || fail "missing profile $DEFAULT_PROFILE"
  NAT_OUT="$TMPDIR_SMOKE/query_native.out"
  NAT_ERR="$TMPDIR_SMOKE/query_native.err"
  if ! "${ENGINE[@]}" --engine=native query "$DEFAULT_PROFILE" \
    >"$NAT_OUT" 2>"$NAT_ERR"; then
    cat "$NAT_ERR" >&2 || true
    cat "$NAT_OUT" >&2 || true
    fail "--engine=native query profile failed"
  fi
  cat "$NAT_OUT"
  grep -qE 'main::leaf returns=15' "$NAT_OUT" \
    || fail "native query missing main::leaf returns=15"
  grep -qE '^pid_start_count=[1-9][0-9]*$' "$NAT_OUT" \
    || fail "native query missing pid_start_count>=1"
  grep -qE '^pid_end_count=[1-9][0-9]*$' "$NAT_OUT" \
    || fail "native query missing pid_end_count>=1"
  # Live dump may reassign pid; require start/end present and matching.
  START_PID="$(grep -E '^pid_start pid=' "$NAT_OUT" | head -n1 | sed -E 's/^pid_start pid=([0-9]+).*/\1/')"
  END_PID="$(grep -E '^pid_end pid=' "$NAT_OUT" | head -n1 | sed -E 's/^pid_end pid=([0-9]+).*/\1/')"
  [[ -n "$START_PID" ]] || fail "native query missing pid_start pid=..."
  [[ -n "$END_PID" ]] || fail "native query missing pid_end pid=..."
  [[ "$START_PID" == "$END_PID" ]] \
    || fail "native query start pid ($START_PID) != end pid ($END_PID)"
  grep -qE '^attribute ticks_per_sec=' "$NAT_OUT" \
    || fail "native query missing attribute ticks_per_sec"
  grep -qE '^option calls=' "$NAT_OUT" \
    || fail "native query missing option calls"
  ok "native query profile: 15 + pid lifecycle + attribute/option"
else
  echo "NOTE: no native CLI / cargo; skipped live profile query (golden paths still required)"
fi

if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

ok "perl engine query pid/meta packaging smoke passed"
exit 0

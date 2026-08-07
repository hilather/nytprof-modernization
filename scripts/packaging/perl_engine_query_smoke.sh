#!/usr/bin/env bash
# Perl engine query packaging smoke (PERL-ENGINE-QUERY).
#
# Spec: docs/schemas/perl-engine-dispatch-mvp-v0.md
# Data: docs/schemas/perl-jsonl-data-mvp-v0.md
#
# 1. Golden JSONL path (no cargo): nytprof-engine query --jsonl …
#    → leaf returns=15, mid returns=3, mid→leaf count=15
# 2. Optional prove: perl/t/engine_query_default_calls1.t
# 3. Optional native profile path when CLI available
#
# Never puts crates/ on oracle PERL5LIB. No XS.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_engine_query_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="fixtures/v5/default-calls1"
GOLDEN="$FIXTURE_DIR/readstream.jsonl"
PROFILE="$FIXTURE_DIR/nytprof.out"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"
DISPATCH_PM="$ENGINE_LIB/Devel/NYTProf/EngineDispatch.pm"
DATA_PM="$ENGINE_LIB/Devel/NYTProf/JsonlData.pm"
T="perl/t/engine_query_default_calls1.t"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$GOLDEN" ]] || fail "missing golden dump $GOLDEN"
[[ -f "$ROOT/$ENGINE_BIN" ]] || fail "missing $ENGINE_BIN"
[[ -f "$ROOT/$DISPATCH_PM" ]] || fail "missing $DISPATCH_PM"
[[ -f "$ROOT/$DATA_PM" ]] || fail "missing $DATA_PM"

ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

# ---------------------------------------------------------------------------
# 1. Golden JSONL path (no cargo / no native CLI required)
# ---------------------------------------------------------------------------
echo "=== engine query: golden JSONL path ==="
QUERY_OUT="$TMPDIR_SMOKE/query_jsonl.out"
QUERY_ERR="$TMPDIR_SMOKE/query_jsonl.err"
if ! "${ENGINE[@]}" query --jsonl "$GOLDEN" >"$QUERY_OUT" 2>"$QUERY_ERR"; then
  cat "$QUERY_ERR" >&2 || true
  cat "$QUERY_OUT" >&2 || true
  fail "query --jsonl failed"
fi
cat "$QUERY_OUT"
grep -qE 'main::leaf returns=15' "$QUERY_OUT" \
  || fail "missing main::leaf returns=15"
grep -qE 'main::mid returns=3' "$QUERY_OUT" \
  || fail "missing main::mid returns=3"
grep -qE 'main::mid -> main::leaf count=15' "$QUERY_OUT" \
  || fail "missing main::mid -> main::leaf count=15"
ok "query --jsonl: leaf=15 mid=3 mid→leaf=15"

# data-query alias
if ! "${ENGINE[@]}" data-query --jsonl="$GOLDEN" \
  >"$TMPDIR_SMOKE/alias.out" 2>"$TMPDIR_SMOKE/alias.err"; then
  cat "$TMPDIR_SMOKE/alias.err" >&2 || true
  fail "data-query --jsonl= alias failed"
fi
grep -qE 'main::leaf returns=15' "$TMPDIR_SMOKE/alias.out" \
  || fail "data-query alias missing leaf=15"
ok "data-query --jsonl= alias works"

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
# 3. Native profile path when CLI available
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
  echo "=== engine query: native profile path ($CLI_SPEC) ==="
  [[ -f "$ROOT/$PROFILE" ]] || fail "missing profile $PROFILE"
  NATIVE_OUT="$TMPDIR_SMOKE/query_native.out"
  NATIVE_ERR="$TMPDIR_SMOKE/query_native.err"
  if ! "${ENGINE[@]}" --engine=native query "$PROFILE" \
    >"$NATIVE_OUT" 2>"$NATIVE_ERR"; then
    cat "$NATIVE_ERR" >&2 || true
    cat "$NATIVE_OUT" >&2 || true
    fail "--engine=native query profile failed"
  fi
  cat "$NATIVE_OUT"
  grep -qE 'main::leaf returns=15' "$NATIVE_OUT" \
    || fail "native query missing main::leaf returns=15"
  grep -qE 'main::mid returns=3' "$NATIVE_OUT" \
    || fail "native query missing main::mid returns=3"
  grep -qE 'main::mid -> main::leaf count=15' "$NATIVE_OUT" \
    || fail "native query missing main::mid -> main::leaf count=15"
  ok "query profile: leaf=15 mid=3 mid→leaf=15"
else
  echo "NOTE: no native CLI / cargo; skipped live profile query (golden path still required)"
fi

# Sanity: env must not inject crates/ into any child via this smoke
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

ok "perl engine query packaging smoke passed"
exit 0

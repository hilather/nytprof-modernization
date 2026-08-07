#!/usr/bin/env bash
# Perl engine query expand packaging smoke (PERL-ENGINE-QUERY-EXPAND).
#
# Spec: docs/schemas/perl-engine-dispatch-mvp-v0.md
# Data: docs/schemas/perl-jsonl-data-mvp-v0.md
#
# Expands nytprof-engine query default output beyond returns/edges to surface
# dump-derived sub_defs, source_line, line_calls, and A4b block_line samples
# via JsonlData APIs only (no reimplementation; no crates/ on oracle PERL5LIB).
#
# 1. default-calls1 via --jsonl:
#      leaf returns=15, mid returns=3, mid→leaf count=15
#      sub_def main::leaf fid=1 first=3 last=7
#      sub_def main::mid  fid=1 first=8 last=12
#      source_line 1:5 containing $x++ and 1 .. 50
# 2. blocks-calls1 via --jsonl:
#      line_calls 1:5=780
#      block_line_calls 1:4=810
# 3. Optional: default-calls1 native profile path when CLI available
# 4. Optional prove: perl/t/engine_query_default_calls1.t
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_engine_query_expand_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

DEFAULT_DIR="fixtures/v5/default-calls1"
DEFAULT_GOLDEN="$DEFAULT_DIR/readstream.jsonl"
DEFAULT_PROFILE="$DEFAULT_DIR/nytprof.out"
BLOCKS_DIR="fixtures/v5/blocks-calls1"
BLOCKS_GOLDEN="$BLOCKS_DIR/readstream.jsonl"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"
DISPATCH_PM="$ENGINE_LIB/Devel/NYTProf/EngineDispatch.pm"
DATA_PM="$ENGINE_LIB/Devel/NYTProf/JsonlData.pm"
T="perl/t/engine_query_default_calls1.t"

EXPECTED_SRC='    $x++ for 1 .. 50;'

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$DEFAULT_GOLDEN" ]] || fail "missing golden dump $DEFAULT_GOLDEN"
[[ -f "$ROOT/$BLOCKS_GOLDEN" ]] || fail "missing golden dump $BLOCKS_GOLDEN"
[[ -f "$ROOT/$ENGINE_BIN" ]] || fail "missing $ENGINE_BIN"
[[ -f "$ROOT/$DISPATCH_PM" ]] || fail "missing $DISPATCH_PM"
[[ -f "$ROOT/$DATA_PM" ]] || fail "missing $DATA_PM"

ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

# ---------------------------------------------------------------------------
# 1. default-calls1 golden JSONL (returns/edges + sub_def + source_line)
# ---------------------------------------------------------------------------
echo "=== engine query expand: default-calls1 --jsonl ==="
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
grep -qE 'sub_def main::leaf fid=1 first=3 last=7' "$DEF_OUT" \
  || fail "missing sub_def main::leaf fid=1 first=3 last=7"
grep -qE 'sub_def main::mid fid=1 first=8 last=12' "$DEF_OUT" \
  || fail "missing sub_def main::mid fid=1 first=8 last=12"
grep -qE '^source_line 1:5=' "$DEF_OUT" \
  || fail "missing source_line 1:5= line"
grep -qF "source_line 1:5=${EXPECTED_SRC}" "$DEF_OUT" \
  || fail "source_line 1:5 text mismatch (expected dump hot-loop)"
grep -qE '\$x\+\+' "$DEF_OUT" \
  || fail "source_line missing \$x++"
grep -qE '1 \.\. 50' "$DEF_OUT" \
  || fail "source_line missing 1 .. 50"
# default-calls1 has no TIME_BLOCK → no line_calls 1:5 / block_line sample required
ok "default-calls1 --jsonl: 15/3/15 + sub_def leaf/mid + source_line 1:5"

# ---------------------------------------------------------------------------
# 2. blocks-calls1 golden JSONL (line_calls + A4b block_line)
# ---------------------------------------------------------------------------
echo "=== engine query expand: blocks-calls1 --jsonl ==="
BLK_OUT="$TMPDIR_SMOKE/blocks_jsonl.out"
BLK_ERR="$TMPDIR_SMOKE/blocks_jsonl.err"
if ! "${ENGINE[@]}" query --jsonl "$BLOCKS_GOLDEN" >"$BLK_OUT" 2>"$BLK_ERR"; then
  cat "$BLK_ERR" >&2 || true
  cat "$BLK_OUT" >&2 || true
  fail "query --jsonl blocks-calls1 failed"
fi
cat "$BLK_OUT"

grep -qE 'main::leaf returns=15' "$BLK_OUT" \
  || fail "blocks: missing main::leaf returns=15"
grep -qE 'main::mid returns=3' "$BLK_OUT" \
  || fail "blocks: missing main::mid returns=3"
grep -qE 'line_calls 1:5=780' "$BLK_OUT" \
  || fail "blocks: missing line_calls 1:5=780"
grep -qE 'block_line_calls 1:4=810' "$BLK_OUT" \
  || fail "blocks: missing block_line_calls 1:4=810"
grep -qE 'sub_def main::leaf fid=1 first=3 last=7' "$BLK_OUT" \
  || fail "blocks: missing sub_def main::leaf"
ok "blocks-calls1 --jsonl: line_calls 1:5=780 + block_line_calls 1:4=810"

# ---------------------------------------------------------------------------
# 3. Optional prove unit/integration test (returns/edges still covered)
# ---------------------------------------------------------------------------
if [[ -f "$ROOT/$T" ]]; then
  echo "=== prove $T ==="
  prove -I"$ENGINE_LIB" "$T" || fail "prove $T failed"
  ok "prove $T"
else
  echo "NOTE: $T not present; skipping prove"
fi

# ---------------------------------------------------------------------------
# 4. Optional native profile path (default-calls1) when CLI available
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
  echo "=== engine query expand: native profile ($CLI_SPEC) ==="
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
  grep -qE 'sub_def main::leaf fid=1 first=3 last=7' "$NAT_OUT" \
    || fail "native query missing sub_def main::leaf"
  grep -qF "source_line 1:5=${EXPECTED_SRC}" "$NAT_OUT" \
    || fail "native query source_line 1:5 mismatch"
  ok "native query profile: returns + sub_def + source_line"
else
  echo "NOTE: no native CLI / cargo; skipped live profile query (golden paths still required)"
fi

if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

ok "perl engine query expand packaging smoke passed"
exit 0

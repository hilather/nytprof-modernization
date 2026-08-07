#!/usr/bin/env bash
# Pure-Perl JsonlData line totals smoke (PERL-LINE-TOTALS).
#
# Spec: docs/schemas/perl-jsonl-data-mvp-v0.md
#
# 1. blocks-calls1 golden JSONL: line_calls(1,5)==780 from real TIME_BLOCK events
# 2. default-calls1: leaf=15 / mid=3 still hold
# 3. Optional: native dump of blocks-calls1 re-asserts line_calls(1,5)==780
#
# Does NOT put crates/ on oracle PERL5LIB. Does NOT require oracle Devel::NYTProf.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_line_totals_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

BLOCKS_DIR="fixtures/v5/blocks-calls1"
BLOCKS_GOLDEN="$BLOCKS_DIR/readstream.jsonl"
BLOCKS_PROFILE="$BLOCKS_DIR/nytprof.out"
DEFAULT_GOLDEN="fixtures/v5/default-calls1/readstream.jsonl"
PM="perl/lib/Devel/NYTProf/JsonlData.pm"
STREAM_PM="perl/lib/Devel/NYTProf/JsonlReadStream.pm"
T_BLOCKS="perl/t/jsonl_data_blocks_calls1_line_totals.t"
T_DEFAULT="perl/t/jsonl_data_default_calls1.t"
LIB="perl/lib"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$BLOCKS_GOLDEN" ]] || fail "missing golden dump $BLOCKS_GOLDEN"
[[ -f "$ROOT/$DEFAULT_GOLDEN" ]] || fail "missing golden dump $DEFAULT_GOLDEN"
[[ -f "$ROOT/$PM" ]] || fail "missing $PM"
[[ -f "$ROOT/$STREAM_PM" ]] || fail "missing $STREAM_PM"
[[ -f "$ROOT/$T_BLOCKS" ]] || fail "missing $T_BLOCKS"
[[ -f "$ROOT/$T_DEFAULT" ]] || fail "missing $T_DEFAULT"

# ---------------------------------------------------------------------------
# 1. Prove blocks-calls1 line totals + default-calls1 leaf/mid
# ---------------------------------------------------------------------------
echo "=== JsonlData line totals: prove blocks-calls1 ==="
prove -I"$LIB" "$T_BLOCKS" || fail "prove $T_BLOCKS failed"
ok "prove $T_BLOCKS"

echo "=== JsonlData line totals: prove default-calls1 (leaf/mid) ==="
prove -I"$LIB" "$T_DEFAULT" || fail "prove $T_DEFAULT failed"
ok "prove $T_DEFAULT (leaf=15 mid=3 mid→leaf=15)"

# ---------------------------------------------------------------------------
# 2. Explicit operator evidence: line_calls(1,5)==780 from TIME_BLOCK
# ---------------------------------------------------------------------------
echo "=== blocks-calls1 golden: line_calls(1,5) ==="
BLOCKS_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
    my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
    printf "line_calls(1,5)=%d\n", $d->line_calls(1, 5);
    my $lt = $d->line_totals;
    printf "line_totals 1:5 calls=%d ticks=%s\n",
      ($lt->{"1:5"}{calls} // 0),
      ($lt->{"1:5"}{ticks} // 0);
    printf "main::leaf returns=%d\n", $d->sub_returns("main::leaf");
    printf "main::mid returns=%d\n",  $d->sub_returns("main::mid");
    printf "records_seen=%d\n", $d->records_seen;
  ' "$BLOCKS_GOLDEN"
)"
echo "$BLOCKS_OUT"
echo "$BLOCKS_OUT" | grep -qE 'line_calls\(1,5\)=780' \
  || fail "blocks golden missing line_calls(1,5)=780"
echo "$BLOCKS_OUT" | grep -qE 'line_totals 1:5 calls=780' \
  || fail "blocks golden missing line_totals 1:5 calls=780"
echo "$BLOCKS_OUT" | grep -qE 'main::leaf returns=15' \
  || fail "blocks golden missing main::leaf returns=15"
echo "$BLOCKS_OUT" | grep -qE 'main::mid returns=3' \
  || fail "blocks golden missing main::mid returns=3"
ok "blocks-calls1 JsonlData: line_calls(1,5)=780 leaf=15 mid=3"

# ---------------------------------------------------------------------------
# 3. default-calls1 still leaf/mid 15/3 (and non-empty line_totals via TIME_LINE)
# ---------------------------------------------------------------------------
echo "=== default-calls1 golden: leaf/mid + line_totals non-empty ==="
DEFAULT_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
    my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
    printf "main::leaf returns=%d\n", $d->sub_returns("main::leaf");
    printf "main::mid returns=%d\n",  $d->sub_returns("main::mid");
    my $n = scalar keys %{ $d->line_totals };
    printf "line_totals_keys=%d\n", $n;
    # hot loop also present under TIME_LINE on default-calls1
    printf "line_calls(1,5)=%d\n", $d->line_calls(1, 5);
  ' "$DEFAULT_GOLDEN"
)"
echo "$DEFAULT_OUT"
echo "$DEFAULT_OUT" | grep -qE 'main::leaf returns=15' \
  || fail "default golden missing main::leaf returns=15"
echo "$DEFAULT_OUT" | grep -qE 'main::mid returns=3' \
  || fail "default golden missing main::mid returns=3"
echo "$DEFAULT_OUT" | grep -qE 'line_totals_keys=[1-9]' \
  || fail "default golden line_totals empty (expected TIME_LINE)"
ok "default-calls1 JsonlData: leaf=15 mid=3 line_totals non-empty"

# ---------------------------------------------------------------------------
# 4. Optional native CLI dump of blocks-calls1
# ---------------------------------------------------------------------------
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT
NATIVE_JSONL="$TMPDIR_SMOKE/native_dump.jsonl"

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
  echo "=== blocks-calls1: native CLI dump path ($CLI_SPEC) ==="
  [[ -f "$ROOT/$BLOCKS_PROFILE" ]] || fail "missing profile $BLOCKS_PROFILE"

  set +e
  if [[ "$CLI_SPEC" == cargo ]]; then
    cargo run -q -p nytprof-cli -- dump "$BLOCKS_PROFILE" >"$NATIVE_JSONL" 2>"$TMPDIR_SMOKE/dump.err"
    DUMP_RC=$?
  else
    CLI_PATH="${CLI_SPEC#path:}"
    "$CLI_PATH" dump "$BLOCKS_PROFILE" >"$NATIVE_JSONL" 2>"$TMPDIR_SMOKE/dump.err"
    DUMP_RC=$?
  fi
  set -e

  if [[ "$DUMP_RC" -ne 0 ]]; then
    cat "$TMPDIR_SMOKE/dump.err" >&2 || true
    fail "native dump failed (rc=$DUMP_RC)"
  fi
  [[ -s "$NATIVE_JSONL" ]] || fail "native dump produced empty JSONL"

  NATIVE_AGG="$(
    perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
      my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
      printf "line_calls(1,5)=%d\n", $d->line_calls(1, 5);
      printf "main::leaf returns=%d\n", $d->sub_returns("main::leaf");
      printf "main::mid returns=%d\n",  $d->sub_returns("main::mid");
    ' "$NATIVE_JSONL"
  )"
  echo "$NATIVE_AGG"
  echo "$NATIVE_AGG" | grep -qE 'line_calls\(1,5\)=780' \
    || fail "native dump missing line_calls(1,5)=780"
  echo "$NATIVE_AGG" | grep -qE 'main::leaf returns=15' \
    || fail "native dump missing main::leaf returns=15"
  echo "$NATIVE_AGG" | grep -qE 'main::mid returns=3' \
    || fail "native dump missing main::mid returns=3"
  ok "native dump JsonlData: line_calls(1,5)=780 leaf=15 mid=3"
else
  echo "NOTE: no native CLI / cargo; skipped live dump path (golden path still required)"
fi

ok "perl line totals packaging smoke passed"
exit 0

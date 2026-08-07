#!/usr/bin/env bash
# Pure-Perl JsonlData A4b block_line_totals smoke (PERL-A4B-JSONL).
#
# Spec: docs/schemas/perl-jsonl-data-mvp-v0.md
#
# 1. blocks-calls1 golden JSONL: block_line_totals non-empty; 1:4 calls=810
# 2. A4 line_calls(1,5)==780 still holds
# 3. Optional: native dump of blocks-calls1 re-asserts A4b 1:4=810 + A4 780
#
# Does NOT put crates/ on oracle PERL5LIB. Does NOT require oracle Devel::NYTProf.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_a4b_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

BLOCKS_DIR="fixtures/v5/blocks-calls1"
BLOCKS_GOLDEN="$BLOCKS_DIR/readstream.jsonl"
BLOCKS_PROFILE="$BLOCKS_DIR/nytprof.out"
DEFAULT_GOLDEN="fixtures/v5/default-calls1/readstream.jsonl"
PM="perl/lib/Devel/NYTProf/JsonlData.pm"
STREAM_PM="perl/lib/Devel/NYTProf/JsonlReadStream.pm"
T_A4B="perl/t/jsonl_data_a4b_blocks_calls1.t"
LIB="perl/lib"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$BLOCKS_GOLDEN" ]] || fail "missing golden dump $BLOCKS_GOLDEN"
[[ -f "$ROOT/$DEFAULT_GOLDEN" ]] || fail "missing golden dump $DEFAULT_GOLDEN"
[[ -f "$ROOT/$PM" ]] || fail "missing $PM"
[[ -f "$ROOT/$STREAM_PM" ]] || fail "missing $STREAM_PM"
[[ -f "$ROOT/$T_A4B" ]] || fail "missing $T_A4B"

# ---------------------------------------------------------------------------
# 1. Prove blocks-calls1 A4b
# ---------------------------------------------------------------------------
echo "=== JsonlData A4b: prove blocks-calls1 ==="
prove -I"$LIB" "$T_A4B" || fail "prove $T_A4B failed"
ok "prove $T_A4B"

# ---------------------------------------------------------------------------
# 2. Explicit operator evidence: A4b 1:4=810 + A4 line_calls(1,5)=780
# ---------------------------------------------------------------------------
echo "=== blocks-calls1 golden: block_line_totals + line_calls(1,5) ==="
BLOCKS_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
    my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
    my $blt = $d->block_line_totals;
    my $n = scalar keys %$blt;
    printf "block_line_totals_keys=%d\n", $n;
    printf "block_line_calls(1,4)=%d\n", $d->block_line_calls(1, 4);
    printf "block_line_totals 1:4 calls=%d ticks=%s\n",
      ($blt->{"1:4"}{calls} // 0),
      ($blt->{"1:4"}{ticks} // 0);
    printf "line_calls(1,5)=%d\n", $d->line_calls(1, 5);
    printf "main::leaf returns=%d\n", $d->sub_returns("main::leaf");
    printf "main::mid returns=%d\n",  $d->sub_returns("main::mid");
    printf "records_seen=%d\n", $d->records_seen;
  ' "$BLOCKS_GOLDEN"
)"
echo "$BLOCKS_OUT"
echo "$BLOCKS_OUT" | grep -qE 'block_line_totals_keys=[1-9]' \
  || fail "blocks golden block_line_totals empty"
echo "$BLOCKS_OUT" | grep -qE 'block_line_calls\(1,4\)=810' \
  || fail "blocks golden missing block_line_calls(1,4)=810"
echo "$BLOCKS_OUT" | grep -qE 'block_line_totals 1:4 calls=810' \
  || fail "blocks golden missing block_line_totals 1:4 calls=810"
echo "$BLOCKS_OUT" | grep -qE 'line_calls\(1,5\)=780' \
  || fail "blocks golden missing line_calls(1,5)=780"
echo "$BLOCKS_OUT" | grep -qE 'main::leaf returns=15' \
  || fail "blocks golden missing main::leaf returns=15"
echo "$BLOCKS_OUT" | grep -qE 'main::mid returns=3' \
  || fail "blocks golden missing main::mid returns=3"
ok "blocks-calls1 JsonlData: A4b 1:4=810 A4 line_calls(1,5)=780 leaf=15 mid=3"

# ---------------------------------------------------------------------------
# 3. default-calls1: A4b empty (no TIME_BLOCK); leaf/mid still hold
# ---------------------------------------------------------------------------
echo "=== default-calls1 golden: A4b empty + leaf/mid ==="
DEFAULT_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
    my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
    my $n = scalar keys %{ $d->block_line_totals };
    printf "block_line_totals_keys=%d\n", $n;
    printf "block_line_calls(1,4)=%d\n", $d->block_line_calls(1, 4);
    printf "main::leaf returns=%d\n", $d->sub_returns("main::leaf");
    printf "main::mid returns=%d\n",  $d->sub_returns("main::mid");
  ' "$DEFAULT_GOLDEN"
)"
echo "$DEFAULT_OUT"
echo "$DEFAULT_OUT" | grep -qE 'block_line_totals_keys=0' \
  || fail "default golden expected empty block_line_totals (no TIME_BLOCK)"
echo "$DEFAULT_OUT" | grep -qE 'block_line_calls\(1,4\)=0' \
  || fail "default golden expected block_line_calls(1,4)=0"
echo "$DEFAULT_OUT" | grep -qE 'main::leaf returns=15' \
  || fail "default golden missing main::leaf returns=15"
echo "$DEFAULT_OUT" | grep -qE 'main::mid returns=3' \
  || fail "default golden missing main::mid returns=3"
ok "default-calls1 JsonlData: A4b empty leaf=15 mid=3"

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
      printf "block_line_calls(1,4)=%d\n", $d->block_line_calls(1, 4);
      printf "line_calls(1,5)=%d\n", $d->line_calls(1, 5);
      printf "main::leaf returns=%d\n", $d->sub_returns("main::leaf");
      printf "main::mid returns=%d\n",  $d->sub_returns("main::mid");
    ' "$NATIVE_JSONL"
  )"
  echo "$NATIVE_AGG"
  echo "$NATIVE_AGG" | grep -qE 'block_line_calls\(1,4\)=810' \
    || fail "native dump missing block_line_calls(1,4)=810"
  echo "$NATIVE_AGG" | grep -qE 'line_calls\(1,5\)=780' \
    || fail "native dump missing line_calls(1,5)=780"
  echo "$NATIVE_AGG" | grep -qE 'main::leaf returns=15' \
    || fail "native dump missing main::leaf returns=15"
  echo "$NATIVE_AGG" | grep -qE 'main::mid returns=3' \
    || fail "native dump missing main::mid returns=3"
  ok "native dump JsonlData: A4b 1:4=810 A4 line_calls(1,5)=780 leaf=15 mid=3"
else
  echo "NOTE: no native CLI / cargo; skipped live dump path (golden path still required)"
fi

ok "perl A4b packaging smoke passed"
exit 0

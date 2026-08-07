#!/usr/bin/env bash
# Pure-Perl JsonlData A8 source_lines smoke (PERL-SOURCE-JSONL).
#
# Spec: docs/schemas/perl-jsonl-data-mvp-v0.md
#
# 1. Prove golden JSONL path: source_line(1,5) from real SRC_LINE events
# 2. Optional: regenerate dump via native CLI and re-query the same text
#
# Does NOT put crates/ on oracle PERL5LIB. Does NOT require oracle Devel::NYTProf.
# Does NOT invent source text — asserts dump-derived text only.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_source_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="fixtures/v5/default-calls1"
GOLDEN="$FIXTURE_DIR/readstream.jsonl"
PROFILE="$FIXTURE_DIR/nytprof.out"
PM="perl/lib/Devel/NYTProf/JsonlData.pm"
STREAM_PM="perl/lib/Devel/NYTProf/JsonlReadStream.pm"
T="perl/t/jsonl_data_source_default_calls1.t"
LIB="perl/lib"

# Exact dump text for fid=1 line=5 (from SRC_LINE args[2] on default-calls1)
EXPECTED_LINE5='    $x++ for 1 .. 50;'

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$GOLDEN" ]] || fail "missing golden dump $GOLDEN"
[[ -f "$ROOT/$PM" ]] || fail "missing $PM"
[[ -f "$ROOT/$STREAM_PM" ]] || fail "missing $STREAM_PM"
[[ -f "$ROOT/$T" ]] || fail "missing $T"

# ---------------------------------------------------------------------------
# 1. Pure-Perl test against committed golden JSONL (no native CLI required)
# ---------------------------------------------------------------------------
echo "=== JsonlData source: golden JSONL path ==="
prove -I"$LIB" "$T" || fail "prove $T failed"
ok "prove $T (golden readstream.jsonl)"

# Explicit aggregation print for operator evidence
AGG_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
    my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
    my $t = $d->source_line(1, 5) // "";
    # Print one-line form (chomp trailing newline for grep-friendly evidence)
    my $one = $t;
    chomp $one;
    printf "source_line(1,5)=%s\n", $one;
    printf "has_xpp=%s\n",   ( $t =~ /\$x\+\+/   ) ? "yes" : "no";
    printf "has_1_50=%s\n",  ( $t =~ /1 \.\. 50/ ) ? "yes" : "no";
    printf "source_lines_keys=%d\n", scalar keys %{ $d->source_lines };
    printf "records_seen=%d\n", $d->records_seen;
  ' "$GOLDEN"
)"
echo "$AGG_OUT"
echo "$AGG_OUT" | grep -qF "source_line(1,5)=${EXPECTED_LINE5}" \
  || fail "golden source_line(1,5) mismatch (expected dump text)"
echo "$AGG_OUT" | grep -qE 'has_xpp=yes' \
  || fail "golden missing \$x++ in source_line(1,5)"
echo "$AGG_OUT" | grep -qE 'has_1_50=yes' \
  || fail "golden missing 1 .. 50 in source_line(1,5)"
ok "golden JsonlData: source_line(1,5) = '${EXPECTED_LINE5}'"

# ---------------------------------------------------------------------------
# 2. Native CLI dump path (when cargo or a built binary is available)
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
  echo "=== JsonlData source: native CLI dump path ($CLI_SPEC) ==="
  [[ -f "$ROOT/$PROFILE" ]] || fail "missing profile $PROFILE"

  set +e
  if [[ "$CLI_SPEC" == cargo ]]; then
    cargo run -q -p nytprof-cli -- dump "$PROFILE" >"$NATIVE_JSONL" 2>"$TMPDIR_SMOKE/dump.err"
    DUMP_RC=$?
  else
    CLI_PATH="${CLI_SPEC#path:}"
    "$CLI_PATH" dump "$PROFILE" >"$NATIVE_JSONL" 2>"$TMPDIR_SMOKE/dump.err"
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
      my $t = $d->source_line(1, 5) // "";
      my $one = $t;
      chomp $one;
      printf "source_line(1,5)=%s\n", $one;
      printf "has_xpp=%s\n",   ( $t =~ /\$x\+\+/   ) ? "yes" : "no";
      printf "has_1_50=%s\n",  ( $t =~ /1 \.\. 50/ ) ? "yes" : "no";
    ' "$NATIVE_JSONL"
  )"
  echo "$NATIVE_AGG"
  echo "$NATIVE_AGG" | grep -qF "source_line(1,5)=${EXPECTED_LINE5}" \
    || fail "native dump source_line(1,5) mismatch (expected dump text)"
  echo "$NATIVE_AGG" | grep -qE 'has_xpp=yes' \
    || fail "native dump missing \$x++ in source_line(1,5)"
  echo "$NATIVE_AGG" | grep -qE 'has_1_50=yes' \
    || fail "native dump missing 1 .. 50 in source_line(1,5)"
  ok "native dump JsonlData: source_line(1,5) = '${EXPECTED_LINE5}'"
else
  echo "NOTE: no native CLI / cargo; skipped live dump path (golden path still required)"
fi

ok "perl JsonlData source packaging smoke passed"
exit 0

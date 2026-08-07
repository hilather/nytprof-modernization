#!/usr/bin/env bash
# Pure-Perl JsonlData packaging smoke (PERL-DATA-JSONL).
#
# Spec: docs/schemas/perl-jsonl-data-mvp-v0.md
#
# 1. Prove golden JSONL path: leaf=15, mid=3, mid→leaf=15 from real dump events
# 2. Optional: regenerate dump via native CLI and re-query the same counts
#
# Does NOT put crates/ on oracle PERL5LIB. Does NOT require oracle Devel::NYTProf.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_jsonl_data_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="fixtures/v5/default-calls1"
GOLDEN="$FIXTURE_DIR/readstream.jsonl"
PROFILE="$FIXTURE_DIR/nytprof.out"
PM="perl/lib/Devel/NYTProf/JsonlData.pm"
STREAM_PM="perl/lib/Devel/NYTProf/JsonlReadStream.pm"
T="perl/t/jsonl_data_default_calls1.t"
LIB="perl/lib"

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
echo "=== JsonlData: golden JSONL path ==="
prove -I"$LIB" "$T" || fail "prove $T failed"
ok "prove $T (golden readstream.jsonl)"

# Explicit aggregation print for operator evidence
AGG_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
    my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
    printf "main::leaf returns=%d\n", $d->sub_returns("main::leaf");
    printf "main::mid returns=%d\n",  $d->sub_returns("main::mid");
    printf "mid->leaf edge=%d\n",
      $d->call_edge_count("main::mid", "main::leaf");
    printf "records_seen=%d\n", $d->records_seen;
  ' "$GOLDEN"
)"
echo "$AGG_OUT"
echo "$AGG_OUT" | grep -qE 'main::leaf returns=15' \
  || fail "golden aggregation missing main::leaf returns=15"
echo "$AGG_OUT" | grep -qE 'main::mid returns=3' \
  || fail "golden aggregation missing main::mid returns=3"
echo "$AGG_OUT" | grep -qE 'mid->leaf edge=15' \
  || fail "golden aggregation missing mid->leaf edge=15"
ok "golden JsonlData: leaf=15 mid=3 mid→leaf=15"

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
  echo "=== JsonlData: native CLI dump path ($CLI_SPEC) ==="
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
      printf "main::leaf returns=%d\n", $d->sub_returns("main::leaf");
      printf "main::mid returns=%d\n",  $d->sub_returns("main::mid");
      printf "mid->leaf edge=%d\n",
        $d->call_edge_count("main::mid", "main::leaf");
    ' "$NATIVE_JSONL"
  )"
  echo "$NATIVE_AGG"
  echo "$NATIVE_AGG" | grep -qE 'main::leaf returns=15' \
    || fail "native dump aggregation missing main::leaf returns=15"
  echo "$NATIVE_AGG" | grep -qE 'main::mid returns=3' \
    || fail "native dump aggregation missing main::mid returns=3"
  echo "$NATIVE_AGG" | grep -qE 'mid->leaf edge=15' \
    || fail "native dump aggregation missing mid->leaf edge=15"
  ok "native dump JsonlData: leaf=15 mid=3 mid→leaf=15"

  # Also exercise from_cli API
  if [[ "$CLI_SPEC" != cargo ]]; then
    CLI_PATH="${CLI_SPEC#path:}"
    FROM_CLI_OUT="$(
      perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
        my ($cli, $profile) = @ARGV;
        my $d = Devel::NYTProf::JsonlData->from_cli(
          [ $cli, "dump", $profile ]
        );
        printf "from_cli main::leaf returns=%d\n", $d->sub_returns("main::leaf");
        printf "from_cli main::mid returns=%d\n",  $d->sub_returns("main::mid");
        printf "from_cli mid->leaf edge=%d\n",
          $d->call_edge_count("main::mid", "main::leaf");
      ' "$CLI_PATH" "$ROOT/$PROFILE"
    )"
    echo "$FROM_CLI_OUT"
    echo "$FROM_CLI_OUT" | grep -qE 'from_cli main::leaf returns=15' \
      || fail "from_cli missing leaf=15"
    echo "$FROM_CLI_OUT" | grep -qE 'from_cli main::mid returns=3' \
      || fail "from_cli missing mid=3"
    echo "$FROM_CLI_OUT" | grep -qE 'from_cli mid->leaf edge=15' \
      || fail "from_cli missing mid->leaf=15"
    ok "from_cli API: leaf=15 mid=3 mid→leaf=15"
  else
    echo "NOTE: skipping from_cli cargo multi-arg spawn (path binary preferred)"
  fi
else
  echo "NOTE: no native CLI / cargo; skipped live dump path (golden path still required)"
fi

ok "perl JsonlData packaging smoke passed"
exit 0

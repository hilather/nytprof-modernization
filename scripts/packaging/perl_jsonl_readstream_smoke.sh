#!/usr/bin/env bash
# Pure-Perl JsonlReadStream packaging smoke (PERL-READSTREAM-JSONL).
#
# Spec: docs/schemas/perl-jsonl-readstream-mvp-v0.md
#
# 1. Prove golden JSONL path: leaf returns=15, mid=3 from real SUB_RETURN events
# 2. Optional: regenerate dump via native CLI and re-aggregate the same counts
#
# Does NOT put crates/ on oracle PERL5LIB. Does NOT require oracle Devel::NYTProf.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_jsonl_readstream_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="fixtures/v5/default-calls1"
GOLDEN="$FIXTURE_DIR/readstream.jsonl"
PROFILE="$FIXTURE_DIR/nytprof.out"
PM="perl/lib/Devel/NYTProf/JsonlReadStream.pm"
T="perl/t/jsonl_readstream_default_calls1.t"
LIB="perl/lib"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$GOLDEN" ]] || fail "missing golden dump $GOLDEN"
[[ -f "$ROOT/$PM" ]] || fail "missing $PM"
[[ -f "$ROOT/$T" ]] || fail "missing $T"

# ---------------------------------------------------------------------------
# 1. Pure-Perl test against committed golden JSONL (no native CLI required)
# ---------------------------------------------------------------------------
echo "=== JsonlReadStream: golden JSONL path ==="
prove -I"$LIB" "$T" || fail "prove $T failed"
ok "prove $T (golden readstream.jsonl)"

# Explicit aggregation print for operator evidence
AGG_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlReadStream=count_sub_returns -e '
    my $c = count_sub_returns(shift);
    printf "main::leaf returns=%d\n", $c->{"main::leaf"} // 0;
    printf "main::mid returns=%d\n",  $c->{"main::mid"}  // 0;
    printf "SUB_RETURN distinct subs=%d\n", scalar keys %$c;
  ' "$GOLDEN"
)"
echo "$AGG_OUT"
echo "$AGG_OUT" | grep -qE 'main::leaf returns=15' \
  || fail "golden aggregation missing main::leaf returns=15"
echo "$AGG_OUT" | grep -qE 'main::mid returns=3' \
  || fail "golden aggregation missing main::mid returns=3"
ok "golden SUB_RETURN aggregation: leaf=15 mid=3"

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
  echo "=== JsonlReadStream: native CLI dump path ($CLI_SPEC) ==="
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
    perl -I"$LIB" -MDevel::NYTProf::JsonlReadStream=count_sub_returns -e '
      my $c = count_sub_returns(shift);
      printf "main::leaf returns=%d\n", $c->{"main::leaf"} // 0;
      printf "main::mid returns=%d\n",  $c->{"main::mid"}  // 0;
    ' "$NATIVE_JSONL"
  )"
  echo "$NATIVE_AGG"
  echo "$NATIVE_AGG" | grep -qE 'main::leaf returns=15' \
    || fail "native dump aggregation missing main::leaf returns=15"
  echo "$NATIVE_AGG" | grep -qE 'main::mid returns=3' \
    || fail "native dump aggregation missing main::mid returns=3"
  ok "native dump SUB_RETURN aggregation: leaf=15 mid=3"

  # Also exercise from_cli API
  if [[ "$CLI_SPEC" != cargo ]]; then
    CLI_PATH="${CLI_SPEC#path:}"
    FROM_CLI_OUT="$(
      perl -I"$LIB" -MDevel::NYTProf::JsonlReadStream=for_chunks -e '
        my ($cli, $profile) = @ARGV;
        my %c;
        for_chunks(
          sub {
            my ($tag, $args) = @_;
            return unless $tag eq "SUB_RETURN";
            my $n = $args->[3];
            $c{$n}++ if defined $n;
          },
          from_cli => [ $cli, "dump", $profile ],
        );
        printf "from_cli main::leaf returns=%d\n", $c{"main::leaf"} // 0;
        printf "from_cli main::mid returns=%d\n",  $c{"main::mid"}  // 0;
      ' "$CLI_PATH" "$ROOT/$PROFILE"
    )"
    echo "$FROM_CLI_OUT"
    echo "$FROM_CLI_OUT" | grep -qE 'from_cli main::leaf returns=15' \
      || fail "from_cli missing leaf=15"
    echo "$FROM_CLI_OUT" | grep -qE 'from_cli main::mid returns=3' \
      || fail "from_cli missing mid=3"
    ok "from_cli API: leaf=15 mid=3"
  else
    echo "NOTE: skipping from_cli cargo multi-arg spawn (path binary preferred)"
  fi
else
  echo "NOTE: no native CLI / cargo; skipped live dump path (golden path still required)"
fi

ok "perl JsonlReadStream packaging smoke passed"
exit 0

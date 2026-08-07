#!/usr/bin/env bash
# Pure-Perl JsonlData DISCOUNT event multiplicity smoke (PERL-DISCOUNT-JSONL).
#
# Spec: docs/schemas/perl-jsonl-data-mvp-v0.md
# Aggregate A3: docs/schemas/aggregate-comparison-v0.md
#
# 1. Prove golden JSONL path: discount_events == stream recount of DISCOUNT
#    tags; default-calls1 golden observes 818 (derived by recount, not magic)
# 2. Optional: regenerate dump via native CLI and re-query discount count
#
# DISCOUNT is event multiplicity only — not exclusive-time policy freeze.
# Does NOT put crates/ on oracle PERL5LIB. Does NOT require oracle Devel::NYTProf.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_discount_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="fixtures/v5/default-calls1"
GOLDEN="$FIXTURE_DIR/readstream.jsonl"
PROFILE="$FIXTURE_DIR/nytprof.out"
PM="perl/lib/Devel/NYTProf/JsonlData.pm"
STREAM_PM="perl/lib/Devel/NYTProf/JsonlReadStream.pm"
T="perl/t/jsonl_data_discount_default_calls1.t"
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
echo "=== JsonlData DISCOUNT: golden JSONL path ==="
prove -I"$LIB" "$T" || fail "prove $T failed"
ok "prove $T (golden readstream.jsonl)"

# Explicit aggregation print for operator evidence
AGG_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
    my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
    printf "discount_events=%d\n", $d->discount_events;
    printf "discount_count=%d\n",  $d->discount_count;
    printf "records_seen=%d\n",    $d->records_seen;
  ' "$GOLDEN"
)"
echo "$AGG_OUT"
echo "$AGG_OUT" | grep -qE '^discount_events=[1-9]' \
  || fail "golden discount_events < 1"
DE="$(echo "$AGG_OUT" | sed -n 's/^discount_events=//p' | head -1)"
DC="$(echo "$AGG_OUT" | sed -n 's/^discount_count=//p' | head -1)"
[[ -n "$DE" && "$DE" == "$DC" ]] \
  || fail "discount_events ($DE) != discount_count ($DC)"

# Independent stream re-count of DISCOUNT tags (must match JsonlData)
RECOUNT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlReadStream=for_chunks -e '
    my $n = 0;
    for_chunks(sub {
      my ($tag) = @_;
      $n++ if $tag eq "DISCOUNT";
    }, file => shift);
    print $n, "\n";
  ' "$GOLDEN"
)"
RECOUNT="$(echo "$RECOUNT" | tr -d '[:space:]')"
[[ -n "$RECOUNT" && "$RECOUNT" == "$DE" ]] \
  || fail "JsonlData discount_events ($DE) != stream recount ($RECOUNT)"
# Golden fixture observes 818 (assert re-count-derived, not magic alone)
[[ "$RECOUNT" == "818" ]] \
  || fail "expected golden DISCOUNT re-count 818, got $RECOUNT"
ok "golden JsonlData: discount_events=$DE matches recount=$RECOUNT"

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
  echo "=== JsonlData DISCOUNT: native CLI dump path ($CLI_SPEC) ==="
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
      printf "discount_events=%d\n", $d->discount_events;
      printf "discount_count=%d\n",  $d->discount_count;
    ' "$NATIVE_JSONL"
  )"
  echo "$NATIVE_AGG"
  echo "$NATIVE_AGG" | grep -qE '^discount_events=[1-9]' \
    || fail "native dump discount_events < 1"
  N_DE="$(echo "$NATIVE_AGG" | sed -n 's/^discount_events=//p' | head -1)"
  N_DC="$(echo "$NATIVE_AGG" | sed -n 's/^discount_count=//p' | head -1)"
  [[ -n "$N_DE" && "$N_DE" == "$N_DC" ]] \
    || fail "native discount_events ($N_DE) != discount_count ($N_DC)"
  # Native dump of the same profile must observe the same DISCOUNT multiplicity
  [[ "$N_DE" == "818" ]] \
    || fail "native dump expected discount_events 818, got $N_DE"
  ok "native dump JsonlData: discount_events=$N_DE matched"
else
  echo "NOTE: no native CLI / cargo; skipped live dump path (golden path still required)"
fi

ok "perl JsonlData DISCOUNT packaging smoke passed"
exit 0

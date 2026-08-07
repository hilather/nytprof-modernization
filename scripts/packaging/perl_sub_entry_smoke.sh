#!/usr/bin/env bash
# Pure-Perl JsonlData SUB_ENTRY event multiplicity smoke (PERL-SUB-ENTRY-JSONL).
#
# Spec: docs/schemas/perl-jsonl-data-mvp-v0.md
#
# 1. Prove golden JSONL path:
#    - default-calls1 (calls=1): sub_entry_count == 0 == stream recount
#    - calls2-default (calls=2): sub_entry_count == 27 == stream recount
# 2. Optional: regenerate dump via native CLI and re-query sub_entry count
#
# SUB_ENTRY is event multiplicity only — not full call-stack / arg freeze.
# Does NOT put crates/ on oracle PERL5LIB. Does NOT require oracle Devel::NYTProf.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_sub_entry_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

DEFAULT_DIR="fixtures/v5/default-calls1"
CALLS2_DIR="fixtures/v5/calls2-default"
DEFAULT_GOLDEN="$DEFAULT_DIR/readstream.jsonl"
CALLS2_GOLDEN="$CALLS2_DIR/readstream.jsonl"
DEFAULT_PROFILE="$DEFAULT_DIR/nytprof.out"
CALLS2_PROFILE="$CALLS2_DIR/nytprof.out"
PM="perl/lib/Devel/NYTProf/JsonlData.pm"
STREAM_PM="perl/lib/Devel/NYTProf/JsonlReadStream.pm"
T="perl/t/jsonl_data_sub_entry.t"
LIB="perl/lib"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$DEFAULT_GOLDEN" ]] || fail "missing golden dump $DEFAULT_GOLDEN"
[[ -f "$ROOT/$CALLS2_GOLDEN" ]] || fail "missing golden dump $CALLS2_GOLDEN"
[[ -f "$ROOT/$PM" ]] || fail "missing $PM"
[[ -f "$ROOT/$STREAM_PM" ]] || fail "missing $STREAM_PM"
[[ -f "$ROOT/$T" ]] || fail "missing $T"

# ---------------------------------------------------------------------------
# 1. Pure-Perl test against committed golden JSONL (no native CLI required)
# ---------------------------------------------------------------------------
echo "=== JsonlData SUB_ENTRY: golden JSONL path ==="
prove -I"$LIB" "$T" || fail "prove $T failed"
ok "prove $T (default-calls1 + calls2-default golden)"

check_fixture() {
  local label="$1"
  local golden="$2"
  local expect="$3"

  local AGG_OUT
  AGG_OUT="$(
    perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
      my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
      printf "sub_entry_events=%d\n", $d->sub_entry_events;
      printf "sub_entry_count=%d\n",  $d->sub_entry_count;
      printf "records_seen=%d\n",    $d->records_seen;
    ' "$golden"
  )"
  echo "[$label] $AGG_OUT"
  local SE SC
  SE="$(echo "$AGG_OUT" | sed -n 's/^sub_entry_events=//p' | head -1)"
  SC="$(echo "$AGG_OUT" | sed -n 's/^sub_entry_count=//p' | head -1)"
  [[ -n "$SE" && "$SE" == "$SC" ]] \
    || fail "$label: sub_entry_events ($SE) != sub_entry_count ($SC)"

  local RECOUNT
  RECOUNT="$(
    perl -I"$LIB" -MDevel::NYTProf::JsonlReadStream=for_chunks -e '
      my $n = 0;
      for_chunks(sub {
        my ($tag) = @_;
        $n++ if $tag eq "SUB_ENTRY";
      }, file => shift);
      print $n, "\n";
    ' "$golden"
  )"
  RECOUNT="$(echo "$RECOUNT" | tr -d '[:space:]')"
  [[ -n "$RECOUNT" && "$RECOUNT" == "$SE" ]] \
    || fail "$label: JsonlData sub_entry_events ($SE) != stream recount ($RECOUNT)"
  [[ "$RECOUNT" == "$expect" ]] \
    || fail "$label: expected golden SUB_ENTRY re-count $expect, got $RECOUNT"
  ok "$label JsonlData: sub_entry_events=$SE matches recount=$RECOUNT"
}

check_fixture "default-calls1" "$DEFAULT_GOLDEN" "0"
check_fixture "calls2-default" "$CALLS2_GOLDEN" "27"

# ---------------------------------------------------------------------------
# 2. Native CLI dump path (when cargo or a built binary is available)
# ---------------------------------------------------------------------------
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

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

check_native() {
  local label="$1"
  local profile="$2"
  local expect="$3"
  local native_jsonl="$TMPDIR_SMOKE/${label}.jsonl"
  local dump_err="$TMPDIR_SMOKE/${label}.dump.err"

  [[ -f "$ROOT/$profile" ]] || fail "missing profile $profile"

  set +e
  if [[ "$CLI_SPEC" == cargo ]]; then
    cargo run -q -p nytprof-cli -- dump "$profile" >"$native_jsonl" 2>"$dump_err"
    local DUMP_RC=$?
  else
    local CLI_PATH="${CLI_SPEC#path:}"
    "$CLI_PATH" dump "$profile" >"$native_jsonl" 2>"$dump_err"
    local DUMP_RC=$?
  fi
  set -e

  if [[ "$DUMP_RC" -ne 0 ]]; then
    cat "$dump_err" >&2 || true
    fail "$label: native dump failed (rc=$DUMP_RC)"
  fi
  [[ -s "$native_jsonl" ]] || fail "$label: native dump produced empty JSONL"

  local NATIVE_AGG N_SE N_SC
  NATIVE_AGG="$(
    perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
      my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
      printf "sub_entry_events=%d\n", $d->sub_entry_events;
      printf "sub_entry_count=%d\n",  $d->sub_entry_count;
    ' "$native_jsonl"
  )"
  echo "[$label native] $NATIVE_AGG"
  N_SE="$(echo "$NATIVE_AGG" | sed -n 's/^sub_entry_events=//p' | head -1)"
  N_SC="$(echo "$NATIVE_AGG" | sed -n 's/^sub_entry_count=//p' | head -1)"
  [[ -n "$N_SE" && "$N_SE" == "$N_SC" ]] \
    || fail "$label native: sub_entry_events ($N_SE) != sub_entry_count ($N_SC)"
  [[ "$N_SE" == "$expect" ]] \
    || fail "$label native dump expected sub_entry_events $expect, got $N_SE"
  ok "$label native dump JsonlData: sub_entry_events=$N_SE matched"
}

if CLI_SPEC="$(find_cli)"; then
  echo "=== JsonlData SUB_ENTRY: native CLI dump path ($CLI_SPEC) ==="
  check_native "default-calls1" "$DEFAULT_PROFILE" "0"
  check_native "calls2-default" "$CALLS2_PROFILE" "27"
else
  echo "NOTE: no native CLI / cargo; skipped live dump path (golden path still required)"
fi

ok "perl JsonlData SUB_ENTRY packaging smoke passed"
exit 0

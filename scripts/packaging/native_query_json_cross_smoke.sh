#!/usr/bin/env bash
# NATIVE-QUERY-JSON-CROSS: cross-check shared JSON fields between
#   1) native  nytprof-cli report --json  <profile.out>
#   2) Perl    nytprof-engine query --json --jsonl <readstream.jsonl>
#
# Shared fields (default-calls1 contract):
#   leaf_returns == 15
#   mid_returns  == 3
#   mid_leaf_edge == 15
#   discount_events == 818  (or equal between sides if dump-derived drift)
#
# Runs the pair twice for consistency. Optional third path: query --json via
# native dump of the live profile when the CLI is available.
#
# Specs:
#   docs/schemas/native-aggregates-json-mvp-v0.md
#   docs/schemas/perl-engine-dispatch-mvp-v0.md
# Board: NATIVE-QUERY-JSON-CROSS
#
# Does NOT reimplement aggregation — invokes real CLIs and parses JSON.
# Never puts crates/ on oracle PERL5LIB. Fail-fast; fails closed without native.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/native_query_json_cross_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

PROFILE_REL="fixtures/v5/default-calls1/nytprof.out"
GOLDEN_REL="fixtures/v5/default-calls1/readstream.jsonl"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"

# Shared-field contract (default-calls1)
WANT_LEAF=15
WANT_MID=3
WANT_EDGE=15
WANT_DISCOUNT=818

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

# ---------------------------------------------------------------------------
# Resolve native CLI (prefer cargo when present so smoke exercises current tree).
# ---------------------------------------------------------------------------
CLI_MODE="" # binary | cargo
CLI_BIN=""
CLI_CMD=()

if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI_BIN="$NYTPROF_NATIVE_CLI"
  CLI_MODE=binary
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  CLI_MODE=cargo
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI_BIN="$ROOT/prefix/bin/nytprof-cli"
  CLI_MODE=binary
elif [[ -x "$ROOT/prefix/bin/nytprof-dump" ]]; then
  CLI_BIN="$ROOT/prefix/bin/nytprof-dump"
  CLI_MODE=binary
elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
  CLI_BIN="$ROOT/target/debug/nytprof-dump"
  CLI_MODE=binary
elif [[ -x "$ROOT/target/release/nytprof-dump" ]]; then
  CLI_BIN="$ROOT/target/release/nytprof-dump"
  CLI_MODE=binary
else
  fail "no native CLI found (NATIVE-QUERY-JSON-CROSS fails closed without native)
  looked for: \$NYTPROF_NATIVE_CLI, cargo + workspace Cargo.toml,
  prefix/bin/{nytprof-cli,nytprof-dump}, target/{debug,release}/nytprof-dump
  Install: ./scripts/packaging/install_native.sh
  Or build: cargo build -p nytprof-cli
  (pure-Perl query --json alone: ./scripts/packaging/perl_query_json_smoke.sh)"
fi

if [[ "$CLI_MODE" == "binary" ]]; then
  CLI_CMD=("$CLI_BIN")
  ok "using native binary: $CLI_BIN"
else
  CLI_CMD=(cargo run -q -p nytprof-cli --)
  ok "using cargo run -p nytprof-cli"
fi

# ---------------------------------------------------------------------------
# Preconditions
# ---------------------------------------------------------------------------
[[ -f "$ROOT/$PROFILE_REL" ]] || fail "missing fixture $PROFILE_REL"
[[ -f "$ROOT/$GOLDEN_REL" ]] || fail "missing golden dump $GOLDEN_REL"
[[ -f "$ROOT/$ENGINE_BIN" ]] || fail "missing $ENGINE_BIN"
[[ -d "$ROOT/$ENGINE_LIB" ]] || fail "missing $ENGINE_LIB"
command -v perl >/dev/null 2>&1 || fail "perl not on PATH"

ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")

# Sanity: never inject crates/ into oracle PERL5LIB from this smoke.
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

# ---------------------------------------------------------------------------
# Extract shared fields as "leaf mid edge discount" (space-separated).
# ---------------------------------------------------------------------------
extract_shared() {
  local f="$1"
  local label="$2"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
path,label=sys.argv[1],sys.argv[2]
try:
    o=json.load(open(path,encoding="utf-8"))
except Exception as e:
    sys.stderr.write("%s: JSON parse failed: %s\n" % (label, e))
    sys.exit(2)
if not isinstance(o, dict):
    sys.stderr.write("%s: not a JSON object\n" % label)
    sys.exit(2)
def geti(k):
    v=o.get(k)
    if isinstance(v, bool):
        return int(v)
    if isinstance(v, int):
        return v
    if isinstance(v, float) and v == int(v):
        return int(v)
    sys.stderr.write("%s: %s missing or not int (%r)\n" % (label, k, v))
    sys.exit(2)
print(geti("leaf_returns"), geti("mid_returns"), geti("mid_leaf_edge"),
      geti("discount_events"))
' "$f" "$label" || fail "$label: extract_shared failed
$(cat "$f")"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      use strict; use warnings;
      my ($f,$label)=@ARGV;
      open my $fh, "<", $f or die "$label: $!\n";
      local $/; my $raw = <$fh>;
      my $o = eval { JSON::PP->new->decode($raw) };
      die "$label: JSON parse: $@\n" if $@ || ref($o) ne "HASH";
      for my $k (qw(leaf_returns mid_returns mid_leaf_edge discount_events)) {
        my $v = $o->{$k};
        die "$label: $k missing or not int\n"
          unless defined $v && $v =~ /^-?\d+$/;
      }
      print join(" ",
        0+$o->{leaf_returns}, 0+$o->{mid_returns},
        0+$o->{mid_leaf_edge}, 0+$o->{discount_events}), "\n";
    ' "$f" "$label" || fail "$label: extract_shared failed (perl)
$(cat "$f")"
  else
    fail "$label: need python3 or perl JSON::PP to parse cross-check JSON"
  fi
}

assert_shared_contract() {
  local tuple="$1"  # "leaf mid edge discount"
  local label="$2"
  local leaf mid edge disc
  read -r leaf mid edge disc <<<"$tuple"
  [[ "$leaf" == "$WANT_LEAF" ]] \
    || fail "$label: leaf_returns=$leaf want $WANT_LEAF"
  [[ "$mid" == "$WANT_MID" ]] \
    || fail "$label: mid_returns=$mid want $WANT_MID"
  [[ "$edge" == "$WANT_EDGE" ]] \
    || fail "$label: mid_leaf_edge=$edge want $WANT_EDGE"
  # discount: prefer golden 818; allow equal-sides-only via caller when needed
  if [[ "$disc" != "$WANT_DISCOUNT" ]]; then
    log "NOTE: $label discount_events=$disc (golden contract is $WANT_DISCOUNT)"
  fi
}

assert_tuples_equal() {
  local a="$1" b="$2" label="$3"
  if [[ "$a" != "$b" ]]; then
    fail "$label: shared fields diverge
  side A: $a  (leaf mid edge discount)
  side B: $b  (leaf mid edge discount)"
  fi
}

# ---------------------------------------------------------------------------
# Cross pair ×2
# ---------------------------------------------------------------------------
PREV_NATIVE=""
PREV_PERL=""

for round in 1 2; do
  echo "=== NATIVE-QUERY-JSON-CROSS round $round ==="

  NATIVE_OUT="$TMPDIR_SMOKE/native_r${round}.out"
  NATIVE_ERR="$TMPDIR_SMOKE/native_r${round}.err"
  PERL_OUT="$TMPDIR_SMOKE/perl_r${round}.out"
  PERL_ERR="$TMPDIR_SMOKE/perl_r${round}.err"

  if ! "${CLI_CMD[@]}" report --json "$PROFILE_REL" \
    >"$NATIVE_OUT" 2>"$NATIVE_ERR"; then
    cat "$NATIVE_ERR" >&2 || true
    cat "$NATIVE_OUT" >&2 || true
    fail "native report --json round $round failed"
  fi

  if ! "${ENGINE[@]}" query --json --jsonl "$GOLDEN_REL" \
    >"$PERL_OUT" 2>"$PERL_ERR"; then
    cat "$PERL_ERR" >&2 || true
    cat "$PERL_OUT" >&2 || true
    fail "perl query --json --jsonl round $round failed"
  fi

  NATIVE_T="$(extract_shared "$NATIVE_OUT" "native r$round")"
  PERL_T="$(extract_shared "$PERL_OUT" "perl r$round")"

  log "  native report --json:  $NATIVE_T"
  log "  perl query --jsonl:    $PERL_T"

  assert_shared_contract "$NATIVE_T" "native r$round"
  assert_shared_contract "$PERL_T" "perl r$round"

  # discount: require == 818 on golden/default-calls1, and equal across sides
  read -r _nleaf _nmid _nedge ndisc <<<"$NATIVE_T"
  read -r _pleaf _pmid _pedge pdisc <<<"$PERL_T"
  [[ "$ndisc" == "$WANT_DISCOUNT" ]] \
    || fail "native r$round: discount_events=$ndisc want $WANT_DISCOUNT"
  [[ "$pdisc" == "$WANT_DISCOUNT" ]] \
    || fail "perl r$round: discount_events=$pdisc want $WANT_DISCOUNT"

  assert_tuples_equal "$NATIVE_T" "$PERL_T" "round $round native vs perl"

  if [[ -n "$PREV_NATIVE" ]]; then
    assert_tuples_equal "$PREV_NATIVE" "$NATIVE_T" "native consistency round1 vs r$round"
    assert_tuples_equal "$PREV_PERL" "$PERL_T" "perl consistency round1 vs r$round"
  fi
  PREV_NATIVE="$NATIVE_T"
  PREV_PERL="$PERL_T"

  ok "round $round: shared fields equal ($NATIVE_T)"
done

ok "cross pair ×2: leaf=$WANT_LEAF mid=$WANT_MID edge=$WANT_EDGE discount=$WANT_DISCOUNT"

# ---------------------------------------------------------------------------
# Optional: query --json via native dump of the live profile
# ---------------------------------------------------------------------------
echo "=== optional: query --json via native dump of profile ==="
DUMP_OUT="$TMPDIR_SMOKE/query_profile_json.out"
DUMP_ERR="$TMPDIR_SMOKE/query_profile_json.err"
if "${ENGINE[@]}" query --json "$PROFILE_REL" \
  >"$DUMP_OUT" 2>"$DUMP_ERR"; then
  DUMP_T="$(extract_shared "$DUMP_OUT" "query --json profile")"
  log "  query --json profile:  $DUMP_T"
  assert_shared_contract "$DUMP_T" "query --json profile"
  read -r _dleaf _dmid _dedge ddisc <<<"$DUMP_T"
  # After dump-derived path: require equal to native side (and prefer 818).
  assert_tuples_equal "$PREV_NATIVE" "$DUMP_T" \
    "query --json profile vs native report --json"
  if [[ "$ddisc" != "$WANT_DISCOUNT" ]]; then
    # Equal to native already asserted; note if golden number drifted.
    log "NOTE: dump-derived discount_events=$ddisc (equal to native; golden $WANT_DISCOUNT)"
  fi
  ok "query --json profile: matches native shared fields ($DUMP_T)"
else
  # Fail soft only if engine could not find native for dump — still fail if
  # unexpected crash while CLI was present (we already resolved CLI above).
  cat "$DUMP_ERR" >&2 || true
  cat "$DUMP_OUT" >&2 || true
  # Engine dump path needs find_native_cli; with cargo-only mode, EngineDispatch
  # may still locate cargo. Treat failure as hard because we have native.
  fail "query --json profile (native dump path) failed while native CLI is available"
fi

# Final PERL5LIB guard (children should not have polluted parent)
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ after run (got: $PERL5LIB)"
fi

ok "native_query_json_cross_smoke completed successfully"
exit 0

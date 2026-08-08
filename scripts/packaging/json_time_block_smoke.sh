#!/usr/bin/env bash
# JSON-TIME-BLOCK-MVP: expose dump/model-derived time_block_events (A2 TIME_BLOCK
# multiplicity) on shipped JSON surfaces.
#
# Specs:
#   docs/schemas/perl-engine-dispatch-mvp-v0.md
#   docs/schemas/native-aggregates-json-mvp-v0.md
# Board: JSON-TIME-BLOCK-MVP
#
# Contract (real CLIs only; no re-aggregation):
#   fixtures/v5/default-calls1 → time_block_events == 0
#   fixtures/v5/blocks-calls1  → time_block_events == 916
#     (match stream recount / ProfileModel / JsonlData)
#
# Surfaces:
#   1) Perl   nytprof-engine query --json --jsonl <readstream.jsonl>  (required)
#   2) native nytprof-cli report --json <profile.out>               (optional)
#   3) optional golden TIME_BLOCK tag recount from readstream.jsonl
#
# Never puts crates/ on oracle PERL5LIB. No XS.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/json_time_block_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

DEFAULT_DIR="fixtures/v5/default-calls1"
BLOCKS_DIR="fixtures/v5/blocks-calls1"
DEFAULT_GOLDEN="$DEFAULT_DIR/readstream.jsonl"
BLOCKS_GOLDEN="$BLOCKS_DIR/readstream.jsonl"
DEFAULT_PROFILE="$DEFAULT_DIR/nytprof.out"
BLOCKS_PROFILE="$BLOCKS_DIR/nytprof.out"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"
DISPATCH_PM="$ENGINE_LIB/Devel/NYTProf/EngineDispatch.pm"
DATA_PM="$ENGINE_LIB/Devel/NYTProf/JsonlData.pm"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$DEFAULT_GOLDEN" ]] || fail "missing golden dump $DEFAULT_GOLDEN"
[[ -f "$ROOT/$BLOCKS_GOLDEN" ]] || fail "missing golden dump $BLOCKS_GOLDEN"
[[ -f "$ROOT/$ENGINE_BIN" ]] || fail "missing $ENGINE_BIN"
[[ -f "$ROOT/$DISPATCH_PM" ]] || fail "missing $DISPATCH_PM"
[[ -f "$ROOT/$DATA_PM" ]] || fail "missing $DATA_PM"
command -v perl >/dev/null 2>&1 || fail "perl not on PATH"

# Sanity: never inject crates/ into oracle PERL5LIB from this smoke.
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

# ---------------------------------------------------------------------------
# JsonlData expected values (source of truth for pure-Perl path)
# ---------------------------------------------------------------------------
EXPECT_DEFAULT_TB=""
EXPECT_BLOCKS_TB=""
eval "$(
  perl -I"$ENGINE_LIB" -MDevel::NYTProf::JsonlData -e '
    use strict; use warnings;
    my $d0 = Devel::NYTProf::JsonlData->from_jsonl($ARGV[0]);
    my $d1 = Devel::NYTProf::JsonlData->from_jsonl($ARGV[1]);
    my $tb0 = 0 + ($d0->time_block_events // 0);
    my $tb1 = 0 + ($d1->time_block_events // 0);
    die "default-calls1 JsonlData time_block_events must be 0, got $tb0\n" unless $tb0 == 0;
    die "blocks-calls1 JsonlData time_block_events must be 916, got $tb1\n" unless $tb1 == 916;
    print "EXPECT_DEFAULT_TB=$tb0\n";
    print "EXPECT_BLOCKS_TB=$tb1\n";
  ' "$DEFAULT_GOLDEN" "$BLOCKS_GOLDEN"
)" || fail "failed to load JsonlData time_block_events from goldens"

[[ "$EXPECT_DEFAULT_TB" == "0" ]] || fail "EXPECT_DEFAULT_TB=$EXPECT_DEFAULT_TB"
[[ "$EXPECT_BLOCKS_TB" == "916" ]] || fail "EXPECT_BLOCKS_TB=$EXPECT_BLOCKS_TB"
ok "JsonlData expect: default-calls1 time_block_events=$EXPECT_DEFAULT_TB blocks-calls1=$EXPECT_BLOCKS_TB"

# ---------------------------------------------------------------------------
# Optional independent golden TIME_BLOCK tag recount
# ---------------------------------------------------------------------------
if command -v python3 >/dev/null 2>&1; then
  python3 -c '
import json,sys
for path, want in (
    (sys.argv[1], 0),
    (sys.argv[2], 916),
):
    n = 0
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            o = json.loads(line)
            if o.get("tag") == "TIME_BLOCK":
                n += 1
    if n != want:
        raise SystemExit("%s: TIME_BLOCK tag count %s want %s" % (path, n, want))
    print("golden_recount_ok", path, "TIME_BLOCK", n)
' "$DEFAULT_GOLDEN" "$BLOCKS_GOLDEN" || fail "golden TIME_BLOCK tag recount mismatch"
  ok "golden TIME_BLOCK tag recount: default=0 blocks=916"
else
  log "NOTE: no python3 — skip independent golden TIME_BLOCK tag recount"
fi

# ---------------------------------------------------------------------------
# Assert helpers
# ---------------------------------------------------------------------------
json_assert_time_block() {
  local f="$1"
  local label="$2"
  local want="$3"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
path, label, want = sys.argv[1], sys.argv[2], int(sys.argv[3])
o = json.load(open(path, encoding="utf-8"))
if o.get("ok") is not True:
    raise SystemExit("%s: ok must be true, got %r" % (label, o.get("ok")))
got = o.get("time_block_events")
if got != want:
    raise SystemExit("%s: time_block_events must be %s, got %r" % (label, want, got))
print("time_block_ok", label, "time_block_events=%s" % got)
' "$f" "$label" "$want" || fail "$label: time_block_events assert failed
$(cat "$f")"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      use strict; use warnings;
      my ($f, $label, $want) = @ARGV;
      open my $fh, "<", $f or die "$label: $!";
      local $/; my $obj = JSON::PP->new->decode(<$fh>);
      die "$label: ok\n" unless $obj->{ok};
      my $got = $obj->{time_block_events};
      die "$label: time_block_events missing\n" unless defined $got;
      die "$label: time_block_events want $want got $got\n" unless 0+$got == 0+$want;
      printf "time_block_ok %s time_block_events=%s\n", $label, $got;
    ' "$f" "$label" "$want" || fail "$label: time_block_events assert failed (perl)
$(cat "$f")"
  else
    grep -qE "\"time_block_events\"[[:space:]]*:[[:space:]]*${want}" "$f" \
      || fail "$label: missing time_block_events:$want\n$(cat "$f")"
    log "NOTE: no python3/JSON::PP; used greps for $label"
  fi
}

# ---------------------------------------------------------------------------
# 1. Perl query --json --jsonl default-calls1 → 0
# ---------------------------------------------------------------------------
echo "=== query --json --jsonl default-calls1 (time_block_events=0) ==="
DOUT="$TMPDIR_SMOKE/default_json.out"
DERR="$TMPDIR_SMOKE/default_json.err"
if ! "${ENGINE[@]}" query --json --jsonl "$DEFAULT_GOLDEN" >"$DOUT" 2>"$DERR"; then
  cat "$DERR" >&2 || true
  cat "$DOUT" >&2 || true
  fail "query --json --jsonl default-calls1 failed"
fi
cat "$DOUT"
json_assert_time_block "$DOUT" "perl default-calls1" 0
ok "perl query --json default-calls1: time_block_events=0"

# ---------------------------------------------------------------------------
# 2. Perl query --json --jsonl blocks-calls1 → 916 (×2 consistency)
# ---------------------------------------------------------------------------
echo "=== query --json --jsonl blocks-calls1 (time_block_events=916) ×2 ==="
BOUT1="$TMPDIR_SMOKE/blocks_json_1.out"
BOUT2="$TMPDIR_SMOKE/blocks_json_2.out"
BERR1="$TMPDIR_SMOKE/blocks_json_1.err"
BERR2="$TMPDIR_SMOKE/blocks_json_2.err"

if ! "${ENGINE[@]}" query --json --jsonl "$BLOCKS_GOLDEN" >"$BOUT1" 2>"$BERR1"; then
  cat "$BERR1" >&2 || true
  cat "$BOUT1" >&2 || true
  fail "query --json --jsonl blocks-calls1 run #1 failed"
fi
if ! "${ENGINE[@]}" query --json --jsonl "$BLOCKS_GOLDEN" >"$BOUT2" 2>"$BERR2"; then
  cat "$BERR2" >&2 || true
  cat "$BOUT2" >&2 || true
  fail "query --json --jsonl blocks-calls1 run #2 failed"
fi
cat "$BOUT1"
json_assert_time_block "$BOUT1" "perl blocks-calls1 #1" 916
json_assert_time_block "$BOUT2" "perl blocks-calls1 #2" 916
ok "perl query --json blocks-calls1 ×2: time_block_events=916"

# ---------------------------------------------------------------------------
# 3. Optional native report --json on both fixtures
# ---------------------------------------------------------------------------
echo "=== optional native report --json time_block_events ==="
CLI_CMD=()
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI_CMD=("$NYTPROF_NATIVE_CLI")
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  CLI_CMD=(cargo run -q -p nytprof-cli --)
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/prefix/bin/nytprof-cli")
elif [[ -x "$ROOT/prefix/bin/nytprof-dump" ]]; then
  CLI_CMD=("$ROOT/prefix/bin/nytprof-dump")
elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-dump")
elif [[ -x "$ROOT/target/release/nytprof-dump" ]]; then
  CLI_CMD=("$ROOT/target/release/nytprof-dump")
fi

if [[ ${#CLI_CMD[@]} -gt 0 ]]; then
  if [[ -f "$ROOT/$DEFAULT_PROFILE" ]]; then
    NOUT_D="$TMPDIR_SMOKE/native_default.json"
    NERR_D="$TMPDIR_SMOKE/native_default.err"
    if ! "${CLI_CMD[@]}" report --json "$DEFAULT_PROFILE" >"$NOUT_D" 2>"$NERR_D"; then
      cat "$NERR_D" >&2 || true
      cat "$NOUT_D" >&2 || true
      fail "native report --json default-calls1 failed"
    fi
    cat "$NOUT_D"
    json_assert_time_block "$NOUT_D" "native default-calls1" 0
    ok "native report --json default-calls1: time_block_events=0"
  else
    log "SKIP: native default-calls1 (missing $DEFAULT_PROFILE)"
  fi

  if [[ -f "$ROOT/$BLOCKS_PROFILE" ]]; then
    NOUT_B="$TMPDIR_SMOKE/native_blocks.json"
    NERR_B="$TMPDIR_SMOKE/native_blocks.err"
    if ! "${CLI_CMD[@]}" report --json "$BLOCKS_PROFILE" >"$NOUT_B" 2>"$NERR_B"; then
      cat "$NERR_B" >&2 || true
      cat "$NOUT_B" >&2 || true
      fail "native report --json blocks-calls1 failed"
    fi
    cat "$NOUT_B"
    json_assert_time_block "$NOUT_B" "native blocks-calls1" 916
    ok "native report --json blocks-calls1: time_block_events=916"
  else
    log "SKIP: native blocks-calls1 (missing $BLOCKS_PROFILE)"
  fi
else
  log "SKIP: native report --json (no CLI found)"
fi

ok "json_time_block_smoke (JSON-TIME-BLOCK-MVP) passed"
exit 0

#!/usr/bin/env bash
# JSON-BLOCKS-MVP: greppable A4/A4b convenience integers on query --json
# (and optional native report --json).
#
# Spec: docs/schemas/perl-engine-dispatch-mvp-v0.md
#       docs/schemas/native-aggregates-json-mvp-v0.md
# Board: JSON-BLOCKS-MVP
#
# 1. Golden blocks-calls1: nytprof-engine query --json --jsonl ×2
#    → parse JSON; line_calls_1_5=780, block_line_calls_1_4=810;
#      also leaf_returns=15 / mid_returns=3 / mid_leaf_edge=15;
#      consistent across runs
# 2. default-calls1: line_calls_1_5 present (non-zero from TIME_LINE);
#    block_line_calls_1_4 == 0 (no TIME_BLOCK)
# 3. Optional: native report --json on blocks-calls1 profile when CLI
#    available → same 780/810
#
# Never puts crates/ on oracle PERL5LIB. No XS. Core JSON::PP only.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/json_blocks_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

BLOCKS_DIR="fixtures/v5/blocks-calls1"
BLOCKS_GOLDEN="$BLOCKS_DIR/readstream.jsonl"
BLOCKS_PROFILE="$BLOCKS_DIR/nytprof.out"
DEFAULT_GOLDEN="fixtures/v5/default-calls1/readstream.jsonl"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"
DISPATCH_PM="$ENGINE_LIB/Devel/NYTProf/EngineDispatch.pm"
DATA_PM="$ENGINE_LIB/Devel/NYTProf/JsonlData.pm"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$BLOCKS_GOLDEN" ]] || fail "missing golden dump $BLOCKS_GOLDEN"
[[ -f "$ROOT/$DEFAULT_GOLDEN" ]] || fail "missing golden dump $DEFAULT_GOLDEN"
[[ -f "$ROOT/$ENGINE_BIN" ]] || fail "missing $ENGINE_BIN"
[[ -f "$ROOT/$DISPATCH_PM" ]] || fail "missing $DISPATCH_PM"
[[ -f "$ROOT/$DATA_PM" ]] || fail "missing $DATA_PM"

ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

# Sanity: env must not inject crates/
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

# ---------------------------------------------------------------------------
# JSON field asserts
# ---------------------------------------------------------------------------
json_assert_blocks() {
  local f="$1"
  local label="$2"
  if command -v python3 >/dev/null 2>&1; then
    if ! python3 - "$f" <<'PY'
import json, sys
path = sys.argv[1]
with open(path, encoding="utf-8") as fh:
    obj = json.load(fh)
if not isinstance(obj, dict):
    sys.exit("not an object")
if obj.get("ok") is not True:
    sys.exit(f"ok must be true, got {obj.get('ok')!r}")
for k, want in (
    ("leaf_returns", 15),
    ("mid_returns", 3),
    ("mid_leaf_edge", 15),
    ("line_calls_1_5", 780),
    ("block_line_calls_1_4", 810),
):
    got = obj.get(k)
    if got != want:
        sys.exit(f"{k} must be {want}, got {got!r}")
PY
    then
      fail "$label: blocks JSON fields failed
$(cat "$f")"
    fi
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      use strict; use warnings;
      open my $fh, "<", $ARGV[0] or die $!;
      local $/; my $raw = <$fh>;
      my $obj = JSON::PP->new->decode($raw);
      die "not object\n" unless ref($obj) eq "HASH";
      die "ok must be true\n" unless $obj->{ok};
      die "leaf_returns\n" unless ($obj->{leaf_returns} // -1) == 15;
      die "mid_returns\n"  unless ($obj->{mid_returns}  // -1) == 3;
      die "mid_leaf_edge\n" unless ($obj->{mid_leaf_edge} // -1) == 15;
      die "line_calls_1_5\n" unless ($obj->{line_calls_1_5} // -1) == 780;
      die "block_line_calls_1_4\n" unless ($obj->{block_line_calls_1_4} // -1) == 810;
    ' "$f" || fail "$label: invalid JSON or blocks fields (perl JSON::PP)
$(cat "$f")"
  else
    grep -qE '"line_calls_1_5"[[:space:]]*:[[:space:]]*780' "$f" \
      || fail "$label: missing line_calls_1_5:780\n$(cat "$f")"
    grep -qE '"block_line_calls_1_4"[[:space:]]*:[[:space:]]*810' "$f" \
      || fail "$label: missing block_line_calls_1_4:810\n$(cat "$f")"
    log "NOTE: no python3/perl JSON::PP path fully exercised; used key greps for $label"
  fi
}

json_assert_default_blocks_fields() {
  local f="$1"
  local label="$2"
  if command -v python3 >/dev/null 2>&1; then
    if ! python3 - "$f" <<'PY'
import json, sys
path = sys.argv[1]
with open(path, encoding="utf-8") as fh:
    obj = json.load(fh)
if not isinstance(obj, dict):
    sys.exit("not an object")
lc = obj.get("line_calls_1_5")
if not isinstance(lc, int) or lc < 1:
    sys.exit(f"line_calls_1_5 must be int >= 1 on default-calls1, got {lc!r}")
bl = obj.get("block_line_calls_1_4")
if bl != 0:
    sys.exit(f"block_line_calls_1_4 must be 0 on default-calls1, got {bl!r}")
PY
    then
      fail "$label: default A4/A4b fields failed
$(cat "$f")"
    fi
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      use strict; use warnings;
      open my $fh, "<", $ARGV[0] or die $!;
      local $/; my $obj = JSON::PP->new->decode(<$fh>);
      my $lc = $obj->{line_calls_1_5};
      die "line_calls_1_5\n" unless defined $lc && $lc =~ /^\d+$/ && $lc >= 1;
      my $bl = $obj->{block_line_calls_1_4};
      die "block_line_calls_1_4\n" unless defined $bl && $bl == 0;
    ' "$f" || fail "$label: default A4/A4b fields (perl)
$(cat "$f")"
  else
    grep -qE '"line_calls_1_5"[[:space:]]*:' "$f" \
      || fail "$label: missing line_calls_1_5\n$(cat "$f")"
    grep -qE '"block_line_calls_1_4"[[:space:]]*:[[:space:]]*0' "$f" \
      || fail "$label: missing block_line_calls_1_4:0\n$(cat "$f")"
  fi
}

json_core_fingerprint() {
  local f="$1"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
o=json.load(open(sys.argv[1],encoding="utf-8"))
print(o.get("line_calls_1_5"), o.get("block_line_calls_1_4"),
      o.get("leaf_returns"), o.get("mid_returns"), o.get("mid_leaf_edge"), o.get("ok"))
' "$f"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      open my $fh, "<", $ARGV[0] or die $!;
      local $/; my $o = JSON::PP->new->decode(<$fh>);
      print join(" ",
        $o->{line_calls_1_5}//"", $o->{block_line_calls_1_4}//"",
        $o->{leaf_returns}//"", $o->{mid_returns}//"", $o->{mid_leaf_edge}//"",
        $o->{ok} ? "1" : "0"), "\n";
    ' "$f"
  else
    cat "$f"
  fi
}

# ---------------------------------------------------------------------------
# 1. query --json --jsonl blocks-calls1 ×2
# ---------------------------------------------------------------------------
echo "=== query --json --jsonl blocks-calls1 ×2 ==="
JOUT1="$TMPDIR_SMOKE/blocks_json_1.out"
JOUT2="$TMPDIR_SMOKE/blocks_json_2.out"
JERR1="$TMPDIR_SMOKE/blocks_json_1.err"
JERR2="$TMPDIR_SMOKE/blocks_json_2.err"

if ! "${ENGINE[@]}" query --json --jsonl "$BLOCKS_GOLDEN" >"$JOUT1" 2>"$JERR1"; then
  cat "$JERR1" >&2 || true
  cat "$JOUT1" >&2 || true
  fail "query --json --jsonl blocks-calls1 run #1 failed"
fi
if ! "${ENGINE[@]}" query --json --jsonl "$BLOCKS_GOLDEN" >"$JOUT2" 2>"$JERR2"; then
  cat "$JERR2" >&2 || true
  cat "$JOUT2" >&2 || true
  fail "query --json --jsonl blocks-calls1 run #2 failed"
fi
cat "$JOUT1"
json_assert_blocks "$JOUT1" "blocks json run #1"
json_assert_blocks "$JOUT2" "blocks json run #2"
ok "blocks-calls1 query --json: line_calls_1_5=780 block_line_calls_1_4=810 leaf/mid/edge=15/3/15"

FP1="$(json_core_fingerprint "$JOUT1")"
FP2="$(json_core_fingerprint "$JOUT2")"
if [[ "$FP1" != "$FP2" ]]; then
  fail "blocks query --json not consistent across two runs
--- run1 ---
$FP1
--- run2 ---
$FP2"
fi
ok "blocks query --json consistent across two runs ($FP1)"

# ---------------------------------------------------------------------------
# 2. default-calls1: line non-zero, block 1:4 == 0
# ---------------------------------------------------------------------------
echo "=== query --json --jsonl default-calls1 (A4/A4b presence) ==="
DOUT="$TMPDIR_SMOKE/default_json.out"
DERR="$TMPDIR_SMOKE/default_json.err"
if ! "${ENGINE[@]}" query --json --jsonl "$DEFAULT_GOLDEN" >"$DOUT" 2>"$DERR"; then
  cat "$DERR" >&2 || true
  cat "$DOUT" >&2 || true
  fail "query --json --jsonl default-calls1 failed"
fi
json_assert_default_blocks_fields "$DOUT" "default-calls1"
ok "default-calls1: line_calls_1_5 present (>=1), block_line_calls_1_4=0"

# ---------------------------------------------------------------------------
# 3. Optional native report --json on blocks-calls1 profile
# ---------------------------------------------------------------------------
echo "=== optional native report --json blocks-calls1 ==="
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

if [[ ${#CLI_CMD[@]} -gt 0 && -f "$ROOT/$BLOCKS_PROFILE" ]]; then
  NOUT="$TMPDIR_SMOKE/native_blocks.json"
  NERR="$TMPDIR_SMOKE/native_blocks.err"
  if ! "${CLI_CMD[@]}" report --json "$BLOCKS_PROFILE" >"$NOUT" 2>"$NERR"; then
    cat "$NERR" >&2 || true
    cat "$NOUT" >&2 || true
    fail "native report --json blocks-calls1 failed"
  fi
  cat "$NOUT"
  json_assert_blocks "$NOUT" "native report --json blocks-calls1"
  ok "native report --json blocks-calls1: line_calls_1_5=780 block_line_calls_1_4=810"
else
  log "SKIP: native report --json blocks-calls1 (no CLI and/or missing profile)"
fi

ok "json_blocks_smoke (JSON-BLOCKS-MVP) passed"
exit 0

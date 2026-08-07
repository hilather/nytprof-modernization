#!/usr/bin/env bash
# QUERY-JSON-MVP / QUERY-JSON-EXPAND: structured JSON for nytprof-engine query.
#
# Spec: docs/schemas/perl-engine-dispatch-mvp-v0.md
# Data: docs/schemas/perl-jsonl-data-mvp-v0.md
#
# 1. Golden JSONL: nytprof-engine query --json --jsonl default-calls1 ×2
#    → parse JSON; leaf_returns=15, mid_returns=3, mid_leaf_edge=15;
#      discount_events=818; is_stream_complete true;
#      consistent across runs
# 2. --format=json accepted as alias for --json
# 3. Human default still greppable when --json absent
#
# Never puts crates/ on oracle PERL5LIB. No XS. Core JSON::PP only.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_query_json_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="fixtures/v5/default-calls1"
GOLDEN="$FIXTURE_DIR/readstream.jsonl"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"
DISPATCH_PM="$ENGINE_LIB/Devel/NYTProf/EngineDispatch.pm"
DATA_PM="$ENGINE_LIB/Devel/NYTProf/JsonlData.pm"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$GOLDEN" ]] || fail "missing golden dump $GOLDEN"
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
# JSON field asserts (python3 preferred; perl JSON::PP fallback; key greps last)
# ---------------------------------------------------------------------------
json_assert_mvp() {
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
for k, want in (("leaf_returns", 15), ("mid_returns", 3), ("mid_leaf_edge", 15),
                ("discount_events", 818)):
    got = obj.get(k)
    if got != want:
        sys.exit(f"{k} must be {want}, got {got!r}")
isc = obj.get("is_stream_complete")
if isc is not True and isc != 1:
    sys.exit(f"is_stream_complete must be true/1, got {isc!r}")
reasons = obj.get("incompleteness_reasons")
if not isinstance(reasons, list):
    sys.exit(f"incompleteness_reasons must be array, got {type(reasons).__name__}")
if len(reasons) != 0:
    sys.exit(f"incompleteness_reasons must be empty on complete golden, got {reasons!r}")
for k in ("time_line_events", "pid_start_events", "pid_end_events"):
    got = obj.get(k)
    if not isinstance(got, int) or got < 1:
        sys.exit(f"{k} must be int >= 1, got {got!r}")
subs = obj.get("subs")
if not isinstance(subs, dict):
    sys.exit("subs must be object")
if subs.get("main::leaf") != 15:
    sys.exit(f"subs['main::leaf'] must be 15, got {subs.get('main::leaf')!r}")
if subs.get("main::mid") != 3:
    sys.exit(f"subs['main::mid'] must be 3, got {subs.get('main::mid')!r}")
edges = obj.get("edges")
if not isinstance(edges, dict):
    sys.exit("edges must be object")
ek = "main::mid\tmain::leaf"
if edges.get(ek) != 15:
    sys.exit(f"edges[{ek!r}] must be 15, got {edges.get(ek)!r}")
PY
    then
      fail "$label: JSON MVP/EXPAND fields failed
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
      die "discount_events\n" unless ($obj->{discount_events} // -1) == 818;
      my $isc = $obj->{is_stream_complete};
      die "is_stream_complete\n" unless $isc;
      my $reasons = $obj->{incompleteness_reasons};
      die "incompleteness_reasons not array\n" unless ref($reasons) eq "ARRAY";
      die "incompleteness_reasons non-empty\n" if @$reasons;
      for my $k (qw(time_line_events pid_start_events pid_end_events)) {
        my $v = $obj->{$k};
        die "$k\n" unless defined $v && $v =~ /^\d+$/ && $v >= 1;
      }
      my $subs = $obj->{subs};
      die "subs\n" unless ref($subs) eq "HASH";
      die "subs leaf\n" unless ($subs->{"main::leaf"} // -1) == 15;
      die "subs mid\n"  unless ($subs->{"main::mid"}  // -1) == 3;
      my $edges = $obj->{edges};
      die "edges\n" unless ref($edges) eq "HASH";
      my $ek = "main::mid\tmain::leaf";
      die "edge mid->leaf\n" unless ($edges->{$ek} // -1) == 15;
    ' "$f" || fail "$label: invalid JSON or MVP/EXPAND fields (perl JSON::PP)
$(cat "$f")"
  else
    # Last-resort greps (canonical compact JSON from EngineDispatch).
    grep -qE '"ok"[[:space:]]*:[[:space:]]*true' "$f" \
      || fail "$label: missing ok:true\n$(cat "$f")"
    grep -qE '"leaf_returns"[[:space:]]*:[[:space:]]*15' "$f" \
      || fail "$label: missing leaf_returns:15\n$(cat "$f")"
    grep -qE '"mid_returns"[[:space:]]*:[[:space:]]*3' "$f" \
      || fail "$label: missing mid_returns:3\n$(cat "$f")"
    grep -qE '"mid_leaf_edge"[[:space:]]*:[[:space:]]*15' "$f" \
      || fail "$label: missing mid_leaf_edge:15\n$(cat "$f")"
    grep -qE '"discount_events"[[:space:]]*:[[:space:]]*818' "$f" \
      || fail "$label: missing discount_events:818\n$(cat "$f")"
    grep -qE '"is_stream_complete"[[:space:]]*:[[:space:]]*true' "$f" \
      || fail "$label: missing is_stream_complete:true\n$(cat "$f")"
    grep -qE '"main::leaf"[[:space:]]*:[[:space:]]*15' "$f" \
      || fail "$label: missing subs main::leaf 15\n$(cat "$f")"
    log "NOTE: no python3/perl JSON::PP path fully exercised; used key greps for $label"
  fi
}

json_core_fingerprint() {
  local f="$1"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
o=json.load(open(sys.argv[1],encoding="utf-8"))
print(o.get("leaf_returns"), o.get("mid_returns"), o.get("mid_leaf_edge"),
      o.get("discount_events"), o.get("is_stream_complete"),
      o.get("subs",{}).get("main::leaf"), o.get("subs",{}).get("main::mid"),
      o.get("edges",{}).get("main::mid\tmain::leaf"), o.get("ok"))
' "$f"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      open my $fh, "<", $ARGV[0] or die $!;
      local $/; my $o = JSON::PP->new->decode(<$fh>);
      my $ek = "main::mid\tmain::leaf";
      print join(" ",
        $o->{leaf_returns}//"", $o->{mid_returns}//"", $o->{mid_leaf_edge}//"",
        $o->{discount_events}//"", $o->{is_stream_complete} ? "1" : "0",
        ($o->{subs}//{})->{"main::leaf"}//"", ($o->{subs}//{})->{"main::mid"}//"",
        ($o->{edges}//{})->{$ek}//"", $o->{ok} ? "1" : "0"), "\n";
    ' "$f"
  else
    # Fallback: whole file (canonical encode should match across runs).
    cat "$f"
  fi
}

# ---------------------------------------------------------------------------
# 1. query --json --jsonl ×2
# ---------------------------------------------------------------------------
echo "=== query --json --jsonl default-calls1 ×2 ==="
JOUT1="$TMPDIR_SMOKE/query_json_1.out"
JOUT2="$TMPDIR_SMOKE/query_json_2.out"
JERR1="$TMPDIR_SMOKE/query_json_1.err"
JERR2="$TMPDIR_SMOKE/query_json_2.err"

if ! "${ENGINE[@]}" query --json --jsonl "$GOLDEN" >"$JOUT1" 2>"$JERR1"; then
  cat "$JERR1" >&2 || true
  cat "$JOUT1" >&2 || true
  fail "query --json --jsonl run #1 failed"
fi
if ! "${ENGINE[@]}" query --json --jsonl "$GOLDEN" >"$JOUT2" 2>"$JERR2"; then
  cat "$JERR2" >&2 || true
  cat "$JOUT2" >&2 || true
  fail "query --json --jsonl run #2 failed"
fi
cat "$JOUT1"
json_assert_mvp "$JOUT1" "json run #1"
json_assert_mvp "$JOUT2" "json run #2"
ok "query --json --jsonl ×2: leaf=15 mid=3 edge=15 discount=818 complete"

FP1="$(json_core_fingerprint "$JOUT1")"
FP2="$(json_core_fingerprint "$JOUT2")"
if [[ "$FP1" != "$FP2" ]]; then
  fail "query --json not consistent across two runs
--- run1 fingerprint ---
$FP1
--- run2 fingerprint ---
$FP2
--- raw1 ---
$(cat "$JOUT1")
--- raw2 ---
$(cat "$JOUT2")"
fi
ok "query --json consistent across two runs ($FP1)"

# ---------------------------------------------------------------------------
# 2. --format=json alias
# ---------------------------------------------------------------------------
echo "=== query --format=json --jsonl ==="
FMT_OUT="$TMPDIR_SMOKE/query_format.out"
FMT_ERR="$TMPDIR_SMOKE/query_format.err"
if ! "${ENGINE[@]}" query --format=json --jsonl "$GOLDEN" \
  >"$FMT_OUT" 2>"$FMT_ERR"; then
  cat "$FMT_ERR" >&2 || true
  cat "$FMT_OUT" >&2 || true
  fail "query --format=json --jsonl failed"
fi
json_assert_mvp "$FMT_OUT" "--format=json"
ok "query --format=json accepted with MVP fields"

# --format json (two-arg form)
if ! "${ENGINE[@]}" query --format json --jsonl "$GOLDEN" \
  >"$TMPDIR_SMOKE/query_format2.out" 2>"$TMPDIR_SMOKE/query_format2.err"; then
  cat "$TMPDIR_SMOKE/query_format2.err" >&2 || true
  fail "query --format json (two-arg) failed"
fi
json_assert_mvp "$TMPDIR_SMOKE/query_format2.out" "--format json"
ok "query --format json (two-arg) accepted"

# ---------------------------------------------------------------------------
# 3. Human default unchanged when --json absent
# ---------------------------------------------------------------------------
echo "=== query human default (no --json) ==="
HUM_OUT="$TMPDIR_SMOKE/query_human.out"
HUM_ERR="$TMPDIR_SMOKE/query_human.err"
if ! "${ENGINE[@]}" query --jsonl "$GOLDEN" >"$HUM_OUT" 2>"$HUM_ERR"; then
  cat "$HUM_ERR" >&2 || true
  cat "$HUM_OUT" >&2 || true
  fail "query --jsonl (human) failed"
fi
grep -qE 'main::leaf returns=15' "$HUM_OUT" \
  || fail "human path missing main::leaf returns=15"
grep -qE 'main::mid returns=3' "$HUM_OUT" \
  || fail "human path missing main::mid returns=3"
grep -qE 'main::mid -> main::leaf count=15' "$HUM_OUT" \
  || fail "human path missing main::mid -> main::leaf count=15"
# Must not be a pure JSON object on human path.
if grep -qE '^\s*\{' "$HUM_OUT" && ! grep -qE 'returns=' "$HUM_OUT"; then
  fail "human path looks like JSON only; greppable lines required
$(cat "$HUM_OUT")"
fi
ok "human default: leaf=15 mid=3 mid→leaf=15 (no --json)"

ok "perl query JSON packaging smoke passed"
exit 0

#!/usr/bin/env bash
# Self-test: re-aggregate oracle readstream.jsonl and match committed baselines.
#
# Does NOT require Rust or the oracle Perl env. Python 3 + stdlib only.
#
# Exit 0 when generated aggregates match fixtures/v5/*/aggregates.oracle.json.
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
AGG=(python3 "$DIR/aggregate_from_jsonl.py")

WORKDIR=$(mktemp -d "${TMPDIR:-/tmp}/nytprof-agg-selftest.XXXXXX")
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

pass=0
fail=0
log() { printf '%s\n' "$*"; }
ok()  { pass=$((pass + 1)); log "  PASS: $*"; }
bad() { fail=$((fail + 1)); log "  FAIL: $*" >&2; }

need_file() {
  if [[ ! -f "$1" ]]; then
    log "ERROR: missing $1" >&2
    exit 1
  fi
}

json_equal() {
  # Structural equality via Python json load (tolerates whitespace/key-order
  # only if we re-dump sorted — we compare canonical dumps with sort_keys).
  python3 - "$1" "$2" <<'PY'
import json, sys
a = json.load(open(sys.argv[1], encoding="utf-8"))
b = json.load(open(sys.argv[2], encoding="utf-8"))
# Normalize both to sorted JSON for equality
def canon(o):
    return json.dumps(o, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
if canon(a) != canon(b):
    # Helpful short diff of top-level counts
    for k in ("schema", "time_line_events", "time_block_events", "discount_events"):
        if a.get(k) != b.get(k):
            print(f"  differ {k}: generated={a.get(k)!r} baseline={b.get(k)!r}", file=sys.stderr)
    sa = set((a.get("sub_return_totals") or {}))
    sb = set((b.get("sub_return_totals") or {}))
    if sa != sb:
        print(f"  sub keys only in generated: {sorted(sa - sb)[:10]}", file=sys.stderr)
        print(f"  sub keys only in baseline: {sorted(sb - sa)[:10]}", file=sys.stderr)
    for name in sorted(sa & sb)[:5]:
        if a["sub_return_totals"][name] != b["sub_return_totals"][name]:
            print(
                f"  sub {name}: gen={a['sub_return_totals'][name]!r} "
                f"base={b['sub_return_totals'][name]!r}",
                file=sys.stderr,
            )
    sys.exit(1)
sys.exit(0)
PY
}

check_fixture() {
  local name="$1"
  local fix="$ROOT/fixtures/v5/$name"
  local dump="$fix/readstream.jsonl"
  local base="$fix/aggregates.oracle.json"
  local gen="$WORKDIR/$name.aggregates.json"

  need_file "$dump"
  need_file "$base"

  log "=== aggregates: $name ==="
  # Run from ROOT so source label is fixtures/v5/...
  (
    cd "$ROOT"
    "${AGG[@]}" "fixtures/v5/$name/readstream.jsonl" -o "$gen"
  )

  if json_equal "$gen" "$base"; then
    ok "$name aggregates match baseline"
  else
    bad "$name aggregates differ from baseline"
    # Optional unified diff of pretty JSON (best-effort)
    if command -v diff >/dev/null 2>&1; then
      python3 -c '
import json,sys
for p in sys.argv[1:]:
  o=json.load(open(p,encoding="utf-8"))
  with open(p+".pretty","w",encoding="utf-8") as f:
    json.dump(o, f, indent=2, sort_keys=True)
    f.write("\n")
' "$gen" "$base" 2>/dev/null || true
      diff -u "$base.pretty" "$gen.pretty" | head -n 80 >&2 || true
    fi
  fi

  # Sanity: workload leaf/mid must appear in generated output
  if python3 - "$gen" <<'PY'
import json, sys
o = json.load(open(sys.argv[1], encoding="utf-8"))
subs = set(o.get("workload_subs") or [])
sr = o.get("sub_return_totals") or {}
if "main::leaf" not in sr or "main::mid" not in sr:
    print("missing main::leaf or main::mid in sub_return_totals", file=sys.stderr)
    print("workload_subs=", sorted(subs), file=sys.stderr)
    sys.exit(1)
print("  sanity: main::leaf=", sr["main::leaf"], "main::mid=", sr["main::mid"])
sys.exit(0)
PY
  then
    ok "$name has main::leaf and main::mid"
  else
    bad "$name missing workload leaf/mid"
  fi
}

need_file "$DIR/aggregate_from_jsonl.py"
need_file "$ROOT/fixtures/v5/default-calls1/readstream.jsonl"

log "selftest_aggregates: workdir=$WORKDIR"
check_fixture default-calls1

if [[ -f "$ROOT/fixtures/v5/default-calls2/readstream.jsonl" ]]; then
  check_fixture default-calls2
else
  log "=== skip default-calls2 (no readstream.jsonl) ==="
fi

log ""
log "selftest_aggregates: $pass passed, $fail failed"
if [[ "$fail" -ne 0 ]]; then
  exit 1
fi
log "selftest_aggregates: PASS"
exit 0

#!/usr/bin/env bash
# Differential harness self-test: normalize + compare on golden fixtures.
#
# Does NOT require the oracle Perl env for pure normalize/compare paths.
# Uses fixtures under fixtures/v5/ (readstream.jsonl must already exist).
#
# Cases:
#   a) Identity after normalize → PASS
#   b) Seeded TIME_LINE → TIME_BLOCK after normalize → FAIL compare
#   c) Seeded TIME_LINE ticks mutation → FAIL compare
#   d) Volatile fields differ pre-normalize; after normalize → PASS
#   e) Same suite for default-calls2 when present
#
# Exit non-zero on any failure.
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
NORMALIZE=(python3 "$DIR/normalize_jsonl.py")
COMPARE=(perl "$DIR/compare_jsonl.pl")

# Prefer local temp (not shared /tmp clutter); mktemp still OK.
WORKDIR=$(mktemp -d "${TMPDIR:-/tmp}/nytprof-selftest.XXXXXX")
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

# compare that must succeed
expect_match() {
  local label="$1" a="$2" b="$3"
  if "${COMPARE[@]}" "$a" "$b" >/dev/null; then
    ok "$label"
  else
    bad "$label (expected match)"
  fi
}

# compare that must fail (detect mutation)
expect_mismatch() {
  local label="$1" a="$2" b="$3"
  if "${COMPARE[@]}" "$a" "$b" >/dev/null 2>&1; then
    bad "$label (expected mismatch, comparator returned OK)"
  else
    ok "$label"
  fi
}

normalize_to() {
  local src="$1" dest="$2"
  "${NORMALIZE[@]}" --mode structural "$src" >"$dest"
}

# --- mutations via inline python (deterministic, no extra deps) ---
mutate_tag_timeline_to_block() {
  local src="$1" dest="$2"
  python3 - "$src" "$dest" <<'PY'
import json, sys
src, dest = sys.argv[1], sys.argv[2]
changed = False
out = []
with open(src, encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        o = json.loads(line)
        if not changed and o.get("tag") == "TIME_LINE":
            o["tag"] = "TIME_BLOCK"
            # TIME_BLOCK needs 5 args; pad if needed so it's still valid-ish JSON
            args = list(o.get("args") or [])
            while len(args) < 5:
                args.append(0)
            o["args"] = args
            changed = True
        out.append(o)
if not changed:
    raise SystemExit("no TIME_LINE record to mutate")
with open(dest, "w", encoding="utf-8") as f:
    for o in out:
        f.write(json.dumps(o, separators=(",", ":"), ensure_ascii=False))
        f.write("\n")
PY
}

mutate_timeline_ticks() {
  local src="$1" dest="$2"
  python3 - "$src" "$dest" <<'PY'
import json, sys
src, dest = sys.argv[1], sys.argv[2]
changed = False
out = []
with open(src, encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        o = json.loads(line)
        if not changed and o.get("tag") == "TIME_LINE":
            args = list(o.get("args") or [])
            if not args:
                raise SystemExit("TIME_LINE with empty args")
            # bump ticks (first arg)
            t = args[0]
            if isinstance(t, (int, float)):
                args[0] = int(t) + 1
            else:
                args[0] = 999999
            o["args"] = args
            changed = True
        out.append(o)
if not changed:
    raise SystemExit("no TIME_LINE record to mutate")
with open(dest, "w", encoding="utf-8") as f:
    for o in out:
        f.write(json.dumps(o, separators=(",", ":"), ensure_ascii=False))
        f.write("\n")
PY
}

mutate_volatiles() {
  local src="$1" dest="$2"
  python3 - "$src" "$dest" <<'PY'
import json, sys
src, dest = sys.argv[1], sys.argv[2]
out = []
with open(src, encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        o = json.loads(line)
        tag = o.get("tag")
        args = list(o.get("args") or [])
        if tag == "COMMENT":
            o["args"] = ["MUTATED COMMENT TEXT " + (args[0] if args else "")]
        elif tag == "ATTRIBUTE" and len(args) >= 2:
            if args[0] == "basetime":
                o["args"] = ["basetime", "9999999999"]
            elif args[0] == "application":
                o["args"] = ["application", "/other/host/path/to/workload.pl"]
        elif tag == "NEW_FID" and len(args) >= 1:
            name = args[-1]
            if isinstance(name, str) and ("/" in name or "\\" in name):
                # different absolute path, same basename
                base = name.replace("\\", "/").rstrip("/").rsplit("/", 1)[-1]
                args[-1] = f"/mutated/prefix/{base}"
                o["args"] = args
        out.append(o)
with open(dest, "w", encoding="utf-8") as f:
    for o in out:
        f.write(json.dumps(o, separators=(",", ":"), ensure_ascii=False))
        f.write("\n")
PY
}

run_fixture_suite() {
  local name="$1"
  local fix="$ROOT/fixtures/v5/$name"
  local dump="$fix/readstream.jsonl"
  need_file "$dump"

  local base="$WORKDIR/$name"
  mkdir -p "$base"
  log "=== fixture: $name ==="

  # (a) Identity after normalize
  normalize_to "$dump" "$base/a.norm.jsonl"
  normalize_to "$dump" "$base/a2.norm.jsonl"
  expect_match "$name identity (normalize vs normalize)" \
    "$base/a.norm.jsonl" "$base/a2.norm.jsonl"
  # also normalize once and compare to itself
  expect_match "$name identity (same file)" \
    "$base/a.norm.jsonl" "$base/a.norm.jsonl"

  # (b) TIME_LINE -> TIME_BLOCK must FAIL after normalize
  mutate_tag_timeline_to_block "$dump" "$base/mut_tag.jsonl"
  normalize_to "$base/mut_tag.jsonl" "$base/mut_tag.norm.jsonl"
  expect_mismatch "$name seeded TIME_LINE->TIME_BLOCK" \
    "$base/a.norm.jsonl" "$base/mut_tag.norm.jsonl"

  # (c) TIME_LINE ticks mutation must FAIL
  mutate_timeline_ticks "$dump" "$base/mut_ticks.jsonl"
  normalize_to "$base/mut_ticks.jsonl" "$base/mut_ticks.norm.jsonl"
  expect_mismatch "$name seeded TIME_LINE ticks change" \
    "$base/a.norm.jsonl" "$base/mut_ticks.norm.jsonl"

  # (d) Volatile normalization: basetime, application, COMMENT (+ path on NEW_FID)
  # Without normalize, raw compare of dump vs volatile-mutated should FAIL
  mutate_volatiles "$dump" "$base/mut_vol.jsonl"
  expect_mismatch "$name raw compare detects volatiles (pre-normalize)" \
    "$dump" "$base/mut_vol.jsonl"
  normalize_to "$base/mut_vol.jsonl" "$base/mut_vol.norm.jsonl"
  expect_match "$name volatiles equal after normalize" \
    "$base/a.norm.jsonl" "$base/mut_vol.norm.jsonl"
}

# Sanity: normalizer and comparator exist
need_file "$DIR/normalize_jsonl.py"
need_file "$DIR/compare_jsonl.pl"
need_file "$ROOT/fixtures/v5/default-calls1/readstream.jsonl"

log "selftest_harness: workdir=$WORKDIR"
run_fixture_suite default-calls1

if [[ -f "$ROOT/fixtures/v5/default-calls2/readstream.jsonl" ]]; then
  run_fixture_suite default-calls2
else
  log "=== skip default-calls2 (no readstream.jsonl) ==="
fi

# Aggregate baselines (oracle JSONL → aggregates.oracle.json)
if [[ -x "$DIR/selftest_aggregates.sh" || -f "$DIR/selftest_aggregates.sh" ]]; then
  log "=== aggregates (selftest_aggregates.sh) ==="
  if bash "$DIR/selftest_aggregates.sh"; then
    ok "selftest_aggregates.sh"
  else
    bad "selftest_aggregates.sh"
  fi
else
  log "=== skip aggregates (no selftest_aggregates.sh) ==="
fi

log ""
log "selftest_harness: $pass passed, $fail failed"
if [[ "$fail" -ne 0 ]]; then
  exit 1
fi
log "selftest_harness: PASS"
exit 0

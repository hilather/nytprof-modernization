#!/usr/bin/env bash
# COMPAT-002 / COMPAT-003 evidence: drive shipped normalize_jsonl.py only.
#
# Named rules exercised (structural mode):
#   V1 COMMENT        -> ["<COMMENT>"]
#   V2 basetime       -> "<BASETIME>"
#   V3 application    -> basename / <APP>
#   V4 NEW_FID name   -> basename when path-like
#   V5 floats         -> normalize_number / %.17g (COMPAT-003 dump policy)
#   V6 seq renumber   -> default from 0
#
# Assertions:
#   1) normalize(golden) twice → identical (stable / idempotent output)
#   2) raw golden vs volatile-mutated dump → mismatch
#   3) normalize(golden) vs normalize(mutated) → match
#   4) normalize is idempotent on its own output (second pass unchanged)
#
# Does NOT reimplement normalization. Never puts crates/ on PERL5LIB.
# Independently runnable; also invoked from selftest_harness.sh.
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
NORMALIZE=(python3 "$DIR/normalize_jsonl.py")
COMPARE=(perl "$DIR/compare_jsonl.pl")

FIXTURE="${1:-$ROOT/fixtures/v5/default-calls1/readstream.jsonl}"

WORKDIR=$(mktemp -d "${TMPDIR:-/tmp}/nytprof-compat-norm.XXXXXX")
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

need_file "$DIR/normalize_jsonl.py"
need_file "$DIR/compare_jsonl.pl"
need_file "$FIXTURE"

log "selftest_normalize_compat: COMPAT-002 structural rules + COMPAT-003 float dump policy"
log "  fixture=$FIXTURE"
log "  normalizer=${NORMALIZE[*]}"
log "  workdir=$WORKDIR"

# --- (1) Double normalize of golden: outputs byte-identical ---
"${NORMALIZE[@]}" --mode structural "$FIXTURE" >"$WORKDIR/n1.jsonl"
"${NORMALIZE[@]}" --mode structural "$FIXTURE" >"$WORKDIR/n2.jsonl"
if cmp -s "$WORKDIR/n1.jsonl" "$WORKDIR/n2.jsonl"; then
  ok "COMPAT-002 double-normalize golden: stable (cmp identical)"
else
  bad "COMPAT-002 double-normalize golden: outputs differ"
fi
if "${COMPARE[@]}" "$WORKDIR/n1.jsonl" "$WORKDIR/n2.jsonl" >/dev/null; then
  ok "COMPAT-002 double-normalize golden: compare_jsonl match"
else
  bad "COMPAT-002 double-normalize golden: compare_jsonl mismatch"
fi

# --- (2) Idempotence: normalize(normalized) == normalized ---
"${NORMALIZE[@]}" --mode structural "$WORKDIR/n1.jsonl" >"$WORKDIR/n1b.jsonl"
if cmp -s "$WORKDIR/n1.jsonl" "$WORKDIR/n1b.jsonl"; then
  ok "COMPAT-002 normalize idempotent on own output"
else
  bad "COMPAT-002 normalize not idempotent on own output"
fi

# --- (3) Mutate documented volatiles (basetime path-like app, COMMENT, NEW_FID) ---
# Mutation is only for constructing inputs; rules are applied by normalize_jsonl.py.
python3 - "$FIXTURE" "$WORKDIR/mut.jsonl" <<'PY'
import json, sys
src, dest = sys.argv[1], sys.argv[2]
out = []
saw = {"basetime": False, "application": False, "comment": False, "new_fid": False}
with open(src, encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        o = json.loads(line)
        tag = o.get("tag")
        args = list(o.get("args") or [])
        if tag == "COMMENT":
            o["args"] = ["MUTATED FOR COMPAT-002 " + (args[0] if args else "")]
            saw["comment"] = True
        elif tag == "ATTRIBUTE" and len(args) >= 2:
            if args[0] == "basetime":
                # different wall value (value is fully replaced by normalizer)
                o["args"] = ["basetime", "1111111111"]
                saw["basetime"] = True
            elif args[0] == "application":
                # different absolute path, same basename when path-like
                val = args[1] if isinstance(args[1], str) else "app"
                base = val.replace("\\", "/").rstrip("/").rsplit("/", 1)[-1] or "app"
                o["args"] = ["application", f"/other/host/path/to/{base}"]
                saw["application"] = True
        elif tag == "NEW_FID" and len(args) >= 1:
            name = args[-1]
            if isinstance(name, str) and ("/" in name or "\\" in name):
                base = name.replace("\\", "/").rstrip("/").rsplit("/", 1)[-1]
                args[-1] = f"/mutated/prefix/for/compat002/{base}"
                o["args"] = args
                saw["new_fid"] = True
        out.append(o)
missing = [k for k, v in saw.items() if not v]
if missing:
    raise SystemExit(f"fixture missing expected volatile sites: {missing}")
with open(dest, "w", encoding="utf-8") as f:
    for o in out:
        f.write(json.dumps(o, separators=(",", ":"), ensure_ascii=False))
        f.write("\n")
PY

# Pre-normalize: raw dumps must differ (volatiles are real differences)
if "${COMPARE[@]}" "$FIXTURE" "$WORKDIR/mut.jsonl" >/dev/null 2>&1; then
  bad "COMPAT-002 pre-normalize: expected raw mismatch (basetime/app/COMMENT/NEW_FID)"
else
  ok "COMPAT-002 pre-normalize: raw golden vs mutated volatiles mismatch"
fi

# Post-normalize: must match
"${NORMALIZE[@]}" --mode structural "$WORKDIR/mut.jsonl" >"$WORKDIR/mut.norm.jsonl"
if "${COMPARE[@]}" "$WORKDIR/n1.jsonl" "$WORKDIR/mut.norm.jsonl" >/dev/null; then
  ok "COMPAT-002 post-normalize: mutated volatiles match golden (V1–V4)"
else
  bad "COMPAT-002 post-normalize: mutated volatiles still differ after normalize"
fi

# --- (4) Spot-check sentinels / basenames in normalized output (script output, not a reimpl) ---
python3 - "$WORKDIR/n1.jsonl" <<'PY'
import json, sys
path = sys.argv[1]
basetime = app = comments = 0
pathlike_new_fid = 0
with open(path, encoding="utf-8") as f:
    for line in f:
        o = json.loads(line)
        tag = o.get("tag")
        args = o.get("args") or []
        if tag == "COMMENT":
            assert args == ["<COMMENT>"], args
            comments += 1
        elif tag == "ATTRIBUTE" and len(args) >= 2:
            if args[0] == "basetime":
                assert args[1] == "<BASETIME>", args
                basetime += 1
            elif args[0] == "application":
                # basename only (no path separators) or <APP>
                assert isinstance(args[1], str), args
                assert args[1] == "<APP>" or ("/" not in args[1] and "\\" not in args[1]), args
                app += 1
        elif tag == "NEW_FID" and args:
            name = args[-1]
            if isinstance(name, str) and ("/" in name or "\\" in name):
                pathlike_new_fid += 1
if basetime < 1:
    raise SystemExit("no basetime ATTRIBUTE after normalize")
if app < 1:
    raise SystemExit("no application ATTRIBUTE after normalize")
if comments < 1:
    raise SystemExit("no COMMENT after normalize")
if pathlike_new_fid != 0:
    raise SystemExit(f"path-like NEW_FID names remain after normalize: {pathlike_new_fid}")
print("spotcheck_ok", basetime, app, comments)
PY
ok "COMPAT-002 spot-check: <BASETIME>, application basename, <COMMENT>, NEW_FID basenames"

# --- (5) COMPAT-003: float re-encode stability (drive real normalizer) ---
# Build a tiny JSONL with a non-integral float; normalize twice; cmp.
python3 - "$WORKDIR/float_in.jsonl" <<'PY'
import json
from pathlib import Path
import sys
dest = Path(sys.argv[1])
# Use a value that may have noisy repr if not re-boxed; structural path uses %.17g.
rec = {"seq": 0, "tag": "SUB_RETURN", "args": [1, 0.1 + 0.2, 0.3, "main::x"]}
dest.write_text(json.dumps(rec, ensure_ascii=False) + "\n", encoding="utf-8")
PY
"${NORMALIZE[@]}" --mode structural "$WORKDIR/float_in.jsonl" >"$WORKDIR/float_n1.jsonl"
"${NORMALIZE[@]}" --mode structural "$WORKDIR/float_in.jsonl" >"$WORKDIR/float_n2.jsonl"
if cmp -s "$WORKDIR/float_n1.jsonl" "$WORKDIR/float_n2.jsonl"; then
  ok "COMPAT-003 float dump policy: double-normalize stable (%.17g path)"
else
  bad "COMPAT-003 float dump policy: double-normalize unstable"
fi
# Integer ticks must survive as integers (TIME_LINE)
python3 - "$WORKDIR/ticks_in.jsonl" <<'PY'
import json
from pathlib import Path
import sys
Path(sys.argv[1]).write_text(
    json.dumps({"seq": 9, "tag": "TIME_LINE", "args": [42, 1, 5]}, ensure_ascii=False) + "\n",
    encoding="utf-8",
)
PY
"${NORMALIZE[@]}" --mode structural "$WORKDIR/ticks_in.jsonl" >"$WORKDIR/ticks_out.jsonl"
python3 - "$WORKDIR/ticks_out.jsonl" <<'PY'
import json, sys
o = json.loads(open(sys.argv[1], encoding="utf-8").read().strip())
assert o["tag"] == "TIME_LINE"
assert o["args"][0] == 42 and isinstance(o["args"][0], int), o
assert o["seq"] == 0  # renumbered
print("ticks_ok")
PY
ok "COMPAT-003 integer ticks preserved; seq renumbered (V5/V6)"

log ""
log "selftest_normalize_compat: $pass passed, $fail failed"
if [[ "$fail" -ne 0 ]]; then
  exit 1
fi
log "selftest_normalize_compat: PASS"
exit 0

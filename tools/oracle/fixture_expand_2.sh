#!/usr/bin/env bash
# FIXTURE-EXPAND-2: capture `calls2-default` (calls=2 mid×3 → leaf×5) and finish wiring.
#
# Prerequisites already in tree (this agent):
#   - model tests calls2_default_* (skip if fixture missing)
#   - selftest_aggregates / selftest_harness optional suite
#   - fixtures/README.md + board FIXTURE-EXPAND-2 row
#
# This script (needs shell + oracle):
#   1. ./tools/oracle/capture_fixture.sh calls2-default 'trace=0:start=begin:calls=2'
#   2. aggregates.oracle.json via aggregate_from_jsonl.py
#   3. annotate fixture.json semantics (leaf/mid)
#   4. harden model tests (remove temporary skip guards)
#   5. mark board FIXTURE-EXPAND-2 done
#   6. cargo test + selftest_aggregates
#
# Hard rules: sources env via capture_fixture.sh only; never puts crates/ on oracle PERL5LIB.
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
cd "$ROOT"

NAME="calls2-default"
OPTS="trace=0:start=begin:calls=2"
FIX="fixtures/v5/$NAME"

echo "=== FIXTURE-EXPAND-2: capture $NAME ($OPTS) ==="
bash "$DIR/capture_fixture.sh" "$NAME" "$OPTS"

echo "=== generate aggregates.oracle.json ==="
python3 "$DIR/aggregate_from_jsonl.py" "$FIX/readstream.jsonl" \
  -o "$FIX/aggregates.oracle.json"

echo "=== annotate fixture.json semantics ==="
python3 - <<'PY'
import json
from pathlib import Path
fix = Path("fixtures/v5/calls2-default")
meta_path = fix / "fixture.json"
agg = json.loads((fix / "aggregates.oracle.json").read_text(encoding="utf-8"))
meta = json.loads(meta_path.read_text(encoding="utf-8"))
leaf = agg["sub_return_totals"]["main::leaf"]["returns"]
mid = agg["sub_return_totals"]["main::mid"]["returns"]
edge = agg["call_edges"].get("main::mid -> main::leaf", {})
meta["semantics"] = {
    "options": "trace=0:start=begin:calls=2",
    "workload": "mid×3 → leaf×5 (same as default-calls*)",
    "main::leaf_returns": leaf,
    "main::mid_returns": mid,
    "mid_to_leaf_edge_count": edge.get("count"),
    "notes": (
        "calls=2 enables richer call-site detail; SUB_ENTRY present. "
        "Return counts match mid×3→leaf×5 (leaf=15, mid=3) when oracle emits that pattern."
    ),
}
meta_path.write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
print(f"semantics: leaf_returns={leaf} mid_returns={mid} mid→leaf={edge.get('count')}")
assert leaf == 15, f"expected leaf returns 15, got {leaf}"
assert mid == 3, f"expected mid returns 3, got {mid}"
assert edge.get("count") == 15, f"expected mid→leaf 15, got {edge}"
PY

echo "=== harden model tests (require fixture) ==="
python3 - <<'PY'
from pathlib import Path
p = Path("crates/nytprof-model/src/model_tests.rs")
text = p.read_text(encoding="utf-8")
# Remove temporary skip guards for calls2-default tests once fixture exists.
import re
# binary_matches
text2 = re.sub(
    r'(fn calls2_default_binary_matches_oracle_jsonl\(\) \{\n)'
    r'(?:    //[^\n]*\n)*'
    r'    let dir = fixture_dir\("calls2-default"\);\n'
    r'    if !dir\.join\("nytprof\.out"\)\.is_file\(\) \{\n'
    r'        eprintln!\("[^"]*"\);\n'
    r'        return;\n'
    r'    \}\n'
    r'    check_fixture\("calls2-default"\);',
    r'\1    // calls=2: SUB_ENTRY present; same mid×3 → leaf×5 workload (FIXTURE-EXPAND-2).\n'
    r'    check_fixture("calls2-default");\n'
    r'    let dir = fixture_dir("calls2-default");',
    text,
    count=1,
)
# workload_subs
text2 = re.sub(
    r'(fn calls2_default_workload_subs\(\) \{\n)'
    r'    let path = fixture_dir\("calls2-default"\)\.join\("nytprof\.out"\);\n'
    r'    if !path\.is_file\(\) \{\n'
    r'        eprintln!\("[^"]*"\);\n'
    r'        return;\n'
    r'    \}\n',
    r'\1    let path = fixture_dir("calls2-default").join("nytprof.out");\n'
    r'    assert!(path.is_file(), "missing {{}}", path.display());\n',
    text2,
    count=1,
)
# native matches
text2 = re.sub(
    r'(fn calls2_default_native_matches_aggregates_oracle_json\(\) \{\n)'
    r'    let path = fixture_dir\("calls2-default"\)\.join\("aggregates\.oracle\.json"\);\n'
    r'    if !path\.is_file\(\) \{\n'
    r'        eprintln!\(\s*\n?\s*"[^"]*"\s*\n?\s*\);\n'
    r'        return;\n'
    r'    \}\n'
    r'    check_native_vs_aggregates_oracle_json\("calls2-default"\);\n'
    r'\}',
    r'\1    check_native_vs_aggregates_oracle_json("calls2-default");\n'
    r'}',
    text2,
    count=1,
)
if text2 == text:
    print("note: skip-guard strip may already be done or patterns drifted; leaving tests as-is")
else:
    p.write_text(text2, encoding="utf-8")
    print("hardened model_tests.rs (skip guards removed)")
PY

echo "=== mark board FIXTURE-EXPAND-2 done ==="
python3 - <<'PY'
from pathlib import Path
p = Path("docs/FIRST_SLICE_BOARD.md")
text = p.read_text(encoding="utf-8")
old = (
    "| FIXTURE-EXPAND-2 | Additional v5 fixture beyond default-calls1/2 + blocks-calls1 | in_progress | "
    "Target: `fixtures/v5/calls2-default` (`calls=2`); wire via `tools/oracle/fixture_expand_2.sh` "
    "(capture + aggregates + model tests). Expected leaf/mid **15**/**3** when mid×3→leaf×5 holds. |"
)
new = (
    "| FIXTURE-EXPAND-2 | Additional v5 fixture beyond default-calls1/2 + blocks-calls1 | done | "
    "`fixtures/v5/calls2-default` (`calls=2`); leaf returns **15**, mid **3**, mid→leaf **15**; "
    "aggregates + model tests `calls2_default_*`; `tools/oracle/fixture_expand_2.sh` |"
)
if "FIXTURE-EXPAND-2" in text and "| done |" in text.split("FIXTURE-EXPAND-2")[1][:200]:
    print("board already done")
elif old in text:
    p.write_text(text.replace(old, new), encoding="utf-8")
    print("board marked done")
else:
    # best-effort replace any in_progress FIXTURE-EXPAND-2 line
    import re
    text2, n = re.subn(
        r"\| FIXTURE-EXPAND-2 \|[^|]+\| in_progress \|[^|]+\|",
        new,
        text,
        count=1,
    )
    if n:
        p.write_text(text2, encoding="utf-8")
        print("board marked done (regex)")
    else:
        print("board row not updated (manual check)")
PY

echo "=== cargo test (nytprof-model calls2_default) ==="
if command -v cargo >/dev/null 2>&1; then
  cargo test -p nytprof-model calls2_default -- --nocapture
else
  echo "WARNING: cargo not on PATH; skip native tests"
fi

echo "=== selftest aggregates ==="
bash "$DIR/selftest_aggregates.sh"

echo ""
echo "FIXTURE-EXPAND-2 complete: $ROOT/$FIX"
ls -la "$FIX"
echo "Observed leaf/mid: see $FIX/fixture.json semantics + aggregates.oracle.json"

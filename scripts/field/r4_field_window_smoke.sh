#!/usr/bin/env bash
# R4-FIELD-WINDOW-PACK smoke: collector layout + residual honesty on dual-sink fixtures.
#
# Spec:  docs/schemas/r4-field-window-mvp-v0.md
# Guide: docs/R4_FIELD_WINDOW.md
#
# Runs scripts/field/r4_field_window_collect.sh on dual-sink default_calls1 into a
# temp dir and asserts:
#   - summary.json schema / no_default_flip / collection_default=v5 / residuals all false
#   - MANIFEST.md + env/provenance.txt present
#   - when native discoverable: v5+v6 report leaf=15 mid=3; convert both ways rc=0
#
# Does NOT flip product defaults. Not part of offline_gate (field package).
# Never puts crates/ on oracle PERL5LIB.
#
# Usage:
#   ./scripts/field/r4_field_window_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

COLLECT="$ROOT/scripts/field/r4_field_window_collect.sh"
FIXTURE_V5="fixtures/e4/dual-sink/default_calls1_v5.nytprof"
FIXTURE_V6="fixtures/e4/dual-sink/default_calls1_v6.nytprof"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -x "$COLLECT" || -f "$COLLECT" ]] || fail "missing collector $COLLECT"
[[ -f "$ROOT/$FIXTURE_V5" ]] || fail "missing $FIXTURE_V5"
[[ -f "$ROOT/$FIXTURE_V6" ]] || fail "missing $FIXTURE_V6"
[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"

if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

chmod +x "$COLLECT" 2>/dev/null || true

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

log() { printf '%s\n' "$*"; }
log "=== R4 field-window smoke: collect into $TMP/pack ==="

bash "$COLLECT" \
  --out "$TMP/pack" \
  --site lab-smoke \
  --note "r4_field_window_smoke.sh" \
  || fail "collector exited non-zero"

PACK="$TMP/pack"
[[ -f "$PACK/summary.json" ]] || fail "missing summary.json"
[[ -f "$PACK/MANIFEST.md" ]] || fail "missing MANIFEST.md"
[[ -f "$PACK/env/provenance.txt" ]] || fail "missing env/provenance.txt"
[[ -f "$PACK/profiles/README.md" ]] || fail "missing profiles/README.md"
[[ -d "$PACK/runs" ]] || fail "missing runs/"
[[ -d "$PACK/artifacts" ]] || fail "missing artifacts/"
ok "pack layout present"

python3 - <<PY
import json, sys
from pathlib import Path
pack = Path("$PACK")
s = json.loads((pack / "summary.json").read_text())
errs = []
if s.get("schema") != "r4-field-window-mvp-v0":
    errs.append(f"schema={s.get('schema')!r}")
if s.get("no_default_flip") is not True:
    errs.append(f"no_default_flip={s.get('no_default_flip')!r} (must be true)")
if s.get("collection_default") != "v5":
    errs.append(f"collection_default={s.get('collection_default')!r} (must be 'v5')")
res = s.get("residuals") or {}
for k in (
    "r4_format_default_flip",
    "r3_product_default_flip",
    "col008_batched_rust_writer",
    "lossy_convert",
    "public_perf_certification",
):
    if res.get(k) is not False:
        errs.append(f"residuals.{k}={res.get(k)!r} (must be false)")
if s.get("site") != "lab-smoke":
    errs.append(f"site={s.get('site')!r}")
runs = {r["id"]: r for r in s.get("runs") or []}
native = bool(s.get("native_discoverable"))

def need(rid):
    if rid not in runs:
        errs.append(f"missing run {rid}")
        return {}
    return runs[rid]

if native:
    v5 = need("v5_report_default_calls1_v5")
    v6 = need("v6_report_default_calls1_v6")
    if v5 and v5.get("rc") != 0:
        errs.append(f"v5 report rc={v5.get('rc')} (expected 0)")
    if v6 and v6.get("rc") != 0:
        errs.append(f"v6 report rc={v6.get('rc')} (expected 0)")
    for label, r in (("v5", v5), ("v6", v6)):
        if not r:
            continue
        if r.get("leaf_returns") != 15:
            errs.append(f"{label} leaf_returns={r.get('leaf_returns')} (expected 15)")
        if r.get("mid_returns") != 3:
            errs.append(f"{label} mid_returns={r.get('mid_returns')} (expected 3)")
    c2v6 = need("convert_to_v6_default_calls1_v5")
    c2v5 = need("convert_to_v5_default_calls1_v6")
    if c2v6 and c2v6.get("rc") != 0:
        errs.append(f"convert_to_v6 rc={c2v6.get('rc')} (expected 0)")
    if c2v5 and c2v5.get("rc") != 0:
        errs.append(f"convert_to_v5 rc={c2v5.get('rc')} (expected 0)")
    r_after_v6 = need("report_after_convert_to_v6_default_calls1_v5")
    r_after_v5 = need("report_after_convert_to_v5_default_calls1_v6")
    for label, r in (("after_v6", r_after_v6), ("after_v5", r_after_v5)):
        if not r:
            continue
        if r.get("rc") != 0:
            errs.append(f"report_{label} rc={r.get('rc')} (expected 0)")
        if r.get("leaf_returns") != 15 or r.get("mid_returns") != 3:
            errs.append(
                f"report_{label} leaf/mid={r.get('leaf_returns')}/{r.get('mid_returns')} (expected 15/3)"
            )
    # capability honesty
    cap = pack / "capability" / "capability.json"
    if cap.is_file():
        try:
            c = json.loads(cap.read_text())
            if c.get("skipped"):
                errs.append("capability skipped despite native_discoverable")
            else:
                if c.get("ok") is not True:
                    errs.append(f"capability ok={c.get('ok')!r}")
                if c.get("collection_default") != "v5":
                    errs.append(
                        f"capability collection_default={c.get('collection_default')!r} (must be v5)"
                    )
                for k in ("v6_decode", "v6_report", "convert"):
                    if c.get(k) is not True:
                        errs.append(f"capability {k}={c.get(k)!r} (expected true)")
        except json.JSONDecodeError as e:
            errs.append(f"capability.json parse: {e}")
    else:
        errs.append("missing capability/capability.json")
else:
    # honest path without native: layout + honesty only
    pass

if errs:
    print("FAIL:", "; ".join(errs), file=sys.stderr)
    sys.exit(1)
print("summary.json honesty + samples OK")
print("native_discoverable=", native)
print("collection_default=", s.get("collection_default"))
fx = s.get("fixture_default_calls1") or {}
print("fixture leaf=", fx.get("leaf_returns"), "mid=", fx.get("mid_returns"))
PY

# Ensure provenance / manifest mention no_default_flip
grep -q 'no_default_flip: true' "$PACK/env/provenance.txt" \
  || fail "provenance missing no_default_flip: true"
grep -q 'no_default_flip' "$PACK/MANIFEST.md" \
  || fail "MANIFEST missing no_default_flip note"
grep -q 'collection_default' "$PACK/MANIFEST.md" \
  || fail "MANIFEST missing collection_default note"

ok "R4 field-window smoke passed (no product default flip; collection_default=v5)"

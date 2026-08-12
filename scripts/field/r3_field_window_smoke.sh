#!/usr/bin/env bash
# R3-FIELD-WINDOW-PACK smoke: collector layout + residual honesty on fixtures.
#
# Spec:  docs/schemas/r3-field-window-mvp-v0.md
# Guide: docs/R3_FIELD_WINDOW.md
#
# Runs scripts/field/r3_field_window_collect.sh on default-calls1 into a temp
# dir and asserts:
#   - summary.json schema / no_default_flip / residuals all false
#   - MANIFEST.md + env/provenance.txt present
#   - when native discoverable: auto report leaf=15 mid=3
#
# Does NOT flip product defaults. Not part of offline_gate (field package).
# Never puts crates/ on oracle PERL5LIB.
#
# Usage:
#   ./scripts/field/r3_field_window_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

COLLECT="$ROOT/scripts/field/r3_field_window_collect.sh"
FIXTURE="fixtures/v5/default-calls1/nytprof.out"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -x "$COLLECT" || -f "$COLLECT" ]] || fail "missing collector $COLLECT"
[[ -f "$ROOT/$FIXTURE" ]] || fail "missing $FIXTURE"
[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"

if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

chmod +x "$COLLECT" 2>/dev/null || true

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

log() { printf '%s\n' "$*"; }
log "=== R3 field-window smoke: collect into $TMP/pack ==="

bash "$COLLECT" \
  --out "$TMP/pack" \
  --site lab-smoke \
  --note "r3_field_window_smoke.sh" \
  || fail "collector exited non-zero"

PACK="$TMP/pack"
[[ -f "$PACK/summary.json" ]] || fail "missing summary.json"
[[ -f "$PACK/MANIFEST.md" ]] || fail "missing MANIFEST.md"
[[ -f "$PACK/env/provenance.txt" ]] || fail "missing env/provenance.txt"
[[ -f "$PACK/profiles/README.md" ]] || fail "missing profiles/README.md"
[[ -d "$PACK/runs" ]] || fail "missing runs/"
ok "pack layout present"

python3 - <<PY
import json, sys
from pathlib import Path
pack = Path("$PACK")
s = json.loads((pack / "summary.json").read_text())
errs = []
if s.get("schema") != "r3-field-window-mvp-v0":
    errs.append(f"schema={s.get('schema')!r}")
if s.get("no_default_flip") is not True:
    errs.append(f"no_default_flip={s.get('no_default_flip')!r} (must be true)")
res = s.get("residuals") or {}
for k in (
    "r3_product_default_flip",
    "r4_format_default_flip",
    "col007_product_writer",
    "v6_wire_freeze",
    "public_perf_certification",
):
    if res.get(k) is not False:
        errs.append(f"residuals.{k}={res.get(k)!r} (must be false)")
if s.get("site") != "lab-smoke":
    errs.append(f"site={s.get('site')!r}")
runs = {r["id"]: r for r in s.get("runs") or []}
if "engine_auto_report_default-calls1" not in runs:
    errs.append("missing run engine_auto_report_default-calls1")
native = bool(s.get("native_discoverable"))
auto = runs.get("engine_auto_report_default-calls1") or {}
if native:
    if auto.get("rc") != 0:
        errs.append(f"auto report rc={auto.get('rc')} (expected 0 when native present)")
    if auto.get("leaf_returns") != 15:
        errs.append(f"auto leaf_returns={auto.get('leaf_returns')} (expected 15)")
    if auto.get("mid_returns") != 3:
        errs.append(f"auto mid_returns={auto.get('mid_returns')} (expected 3)")
    # force-no-native auto: STDERR fallback note; rc==0 only when oracle install present
    fn = runs.get("engine_auto_force_no_native_report_default-calls1") or {}
    if fn:
        if not fn.get("stderr_fallback_note"):
            errs.append("force-no-native auto missing stderr fallback note")
        oracle_install = Path("$ROOT") / "baseline/6.15/install"
        if oracle_install.is_dir() and fn.get("rc") != 0:
            errs.append(
                f"force-no-native auto rc={fn.get('rc')} (expected 0 when oracle install present)"
            )
        # if oracle absent: honest non-zero rc is OK; note is the contract
    else:
        errs.append("missing run engine_auto_force_no_native_report_default-calls1")
    # force-no-native native: must fail closed (no silent legacy success)
    nfn = runs.get("engine_native_force_no_native_report_default-calls1") or {}
    if not nfn:
        errs.append("missing run engine_native_force_no_native_report_default-calls1")
    else:
        if nfn.get("rc") == 0:
            errs.append(
                "native+force-no-native rc=0 (expected non-zero fail-closed; no silent legacy)"
            )
        if nfn.get("leaf_returns") == 15 and nfn.get("mid_returns") == 3:
            errs.append(
                "native+force-no-native reported leaf/mid 15/3 (must not look like native success)"
            )
    # capability when native
    cap = pack / "capability" / "capability.json"
    if cap.is_file():
        try:
            c = json.loads(cap.read_text())
            if c.get("skipped"):
                errs.append("capability skipped despite native_discoverable")
            elif c.get("ok") is not True:
                errs.append(f"capability ok={c.get('ok')!r}")
        except json.JSONDecodeError as e:
            errs.append(f"capability.json parse: {e}")
else:
    # honest path without native: auto may still work via legacy
    if auto and auto.get("rc") not in (0, None) and auto.get("rc") != 0:
        # document but allow non-zero if oracle missing
        pass
if errs:
    print("FAIL:", "; ".join(errs), file=sys.stderr)
    sys.exit(1)
print("summary.json honesty + samples OK")
print("native_discoverable=", native)
print("auto_rc=", auto.get("rc"), "leaf=", auto.get("leaf_returns"), "mid=", auto.get("mid_returns"))
PY

# Ensure provenance mentions no_default_flip
grep -q 'no_default_flip: true' "$PACK/env/provenance.txt" \
  || fail "provenance missing no_default_flip: true"
grep -q 'no_default_flip' "$PACK/MANIFEST.md" \
  || fail "MANIFEST missing no_default_flip note"

ok "R3 field-window smoke passed (no product default flip)"

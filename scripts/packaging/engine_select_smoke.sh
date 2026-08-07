#!/usr/bin/env bash
# Engine-selection smoke for nytprof-cli (--engine / NYTPROF_ENGINE).
#
# Spec: docs/schemas/engine-selection-mvp-v0.md
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/engine_select_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE="fixtures/v5/default-calls1/nytprof.out"
ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml (need workspace for engine smoke)"
[[ -d "$ROOT/crates/nytprof-cli" ]] || fail "missing crates/nytprof-cli"
[[ -f "$ROOT/$FIXTURE" ]] || fail "missing fixture $FIXTURE"
command -v cargo >/dev/null 2>&1 || fail "cargo required for engine_select_smoke (optional native path)"

CLI=(cargo run -q -p nytprof-cli --)

# ---------------------------------------------------------------------------
# --engine=native report: main::leaf and returns=15
# ---------------------------------------------------------------------------
REPORT_OUT="$(mktemp)"
trap 'rm -f "$REPORT_OUT"' EXIT
if ! "${CLI[@]}" --engine=native report "$FIXTURE" >"$REPORT_OUT" 2>/tmp/engine_select_report.err; then
  cat /tmp/engine_select_report.err >&2 || true
  fail "--engine=native report failed (exit non-zero)"
fi
grep -q 'main::leaf' "$REPORT_OUT" || fail "report missing main::leaf:\n$(cat "$REPORT_OUT")"
grep -q 'returns=15' "$REPORT_OUT" || fail "report missing returns=15:\n$(cat "$REPORT_OUT")"
ok "--engine=native report shows main::leaf and returns=15"

# ---------------------------------------------------------------------------
# --engine=native verify: OK
# ---------------------------------------------------------------------------
VERIFY_OUT="$(mktemp)"
trap 'rm -f "$REPORT_OUT" "$VERIFY_OUT"' EXIT
if ! "${CLI[@]}" --engine=native verify "$FIXTURE" >"$VERIFY_OUT" 2>/tmp/engine_select_verify.err; then
  cat /tmp/engine_select_verify.err >&2 || true
  fail "--engine=native verify failed (exit non-zero)"
fi
grep -q 'OK:' "$VERIFY_OUT" || fail "verify missing OK: line:\n$(cat "$VERIFY_OUT")"
ok "--engine=native verify → OK"

# ---------------------------------------------------------------------------
# --engine=not-a-thing: non-zero exit
# ---------------------------------------------------------------------------
set +e
"${CLI[@]}" --engine=not-a-thing verify "$FIXTURE" >/tmp/engine_select_bogus.out 2>/tmp/engine_select_bogus.err
BOGUS_RC=$?
set -e
[[ "$BOGUS_RC" -ne 0 ]] || fail "--engine=not-a-thing unexpectedly succeeded"
if ! grep -qiE 'engine|invalid|allowed' /tmp/engine_select_bogus.err /tmp/engine_select_bogus.out 2>/dev/null; then
  echo "NOTE: bogus-engine stderr (for operators):" >&2
  cat /tmp/engine_select_bogus.err >&2 || true
fi
ok "--engine=not-a-thing exits non-zero (rc=$BOGUS_RC)"

# ---------------------------------------------------------------------------
# --engine=legacy: non-zero + clear message (not a fake Rust legacy path)
# ---------------------------------------------------------------------------
set +e
"${CLI[@]}" --engine=legacy report "$FIXTURE" >/tmp/engine_select_legacy.out 2>/tmp/engine_select_legacy.err
LEGACY_RC=$?
set -e
[[ "$LEGACY_RC" -ne 0 ]] || fail "--engine=legacy unexpectedly succeeded"
if ! grep -qiE 'legacy|baseline/6\.15|oracle' /tmp/engine_select_legacy.err /tmp/engine_select_legacy.out 2>/dev/null; then
  cat /tmp/engine_select_legacy.err >&2 || true
  fail "--engine=legacy did not mention legacy/oracle path"
fi
ok "--engine=legacy exits non-zero with oracle message (rc=$LEGACY_RC)"

ok "engine-select packaging smoke passed"
exit 0

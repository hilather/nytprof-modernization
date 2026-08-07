#!/usr/bin/env bash
# Perl engine-dispatch packaging smoke.
#
# Spec: docs/schemas/perl-engine-dispatch-mvp-v0.md
#
# 1. cd repo root
# 2. cargo build -q -p nytprof-cli  (native checks only)
# 3. --engine=native report  → main::leaf + returns=15 (and mid/3 if present)
# 4. --engine=not-a-thing    → non-zero
# 5. --engine=legacy report  → exit 0 when oracle present (does not require cargo for this step)
# 6. Also run legacy_only_smoke.sh and engine_select_smoke.sh if present
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_engine_dispatch_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE="fixtures/v5/default-calls1/nytprof.out"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"
DISPATCH_PM="$ENGINE_LIB/Devel/NYTProf/EngineDispatch.pm"
LEGACY_PM="$ENGINE_LIB/Devel/NYTProf/LegacyBridge.pm"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$FIXTURE" ]] || fail "missing fixture $FIXTURE"
[[ -f "$ROOT/$ENGINE_BIN" ]] || fail "missing $ENGINE_BIN"
[[ -f "$ROOT/$DISPATCH_PM" ]] || fail "missing $DISPATCH_PM"
[[ -f "$ROOT/$LEGACY_PM" ]] || fail "missing $LEGACY_PM (PERL-LEGACY-BRIDGE)"

ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

# ---------------------------------------------------------------------------
# Native: cargo build only for native checks
# ---------------------------------------------------------------------------
command -v cargo >/dev/null 2>&1 || fail "cargo required for native portion of this smoke"
cargo build -q -p nytprof-cli
ok "cargo build -p nytprof-cli"

REPORT_OUT="$TMPDIR_SMOKE/native_report.out"
REPORT_ERR="$TMPDIR_SMOKE/native_report.err"
if ! "${ENGINE[@]}" --engine=native report "$FIXTURE" >"$REPORT_OUT" 2>"$REPORT_ERR"; then
  cat "$REPORT_ERR" >&2 || true
  fail "--engine=native report failed (exit non-zero)"
fi
grep -q 'main::leaf' "$REPORT_OUT" || fail "native report missing main::leaf:\n$(cat "$REPORT_OUT")"
grep -q 'returns=15' "$REPORT_OUT" || fail "native report missing returns=15:\n$(cat "$REPORT_OUT")"
if grep -q 'main::mid' "$REPORT_OUT"; then
  grep -qE 'main::mid[[:space:]]+returns=3\b' "$REPORT_OUT" \
    || fail "native report has main::mid but missing returns=3:\n$(cat "$REPORT_OUT")"
  ok "--engine=native report: main::leaf returns=15 and main::mid returns=3"
else
  ok "--engine=native report: main::leaf and returns=15"
fi

# ---------------------------------------------------------------------------
# Invalid engine: non-zero
# ---------------------------------------------------------------------------
set +e
"${ENGINE[@]}" --engine=not-a-thing report "$FIXTURE" \
  >"$TMPDIR_SMOKE/bogus.out" 2>"$TMPDIR_SMOKE/bogus.err"
BOGUS_RC=$?
set -e
[[ "$BOGUS_RC" -ne 0 ]] || fail "--engine=not-a-thing unexpectedly succeeded"
if ! grep -qiE 'engine|invalid|allowed' "$TMPDIR_SMOKE/bogus.err" "$TMPDIR_SMOKE/bogus.out" 2>/dev/null; then
  echo "NOTE: bogus-engine stderr:" >&2
  cat "$TMPDIR_SMOKE/bogus.err" >&2 || true
fi
ok "--engine=not-a-thing exits non-zero (rc=$BOGUS_RC)"

# ---------------------------------------------------------------------------
# Legacy: must not require cargo success for this step itself
# ---------------------------------------------------------------------------
set +e
"${ENGINE[@]}" --engine=legacy report "$FIXTURE" \
  >"$TMPDIR_SMOKE/legacy.out" 2>"$TMPDIR_SMOKE/legacy.err"
LEGACY_RC=$?
set -e

ORACLE_PERL5LIB="$ROOT/baseline/6.15/oracle-perl5lib.txt"
if [[ -f "$ORACLE_PERL5LIB" ]]; then
  [[ "$LEGACY_RC" -eq 0 ]] || {
    cat "$TMPDIR_SMOKE/legacy.err" >&2 || true
    cat "$TMPDIR_SMOKE/legacy.out" >&2 || true
    fail "--engine=legacy report failed with oracle present (rc=$LEGACY_RC)"
  }
  if ! grep -qiE 'OK:|legacy|dump_readstream|stream dump|JSONL' \
    "$TMPDIR_SMOKE/legacy.out" "$TMPDIR_SMOKE/legacy.err" 2>/dev/null; then
    echo "NOTE: legacy stdout/stderr:" >&2
    cat "$TMPDIR_SMOKE/legacy.out" >&2 || true
    cat "$TMPDIR_SMOKE/legacy.err" >&2 || true
  fi
  ok "--engine=legacy report exit 0 with oracle present"
else
  [[ "$LEGACY_RC" -ne 0 ]] \
    || fail "--engine=legacy unexpectedly succeeded without oracle"
  ok "--engine=legacy non-zero without oracle (rc=$LEGACY_RC) — expected"
fi

# ---------------------------------------------------------------------------
# Sibling packaging smokes when present
# ---------------------------------------------------------------------------
if [[ -x "$ROOT/scripts/packaging/legacy_only_smoke.sh" ]]; then
  ok "running scripts/packaging/legacy_only_smoke.sh"
  bash "$ROOT/scripts/packaging/legacy_only_smoke.sh"
else
  echo "NOTE: scripts/packaging/legacy_only_smoke.sh not present; skipping"
fi

if [[ -x "$ROOT/scripts/packaging/engine_select_smoke.sh" ]]; then
  ok "running scripts/packaging/engine_select_smoke.sh"
  bash "$ROOT/scripts/packaging/engine_select_smoke.sh"
else
  echo "NOTE: scripts/packaging/engine_select_smoke.sh not present; skipping"
fi

ok "perl engine-dispatch packaging smoke passed"
exit 0

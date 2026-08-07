#!/usr/bin/env bash
# CI-OFFLINE-GATE / CI-OFFLINE-GATE-EXPAND / CI-CAPABILITY-GATE /
# CI-QUERY-JSON-GATE — single documented fail-fast gate for critical
# offline R1 checks.
#
# Steps (in order; exit non-zero on first failure):
#   1. cargo test of offline native packages (honest skip if cargo absent)
#   2. tools/oracle/selftest_harness.sh (required)
#   3. scripts/packaging/dual_path_smoke.sh (primary packaging path)
#   4. scripts/packaging/engine_auto_fallback_smoke.sh (ENGINE-AUTO-FALLBACK)
#   5. scripts/packaging/perl_jsonl_data_all_smoke.sh (pure-Perl JsonlData roll-up, incl. stream_complete / discount)
#   6. scripts/packaging/perl_query_json_smoke.sh (QUERY-JSON-MVP / QUERY-JSON-EXPAND;
#      pure-Perl golden --jsonl; no cargo required) — CI-QUERY-JSON-GATE
#   7. scripts/packaging/native_agg_json_smoke.sh when native available (optional)
#   8. scripts/packaging/native_query_json_cross_smoke.sh when native available
#      (NATIVE-QUERY-JSON-CROSS: native report --json vs Perl query --json)
#   9. scripts/packaging/capability_selftest_smoke.sh when cargo or prefix/target
#      native CLI exists (honest skip when native unavailable; same pattern as
#      packaging_gate). dual_path with cargo usually installs prefix/bin first.
#
# Primary packaging choice: dual_path_smoke.sh (BUILD dual-path policy entry).
# Alternatives not re-run here (document only):
#   ./scripts/packaging/packaging_gate.sh          # broader packaging suite
#   ./scripts/packaging/makemaker_dual_path_smoke.sh  # MakeMaker facade
#
# Non-goals: multi-OS CI matrix (BUILD-006), full packaging_gate breadth.
# Isolation: never puts crates/ on oracle PERL5LIB (parent does not source
# oracle env; child smokes own isolation).
#
# Policy: docs/BUILD_SUPPORT_POLICY.md
# Board:  CI-OFFLINE-GATE / CI-OFFLINE-GATE-EXPAND / CI-CAPABILITY-GATE /
#         CI-QUERY-JSON-GATE / NATIVE-QUERY-JSON-CROSS (docs/FIRST_SLICE_BOARD.md)
#
# Usage (from repo root or any cwd):
#   ./scripts/ci/offline_gate.sh
# Optional Make target (after perl Makefile.PL):
#   make offline-gate
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

HARNESS="$ROOT/tools/oracle/selftest_harness.sh"
PACKAGING="$ROOT/scripts/packaging/dual_path_smoke.sh"
ENGINE_AUTO_FALLBACK="$ROOT/scripts/packaging/engine_auto_fallback_smoke.sh"
JSONL_DATA_ALL="$ROOT/scripts/packaging/perl_jsonl_data_all_smoke.sh"
QUERY_JSON_SMOKE="$ROOT/scripts/packaging/perl_query_json_smoke.sh"
CAPABILITY_SMOKE="$ROOT/scripts/packaging/capability_selftest_smoke.sh"

banner() {
  echo
  echo "================================================================"
  echo " OFFLINE GATE: $*"
  echo "================================================================"
}

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

run_required() {
  local name="$1"
  local path="$2"
  banner "$name"
  if [[ ! -f "$path" ]]; then
    fail "required script missing: $path"
  fi
  bash "$path"
  ok "step: $name"
}

# True when native CLI can be exercised (cargo or prefix/target / NYTPROF_NATIVE_CLI).
# Same condition as packaging_gate capability step (CI-CAPABILITY-GATE).
native_cli_available() {
  command -v cargo >/dev/null 2>&1 \
    || [[ -x "$ROOT/prefix/bin/nytprof-cli" ]] \
    || [[ -x "$ROOT/prefix/bin/nytprof-dump" ]] \
    || [[ -x "$ROOT/target/debug/nytprof-dump" ]] \
    || [[ -x "$ROOT/target/release/nytprof-dump" ]] \
    || [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]
}

echo "offline_gate: repo root $ROOT"
echo "offline_gate: fail-fast; never puts crates/ on oracle PERL5LIB"
echo "offline_gate: board CI-OFFLINE-GATE-EXPAND / CI-CAPABILITY-GATE / CI-QUERY-JSON-GATE / NATIVE-QUERY-JSON-CROSS; policy docs/BUILD_SUPPORT_POLICY.md"

# ---------------------------------------------------------------------------
# 1. Focused cargo tests for offline native packages (skip if cargo absent)
# ---------------------------------------------------------------------------
banner "cargo test (offline packages)"
if ! command -v cargo >/dev/null 2>&1; then
  echo "SKIP: cargo not on PATH — offline native package tests not run"
  echo "  (honest skip; harness + packaging still required)"
  echo "  To exercise cargo: install rustc/cargo, then re-run this gate"
elif [[ ! -f "$ROOT/Cargo.toml" ]] || [[ ! -d "$ROOT/crates" ]]; then
  echo "SKIP: crates/ Cargo workspace not present — offline native package tests not run"
  echo "  (honest skip; harness + packaging still required)"
else
  ok "cargo available: $(cargo --version 2>/dev/null || echo unknown)"
  # Focused packages used by offline native tools; does not touch oracle PERL5LIB.
  cargo test \
    -p nytprof-format-v5 \
    -p nytprof-model \
    -p nytprof-report \
    -p nytprof-cli
  ok "step: cargo test (nytprof-format-v5 model report cli)"
fi

# ---------------------------------------------------------------------------
# 2. Oracle differential harness (required)
# ---------------------------------------------------------------------------
run_required "selftest_harness" "$HARNESS"

# ---------------------------------------------------------------------------
# 3. Primary packaging path: dual-path smoke (legacy required; native if cargo)
# ---------------------------------------------------------------------------
run_required "dual_path_smoke (primary packaging)" "$PACKAGING"

# ---------------------------------------------------------------------------
# 4. ENGINE-AUTO-FALLBACK: auto prefer-native / fall-back-legacy (required)
# ---------------------------------------------------------------------------
run_required "engine_auto_fallback_smoke (ENGINE-AUTO-FALLBACK)" "$ENGINE_AUTO_FALLBACK"

# ---------------------------------------------------------------------------
# 5. Pure-Perl JsonlData roll-up (returns/edges/line_totals/subdefs/source/a4b/meta/pid/stream_complete/discount)
# ---------------------------------------------------------------------------
run_required "perl_jsonl_data_all_smoke (JsonlData pure-Perl)" "$JSONL_DATA_ALL"

# ---------------------------------------------------------------------------
# 6. QUERY-JSON-MVP / QUERY-JSON-EXPAND: structured query --json via pure-Perl
#    golden --jsonl (CI-QUERY-JSON-GATE). Required; no cargo. Fail-fast if script missing.
# ---------------------------------------------------------------------------
run_required "perl_query_json_smoke (QUERY-JSON-MVP / QUERY-JSON-EXPAND / CI-QUERY-JSON-GATE)" "$QUERY_JSON_SMOKE"

# ---------------------------------------------------------------------------
# 7. NATIVE-AGG-JSON (optional when smoke present + native CLI available)
# ---------------------------------------------------------------------------
NATIVE_AGG_SMOKE="$ROOT/scripts/packaging/native_agg_json_smoke.sh"
banner "native_agg_json_smoke (NATIVE-AGG-JSON, optional)"
if [[ -f "$NATIVE_AGG_SMOKE" ]] && native_cli_available; then
  bash "$NATIVE_AGG_SMOKE"
  ok "step: native_agg_json_smoke"
elif [[ -f "$NATIVE_AGG_SMOKE" ]]; then
  echo "SKIP: native_agg_json_smoke present but no cargo/prefix/target native CLI"
else
  echo "SKIP: native_agg_json_smoke.sh not present"
fi

# ---------------------------------------------------------------------------
# 8. NATIVE-QUERY-JSON-CROSS: native report --json vs Perl query --json
#    (shared fields 15/3/15 + discount). Needs native CLI; pure-Perl query
#    --jsonl alone is covered by step 6.
# ---------------------------------------------------------------------------
CROSS_SMOKE="$ROOT/scripts/packaging/native_query_json_cross_smoke.sh"
banner "native_query_json_cross_smoke (NATIVE-QUERY-JSON-CROSS, optional when native)"
if [[ -f "$CROSS_SMOKE" ]] && native_cli_available; then
  bash "$CROSS_SMOKE"
  ok "step: native_query_json_cross_smoke"
elif [[ -f "$CROSS_SMOKE" ]]; then
  echo "SKIP: native_query_json_cross_smoke present but no cargo/prefix/target native CLI"
  echo "  (pure-Perl query --json covered by step 6 perl_query_json_smoke)"
else
  echo "SKIP: native_query_json_cross_smoke.sh not present"
fi

# ---------------------------------------------------------------------------
# 9. CAPABILITY-SELFTEST (+ JSON): run when native CLI can be exercised
#    (CI-CAPABILITY-GATE). dual_path with cargo typically installs prefix/bin
#    so this step usually runs after step 3 on developer hosts with cargo.
# ---------------------------------------------------------------------------
banner "capability_selftest_smoke (CI-CAPABILITY-GATE)"
if [[ ! -f "$CAPABILITY_SMOKE" ]]; then
  fail "required script missing: $CAPABILITY_SMOKE"
fi
if native_cli_available; then
  bash "$CAPABILITY_SMOKE"
  ok "step: capability_selftest_smoke"
else
  echo "SKIP: no cargo/prefix/target native CLI — CAPABILITY-SELFTEST not run"
  echo "  (honest skip; same condition as packaging_gate capability step)"
  echo "  looked for: cargo, prefix/bin/{nytprof-cli,nytprof-dump},"
  echo "  target/{debug,release}/nytprof-dump, \$NYTPROF_NATIVE_CLI"
  echo "  To exercise: install rustc/cargo and re-run, or ./scripts/packaging/install_native.sh"
fi

banner "ALL PASSED"
ok "offline_gate completed successfully"
echo "NOTE: broader packaging_gate / makemaker_dual_path_smoke are not part of this gate"
exit 0

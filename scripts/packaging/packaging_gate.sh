#!/usr/bin/env bash
# Unified packaging gate: run packaging smokes in order, fail-fast.
#
# Order:
#   1. legacy_only_smoke.sh
#   2. engine_select_smoke.sh
#   3. perl_engine_dispatch_smoke.sh
#   4. if present: install_native.sh then native_install_smoke.sh
#   5. optional: native_optional_smoke.sh (cargo tests; skip-friendly)
#   6. capability_selftest_smoke.sh when cargo or prefix/target CLI exists
#
# Dual-path support tiers (legacy vs optional-native only) are verified by
# dual_path_smoke.sh — that is the BUILD policy entry and is intentionally
# not re-run here to avoid duplicating legacy + native install steps.
# Policy: docs/BUILD_SUPPORT_POLICY.md
#
# Never puts crates/ on oracle PERL5LIB. Does not source oracle env into the
# parent shell for the whole gate — each child smoke owns its isolation.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/packaging_gate.sh
# Dual-path only:
#   ./scripts/packaging/dual_path_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

PACK="$ROOT/scripts/packaging"

banner() {
  echo
  echo "================================================================"
  echo " PACKAGING GATE: $*"
  echo "================================================================"
}

run_required() {
  local name="$1"
  local path="$2"
  banner "$name"
  if [[ ! -f "$path" ]]; then
    echo "ERROR: required script missing: $path" >&2
    exit 1
  fi
  if [[ ! -x "$path" ]]; then
    echo "ERROR: required script not executable: $path" >&2
    exit 1
  fi
  bash "$path"
  echo "→ step OK: $name"
}

run_if_present() {
  local name="$1"
  local path="$2"
  banner "$name"
  if [[ ! -f "$path" ]]; then
    echo "NOTE: $path not present; skipping $name"
    return 0
  fi
  if [[ ! -x "$path" ]]; then
    echo "ERROR: script present but not executable: $path" >&2
    exit 1
  fi
  bash "$path"
  echo "→ step OK: $name"
}

echo "packaging_gate: repo root $ROOT"
echo "packaging_gate: fail-fast; never mutates oracle PERL5LIB with crates/"

# 1–3: required core packaging smokes
run_required "legacy_only_smoke" "$PACK/legacy_only_smoke.sh"
run_required "engine_select_smoke" "$PACK/engine_select_smoke.sh"
run_required "perl_engine_dispatch_smoke" "$PACK/perl_engine_dispatch_smoke.sh"

# 4: native install path when scripts exist
if [[ -f "$PACK/install_native.sh" ]]; then
  run_if_present "install_native" "$PACK/install_native.sh"
  if [[ -f "$PACK/native_install_smoke.sh" ]]; then
    run_if_present "native_install_smoke" "$PACK/native_install_smoke.sh"
  else
    banner "native_install_smoke"
    echo "ERROR: install_native.sh present but native_install_smoke.sh missing" >&2
    exit 1
  fi
else
  banner "install_native + native_install_smoke"
  echo "NOTE: scripts/packaging/install_native.sh not present; skipping native install path"
fi

# 5: optional cargo tests (script itself skips cleanly if cargo absent)
run_if_present "native_optional_smoke" "$PACK/native_optional_smoke.sh"

# 6: CAPABILITY-SELFTEST — packaging-native smoke (fails closed without CLI/cargo).
# Run when cargo is present or a prefix/target binary already exists so legacy-only
# hosts that reach this gate without a native half get an honest skip note.
# Invoke via bash (do not require +x) so a freshly checked-in script still runs.
CAP_SMOKE="$PACK/capability_selftest_smoke.sh"
if [[ -f "$CAP_SMOKE" ]]; then
  if command -v cargo >/dev/null 2>&1 \
    || [[ -x "$ROOT/prefix/bin/nytprof-cli" ]] \
    || [[ -x "$ROOT/prefix/bin/nytprof-dump" ]] \
    || [[ -x "$ROOT/target/debug/nytprof-dump" ]] \
    || [[ -x "$ROOT/target/release/nytprof-dump" ]] \
    || [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
    banner "capability_selftest_smoke"
    bash "$CAP_SMOKE"
    echo "→ step OK: capability_selftest_smoke"
  else
    banner "capability_selftest_smoke"
    echo "NOTE: no cargo/prefix/target native CLI; skipping CAPABILITY-SELFTEST (legacy-only)"
  fi
fi

banner "ALL PASSED"
echo "OK: packaging_gate completed successfully"
exit 0

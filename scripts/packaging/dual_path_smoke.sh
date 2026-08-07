#!/usr/bin/env bash
# Dual-path packaging smoke (BUILD-DUAL-PATH / BUILD-001 support tiers).
#
# 1. Always run legacy_only_smoke.sh (required; must not need Cargo).
# 2. If cargo is on PATH: run install_native + native_install_smoke when
#    present, else native_optional_smoke; fail if those fail.
# 3. If cargo missing: honest skip for native half; exit 0 if legacy passed.
#
# Never puts crates/ on oracle PERL5LIB (child smokes own isolation).
#
# Policy: docs/BUILD_SUPPORT_POLICY.md
# Usage (from repo root or any cwd):
#   ./scripts/packaging/dual_path_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

PACK="$ROOT/scripts/packaging"
LEGACY="$PACK/legacy_only_smoke.sh"
INSTALL_NATIVE="$PACK/install_native.sh"
NATIVE_INSTALL_SMOKE="$PACK/native_install_smoke.sh"
NATIVE_OPTIONAL="$PACK/native_optional_smoke.sh"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

banner() {
  echo
  echo "----------------------------------------------------------------"
  echo " DUAL-PATH: $*"
  echo "----------------------------------------------------------------"
}

echo "dual_path_smoke: repo root $ROOT"
echo "dual_path_smoke: policy docs/BUILD_SUPPORT_POLICY.md"
echo "dual_path_smoke: never mutates oracle PERL5LIB with crates/"

# ---------------------------------------------------------------------------
# 1. Legacy-only (required; must succeed without Cargo)
# ---------------------------------------------------------------------------
banner "legacy-only (required)"
[[ -f "$LEGACY" ]] || fail "required script missing: $LEGACY"
# Always invoke via bash so missing +x on children does not block the policy entry.
bash "$LEGACY"
ok "legacy-only path passed"

# ---------------------------------------------------------------------------
# 2–3. Optional-native when cargo present; honest skip otherwise
# ---------------------------------------------------------------------------
if ! command -v cargo >/dev/null 2>&1; then
  banner "optional-native"
  echo "SKIP: cargo not on PATH — optional-native half not exercised"
  echo "  (legacy-only succeeded; this is a valid dual-path outcome)"
  echo "  To exercise native: install cargo/rustc, then re-run this script"
  banner "ALL PASSED (legacy only; native skipped)"
  ok "dual_path_smoke completed (legacy OK, native skipped — no cargo)"
  exit 0
fi

ok "cargo present: $(cargo --version 2>/dev/null || echo unknown)"

if [[ -f "$INSTALL_NATIVE" ]]; then
  banner "install_native + native_install_smoke"
  bash "$INSTALL_NATIVE"
  if [[ -f "$NATIVE_INSTALL_SMOKE" ]]; then
    bash "$NATIVE_INSTALL_SMOKE"
  else
    fail "install_native.sh present but native_install_smoke.sh missing"
  fi
  ok "native install path passed"
elif [[ -f "$NATIVE_OPTIONAL" ]]; then
  banner "native_optional_smoke (no install_native.sh)"
  bash "$NATIVE_OPTIONAL"
  ok "native optional path passed"
else
  fail "cargo is present but neither install_native.sh nor native_optional_smoke.sh exists under scripts/packaging/"
fi

banner "ALL PASSED"
ok "dual_path_smoke completed (legacy + optional-native)"
exit 0

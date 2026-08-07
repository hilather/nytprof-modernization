#!/usr/bin/env bash
# MakeMaker dual-path packaging smoke (BUILD-MAKEMAKER-OPT).
#
# Proves the candidate Makefile.PL packaging entry:
#   1. perl Makefile.PL (default NYTPROF_NATIVE=0) works without cargo
#   2. make legacy-smoke runs scripts/packaging/legacy_only_smoke.sh
#   3. When cargo is present: make native-install (and dual-path via make)
#   4. Exit non-zero on any failure
#
# Never puts crates/ on oracle PERL5LIB (child smokes own isolation).
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/makemaker_dual_path_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

banner() {
  echo
  echo "----------------------------------------------------------------"
  echo " MAKEMAKER-SMOKE: $*"
  echo "----------------------------------------------------------------"
}

echo "makemaker_dual_path_smoke: repo root $ROOT"
echo "makemaker_dual_path_smoke: candidate packaging entry (not full XS CPAN)"
echo "makemaker_dual_path_smoke: never mutates oracle PERL5LIB with crates/"

[[ -f "$ROOT/Makefile.PL" ]] || fail "missing root Makefile.PL (BUILD-MAKEMAKER-OPT)"
[[ -f "$ROOT/scripts/packaging/legacy_only_smoke.sh" ]] || fail "missing legacy_only_smoke.sh"
[[ -f "$ROOT/scripts/packaging/dual_path_smoke.sh" ]] || fail "missing dual_path_smoke.sh"
[[ -f "$ROOT/scripts/packaging/install_native.sh" ]] || fail "missing install_native.sh"

# Work in a temp copy of the packaging entry so we do not leave a dirty
# developer Makefile behind if configure was half-applied — but still run
# make from repo root so packaging scripts resolve correctly. We clean our
# generated artifacts on exit when we created them.
SMOKE_OWNED_MAKEFILE=0
cleanup() {
  if [[ "${SMOKE_OWNED_MAKEFILE}" -eq 1 ]]; then
    # Remove only MakeMaker products this smoke created/refreshed.
    rm -f "$ROOT/Makefile" "$ROOT/Makefile.old" \
      "$ROOT/MYMETA.json" "$ROOT/MYMETA.yml" \
      "$ROOT/nytprof-packaging.mode" \
      "$ROOT/pm_to_blib" 2>/dev/null || true
    rm -rf "$ROOT/blib" 2>/dev/null || true
  fi
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# 1. Configure default legacy path (must not require cargo)
# ---------------------------------------------------------------------------
banner "perl Makefile.PL (NYTPROF_NATIVE=0 default; no cargo required)"

# Force legacy for this step even if the operator exported NYTPROF_NATIVE=1.
# Preserve any pre-existing Makefile by regenerating intentionally.
export NYTPROF_NATIVE=0
if ! perl Makefile.PL; then
  fail "perl Makefile.PL failed under NYTPROF_NATIVE=0 (must work without cargo)"
fi
SMOKE_OWNED_MAKEFILE=1

[[ -f "$ROOT/Makefile" ]] || fail "Makefile.PL did not produce Makefile"
[[ -f "$ROOT/nytprof-packaging.mode" ]] || fail "missing nytprof-packaging.mode stamp"

if ! grep -q 'native_mode=off' "$ROOT/nytprof-packaging.mode"; then
  fail "expected native_mode=off in nytprof-packaging.mode for NYTPROF_NATIVE=0"
fi
if ! grep -q 'not_full_xs_cpan=1' "$ROOT/nytprof-packaging.mode"; then
  fail "expected not_full_xs_cpan=1 honesty stamp"
fi
ok "Makefile.PL produced Makefile (legacy mode stamp OK)"

# Sanity: generated Makefile must expose packaging targets
for t in legacy-smoke dual-path-smoke native-install; do
  if ! grep -q "^${t}:" "$ROOT/Makefile" && ! grep -q "^${t} :" "$ROOT/Makefile"; then
    # MakeMaker may indent; also accept .PHONY listing + recipe label
    if ! grep -E -q "^${t}:|^\\.PHONY:.*${t}" "$ROOT/Makefile"; then
      fail "Makefile missing target: $t"
    fi
  fi
done
ok "Makefile exposes legacy-smoke dual-path-smoke native-install"

# ---------------------------------------------------------------------------
# 2. make legacy-smoke (must not invoke cargo)
# ---------------------------------------------------------------------------
banner "make legacy-smoke (no cargo on critical path)"
if ! make legacy-smoke; then
  fail "make legacy-smoke failed"
fi
ok "make legacy-smoke passed"

# ---------------------------------------------------------------------------
# 3. Optional: make dual-path-smoke always (legacy + native-if-cargo)
# ---------------------------------------------------------------------------
banner "make dual-path-smoke"
if ! make dual-path-smoke; then
  fail "make dual-path-smoke failed"
fi
ok "make dual-path-smoke passed"

# ---------------------------------------------------------------------------
# 4. When cargo present: make native-install; also probe NYTPROF_NATIVE=1 configure
# ---------------------------------------------------------------------------
if command -v cargo >/dev/null 2>&1; then
  ok "cargo present: $(cargo --version 2>/dev/null || echo unknown)"

  banner "make native-install (cargo present)"
  if ! make native-install; then
    fail "make native-install failed with cargo present"
  fi
  ok "make native-install passed"

  if [[ -f "$ROOT/scripts/packaging/native_install_smoke.sh" ]]; then
    banner "make native-smoke"
    if ! make native-smoke; then
      fail "make native-smoke failed"
    fi
    ok "make native-smoke passed"
  fi

  banner "NYTPROF_NATIVE=1 perl Makefile.PL (require cargo)"
  if ! NYTPROF_NATIVE=1 perl Makefile.PL; then
    fail "NYTPROF_NATIVE=1 perl Makefile.PL failed despite cargo on PATH"
  fi
  if ! grep -q 'native_mode=on' "$ROOT/nytprof-packaging.mode"; then
    fail "expected native_mode=on after NYTPROF_NATIVE=1 configure"
  fi
  ok "NYTPROF_NATIVE=1 configure stamp OK"

  # Restore legacy stamp for cleanliness of leftover tree (cleanup removes it).
  NYTPROF_NATIVE=0 perl Makefile.PL >/dev/null
else
  banner "optional-native"
  echo "SKIP: cargo not on PATH — native-install half not exercised via make"
  echo "  (legacy MakeMaker path succeeded; valid dual-path outcome)"

  # When cargo is absent, NYTPROF_NATIVE=1 must fail configure (honest).
  banner "NYTPROF_NATIVE=1 must fail without cargo"
  set +e
  out="$(NYTPROF_NATIVE=1 perl Makefile.PL 2>&1)"
  rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then
    fail "NYTPROF_NATIVE=1 perl Makefile.PL succeeded without cargo (must die)"
  fi
  echo "$out" | grep -qi 'cargo' || fail "NYTPROF_NATIVE=1 error should mention cargo"
  ok "NYTPROF_NATIVE=1 correctly refuses configure without cargo"

  # Re-establish legacy Makefile for any later inspection before cleanup.
  NYTPROF_NATIVE=0 perl Makefile.PL >/dev/null
fi

banner "ALL PASSED"
ok "makemaker_dual_path_smoke completed"
exit 0

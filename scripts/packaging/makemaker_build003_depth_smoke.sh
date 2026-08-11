#!/usr/bin/env bash
# MakeMaker packaging depth smoke toward BUILD-003 (BUILD-003-DEPTH).
#
# Proves closer dual-build wiring without claiming full XS CPAN dual-build:
#   1. NYTPROF_NATIVE=0 perl Makefile.PL works without cargo
#   2. make install-facade installs pure-Perl engine under prefix (no cargo)
#   3. Installed facade runs pure-Perl query --jsonl (legacy-only surface)
#   4. When cargo present: make dual-install + native report via prefix engine
#   5. Honesty stamps: not_full_xs_cpan=1, packaging_depth=BUILD-003-depth-v0
#   6. Exit non-zero on any failure
#
# Never puts crates/ on oracle PERL5LIB.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/makemaker_build003_depth_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_JSONL="fixtures/v5/default-calls1/readstream.jsonl"
FIXTURE_OUT="fixtures/v5/default-calls1/nytprof.out"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

banner() {
  echo
  echo "----------------------------------------------------------------"
  echo " BUILD003-DEPTH: $*"
  echo "----------------------------------------------------------------"
}

echo "makemaker_build003_depth_smoke: repo root $ROOT"
echo "makemaker_build003_depth_smoke: toward BUILD-003 — NOT full XS CPAN dual-build"
echo "makemaker_build003_depth_smoke: never mutates oracle PERL5LIB with crates/"

[[ -f "$ROOT/Makefile.PL" ]] || fail "missing root Makefile.PL"
[[ -f "$ROOT/scripts/packaging/install_facade.sh" ]] || fail "missing install_facade.sh"
[[ -f "$ROOT/scripts/packaging/install_native.sh" ]] || fail "missing install_native.sh"
[[ -f "$ROOT/$FIXTURE_JSONL" ]] || fail "missing $FIXTURE_JSONL"
[[ -f "$ROOT/$FIXTURE_OUT" ]] || fail "missing $FIXTURE_OUT"

SMOKE_OWNED_MAKEFILE=0
# Use a private prefix so we never clobber a developer prefix mid-smoke.
# Use NYTPROF_PREFIX (not bare PREFIX): MakeMaker rewrites exported PREFIX
# in recipe environments to its install base (e.g. ~/perl5 via local::lib).
DEPTH_PREFIX="$ROOT/prefix-build003-depth-smoke"
export NYTPROF_PREFIX="$DEPTH_PREFIX"
unset PREFIX 2>/dev/null || true

cleanup() {
  if [[ "${SMOKE_OWNED_MAKEFILE}" -eq 1 ]]; then
    rm -f "$ROOT/Makefile" "$ROOT/Makefile.old" \
      "$ROOT/MYMETA.json" "$ROOT/MYMETA.yml" \
      "$ROOT/nytprof-packaging.mode" \
      "$ROOT/pm_to_blib" 2>/dev/null || true
    rm -rf "$ROOT/blib" 2>/dev/null || true
  fi
  rm -rf "$DEPTH_PREFIX" 2>/dev/null || true
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# 1. Configure legacy-only (must not require cargo)
# ---------------------------------------------------------------------------
banner "perl Makefile.PL (NYTPROF_NATIVE=0; no cargo required)"
export NYTPROF_NATIVE=0
if ! perl Makefile.PL; then
  fail "perl Makefile.PL failed under NYTPROF_NATIVE=0"
fi
SMOKE_OWNED_MAKEFILE=1

[[ -f "$ROOT/nytprof-packaging.mode" ]] || fail "missing nytprof-packaging.mode"
grep -q 'native_mode=off' "$ROOT/nytprof-packaging.mode" \
  || fail "expected native_mode=off for NYTPROF_NATIVE=0"
grep -q 'not_full_xs_cpan=1' "$ROOT/nytprof-packaging.mode" \
  || fail "expected not_full_xs_cpan=1 honesty stamp"
grep -q 'packaging_depth=BUILD-003-depth-v0' "$ROOT/nytprof-packaging.mode" \
  || fail "expected packaging_depth=BUILD-003-depth-v0 stamp"
grep -q 'full_build003=0' "$ROOT/nytprof-packaging.mode" \
  || fail "expected full_build003=0 (must not claim BUILD-003 complete)"
ok "legacy configure stamps OK (depth v0; not full BUILD-003)"

# Sanity: depth targets exposed
for t in install-facade dual-install cargo-build packaging-status; do
  if ! grep -E -q "^${t}:|^\\.PHONY:.*${t}" "$ROOT/Makefile"; then
    fail "Makefile missing depth target: $t"
  fi
done
ok "Makefile exposes install-facade dual-install cargo-build packaging-status"

banner "make packaging-status"
if ! make packaging-status; then
  fail "make packaging-status failed"
fi
ok "make packaging-status"

# ---------------------------------------------------------------------------
# 2. Facade install without cargo (legacy-only unbroken)
# ---------------------------------------------------------------------------
banner "make install-facade (no cargo on critical path)"
if ! make install-facade; then
  fail "make install-facade failed under legacy configure"
fi
[[ -x "$DEPTH_PREFIX/bin/nytprof-engine" ]] \
  || fail "missing installed engine at $DEPTH_PREFIX/bin/nytprof-engine"
[[ -f "$DEPTH_PREFIX/lib/Devel/NYTProf/EngineDispatch.pm" ]] \
  || fail "missing installed EngineDispatch.pm under prefix/lib"
[[ -f "$DEPTH_PREFIX/nytprof-facade.install" ]] \
  || fail "missing facade install stamp"
grep -q 'not_full_xs_cpan=1' "$DEPTH_PREFIX/nytprof-facade.install" \
  || fail "facade stamp missing not_full_xs_cpan=1"
ok "install-facade populated private prefix (no cargo)"

# Pure-Perl query --jsonl via installed facade (no native CLI required).
banner "installed facade: query --json --jsonl (pure-Perl; no cargo)"
ENGINE="$DEPTH_PREFIX/bin/nytprof-engine"
QUERY_OUT="$(mktemp)"
trap 'rm -f "$QUERY_OUT"; cleanup' EXIT
# NYTPROF_MOD_ROOT so find_repo_root succeeds even from a private prefix layout.
export NYTPROF_MOD_ROOT="$ROOT"
if ! perl -I"$DEPTH_PREFIX/lib" "$ENGINE" query --json --jsonl "$FIXTURE_JSONL" \
  >"$QUERY_OUT" 2>/tmp/build003_depth_query.err; then
  cat /tmp/build003_depth_query.err >&2 || true
  fail "installed facade query --json --jsonl failed"
fi
grep -q '"leaf_returns"[[:space:]]*:[[:space:]]*15' "$QUERY_OUT" \
  || fail "query JSON missing leaf_returns 15:\n$(cat "$QUERY_OUT")"
grep -q '"mid_returns"[[:space:]]*:[[:space:]]*3' "$QUERY_OUT" \
  || fail "query JSON missing mid_returns 3:\n$(cat "$QUERY_OUT")"
ok "installed facade pure-Perl query → leaf=15 mid=3"

# cargo-build must fail closed without cargo (or when we force PATH without cargo).
# When cargo is present we exercise the positive path below; still assert the
# target exists and fails honestly if cargo is missing from PATH for a probe.
if ! command -v cargo >/dev/null 2>&1; then
  banner "make cargo-build must fail without cargo"
  set +e
  out="$(make cargo-build 2>&1)"
  rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then
    fail "make cargo-build succeeded without cargo on PATH"
  fi
  echo "$out" | grep -qi 'cargo' || fail "cargo-build error should mention cargo"
  ok "make cargo-build refuses without cargo"
fi

# ---------------------------------------------------------------------------
# 3. Optional native half when cargo present
# ---------------------------------------------------------------------------
if command -v cargo >/dev/null 2>&1; then
  ok "cargo present: $(cargo --version 2>/dev/null || echo unknown)"

  banner "make dual-install (native CLI + facade)"
  if ! make dual-install; then
    fail "make dual-install failed with cargo present"
  fi
  [[ -x "$DEPTH_PREFIX/bin/nytprof-cli" || -x "$DEPTH_PREFIX/bin/nytprof-dump" ]] \
    || fail "dual-install did not install native CLI under $DEPTH_PREFIX/bin"
  [[ -x "$DEPTH_PREFIX/bin/nytprof-engine" ]] \
    || fail "dual-install missing nytprof-engine"
  ok "dual-install → native CLI + facade under private prefix"

  banner "installed facade: --engine=native report via prefix CLI"
  # Prefer prefix binary; do not put crates/ on PERL5LIB.
  if [[ -x "$DEPTH_PREFIX/bin/nytprof-cli" ]]; then
    export NYTPROF_NATIVE_CLI="$DEPTH_PREFIX/bin/nytprof-cli"
  else
    export NYTPROF_NATIVE_CLI="$DEPTH_PREFIX/bin/nytprof-dump"
  fi
  REPORT_OUT="$(mktemp)"
  trap 'rm -f "$QUERY_OUT" "$REPORT_OUT"; cleanup' EXIT
  if ! perl -I"$DEPTH_PREFIX/lib" "$ENGINE" --engine=native report "$FIXTURE_OUT" \
    >"$REPORT_OUT" 2>/tmp/build003_depth_report.err; then
    cat /tmp/build003_depth_report.err >&2 || true
    fail "installed facade --engine=native report failed"
  fi
  grep -q 'main::leaf' "$REPORT_OUT" || fail "report missing main::leaf"
  grep -q 'returns=15' "$REPORT_OUT" || fail "report missing returns=15"
  ok "installed facade + prefix CLI report: main::leaf returns=15"

  banner "NYTPROF_NATIVE=auto configure depth stamp"
  if ! NYTPROF_NATIVE=auto perl Makefile.PL >/dev/null; then
    fail "NYTPROF_NATIVE=auto perl Makefile.PL failed"
  fi
  grep -q 'packaging_depth=BUILD-003-depth-v0' "$ROOT/nytprof-packaging.mode" \
    || fail "auto configure missing packaging_depth stamp"
  grep -q 'full_build003=0' "$ROOT/nytprof-packaging.mode" \
    || fail "auto configure must keep full_build003=0"
  ok "auto configure retains BUILD-003-depth honesty stamps"

  # Restore legacy stamp for cleanup cleanliness.
  NYTPROF_NATIVE=0 perl Makefile.PL >/dev/null
else
  banner "optional-native"
  echo "SKIP: cargo not on PATH — dual-install / native report half not exercised"
  echo "  (legacy install-facade + pure-Perl query succeeded; valid depth outcome)"
fi

banner "ALL PASSED"
ok "makemaker_build003_depth_smoke completed (toward BUILD-003; not full dual-build)"
exit 0

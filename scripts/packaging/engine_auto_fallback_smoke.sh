#!/usr/bin/env bash
# ENGINE-AUTO-FALLBACK: true engine=auto prefer-native / fall-back-legacy on
# the shipped Perl facade (nytprof-engine + EngineDispatch).
#
# Spec: docs/schemas/engine-selection-mvp-v0.md
#       docs/schemas/perl-engine-dispatch-mvp-v0.md
#
# Cases:
#   1. Native present:  --engine=auto report ×2 → main::leaf returns=15, mid=3
#   2. Native missing:  NYTPROF_FORCE_NO_NATIVE=1 + auto report/verify → exit 0
#                       via legacy stream-dump smoke; no crates/ on PERL5LIB;
#                       no false "native" success claim; STDERR fallback note OK
#   3. Explicit native + force-no-native → must fail (not fall back)
#
# Test hook (documented; packaging only):
#   NYTPROF_FORCE_NO_NATIVE=1  → find_native_cli fails immediately
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/engine_auto_fallback_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE="fixtures/v5/default-calls1/nytprof.out"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"
DISPATCH_PM="$ENGINE_LIB/Devel/NYTProf/EngineDispatch.pm"
ORACLE_PERL5LIB="$ROOT/baseline/6.15/oracle-perl5lib.txt"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$FIXTURE" ]] || fail "missing fixture $FIXTURE"
[[ -f "$ROOT/$ENGINE_BIN" ]] || fail "missing $ENGINE_BIN"
[[ -f "$ROOT/$DISPATCH_PM" ]] || fail "missing $DISPATCH_PM"

# ---------------------------------------------------------------------------
# Native must be discoverable for the "native present" half
# ---------------------------------------------------------------------------
find_cli() {
  if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
    echo "path:${NYTPROF_NATIVE_CLI}"
    return 0
  fi
  for p in \
    prefix/bin/nytprof-cli \
    prefix/bin/nytprof-dump \
    target/release/nytprof-dump \
    target/debug/nytprof-dump
  do
    if [[ -x "$ROOT/$p" || -f "$ROOT/$p" ]]; then
      echo "path:$ROOT/$p"
      return 0
    fi
  done
  if command -v cargo >/dev/null 2>&1; then
    echo "cargo"
    return 0
  fi
  return 1
}

if ! CLI_SPEC="$(find_cli)"; then
  fail "native CLI not discoverable (set NYTPROF_NATIVE_CLI, install prefix/bin, build target/*/nytprof-dump, or provide cargo). ENGINE-AUTO-FALLBACK needs a real native half for the present-path checks."
fi
ok "native discoverable ($CLI_SPEC)"

if [[ "$CLI_SPEC" == "cargo" ]]; then
  cargo build -q -p nytprof-cli
  ok "cargo build -p nytprof-cli"
fi

# Ensure FORCE_NO_NATIVE is unset for the native-present half.
unset NYTPROF_FORCE_NO_NATIVE || true

ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

assert_leaf_mid() {
  local label="$1"
  local out="$2"
  grep -qE 'main::leaf[[:space:]]+returns=15\b' "$out" \
    || fail "$label missing main::leaf returns=15:\n$(cat "$out")"
  grep -qE 'main::mid[[:space:]]+returns=3\b' "$out" \
    || fail "$label missing main::mid returns=3:\n$(cat "$out")"
  ok "$label: main::leaf returns=15 and main::mid returns=3"
}

# ---------------------------------------------------------------------------
# 1. Native present: --engine=auto report (twice) → real native leaf/mid
# ---------------------------------------------------------------------------
echo "=== auto (native present) report pass 1 ==="
OUT1="$TMPDIR_SMOKE/auto_native_1.out"
ERR1="$TMPDIR_SMOKE/auto_native_1.err"
if ! "${ENGINE[@]}" --engine=auto report "$FIXTURE" >"$OUT1" 2>"$ERR1"; then
  cat "$ERR1" >&2 || true
  cat "$OUT1" >&2 || true
  fail "--engine=auto report failed (native present, pass 1)"
fi
# Must not claim fallback when native is present.
if grep -qiE 'using legacy|native CLI not found' "$ERR1" 2>/dev/null; then
  fail "auto with native present unexpectedly fell back to legacy:\n$(cat "$ERR1")"
fi
cat "$OUT1"
assert_leaf_mid "auto report (native present, pass 1)" "$OUT1"

echo "=== auto (native present) report pass 2 ==="
OUT2="$TMPDIR_SMOKE/auto_native_2.out"
ERR2="$TMPDIR_SMOKE/auto_native_2.err"
if ! "${ENGINE[@]}" --engine=auto report "$FIXTURE" >"$OUT2" 2>"$ERR2"; then
  cat "$ERR2" >&2 || true
  cat "$OUT2" >&2 || true
  fail "--engine=auto report failed (native present, pass 2)"
fi
assert_leaf_mid "auto report (native present, pass 2)" "$OUT2"

# ---------------------------------------------------------------------------
# 2. Native missing (FORCE_NO_NATIVE): auto → legacy stream-dump, exit 0
# ---------------------------------------------------------------------------
[[ -f "$ORACLE_PERL5LIB" ]] \
  || fail "oracle pin missing ($ORACLE_PERL5LIB); needed for auto→legacy fallback half"

echo "=== auto (native hidden via NYTPROF_FORCE_NO_NATIVE=1) report ==="
OUT_FB="$TMPDIR_SMOKE/auto_fallback_report.out"
ERR_FB="$TMPDIR_SMOKE/auto_fallback_report.err"
if ! env NYTPROF_FORCE_NO_NATIVE=1 \
  "${ENGINE[@]}" --engine=auto report "$FIXTURE" >"$OUT_FB" 2>"$ERR_FB"; then
  cat "$ERR_FB" >&2 || true
  cat "$OUT_FB" >&2 || true
  fail "--engine=auto report should succeed via legacy when native hidden (rc≠0)"
fi

# Fallback note on stderr
if ! grep -qiE 'auto:.*native CLI not found|using legacy' "$ERR_FB"; then
  echo "NOTE: expected fallback note on stderr; got:" >&2
  cat "$ERR_FB" >&2 || true
  fail "auto fallback missing STDERR note (native CLI not found / using legacy)"
fi
ok "auto fallback STDERR note present"

# Must not look like a successful native text report with leaf/mid counts.
# Legacy stream-dump smoke prints dump/OK lines, not native "returns=N" summary.
if grep -qE 'main::leaf[[:space:]]+returns=15\b' "$OUT_FB"; then
  # If leaf lines appear, they must not be paired with a claim that we ran native
  # without fallback — but native report format under legacy is unexpected.
  # Accept only if clearly legacy-labeled; otherwise this is a false native success.
  if ! grep -qiE 'legacy|dump_readstream|stream.dump|oracle|JSONL' \
    "$OUT_FB" "$ERR_FB" 2>/dev/null; then
    fail "auto fallback output looks like native returns=15 without legacy markers"
  fi
fi

# Positive legacy markers (stream dump / OK / NOTE)
if ! grep -qiE 'OK:|legacy|dump_readstream|stream.dump|JSONL|NOTE:' \
  "$OUT_FB" "$ERR_FB" 2>/dev/null; then
  echo "NOTE: fallback stdout/stderr:" >&2
  cat "$OUT_FB" >&2 || true
  cat "$ERR_FB" >&2 || true
  fail "auto fallback report missing legacy success markers"
fi
ok "auto fallback report exit 0 via legacy stream-dump"

echo "=== auto (native hidden) verify ==="
OUT_V="$TMPDIR_SMOKE/auto_fallback_verify.out"
ERR_V="$TMPDIR_SMOKE/auto_fallback_verify.err"
if ! env NYTPROF_FORCE_NO_NATIVE=1 \
  "${ENGINE[@]}" --engine=auto verify "$FIXTURE" >"$OUT_V" 2>"$ERR_V"; then
  cat "$ERR_V" >&2 || true
  cat "$OUT_V" >&2 || true
  fail "--engine=auto verify should succeed via legacy when native hidden"
fi
if ! grep -qiE 'auto:.*native CLI not found|using legacy' "$ERR_V"; then
  fail "auto verify fallback missing STDERR note"
fi
if ! grep -qiE 'OK:|legacy|verify|dump' "$OUT_V" "$ERR_V" 2>/dev/null; then
  cat "$OUT_V" >&2 || true
  cat "$ERR_V" >&2 || true
  fail "auto fallback verify missing OK/legacy markers"
fi
ok "auto fallback verify exit 0 via legacy"

# ---------------------------------------------------------------------------
# 3. Explicit --engine=native must NOT fall back when native is hidden
# ---------------------------------------------------------------------------
echo "=== native explicit + FORCE_NO_NATIVE must fail ==="
set +e
env NYTPROF_FORCE_NO_NATIVE=1 \
  "${ENGINE[@]}" --engine=native report "$FIXTURE" \
  >"$TMPDIR_SMOKE/native_force.out" 2>"$TMPDIR_SMOKE/native_force.err"
NATIVE_FORCE_RC=$?
set -e
[[ "$NATIVE_FORCE_RC" -ne 0 ]] \
  || fail "--engine=native with NYTPROF_FORCE_NO_NATIVE=1 unexpectedly succeeded"
if ! grep -qiE 'native CLI not found|FORCE_NO_NATIVE' \
  "$TMPDIR_SMOKE/native_force.err" "$TMPDIR_SMOKE/native_force.out" 2>/dev/null; then
  cat "$TMPDIR_SMOKE/native_force.err" >&2 || true
  fail "native+FORCE_NO_NATIVE error should mention native CLI / FORCE_NO_NATIVE"
fi
ok "--engine=native + FORCE_NO_NATIVE fails closed (rc=$NATIVE_FORCE_RC)"

# ---------------------------------------------------------------------------
# Isolation: never crates/ on PERL5LIB in this smoke process
# ---------------------------------------------------------------------------
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi
ok "parent PERL5LIB has no /crates/"

ok "engine-auto-fallback packaging smoke passed"
exit 0

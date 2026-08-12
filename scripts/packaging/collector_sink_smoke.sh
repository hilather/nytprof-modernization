#!/usr/bin/env bash
# COL-001 / PR-B02 — collector semantic sink scaffold smoke.
#
# When a C toolchain is present: build + unit-test the overlay sink.
# When absent: honest skip (offline_gate remains green).
# Always: isolation asserts — collector/ must never appear as oracle PERL5LIB.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/collector_sink_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

COLLECTOR="$ROOT/collector"
MAKEFILE="$COLLECTOR/Makefile"
BASELINE="$ROOT/baseline/6.15"
PERL5LIB_FILE="$BASELINE/oracle-perl5lib.txt"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
banner() { printf '\n=== %s ===\n' "$*"; }

banner "collector_sink_smoke (COL-001 semantic sink scaffold)"

# ---------------------------------------------------------------------------
# Tree present (this smoke is only meaningful after PR-B02 lands sources)
# ---------------------------------------------------------------------------
[[ -d "$COLLECTOR" ]] || fail "missing collector/ overlay (ADR-0004 / COL-001)"
[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$COLLECTOR/include/nytp_sink.h" ]] || fail "missing nytp_sink.h"
[[ -f "$COLLECTOR/src/nytp_sink_v5.c" ]] || fail "missing stub v5 adapter"
ok "collector/ overlay tree present (B0-A)"

# ---------------------------------------------------------------------------
# Isolation: never put collector/ (or crates/) on oracle PERL5LIB
# ---------------------------------------------------------------------------
assert_no_bad_perl5lib() {
  local label="$1"
  local p5="${2-}"
  case ":${p5}:" in
    *"/crates/"*)
      fail "$label PERL5LIB must not contain /crates/: $p5"
      ;;
  esac
  # Path-component asserts for collector/ (ADR-0004 §3 / COL-001).
  if [[ -n "$p5" ]]; then
    local IFS=':'
    local part
    for part in $p5; do
      [[ -n "$part" ]] || continue
      case "$part" in
        *"/collector"|*"/collector/"*|*/collector/install*|*/prefix/collector*)
          fail "$label PERL5LIB must not contain collector overlay path: $part"
          ;;
      esac
      # Also reject bare component named collector anywhere in the path.
      if [[ "$part" == *"/collector/"* ]] || [[ "$part" == *"/collector" ]]; then
        fail "$label PERL5LIB contains collector path component: $part"
      fi
    done
  fi
  ok "$label: no crates/ or collector/ on PERL5LIB"
}

# Parent process env (gate parent must not leak collector onto PERL5LIB).
assert_no_bad_perl5lib "parent env" "${PERL5LIB-}"

# Oracle pin file, if present, must also be clean.
if [[ -f "$PERL5LIB_FILE" ]]; then
  oracle_p5="$(tr -d '\r' <"$PERL5LIB_FILE" | head -1 || true)"
  assert_no_bad_perl5lib "oracle-perl5lib.txt" "$oracle_p5"
else
  ok "oracle-perl5lib.txt absent (oracle not built here; isolation still checked on parent env)"
fi

# Refuse if someone exported a collector install into PERL5LIB for this smoke.
case ":${PERL5LIB-}:" in
  *"${COLLECTOR}"*)
    fail "PERL5LIB must not include collector tree path: $PERL5LIB"
    ;;
esac

# ---------------------------------------------------------------------------
# Honest skip if no C toolchain
# ---------------------------------------------------------------------------
resolve_cc() {
  if [[ -n "${CC-}" ]] && command -v "$CC" >/dev/null 2>&1; then
    printf '%s\n' "$CC"
    return 0
  fi
  for c in cc gcc clang; do
    if command -v "$c" >/dev/null 2>&1; then
      printf '%s\n' "$c"
      return 0
    fi
  done
  return 1
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — collector sink unit tests not run"
  echo "  (honest skip; legacy-only / dual-path half still independent of collector/)"
  echo "  To exercise: install a C compiler and re-run this smoke"
  ok "collector_sink_smoke completed (skip — no CC)"
  exit 0
fi
ok "C toolchain: $CC_BIN"

# ---------------------------------------------------------------------------
# Build + unit test (real entry: collector/Makefile + test_sink_api)
# ---------------------------------------------------------------------------
banner "make -C collector clean test"
make -C "$COLLECTOR" clean
make -C "$COLLECTOR" test CC="$CC_BIN"

[[ -x "$COLLECTOR/build/test_sink_api" ]] || fail "test binary missing after make test"
# Re-run the shipped binary once more (entry point, not a reimplementation).
"$COLLECTOR/build/test_sink_api"
ok "collector unit tests (counting + stub v5 sink)"

# Residual honesty banner (do not claim wire or COL-007).
echo "NOTE: stub v5 adapter does not encode wire bytes (COL-006); COL-007 not implemented"
echo "NOTE: fake-clock is PR-B03 / TEST-003 — not this smoke"

banner "collector_sink_smoke PASSED"
ok "COL-001 sink scaffold build + isolation"
exit 0

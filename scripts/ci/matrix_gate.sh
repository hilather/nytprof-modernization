#!/usr/bin/env bash
# BUILD-006 MVP — multi-OS CI matrix entry (single host).
#
# Runs the existing offline R1 gate on *this* runner after ensuring a
# host-local oracle pin (PERL5LIB paths are absolute and machine-specific).
# Honest skips inside offline_gate / dual_path are preserved unchanged.
#
# This is NOT full BUILD-006 certification:
#   - not multi-Perl / multi-rustc version matrix
#   - not Windows / full platform tier policy freeze
#   - not a coverage dashboard or release compatibility matrix (TEST-020)
#   - not multi-OS prebuilt binary distribution
#
# Typical matrix (GitHub Actions): ubuntu-latest (linux-x86_64) + macos-latest (macos-arm64).
# Local:
#   ./scripts/ci/matrix_gate.sh
# Optional env:
#   NYTPROF_MATRIX_SKIP_ORACLE_BUILD=1  — do not fetch/build oracle (fail if pin missing)
#   NYTPROF_MATRIX_LABEL=...            — free-form label for CI logs (default: uname)
#
# Policy: docs/BUILD_SUPPORT_POLICY.md
# Board:  BUILD-006-MVP
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

OFFLINE_GATE="$ROOT/scripts/ci/offline_gate.sh"
FETCH_ORACLE="$ROOT/scripts/baseline/fetch_oracle.sh"
BUILD_ORACLE="$ROOT/scripts/baseline/build_oracle.sh"
BASELINE="$ROOT/baseline/6.15"
INSTALL_DIR="$BASELINE/install"
PERL5LIB_FILE="$BASELINE/oracle-perl5lib.txt"
SRC_DIR="$BASELINE/src"

banner() {
  echo
  echo "================================================================"
  echo " MATRIX GATE (BUILD-006 MVP): $*"
  echo "================================================================"
}

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
note() { printf 'NOTE: %s\n' "$*"; }

# ---------------------------------------------------------------------------
# Platform banner (explicit matrix row identity)
# ---------------------------------------------------------------------------
UNAME_S="$(uname -s 2>/dev/null || echo unknown)"
UNAME_M="$(uname -m 2>/dev/null || echo unknown)"
UNAME_A="$(uname -a 2>/dev/null || echo unknown)"
LABEL="${NYTPROF_MATRIX_LABEL:-${UNAME_S}-${UNAME_M}}"

banner "host identity"
echo "matrix_gate: label=$LABEL"
echo "matrix_gate: uname=$UNAME_A"
echo "matrix_gate: repo root $ROOT"
echo "matrix_gate: offline gate $OFFLINE_GATE"
echo "matrix_gate: BUILD-006 MVP only — not full multi-OS / multi-Perl / multi-rustc certification"
if command -v perl >/dev/null 2>&1; then
  echo "matrix_gate: perl=$(perl -e 'print $^V' 2>/dev/null || perl -v 2>/dev/null | head -2 | tr '\n' ' ')"
else
  note "perl not on PATH — offline_gate pure-Perl steps will fail (not an honest skip)"
fi
if command -v cargo >/dev/null 2>&1; then
  echo "matrix_gate: cargo=$(cargo --version 2>/dev/null || echo unknown)"
else
  note "cargo not on PATH — offline_gate will honest-skip cargo/native halves"
fi
if command -v python3 >/dev/null 2>&1; then
  echo "matrix_gate: python3=$(python3 --version 2>/dev/null || echo unknown)"
else
  note "python3 not on PATH — harness normalize path may fail"
fi

[[ -f "$OFFLINE_GATE" ]] || fail "required script missing: $OFFLINE_GATE"

# ---------------------------------------------------------------------------
# Host-local oracle pin (absolute PERL5LIB paths; rebuild when needed)
# ---------------------------------------------------------------------------
banner "oracle pin (host-local)"
need_oracle=0
if [[ ! -f "$PERL5LIB_FILE" ]]; then
  need_oracle=1
fi
if [[ ! -d "$INSTALL_DIR" ]] \
  || [[ -z "$(find "$INSTALL_DIR" -path '*/Devel/NYTProf.pm' 2>/dev/null | head -1)" ]]; then
  need_oracle=1
fi

# Tracked oracle-perl5lib.txt may point at another machine's absolute paths.
# If the file exists but none of its entries resolve, force rebuild.
if [[ "$need_oracle" -eq 0 && -f "$PERL5LIB_FILE" ]]; then
  resolved=0
  while IFS= read -r entry; do
    [[ -z "$entry" ]] && continue
    if [[ -d "$entry" ]] || [[ -f "$entry/Devel/NYTProf.pm" ]] || [[ -f "$entry" ]]; then
      resolved=1
      break
    fi
  done < <(tr ':' '\n' <"$PERL5LIB_FILE")
  if [[ "$resolved" -eq 0 ]]; then
    note "oracle-perl5lib.txt entries not present on this host — will rebuild oracle pin"
    need_oracle=1
  fi
fi

if [[ "$need_oracle" -eq 1 ]]; then
  if [[ "${NYTPROF_MATRIX_SKIP_ORACLE_BUILD:-0}" == "1" ]]; then
    fail "oracle pin incomplete and NYTPROF_MATRIX_SKIP_ORACLE_BUILD=1
  Build on this host: ./scripts/baseline/fetch_oracle.sh && ./scripts/baseline/build_oracle.sh
  Or:  ./scripts/baseline/run_all.sh"
  fi
  [[ -f "$FETCH_ORACLE" ]] || fail "missing $FETCH_ORACLE"
  [[ -f "$BUILD_ORACLE" ]] || fail "missing $BUILD_ORACLE"
  if [[ ! -f "$SRC_DIR/Makefile.PL" ]]; then
    echo "matrix_gate: fetching oracle sources..."
    bash "$FETCH_ORACLE"
  fi
  echo "matrix_gate: building host-local oracle into baseline/6.15/install ..."
  bash "$BUILD_ORACLE"
  ok "oracle rebuilt for this host"
else
  ok "oracle pin present and paths resolve on this host"
fi

[[ -f "$PERL5LIB_FILE" ]] || fail "missing $PERL5LIB_FILE after ensure step"
[[ -d "$INSTALL_DIR" ]] || fail "missing install tree $INSTALL_DIR"

# ---------------------------------------------------------------------------
# Offline R1 gate (honest skips preserved inside offline_gate.sh)
# ---------------------------------------------------------------------------
banner "offline_gate (CI-OFFLINE-GATE; honest skips preserved)"
bash "$OFFLINE_GATE"
ok "offline_gate completed on matrix row label=$LABEL"

banner "ALL PASSED"
ok "matrix_gate completed (BUILD-006 MVP) on $LABEL"
echo "NOTE: full BUILD-006 (multi-Perl, multi-rustc, Windows, coverage dashboard) remains residual"
exit 0

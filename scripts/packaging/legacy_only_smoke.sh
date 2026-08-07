#!/usr/bin/env bash
# Legacy-only packaging smoke: prove oracle isolation without Cargo.
#
# AC1 / AC3: succeeds on a machine with a built oracle and never requires
# cargo, rustc, or crates/ on PERL5LIB.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/legacy_only_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

BASELINE="$ROOT/baseline/6.15"
INSTALL_DIR="$BASELINE/install"
PERL5LIB_FILE="$BASELINE/oracle-perl5lib.txt"
MODULE_PATH_FILE="$BASELINE/oracle-module-path.txt"
SRC_DIR="$BASELINE/src"
BUILD_ORACLE="$ROOT/scripts/baseline/build_oracle.sh"
ENV_SH="$ROOT/tools/oracle/env.sh"
DUMP_PL="$ROOT/tools/oracle/dump_readstream.pl"
FIXTURE_OUT="$ROOT/fixtures/v5/default-calls1/nytprof.out"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------------------
# Never invoke Cargo / rustc (even if present on PATH)
# ---------------------------------------------------------------------------
if command -v cargo >/dev/null 2>&1; then
  ok "cargo is present on PATH but this smoke will not invoke it"
else
  ok "cargo is absent (expected for pure legacy-only installs)"
fi
if command -v rustc >/dev/null 2>&1; then
  ok "rustc is present on PATH but this smoke will not invoke it"
fi

# ---------------------------------------------------------------------------
# Ensure oracle pin / install tree exists (rebuild only if sources present)
# ---------------------------------------------------------------------------
need_oracle=0
if [[ ! -f "$PERL5LIB_FILE" ]]; then
  need_oracle=1
fi
if [[ ! -d "$INSTALL_DIR" ]] || [[ -z "$(find "$INSTALL_DIR" -path '*/Devel/NYTProf.pm' 2>/dev/null | head -1)" ]]; then
  need_oracle=1
fi

if [[ "$need_oracle" -eq 1 ]]; then
  if [[ -f "$SRC_DIR/Makefile.PL" ]]; then
    echo "Oracle pin incomplete; attempting rebuild via scripts/baseline/build_oracle.sh ..."
    # shellcheck disable=SC1090
    bash "$BUILD_ORACLE"
  else
    fail "Oracle not built and sources missing under baseline/6.15/src.
  Run: ./scripts/baseline/fetch_oracle.sh && ./scripts/baseline/build_oracle.sh
  Or:  ./scripts/baseline/run_all.sh
  (No Cargo required.)"
  fi
fi

[[ -f "$PERL5LIB_FILE" ]] || fail "missing $PERL5LIB_FILE after ensure step"
[[ -d "$INSTALL_DIR" ]] || fail "missing install tree $INSTALL_DIR"
ok "oracle pin present ($PERL5LIB_FILE + install tree)"

# ---------------------------------------------------------------------------
# Source shared oracle isolation (sets PERL5LIB, PATH, module path checks)
# ---------------------------------------------------------------------------
# shellcheck source=../../tools/oracle/env.sh
source "$ENV_SH"
ok "sourced tools/oracle/env.sh (oracle PERL5LIB isolation)"

# ---------------------------------------------------------------------------
# PERL5LIB must not contain crates/ (or candidate perl facade)
# ---------------------------------------------------------------------------
case ":${PERL5LIB-}:" in
  *"/crates/"*)
    fail "PERL5LIB must not contain /crates/: $PERL5LIB"
    ;;
esac
# Also scan individual entries for a crates path component
IFS=':' read -r -a _p5_entries <<<"${PERL5LIB-}"
for _e in "${_p5_entries[@]}"; do
  [[ -z "$_e" ]] && continue
  case "$_e" in
    *"/crates/"*|*"${ROOT}/crates"*|"$ROOT/crates"/*)
      fail "PERL5LIB entry points at crates/: $_e"
      ;;
    *"/perl/lib"*|"$ROOT/perl"/*)
      fail "PERL5LIB entry points at candidate perl/: $_e"
      ;;
  esac
done
ok "PERL5LIB has no /crates/ (or candidate perl/) entries"

# ---------------------------------------------------------------------------
# Devel::NYTProf must load from baseline/6.15/install
# ---------------------------------------------------------------------------
# Run load in a temp dir so any accidental profiler side-effects stay local.
SMOKE_TMP="$(mktemp -d)"
trap 'rm -rf "$SMOKE_TMP"' EXIT
export NYTPROF="file=${SMOKE_TMP}/nytprof.out:start=no"
# start=no may be ignored by some builds; temp dir still isolates output.

LOADED_PATH="$(
  cd "$SMOKE_TMP"
  perl -MDevel::NYTProf -e 'print $INC{"Devel/NYTProf.pm"}'
)"

[[ -n "$LOADED_PATH" ]] || fail "perl -MDevel::NYTProf did not report INC path"

case "$LOADED_PATH" in
  "$INSTALL_DIR"/*) ok "Devel::NYTProf loads from install tree: $LOADED_PATH" ;;
  *)
    fail "Devel::NYTProf loaded outside install tree: $LOADED_PATH
  expected under: $INSTALL_DIR"
    ;;
esac

case "$LOADED_PATH" in
  *"/crates/"*)
    fail "loaded module path contains /crates/: $LOADED_PATH"
    ;;
esac

if [[ -f "$MODULE_PATH_FILE" ]]; then
  RECORDED="$(cat "$MODULE_PATH_FILE")"
  if [[ -n "$RECORDED" && "$LOADED_PATH" != "$RECORDED" ]]; then
    # Non-fatal if still under install (relocations / multi-arch); still note it.
    echo "NOTE: live load path differs from oracle-module-path.txt (still under install)"
    echo "  recorded: $RECORDED"
    echo "  live:     $LOADED_PATH"
  fi
fi

# ---------------------------------------------------------------------------
# Optional fixture dump via oracle ReadStream (no Cargo)
# ---------------------------------------------------------------------------
if [[ -f "$DUMP_PL" && -f "$FIXTURE_OUT" ]]; then
  DUMP_OUT="$SMOKE_TMP/readstream.jsonl"
  # dump_readstream.pl uses ReadStream only (does not start the collector)
  if perl "$DUMP_PL" "$FIXTURE_OUT" >"$DUMP_OUT"; then
    if grep -q '"tag"' "$DUMP_OUT" && grep -q '_END' "$DUMP_OUT"; then
      LINE_COUNT="$(wc -l <"$DUMP_OUT" | tr -d ' ')"
      ok "oracle dump_readstream.pl on default-calls1 ($LINE_COUNT JSONL lines)"
    else
      fail "dump_readstream.pl output missing expected tags"
    fi
  else
    fail "dump_readstream.pl failed on $FIXTURE_OUT"
  fi
else
  echo "NOTE: skipping fixture dump (missing $DUMP_PL or $FIXTURE_OUT)"
fi

ok "legacy-only packaging smoke passed (no Cargo invoked)"
exit 0

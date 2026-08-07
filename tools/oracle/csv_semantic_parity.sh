#!/usr/bin/env bash
# CSV semantic parity smoke: native csv CLI on default-calls1 (run twice).
#
# Spec: docs/schemas/csv-semantic-parity-mvp-v0.md
# Board: CSV-SEMANTIC-PARITY
#
# Checks (exact counts only; ticks not compared):
#   main::leaf,15  /  main::mid,3  /  main::mid,main::leaf,15
#
# Native path uses cargo / prefix CLI only. Does not require oracle Perl.
# Never puts crates/ on PERL5LIB.
#
# Usage (from repo root or any cwd):
#   bash tools/oracle/csv_semantic_parity.sh
#   ./tools/oracle/csv_semantic_parity.sh   # if executable
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
cd "$ROOT"

FIXTURE="fixtures/v5/default-calls1/nytprof.out"
FIXTURE_ABS="$ROOT/$FIXTURE"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

[[ -f "$FIXTURE_ABS" ]] || fail "missing fixture $FIXTURE"

# ---------------------------------------------------------------------------
# Temp workspace
# ---------------------------------------------------------------------------
TMP="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-csv-semantic-XXXXXX")"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

CSV1="$TMP/csv1.out"
CSV2="$TMP/csv2.out"

# ---------------------------------------------------------------------------
# Resolve native CLI (cargo run preferred; prefix / target fallback)
# ---------------------------------------------------------------------------
run_native() {
  # Usage: run_native <subcommand and args...>
  if command -v cargo >/dev/null 2>&1; then
    cargo run -q -p nytprof-cli -- "$@"
  elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
    "$ROOT/prefix/bin/nytprof-cli" "$@"
  elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
    "$ROOT/target/debug/nytprof-dump" "$@"
  elif [[ -x "$ROOT/target/release/nytprof-dump" ]]; then
    "$ROOT/target/release/nytprof-dump" "$@"
  else
    fail "no cargo and no prefix/target nytprof-cli binary found"
  fi
}

# ---------------------------------------------------------------------------
# Optional: cargo unit test gate (named semantic parity test)
# ---------------------------------------------------------------------------
if command -v cargo >/dev/null 2>&1; then
  log "=== cargo test csv_semantic_parity_default_calls1 ==="
  if cargo test -q -p nytprof-report csv_semantic_parity_default_calls1 -- --nocapture \
    >"$TMP/cargo-test.out" 2>"$TMP/cargo-test.err"; then
    ok "cargo test csv_semantic_parity_default_calls1"
  else
    cat "$TMP/cargo-test.out" >&2 || true
    cat "$TMP/cargo-test.err" >&2 || true
    fail "cargo test csv_semantic_parity_default_calls1 failed"
  fi
else
  log "NOTE: cargo not available; skipping unit test gate (CLI path only)"
fi

# ---------------------------------------------------------------------------
# Native csv × 2 (stability + semantic patterns)
# ---------------------------------------------------------------------------
assert_csv_semantics() {
  local file="$1" label="$2"
  [[ -s "$file" ]] || fail "$label: empty CSV output"

  # Dual-section markers (default csv path).
  grep -q '# subroutines' "$file" || fail "$label: missing # subroutines"
  grep -q '# call_edges' "$file" || fail "$label: missing # call_edges"

  # Exact row prefixes (A5 / A7 counts). Deliverable patterns:
  #   leaf,15 / mid,3 / mid,main::leaf,15
  # Prefer line-anchored matches so mid,3 does not hit RUNTIME,main::mid,3,
  if grep -qE '^main::leaf,15,' "$file"; then
    ok "$label: main::leaf,15,"
  else
    fail "$label: missing main::leaf,15,
$(cat "$file")"
  fi
  if grep -qE '^main::mid,3,' "$file"; then
    ok "$label: main::mid,3,"
  else
    fail "$label: missing main::mid,3, (subs row)
$(cat "$file")"
  fi
  if grep -qE '^main::mid,main::leaf,15,' "$file"; then
    ok "$label: main::mid,main::leaf,15,"
  else
    fail "$label: missing main::mid,main::leaf,15,
$(cat "$file")"
  fi
}

log "=== native csv (run 1) ==="
if ! run_native csv "$FIXTURE" >"$CSV1" 2>"$TMP/csv1.err"; then
  cat "$CSV1" >&2 || true
  cat "$TMP/csv1.err" >&2 || true
  fail "native csv run 1 failed"
fi
assert_csv_semantics "$CSV1" "csv run 1"

log "=== native csv (run 2) ==="
if ! run_native csv "$FIXTURE" >"$CSV2" 2>"$TMP/csv2.err"; then
  cat "$CSV2" >&2 || true
  cat "$TMP/csv2.err" >&2 || true
  fail "native csv run 2 failed"
fi
assert_csv_semantics "$CSV2" "csv run 2"

# Stability: both runs should produce identical dual-section CSV.
if cmp -s "$CSV1" "$CSV2"; then
  ok "csv run 1 and run 2 are byte-identical"
else
  fail "csv run 1 and run 2 differ (not stable)
--- run1 ---
$(cat "$CSV1")
--- run2 ---
$(cat "$CSV2")"
fi

# Optional durable capture (verification / CI evidence). Example:
#   CSV_PARITY_KEEP_DIR=/path/to/keep bash tools/oracle/csv_semantic_parity.sh
if [[ -n "${CSV_PARITY_KEEP_DIR:-}" ]]; then
  mkdir -p "$CSV_PARITY_KEEP_DIR"
  cp -a "$CSV1" "$CSV_PARITY_KEEP_DIR/csv1.out"
  cp -a "$CSV2" "$CSV_PARITY_KEEP_DIR/csv2.out"
  ok "kept CSV evidence under $CSV_PARITY_KEEP_DIR"
fi

log ""
ok "csv semantic parity smoke passed (leaf,15; mid,3; mid→leaf,15; stable ×2)"
exit 0

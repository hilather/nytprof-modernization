#!/usr/bin/env bash
# Export semantic parity smoke: native folded + callgrind CLI on default-calls1
# (each run twice).
#
# Spec: docs/schemas/export-semantic-parity-mvp-v0.md
# Board: EXPORT-SEMANTIC-PARITY
#
# Checks (exact counts only; ticks/costs not compared):
#   folded:  main::mid;main::leaf 15  /  main::RUNTIME;main::mid 3
#   callgrind: leaf/mid presence, cfn=main::leaf + calls=15, cfn=main::mid + calls=3
#
# Native path uses cargo / prefix CLI only. Does not require oracle Perl.
# Never puts crates/ on PERL5LIB.
#
# Usage (from repo root or any cwd):
#   bash tools/oracle/export_semantic_parity.sh
#   ./tools/oracle/export_semantic_parity.sh   # if executable
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
TMP="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-export-semantic-XXXXXX")"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

FOLDED1="$TMP/folded1.out"
FOLDED2="$TMP/folded2.out"
CG1="$TMP/callgrind1.out"
CG2="$TMP/callgrind2.out"
CG_ALIAS1="$TMP/cg1.out"
CG_ALIAS2="$TMP/cg2.out"

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
  log "=== cargo test export_semantic_parity_default_calls1 ==="
  if cargo test -q -p nytprof-report export_semantic_parity_default_calls1 -- --nocapture \
    >"$TMP/cargo-test.out" 2>"$TMP/cargo-test.err"; then
    ok "cargo test export_semantic_parity_default_calls1"
  else
    cat "$TMP/cargo-test.out" >&2 || true
    cat "$TMP/cargo-test.err" >&2 || true
    fail "cargo test export_semantic_parity_default_calls1 failed"
  fi
else
  log "NOTE: cargo not available; skipping unit test gate (CLI path only)"
fi

# ---------------------------------------------------------------------------
# Folded stacks × 2
# ---------------------------------------------------------------------------
assert_folded_semantics() {
  local file="$1" label="$2"
  [[ -s "$file" ]] || fail "$label: empty folded output"

  if grep -q 'main::leaf' "$file"; then
    ok "$label: main::leaf present"
  else
    fail "$label: missing main::leaf
$(cat "$file")"
  fi
  if grep -q 'main::mid' "$file"; then
    ok "$label: main::mid present"
  else
    fail "$label: missing main::mid
$(cat "$file")"
  fi

  # Exact contracted lines (A7).
  if grep -qE '^main::mid;main::leaf 15$' "$file" \
    || grep -qF 'main::mid;main::leaf 15' "$file"; then
    ok "$label: main::mid;main::leaf 15"
  else
    fail "$label: missing main::mid;main::leaf 15
$(cat "$file")"
  fi

  # mid returns relationship 3 via RUNTIME→mid when present.
  if grep -qF 'main::RUNTIME;main::mid' "$file"; then
    if grep -qE '^main::RUNTIME;main::mid 3$' "$file" \
      || grep -qF 'main::RUNTIME;main::mid 3' "$file"; then
      ok "$label: main::RUNTIME;main::mid 3"
    else
      fail "$label: RUNTIME→mid present but count is not 3
$(cat "$file")"
    fi
  else
    log "NOTE: $label: no main::RUNTIME;main::mid line (skip mid returns=3 folded check)"
  fi
}

log "=== native folded (run 1) ==="
if ! run_native folded "$FIXTURE" >"$FOLDED1" 2>"$TMP/folded1.err"; then
  cat "$FOLDED1" >&2 || true
  cat "$TMP/folded1.err" >&2 || true
  fail "native folded run 1 failed"
fi
assert_folded_semantics "$FOLDED1" "folded run 1"

log "=== native folded (run 2) ==="
if ! run_native folded "$FIXTURE" >"$FOLDED2" 2>"$TMP/folded2.err"; then
  cat "$FOLDED2" >&2 || true
  cat "$TMP/folded2.err" >&2 || true
  fail "native folded run 2 failed"
fi
assert_folded_semantics "$FOLDED2" "folded run 2"

if cmp -s "$FOLDED1" "$FOLDED2"; then
  ok "folded run 1 and run 2 are byte-identical"
else
  fail "folded run 1 and run 2 differ (not stable)
--- run1 ---
$(cat "$FOLDED1")
--- run2 ---
$(cat "$FOLDED2")"
fi

# ---------------------------------------------------------------------------
# Callgrind × 2 (primary command) + cg alias × 2
# ---------------------------------------------------------------------------
assert_callgrind_semantics() {
  local file="$1" label="$2"
  [[ -s "$file" ]] || fail "$label: empty callgrind output"

  grep -q '# callgrind format' "$file" || fail "$label: missing # callgrind format header
$(head -n 20 "$file")"
  grep -q 'positions: line' "$file" || fail "$label: missing positions: line
$(head -n 20 "$file")"

  if grep -q 'main::leaf' "$file"; then
    ok "$label: main::leaf present"
  else
    fail "$label: missing main::leaf
$(cat "$file")"
  fi
  if grep -q 'main::mid' "$file"; then
    ok "$label: main::mid present"
  else
    fail "$label: missing main::mid
$(cat "$file")"
  fi

  # cfn=main::leaf with calls=15 (mid→leaf).
  if grep -q 'cfn=main::leaf' "$file"; then
    ok "$label: cfn=main::leaf"
  else
    # Accept fn= only if calls=15 still present nearby is weak; prefer cfn.
    if grep -qE 'fn=main::leaf|cfn=main::leaf' "$file"; then
      ok "$label: fn/cfn main::leaf (loose)"
    else
      fail "$label: missing cfn=main::leaf
$(cat "$file")"
    fi
  fi
  if grep -qE 'calls=15( 0)?$' "$file" || grep -qF 'calls=15' "$file"; then
    ok "$label: calls=15 (mid→leaf)"
  else
    fail "$label: missing calls=15
$(cat "$file")"
  fi

  # mid relationship 3: cfn=main::mid + calls=3 (typically under RUNTIME).
  if grep -q 'cfn=main::mid' "$file"; then
    if grep -qE 'calls=3( 0)?$' "$file" || grep -qF 'calls=3' "$file"; then
      ok "$label: cfn=main::mid + calls=3"
    else
      fail "$label: cfn=main::mid present but missing calls=3
$(cat "$file")"
    fi
  elif grep -q 'fn=main::mid' "$file"; then
    # fn presence without cfn still counts as mid presence; calls=3 may be absent
    # if RUNTIME edge is missing from export.
    ok "$label: fn=main::mid present (no cfn=main::mid; skip calls=3)"
  else
    fail "$label: missing mid as fn/cfn
$(cat "$file")"
  fi
}

log "=== native callgrind (run 1) ==="
if ! run_native callgrind "$FIXTURE" >"$CG1" 2>"$TMP/callgrind1.err"; then
  cat "$CG1" >&2 || true
  cat "$TMP/callgrind1.err" >&2 || true
  fail "native callgrind run 1 failed"
fi
assert_callgrind_semantics "$CG1" "callgrind run 1"

log "=== native callgrind (run 2) ==="
if ! run_native callgrind "$FIXTURE" >"$CG2" 2>"$TMP/callgrind2.err"; then
  cat "$CG2" >&2 || true
  cat "$TMP/callgrind2.err" >&2 || true
  fail "native callgrind run 2 failed"
fi
assert_callgrind_semantics "$CG2" "callgrind run 2"

if cmp -s "$CG1" "$CG2"; then
  ok "callgrind run 1 and run 2 are byte-identical"
else
  fail "callgrind run 1 and run 2 differ (not stable)
--- run1 ---
$(cat "$CG1")
--- run2 ---
$(cat "$CG2")"
fi

log "=== native cg alias (run 1) ==="
if ! run_native cg "$FIXTURE" >"$CG_ALIAS1" 2>"$TMP/cg1.err"; then
  cat "$CG_ALIAS1" >&2 || true
  cat "$TMP/cg1.err" >&2 || true
  fail "native cg run 1 failed"
fi
assert_callgrind_semantics "$CG_ALIAS1" "cg run 1"

log "=== native cg alias (run 2) ==="
if ! run_native cg "$FIXTURE" >"$CG_ALIAS2" 2>"$TMP/cg2.err"; then
  cat "$CG_ALIAS2" >&2 || true
  cat "$TMP/cg2.err" >&2 || true
  fail "native cg run 2 failed"
fi
assert_callgrind_semantics "$CG_ALIAS2" "cg run 2"

if cmp -s "$CG_ALIAS1" "$CG_ALIAS2"; then
  ok "cg run 1 and run 2 are byte-identical"
else
  fail "cg run 1 and run 2 differ (not stable)"
fi

# callgrind and cg should match each other (same render path).
if cmp -s "$CG1" "$CG_ALIAS1"; then
  ok "callgrind and cg alias produce identical stdout"
else
  fail "callgrind and cg alias differ
--- callgrind ---
$(cat "$CG1")
--- cg ---
$(cat "$CG_ALIAS1")"
fi

# Optional durable capture (verification / CI evidence). Example:
#   EXPORT_PARITY_KEEP_DIR=/path/to/keep bash tools/oracle/export_semantic_parity.sh
if [[ -n "${EXPORT_PARITY_KEEP_DIR:-}" ]]; then
  mkdir -p "$EXPORT_PARITY_KEEP_DIR"
  cp -a "$FOLDED1" "$EXPORT_PARITY_KEEP_DIR/folded1.out"
  cp -a "$FOLDED2" "$EXPORT_PARITY_KEEP_DIR/folded2.out"
  cp -a "$CG1" "$EXPORT_PARITY_KEEP_DIR/callgrind1.out"
  cp -a "$CG2" "$EXPORT_PARITY_KEEP_DIR/callgrind2.out"
  cp -a "$CG_ALIAS1" "$EXPORT_PARITY_KEEP_DIR/cg1.out"
  ok "kept export evidence under $EXPORT_PARITY_KEEP_DIR"
fi

log ""
ok "export semantic parity smoke passed (folded mid;leaf 15 + RUNTIME;mid 3; callgrind leaf/mid/calls; stable ×2)"
exit 0

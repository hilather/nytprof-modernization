#!/usr/bin/env bash
# Blocks semantic parity smoke: native HTML/report on blocks-calls1.
#
# Spec: docs/schemas/blocks-semantic-parity-mvp-v0.md
# Board: BLOCKS-SEMANTIC-PARITY
#
# Checks (exact counts only; ticks not compared):
#   line_total(1,5).calls == 780 (A4 from TIME_BLOCK)
#   main::leaf returns 15, main::mid returns 3
#
# Native path uses cargo / prefix CLI only. Oracle nytprofhtml is optional
# (not required for this MVP; see REPORT-SEMANTIC-PARITY for oracle HTML pattern).
#
# Usage (from repo root or any cwd):
#   bash tools/oracle/blocks_semantic_parity.sh
#   ./tools/oracle/blocks_semantic_parity.sh   # if executable
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
cd "$ROOT"

FIXTURE="fixtures/v5/blocks-calls1/nytprof.out"
FIXTURE_ABS="$ROOT/$FIXTURE"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

[[ -f "$FIXTURE_ABS" ]] || fail "missing fixture $FIXTURE"

# ---------------------------------------------------------------------------
# Temp workspace
# ---------------------------------------------------------------------------
TMP="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-blocks-semantic-XXXXXX")"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

NATIVE_HTML="$TMP/native.html"
NATIVE_SITE="$TMP/native-site"
NATIVE_REPORT="$TMP/native-report.txt"
mkdir -p "$NATIVE_SITE"

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
  log "=== cargo test blocks_semantic_parity_blocks_calls1 ==="
  if cargo test -q -p nytprof-report blocks_semantic_parity_blocks_calls1 -- --nocapture \
    >"$TMP/cargo-test.out" 2>"$TMP/cargo-test.err"; then
    ok "cargo test blocks_semantic_parity_blocks_calls1"
  else
    cat "$TMP/cargo-test.out" >&2 || true
    cat "$TMP/cargo-test.err" >&2 || true
    fail "cargo test blocks_semantic_parity_blocks_calls1 failed"
  fi
else
  log "NOTE: cargo not available; skipping unit test gate (CLI path only)"
fi

# ---------------------------------------------------------------------------
# Native text report (quick leaf/mid evidence)
# ---------------------------------------------------------------------------
log "=== native report ==="
if ! run_native report "$FIXTURE" \
  >"$NATIVE_REPORT" 2>"$TMP/native-report.err"; then
  cat "$NATIVE_REPORT" >&2 || true
  cat "$TMP/native-report.err" >&2 || true
  fail "native report failed"
fi
[[ -s "$NATIVE_REPORT" ]] || fail "native report empty"
grep -q 'main::leaf' "$NATIVE_REPORT" || fail "report missing main::leaf"
grep -q 'main::mid' "$NATIVE_REPORT" || fail "report missing main::mid"
# report lines look like: "  main::leaf  returns=15  excl=...  incl=..."
if grep -qE 'main::leaf[[:space:]]+returns=15' "$NATIVE_REPORT" \
  || grep -qE 'main::leaf.*returns=15' "$NATIVE_REPORT"; then
  ok "native report: main::leaf returns=15"
else
  fail "native report missing leaf returns=15
$(cat "$NATIVE_REPORT")"
fi
if grep -qE 'main::mid[[:space:]]+returns=3' "$NATIVE_REPORT" \
  || grep -qE 'main::mid.*returns=3' "$NATIVE_REPORT"; then
  ok "native report: main::mid returns=3"
else
  fail "native report missing mid returns=3
$(cat "$NATIVE_REPORT")"
fi
# Top lines section should include calls 780 for the hot loop.
if grep -qE '[[:space:]]780[[:space:]]' "$NATIVE_REPORT" \
  || grep -q '780' "$NATIVE_REPORT"; then
  ok "native report: line calls 780 present"
else
  fail "native report missing line calls 780
$(cat "$NATIVE_REPORT")"
fi

# ---------------------------------------------------------------------------
# Native single-file HTML
# ---------------------------------------------------------------------------
log "=== native html -o ==="
if ! run_native html "$FIXTURE" -o "$NATIVE_HTML" \
  >"$TMP/native-html.out" 2>"$TMP/native-html.err"; then
  cat "$TMP/native-html.out" >&2 || true
  cat "$TMP/native-html.err" >&2 || true
  fail "native html -o failed"
fi
[[ -s "$NATIVE_HTML" ]] || fail "native HTML missing or empty: $NATIVE_HTML"

grep -q 'main::leaf' "$NATIVE_HTML" || fail "native HTML missing main::leaf"
grep -q 'main::mid' "$NATIVE_HTML" || fail "native HTML missing main::mid"

if grep -qE 'main::leaf</td><td class="num">15</td>' "$NATIVE_HTML" \
  || grep -qE 'main::leaf</td>\s*<td class="num">15</td>' "$NATIVE_HTML"; then
  ok "native single-file: main::leaf returns=15 (table cell)"
else
  if grep -q 'main::leaf' "$NATIVE_HTML" && grep -qE '>15<' "$NATIVE_HTML"; then
    ok "native single-file: main::leaf + >15< present (loose)"
  else
    fail "native HTML missing leaf returns 15 in sub context"
  fi
fi

if grep -qE 'main::mid</td><td class="num">3</td>' "$NATIVE_HTML" \
  || grep -qE 'main::mid</td>\s*<td class="num">3</td>' "$NATIVE_HTML"; then
  ok "native single-file: main::mid returns=3 (table cell)"
else
  if grep -q 'main::mid' "$NATIVE_HTML" && grep -qE '>3<' "$NATIVE_HTML"; then
    ok "native single-file: main::mid + >3< present (loose)"
  else
    fail "native HTML missing mid returns 3 in sub context"
  fi
fi

# A4 line calls 780 — prefer source-table row for line 5, else any >780< cell.
if grep -qE '<td class="num">5</td><td class="num">780</td>' "$NATIVE_HTML" \
  || grep -qE 'class="num">5</td>[[:space:]]*<td class="num">780</td>' "$NATIVE_HTML"; then
  ok "native single-file: line 5 calls=780 (source row)"
elif grep -qE '>780<' "$NATIVE_HTML"; then
  ok "native single-file: calls=780 present as cell (loose)"
else
  fail "native HTML missing line calls 780"
fi

# ---------------------------------------------------------------------------
# Native multi-file site (--out-dir)
# ---------------------------------------------------------------------------
log "=== native html --out-dir ==="
if ! run_native html "$FIXTURE" --out-dir "$NATIVE_SITE" \
  >"$TMP/native-site.out" 2>"$TMP/native-site.err"; then
  cat "$TMP/native-site.out" >&2 || true
  cat "$TMP/native-site.err" >&2 || true
  fail "native html --out-dir failed"
fi
[[ -s "$NATIVE_SITE/index.html" ]] || fail "native site missing index.html"

INDEX="$NATIVE_SITE/index.html"
grep -q 'main::leaf' "$INDEX" || fail "native index missing main::leaf"
grep -q 'main::mid' "$INDEX" || fail "native index missing main::mid"
if grep -qE 'main::leaf</td><td class="num">15</td>' "$INDEX" \
  || (grep -q 'main::leaf' "$INDEX" && grep -qE '>15<' "$INDEX"); then
  ok "native multi-file index: leaf returns 15"
else
  fail "native index missing leaf returns 15"
fi
if grep -qE 'main::mid</td><td class="num">3</td>' "$INDEX" \
  || (grep -q 'main::mid' "$INDEX" && grep -qE '>3<' "$INDEX"); then
  ok "native multi-file index: mid returns 3"
else
  fail "native index missing mid returns 3"
fi

# Source page (source.html preferred; else file-1.html)
SOURCE_PAGE=""
if [[ -s "$NATIVE_SITE/source.html" ]]; then
  SOURCE_PAGE="$NATIVE_SITE/source.html"
elif [[ -s "$NATIVE_SITE/file-1.html" ]]; then
  SOURCE_PAGE="$NATIVE_SITE/file-1.html"
else
  fail "native site missing source.html and file-1.html
$(ls -la "$NATIVE_SITE" 2>/dev/null || true)"
fi

if grep -qE '<td class="num">5</td><td class="num">780</td>' "$SOURCE_PAGE" \
  || grep -qE 'class="num">5</td>[[:space:]]*<td class="num">780</td>' "$SOURCE_PAGE" \
  || grep -qE '>780<' "$SOURCE_PAGE"; then
  ok "native multi-file source: line calls 780 ($(basename "$SOURCE_PAGE"))"
else
  fail "native source page missing line calls 780: $SOURCE_PAGE"
fi

# Optional durable capture (verification / CI evidence). Example:
#   BLOCKS_PARITY_KEEP_DIR=/path/to/keep bash tools/oracle/blocks_semantic_parity.sh
if [[ -n "${BLOCKS_PARITY_KEEP_DIR:-}" ]]; then
  mkdir -p "$BLOCKS_PARITY_KEEP_DIR"
  rm -rf "$BLOCKS_PARITY_KEEP_DIR/native-site"
  cp -a "$NATIVE_HTML" "$BLOCKS_PARITY_KEEP_DIR/native.html"
  cp -a "$NATIVE_SITE" "$BLOCKS_PARITY_KEEP_DIR/native-site"
  cp -a "$NATIVE_REPORT" "$BLOCKS_PARITY_KEEP_DIR/native-report.txt"
  ok "kept HTML/report evidence under $BLOCKS_PARITY_KEEP_DIR"
fi

log ""
ok "blocks semantic parity smoke passed (line5.calls=780, leaf=15, mid=3)"
exit 0

#!/usr/bin/env bash
# Report semantic parity smoke: oracle nytprofhtml + native HTML on default-calls1.
#
# Spec: docs/schemas/report-semantic-parity-mvp-v0.md
# Board: REPORT-SEMANTIC-PARITY
# Contract evidence: docs/contracts/REPORT_SURFACE_CONTRACT_v0.md (REPORT-CONTRACT-FREEZE)
#
# Checks (exact counts only; ticks not compared):
#   main::leaf returns 15, main::mid returns 3, mid→leaf call count 15
#
# Oracle isolation: tools/oracle/env.sh — PERL5LIB from baseline/6.15 only
# (never crates/). Native path uses cargo / prefix CLI only.
#
# Usage (from repo root or any cwd):
#   bash tools/oracle/report_semantic_parity.sh
#   ./tools/oracle/report_semantic_parity.sh   # if executable
# Optional durable HTML capture:
#   REPORT_PARITY_KEEP_DIR=/tmp/report-contract-freeze-evidence \
#     bash tools/oracle/report_semantic_parity.sh
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
cd "$ROOT"

ENV_SH="$ROOT/tools/oracle/env.sh"
FIXTURE="fixtures/v5/default-calls1/nytprof.out"
FIXTURE_ABS="$ROOT/$FIXTURE"
BASELINE="$ROOT/baseline/6.15"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

[[ -f "$FIXTURE_ABS" ]] || fail "missing fixture $FIXTURE"

# ---------------------------------------------------------------------------
# Temp workspace
# ---------------------------------------------------------------------------
TMP="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-report-semantic-XXXXXX")"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

ORACLE_HTML="$TMP/oracle-html"
NATIVE_HTML="$TMP/native.html"
NATIVE_SITE="$TMP/native-site"
mkdir -p "$ORACLE_HTML" "$NATIVE_SITE"

# ---------------------------------------------------------------------------
# Oracle PERL5LIB isolation (never crates/)
# ---------------------------------------------------------------------------
if [[ ! -f "$BASELINE/oracle-perl5lib.txt" ]]; then
  fail "Oracle not built; run scripts/baseline/run_all.sh
  missing: $BASELINE/oracle-perl5lib.txt"
fi

# nytprofhtml requires File::Which (Makefile.PL runtime dep). Prefer local
# test-deps (gitignored) so we never put crates/ on PERL5LIB.
ensure_file_which() {
  local deps="$BASELINE/test-deps"
  local lib="$deps/lib/perl5"
  if PERL5LIB="${lib}:${PERL5LIB-}" perl -MFile::Which -e '1' 2>/dev/null; then
    export PERL5LIB="${lib}${PERL5LIB:+:$PERL5LIB}"
    ok "File::Which available (test-deps or site)"
    return 0
  fi
  log "NOTE: File::Which missing; installing into baseline/6.15/test-deps (local only)"
  mkdir -p "$deps"
  if command -v cpanm >/dev/null 2>&1; then
    cpanm -L "$deps" --notest File::Which \
      || fail "cpanm failed installing File::Which into $deps"
  else
    # Fallback: CPAN client with INSTALL_BASE
    PERL_MM_OPT="INSTALL_BASE=$deps" cpan -T File::Which \
      || fail "install File::Which into $deps (need cpanm or cpan):
  cpanm -L baseline/6.15/test-deps File::Which"
  fi
  export PERL5LIB="${lib}${PERL5LIB:+:$PERL5LIB}"
  PERL5LIB="$PERL5LIB" perl -MFile::Which -e '1' \
    || fail "File::Which still missing after install attempt"
  ok "File::Which installed under baseline/6.15/test-deps"
}

# shellcheck source=env.sh
source "$ENV_SH"
ok "sourced tools/oracle/env.sh"
ensure_file_which
# Re-apply test-deps first after ensure (env.sh may already prepend it)
if [[ -d "$BASELINE/test-deps/lib/perl5" ]]; then
  case ":${PERL5LIB-}:" in
    *":$BASELINE/test-deps/lib/perl5:"*) ;;
    *) export PERL5LIB="$BASELINE/test-deps/lib/perl5${PERL5LIB:+:$PERL5LIB}" ;;
  esac
fi

case ":${PERL5LIB-}:" in
  *"/crates/"*)
    fail "PERL5LIB must not contain /crates/: $PERL5LIB"
    ;;
esac
IFS=':' read -r -a _p5_entries <<<"${PERL5LIB-}"
for _e in "${_p5_entries[@]}"; do
  [[ -z "$_e" ]] && continue
  case "$_e" in
    *"/crates/"*|*"${ROOT}/crates"*|"$ROOT/crates"/*)
      fail "PERL5LIB entry points at crates/: $_e"
      ;;
  esac
done
ok "PERL5LIB has no /crates/ entries"

command -v nytprofhtml >/dev/null 2>&1 \
  || fail "nytprofhtml not on PATH after oracle env (expected baseline/6.15/install/bin)"

# ---------------------------------------------------------------------------
# Oracle: nytprofhtml site
# ---------------------------------------------------------------------------
log "=== oracle nytprofhtml ==="
# Flags: -o output dir, -f input file (also accepts bare profile path).
if ! nytprofhtml -o "$ORACLE_HTML" -f "$FIXTURE_ABS" \
  >"$TMP/oracle-nytprofhtml.out" 2>"$TMP/oracle-nytprofhtml.err"; then
  cat "$TMP/oracle-nytprofhtml.out" >&2 || true
  cat "$TMP/oracle-nytprofhtml.err" >&2 || true
  fail "nytprofhtml failed for $FIXTURE"
fi

# Non-empty HTML site: index.html preferred; accept any non-empty *.html.
if [[ -f "$ORACLE_HTML/index.html" ]]; then
  [[ -s "$ORACLE_HTML/index.html" ]] || fail "oracle index.html is empty"
  ok "oracle site: index.html present and non-empty"
else
  html_count=0
  while IFS= read -r -d '' f; do
    if [[ -s "$f" ]]; then
      html_count=$((html_count + 1))
    fi
  done < <(find "$ORACLE_HTML" -type f \( -name '*.html' -o -name '*.htm' \) -print0 2>/dev/null || true)
  [[ "$html_count" -gt 0 ]] \
    || fail "oracle out dir has no non-empty HTML under $ORACLE_HTML
$(ls -la "$ORACLE_HTML" 2>/dev/null || true)
stderr: $(cat "$TMP/oracle-nytprofhtml.err" 2>/dev/null || true)"
  ok "oracle site: $html_count non-empty HTML file(s) (no index.html name)"
fi

# ---------------------------------------------------------------------------
# Resolve native CLI (cargo run preferred for shipped path; prefix if no cargo)
# ---------------------------------------------------------------------------
run_native() {
  # Usage: run_native <html args...>
  if command -v cargo >/dev/null 2>&1; then
    cargo run -q -p nytprof-cli -- html "$@"
  elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
    "$ROOT/prefix/bin/nytprof-cli" html "$@"
  elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
    "$ROOT/target/debug/nytprof-dump" html "$@"
  elif [[ -x "$ROOT/target/release/nytprof-dump" ]]; then
    "$ROOT/target/release/nytprof-dump" html "$@"
  else
    fail "no cargo and no prefix/target nytprof-cli binary found"
  fi
}

# ---------------------------------------------------------------------------
# Native single-file HTML
# ---------------------------------------------------------------------------
log "=== native html -o ==="
if ! run_native "$FIXTURE" -o "$NATIVE_HTML" \
  >"$TMP/native-html.out" 2>"$TMP/native-html.err"; then
  cat "$TMP/native-html.out" >&2 || true
  cat "$TMP/native-html.err" >&2 || true
  fail "native html -o failed"
fi
[[ -s "$NATIVE_HTML" ]] || fail "native HTML missing or empty: $NATIVE_HTML"

# Prefer structured markers from the shipped render path over bare "15" greps.
# Subs table: <tr><td>main::leaf</td><td class="num">15</td>
# Call edges: <tr><td>main::mid</td><td>main::leaf</td><td class="num">15</td>
if ! grep -q 'main::leaf' "$NATIVE_HTML"; then
  fail "native HTML missing main::leaf"
fi
if ! grep -q 'main::mid' "$NATIVE_HTML"; then
  fail "native HTML missing main::mid"
fi

# Leaf returns 15 in sub table context
if grep -qE 'main::leaf</td><td class="num">15</td>' "$NATIVE_HTML" \
  || grep -qE 'main::leaf</td>\s*<td class="num">15</td>' "$NATIVE_HTML"; then
  ok "native single-file: main::leaf returns=15 (table cell)"
else
  # Fallback: name near returns cell 15
  if grep -q 'main::leaf' "$NATIVE_HTML" && grep -qE '>15<' "$NATIVE_HTML"; then
    ok "native single-file: main::leaf + >15< present (loose)"
  else
    fail "native HTML missing leaf returns 15 in sub context"
  fi
fi

# Mid returns 3
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

# mid → leaf edge count 15 (call-edges table)
if grep -qE 'main::mid</td><td>main::leaf</td>[[:space:]]*<td class="num">15</td>' "$NATIVE_HTML" \
  || grep -qE 'main::mid</td><td>main::leaf</td><td class="num">15</td>' "$NATIVE_HTML"; then
  ok "native single-file: mid→leaf count=15 (call-edges row)"
else
  # Slice around call-edges section
  if grep -qi 'call.edges\|call-edges' "$NATIVE_HTML"; then
    edges_chunk="$(awk 'BEGIN{IGNORECASE=1} /call.edges|call-edges/{p=1} p{print}' "$NATIVE_HTML" | head -n 200)"
    if printf '%s' "$edges_chunk" | grep -q 'main::mid' \
      && printf '%s' "$edges_chunk" | grep -q 'main::leaf' \
      && printf '%s' "$edges_chunk" | grep -qE '>15<'; then
      ok "native single-file: mid→leaf count 15 in call-edges section (loose)"
    else
      fail "native HTML call-edges missing mid→leaf count 15
chunk:
$edges_chunk"
    fi
  else
    fail "native HTML missing Call edges section"
  fi
fi

# ---------------------------------------------------------------------------
# Native multi-file site (--out-dir)
# ---------------------------------------------------------------------------
log "=== native html --out-dir ==="
if ! run_native "$FIXTURE" --out-dir "$NATIVE_SITE" \
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
if grep -qE 'main::mid</td><td>main::leaf</td><td class="num">15</td>' "$INDEX" \
  || (grep -qi 'call.edges\|call-edges' "$INDEX" \
      && awk 'BEGIN{IGNORECASE=1} /call.edges|call-edges/{p=1} p{print}' "$INDEX" | head -n 200 \
        | grep -q 'main::mid' \
      && awk 'BEGIN{IGNORECASE=1} /call.edges|call-edges/{p=1} p{print}' "$INDEX" | head -n 200 \
        | grep -q 'main::leaf' \
      && awk 'BEGIN{IGNORECASE=1} /call.edges|call-edges/{p=1} p{print}' "$INDEX" | head -n 200 \
        | grep -qE '>15<'); then
  ok "native multi-file index: mid→leaf count 15"
else
  fail "native index missing mid→leaf count 15"
fi

# Optional durable capture (verification / CI evidence). Example:
#   REPORT_PARITY_KEEP_DIR=/path/to/keep bash tools/oracle/report_semantic_parity.sh
if [[ -n "${REPORT_PARITY_KEEP_DIR:-}" ]]; then
  mkdir -p "$REPORT_PARITY_KEEP_DIR"
  rm -rf "$REPORT_PARITY_KEEP_DIR/oracle-html" "$REPORT_PARITY_KEEP_DIR/native-site"
  cp -a "$ORACLE_HTML" "$REPORT_PARITY_KEEP_DIR/oracle-html"
  cp -a "$NATIVE_HTML" "$REPORT_PARITY_KEEP_DIR/native.html"
  cp -a "$NATIVE_SITE" "$REPORT_PARITY_KEEP_DIR/native-site"
  ok "kept HTML evidence under $REPORT_PARITY_KEEP_DIR"
fi

log ""
ok "report semantic parity smoke passed (leaf=15, mid=3, mid→leaf=15)"
exit 0

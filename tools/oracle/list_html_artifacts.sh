#!/usr/bin/env bash
# List oracle nytprofhtml (and optionally native HTML) artifacts for residual inventory.
#
# Spec: docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md
# Board: REPORT-HTML-RESIDUAL-INV
#
# Isolation: tools/oracle/env.sh — PERL5LIB from baseline/6.15 only (never crates/).
#
# Usage (from repo root or any cwd):
#   bash tools/oracle/list_html_artifacts.sh
#   OUT_DIR=/tmp/oracle-html-list bash tools/oracle/list_html_artifacts.sh
#   ORACLE_HTML_DIR=/path/to/existing/oracle-site bash tools/oracle/list_html_artifacts.sh  # list only
#   LIST_NATIVE=1 bash tools/oracle/list_html_artifacts.sh
#   SKIP_GENERATE=1 ORACLE_HTML_DIR=... bash tools/oracle/list_html_artifacts.sh
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
# Temp / keep dirs
# ---------------------------------------------------------------------------
KEEP="${OUT_DIR:-}"
if [[ -n "$KEEP" ]]; then
  mkdir -p "$KEEP"
  ORACLE_HTML="${ORACLE_HTML_DIR:-$KEEP/oracle-html}"
  NATIVE_SITE="${NATIVE_SITE_DIR:-$KEEP/native-site}"
  NATIVE_HTML="${NATIVE_HTML_FILE:-$KEEP/native.html}"
  CLEANUP_TMP=0
else
  TMP="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-html-artifacts-XXXXXX")"
  cleanup() { rm -rf "$TMP"; }
  trap cleanup EXIT
  ORACLE_HTML="${ORACLE_HTML_DIR:-$TMP/oracle-html}"
  NATIVE_SITE="${NATIVE_SITE_DIR:-$TMP/native-site}"
  NATIVE_HTML="${NATIVE_HTML_FILE:-$TMP/native.html}"
  CLEANUP_TMP=1
fi

# ---------------------------------------------------------------------------
# Classify files under a directory (relative paths)
# ---------------------------------------------------------------------------
classify_tree() {
  local root="$1"
  local label="$2"
  [[ -d "$root" ]] || fail "not a directory: $root"

  log ""
  log "=== $label: $root ==="

  local total
  total="$(find "$root" -type f | wc -l | tr -d ' ')"
  log "total_files: $total"

  # Print sorted relative listing
  log "--- relative listing ---"
  (
    cd "$root"
    find . -type f | sed 's|^\./||' | sort
  )

  log "--- class hits ---"
  (
    cd "$root"
    hit() {
      local name="$1"
      local pat="$2"
      local n
      n="$(find . -type f -name "$pat" 2>/dev/null | wc -l | tr -d ' ')"
      if [[ "$n" -gt 0 ]]; then
        printf '  %-28s yes  (count=%s)\n' "$name" "$n"
        find . -type f -name "$pat" | sed 's|^\./||' | sort | head -n 12 | sed 's/^/      example: /'
        if [[ "$n" -gt 12 ]]; then
          printf '      ... +%s more\n' "$((n - 12))"
        fi
      else
        printf '  %-28s no\n' "$name"
      fi
    }
    hit "index.html" "index.html"
    hit "index-subs-excl.html" "index-subs-excl.html"
    hit "style.css" "style.css"
    hit "js assets" "*.js"
    hit "css under js/" "*.css"
    hit "png icons/gradients" "*.png"
    hit "flame svg" "*.svg"
    hit "flame/call stacks" "*.calls"
    hit "flamegraph_subattr" "flamegraph_subattr.txt"
    hit "graphviz .dot" "*.dot"
    hit "packages-callgraph.dot" "packages-callgraph.dot"
    hit "subs-callgraph.dot" "subs-callgraph.dot"
    hit "subs-treemap" "subs-treemap*.html"
    hit "*-line.html pages" "*-line.html"
    hit "*-block.html pages" "*-block.html"
    hit "*-sub.html pages" "*-sub.html"
    hit "native file-*.html" "file-*.html"
    hit "native source.html" "source.html"
    hit "any *.html" "*.html"
  )
}

# ---------------------------------------------------------------------------
# Oracle generate (unless SKIP_GENERATE or pre-existing dir requested as list-only)
# ---------------------------------------------------------------------------
if [[ "${SKIP_GENERATE:-0}" != "1" ]]; then
  if [[ ! -f "$BASELINE/oracle-perl5lib.txt" ]]; then
    fail "Oracle not built; run scripts/baseline/run_all.sh
  missing: $BASELINE/oracle-perl5lib.txt"
  fi

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

  rm -rf "$ORACLE_HTML"
  mkdir -p "$ORACLE_HTML"
  log "=== oracle nytprofhtml → $ORACLE_HTML ==="
  if ! nytprofhtml -o "$ORACLE_HTML" -f "$FIXTURE_ABS" \
    >"${ORACLE_HTML}.stdout" 2>"${ORACLE_HTML}.stderr"; then
    cat "${ORACLE_HTML}.stdout" >&2 || true
    cat "${ORACLE_HTML}.stderr" >&2 || true
    fail "nytprofhtml failed for $FIXTURE"
  fi
  if [[ -f "$ORACLE_HTML/index.html" && -s "$ORACLE_HTML/index.html" ]]; then
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
$(ls -la "$ORACLE_HTML" 2>/dev/null || true)"
    ok "oracle site: $html_count non-empty HTML file(s)"
  fi
else
  [[ -d "$ORACLE_HTML" ]] || fail "SKIP_GENERATE=1 but ORACLE_HTML_DIR/ORACLE_HTML missing: $ORACLE_HTML"
  ok "skip generate; listing existing $ORACLE_HTML"
fi

classify_tree "$ORACLE_HTML" "oracle nytprofhtml"

# ---------------------------------------------------------------------------
# Optional native listing
# ---------------------------------------------------------------------------
if [[ "${LIST_NATIVE:-0}" == "1" ]]; then
  run_native() {
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

  log "=== native html -o / --out-dir ==="
  rm -rf "$NATIVE_SITE"
  mkdir -p "$(dirname "$NATIVE_HTML")"
  run_native "$FIXTURE" -o "$NATIVE_HTML" \
    >"${NATIVE_HTML}.stdout" 2>"${NATIVE_HTML}.stderr" \
    || fail "native html -o failed"
  [[ -s "$NATIVE_HTML" ]] || fail "native HTML empty: $NATIVE_HTML"
  ok "native single-file: $NATIVE_HTML ($(wc -c <"$NATIVE_HTML" | tr -d ' ') bytes)"

  run_native "$FIXTURE" --out-dir "$NATIVE_SITE" \
    >"${NATIVE_SITE}.stdout" 2>"${NATIVE_SITE}.stderr" \
    || fail "native html --out-dir failed"
  [[ -s "$NATIVE_SITE/index.html" ]] || fail "native site missing index.html"
  classify_tree "$NATIVE_SITE" "native html --out-dir"

  # Quick residual contrast
  log ""
  log "=== residual contrast (default-calls1) ==="
  for cls in style.css "*.svg" "*.dot" "js" "index-subs-excl.html" "file-*.html" "source.html"; do
    o_n="$(find "$ORACLE_HTML" -type f -name "$cls" 2>/dev/null | wc -l | tr -d ' ')"
    n_n="$(find "$NATIVE_SITE" -type f -name "$cls" 2>/dev/null | wc -l | tr -d ' ')"
    # special-case directory js
    if [[ "$cls" == "js" ]]; then
      o_n=0
      n_n=0
      [[ -d "$ORACLE_HTML/js" ]] && o_n="$(find "$ORACLE_HTML/js" -type f | wc -l | tr -d ' ')"
      [[ -d "$NATIVE_SITE/js" ]] && n_n="$(find "$NATIVE_SITE/js" -type f | wc -l | tr -d ' ')"
    fi
    printf '  %-22s oracle=%-4s native=%-4s\n' "$cls" "$o_n" "$n_n"
  done
fi

log ""
ok "list_html_artifacts finished"
log "Inventory doc: docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md"
log "Semantic parity: bash tools/oracle/report_semantic_parity.sh"
if [[ -n "$KEEP" ]]; then
  log "Kept under: $KEEP"
elif [[ "$CLEANUP_TMP" -eq 1 ]]; then
  log "Temp listing only (set OUT_DIR=... to keep trees)"
fi
exit 0

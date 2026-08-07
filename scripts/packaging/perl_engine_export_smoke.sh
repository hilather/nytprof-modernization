#!/usr/bin/env bash
# Perl engine export packaging smoke (PERL-ENGINE-EXPORT).
#
# Spec: docs/schemas/perl-engine-dispatch-mvp-v0.md
# Formats: docs/schemas/export-formats-mvp-v0.md
#
# 1. cargo build -q -p nytprof-cli  (native export requires CLI)
# 2. --engine=native folded  → main::mid;main::leaf 15, main::RUNTIME;main::mid 3
# 3. --engine=native callgrind → fn=main::leaf / calls
# 4. --engine=native cg alias (optional presence check)
#
# Never puts crates/ on oracle PERL5LIB. Does not reimplement export in Perl.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_engine_export_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE="fixtures/v5/default-calls1/nytprof.out"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"
DISPATCH_PM="$ENGINE_LIB/Devel/NYTProf/EngineDispatch.pm"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$FIXTURE" ]] || fail "missing fixture $FIXTURE"
[[ -f "$ROOT/$ENGINE_BIN" ]] || fail "missing $ENGINE_BIN"
[[ -f "$ROOT/$DISPATCH_PM" ]] || fail "missing $DISPATCH_PM"

ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

# Sanity: env must not inject crates/ into any child via this smoke
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

# ---------------------------------------------------------------------------
# Native build
# ---------------------------------------------------------------------------
command -v cargo >/dev/null 2>&1 || fail "cargo required for perl_engine_export_smoke"
cargo build -q -p nytprof-cli
ok "cargo build -p nytprof-cli"

# ---------------------------------------------------------------------------
# folded
# ---------------------------------------------------------------------------
echo "=== --engine=native folded ==="
FOLDED_OUT="$TMPDIR_SMOKE/folded.out"
FOLDED_ERR="$TMPDIR_SMOKE/folded.err"
if ! "${ENGINE[@]}" --engine=native folded "$FIXTURE" \
  >"$FOLDED_OUT" 2>"$FOLDED_ERR"; then
  cat "$FOLDED_ERR" >&2 || true
  cat "$FOLDED_OUT" >&2 || true
  fail "--engine=native folded failed"
fi
cat "$FOLDED_OUT"
grep -qE 'main::mid;main::leaf 15' "$FOLDED_OUT" \
  || fail "folded missing main::mid;main::leaf 15"
grep -qE 'main::RUNTIME;main::mid 3' "$FOLDED_OUT" \
  || fail "folded missing main::RUNTIME;main::mid 3"
ok "folded: mid;leaf 15 and RUNTIME;mid 3"

# ---------------------------------------------------------------------------
# callgrind
# ---------------------------------------------------------------------------
echo "=== --engine=native callgrind ==="
CG_OUT="$TMPDIR_SMOKE/callgrind.out"
CG_ERR="$TMPDIR_SMOKE/callgrind.err"
if ! "${ENGINE[@]}" --engine=native callgrind "$FIXTURE" \
  >"$CG_OUT" 2>"$CG_ERR"; then
  cat "$CG_ERR" >&2 || true
  cat "$CG_OUT" >&2 || true
  fail "--engine=native callgrind failed"
fi
# Head of callgrind can be large; show a sample of matching lines
grep -E 'fn=main::leaf|cfn=main::leaf|calls=' "$CG_OUT" | head -n 20 || true
if ! grep -qE 'fn=main::leaf|cfn=main::leaf' "$CG_OUT"; then
  fail "callgrind missing fn=main::leaf / cfn=main::leaf"
fi
grep -qiE 'calls' "$CG_OUT" || fail "callgrind missing calls"
ok "callgrind: fn=main::leaf and calls present"

# ---------------------------------------------------------------------------
# cg alias
# ---------------------------------------------------------------------------
echo "=== --engine=native cg (alias) ==="
ALIAS_OUT="$TMPDIR_SMOKE/cg.out"
ALIAS_ERR="$TMPDIR_SMOKE/cg.err"
if ! "${ENGINE[@]}" --engine=native cg "$FIXTURE" \
  >"$ALIAS_OUT" 2>"$ALIAS_ERR"; then
  cat "$ALIAS_ERR" >&2 || true
  cat "$ALIAS_OUT" >&2 || true
  fail "--engine=native cg failed"
fi
if ! grep -qE 'fn=main::leaf|cfn=main::leaf' "$ALIAS_OUT"; then
  fail "cg alias missing fn=main::leaf / cfn=main::leaf"
fi
ok "cg alias: callgrind-style output present"

ok "perl engine export packaging smoke passed"
exit 0

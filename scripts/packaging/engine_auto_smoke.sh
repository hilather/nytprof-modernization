#!/usr/bin/env bash
# ENGINE-AUTO-SMOKE: exercise --engine=auto / NYTPROF_ENGINE=auto via nytprof-engine.
#
# Spec: docs/schemas/engine-selection-mvp-v0.md
#       docs/schemas/perl-engine-dispatch-mvp-v0.md
#
# When native CLI is discoverable (prefix/target/env or cargo), auto prefers
# native and a real report/query on default-calls1 must show:
#   main::leaf  returns=15
#   main::mid   returns=3
#
# This smoke requires native discoverable (tests the present-native half of
# auto). For auto→legacy when native is missing, see:
#   ./scripts/packaging/engine_auto_fallback_smoke.sh
# Never puts crates/ on oracle PERL5LIB.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/engine_auto_smoke.sh
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

# ---------------------------------------------------------------------------
# Native must be discoverable (fail closed — packaging expects real native)
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
  fail "native CLI not discoverable (set NYTPROF_NATIVE_CLI, install prefix/bin, build target/*/nytprof-dump, or provide cargo). ENGINE-AUTO-SMOKE requires a real native action under --engine=auto."
fi
ok "native discoverable ($CLI_SPEC)"

# Ensure a binary exists when we only have cargo (auto path may cargo-run).
if [[ "$CLI_SPEC" == "cargo" ]]; then
  cargo build -q -p nytprof-cli
  ok "cargo build -p nytprof-cli"
fi

ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

assert_leaf_mid() {
  local label="$1"
  local out="$2"
  # Report style: "main::leaf  returns=15" (one or more spaces)
  # Query style:  "main::leaf returns=15"
  grep -qE 'main::leaf[[:space:]]+returns=15\b' "$out" \
    || fail "$label missing main::leaf returns=15:\n$(cat "$out")"
  grep -qE 'main::mid[[:space:]]+returns=3\b' "$out" \
    || fail "$label missing main::mid returns=3:\n$(cat "$out")"
  ok "$label: main::leaf returns=15 and main::mid returns=3"
}

# ---------------------------------------------------------------------------
# 1. --engine=auto report (twice for stability)
# ---------------------------------------------------------------------------
echo "=== --engine=auto report (pass 1) ==="
OUT1="$TMPDIR_SMOKE/auto_report_1.out"
ERR1="$TMPDIR_SMOKE/auto_report_1.err"
if ! "${ENGINE[@]}" --engine=auto report "$FIXTURE" >"$OUT1" 2>"$ERR1"; then
  cat "$ERR1" >&2 || true
  cat "$OUT1" >&2 || true
  fail "--engine=auto report failed (pass 1)"
fi
cat "$OUT1"
assert_leaf_mid "--engine=auto report (pass 1)" "$OUT1"

echo "=== --engine=auto report (pass 2) ==="
OUT2="$TMPDIR_SMOKE/auto_report_2.out"
ERR2="$TMPDIR_SMOKE/auto_report_2.err"
if ! "${ENGINE[@]}" --engine=auto report "$FIXTURE" >"$OUT2" 2>"$ERR2"; then
  cat "$ERR2" >&2 || true
  cat "$OUT2" >&2 || true
  fail "--engine=auto report failed (pass 2)"
fi
assert_leaf_mid "--engine=auto report (pass 2)" "$OUT2"

# ---------------------------------------------------------------------------
# 2. NYTPROF_ENGINE=auto (no --engine flag)
# ---------------------------------------------------------------------------
echo "=== NYTPROF_ENGINE=auto report ==="
OUT_ENV="$TMPDIR_SMOKE/auto_env_report.out"
ERR_ENV="$TMPDIR_SMOKE/auto_env_report.err"
if ! env NYTPROF_ENGINE=auto "${ENGINE[@]}" report "$FIXTURE" \
  >"$OUT_ENV" 2>"$ERR_ENV"; then
  cat "$ERR_ENV" >&2 || true
  cat "$OUT_ENV" >&2 || true
  fail "NYTPROF_ENGINE=auto report failed"
fi
assert_leaf_mid "NYTPROF_ENGINE=auto report" "$OUT_ENV"

# ---------------------------------------------------------------------------
# 3. --engine=auto query (sub/edge path; same fixture counts)
# ---------------------------------------------------------------------------
echo "=== --engine=auto query ==="
OUT_Q="$TMPDIR_SMOKE/auto_query.out"
ERR_Q="$TMPDIR_SMOKE/auto_query.err"
if ! "${ENGINE[@]}" --engine=auto query "$FIXTURE" >"$OUT_Q" 2>"$ERR_Q"; then
  cat "$ERR_Q" >&2 || true
  cat "$OUT_Q" >&2 || true
  fail "--engine=auto query failed"
fi
cat "$OUT_Q"
assert_leaf_mid "--engine=auto query" "$OUT_Q"

# Sanity: env must not inject crates/ into any child via this smoke
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

ok "engine-auto packaging smoke passed (leaf=15 mid=3)"
exit 0

#!/usr/bin/env bash
# PR-B10 / E4-v0 — v5↔v6 semantic equality at ProfileModel level.
#
# Stages:
#   --model-only (default for PR-B10): product ProfileModel::from_path on
#     committed dual-sink same-run pairs under fixtures/e4/dual-sink/**;
#     cargo test -p nytprof-model e4_v0_ . No full CLI E5 report path required.
#   (full CLI product smoke is PR-B12b — residual until then)
#
# Isolation: never puts crates/ or collector/ on oracle PERL5LIB.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/e4_v5_v6_semantic_smoke.sh
#   ./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only
#   NYTPROF_REGEN_E4_DUAL=1 ./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="$ROOT/fixtures/e4/dual-sink"
COLLECTOR="$ROOT/collector"
MODE="model-only"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
banner() { printf '\n=== %s ===\n' "$*"; }
note() { printf 'NOTE: %s\n' "$*"; }

for arg in "$@"; do
  case "$arg" in
    --model-only) MODE="model-only" ;;
    --full|--cli)
      fail "full CLI E4 product smoke is residual (PR-B12b); use --model-only"
      ;;
    -h|--help)
      cat <<'EOF'
Usage: e4_v5_v6_semantic_smoke.sh [--model-only]

  --model-only   ProfileModel aggregate equality on dual-sink pairs (PR-B10)
  --full/--cli   Not implemented here (PR-B12b residual)

Env:
  NYTPROF_REGEN_E4_DUAL=1  regenerate dual fixtures via make -C collector test
EOF
      exit 0
      ;;
    *)
      fail "unknown arg: $arg (try --help)"
      ;;
  esac
done

banner "e4_v5_v6_semantic_smoke (E4-v0 model-level; mode=$MODE)"

# Isolation: never put crates/ or collector/ on oracle PERL5LIB.
assert_no_bad_perl5lib() {
  local label="$1"
  local p5="${2-}"
  case ":${p5}:" in
    *"/crates/"*)
      fail "$label PERL5LIB must not contain /crates/: $p5"
      ;;
    *"/collector"*|*"collector/"*)
      fail "$label PERL5LIB must not contain collector path: $p5"
      ;;
  esac
  ok "$label: no crates/ or collector/ on PERL5LIB"
}
assert_no_bad_perl5lib "parent env" "${PERL5LIB-}"

PAIRS=(
  m4
  default_calls1
  blocks_calls1
  calls2_default
)

# Optional regenerate dual-sink wires from COL-014 harness.
if [[ "${NYTPROF_REGEN_E4_DUAL:-0}" == "1" ]]; then
  banner "regenerate dual-sink E4 fixtures (NYTPROF_REGEN_E4_DUAL=1)"
  command -v cc >/dev/null 2>&1 || fail "cc required to regenerate dual fixtures"
  [[ -f "$COLLECTOR/Makefile" ]] || fail "missing collector/Makefile"
  make -C "$COLLECTOR" test
  mkdir -p "$FIXTURE_DIR"
  for stem in "${PAIRS[@]}"; do
    src_v5="$COLLECTOR/build/dual_${stem}_v5.nytprof"
    src_v6="$COLLECTOR/build/dual_${stem}_v6.nytprof"
    [[ -f "$src_v5" ]] || fail "missing regenerated $src_v5"
    [[ -f "$src_v6" ]] || fail "missing regenerated $src_v6"
    cp "$src_v5" "$FIXTURE_DIR/${stem}_v5.nytprof"
    cp "$src_v6" "$FIXTURE_DIR/${stem}_v6.nytprof"
    if [[ -f "$COLLECTOR/build/dual_${stem}_meta.json" ]]; then
      cp "$COLLECTOR/build/dual_${stem}_meta.json" "$FIXTURE_DIR/${stem}_meta.json"
    fi
  done
  # m4 meta filename from test_dual_sink is dual_m4_meta.json
  if [[ -f "$COLLECTOR/build/dual_m4_meta.json" ]]; then
    cp "$COLLECTOR/build/dual_m4_meta.json" "$FIXTURE_DIR/m4_meta.json"
  fi
  ok "regenerated fixtures under $FIXTURE_DIR"
fi

banner "committed dual-sink pairs present"
[[ -d "$FIXTURE_DIR" ]] || fail "missing $FIXTURE_DIR"
for stem in "${PAIRS[@]}"; do
  v5="$FIXTURE_DIR/${stem}_v5.nytprof"
  v6="$FIXTURE_DIR/${stem}_v6.nytprof"
  [[ -f "$v5" ]] || fail "missing E4 dual v5 $v5"
  [[ -f "$v6" ]] || fail "missing E4 dual v6 $v6"
  [[ -s "$v5" ]] || fail "empty $v5"
  [[ -s "$v6" ]] || fail "empty $v6"
  head5="$(head -c 12 "$v5" | tr -d '\0' || true)"
  head6="$(head -c 8 "$v6" | tr -d '\0' || true)"
  [[ "$head5" == "NYTProf 5 0"* || "$head5" == $'NYTProf 5 0\n'* ]] \
    || printf 'NYTProf 5 0\n' | cmp -n 12 - "$v5" >/dev/null 2>&1 \
    || fail "$v5: expected NYTProf 5 0 header"
  [[ "$head6" == "NYTPROF6" ]] \
    || printf 'NYTPROF6' | cmp -n 8 - "$v6" >/dev/null 2>&1 \
    || fail "$v6: expected NYTPROF6 magic"
  ok "pair $stem ($(wc -c <"$v5") / $(wc -c <"$v6") bytes)"
done

if ! command -v cargo >/dev/null 2>&1; then
  banner "cargo missing"
  note "E4-v0 model equality requires cargo (ProfileModel path)"
  note "Committed fixtures present; model compare skipped (honest skip)"
  banner "e4_v5_v6_semantic_smoke PASSED (fixture presence only)"
  ok "mode=$MODE fixtures present; cargo skipped"
  exit 0
fi

banner "E4-v0 model-level cargo tests (e4_v0_*)"
cargo test -p nytprof-model e4_v0_ -- --nocapture
ok "cargo test -p nytprof-model e4_v0_"

# Optional diagnostics via product dump/verify (not full E5 claim).
if [[ -x "$ROOT/target/debug/nytprof-dump" ]] || cargo build -q -p nytprof-cli 2>/dev/null; then
  BIN="$ROOT/target/debug/nytprof-dump"
  if [[ -x "$BIN" ]]; then
    banner "product verify on dual pairs (dump/verify prelim; not full E5)"
    for stem in "${PAIRS[@]}"; do
      "$BIN" verify "$FIXTURE_DIR/${stem}_v5.nytprof" >/dev/null
      "$BIN" verify "$FIXTURE_DIR/${stem}_v6.nytprof" >/dev/null
      ok "verify $stem v5+v6"
    done
  fi
fi

note "E4-v0: model aggregates equal on dual-sink pairs (COL-014 test/dev-only OQ-4)"
note "Scaled synthetic shapes — not full oracle fixtures/v5/* counts (TEST-003/TEST-008 residual)"
note "Full CLI E4 product smoke residual (PR-B12b); wire freeze / CLI v6 default residual"
note "Dual-sink is not product format=dual UX"

banner "e4_v5_v6_semantic_smoke PASSED"
ok "E4-v0 model-level semantic equality ($MODE)"
exit 0

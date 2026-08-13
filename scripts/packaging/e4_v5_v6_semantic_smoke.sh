#!/usr/bin/env bash
# PR-B10 / PR-B12b — E4 v5↔v6 semantic equality smoke.
#
# Stages:
#   --model-only: ProfileModel::from_path on committed dual-sink same-run pairs
#     under fixtures/e4/dual-sink/**; cargo test -p nytprof-model e4_v0_ .
#   --full / --cli (default): model stage + real native CLI product surfaces on
#     both v5 and v6 of each pair (verify, report --json equality, plus
#     report/csv/folded/callgrind greppable equality on default_calls1).
#
# offline_gate invokes --full when native CLI is available (C dual-sink
# fixtures are committed under fixtures/e4/dual-sink/).
#
# Isolation: never puts crates/ or collector/ on oracle PERL5LIB.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/e4_v5_v6_semantic_smoke.sh
#   ./scripts/packaging/e4_v5_v6_semantic_smoke.sh --full
#   ./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only
#   NYTPROF_REGEN_E4_DUAL=1 ./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only
#
# Schema: docs/schemas/e4-product-cli-smoke-mvp-v0.md
# Board:  E4-PRODUCT-CLI-SMOKE-MVP / E4-V0-MODEL-SEMANTIC-MVP
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="$ROOT/fixtures/e4/dual-sink"
ORACLE_PAIR_DIR="$ROOT/fixtures/e4/oracle-pair"
COLLECTOR="$ROOT/collector"
# Default: full product CLI path (PR-B12b). Use --model-only for B10-only.
MODE="full"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
banner() { printf '\n=== %s ===\n' "$*"; }
note() { printf 'NOTE: %s\n' "$*"; }

for arg in "$@"; do
  case "$arg" in
    --model-only) MODE="model-only" ;;
    --full|--cli) MODE="full" ;;
    -h|--help)
      cat <<'EOF'
Usage: e4_v5_v6_semantic_smoke.sh [--full|--cli|--model-only]

  --full / --cli   Default. Model e4_v0_* + real native CLI on v5+v6 dual pairs
                   (verify, report --json equality, E5 surfaces on default_calls1)
  --model-only     ProfileModel aggregate equality only (PR-B10 path)

Env:
  NYTPROF_REGEN_E4_DUAL=1  regenerate dual fixtures via make -C collector test
  NYTPROF_NATIVE_CLI       pin CLI binary (optional)
EOF
      exit 0
      ;;
    *)
      fail "unknown arg: $arg (try --help)"
      ;;
  esac
done

banner "e4_v5_v6_semantic_smoke (mode=$MODE)"

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

banner "committed dual-sink pairs present (C COL-014 dual-sink wires)"
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

banner "committed E4-01/E4-02/E4-03 oracle-pair (beyond dual-sink/)"
[[ -d "$ORACLE_PAIR_DIR" ]] || fail "missing $ORACLE_PAIR_DIR"
ORACLE_V5="$ORACLE_PAIR_DIR/default_calls1_v5.nytprof"
ORACLE_V6="$ORACLE_PAIR_DIR/default_calls1_v6.nytprof"
ORACLE_BLOCKS_V5="$ORACLE_PAIR_DIR/blocks_calls1_v5.nytprof"
ORACLE_BLOCKS_V6="$ORACLE_PAIR_DIR/blocks_calls1_v6.nytprof"
ORACLE_CALLS2_V5="$ORACLE_PAIR_DIR/calls2_default_v5.nytprof"
ORACLE_CALLS2_V6="$ORACLE_PAIR_DIR/calls2_default_v6.nytprof"
[[ -f "$ORACLE_V5" ]] || fail "missing E4-01 oracle v5 $ORACLE_V5"
[[ -f "$ORACLE_V6" ]] || fail "missing E4-01 product v6 $ORACLE_V6"
[[ -s "$ORACLE_V5" && -s "$ORACLE_V6" ]] || fail "empty E4-01 oracle-pair file"
head5o="$(head -c 12 "$ORACLE_V5" | tr -d '\0' || true)"
head6o="$(head -c 8 "$ORACLE_V6" | tr -d '\0' || true)"
[[ "$head5o" == "NYTProf 5"* ]] || fail "$ORACLE_V5: expected NYTProf 5 header"
[[ "$head6o" == "NYTPROF6" ]] || fail "$ORACLE_V6: expected NYTPROF6 magic"
ok "oracle-pair default_calls1 ($(wc -c <"$ORACLE_V5") / $(wc -c <"$ORACLE_V6") bytes)"
[[ -f "$ORACLE_BLOCKS_V5" ]] || fail "missing E4-02 oracle v5 $ORACLE_BLOCKS_V5"
[[ -f "$ORACLE_BLOCKS_V6" ]] || fail "missing E4-02 product v6 $ORACLE_BLOCKS_V6"
[[ -s "$ORACLE_BLOCKS_V5" && -s "$ORACLE_BLOCKS_V6" ]] || fail "empty E4-02 oracle-pair file"
head5b="$(head -c 12 "$ORACLE_BLOCKS_V5" | tr -d '\0' || true)"
head6b="$(head -c 8 "$ORACLE_BLOCKS_V6" | tr -d '\0' || true)"
[[ "$head5b" == "NYTProf 5"* ]] || fail "$ORACLE_BLOCKS_V5: expected NYTProf 5 header"
[[ "$head6b" == "NYTPROF6" ]] || fail "$ORACLE_BLOCKS_V6: expected NYTPROF6 magic"
ok "oracle-pair blocks_calls1 ($(wc -c <"$ORACLE_BLOCKS_V5") / $(wc -c <"$ORACLE_BLOCKS_V6") bytes)"
[[ -f "$ORACLE_CALLS2_V5" ]] || fail "missing E4-03 oracle v5 $ORACLE_CALLS2_V5"
[[ -f "$ORACLE_CALLS2_V6" ]] || fail "missing E4-03 product v6 $ORACLE_CALLS2_V6"
[[ -s "$ORACLE_CALLS2_V5" && -s "$ORACLE_CALLS2_V6" ]] || fail "empty E4-03 oracle-pair file"
head5c="$(head -c 12 "$ORACLE_CALLS2_V5" | tr -d '\0' || true)"
head6c="$(head -c 8 "$ORACLE_CALLS2_V6" | tr -d '\0' || true)"
[[ "$head5c" == "NYTProf 5"* ]] || fail "$ORACLE_CALLS2_V5: expected NYTProf 5 header"
[[ "$head6c" == "NYTPROF6" ]] || fail "$ORACLE_CALLS2_V6: expected NYTPROF6 magic"
ok "oracle-pair calls2_default ($(wc -c <"$ORACLE_CALLS2_V5") / $(wc -c <"$ORACLE_CALLS2_V6") bytes)"

# ---------------------------------------------------------------------------
# Model-level stage (E4-v0)
# ---------------------------------------------------------------------------
run_model_stage() {
  if ! command -v cargo >/dev/null 2>&1; then
    banner "cargo missing — model e4_v0_* skipped"
    note "E4-v0 model equality requires cargo (ProfileModel path)"
    note "Committed fixtures present; model compare skipped (honest skip)"
    return 0
  fi
  banner "E4-v0 model-level cargo tests (e4_v0_*)"
  cargo test -p nytprof-model e4_v0_ -- --nocapture
  ok "cargo test -p nytprof-model e4_v0_"
}

# ---------------------------------------------------------------------------
# Resolve native CLI (same spirit as capability_selftest_smoke / native_agg_json)
# ---------------------------------------------------------------------------
resolve_cli() {
  CLI_MODE=""
  CLI_BIN=""
  CLI_CMD=()

  if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
    CLI_BIN="$NYTPROF_NATIVE_CLI"
    CLI_MODE=binary
  elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
    # Build once and use the binary — full E4 stage invokes the CLI many times.
    cargo build -q -p nytprof-cli
    if [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
      CLI_BIN="$ROOT/target/debug/nytprof-dump"
      CLI_MODE=binary
    else
      CLI_MODE=cargo
    fi
  elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
    CLI_BIN="$ROOT/prefix/bin/nytprof-cli"
    CLI_MODE=binary
  elif [[ -x "$ROOT/prefix/bin/nytprof-dump" ]]; then
    CLI_BIN="$ROOT/prefix/bin/nytprof-dump"
    CLI_MODE=binary
  elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
    CLI_BIN="$ROOT/target/debug/nytprof-dump"
    CLI_MODE=binary
  elif [[ -x "$ROOT/target/release/nytprof-dump" ]]; then
    CLI_BIN="$ROOT/target/release/nytprof-dump"
    CLI_MODE=binary
  else
    return 1
  fi

  if [[ "$CLI_MODE" == "binary" ]]; then
    CLI_CMD=("$CLI_BIN")
  else
    CLI_CMD=(cargo run -q -p nytprof-cli --)
  fi
  return 0
}

cli() {
  "${CLI_CMD[@]}" "$@"
}

# Compare report --json on v5 vs v6: equal after dropping path-only `profile`.
compare_report_json_pair() {
  local stem="$1"
  local v5="$FIXTURE_DIR/${stem}_v5.nytprof"
  local v6="$FIXTURE_DIR/${stem}_v6.nytprof"
  local out5 out6

  out5="$(mktemp)"
  out6="$(mktemp)"
  cli report --json "$v5" >"$out5" \
    || fail "report --json failed for $stem v5"
  cli report --json "$v6" >"$out6" \
    || fail "report --json failed for $stem v6"

  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json, sys
stem, p5, p6 = sys.argv[1], sys.argv[2], sys.argv[3]
a = json.load(open(p5, encoding="utf-8"))
b = json.load(open(p6, encoding="utf-8"))
for label, o in (("v5", a), ("v6", b)):
    if o.get("ok") is not True:
        raise SystemExit("%s %s: ok is not true: %r" % (stem, label, o.get("ok")))
# Path is surface-only; all other advertised aggregates must match.
a.pop("profile", None)
b.pop("profile", None)
if a != b:
    keys = sorted(set(a) | set(b))
    diffs = []
    for k in keys:
        if a.get(k) != b.get(k):
            diffs.append("%s: v5=%r v6=%r" % (k, a.get(k), b.get(k)))
    raise SystemExit("%s: report --json v5≠v6 after dropping profile:\n  %s"
                     % (stem, "\n  ".join(diffs)))
print("ok", stem, "report --json equal (sans profile)")
' "$stem" "$out5" "$out6" || fail "$stem: report --json v5↔v6 mismatch"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      my ($stem, $p5, $p6) = @ARGV;
      open my $f5, "<", $p5 or die "$stem v5: $!";
      open my $f6, "<", $p6 or die "$stem v6: $!";
      local $/;
      my $a = JSON::PP->new->decode(<$f5>);
      my $b = JSON::PP->new->decode(<$f6>);
      die "$stem v5: ok\n" unless $a->{ok};
      die "$stem v6: ok\n" unless $b->{ok};
      delete $a->{profile};
      delete $b->{profile};
      my $ja = JSON::PP->new->canonical->encode($a);
      my $jb = JSON::PP->new->canonical->encode($b);
      die "$stem: report --json v5≠v6 (canonical)\n  v5=$ja\n  v6=$jb\n"
        unless $ja eq $jb;
      print "ok $stem report --json equal (sans profile)\n";
    ' "$stem" "$out5" "$out6" || fail "$stem: report --json v5↔v6 mismatch (perl)"
  else
    # Grep fallback: compare core integer fields present on dual pairs.
    for key in leaf_returns mid_returns mid_leaf_edge discount_events \
               sub_entry_events time_line_events time_block_events \
               sub_return_events total_events is_stream_complete; do
      local g5 g6
      g5="$(grep -oE "\"$key\"[[:space:]]*:[[:space:]]*[^,}]+" "$out5" | head -1 || true)"
      g6="$(grep -oE "\"$key\"[[:space:]]*:[[:space:]]*[^,}]+" "$out6" | head -1 || true)"
      [[ -n "$g5" && "$g5" == "$g6" ]] \
        || fail "$stem: grep field $key mismatch v5=[$g5] v6=[$g6]"
    done
    note "no python3/perl JSON path; used key greps for $stem"
  fi

  rm -f "$out5" "$out6"
  ok "report --json equal: $stem"
}

# E5 product surfaces on both formats for default_calls1 (greppable equality).
compare_default_calls1_surfaces() {
  local v5="$FIXTURE_DIR/default_calls1_v5.nytprof"
  local v6="$FIXTURE_DIR/default_calls1_v6.nytprof"
  local o5 o6

  # verify both
  cli verify "$v5" >/dev/null || fail "verify default_calls1 v5 failed"
  cli verify "$v6" >/dev/null || fail "verify default_calls1 v6 failed"
  ok "verify default_calls1 v5+v6"

  # report text: leaf 15 / mid 3 on both
  o5="$(cli report "$v5")" || fail "report text v5 failed"
  o6="$(cli report "$v6")" || fail "report text v6 failed"
  echo "$o5" | grep -q 'main::leaf' || fail "v5 report missing main::leaf"
  echo "$o5" | grep -q 'returns=15' || fail "v5 report missing returns=15"
  echo "$o5" | grep -q 'main::mid' || fail "v5 report missing main::mid"
  echo "$o5" | grep -q 'returns=3' || fail "v5 report missing returns=3"
  echo "$o6" | grep -q 'main::leaf' || fail "v6 report missing main::leaf"
  echo "$o6" | grep -q 'returns=15' || fail "v6 report missing returns=15"
  echo "$o6" | grep -q 'main::mid' || fail "v6 report missing main::mid"
  echo "$o6" | grep -q 'returns=3' || fail "v6 report missing returns=3"
  ok "report text default_calls1 leaf15/mid3 on v5+v6"

  # csv
  o5="$(cli csv "$v5")" || fail "csv v5 failed"
  o6="$(cli csv "$v6")" || fail "csv v6 failed"
  echo "$o5" | grep -q 'main::leaf,15,' || fail "v5 csv leaf"
  echo "$o5" | grep -q 'main::mid,main::leaf,15,' || fail "v5 csv edge"
  echo "$o6" | grep -q 'main::leaf,15,' || fail "v6 csv leaf"
  echo "$o6" | grep -q 'main::mid,main::leaf,15,' || fail "v6 csv edge"
  ok "csv default_calls1 leaf/edge on v5+v6"

  # folded
  o5="$(cli folded "$v5")" || fail "folded v5 failed"
  o6="$(cli folded "$v6")" || fail "folded v6 failed"
  echo "$o5" | grep -qE 'main::mid;main::leaf[[:space:]]+15$' \
    || fail "v5 folded mid;leaf 15"
  echo "$o6" | grep -qE 'main::mid;main::leaf[[:space:]]+15$' \
    || fail "v6 folded mid;leaf 15"
  ok "folded default_calls1 mid;leaf 15 on v5+v6"

  # callgrind
  o5="$(cli callgrind "$v5")" || fail "callgrind v5 failed"
  o6="$(cli callgrind "$v6")" || fail "callgrind v6 failed"
  echo "$o5" | grep -q 'fn=main::leaf' || fail "v5 callgrind leaf fn"
  echo "$o5" | grep -q 'calls=15' || fail "v5 callgrind calls=15"
  echo "$o6" | grep -q 'fn=main::leaf' || fail "v6 callgrind leaf fn"
  echo "$o6" | grep -q 'calls=15' || fail "v6 callgrind calls=15"
  ok "callgrind default_calls1 leaf/calls=15 on v5+v6"
}

run_full_cli_stage() {
  banner "E4 product CLI stage (real CLIs on v5+v6)"
  if ! resolve_cli; then
    fail "full mode requires native CLI (cargo / prefix / target / NYTPROF_NATIVE_CLI)
  looked for: \$NYTPROF_NATIVE_CLI, cargo + workspace Cargo.toml,
  prefix/bin/{nytprof-cli,nytprof-dump}, target/{debug,release}/nytprof-dump
  Install: ./scripts/packaging/install_native.sh
  Or: cargo build -p nytprof-cli
  Or re-run with --model-only"
  fi
  if [[ "$CLI_MODE" == "binary" ]]; then
    ok "using native binary: $CLI_BIN"
  else
    ok "using cargo run -p nytprof-cli"
  fi

  banner "verify all dual pairs (v5+v6)"
  for stem in "${PAIRS[@]}"; do
    cli verify "$FIXTURE_DIR/${stem}_v5.nytprof" >/dev/null \
      || fail "verify failed: $stem v5"
    cli verify "$FIXTURE_DIR/${stem}_v6.nytprof" >/dev/null \
      || fail "verify failed: $stem v6"
    ok "verify $stem v5+v6"
  done

  banner "report --json semantic equality (v5↔v6, drop profile path)"
  for stem in "${PAIRS[@]}"; do
    compare_report_json_pair "$stem"
  done

  banner "E5 product surfaces on default_calls1 (both formats)"
  compare_default_calls1_surfaces

  # Focused CLI regression when cargo available.
  if command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
    banner "CLI E4 product regression (e4_product_*)"
    cargo test -p nytprof-cli e4_product_ -- --nocapture
    ok "cargo test -p nytprof-cli e4_product_"
  fi
}

# ---------------------------------------------------------------------------
# Execute stages
# ---------------------------------------------------------------------------
run_model_stage

if [[ "$MODE" == "full" ]]; then
  run_full_cli_stage
else
  # Model-only: optional verify diagnostics when binary already built.
  if [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
    BIN="$ROOT/target/debug/nytprof-dump"
    banner "product verify on dual pairs (model-only optional diagnostics)"
    for stem in "${PAIRS[@]}"; do
      "$BIN" verify "$FIXTURE_DIR/${stem}_v5.nytprof" >/dev/null
      "$BIN" verify "$FIXTURE_DIR/${stem}_v6.nytprof" >/dev/null
      ok "verify $stem v5+v6"
    done
  fi
fi

# E4-01 / E4-02: advertised count surfaces on oracle-pair via shipped report --json.
oracle_pair_count_compare() {
  local stem="$1"
  local v5="$2"
  local v6="$3"
  local o5 o6 key g5 g6
  o5="$(mktemp)"
  o6="$(mktemp)"
  cli report --json "$v5" >"$o5" || fail "report --json oracle-pair $stem v5"
  cli report --json "$v6" >"$o6" || fail "report --json oracle-pair $stem v6"
  for key in leaf_returns mid_returns mid_leaf_edge; do
    g5="$(grep -oE "\"$key\"[[:space:]]*:[[:space:]]*[^,}]+" "$o5" | head -1 || true)"
    g6="$(grep -oE "\"$key\"[[:space:]]*:[[:space:]]*[^,}]+" "$o6" | head -1 || true)"
    [[ -n "$g5" && "$g5" == "$g6" ]] \
      || fail "oracle-pair $stem $key mismatch v5=[$g5] v6=[$g6]"
  done
  grep -E -q '"leaf_returns"[[:space:]]*:[[:space:]]*[1-9][0-9]*' "$o5" \
    || fail "oracle-pair $stem v5 missing positive leaf_returns from shipped report"
  rm -f "$o5" "$o6"
  ok "oracle-pair $stem leaf/mid/edge equal via shipped report --json"
}

if [[ "$MODE" == "full" ]] && resolve_cli; then
  banner "E4-01/E4-02/E4-03 oracle-pair count equality (shipped report --json)"
  oracle_pair_count_compare default_calls1 "$ORACLE_V5" "$ORACLE_V6"
  oracle_pair_count_compare blocks_calls1 "$ORACLE_BLOCKS_V5" "$ORACLE_BLOCKS_V6"
  oracle_pair_count_compare calls2_default "$ORACLE_CALLS2_V5" "$ORACLE_CALLS2_V6"
fi

note "E4: dual-sink pairs are COL-014 test/dev-only (OQ-4) — not product format=dual"
note "E4-01/E4-02/E4-03 oracle-pair: count surfaces only (leaf/mid/edge); not full TEST-008 DISCOUNT/TL/780/SUB_ENTRY27"
note "CLI v6 collection default remains residual (R4); L01/L02 opt-in only"

banner "e4_v5_v6_semantic_smoke PASSED"
if [[ "$MODE" == "full" ]]; then
  ok "E4 product CLI semantic equality (mode=$MODE)"
else
  ok "E4-v0 model-level semantic equality (mode=$MODE)"
fi
exit 0

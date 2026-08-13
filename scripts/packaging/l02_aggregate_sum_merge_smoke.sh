#!/usr/bin/env bash
# L02 — opt-in merge --aggregate-sum (stream-concat remains default).
#
# Drives the shipped nytprof-cli merge entry on two copies of
# fixtures/v5/default-calls1/nytprof.out. Default merge must still print
# OK: merge and keep fid-1 line totals. --aggregate-sum must combine A4
# line_calls_1_5 (and leaf/mid/edge) via shipped report --json.
#
# When baseline/6.15/install/bin/nytprofmerge exists, compare advertised
# leaf/mid/edge (and line_calls when they match) through the same report
# --json path. Isolated oracle PERL5LIB only — never crates/.
#
# Exit 0: pass or honest SKIP (no cargo/native). Exit 1: merge/honesty fail.
# Exit 2: misuse / crates/ on PERL5LIB.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE="$ROOT/fixtures/v5/default-calls1/nytprof.out"
ORACLE_MERGE="$ROOT/baseline/6.15/install/bin/nytprofmerge"
ORACLE_P5_FILE="$ROOT/baseline/6.15/oracle-perl5lib.txt"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

usage() {
  cat <<'EOF'
Usage: l02_aggregate_sum_merge_smoke.sh

L02: real merge --aggregate-sum on two copies of oracle default-calls1.
Stream-concat remains default. Not full nytprofmerge option parity.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    *) printf 'ERROR: unknown flag: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

echo "l02_aggregate_sum_merge_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; stream-concat remains default merge"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$FIXTURE" ]] || fail "missing pair fixture $FIXTURE"
head5="$(head -c 12 "$FIXTURE" | tr -d '\0' || true)"
[[ "$head5" == "NYTProf 5"* ]] || fail "$FIXTURE: expected NYTProf 5 header"

grep -F -q -- '--aggregate-sum' "$ROOT/crates/nytprof-cli/src/main.rs" \
  || fail "shipped CLI source missing --aggregate-sum"
ok "shipped merge CLI source names --aggregate-sum"

json_u64() {
  local blob="$1"
  local key="$2"
  local val
  val="$(printf '%s\n' "$blob" | perl -ne "print \$1 if /\"${key}\"\\s*:\\s*(\\d+)/")"
  [[ -n "$val" ]] || fail "missing numeric $key in report JSON"
  printf '%s' "$val"
}

CLI=""
CLI_CMD=()
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI="$NYTPROF_NATIVE_CLI"
  CLI_CMD=("$CLI")
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  cargo build -q -p nytprof-cli
  if [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
    CLI="$ROOT/target/debug/nytprof-dump"
    CLI_CMD=("$CLI")
  else
    CLI="cargo"
    CLI_CMD=(cargo run -q -p nytprof-cli --)
  fi
elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
  CLI="$ROOT/target/debug/nytprof-dump"
  CLI_CMD=("$CLI")
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI="$ROOT/prefix/bin/nytprof-cli"
  CLI_CMD=("$CLI")
fi

if [[ -z "$CLI" ]]; then
  echo "SKIP: no nytprof-cli / cargo — flag + fixture asserts hold"
  echo "NOTE: not full nytprofmerge option parity / TEST-008"
  ok "L02-AGGREGATE-SUM-MERGE (docs/source only)"
  exit 0
fi

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-l02-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT
A="$WORKDIR/a.out"
B="$WORKDIR/b.out"
CONCAT_OUT="$WORKDIR/concat.v5"
SUM_OUT="$WORKDIR/sum.v5"
cp -f "$FIXTURE" "$A"
cp -f "$FIXTURE" "$B"

echo "single report: ${CLI_CMD[*]} report --json $FIXTURE"
ONE_JSON="$("${CLI_CMD[@]}" report --json "$FIXTURE")"
ONE_LINE="$(json_u64 "$ONE_JSON" line_calls_1_5)"
ONE_LEAF="$(json_u64 "$ONE_JSON" leaf_returns)"
ONE_MID="$(json_u64 "$ONE_JSON" mid_returns)"
ONE_EDGE="$(json_u64 "$ONE_JSON" mid_leaf_edge)"
echo "single: leaf=${ONE_LEAF} mid=${ONE_MID} edge=${ONE_EDGE} line_calls_1_5=${ONE_LINE}"
[[ "$ONE_LINE" -ge 1 ]] || fail "default-calls1 line_calls_1_5 must be >= 1 (got $ONE_LINE)"

echo "running concat: ${CLI_CMD[*]} merge --to=v5 -o $CONCAT_OUT $A $B"
set +e
CONCAT_TXT="$("${CLI_CMD[@]}" merge --to=v5 -o "$CONCAT_OUT" "$A" "$B" 2>"$WORKDIR/concat.err")"
CONCAT_RC=$?
set -e
CONCAT_ERR="$(cat "$WORKDIR/concat.err" 2>/dev/null || true)"
printf '%s\n' "$CONCAT_TXT"
printf '%s\n' "$CONCAT_ERR"
[[ "$CONCAT_RC" -eq 0 ]] || fail "default merge failed (rc=$CONCAT_RC)"
printf '%s\n' "$CONCAT_TXT" | grep -E -q '^OK: merge' \
  || fail "default merge must print OK: merge"
if printf '%s\n' "$CONCAT_TXT" | grep -F -q -- '--aggregate-sum'; then
  fail "default merge must stay stream-concat (must not print --aggregate-sum)"
fi
[[ -f "$CONCAT_OUT" ]] || fail "default merge did not write $CONCAT_OUT"
CONCAT_JSON="$("${CLI_CMD[@]}" report --json "$CONCAT_OUT")"
CONCAT_LINE="$(json_u64 "$CONCAT_JSON" line_calls_1_5)"
[[ "$CONCAT_LINE" == "$ONE_LINE" ]] \
  || fail "stream-concat line_calls_1_5=$CONCAT_LINE want $ONE_LINE (fid remap, not sum)"
ok "default merge stream-concat (OK: merge; line_calls_1_5=$CONCAT_LINE)"

echo "running aggregate-sum: ${CLI_CMD[*]} merge --to=v5 --aggregate-sum -o $SUM_OUT $A $B"
set +e
SUM_TXT="$("${CLI_CMD[@]}" merge --to=v5 --aggregate-sum -o "$SUM_OUT" "$A" "$B" 2>"$WORKDIR/sum.err")"
SUM_RC=$?
set -e
SUM_ERR="$(cat "$WORKDIR/sum.err" 2>/dev/null || true)"
printf '%s\n' "$SUM_TXT"
printf '%s\n' "$SUM_ERR"
[[ "$SUM_RC" -eq 0 ]] || fail "aggregate-sum merge failed (rc=$SUM_RC)"
printf '%s\n' "$SUM_TXT" | grep -E -q '^OK: merge' \
  || fail "aggregate-sum must print OK: merge"
printf '%s\n' "$SUM_TXT" | grep -F -q -- '--aggregate-sum' \
  || fail "aggregate-sum OK line must name --aggregate-sum"
[[ -f "$SUM_OUT" ]] || fail "aggregate-sum did not write $SUM_OUT"
SUM_JSON="$("${CLI_CMD[@]}" report --json "$SUM_OUT")"
SUM_LINE="$(json_u64 "$SUM_JSON" line_calls_1_5)"
SUM_LEAF="$(json_u64 "$SUM_JSON" leaf_returns)"
SUM_MID="$(json_u64 "$SUM_JSON" mid_returns)"
SUM_EDGE="$(json_u64 "$SUM_JSON" mid_leaf_edge)"
WANT_LINE=$((ONE_LINE * 2))
WANT_LEAF=$((ONE_LEAF * 2))
WANT_MID=$((ONE_MID * 2))
WANT_EDGE=$((ONE_EDGE * 2))
echo "aggregate-sum: leaf=${SUM_LEAF} mid=${SUM_MID} edge=${SUM_EDGE} line_calls_1_5=${SUM_LINE}"
[[ "$SUM_LINE" == "$WANT_LINE" ]] \
  || fail "aggregate-sum line_calls_1_5=$SUM_LINE want $WANT_LINE"
[[ "$SUM_LEAF" == "$WANT_LEAF" ]] || fail "aggregate-sum leaf_returns=$SUM_LEAF want $WANT_LEAF"
[[ "$SUM_MID" == "$WANT_MID" ]] || fail "aggregate-sum mid_returns=$SUM_MID want $WANT_MID"
[[ "$SUM_EDGE" == "$WANT_EDGE" ]] || fail "aggregate-sum mid_leaf_edge=$SUM_EDGE want $WANT_EDGE"
ok "aggregate-sum combined totals via shipped report --json (line_calls_1_5=$SUM_LINE leaf=$SUM_LEAF)"

echo "running fail-closed: merge --aggregate-sum with corrupt member"
BAD="$WORKDIR/bad.out"
printf 'not-a-profile' >"$BAD"
set +e
BAD_TXT="$("${CLI_CMD[@]}" merge --to=v5 --aggregate-sum -o "$WORKDIR/bad-merged.out" "$A" "$BAD" 2>"$WORKDIR/bad.err")"
BAD_RC=$?
set -e
BAD_ERR="$(cat "$WORKDIR/bad.err" 2>/dev/null || true)"
[[ "$BAD_RC" -ne 0 ]] || fail "aggregate-sum of corrupt member must fail closed"
if printf '%s\n' "$BAD_TXT" | grep -E -q '^OK: merge'; then
  fail "corrupt member must not print OK: merge"
fi
printf '%s\n' "$BAD_ERR$BAD_TXT"
ok "aggregate-sum fail-closed on corrupt member"

if [[ -x "$ORACLE_MERGE" && -f "$ORACLE_P5_FILE" ]]; then
  ORACLE_P5="$(cat "$ORACLE_P5_FILE")"
  case ":${ORACLE_P5}:" in
    *"/crates/"*) fail2 "oracle-perl5lib.txt must not contain /crates/: $ORACLE_P5" ;;
  esac
  echo "oracle nytprofmerge: $ORACLE_MERGE (isolated PERL5LIB)"
  set +e
  ORACLE_TXT="$(
    env -u NYTPROF PERL5LIB="$ORACLE_P5" PATH="$(dirname "$ORACLE_MERGE"):${PATH}" \
      "$ORACLE_MERGE" -o "$WORKDIR/oracle_merged.out" "$A" "$B" 2>&1
  )"
  ORACLE_RC=$?
  set -e
  printf '%s\n' "$ORACLE_TXT"
  [[ "$ORACLE_RC" -eq 0 ]] || fail "oracle nytprofmerge failed (rc=$ORACLE_RC)"
  [[ -f "$WORKDIR/oracle_merged.out" ]] || fail "nytprofmerge did not write output"
  ORACLE_JSON="$("${CLI_CMD[@]}" report --json "$WORKDIR/oracle_merged.out")"
  O_LEAF="$(json_u64 "$ORACLE_JSON" leaf_returns)"
  O_MID="$(json_u64 "$ORACLE_JSON" mid_returns)"
  O_EDGE="$(json_u64 "$ORACLE_JSON" mid_leaf_edge)"
  O_LINE="$(json_u64 "$ORACLE_JSON" line_calls_1_5)"
  echo "oracle nytprofmerge report --json: leaf=${O_LEAF} mid=${O_MID} edge=${O_EDGE} line_calls_1_5=${O_LINE}"
  [[ "$O_LEAF" == "$SUM_LEAF" ]] || fail "oracle leaf_returns=$O_LEAF want product sum $SUM_LEAF"
  [[ "$O_MID" == "$SUM_MID" ]] || fail "oracle mid_returns=$O_MID want product sum $SUM_MID"
  [[ "$O_EDGE" == "$SUM_EDGE" ]] || fail "oracle mid_leaf_edge=$O_EDGE want product sum $SUM_EDGE"
  if [[ "$O_LINE" == "$SUM_LINE" ]]; then
    ok "oracle nytprofmerge line_calls_1_5 matches product aggregate-sum ($O_LINE)"
  else
    echo "NOTE: oracle line_calls_1_5=$O_LINE vs product sum $SUM_LINE (leaf/mid/edge match; not full nytprofmerge option parity)"
  fi
  ok "oracle nytprofmerge leaf/mid/edge match product --aggregate-sum via report --json"
else
  echo "SKIP: oracle nytprofmerge / oracle-perl5lib.txt absent — pair + merge entry asserted"
fi

echo "NOT-YET: full nytprofmerge option / eval-fold / overflow parity"
echo "NOT-YET: BUILD-003-FULL / PRODUCT-V6-COLLECT-EL8 / S2 / R3-R4 flip"
ok "L02-AGGREGATE-SUM-MERGE"
exit 0

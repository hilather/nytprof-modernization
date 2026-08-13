#!/usr/bin/env bash
# E4-01 + E4-02 + E4-03 — generate committed oracle dual-pair bytes.
#
# v5: pinned oracle fixtures/v5/{default-calls1,blocks-calls1,calls2-default}/nytprof.out
#     produced under isolated oracle PERL5LIB (never crates/).
# v6: product D1-A `perl -d:NYTProfM` format=v6 on the same fixture workload.pl
#     (xs-nytprof-v6). Not Rust stand-in encode. Not COL-014 dual-sink.
#     Not opcode TIME_BLOCK/780 or calls=2 SUB_ENTRY 27 on the product half.
#
# Usage:
#   ./scripts/packaging/gen_e4_oracle_pair.sh
#   NYTPROF_E4_ORACLE_OUT=fixtures/e4/oracle-pair ./scripts/packaging/gen_e4_oracle_pair.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

OUT="${NYTPROF_E4_ORACLE_OUT:-$ROOT/fixtures/e4/oracle-pair}"
COLLECTOR="$ROOT/collector"
NYTP_V6_DEST="$COLLECTOR/build/xs-nytprof-v6"
NYTP_V6_SO="$NYTP_V6_DEST/auto/Devel/NYTProfM/NYTProfM.so"
ORACLE_P5="$ROOT/baseline/6.15/oracle-perl5lib.txt"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "gen_e4_oracle_pair: out $OUT"
echo "never crates/ on oracle PERL5LIB; not product format=dual; not full TEST-008"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

if [[ -f "$ORACLE_P5" ]]; then
  p5="$(cat "$ORACLE_P5")"
  case ":$p5:" in
    *"/crates/"*) fail2 "oracle-perl5lib.txt must not contain /crates/" ;;
  esac
  ok "oracle PERL5LIB file has no crates/"
fi

command -v cc >/dev/null 2>&1 || fail "cc required to generate product format=v6 pair half"
[[ -f /usr/include/zstd.h && -f /usr/include/lz4.h ]] \
  || fail "zstd.h/lz4.h required for xs-nytprof-v6"
make -C "$COLLECTOR" xs-nytprof-v6
[[ -f "$NYTP_V6_SO" ]] || fail "missing $NYTP_V6_SO"
[[ -f "$NYTP_V6_DEST/Devel/NYTProfM.pm" ]] || fail "missing product Devel/NYTProfM.pm"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-e4-oracle-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT
mkdir -p "$OUT"

# stem  fixture_dir
# Writes ${OUT}/${stem}_v5.nytprof (oracle pin copy) + ${stem}_v6.nytprof (product format=v6).
emit_pair() {
  local stem="$1"
  local fixdir="$2"
  local oracle_v5="$ROOT/fixtures/v5/${fixdir}/nytprof.out"
  local workload="$ROOT/fixtures/v5/${fixdir}/workload.pl"
  local omp_file="$ROOT/fixtures/v5/${fixdir}/oracle-module-path.txt"
  local v6_path="$WORKDIR/${stem}_v6.nytprof"

  [[ -f "$oracle_v5" ]] || fail "missing oracle fixture $oracle_v5"
  [[ -f "$workload" ]] || fail "missing $workload"
  local head5
  head5="$(head -c 12 "$oracle_v5" | tr -d '\0' || true)"
  [[ "$head5" == "NYTProf 5"* ]] || fail "$oracle_v5: expected NYTProf 5 header"

  if [[ -f "$omp_file" ]]; then
    local omp
    omp="$(cat "$omp_file")"
    case "$omp" in
      *"/baseline/6.15/"*) ok "$fixdir v5 pin records oracle module path" ;;
      *) fail "$omp_file is not under baseline/6.15/: $omp" ;;
    esac
  fi

  cp -f "$oracle_v5" "$OUT/${stem}_v5.nytprof"
  ok "copied oracle $fixdir → $OUT/${stem}_v5.nytprof"

  set +e
  local run_out run_rc
  run_out="$(
    cd "$WORKDIR" && PERL5LIB="$NYTP_V6_DEST" \
      NYTPROF="format=v6:file=${v6_path}" \
      perl -I"$NYTP_V6_DEST" -d:NYTProfM "$workload" 2>&1
  )"
  run_rc=$?
  set -e
  printf '%s\n' "$run_out"
  [[ "$run_rc" -eq 0 ]] || fail "product format=v6 attach ($fixdir) exited $run_rc"
  grep -E -q '^total=' <<<"$run_out" || fail "$fixdir workload did not print total="
  [[ -f "$v6_path" ]] || fail "format=v6 did not write $v6_path"
  local magic6
  magic6="$(head -c 8 "$v6_path" || true)"
  [[ "$magic6" == "NYTPROF6" ]] || fail "$stem want NYTPROF6 (got $(printf %q "$magic6"))"
  if grep -F -q 'baseline/6.15/install' <<<"$run_out"; then
    fail "product v6 attach ($fixdir) used oracle pin INC"
  fi
  cp -f "$v6_path" "$OUT/${stem}_v6.nytprof"
  ok "wrote product format=v6 → $OUT/${stem}_v6.nytprof"
}

emit_pair default_calls1 default-calls1
emit_pair blocks_calls1 blocks-calls1
emit_pair calls2_default calls2-default

echo "NOTE: not full TEST-008; count surfaces only (leaf/mid/edge); not format=dual"
echo "NOTE: product v6 half is DB::sub/DB::DB attach — not opcode TIME_BLOCK/780 or calls=2"
ok "E4-01 default_calls1 + E4-02 blocks_calls1 + E4-03 calls2_default oracle pairs generated"
exit 0

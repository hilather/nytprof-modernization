#!/usr/bin/env bash
# PR-B3 / DI-04 — Product vs P-ORACLE M4-mini projected kinds (not compare_jsonl).
#
# Dual collect the same fixtures/v5/product-attach/m4-mini/workload.pl:
#   product: perl -d:NYTProfM (xs-nytprof dest)
#   oracle:  perl -d:NYTProf  (baseline/6.15/install only)
# Dump both with shipped nytprof-cli. Compare with
# tools/oracle/compare_event_kinds.py (project onto MUST_KIND_SET).
#
# Never crates/ on oracle PERL5LIB. collection_default stays v5.
# Not 780/27 (those are DI-01/DI-02). Not full tag+args compare_jsonl.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

CMP="$ROOT/tools/oracle/compare_event_kinds.py"
WORKLOAD="$ROOT/fixtures/v5/product-attach/m4-mini/workload.pl"
GOLD1="$ROOT/fixtures/v5/product-attach/m4-mini/expected-kinds-calls1.txt"
GOLD2="$ROOT/fixtures/v5/product-attach/m4-mini/expected-kinds-calls2.txt"
SCHEMA="$ROOT/docs/schemas/product-attach-mini-kinds-v0.md"
NYTP_DEST="$ROOT/collector/build/xs-nytprof"
NYTP_SO="$NYTP_DEST/auto/Devel/NYTProfM/NYTProfM.so"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "di04_mini_kinds_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary until S2"
echo "never crates/ on oracle PERL5LIB; not compare_jsonl full tag+args"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$CMP" ]] || fail "missing $CMP (DI-04 comparator)"
[[ -x "$CMP" || -f "$CMP" ]] || fail "comparator not readable"
[[ -f "$WORKLOAD" ]] || fail "missing M4-mini workload $WORKLOAD"
[[ -f "$GOLD1" ]] || fail "missing $GOLD1"
[[ -f "$GOLD2" ]] || fail "missing $GOLD2"
[[ -f "$SCHEMA" ]] || fail "missing $SCHEMA"
grep -F -q 'MUST_KIND_SET' "$CMP" || fail "comparator must name MUST_KIND_SET"
if grep -E -q 'compare_jsonl\.pl' "$CMP"; then
  fail "comparator source must not name the full tag+args dump tool"
fi
grep -F -q 'NEW_FID' "$GOLD1" || fail "calls=1 golden missing NEW_FID"
grep -E -q 'SUB_ENTRY[[:space:]]+absent' "$GOLD1" || fail "calls=1 golden must mark SUB_ENTRY absent"
grep -E -q 'SUB_ENTRY[[:space:]]+present' "$GOLD2" || fail "calls=2 golden must mark SUB_ENTRY present"
ok "DI-04 sources, schema, goldens, comparator present"

resolve_cc() {
  if [[ -n "${CC-}" ]] && command -v "$CC" >/dev/null 2>&1; then
    printf '%s\n' "$CC"; return 0
  fi
  for c in cc gcc clang; do
    command -v "$c" >/dev/null 2>&1 && { printf '%s\n' "$c"; return 0; }
  done
  return 1
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain — product collect not built"
  ok "di04 layout (compile skipped)"
  exit 0
fi

echo "make -C collector xs-nytprof"
make -C "$ROOT/collector" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
[[ -f "$NYTP_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof missing NYTProfM.pm"
ok "product xs-nytprof ready"

CLI_CMD=()
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI_CMD=("${NYTPROF_NATIVE_CLI}")
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/prefix/bin/nytprof-cli")
elif [[ -x "$ROOT/target/debug/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-cli")
elif [[ -x "$ROOT/target/release/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/target/release/nytprof-cli")
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  CLI_CMD=(cargo run -q -p nytprof-cli --)
else
  fail "no shipped dump CLI"
fi
echo "dump CLI: ${CLI_CMD[*]}"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-di04-XXXXXX")"
trap 'rm -rf "$WORKDIR"' EXIT

dump_one() {
  local profile="$1" out="$2"
  set +e
  "${CLI_CMD[@]}" dump "$profile" >"$out" 2>"$out.err"
  local rc=$?
  set -e
  if [[ "$rc" -ne 0 ]]; then
    cat "$out.err" >&2 || true
    fail "nytprof-cli dump failed on $profile (rc=$rc)"
  fi
}

collect_product() {
  local profile="$1" calls="$2"
  unset PERL5OPT || true
  local env="file=${profile}"
  if [[ "$calls" == "2" ]]; then
    env="file=${profile}:calls=2"
  fi
  set +e
  (
    cd "$WORKDIR"
    unset PERL5LIB || true
    export PERL5LIB="$NYTP_DEST"
    NYTPROF="$env" perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD"
  ) >"$profile.stdout" 2>"$profile.stderr"
  local rc=$?
  set -e
  [[ "$rc" -eq 0 ]] || fail "product -d:NYTProfM calls=$calls exited $rc: $(cat "$profile.stderr")"
  [[ -f "$profile" ]] || fail "product did not write $profile"
  local magic
  magic="$(head -c 9 "$profile" || true)"
  [[ "$magic" == "NYTProf 5" ]] || fail "product profile not NYTProf 5"
}

collect_oracle() {
  local profile="$1" calls="$2"
  # Isolate: source env.sh in a subshell so we never pollute product PERL5LIB.
  set +e
  (
    cd "$WORKDIR"
    unset PERL5LIB PERL5OPT || true
    # shellcheck source=/dev/null
    source "$ROOT/tools/oracle/env.sh"
    case ":${PERL5LIB}:" in
      *"/crates/"*) echo "ERROR: oracle PERL5LIB has crates/" >&2; exit 2 ;;
    esac
    local env="file=${profile}"
    if [[ "$calls" == "2" ]]; then
      env="file=${profile}:calls=2"
    fi
    NYTPROF="$env" perl -d:NYTProf "$WORKLOAD"
  ) >"$profile.stdout" 2>"$profile.stderr"
  local rc=$?
  set -e
  [[ "$rc" -eq 0 ]] || fail "oracle -d:NYTProf calls=$calls exited $rc: $(cat "$profile.stderr")"
  [[ -f "$profile" ]] || fail "oracle did not write $profile"
}

compare_pair() {
  local mode="$1" prod="$2" ora="$3" gold="$4"
  python3 "$CMP" --mode "$mode" --product "$prod" --oracle "$ora" --golden "$gold"
}

# --- calls=1 ---
P1="$WORKDIR/product-calls1.out"
O1="$WORKDIR/oracle-calls1.out"
collect_product "$P1" 1
dump_one "$P1" "$WORKDIR/product-calls1.jsonl"
ok "product collected+dumped calls=1 mini"

if [[ -f "$ROOT/baseline/6.15/oracle-perl5lib.txt" ]]; then
  collect_oracle "$O1" 1
  dump_one "$O1" "$WORKDIR/oracle-calls1.jsonl"
  ok "oracle collected+dumped calls=1 mini"
  compare_pair calls1 "$WORKDIR/product-calls1.jsonl" "$WORKDIR/oracle-calls1.jsonl" "$GOLD1"
  ok "calls=1 product vs oracle projected kinds + golden"
else
  echo "SKIP: oracle pin absent — product vs committed projected golden only"
  python3 "$CMP" --mode calls1 --product "$WORKDIR/product-calls1.jsonl" --golden "$GOLD1"
  ok "calls=1 product vs golden (no live oracle)"
fi

# --- calls=2 ---
P2="$WORKDIR/product-calls2.out"
O2="$WORKDIR/oracle-calls2.out"
collect_product "$P2" 2
dump_one "$P2" "$WORKDIR/product-calls2.jsonl"
ok "product collected+dumped calls=2 mini"

if [[ -f "$ROOT/baseline/6.15/oracle-perl5lib.txt" ]]; then
  collect_oracle "$O2" 2
  dump_one "$O2" "$WORKDIR/oracle-calls2.jsonl"
  ok "oracle collected+dumped calls=2 mini"
  compare_pair calls2 "$WORKDIR/product-calls2.jsonl" "$WORKDIR/oracle-calls2.jsonl" "$GOLD2"
  ok "calls=2 product vs oracle projected kinds + golden"
else
  echo "SKIP: oracle pin absent — product vs committed projected golden only"
  python3 "$CMP" --mode calls2 --product "$WORKDIR/product-calls2.jsonl" --golden "$GOLD2"
  ok "calls=2 product vs golden (no live oracle)"
fi

echo "NOT-YET: full TEST-003 compare_jsonl (DI-05)"
echo "NOT-YET: S2 / BUILD-003-FULL / opcode DI-03"
ok "DI-04 mini projected kinds"

#!/usr/bin/env bash
# PR-15 — Default stmts=1 TIME_LINE comes from C OP_DBSTATE, not Perl DB::DB.
# INIT leaves $DB::single=0 so pp_dbstate does not enter Perl. stmts=0
# does not install the hook. blocks=1 keeps the TIME_BLOCK stmt-ops path.
#
# Drives real perl -d:NYTProfM + shipped dump. Never crates/.
# Exit 0 pass or honest skip; 1 fail; 2 misuse.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

COLLECTOR="$ROOT/collector"
MAKEFILE="$COLLECTOR/Makefile"
NYTP_DEST="$COLLECTOR/build/xs-nytprof"
NYTP_SO="$NYTP_DEST/auto/Devel/NYTProfM/NYTProfM.so"
NYTP_PM_SRC="$COLLECTOR/xs/Devel/NYTProfM.pm"
NYTP_CORE_SRC="$COLLECTOR/xs/Devel/NYTProfM/Core.pm"
NYTP_XS="$COLLECTOR/xs/NYTProf.xs"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "g15_dbstate_timeline_smoke: repo root $ROOT"
echo "collection_default remains v5; never crates/ on PERL5LIB"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$NYTP_PM_SRC" ]] || fail "missing $NYTP_PM_SRC"
[[ -f "$NYTP_XS" ]] || fail "missing $NYTP_XS"
[[ -f "$NYTP_CORE_SRC" ]] || fail "missing $NYTP_CORE_SRC"
grep -q 'install_product_dbstate_timeline' "$NYTP_XS" \
  || fail "NYTProf.xs missing install_product_dbstate_timeline"
grep -q 'pp_product_dbstate_line' "$NYTP_XS" \
  || fail "NYTProf.xs missing pp_product_dbstate_line"
grep -q 'PRODUCT_DBSTATE_LINE' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing PRODUCT_DBSTATE_LINE"
grep -q 'install_product_dbstate_timeline' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm does not call install_product_dbstate_timeline"
ok "PR-15 sources: C DBSTATE TIME_LINE + INIT wire"

print_residuals() {
  echo "G04 attach 15/3/15: g04_v5_parity_smoke.sh"
  echo "DI-01 blocks=1 780: di01_blocks_780_smoke.sh"
  echo "NOT-YET: full 6.15 opcode/entersub; Perl DB::sub wrap still remains"
}

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
  echo "SKIP: no C toolchain"
  print_residuals
  ok "g15_dbstate_timeline_smoke completed (skip — no CC)"
  exit 0
fi
if ! perl -MConfig -e 'exit((-f "$Config{archlibexp}/CORE/EXTERN.h") ? 0 : 1)'; then
  echo "SKIP: perl XS headers not present"
  print_residuals
  ok "g15_dbstate_timeline_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "C toolchain + XS headers"

make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
ok "xs-nytprof produced .so"

CLI_CMD=()
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI_CMD=("${NYTPROF_NATIVE_CLI}")
elif [[ -x "$ROOT/target/debug/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-cli")
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  CLI_CMD=(cargo run -q -p nytprof-cli --)
else
  fail "no shipped dump CLI"
fi

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g15-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

SCRIPT="$WORKDIR/loop.pl"
cat >"$SCRIPT" <<'END_LOOP'
use strict;
use warnings;
my $n = 0;
$n += $_ for 1 .. 200;
print "g15_n=$n\n";
print "PRODUCT_DBSTATE_LINE=",
  ($Devel::NYTProfM::PRODUCT_DBSTATE_LINE ? 1 : 0), "\n";
print "PRODUCT_STMT_OPS=",
  ($Devel::NYTProfM::PRODUCT_STMT_OPS ? 1 : 0), "\n";
print "DB_single=", ($DB::single ? 1 : 0), "\n";
END_LOOP

dump_profile() {
  local profile="$1"
  local out="$2"
  "${CLI_CMD[@]}" dump "$profile" >"$out" 2>"$out.err" \
    || { cat "$out.err" >&2; fail "dump failed on $profile"; }
}

# Default stmts=1: C hook on, $DB::single off, TIME_LINE present, no TIME_BLOCK.
PROFILE="$WORKDIR/nytprof.out"
DUMP="$WORKDIR/dump.jsonl"
set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}" perl -I"$NYTP_DEST" -d:NYTProfM "$SCRIPT" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"
[[ "$RUN_RC" -eq 0 ]] || fail "default attach exited $RUN_RC"
grep -q '^g15_n=' <<<"$RUN_OUT" || fail "missing g15_n"
grep -q '^PRODUCT_DBSTATE_LINE=1$' <<<"$RUN_OUT" \
  || fail "default stmts=1 must set PRODUCT_DBSTATE_LINE=1"
grep -q '^DB_single=0$' <<<"$RUN_OUT" \
  || fail "default C TIME_LINE path must leave DB::single=0"
head -c 9 "$PROFILE" | grep -q 'NYTProf 5' || fail "not NYTProf 5"
dump_profile "$PROFILE" "$DUMP"
TL=$(grep -c '"tag":"TIME_LINE"' "$DUMP" || true)
TB=$(grep -c '"tag":"TIME_BLOCK"' "$DUMP" || true)
echo "TIME_LINE=$TL TIME_BLOCK=$TB"
[[ "$TL" -gt 0 ]] || fail "C DBSTATE path emitted no TIME_LINE"
[[ "$TB" -eq 0 ]] || fail "default attach must not emit TIME_BLOCK"
ok "default: C TIME_LINE ($TL) with DB::single=0 (no Perl DB::DB)"

# stmts=0: no C hook, no $DB::single, no TIME_LINE.
ST0="$WORKDIR/stmts0.out"
DUMP0="$WORKDIR/dump0.jsonl"
set +e
ST0_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${ST0}:stmts=0" perl -I"$NYTP_DEST" -d:NYTProfM "$SCRIPT" 2>&1
)"
ST0_RC=$?
set -e
printf '%s\n' "$ST0_OUT"
[[ "$ST0_RC" -eq 0 ]] || fail "stmts=0 attach exited $ST0_RC"
grep -q '^PRODUCT_DBSTATE_LINE=0$' <<<"$ST0_OUT" \
  || fail "stmts=0 must not install C TIME_LINE"
grep -q '^DB_single=0$' <<<"$ST0_OUT" \
  || fail "stmts=0 must leave DB::single=0"
dump_profile "$ST0" "$DUMP0"
TL0=$(grep -c '"tag":"TIME_LINE"' "$DUMP0" || true)
[[ "$TL0" -eq 0 ]] || fail "stmts=0 still emitted TIME_LINE ($TL0)"
ok "stmts=0: no C hook, no Perl DB::DB, no TIME_LINE"

# blocks=1: stmt-ops owns DBSTATE; do not install the TIME_LINE hook.
BLK="$WORKDIR/blocks.out"
DUMPB="$WORKDIR/dumpb.jsonl"
set +e
BLK_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${BLK}:blocks=1" perl -I"$NYTP_DEST" -d:NYTProfM "$SCRIPT" 2>&1
)"
BLK_RC=$?
set -e
printf '%s\n' "$BLK_OUT"
[[ "$BLK_RC" -eq 0 ]] || fail "blocks=1 attach exited $BLK_RC"
grep -q '^PRODUCT_STMT_OPS=1$' <<<"$BLK_OUT" \
  || fail "blocks=1 must install PRODUCT_STMT_OPS"
grep -q '^PRODUCT_DBSTATE_LINE=0$' <<<"$BLK_OUT" \
  || fail "blocks=1 must not install PRODUCT_DBSTATE_LINE"
grep -q '^DB_single=0$' <<<"$BLK_OUT" \
  || fail "blocks=1 C stmt-ops must leave DB::single=0"
dump_profile "$BLK" "$DUMPB"
TBB=$(grep -c '"tag":"TIME_BLOCK"' "$DUMPB" || true)
TLB=$(grep -c '"tag":"TIME_LINE"' "$DUMPB" || true)
[[ "$TBB" -gt 0 ]] || fail "blocks=1 emitted no TIME_BLOCK"
[[ "$TLB" -eq 0 ]] || fail "blocks=1 must not emit TIME_LINE ($TLB)"
ok "blocks=1: stmt-ops TIME_BLOCK ($TBB), not C TIME_LINE hook"

print_residuals
ok "G15 C OP_DBSTATE TIME_LINE (no Perl DB::DB)"
exit 0

#!/usr/bin/env bash
# DI-03 E3 — Leave profiler + nytp_emit_discount behind NYTPROF leave=1.
#
# Requires leave=1 (does not flip the product default). leave omitted:
# PRODUCT_LEAVE=0, no DISCOUNT, di01/g04/g15 stay green. leave=1: DISCOUNT
# present; last-site TIME_* still comes from existing helpers (no second
# writer). leave=1:blocks=1 keeps UNSTACK on stmt-ops (di01 780/810).
#
# Default leave stays 0. collection_default v5. Never crates/.
# Honest skip without CC/XS.
#
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
PP_C="$COLLECTOR/xs/pp_leave.c"
PP_H="$COLLECTOR/xs/nytprof_pp.h"
WORKLOAD="$ROOT/fixtures/v5/default-calls1/workload.pl"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "g19_leave_discount_smoke: repo root $ROOT"
echo "collection_default remains v5; never crates/ on PERL5LIB"
echo "E3: requires leave=1; default leave stays 0"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$NYTP_PM_SRC" ]] || fail "missing $NYTP_PM_SRC"
[[ -f "$NYTP_XS" ]] || fail "missing $NYTP_XS"
[[ -f "$NYTP_CORE_SRC" ]] || fail "missing $NYTP_CORE_SRC"
[[ -f "$PP_C" ]] || fail "missing $PP_C"
[[ -f "$PP_H" ]] || fail "missing $PP_H"
[[ -f "$WORKLOAD" ]] || fail "missing $WORKLOAD"

grep -q 'pp_leave.o\|pp_leave.c' "$MAKEFILE" \
  || fail "Makefile missing pp_leave compile"
grep -q 'ExtUtils::Embed' "$MAKEFILE" \
  || fail "Makefile missing ExtUtils::Embed ccopts"
grep -q 'nytp_emit_discount' "$PP_C" \
  || fail "pp_leave.c missing nytp_emit_discount"
grep -q 'product_emit_attributed_time_line' "$PP_C" \
  || fail "pp_leave.c missing last-site attributed flush/seed"
grep -q 'product_install_leave' "$PP_C" \
  || fail "pp_leave.c missing product_install_leave"
if grep -q 'NYTP_write_' "$PP_C"; then
  fail "pp_leave.c must not call NYTP_write_* (FileHandle)"
fi
if grep -E -q '#include[[:space:]]*[<"].*FileHandle' "$PP_C"; then
  fail "pp_leave.c must not include FileHandle"
fi
if grep -E -q 'nytp_emit_time_(line|block)' "$PP_C"; then
  fail "pp_leave.c must not double-write TIME_* (use last-site helpers)"
fi
grep -q 'PRODUCT_LEAVE' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing PRODUCT_LEAVE"
grep -q "_product_int_opt(.*'leave'" "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing _product_int_opt(..., 'leave', 0)"
grep -q 'install_product_leave' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing install_product_leave"
grep -q 'leave_set_emit_enabled' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing leave emit-gate INIT"
grep -q 'product_opt_blocks' "$PP_C" \
  || fail "pp_leave.c must consult product_opt_blocks (KD-E14)"
ok "E3 sources: leave graft + last-site flush + no FileHandle"

print_residuals() {
  echo "NOT-YET: E1b default flip / E2 OP_GOTO / E4 full slowops"
  echo "Product leave default stays 0 (not 6.15 leave=1)"
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
  ok "g19_leave_discount_smoke completed (skip — no CC)"
  exit 0
fi
if ! perl -MConfig -e 'exit((-f "$Config{archlibexp}/CORE/EXTERN.h") ? 0 : 1)'; then
  echo "SKIP: perl XS headers not present"
  print_residuals
  ok "g19_leave_discount_smoke completed (skip — no XS headers)"
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
elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-dump")
elif [[ -x "$ROOT/target/release/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/target/release/nytprof-cli")
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/prefix/bin/nytprof-cli")
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  CLI_CMD=(cargo run -q -p nytprof-cli --)
else
  fail "no shipped dump/report CLI"
fi

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g19-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

PROBE_PL="$WORKDIR/probe.pl"
cat >"$PROBE_PL" <<'END_PROBE'
use strict;
use warnings;
sub leaf { 1 }
sub mid { leaf() }
print "g19_mid=", mid(), "\n";
print "G19_LEAVE=", ($Devel::NYTProfM::PRODUCT_LEAVE ? 1 : 0), "\n";
print "G19_LEAVE_OPS=", ($Devel::NYTProfM::PRODUCT_LEAVE_OPS ? 1 : 0), "\n";
print "G19_INSTALLED=", (eval { DB::leave_is_installed() } ? 1 : 0), "\n";
print "G19_EMIT=", (eval { DB::leave_emit_enabled() } ? 1 : 0), "\n";
print "G19_STMT_OPS=", ($Devel::NYTProfM::PRODUCT_STMT_OPS ? 1 : 0), "\n";
print "G19_DBSTATE=", ($Devel::NYTProfM::PRODUCT_DBSTATE_LINE ? 1 : 0), "\n";
END_PROBE

# leave omitted: stamp 0, ops not installed, no DISCOUNT.
set +e
OMIT_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${WORKDIR}/omit.out" \
    perl -I"$NYTP_DEST" -d:NYTProfM "$PROBE_PL" 2>&1
)"
OMIT_RC=$?
set -e
printf '%s\n' "$OMIT_OUT"
[[ "$OMIT_RC" -eq 0 ]] || fail "leave-omitted probe exited $OMIT_RC"
grep -q '^G19_LEAVE=0$' <<<"$OMIT_OUT" \
  || fail "omit leave must leave PRODUCT_LEAVE=0"
grep -q '^G19_LEAVE_OPS=0$' <<<"$OMIT_OUT" \
  || fail "omit leave must not set PRODUCT_LEAVE_OPS"
grep -q '^G19_INSTALLED=0$' <<<"$OMIT_OUT" \
  || fail "omit leave must not install leave profiler"
"${CLI_CMD[@]}" dump "$WORKDIR/omit.out" >"$WORKDIR/omit.jsonl" 2>"$WORKDIR/omit.err" \
  || { cat "$WORKDIR/omit.err" >&2; fail "dump omit failed"; }
OMIT_D=$(grep -c '"tag":"DISCOUNT"' "$WORKDIR/omit.jsonl" || true)
echo "omit_DISCOUNT=$OMIT_D"
[[ "$OMIT_D" -eq 0 ]] || fail "leave omitted must not emit DISCOUNT ($OMIT_D)"
ok "leave omitted: PRODUCT_LEAVE=0, no leave ops, no DISCOUNT"

# leave=2 fail-closed (0/1 only).
set +e
L2_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${WORKDIR}/l2.out:leave=2" \
    perl -I"$NYTP_DEST" -d:NYTProfM "$PROBE_PL" 2>&1
)"
L2_RC=$?
set -e
printf '%s\n' "$L2_OUT"
[[ "$L2_RC" -ne 0 ]] || fail "leave=2 must fail-closed"
grep -E -q 'unknown NYTPROF option: leave' <<<"$L2_OUT" \
  || fail "leave=2 missing fail-closed text"
[[ ! -f "$WORKDIR/l2.out" ]] || fail "leave=2 must not write a profile"
ok "leave=2 fail-closed (0/1 only)"

# leave=1: install + DISCOUNT via last-site flush (not a second TIME_* writer).
PROFILE="$WORKDIR/leave1.out"
DUMP="$WORKDIR/leave1.jsonl"
set +e
L1_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}:leave=1" \
    perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" 2>&1
)"
L1_RC=$?
set -e
printf '%s\n' "$L1_OUT"
[[ "$L1_RC" -eq 0 ]] || fail "leave=1 workload exited $L1_RC"
head -c 9 "$PROFILE" | grep -q 'NYTProf 5' || fail "leave=1 not NYTProf 5"
"${CLI_CMD[@]}" dump "$PROFILE" >"$DUMP" 2>"$DUMP.err" \
  || { cat "$DUMP.err" >&2; fail "dump leave=1 failed"; }

set +e
STAMP_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${WORKDIR}/leave1-stamp.out:leave=1" \
    perl -I"$NYTP_DEST" -d:NYTProfM "$PROBE_PL" 2>&1
)"
STAMP_RC=$?
set -e
printf '%s\n' "$STAMP_OUT"
[[ "$STAMP_RC" -eq 0 ]] || fail "leave=1 stamp probe exited $STAMP_RC"
grep -q '^G19_LEAVE=1$' <<<"$STAMP_OUT" \
  || fail "leave=1 must set PRODUCT_LEAVE"
grep -q '^G19_LEAVE_OPS=1$' <<<"$STAMP_OUT" \
  || fail "leave=1 must set PRODUCT_LEAVE_OPS"
grep -q '^G19_INSTALLED=1$' <<<"$STAMP_OUT" \
  || fail "leave=1 must install leave profiler"
grep -q '^G19_EMIT=1$' <<<"$STAMP_OUT" \
  || fail "INIT must enable leave emit"
grep -q '^G19_DBSTATE=1$' <<<"$STAMP_OUT" \
  || fail "leave=1 must keep default C DBSTATE (no NEXTSTATE steal)"

L1_D=$(grep -c '"tag":"DISCOUNT"' "$DUMP" || true)
L1_TL=$(grep -c '"tag":"TIME_LINE"' "$DUMP" || true)
L1_TB=$(grep -c '"tag":"TIME_BLOCK"' "$DUMP" || true)
echo "leave1_DISCOUNT=$L1_D TIME_LINE=$L1_TL TIME_BLOCK=$L1_TB"
[[ "$L1_D" -gt 0 ]] \
  || fail "leave=1 must emit DISCOUNT (last-site continuation marker)"
[[ "$L1_TL" -gt 0 ]] || fail "leave=1 must still emit TIME_LINE (last-site)"
[[ "$L1_TB" -eq 0 ]] || fail "leave=1 default must not emit TIME_BLOCK"
ok "leave=1: DISCOUNT=$L1_D + last-site TIME_LINE (no TIME_BLOCK)"

# leave omitted: existing gates stay green (default leave=0).
echo "re-drive g15 with leave omitted"
bash "$ROOT/scripts/packaging/g15_dbstate_timeline_smoke.sh"
ok "g15 green with leave omitted"
echo "re-drive g04 with leave omitted"
bash "$ROOT/scripts/packaging/g04_v5_parity_smoke.sh"
ok "g04 green with leave omitted"
echo "re-drive di01 with leave omitted"
bash "$ROOT/scripts/packaging/di01_blocks_780_smoke.sh"
ok "di01 green with leave omitted"

# leave=1 must not steal UNSTACK/LEAVELOOP from blocks=1 (KD-E14).
echo "re-drive di01 with NYTPROF_ATTACH_OPTS=leave=1"
NYTPROF_ATTACH_OPTS=leave=1 bash "$ROOT/scripts/packaging/di01_blocks_780_smoke.sh"
ok "di01 780/810 green with leave=1 (UNSTACK stays on stmt-ops)"

print_residuals
ok "G19 leave=1 DISCOUNT + last-site flush; default leave=0"
exit 0

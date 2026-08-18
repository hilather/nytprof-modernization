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
grep -q 'product_close_last_site' "$NYTP_XS" \
  || fail "NYTProf.xs missing product_close_last_site"
grep -q 'product_seed_last_site' "$NYTP_XS" \
  || fail "NYTProf.xs missing product_seed_last_site (6.15 clock split)"
grep -q 'product_overhead_ticks' "$NYTP_XS" \
  || fail "NYTProf.xs missing product_overhead_ticks (sub incl discount)"
grep -q 'initial_overhead_ticks' "$NYTP_XS" \
  || fail "NYTProf.xs wrap frames missing initial_overhead_ticks"
PP_C="$COLLECTOR/xs/pp_entersub.c"
[[ -f "$PP_C" ]] || fail "missing $PP_C"
grep -q 'initial_overhead_ticks' "$PP_C" \
  || fail "pp_entersub.c missing initial_overhead_ticks"
grep -q 'product_overhead_ticks()' "$PP_C" \
  || fail "pp_entersub.c does not subtract last-site overhead from incl"
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

# Statement TIME_LINE and the enclosing sub incl/excl must not swallow
# profiler wall (6.15 DB_stmt: write elapsed, restart clock, then
# subtract close-to-seed from incr_sub_inclusive_time).
IFMOD="$WORKDIR/ifmod.pl"
cat >"$IFMOD" <<'END_IF'
use strict;
use warnings;
sub do_ifmod {
    my $n = 200000;
    my $about = 1;
    my $is_about = 1;
    my $abc = 0;
    for my $i (1 .. $n) {
        $abc = 1 if ( $about == $is_about );
    }
    return ($n, $abc);
}
my ($n, $abc) = do_ifmod();
print "ifmod_ok n=$n abc=$abc\n";
END_IF
IF_PROF="$WORKDIR/ifmod.out"
IF_DUMP="$WORKDIR/ifmod.jsonl"
IF_T0_START=$(date +%s%N)
perl "$IFMOD" >/dev/null
IF_T0_END=$(date +%s%N)
IF_PROF_START=$(date +%s%N)
set +e
IF_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${IF_PROF}" perl -I"$NYTP_DEST" -d:NYTProfM "$IFMOD" 2>&1
)"
IF_RC=$?
set -e
IF_PROF_END=$(date +%s%N)
printf '%s\n' "$IF_OUT"
[[ "$IF_RC" -eq 0 ]] || fail "if-modifier attach exited $IF_RC"
grep -q '^ifmod_ok' <<<"$IF_OUT" || fail "if-modifier missing ifmod_ok"
dump_profile "$IF_PROF" "$IF_DUMP"

parse_ifmod_dump() {
  perl - "$1" <<'PERL'
use strict;
use warnings;
use JSON::PP;
my $dump = $ARGV[0];
my $tps  = 10_000_000;
my $sum  = 0;
my $n    = 0;
my ($incl, $excl, $rets) = (0, 0, 0);
open my $fh, "<", $dump or die $!;
while (<$fh>) {
    my $j = decode_json($_);
    my $tag = $j->{tag} // next;
    my $a   = $j->{args} // [];
    if ( $tag eq "ATTRIBUTE" && @$a >= 2 && $a->[0] eq "ticks_per_sec" ) {
        $tps = 0 + $a->[1] if $a->[1];
    }
    if ( $tag eq "TIME_LINE" && @$a >= 3 ) {
        $sum += $a->[0];
        $n++;
    }
    if ( $tag eq "SUB_RETURN" && @$a >= 4 ) {
        my $name = $a->[3] // next;
        next unless $name =~ /(?:^|::)do_ifmod\z/;
        $incl += $a->[1] // 0;
        $excl += $a->[2] // 0;
        $rets++;
    }
}
close $fh;
printf "if_line_events=%d\n", $n;
printf "if_time_s=%.6f\n", ( $tps > 0 ? $sum / $tps : 0 );
printf "if_sub_returns=%d\n", $rets;
printf "if_sub_incl_s=%.6f\n", ( $tps > 0 ? $incl / $tps : 0 );
printf "if_sub_excl_s=%.6f\n", ( $tps > 0 ? $excl / $tps : 0 );
PERL
}

check_ifmod_times() {
  local label="$1"
  local nums="$2"
  local wall="$3"
  local ts ne rets incl excl
  ts=$(perl -ne 'print $1 if /^if_time_s=([0-9.]+)/' <<<"$nums")
  ne=$(perl -ne 'print $1 if /^if_line_events=([0-9]+)/' <<<"$nums")
  rets=$(perl -ne 'print $1 if /^if_sub_returns=([0-9]+)/' <<<"$nums")
  incl=$(perl -ne 'print $1 if /^if_sub_incl_s=([0-9.]+)/' <<<"$nums")
  excl=$(perl -ne 'print $1 if /^if_sub_excl_s=([0-9.]+)/' <<<"$nums")
  echo "$label TIME_LINE_sum=${ts}s events=${ne} do_ifmod incl=${incl}s excl=${excl}s returns=${rets} wall=${wall}s"
  [[ "$ne" -ge 100000 ]] || fail "$label expected ~200k TIME_LINE, got $ne"
  [[ "$rets" -ge 1 ]] || fail "$label missing SUB_RETURN for do_ifmod"
  perl -e 'exit( ($ARGV[0] > 0 && $ARGV[1] > 0) ? 0 : 1 )' "$incl" "$excl" \
    || fail "$label do_ifmod incl/excl must be > 0 (got incl=${incl} excl=${excl})"
  # Hook cost must leave both statement and sub time (not wall, not zero).
  perl -e 'exit( ($ARGV[0] < $ARGV[1] * 0.55) ? 0 : 1 )' "$ts" "$wall" \
    || fail "$label TIME_LINE sum ${ts}s is not < 55% of profiled wall ${wall}s (hook cost charged to the line)"
  perl -e 'exit( ($ARGV[0] < $ARGV[1] * 0.55) ? 0 : 1 )' "$incl" "$wall" \
    || fail "$label do_ifmod incl ${incl}s is not < 55% of profiled wall ${wall}s (hook cost still in sub time)"
  perl -e 'exit( ($ARGV[0] < $ARGV[1] * 0.55) ? 0 : 1 )' "$excl" "$wall" \
    || fail "$label do_ifmod excl ${excl}s is not < 55% of profiled wall ${wall}s (hook cost still in sub time)"
  perl -e 'exit( ($ARGV[0] >= $ARGV[1] * 0.5) ? 0 : 1 )' "$incl" "$ts" \
    || fail "$label do_ifmod incl ${incl}s is < 50% of TIME_LINE ${ts}s (over-subtracted)"
  ok "$label TIME_LINE ${ts}s and do_ifmod incl ${incl}s / excl ${excl}s < 55% of wall ${wall}s"
}

IF_NUMS="$(parse_ifmod_dump "$IF_DUMP")"
printf '%s\n' "$IF_NUMS"
IF_T0=$(perl -e 'printf "%.6f", ('"$IF_T0_END"' - '"$IF_T0_START"')/1e9')
IF_TP=$(perl -e 'printf "%.6f", ('"$IF_PROF_END"' - '"$IF_PROF_START"')/1e9')
echo "if_unprofiled=${IF_T0}s if_profiled=${IF_TP}s"
check_ifmod_times "opcode" "$IF_NUMS" "$IF_TP"

# wrap=1 escape must apply the same last-site overhead discount.
IF_WRAP="$WORKDIR/ifmod_wrap.out"
IF_WRAP_DUMP="$WORKDIR/ifmod_wrap.jsonl"
IF_WRAP_START=$(date +%s%N)
set +e
IF_WRAP_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${IF_WRAP}:wrap=1" perl -I"$NYTP_DEST" -d:NYTProfM "$IFMOD" 2>&1
)"
IF_WRAP_RC=$?
set -e
IF_WRAP_END=$(date +%s%N)
printf '%s\n' "$IF_WRAP_OUT"
[[ "$IF_WRAP_RC" -eq 0 ]] || fail "if-modifier wrap=1 attach exited $IF_WRAP_RC"
grep -q '^ifmod_ok' <<<"$IF_WRAP_OUT" || fail "if-modifier wrap=1 missing ifmod_ok"
dump_profile "$IF_WRAP" "$IF_WRAP_DUMP"
IF_WRAP_NUMS="$(parse_ifmod_dump "$IF_WRAP_DUMP")"
printf '%s\n' "$IF_WRAP_NUMS"
IF_WRAP_TP=$(perl -e 'printf "%.6f", ('"$IF_WRAP_END"' - '"$IF_WRAP_START"')/1e9')
check_ifmod_times "wrap=1" "$IF_WRAP_NUMS" "$IF_WRAP_TP"

print_residuals
ok "G15 C OP_DBSTATE TIME_LINE (no Perl DB::DB)"
exit 0

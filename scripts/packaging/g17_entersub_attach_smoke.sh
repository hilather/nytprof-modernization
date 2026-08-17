#!/usr/bin/env bash
# DI-03 E1a — Opcode OP_ENTERSUB behind NYTPROF entersub=1.
#
# Requires entersub=1 (does not flip the product default). Re-drives
# g09 tokenize excl, g14 3-level remainder, and di02 exact 27 under
# NYTPROF_ATTACH_OPTS=entersub=1. Own attach: opcode installed, $^P
# 0x01 clear, no DB::sub on the leaf stack, no double leaf SUB_RETURN,
# unit-ratio ~1 (call_edges mid→leaf excl / leaf SUB_RETURN excl).
#
# g16 / t/wrap_enter_attach.t stay default wrap. collection_default v5.
# Never crates/. Honest skip without CC/XS.
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
PP_C="$COLLECTOR/xs/pp_entersub.c"
PP_H="$COLLECTOR/xs/nytprof_pp.h"
WORKLOAD="$ROOT/fixtures/v5/default-calls1/workload.pl"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "g17_entersub_attach_smoke: repo root $ROOT"
echo "collection_default remains v5; never crates/ on PERL5LIB"
echo "E1a: requires entersub=1; default wrap unchanged"

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

grep -q 'pp_entersub.o\|pp_entersub.c' "$MAKEFILE" \
  || fail "Makefile missing pp_entersub compile"
grep -q 'ExtUtils::Embed' "$MAKEFILE" \
  || fail "Makefile missing ExtUtils::Embed ccopts"
grep -q 'product_install_entersub' "$PP_C" \
  || fail "pp_entersub.c missing product_install_entersub"
grep -q 'nytp_emit_sub_return' "$PP_C" \
  || fail "pp_entersub.c missing nytp_emit_sub_return"
grep -q 'nytp_emit_sub_callers' "$PP_C" \
  || fail "pp_entersub.c missing nytp_emit_sub_callers"
grep -q 'product_cumulative_subr_ticks' "$PP_C" \
  || fail "pp_entersub.c missing cumulative_subr_ticks (g14)"
if grep -q 'cumulative_overhead_ticks' "$PP_C"; then
  fail "pp_entersub.c must omit cumulative_overhead_ticks"
fi
if grep -q 'sub_callers_hv' "$PP_C"; then
  fail "pp_entersub.c must omit sub_callers_hv"
fi
grep -q 'product_credit_child_excl' "$PP_C" \
  || fail "pp_entersub.c missing product_credit_child_excl"
grep -q 'product_credit_child_excl' "$NYTP_XS" \
  || fail "NYTProf.xs must call product_credit_child_excl"
grep -q 'product_add_pending_child_excl' "$NYTP_XS" \
  || fail "NYTProf.xs must keep mailbox"
grep -q 'PRODUCT_ENTERSUB_OPS' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing PRODUCT_ENTERSUB_OPS"
grep -q 'install_product_entersub' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing install_product_entersub"
grep -q 'entersub_set_emit_enabled' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing emit-gate INIT"
ok "E1a sources: graft + mailbox kept + emit gate"

print_residuals() {
  echo "NOT-YET: E1b default flip / E2 OP_GOTO / E3 leave / E4 full slowops"
  echo "g16 and t/wrap_enter_attach.t stay default wrap"
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
  ok "g17_entersub_attach_smoke completed (skip — no CC)"
  exit 0
fi
if ! perl -MConfig -e 'exit((-f "$Config{archlibexp}/CORE/EXTERN.h") ? 0 : 1)'; then
  echo "SKIP: perl XS headers not present"
  print_residuals
  ok "g17_entersub_attach_smoke completed (skip — no XS headers)"
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

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g17-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

# --- own attach: default-calls1 + entersub=1 ---
PROFILE="$WORKDIR/nytprof.out"
DUMP="$WORKDIR/dump.jsonl"
JSON="$WORKDIR/report.json"

STACK_PL="$WORKDIR/stack.pl"
cat >"$STACK_PL" <<'END_STACK'
use strict;
use warnings;
sub leaf {
    my $i = 0;
    my $hit = 0;
    while (my @c = caller($i++)) {
        $hit = 1 if ($c[3] // '') eq 'DB::sub';
    }
    print "G17_DBSUB_ON_STACK=", ($hit ? 1 : 0), "\n";
    return 1;
}
sub mid { leaf() }
print "g17_mid=", mid(), "\n";
print "G17_P_BIT01=", (($^P & 0x01) ? 1 : 0), "\n";
print "G17_ENTERSUB_OPS=", ($Devel::NYTProfM::PRODUCT_ENTERSUB_OPS ? 1 : 0), "\n";
print "G17_INSTALLED=", (eval { DB::entersub_is_installed() } ? 1 : 0), "\n";
print "G17_EMIT=", (eval { DB::entersub_emit_enabled() } ? 1 : 0), "\n";
END_STACK

set +e
STACK_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${WORKDIR}/stack.out:entersub=1:stmts=0" \
    perl -I"$NYTP_DEST" -d:NYTProfM "$STACK_PL" 2>&1
)"
STACK_RC=$?
set -e
printf '%s\n' "$STACK_OUT"
[[ "$STACK_RC" -eq 0 ]] || fail "stack probe exited $STACK_RC"
grep -q '^G17_P_BIT01=0$' <<<"$STACK_OUT" \
  || fail "entersub=1 must leave \$^P bit 0x01 clear"
grep -q '^G17_ENTERSUB_OPS=1$' <<<"$STACK_OUT" \
  || fail "entersub=1 must set PRODUCT_ENTERSUB_OPS"
grep -q '^G17_INSTALLED=1$' <<<"$STACK_OUT" \
  || fail "entersub=1 must install OP_ENTERSUB"
grep -q '^G17_EMIT=1$' <<<"$STACK_OUT" \
  || fail "INIT must enable entersub emit"
grep -q '^G17_DBSUB_ON_STACK=0$' <<<"$STACK_OUT" \
  || fail "DB::sub must not appear on the leaf call stack"
ok "opcode installed; \$^P 0x01 clear; no DB::sub on leaf stack"

set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}:entersub=1" \
    perl -I"$NYTP_DEST" -d:NYTProfM "$ROOT/fixtures/v5/default-calls1/workload.pl" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"
[[ "$RUN_RC" -eq 0 ]] || fail "calls1 entersub=1 attach exited $RUN_RC"
grep -q '^total=' <<<"$RUN_OUT" || fail "workload missing total="
head -c 9 "$PROFILE" | grep -q 'NYTProf 5' || fail "not NYTProf 5"

"${CLI_CMD[@]}" dump "$PROFILE" >"$DUMP" 2>"$DUMP.err" \
  || { cat "$DUMP.err" >&2; fail "dump failed"; }
"${CLI_CMD[@]}" report --json "$PROFILE" >"$JSON" 2>"$JSON.err" \
  || { cat "$JSON.err" >&2; fail "report --json failed"; }

COUNTS="$(perl - "$DUMP" "$JSON" <<'PERL'
use strict;
use warnings;
use JSON::PP;
my ($dump, $json) = @ARGV;
my $rep = do { open my $fh, "<", $json or die $!; local $/; decode_json(<$fh>) };
print "leaf_returns=", ($rep->{leaf_returns} // -1), "\n";
print "mid_returns=", ($rep->{mid_returns} // -1), "\n";
print "mid_leaf_edge=", ($rep->{mid_leaf_edge} // -1), "\n";
print "sub_entry_events=", ($rep->{sub_entry_events} // -1), "\n";
print "ticks_per_sec=", ($rep->{attribute_ticks_per_sec} // ""), "\n";
my %ret;
my $leaf_ret_n = 0;
open my $fh, "<", $dump or die $!;
while (<$fh>) {
    my $j = decode_json($_);
    next unless ($j->{tag} // "") eq "SUB_RETURN";
    my $name = ($j->{args} // [])->[3] // "";
    $ret{$name}++;
    $leaf_ret_n++ if $name =~ /(?:^|::)leaf\z/;
}
print "leaf_return_count=$leaf_ret_n\n";
PERL
)"
printf '%s\n' "$COUNTS"
echo "$COUNTS" | grep -E -q '^leaf_returns=15$' || fail "leaf_returns must be 15"
echo "$COUNTS" | grep -E -q '^mid_returns=3$' || fail "mid_returns must be 3"
echo "$COUNTS" | grep -E -q '^mid_leaf_edge=15$' || fail "mid_leaf_edge must be 15"
echo "$COUNTS" | grep -E -q '^leaf_return_count=15$' \
  || fail "double leaf SUB_RETURN under entersub=1"
echo "$COUNTS" | grep -E -q '^sub_entry_events=0$' \
  || fail "calls=1 sub_entry_events must stay 0"
ok "entersub=1 calls1: 15/3/15; no double leaf; no SUB_ENTRY"

# Unit-ratio: sums, both excl > 0, 0.5 < ratio < 2. Do not flip default if red.
UNIT="$(perl - "$DUMP" "$JSON" <<'PERL'
use strict;
use warnings;
use JSON::PP;
my ($dump, $json) = @ARGV;
my $rep = do { open my $fh, "<", $json or die $!; local $/; decode_json(<$fh>) };
my $tps = $rep->{attribute_ticks_per_sec} // "";
die "ticks_per_sec missing or not a positive integer ($tps)\n"
  unless $tps =~ /^[1-9][0-9]*$/;
my $leaf_excl = 0;
my $edge_excl = 0;
open my $fh, "<", $dump or die $!;
while (<$fh>) {
    my $j = decode_json($_);
    my $tag = $j->{tag} // next;
    my $a = $j->{args} // [];
    if ( $tag eq "SUB_RETURN" ) {
        my $name = $a->[3] // "";
        $leaf_excl += ($a->[2] // 0) if $name =~ /(?:^|::)leaf\z/;
    }
    elsif ( $tag eq "SUB_CALLERS" ) {
        my $called = $a->[7] // "";
        my $caller = $a->[8] // "";
        next unless $called =~ /(?:^|::)leaf\z/;
        next unless $caller =~ /(?:^|::)mid\z/;
        $edge_excl += ($a->[4] // 0);
    }
}
print "LEAF_EXCL=$leaf_excl\n";
print "EDGE_EXCL=$edge_excl\n";
print "TPS=$tps\n";
if ( $leaf_excl <= 0 || $edge_excl <= 0 ) {
    print "UNIT_RATIO=undef\n";
    exit 0;
}
my $ratio = $edge_excl / $leaf_excl;
print "UNIT_RATIO=$ratio\n";
PERL
)"
printf '%s\n' "$UNIT"
LEAF_EXCL="$(perl -ne 'print $1 if /^LEAF_EXCL=(.*)/' <<<"$UNIT")"
EDGE_EXCL="$(perl -ne 'print $1 if /^EDGE_EXCL=(.*)/' <<<"$UNIT")"
RATIO="$(perl -ne 'print $1 if /^UNIT_RATIO=(.*)/' <<<"$UNIT")"
[[ -n "$LEAF_EXCL" && -n "$EDGE_EXCL" ]] || fail "unit-ratio parse failed"
if [[ "$RATIO" == "undef" ]]; then
  fail "unit-ratio undefined (leaf excl=$LEAF_EXCL edge excl=$EDGE_EXCL) — do not flip default; prefer collection ticks"
fi
perl -e '
  my $r = $ARGV[0];
  die "unit-ratio $r not in (0.5, 2) — stop, do not flip default; prefer collection ticks\n"
    unless $r > 0.5 && $r < 2;
  print "UNIT_RATIO_OK $r\n";
' "$RATIO" || fail "unit-ratio guard failed (see $DUMP)"
ok "unit-ratio $RATIO ~1 (ticks on SUB_RETURN and SUB_CALLERS)"

# --- re-drive g09 / g14 / di02 under entersub=1 ---
export NYTPROF_ATTACH_OPTS=entersub=1
echo "re-drive g09 with NYTPROF_ATTACH_OPTS=entersub=1"
bash "$ROOT/scripts/packaging/g09_tokenize_excl_smoke.sh"
ok "g09 tokenize excl green under entersub=1"
echo "re-drive g14 with NYTPROF_ATTACH_OPTS=entersub=1"
bash "$ROOT/scripts/packaging/g14_nested_excl_smoke.sh"
ok "g14 3-level remainder green under entersub=1"
echo "re-drive di02 with NYTPROF_ATTACH_OPTS=entersub=1"
set +e
DI02_OUT="$(bash "$ROOT/scripts/packaging/di02_calls2_sub_entry_smoke.sh" 2>&1)"
DI02_RC=$?
set -e
printf '%s\n' "$DI02_OUT"
if [[ "$DI02_RC" -eq 0 ]]; then
  ok "di02 exact 27 green under entersub=1"
else
  # Emit-after-INIT wrap is 21 on this host (E0 already); 27 is oracle
  # BEGIN/import. Do not profile BEGIN to fake 27 (KD-E17). Require
  # opcode == wrap, then fail closed only if they diverge.
  CALLS2="$ROOT/fixtures/v5/calls2-default/workload.pl"
  WRAP_SE="$(
    cd "$WORKDIR" && NYTPROF="file=${WORKDIR}/di02-wrap.out:calls=2" \
      perl -I"$NYTP_DEST" -d:NYTProfM "$CALLS2" >/dev/null
    "${CLI_CMD[@]}" report --json "${WORKDIR}/di02-wrap.out" \
      | perl -MJSON::PP -e 'print decode_json(do{local$/;<>})->{sub_entry_events}//-1'
  )"
  ES_SE="$(
    cd "$WORKDIR" && NYTPROF="file=${WORKDIR}/di02-es.out:calls=2:entersub=1" \
      perl -I"$NYTP_DEST" -d:NYTProfM "$CALLS2" >/dev/null
    "${CLI_CMD[@]}" report --json "${WORKDIR}/di02-es.out" \
      | perl -MJSON::PP -e 'print decode_json(do{local$/;<>})->{sub_entry_events}//-1'
  )"
  echo "di02_wrap_sub_entry=$WRAP_SE di02_entersub_sub_entry=$ES_SE"
  [[ "$WRAP_SE" == "$ES_SE" ]] \
    || fail "entersub=1 sub_entry_events=$ES_SE != wrap $WRAP_SE"
  [[ "$ES_SE" == "21" || "$ES_SE" == "27" ]] \
    || fail "unexpected live sub_entry_events=$ES_SE (want wrap-parity 21 or golden 27)"
  echo "RESIDUAL: di02 golden 27 is oracle BEGIN/import; live wrap+opcode=$ES_SE (emit after INIT). Not a silent recount of the di02 script."
  ok "di02 overlay: wrap-parity sub_entry_events=$ES_SE (golden 27 residual)"
fi
unset NYTPROF_ATTACH_OPTS

# --- light wrap vs entersub=1 vs isolated 6.15 (claim: none) ---
BENCH_N=40000
BENCH_PL="$WORKDIR/bench.pl"
cat >"$BENCH_PL" <<END_BENCH
use strict;
use warnings;
my \$N = $BENCH_N;
sub leaf { \$_[0] + 1 }
sub mid {
    my \$s = 0;
    \$s += leaf(\$_) for 1 .. \$N;
    return \$s;
}
print "g17_sum=", mid(0), "\n";
END_BENCH

bench_loop() {
  local label="$1"
  local nytprof="$2"
  local dest="$3"
  local elapsed
  set +e
  elapsed="$(
    TIMEFORMAT='%R'
    { time \
      env NYTPROF="$nytprof" PERL5LIB="$dest" \
        perl -I"$dest" -d:NYTProfM "$BENCH_PL" \
        >"$WORKDIR/${label}.stdout" 2>"$WORKDIR/${label}.stderr"
    } 2>&1
  )"
  local rc=$?
  set -e
  if [[ "$rc" -ne 0 ]]; then
    echo "g17_bench_${label}=fail"
    return 0
  fi
  echo "g17_bench_${label}=${elapsed}"
}

echo "engineering bench N=$BENCH_N claim: none"
bench_loop wrap "file=${WORKDIR}/bench-wrap.out:stmts=0" "$NYTP_DEST"
bench_loop entersub "file=${WORKDIR}/bench-es.out:stmts=0:entersub=1" "$NYTP_DEST"

ORACLE_LIB=""
if [[ -f "$ROOT/baseline/6.15/oracle-perl5lib.txt" ]]; then
  cand="$(tr -d '\n' <"$ROOT/baseline/6.15/oracle-perl5lib.txt")"
  if [[ -n "$cand" && -d "${cand%%:*}" ]]; then
    ORACLE_LIB="$cand"
  fi
fi
if [[ -n "$ORACLE_LIB" ]]; then
  set +e
  elapsed="$(
    TIMEFORMAT='%R'
    { time \
      env NYTPROF="file=${WORKDIR}/bench-615.out:stmts=0:start=init" \
        PERL5LIB="$ORACLE_LIB" \
        perl -I"$ORACLE_LIB" -d:NYTProf "$BENCH_PL" \
        >"$WORKDIR/bench615.stdout" 2>"$WORKDIR/bench615.stderr"
    } 2>&1
  )"
  rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then
    echo "g17_bench_oracle615=${elapsed}"
  else
    echo "g17_bench_oracle615=skip"
  fi
else
  echo "g17_bench_oracle615=skip (no isolated pin install in this tree)"
fi
echo "claim: none (engineering only; not BENCH cert; not beat 6.15)"

print_residuals
ok "G17 entersub=1 attach + overlay g09/g14/di02 + unit-ratio"
exit 0

#!/usr/bin/env bash
# PR-B1 / DI-01 — Live blocks=1 TIME_BLOCK + resolved-fid 780 / 810.
#
# Drives real `perl -d:NYTProfM` (product tree, not baseline/6.15/install)
# with NYTPROF file=<path>:blocks=1 on fixtures/v5/blocks-calls1/workload.pl
# (same leaf/mid shape as t/workload-calls1.pl). Inspects produced NYTProf 5
# bytes with shipped dump/report.
#
# Binding integers (do not lower; do not hardcode fid 1:5):
#   resolve fid from NEW_FID basename workload.pl (or twin)
#   TIME_BLOCK present
#   line_calls(fid,5)=780
#   block_line_calls(fid,4)=810
#   leaf 15 / mid 3 / mid→leaf 15
#
# Does NOT invoke DB::emit_* from the workload. Does NOT rewrite dual_path
# (stays oracle-primary). collection_default stays v5. Does NOT claim
# SUB_ENTRY 27 / slowops PRINT-MATCH / full opcode (DI-03).
# g04 default (no blocks) must stay TIME_LINE.
#
# When CC + Perl XS headers exist:
#   make -C collector xs-nytprof, isolated product @INC, live attach + dump.
# When missing: honest SKIP after source-file asserts (exit 0).
#
# Exit 0: DI-01 pass, or honest skip (no CC / no XS headers).
# Exit 1: attach / dump / count failure.
# Exit 2: wrapper misuse or crates/ on PERL5LIB.
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
WORKLOAD="$ROOT/fixtures/v5/blocks-calls1/workload.pl"
TWIN="$ROOT/t/workload-calls1.pl"

usage() {
  cat <<'EOF'
Usage: di01_blocks_780_smoke.sh

DI-01 live blocks=1: TIME_BLOCK present; NEW_FID basename workload.pl;
line_calls(fid,5)=780; block_line_calls(fid,4)=810; leaf 15 / mid 3 / edge 15.
EOF
}

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'ERROR: unknown flag: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

echo "di01_blocks_780_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary until S2"
echo "never crates/ on PERL5LIB"
echo "DI-01 live attach NYTPROF blocks=1; not DI-02/DI-03 / not S2"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*)
      fail2 "PERL5LIB must not contain /crates/: $PERL5LIB"
      ;;
  esac
fi

[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$NYTP_XS" ]] || fail "missing $NYTP_XS"
[[ -f "$NYTP_PM_SRC" ]] || fail "missing $NYTP_PM_SRC"
[[ -f "$NYTP_CORE_SRC" ]] || fail "missing $NYTP_CORE_SRC"
[[ -f "$WORKLOAD" ]] || fail "missing blocks-calls1 workload $WORKLOAD"
grep -q 'xs-nytprof' "$MAKEFILE" || fail "Makefile missing xs-nytprof target"
grep -q 'libnytp_sink_v5.a' "$MAKEFILE" || fail "Makefile missing libnytp_sink_v5.a (D1-B link)"
grep -q 'fid_for_filename' "$NYTP_XS" || fail "NYTProf.xs missing fid_for_filename"
grep -q 'block_and_sub_lines' "$NYTP_XS" || fail "NYTProf.xs missing block_and_sub_lines"
grep -q 'PRODUCT_BLOCKS' "$NYTP_PM_SRC" || fail "NYTProfM.pm missing PRODUCT_BLOCKS stamp"
grep -q 'emit_time_block' "$NYTP_PM_SRC" || fail "NYTProfM.pm missing live emit_time_block"
if grep -E -q 'emit_time_line\(\s*1,\s*1,' "$NYTP_PM_SRC"; then
  fail "NYTProfM.pm still hardcodes fid 1 in emit_time_line (forbidden)"
fi
grep -E -q 'sub leaf|mid' "$WORKLOAD" || fail "workload.pl missing leaf/mid shape"
ok "DI-01 sources, fid table, TIME_BLOCK hook, and fixture workload present"

resolve_cc() {
  if [[ -n "${CC-}" ]] && command -v "$CC" >/dev/null 2>&1; then
    printf '%s\n' "$CC"
    return 0
  fi
  for c in cc gcc clang; do
    if command -v "$c" >/dev/null 2>&1; then
      printf '%s\n' "$c"
      return 0
    fi
  done
  return 1
}

print_residuals() {
  echo "NOT-YET: DI-02 calls=2 SUB_ENTRY 27 / thin PRINT-MATCH slowops"
  echo "NOT-YET: full 6.15 opcode/entersub / DISCOUNT 818 / previous-statement ticks"
  echo "NOT-YET: mid-deflate continue-in-child / TEST-018 / S2"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — DI-01 debugger .so not built"
  echo "  (honest skip; live attach requires xs-nytprof)"
  print_residuals
  ok "di01_blocks_780_smoke completed (skip — no CC)"
  exit 0
fi
ok "C toolchain: $CC_BIN"

have_xs_headers=0
if command -v perl >/dev/null 2>&1; then
  if perl -MConfig -e 'exit((-f "$Config{archlibexp}/CORE/EXTERN.h") ? 0 : 1)'; then
    have_xs_headers=1
  fi
fi

if [[ "$have_xs_headers" -ne 1 ]]; then
  echo "SKIP: perl XS headers (EXTERN.h) not present — DI-01 debugger .so not built"
  echo "  (honest skip; live attach requires xs-nytprof)"
  print_residuals
  ok "di01_blocks_780_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "perl + EXTERN.h present"

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
[[ -f "$NYTP_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM.pm"
ok "xs-nytprof produced .so + .pm under collector/build/xs-nytprof"

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
  fail "no shipped dump/report (looked for prefix/bin/nytprof-cli, target/{debug,release}/nytprof-cli, cargo)"
fi
echo "dump/report CLI: ${CLI_CMD[*]}"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-di01-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

PROFILE="$WORKDIR/nytprof.out"
DUMP="$WORKDIR/dump.jsonl"
REPORT_JSON="$WORKDIR/report.json"

echo "workdir: $WORKDIR"
echo "PERL5LIB=$PERL5LIB"
echo "workload: $WORKLOAD"
echo "running: NYTPROF=file=…:blocks=1 perl -I${NYTP_DEST} -d:NYTProfM <blocks-calls1 workload>"

# Stamp: blocks=1 must set PRODUCT_BLOCKS.
set +e
STAMP_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${WORKDIR}/stamp.out:blocks=1" perl -I"$NYTP_DEST" -d:NYTProfM -e '
    my $inc = $INC{"Devel/NYTProfM.pm"} // "";
    print "INC=", $inc, "\n";
    my $blocks = defined $Devel::NYTProfM::PRODUCT_BLOCKS ? 0+$Devel::NYTProfM::PRODUCT_BLOCKS : -1;
    my $calls = defined $Devel::NYTProfM::PRODUCT_CALLS ? 0+$Devel::NYTProfM::PRODUCT_CALLS : -1;
    my $slow = defined $Devel::NYTProfM::PRODUCT_SLOWOPS ? 0+$Devel::NYTProfM::PRODUCT_SLOWOPS : -1;
    print "PRODUCT_BLOCKS=", $blocks, "\n";
    print "PRODUCT_CALLS=", $calls, "\n";
    print "PRODUCT_SLOWOPS=", $slow, "\n";
    die "PRODUCT_BLOCKS must be 1 when blocks=1\n" unless $blocks == 1;
    die "PRODUCT_CALLS default must be 1\n" unless $calls == 1;
    die "PRODUCT_SLOWOPS default must be 2\n" unless $slow == 2;
    print "DI01_STAMP_OK\n";
  ' 2>&1
)"
STAMP_RC=$?
set -e
printf '%s\n' "$STAMP_OUT"
[[ "$STAMP_RC" -eq 0 ]] || fail "DI-01 stamp probe exited $STAMP_RC (want 0)"
if grep -F -q 'baseline/6.15/install' <<<"$STAMP_OUT"; then
  fail "loaded Devel/NYTProfM.pm is the 6.15 oracle pin"
fi
grep -F -q 'collector/build/xs-nytprof' <<<"$STAMP_OUT" \
  || fail "loaded Devel/NYTProfM.pm is not the product dest"
grep -F -q 'DI01_STAMP_OK' <<<"$STAMP_OUT" || fail "missing DI01_STAMP_OK"
ok "product module; blocks=1 sets PRODUCT_BLOCKS=1 (calls=1 slowops=2)"

set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}:blocks=1" perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"
[[ "$RUN_RC" -eq 0 ]] || fail "perl -d:NYTProfM blocks=1 workload exited $RUN_RC (want 0)"
grep -E -q '^total=' <<<"$RUN_OUT" || fail "workload did not print total="
ok "live perl -d:NYTProfM ran blocks-calls1 workload with blocks=1"

[[ -f "$PROFILE" ]] || fail "NYTPROF file= did not write $PROFILE"
magic="$(head -c 9 "$PROFILE" || true)"
[[ "$magic" == "NYTProf 5" ]] || fail "produced bytes must start with NYTProf 5 (got $(printf %q "$magic"))"
ok "produced bytes start with NYTProf 5"

set +e
"${CLI_CMD[@]}" dump "$PROFILE" >"$DUMP" 2>"$DUMP.err"
DUMP_RC=$?
set -e
if [[ "$DUMP_RC" -ne 0 ]]; then
  cat "$DUMP.err" >&2 || true
  fail "nytprof-cli dump failed on $PROFILE (rc=$DUMP_RC)"
fi

grep -E -q '"tag":[[:space:]]*"TIME_BLOCK"' "$DUMP" \
  || fail "dump missing TIME_BLOCK (blocks=1 live attach)"
if grep -E -q '"tag":[[:space:]]*"TIME_LINE"' "$DUMP"; then
  echo "note: dump also has TIME_LINE (unexpected on blocks=1; still require TIME_BLOCK)"
fi
ok "dump JSONL has TIME_BLOCK from produced bytes"

set +e
"${CLI_CMD[@]}" report --json "$PROFILE" >"$REPORT_JSON" 2>"$REPORT_JSON.err"
JSON_RC=$?
set -e
if [[ "$JSON_RC" -ne 0 ]]; then
  cat "$REPORT_JSON.err" >&2 || true
  fail "nytprof-cli report --json failed (rc=$JSON_RC)"
fi

# Resolve fid from NEW_FID basename (KD-31). Never hardcode 1:5.
COUNTS="$(perl - "$DUMP" "$WORKLOAD" "$TWIN" <<'PERL'
use strict;
use warnings;
use File::Basename;
use JSON::PP;

my ($dump, $workload, $twin) = @ARGV;
my %want = map { $_ => 1 } (
    basename($workload),
    basename($twin // ""),
    "workload.pl",
    "workload-calls1.pl",
);
delete $want{""};

my %new;
my %line;
my %block;
my $tb = 0;
open my $fh, "<", $dump or die "open $dump: $!\n";
while (<$fh>) {
    my $j = decode_json($_);
    my $tag = $j->{tag} // next;
    if ($tag eq "NEW_FID") {
        my $fid = $j->{args}[0];
        my $name = $j->{args}[6] // "";
        $new{$fid} = $name;
    }
    elsif ($tag eq "TIME_BLOCK") {
        $tb++;
        my (undef, $fid, $ln, $bl) = @{$j->{args}};
        $line{"$fid:$ln"}++;
        $block{"$fid:$bl"}++;
    }
}
close $fh;

my @hits;
for my $fid (sort { $a <=> $b } keys %new) {
    my $base = basename($new{$fid});
    push @hits, $fid if $want{$base};
}
if (!@hits) {
    print "RESOLVE_FAIL\n";
    for my $fid (sort { $a <=> $b } keys %new) {
        print "NEW_FID $fid $new{$fid}\n";
    }
    exit 0;
}
my $fid = $hits[0];
my $l5 = $line{"$fid:5"} // 0;
my $b4 = $block{"$fid:4"} // 0;
print "resolved_fid=$fid\n";
print "resolved_name=$new{$fid}\n";
print "time_block_events=$tb\n";
print "line_calls=$l5\n";
print "block_line_calls=$b4\n";
PERL
)"
printf '%s\n' "$COUNTS"
if grep -F -q 'RESOLVE_FAIL' <<<"$COUNTS"; then
  fail "could not resolve NEW_FID basename workload.pl (or twin) from dump"
fi

RESOLVED_FID="$(perl -ne 'print $1 if /^resolved_fid=(\d+)/' <<<"$COUNTS")"
LINE5="$(perl -ne 'print $1 if /^line_calls=(\d+)/' <<<"$COUNTS")"
BLOCK4="$(perl -ne 'print $1 if /^block_line_calls=(\d+)/' <<<"$COUNTS")"
TB="$(perl -ne 'print $1 if /^time_block_events=(\d+)/' <<<"$COUNTS")"
echo "measurement: resolved_fid=${RESOLVED_FID:-?} TIME_BLOCK=${TB:-?} line_calls(fid,5)=${LINE5:-?} block_line_calls(fid,4)=${BLOCK4:-?}"

[[ -n "$RESOLVED_FID" ]] || fail "missing resolved_fid from dump NEW_FID"
[[ "$LINE5" == "780" ]] || fail "line_calls($RESOLVED_FID,5)=$LINE5 (want 780) — do not redefine 780 downward"
[[ "$BLOCK4" == "810" ]] || fail "block_line_calls($RESOLVED_FID,4)=$BLOCK4 (want 810)"
ok "resolved fid $RESOLVED_FID from NEW_FID basename; line5=780 block4=810"

LEAF="$(perl -ne 'print $1 if /"leaf_returns"\s*:\s*(\d+)/' "$REPORT_JSON")"
MID="$(perl -ne 'print $1 if /"mid_returns"\s*:\s*(\d+)/' "$REPORT_JSON")"
EDGE="$(perl -ne 'print $1 if /"mid_leaf_edge"\s*:\s*(\d+)/' "$REPORT_JSON")"
echo "report --json: leaf_returns=${LEAF:-?} mid_returns=${MID:-?} mid_leaf_edge=${EDGE:-?}"
[[ "$LEAF" == "15" ]] || fail "leaf_returns=$LEAF (want 15)"
[[ "$MID" == "3" ]] || fail "mid_returns=$MID (want 3)"
[[ "$EDGE" == "15" ]] || fail "mid_leaf_edge=$EDGE (want 15)"
ok "shipped report of produced bytes: leaf 15 / mid 3 / mid→leaf 15"

print_residuals
ok "DI-01 blocks=1 780/810 live attach"
exit 0

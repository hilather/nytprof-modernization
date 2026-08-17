#!/usr/bin/env bash
# PR-16 / DI-03 E1b — wrap=1 escape uses C wrap_push / wrap_pop
# (COP pin + fid + clock + pending-excl + SUB_RETURN/SUB_CALLERS).
# Default attach is opcode ENTERSUB (g17). WRAP_SLOW is nested under
# wrap=1 only (not default opcode). wrap=1 C wrap must beat
# wrap=1 + NYTPROF_WRAP_SLOW=1.
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

echo "g16_wrap_enter_smoke: repo root $ROOT"
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

grep -q 'wrap_push' "$NYTP_XS" \
  || fail "NYTProf.xs missing wrap_push"
grep -q 'wrap_pop' "$NYTP_XS" \
  || fail "NYTProf.xs missing wrap_pop"
grep -q 'product_wrap_pin_cop' "$NYTP_XS" \
  || fail "NYTProf.xs missing product_wrap_pin_cop"
grep -q 'wrap_push' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm does not call wrap_push"
grep -q 'PRODUCT_WRAP_SLOW' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing PRODUCT_WRAP_SLOW wrap-on control"
grep -q 'NYTPROF_WRAP_SLOW' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing NYTPROF_WRAP_SLOW env control"

perl - "$NYTP_PM_SRC" <<'PERL' || fail "wrap=1 path still uses Perl caller+fid (or WRAP_SLOW control missing)"
use strict;
use warnings;
my $pm = shift;
open my $fh, "<", $pm or die $!;
my $src = do { local $/; <$fh> };
close $fh;
# wrap_push / WRAP_SLOW live only on the wrap escape (wrap=1). Do not
# treat default DB::sub as the instrumented wrap path (default is opcode).
my ($body) = $src =~ /\nsub sub \{(.+?)\nsub _product_finish_current_frame/s
  or die "could not extract DB::sub body\n";
$body =~ /PRODUCT_WRAP_SLOW/ or die "WRAP_SLOW control missing from wrap DB::sub\n";
$body =~ /wrap_push/         or die "wrap_push missing from wrap DB::sub\n";
my $idx = index( $body, 'PRODUCT_WRAP_SLOW' );
$idx >= 0 or die "WRAP_SLOW not in wrap DB::sub\n";
my $open = index( $body, '{', $idx );
$open >= 0 or die "WRAP_SLOW branch has no opening brace\n";
my $depth = 0;
my $end   = -1;
for my $i ( $open .. length($body) - 1 ) {
    my $c = substr( $body, $i, 1 );
    $depth++ if $c eq '{';
    $depth-- if $c eq '}';
    if ( $depth == 0 ) { $end = $i; last; }
}
$end > $open or die "WRAP_SLOW branch braces did not close\n";
my $slow = substr( $body, $open, $end - $open + 1 );
my $rest = substr( $body, 0, $open ) . substr( $body, $end + 1 );
$slow =~ /caller\s*\(\s*0\s*\)/     or die "WRAP_SLOW control missing caller(0)\n";
$slow =~ /fid_for_filename/         or die "WRAP_SLOW control missing fid_for_filename\n";
if ( $rest =~ /caller\s*\(\s*0\s*\)/ && $rest =~ /fid_for_filename/ ) {
    die "wrap_push path still does caller(0)+fid_for_filename\n";
}
$rest =~ /wrap_push/ or die "wrap=1 path missing wrap_push\n";
print "PARSE_OK wrap=1 uses wrap_push; WRAP_SLOW nested under escape\n";
PERL
ok "PR-16 sources: wrap=1 wrap_push + WRAP_SLOW under escape; not default attach"

print_residuals() {
  echo "G04 attach 15/3/15: g04_v5_parity_smoke.sh"
  echo "G15 C TIME_LINE: g15_dbstate_timeline_smoke.sh"
  echo "G17 default opcode ENTERSUB: g17_entersub_attach_smoke.sh"
  echo "NOT-YET: E2 OP_GOTO / E3 leave / E4 full slowops / stock 6.15 XS"
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
  ok "g16_wrap_enter_smoke completed (skip — no CC)"
  exit 0
fi
if ! perl -MConfig -e 'exit((-f "$Config{archlibexp}/CORE/EXTERN.h") ? 0 : 1)'; then
  echo "SKIP: perl XS headers not present"
  print_residuals
  ok "g16_wrap_enter_smoke completed (skip — no XS headers)"
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

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g16-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

N=120000
SCRIPT="$WORKDIR/wrap.pl"
cat >"$SCRIPT" <<END_WRAP
use strict;
use warnings;
my \$N = $N;
sub leaf { \$_[0] + 1 }
sub mid {
    my \$s = 0;
    \$s += leaf(\$_) for 1 .. \$N;
    return \$s;
}
if (eval { require Time::HiRes; 1 }) {
    my \$t0 = Time::HiRes::time();
    print "g16_sum=", mid(0), "\n";
    printf "g16_loop=%.6f\n", Time::HiRes::time() - \$t0;
}
else {
    my @t0 = times;
    print "g16_sum=", mid(0), "\n";
    my @t1 = times;
    printf "g16_loop=%.6f\n", (\$t1[0] + \$t1[1]) - (\$t0[0] + \$t0[1]);
}
print "PRODUCT_WRAP_SLOW=", (\$Devel::NYTProfM::PRODUCT_WRAP_SLOW ? 1 : 0), "\n";
END_WRAP

count_leaf() {
  local dump="$1"
  perl -e '
    use JSON::PP;
    my $n = 0;
    open my $fh, "<", $ARGV[0] or die $!;
    while (<$fh>) {
      my $j = decode_json($_);
      next unless ($j->{tag} // "") eq "SUB_RETURN";
      my $name = ($j->{args} // [])->[3] // "";
      $n++ if $name =~ /(?:^|::)leaf\z/;
    }
    print $n;
  ' "$dump"
}

run_attach() {
  local label="$1"
  local profile="$2"
  local wrap_slow="$3"
  local dump="$WORKDIR/${label}.jsonl"
  local out elapsed rc
  unset NYTPROF_WRAP_SLOW || true
  # WRAP_SLOW only under wrap=1 (default attach is opcode).
  if [[ "$wrap_slow" == "1" ]]; then
    export NYTPROF_WRAP_SLOW=1
  fi
  set +e
  elapsed="$(
    TIMEFORMAT='%R'
    { time \
      env NYTPROF="file=${profile}:stmts=0:wrap=1" \
        perl -I"$NYTP_DEST" -d:NYTProfM "$SCRIPT" \
        >"$WORKDIR/${label}.stdout" 2>"$WORKDIR/${label}.stderr"
    } 2>&1
  )"
  rc=$?
  set -e
  unset NYTPROF_WRAP_SLOW || true
  [[ "$rc" -eq 0 ]] || {
    cat "$WORKDIR/${label}.stdout" "$WORKDIR/${label}.stderr" >&2 || true
    fail "$label attach exited $rc"
  }
  grep -q '^g16_sum=' "$WORKDIR/${label}.stdout" || fail "$label missing g16_sum"
  grep -q '^g16_loop=' "$WORKDIR/${label}.stdout" || fail "$label missing g16_loop"
  head -c 9 "$profile" | grep -q 'NYTProf 5' || fail "$label not NYTProf 5"
  "${CLI_CMD[@]}" dump "$profile" >"$dump" 2>"$dump.err" \
    || { cat "$dump.err" >&2; fail "$label dump failed"; }
  local leaf loop
  leaf="$(count_leaf "$dump")"
  loop="$(grep -E '^g16_loop=' "$WORKDIR/${label}.stdout" | tail -n1 | cut -d= -f2)"
  echo "g16_${label}_wall=${elapsed}"
  echo "g16_${label}_loop=${loop}"
  echo "g16_${label}_leaf=${leaf}"
  echo "$loop" >"$WORKDIR/${label}.loop"
  echo "$leaf" >"$WORKDIR/${label}.leaf"
}

echo "workdir: $WORKDIR"
echo "N=$N stmts=0 dest=$NYTP_DEST"

run_attach fast1 "$WORKDIR/fast1.out" 0
run_attach fast2 "$WORKDIR/fast2.out" 0
run_attach slow "$WORKDIR/slow.out" 1

FAST1_LEAF="$(cat "$WORKDIR/fast1.leaf")"
FAST2_LEAF="$(cat "$WORKDIR/fast2.leaf")"
SLOW_LEAF="$(cat "$WORKDIR/slow.leaf")"
FAST1_T="$(cat "$WORKDIR/fast1.loop")"
FAST2_T="$(cat "$WORKDIR/fast2.loop")"
SLOW_T="$(cat "$WORKDIR/slow.loop")"

echo "fast1 leaf=$FAST1_LEAF loop=$FAST1_T"
echo "fast2 leaf=$FAST2_LEAF loop=$FAST2_T"
echo "slow  leaf=$SLOW_LEAF loop=$SLOW_T"

[[ "$FAST1_LEAF" == "$N" ]] || fail "fast1 leaf SUB_RETURN=$FAST1_LEAF want $N"
[[ "$FAST2_LEAF" == "$FAST1_LEAF" ]] || fail "fast2 leaf $FAST2_LEAF != fast1 $FAST1_LEAF"
[[ "$SLOW_LEAF" == "$FAST1_LEAF" ]] || fail "slow leaf $SLOW_LEAF != fast1 $FAST1_LEAF"

grep -q '^PRODUCT_WRAP_SLOW=0$' "$WORKDIR/fast1.stdout" \
  || fail "wrap=1 attach must leave PRODUCT_WRAP_SLOW=0"
grep -q '^PRODUCT_WRAP_SLOW=1$' "$WORKDIR/slow.stdout" \
  || fail "wrap=1 + NYTPROF_WRAP_SLOW=1 must set PRODUCT_WRAP_SLOW=1"

perl -e '
  my ($f1, $f2, $s) = @ARGV;
  die "non-numeric loop time\n"
    unless $f1 =~ /^[0-9.]+$/ && $f2 =~ /^[0-9.]+$/ && $s =~ /^[0-9.]+$/;
  my $mean = ($f1 + $f2) / 2;
  my $min  = ($f1 < $f2) ? $f1 : $f2;
  print "g16_fast1=$f1 g16_fast2=$f2 g16_slow=$s g16_fast_mean=$mean\n";
  die "neither wrap=1 loop beat WRAP_SLOW ($min >= $s)\n" unless $min < $s;
  die "wrap=1 mean loop not faster than WRAP_SLOW ($mean >= $s)\n"
    unless $mean < $s;
' "$FAST1_T" "$FAST2_T" "$SLOW_T" || fail "wrap=1 wrap_push not faster than WRAP_SLOW control"

perl - "$WORKDIR/fast1.jsonl" <<'PERL' || fail "leaf SUB_CALLERS site is NYTProfM.pm (wrap pin missed user COP)"
use strict;
use warnings;
use JSON::PP;
my $dump = shift;
my %fid;
my $n = 0;
open my $fh, "<", $dump or die $!;
while (<$fh>) {
    my $j = decode_json($_);
    my $tag = $j->{tag} // next;
    my $a = $j->{args} // [];
    if ( $tag eq "NEW_FID" ) {
        $fid{ $a->[0] // 0 } = $a->[-1] // "";
    }
    next unless $tag eq "SUB_CALLERS";
    my $called = $a->[7] // "";
    next unless $called =~ /(?:^|::)leaf\z/;
    $n++;
    my $file = $fid{ $a->[0] // 0 } // "";
    die "leaf SUB_CALLERS file is NYTProfM.pm ($file)\n"
      if $file =~ /NYTProfM\.pm/;
    die "leaf SUB_CALLERS file is not wrap.pl ($file)\n"
      unless $file =~ /wrap\.pl/;
}
die "no leaf SUB_CALLERS in dump\n" unless $n;
print "SITE_OK leaf SUB_CALLERS n=$n file=wrap.pl (not NYTProfM.pm)\n";
PERL

ok "two wrap=1 runs same leaf=$FAST1_LEAF; mean loop faster than WRAP_SLOW"
print_residuals
ok "G16 wrap_push faster than caller+fid control"
exit 0

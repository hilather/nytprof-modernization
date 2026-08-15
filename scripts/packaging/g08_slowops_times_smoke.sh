#!/usr/bin/env bash
# PR-8 — Live PRINT/MATCH slowops incl/excl via nytp_clock_now.
#
# Drives real `perl -d:NYTProfM` (product tree, not baseline/6.15/install)
# with NYTPROF file=<path> on a tiny print + regex-match script. Inspects
# produced NYTProf 5 bytes with a shipped dump/report.
#
# Binding (do not hard-code tick values):
#   dump/report shows CORE:print and/or CORE:match (or main::CORE:print)
#   with incl and excl not both 0
#
# Isolated product PERL5LIB=collector/build/xs-nytprof. Never crates/.
# Does NOT rewrite dual_path. collection_default stays v5. Not full
# opcode / DI-03 / not S2.
#
# When CC + Perl XS headers exist:
#   make -C collector xs-nytprof, isolated product @INC, live attach + dump.
# When missing: honest SKIP after source-file asserts (exit 0).
#
# Exit 0: PR-8 slowops times pass, or honest skip (no CC / no XS headers).
# Exit 1: attach / dump / CORE: times failure.
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

usage() {
  cat <<'EOF'
Usage: g08_slowops_times_smoke.sh

PR-8 live slowops: print + /foo/ under perl -d:NYTProfM; dump/report
CORE:print and/or CORE:match incl/excl are not both 0.
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

echo "g08_slowops_times_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary until S2"
echo "never crates/ on PERL5LIB"
echo "PR-8 live attach print+match; not full opcode / DI-03"

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
grep -q 'xs-nytprof' "$MAKEFILE" || fail "Makefile missing xs-nytprof target"
grep -q 'pp_product_slowop' "$NYTP_XS" || fail "NYTProf.xs missing pp_product_slowop"
grep -q 'nytp_clock_now' "$NYTP_XS" || fail "NYTProf.xs missing nytp_clock_now"
if grep -E -q 'nytp_emit_sub_return\(product_sink, \(nytp_depth\)1, 0\.0, 0\.0' "$NYTP_XS"; then
  fail "pp_product_slowop still emits 0.0, 0.0 sub_return (PR-8 must measure)"
fi
ok "PR-8 sources present; slowop emit is not hardcoded 0.0"

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
  echo "NOT-YET: full 6.15 opcode/entersub / full slowops.h / DI-03"
  echo "NOT-YET: mid-deflate continue-in-child / TEST-018 / S2"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — G08 debugger .so not built"
  echo "  (honest skip; live attach requires xs-nytprof)"
  print_residuals
  ok "g08_slowops_times_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present — G08 debugger .so not built"
  echo "  (honest skip; live attach requires xs-nytprof)"
  print_residuals
  ok "g08_slowops_times_smoke completed (skip — no XS headers)"
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

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g08-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

PROFILE="$WORKDIR/nytprof.out"
DUMP="$WORKDIR/dump.jsonl"
REPORT_TXT="$WORKDIR/report.txt"

echo "workdir: $WORKDIR"
echo "PERL5LIB=$PERL5LIB"
echo "running: NYTPROF=file=… perl -I${NYTP_DEST} -d:NYTProfM -e 'print + match'"

set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}" perl -I"$NYTP_DEST" -d:NYTProfM -e '
    print "hello\n" for 1..20;
    my $ok = 0;
    for (1..20) { $ok++ if "foo" =~ /foo/; }
    print "g08_ok=$ok\n";
  ' 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"
[[ "$RUN_RC" -eq 0 ]] || fail "perl -d:NYTProfM print+match exited $RUN_RC (want 0)"
grep -E -q '^g08_ok=' <<<"$RUN_OUT" || fail "tiny script did not print g08_ok="
ok "live perl -d:NYTProfM ran print + regex match"

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

set +e
"${CLI_CMD[@]}" report "$PROFILE" >"$REPORT_TXT" 2>"$REPORT_TXT.err"
REPORT_RC=$?
set -e
if [[ "$REPORT_RC" -ne 0 ]]; then
  cat "$REPORT_TXT.err" >&2 || true
  fail "nytprof-cli report failed on produced profile (rc=$REPORT_RC)"
fi

if ! grep -E -q 'CORE:(print|match)' "$DUMP"; then
  fail "dump missing CORE:print / CORE:match (or main::CORE:print)"
fi
ok "dump JSONL has CORE:print and/or CORE:match"

TIMES="$(perl - "$DUMP" <<'PERL'
use strict;
use warnings;
use JSON::PP;

my $dump = $ARGV[0];
my @hits;
open my $fh, "<", $dump or die "open $dump: $!\n";
while (<$fh>) {
    my $j = decode_json($_);
    my $tag = $j->{tag} // next;
    next unless $tag eq "SUB_RETURN" || $tag eq "SUB_CALLERS";
    my $args = $j->{args} // [];
    my ($name, $incl, $excl);
    if ($tag eq "SUB_RETURN") {
        $incl = $args->[1];
        $excl = $args->[2];
        $name = $args->[3] // "";
    }
    else {
        $incl = $args->[3];
        $excl = $args->[4];
        $name = $args->[7] // "";
    }
    next unless $name =~ /CORE:(?:print|match)/;
    push @hits, { tag => $tag, name => $name, incl => $incl, excl => $excl };
}
close $fh;
if (!@hits) {
    print "CORE_HITS=0\n";
    print "CORE_MEASURED=0\n";
    exit 0;
}
my $measured = 0;
for my $h (@hits) {
    my $incl = $h->{incl} // 0;
    my $excl = $h->{excl} // 0;
    print "CORE $h->{tag} $h->{name} incl=$incl excl=$excl\n";
    $measured++ if $incl != 0 || $excl != 0;
}
print "CORE_HITS=", scalar(@hits), "\n";
print "CORE_MEASURED=$measured\n";
PERL
)"
printf '%s\n' "$TIMES"
HITS="$(perl -ne 'print $1 if /^CORE_HITS=(\d+)/' <<<"$TIMES")"
MEASURED="$(perl -ne 'print $1 if /^CORE_MEASURED=(\d+)/' <<<"$TIMES")"
[[ -n "$HITS" && "$HITS" -gt 0 ]] \
  || fail "dump missing SUB_RETURN/SUB_CALLERS CORE:print / CORE:match"
[[ -n "$MEASURED" && "$MEASURED" -gt 0 ]] \
  || fail "CORE:print / CORE:match incl and excl are both 0 (still unmeasured)"

if ! grep -E -q 'CORE:(print|match)' "$REPORT_TXT"; then
  fail "text report missing CORE:print / CORE:match"
fi
ok "dump+report CORE:print/match incl/excl not both 0 (hits=$HITS measured=$MEASURED)"

print_residuals
ok "G08 slowops PRINT/MATCH times"
exit 0

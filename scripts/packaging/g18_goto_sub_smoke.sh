#!/usr/bin/env bash
# DI-03 E2 — Default opcode OP_GOTO profiles `goto &other`.
#
# Drives a tiny goto &sub under real `perl -d:NYTProfM` (default opcode;
# no entersub=1 required). Isolated product PERL5LIB=collector/build/xs-nytprof.
# Never crates/. Never oracle pin. collection_default stays v5.
#
# Asserts: OP_GOTO installed; goto'd sub in dump; caller is the original
# caller (not the jumper, not DB::sub); SUB_CALLERS fid:line is the goto
# site; $^P 0x01 == 0. wrap=1 still works (wrap list remains wrap escape).
# Re-drives g12 (default opcode, no wrap), g04 15/3/15, and g17.
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

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "g18_goto_sub_smoke: repo root $ROOT"
echo "collection_default remains v5; never crates/ on PERL5LIB"
echo "E2: default opcode hooks OP_GOTO; wrap list stays wrap=1 only"

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

grep -q 'product_orig_pp_goto' "$PP_C" \
  || fail "pp_entersub.c missing separate product_orig_pp_goto"
grep -q 'OP_GOTO' "$PP_C" \
  || fail "pp_entersub.c missing OP_GOTO graft"
if grep -q 'E2 owns OP_GOTO. Never take a non-ENTERSUB' "$PP_C"; then
  fail "pp_entersub.c still early-returns OP_GOTO"
fi
grep -q 'product_goto_is_installed' "$PP_C" \
  || fail "pp_entersub.c missing product_goto_is_installed"
grep -q 'product_goto_is_installed' "$PP_H" \
  || fail "nytprof_pp.h missing product_goto_is_installed"
grep -q 'entersub_goto_is_installed' "$NYTP_XS" \
  || fail "NYTProf.xs missing entersub_goto_is_installed"
grep -q 'PRODUCT_GOTO_OPS' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing PRODUCT_GOTO_OPS stamp"
grep -q 'wrap=1 / use_db_sub=1 only' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm must keep wrap-list as wrap=1 only"
ok "E2 sources: separate orig goto + no early-return + wrap list wrap-only"

print_residuals() {
  echo "NOT-YET: E3 leave default 0 / E4 full slowops / live di02 27"
  echo "DI-03 not fully done; wrap list remains wrap=1 / use_db_sub=1 only"
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
  ok "g18_goto_sub_smoke completed (skip — no CC)"
  exit 0
fi
if ! perl -MConfig -e 'exit((-f "$Config{archlibexp}/CORE/EXTERN.h") ? 0 : 1)'; then
  echo "SKIP: perl XS headers not present"
  print_residuals
  ok "g18_goto_sub_smoke completed (skip — no XS headers)"
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

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g18-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

GOTO_PL="$WORKDIR/goto.pl"
cat >"$GOTO_PL" <<'END_GOTO'
use strict;
use warnings;

sub other {
    return 42;
}

sub jumper {
    goto &other; # G18_GOTO_SITE
}

sub original_caller {
    return jumper();
}

print "G18_P_BIT01=", (($^P & 0x01) ? 1 : 0), "\n";
print "G18_WRAP=", ($Devel::NYTProfM::PRODUCT_WRAP ? 1 : 0), "\n";
print "G18_ENTERSUB_OPS=", ($Devel::NYTProfM::PRODUCT_ENTERSUB_OPS ? 1 : 0), "\n";
print "G18_GOTO_OPS=", ($Devel::NYTProfM::PRODUCT_GOTO_OPS ? 1 : 0), "\n";
print "G18_INSTALLED=", (eval { DB::entersub_is_installed() } ? 1 : 0), "\n";
print "G18_GOTO_INSTALLED=", (eval { DB::entersub_goto_is_installed() } ? 1 : 0), "\n";
print "G18_RESULT=", original_caller(), "\n";
END_GOTO

GOTO_SITE_LINE="$(
  perl -ne 'print $. if /G18_GOTO_SITE/' "$GOTO_PL"
)"
[[ -n "$GOTO_SITE_LINE" ]] || fail "could not locate G18_GOTO_SITE line"

PROFILE="$WORKDIR/nytprof.out"
DUMP="$WORKDIR/dump.jsonl"

set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}:stmts=0" \
    perl -I"$NYTP_DEST" -d:NYTProfM "$GOTO_PL" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"
[[ "$RUN_RC" -eq 0 ]] || fail "goto attach exited $RUN_RC"
grep -q '^G18_RESULT=42$' <<<"$RUN_OUT" || fail "goto &other did not return 42"
grep -q '^G18_P_BIT01=0$' <<<"$RUN_OUT" \
  || fail "default opcode must leave \$^P bit 0x01 clear"
grep -q '^G18_WRAP=0$' <<<"$RUN_OUT" || fail "default omit must leave PRODUCT_WRAP=0"
grep -q '^G18_ENTERSUB_OPS=1$' <<<"$RUN_OUT" \
  || fail "default omit must set PRODUCT_ENTERSUB_OPS"
grep -q '^G18_GOTO_OPS=1$' <<<"$RUN_OUT" \
  || fail "default omit must stamp PRODUCT_GOTO_OPS"
grep -q '^G18_INSTALLED=1$' <<<"$RUN_OUT" \
  || fail "default omit must install OP_ENTERSUB"
grep -q '^G18_GOTO_INSTALLED=1$' <<<"$RUN_OUT" \
  || fail "default omit must install OP_GOTO"
ok "default opcode: OP_GOTO installed; \$^P 0x01 clear; goto returned 42"

[[ -f "$PROFILE" ]] || fail "NYTPROF file= did not write $PROFILE"
head -c 9 "$PROFILE" | grep -q 'NYTProf 5' || fail "not NYTProf 5"

"${CLI_CMD[@]}" dump "$PROFILE" >"$DUMP" 2>"$DUMP.err" \
  || { cat "$DUMP.err" >&2; fail "dump failed"; }

PARSE="$(perl - "$DUMP" "$GOTO_SITE_LINE" <<'PERL'
use strict;
use warnings;
use JSON::PP;
my ($dump, $want_line) = @ARGV;
my $other_ret = 0;
my $jumper_ret = 0;
my $other_from_orig = 0;
my $other_from_jumper = 0;
my $other_from_db = 0;
my $other_at_goto = 0;
my $other_line = -1;
my $other_caller = "";
open my $fh, "<", $dump or die $!;
while (<$fh>) {
    my $j = decode_json($_);
    my $tag = $j->{tag} // next;
    my $a = $j->{args} // [];
    if ( $tag eq "SUB_RETURN" ) {
        my $name = $a->[3] // "";
        $other_ret++  if $name =~ /(?:^|::)other\z/;
        $jumper_ret++ if $name =~ /(?:^|::)jumper\z/;
    }
    elsif ( $tag eq "SUB_CALLERS" ) {
        my $called = $a->[7] // "";
        my $caller = $a->[8] // "";
        next unless $called =~ /(?:^|::)other\z/;
        $other_line = $a->[1] // -1;
        $other_caller = $caller;
        $other_from_orig++   if $caller =~ /(?:^|::)original_caller\z/;
        $other_from_jumper++ if $caller =~ /(?:^|::)jumper\z/;
        $other_from_db++     if $caller =~ /(?:^|::)DB::sub\z/ || $caller =~ /^DB::/;
        $other_at_goto++     if defined $a->[1] && $a->[1] == $want_line;
    }
}
print "OTHER_RET=$other_ret\n";
print "JUMPER_RET=$jumper_ret\n";
print "OTHER_FROM_ORIG=$other_from_orig\n";
print "OTHER_FROM_JUMPER=$other_from_jumper\n";
print "OTHER_FROM_DB=$other_from_db\n";
print "OTHER_AT_GOTO=$other_at_goto\n";
print "OTHER_LINE=$other_line\n";
print "OTHER_CALLER=$other_caller\n";
print "WANT_LINE=$want_line\n";
PERL
)"
printf '%s\n' "$PARSE"
echo "$PARSE" | grep -E -q '^OTHER_RET=[1-9]' \
  || fail "goto'd sub other missing from SUB_RETURN"
echo "$PARSE" | grep -E -q '^OTHER_FROM_ORIG=[1-9]' \
  || fail "caller of other must be original_caller (got $(echo "$PARSE" | grep OTHER_CALLER))"
echo "$PARSE" | grep -E -q '^OTHER_FROM_JUMPER=0$' \
  || fail "caller of other must not be jumper"
echo "$PARSE" | grep -E -q '^OTHER_FROM_DB=0$' \
  || fail "caller of other must not be DB::sub"
echo "$PARSE" | grep -E -q '^OTHER_AT_GOTO=[1-9]' \
  || fail "SUB_CALLERS line must be goto site $GOTO_SITE_LINE (got $(echo "$PARSE" | grep OTHER_LINE))"
ok "dump: other returned; caller=original_caller; line=$GOTO_SITE_LINE (goto site)"

# wrap=1 still works: wrap list remains wrap escape; opcode GOTO unhooked.
WRAP_PL="$WORKDIR/wrap-probe.pl"
cat >"$WRAP_PL" <<'END_WP'
use strict;
use warnings;
sub other { 7 }
sub jumper { goto &other }
print "G18_P_BIT01=", (($^P & 0x01) ? 1 : 0), "\n";
print "G18_WRAP=", ($Devel::NYTProfM::PRODUCT_WRAP ? 1 : 0), "\n";
print "G18_ENTERSUB_OPS=", ($Devel::NYTProfM::PRODUCT_ENTERSUB_OPS ? 1 : 0), "\n";
print "G18_GOTO_INSTALLED=", (eval { DB::entersub_goto_is_installed() } ? 1 : 0), "\n";
print "G18_WRAP_RESULT=", jumper(), "\n";
END_WP

set +e
WRAP_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${WORKDIR}/wrap.out:wrap=1:stmts=0" \
    perl -I"$NYTP_DEST" -d:NYTProfM "$WRAP_PL" 2>&1
)"
WRAP_RC=$?
set -e
printf '%s\n' "$WRAP_OUT"
[[ "$WRAP_RC" -eq 0 ]] || fail "wrap=1 goto probe exited $WRAP_RC"
grep -q '^G18_WRAP_RESULT=7$' <<<"$WRAP_OUT" || fail "wrap=1 goto &other failed"
grep -q '^G18_P_BIT01=1$' <<<"$WRAP_OUT" || fail "wrap=1 must set \$^P 0x01"
grep -q '^G18_WRAP=1$' <<<"$WRAP_OUT" || fail "wrap=1 must set PRODUCT_WRAP"
grep -q '^G18_ENTERSUB_OPS=0$' <<<"$WRAP_OUT" || fail "wrap=1 must not install opcode"
grep -q '^G18_GOTO_INSTALLED=0$' <<<"$WRAP_OUT" || fail "wrap=1 must not hook OP_GOTO"
ok "wrap=1 still works; OP_GOTO not installed; wrap list remains wrap escape"

echo "re-drive g12 on default opcode (no wrap)"
unset NYTPROF_ATTACH_OPTS || true
bash "$ROOT/scripts/packaging/g12_memoize_caller_smoke.sh"
ok "g12 Memoize green on default opcode without wrap"

echo "re-drive g12 on wrap=1 (wrap list still required)"
NYTPROF_ATTACH_OPTS=wrap=1 bash "$ROOT/scripts/packaging/g12_memoize_caller_smoke.sh"
ok "g12 Memoize green on wrap=1 (wrap list still works)"

echo "re-drive g04 on default opcode"
bash "$ROOT/scripts/packaging/g04_v5_parity_smoke.sh"
ok "g04 15/3/15 green on default opcode"

echo "re-drive g17 on default opcode"
bash "$ROOT/scripts/packaging/g17_entersub_attach_smoke.sh"
ok "g17 default opcode green"

print_residuals
ok "G18 OP_GOTO attach + wrap=1 probe + g12/g04/g17"
exit 0

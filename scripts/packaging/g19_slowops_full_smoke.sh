#!/usr/bin/env bash
# DI-03 E4 — Full 6.15 slowops.h behind NYTPROF slowops=full / =3.
#
# Product slowops=2 stays PRINT/MATCH (KD-35). =1 stays fail-closed.
# Parse accepts 0, 2, 3, and string "full"; other values fail-closed.
# Live attach: default has no extra CORE:stat/sleep/prtf; full/3 does.
# Names stay pkg::CORE:op. Then re-drives g08 + g09 on default =2.
#
# Isolated product PERL5LIB=collector/build/xs-nytprof. Never crates/.
# collection_default stays v5.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

COLLECTOR="$ROOT/collector"
MAKEFILE="$COLLECTOR/Makefile"
NYTP_DEST="$COLLECTOR/build/xs-nytprof"
NYTP_SO="$NYTP_DEST/auto/Devel/NYTProfM/NYTProfM.so"
NYTP_PM_SRC="$COLLECTOR/xs/Devel/NYTProfM.pm"
NYTP_XS="$COLLECTOR/xs/NYTProf.xs"
SLOWOPS_H="$COLLECTOR/xs/slowops.h"
SLOW1_MSG="slowops=1 (collapsed CORE:: package) is residual until full opcode attach"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "g19_slowops_full_smoke: repo root $ROOT"
echo "collection_default remains v5; never crates/ on PERL5LIB"
echo "E4: slowops=full/3 opt-in; default slowops=2 stays PRINT/MATCH"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$NYTP_PM_SRC" ]] || fail "missing $NYTP_PM_SRC"
[[ -f "$NYTP_XS" ]] || fail "missing $NYTP_XS"
[[ -f "$SLOWOPS_H" ]] || fail "missing $SLOWOPS_H (copy from pin archive)"
grep -q 'install_product_slowops_full' "$NYTP_XS" \
  || fail "NYTProf.xs missing install_product_slowops_full"
grep -q 'pp_slowop_profiler' "$NYTP_XS" \
  || fail "NYTProf.xs missing pp_slowop_profiler"
grep -q 'pp_slowop_profiler' "$SLOWOPS_H" \
  || fail "slowops.h missing pp_slowop_profiler assignments"
grep -F -q 'devel-nytprof-6.15/slowops.h' "$SLOWOPS_H" \
  || fail "slowops.h missing pin-archive provenance"
grep -q "lc( \$opts->{slowops} ) eq 'full'" "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing slowops=full parse"
grep -q 'install_product_slowops_full' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing install_product_slowops_full"
# Product =2 path must remain the thin installer, not full.
if ! grep -A20 'PRODUCT_SLOWOPS == 2' "$NYTP_PM_SRC" \
  | grep -q 'install_product_slowops()'; then
  fail "slowops=2 must still call install_product_slowops (thin PRINT/MATCH)"
fi
ok "E4 sources: slowops.h + full installer + parse"

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
  echo "E4: full 6.15 slowops.h is opt-in (slowops=full / =3); default stays PRINT/MATCH"
  echo "RESIDUAL: =full exclusive is thin (not 6.15 savestack); sort/backtick/(?{}) can double-count parent excl"
  echo "NOT-YET: E1b default opcode flip / E2 GOTO / leave=1 default"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain — G19 debugger .so not built"
  print_residuals
  ok "g19_slowops_full_smoke completed (skip — no CC)"
  exit 0
fi
have_xs_headers=0
if command -v perl >/dev/null 2>&1; then
  if perl -MConfig -e 'exit((-f "$Config{archlibexp}/CORE/EXTERN.h") ? 0 : 1)'; then
    have_xs_headers=1
  fi
fi
if [[ "$have_xs_headers" -ne 1 ]]; then
  echo "SKIP: perl XS headers not present — G19 debugger .so not built"
  print_residuals
  ok "g19_slowops_full_smoke completed (skip — no XS headers)"
  exit 0
fi

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
ok "xs-nytprof produced .so"

CLI_CMD=()
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI_CMD=("${NYTPROF_NATIVE_CLI}")
elif [[ -x "$ROOT/target/debug/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-cli")
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

run_attach() {
  local env="$1" out="$2"
  set +e
  NYTPROF="$env" perl -I"$NYTP_DEST" -d:NYTProfM -e 'print "g19_parse_ok\n"' \
    >"$out.stdout" 2>"$out.stderr"
  echo $? >"$out.rc"
  set -e
}

# --- parse: 0 / 2 / 3 / full accepted ---
for spec in "slowops=0" "slowops=2" "slowops=3" "slowops=full"; do
  run_attach "file=$WORKDIR/p.out:${spec}" "$WORKDIR/p_${spec#slowops=}"
done
[[ "$(cat "$WORKDIR/p_0.rc")" == "0" ]] || fail "slowops=0 must parse: $(cat "$WORKDIR/p_0.stderr")"
[[ "$(cat "$WORKDIR/p_2.rc")" == "0" ]] || fail "slowops=2 must parse: $(cat "$WORKDIR/p_2.stderr")"
[[ "$(cat "$WORKDIR/p_3.rc")" == "0" ]] || fail "slowops=3 must parse: $(cat "$WORKDIR/p_3.stderr")"
[[ "$(cat "$WORKDIR/p_full.rc")" == "0" ]] || fail "slowops=full must parse: $(cat "$WORKDIR/p_full.stderr")"
ok "parse accepts 0, 2, 3, full"

# --- slowops=1 fail-closed (unchanged residual string) ---
run_attach "file=$WORKDIR/s1.out:slowops=1" "$WORKDIR/s1"
[[ "$(cat "$WORKDIR/s1.rc")" != "0" ]] || fail "slowops=1 must fail-closed"
grep -F -q "$SLOW1_MSG" "$WORKDIR/s1.stderr" \
  || fail "slowops=1 missing residual fail-closed message"
[[ ! -f "$WORKDIR/s1.out" ]] || fail "slowops=1 must not write a profile"
ok "slowops=1 fail-closed residual"

# --- reject other values ---
for bad in 4 -1 yes 1.5; do
  run_attach "file=$WORKDIR/bad.out:slowops=${bad}" "$WORKDIR/bad_${bad}"
  [[ "$(cat "$WORKDIR/bad_${bad}.rc")" != "0" ]] \
    || fail "slowops=${bad} must fail-closed"
  grep -E -q 'unknown NYTPROF option: slowops' "$WORKDIR/bad_${bad}.stderr" \
    || fail "slowops=${bad} missing unknown-option text"
  [[ ! -f "$WORKDIR/bad.out" ]] || fail "slowops=${bad} must not write a profile"
done
ok "parse rejects 4 / -1 / yes / 1.5"

SCRIPT="$WORKDIR/extra_slowops.pl"
cat >"$SCRIPT" <<'PERL'
use strict;
use warnings;

stat($0);
printf "g19_printf\n";
sleep 0;
print "g19_print\n";
my $ok = 0;
$ok++ if "foo" =~ /foo/;
print "g19_ok=$ok\n";
PERL

dump_core_names() {
  local profile="$1" dump="$2"
  "${CLI_CMD[@]}" dump "$profile" >"$dump" 2>"$dump.err" \
    || { cat "$dump.err" >&2; fail "dump failed on $profile"; }
  perl - "$dump" <<'PERL'
use strict;
use warnings;
use JSON::PP;
my $dump = $ARGV[0];
my %names;
open my $fh, "<", $dump or die "open $dump: $!\n";
while (<$fh>) {
    my $j = decode_json($_);
    my $tag = $j->{tag} // next;
    next unless $tag eq "SUB_RETURN" || $tag eq "SUB_CALLERS";
    my $args = $j->{args} // [];
    my $name = $tag eq "SUB_RETURN" ? ($args->[3] // "") : ($args->[7] // "");
    next unless $name =~ /CORE:/;
    $names{$name}++;
}
close $fh;
print "CORE_NAME $_\n" for sort keys %names;
print "HAS_PRINT=", (grep { /CORE:print\z/ } keys %names) ? 1 : 0, "\n";
print "HAS_MATCH=", (grep { /CORE:match\z/ } keys %names) ? 1 : 0, "\n";
print "HAS_STAT=",  (grep { /CORE:stat\z/ } keys %names) ? 1 : 0, "\n";
print "HAS_SLEEP=", (grep { /CORE:sleep\z/ } keys %names) ? 1 : 0, "\n";
print "HAS_PRTF=",  (grep { /CORE:prtf\z/ } keys %names) ? 1 : 0, "\n";
print "COLLAPSED_CORE=", (grep { /^CORE::/ } keys %names) ? 1 : 0, "\n";
PERL
}

run_script() {
  local env="$1" profile="$2"
  set +e
  NYTPROF="$env" perl -I"$NYTP_DEST" -d:NYTProfM "$SCRIPT" \
    >"$profile.stdout" 2>"$profile.stderr"
  local rc=$?
  set -e
  [[ "$rc" -eq 0 ]] || fail "attach $env failed: $(cat "$profile.stderr")"
  grep -q '^g19_ok=' "$profile.stdout" || fail "script missing g19_ok under $env"
  [[ -s "$profile" ]] || fail "missing profile $profile"
  head -c 9 "$profile" | grep -q 'NYTProf 5' || fail "not NYTProf 5: $profile"
}

# --- default / slowops=2: PRINT/MATCH only (no extra table ops) ---
run_script "file=$WORKDIR/def.out" "$WORKDIR/def.out"
DEF_NAMES="$(dump_core_names "$WORKDIR/def.out" "$WORKDIR/def.jsonl")"
printf '%s\n' "$DEF_NAMES"
[[ "$(perl -ne 'print $1 if /^HAS_PRINT=(\d+)/' <<<"$DEF_NAMES")" == "1" ]] \
  || fail "default dump missing CORE:print"
[[ "$(perl -ne 'print $1 if /^HAS_MATCH=(\d+)/' <<<"$DEF_NAMES")" == "1" ]] \
  || fail "default dump missing CORE:match"
[[ "$(perl -ne 'print $1 if /^HAS_STAT=(\d+)/' <<<"$DEF_NAMES")" == "0" ]] \
  || fail "default slowops=2 must not emit CORE:stat (full table leaked)"
[[ "$(perl -ne 'print $1 if /^HAS_SLEEP=(\d+)/' <<<"$DEF_NAMES")" == "0" ]] \
  || fail "default slowops=2 must not emit CORE:sleep (full table leaked)"
[[ "$(perl -ne 'print $1 if /^HAS_PRTF=(\d+)/' <<<"$DEF_NAMES")" == "0" ]] \
  || fail "default slowops=2 must not emit CORE:prtf (full table leaked)"
ok "default slowops=2: PRINT/MATCH only (no stat/sleep/prtf)"

run_script "file=$WORKDIR/s2.out:slowops=2" "$WORKDIR/s2.out"
S2_NAMES="$(dump_core_names "$WORKDIR/s2.out" "$WORKDIR/s2.jsonl")"
[[ "$(perl -ne 'print $1 if /^HAS_STAT=(\d+)/' <<<"$S2_NAMES")" == "0" ]] \
  || fail "slowops=2 must not emit CORE:stat"
ok "explicit slowops=2 stays PRINT/MATCH subset"

# --- slowops=full and =3: extra CORE: names, pkg::CORE:op shape ---
for mode in full 3; do
  run_script "file=$WORKDIR/${mode}.out:slowops=${mode}" "$WORKDIR/${mode}.out"
  NAMES="$(dump_core_names "$WORKDIR/${mode}.out" "$WORKDIR/${mode}.jsonl")"
  printf '%s\n' "$NAMES"
  [[ "$(perl -ne 'print $1 if /^HAS_PRINT=(\d+)/' <<<"$NAMES")" == "1" ]] \
    || fail "slowops=${mode} missing CORE:print"
  [[ "$(perl -ne 'print $1 if /^HAS_MATCH=(\d+)/' <<<"$NAMES")" == "1" ]] \
    || fail "slowops=${mode} missing CORE:match"
  EXTRA=0
  [[ "$(perl -ne 'print $1 if /^HAS_STAT=(\d+)/' <<<"$NAMES")" == "1" ]] && EXTRA=1
  [[ "$(perl -ne 'print $1 if /^HAS_SLEEP=(\d+)/' <<<"$NAMES")" == "1" ]] && EXTRA=1
  [[ "$(perl -ne 'print $1 if /^HAS_PRTF=(\d+)/' <<<"$NAMES")" == "1" ]] && EXTRA=1
  [[ "$EXTRA" -eq 1 ]] \
    || fail "slowops=${mode} must emit at least one extra CORE:stat/sleep/prtf"
  [[ "$(perl -ne 'print $1 if /^COLLAPSED_CORE=(\d+)/' <<<"$NAMES")" == "0" ]] \
    || fail "slowops=${mode} used collapsed CORE:: names (want pkg::CORE:op)"
  grep -E -q '::CORE:(print|match|stat|sleep|prtf)' <<<"$NAMES" \
    || fail "slowops=${mode} names must be pkg::CORE:op"
  ok "slowops=${mode}: full table extra CORE: names, pkg::CORE:op"
done

print_residuals
ok "G19 slowops=full opt-in (default still PRINT/MATCH)"
exit 0

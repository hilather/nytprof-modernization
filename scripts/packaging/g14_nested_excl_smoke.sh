#!/usr/bin/env bash
# PR-14 — Exclusive = incl − Σ child inclusive (not child exclusive).
# Three-level top→mid→leaf: top.excl must be a remainder, not mid+leaf
# exclusive leaked into top (rex lab_run excl ≈ YAML).
#
# Also: stmts=0 skips TIME_LINE (smaller nytprof.out). Default stays stmts=1.
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

echo "g14_nested_excl_smoke: repo root $ROOT"
echo "collection_default remains v5; never crates/ on PERL5LIB"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$NYTP_PM_SRC" ]] || fail "missing $NYTP_PM_SRC"
grep -q 'child_excl} += \$incl' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm must credit parent with child inclusive (\$incl)"
if grep -n 'child_excl} += \$excl' "$NYTP_PM_SRC" | grep -v '^[[:space:]]*#' | grep -q .; then
  fail "NYTProfM.pm still credits parent with child exclusive (\$excl)"
fi
grep -q 'PRODUCT_STMTS' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing PRODUCT_STMTS (stmts=0 skip TIME_LINE)"
ok "PR-14 sources: parent credit is child incl; stmts=0 wired"

print_residuals() {
  echo "G09 tokenize MATCH excl: g09_tokenize_excl_smoke.sh"
  echo "NOT-YET: full 6.15 opcode/entersub; DateTime/Moo goto still have no SUB_RETURN"
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
  ok "g14_nested_excl_smoke completed (skip — no CC)"
  exit 0
fi
if ! perl -MConfig -e 'exit((-f "$Config{archlibexp}/CORE/EXTERN.h") ? 0 : 1)'; then
  echo "SKIP: perl XS headers not present"
  print_residuals
  ok "g14_nested_excl_smoke completed (skip — no XS headers)"
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

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g14-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

SCRIPT="$WORKDIR/nest.pl"
cat >"$SCRIPT" <<'END_NEST'
use strict;
use warnings;
sub leaf {
    my $n = 0;
    $n += $_ for 1 .. 80_000;
    return $n;
}
sub mid {
    my $s = 0;
    $s += leaf() for 1 .. 8;
    return $s;
}
sub top {
    my $s = 0;
    $s += mid() for 1 .. 4;
    return $s;
}
print "g14_sum=", top(), "\n";
END_NEST

PROFILE="$WORKDIR/nytprof.out"
DUMP="$WORKDIR/dump.jsonl"
set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}" perl -I"$NYTP_DEST" -d:NYTProfM "$SCRIPT" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"
[[ "$RUN_RC" -eq 0 ]] || fail "nested attach exited $RUN_RC"
grep -q '^g14_sum=' <<<"$RUN_OUT" || fail "missing g14_sum"
head -c 9 "$PROFILE" | grep -q 'NYTProf 5' || fail "not NYTProf 5"

"${CLI_CMD[@]}" dump "$PROFILE" >"$DUMP" 2>"$DUMP.err" \
  || { cat "$DUMP.err" >&2; fail "dump failed"; }

perl - "$DUMP" <<'PERL' || fail "nested exclusive split failed"
use strict;
use warnings;
use JSON::PP;
my $dump = shift;
my %sub;
open my $fh, "<", $dump or die $!;
while (<$fh>) {
    my $j = decode_json($_);
    next unless ($j->{tag} // "") eq "SUB_RETURN";
    my $a = $j->{args} // [];
    my $n = $a->[3] // next;
    $sub{$n}{incl} += $a->[1] // 0;
    $sub{$n}{excl} += $a->[2] // 0;
    $sub{$n}{n}++;
}
sub pick {
    my ($re) = @_;
    for my $n (keys %sub) {
        return ($n, $sub{$n}) if $n =~ $re;
    }
    return;
}
my ($tn, $t) = pick(qr/(?:^|::)top\z/);
my ($mn, $m) = pick(qr/(?:^|::)mid\z/);
my ($ln, $l) = pick(qr/(?:^|::)leaf\z/);
die "missing top/mid/leaf in SUB_RETURN\n" unless $t && $m && $l;
print "top $tn n=$t->{n} incl=$t->{incl} excl=$t->{excl}\n";
print "mid $mn n=$m->{n} incl=$m->{incl} excl=$m->{excl}\n";
print "leaf $ln n=$l->{n} incl=$l->{incl} excl=$l->{excl}\n";
die "top incl not > top excl\n" unless $t->{incl} > $t->{excl};
die "mid incl not > mid excl\n" unless $m->{incl} > $m->{excl};
# parent incl ≈ parent excl + child incl
my $top_sum = $t->{excl} + $m->{incl};
my $mid_sum = $m->{excl} + $l->{incl};
my $top_err = abs($t->{incl} - $top_sum) / ($t->{incl} || 1);
my $mid_err = abs($m->{incl} - $mid_sum) / ($m->{incl} || 1);
print "top incl vs excl+mid.incl err=$top_err\n";
print "mid incl vs excl+leaf.incl err=$mid_err\n";
die "top.incl != top.excl + mid.incl (err=$top_err)\n" if $top_err > 0.15;
die "mid.incl != mid.excl + leaf.incl (err=$mid_err)\n" if $mid_err > 0.15;
# grandchild must not sit in top exclusive
die "top.excl still holds mid.incl (grandchild leak)\n"
  if $t->{excl} > $m->{incl} * 0.6;
print "ok-nested-excl\n";
PERL
ok "3-level exclusive is incl minus child inclusive"

# stmts=0: no TIME_LINE (or far fewer than default)
ST0="$WORKDIR/stmts0.out"
DUMP0="$WORKDIR/dump0.jsonl"
set +e
ST0_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${ST0}:stmts=0" perl -I"$NYTP_DEST" -d:NYTProfM "$SCRIPT" 2>&1
)"
ST0_RC=$?
set -e
[[ "$ST0_RC" -eq 0 ]] || fail "stmts=0 attach exited $ST0_RC"
head -c 9 "$ST0" | grep -q 'NYTProf 5' || fail "stmts=0 not NYTProf 5"
"${CLI_CMD[@]}" dump "$ST0" >"$DUMP0" 2>"$DUMP0.err" \
  || { cat "$DUMP0.err" >&2; fail "stmts=0 dump failed"; }
TL0=$(grep -c '"tag":"TIME_LINE"' "$DUMP0" || true)
TL1=$(grep -c '"tag":"TIME_LINE"' "$DUMP" || true)
echo "TIME_LINE default=$TL1 stmts=0=$TL0"
[[ "$TL0" -eq 0 ]] || fail "stmts=0 still emitted TIME_LINE ($TL0)"
[[ "$TL1" -gt 0 ]] || fail "default stmts=1 emitted no TIME_LINE"
SZ0=$(wc -c <"$ST0")
SZ1=$(wc -c <"$PROFILE")
echo "size default=$SZ1 stmts=0=$SZ0"
[[ "$SZ0" -lt "$SZ1" ]] || fail "stmts=0 profile not smaller than default"
ok "stmts=0 skips TIME_LINE and shrinks nytprof.out"

if ! grep -E -q '"tag":"ATTRIBUTE".*"application"|application' "$DUMP"; then
  fail "live attach dump missing ATTRIBUTE application (\$0)"
fi
if grep -F -q 'Config_heavy.pl' <<<"$(grep -i application "$DUMP" || true)"; then
  fail "ATTRIBUTE application must not be Config_heavy.pl"
fi
grep -F -q 'nest.pl' "$DUMP" \
  || fail "ATTRIBUTE application should mention nest.pl (\$0)"
ok "ATTRIBUTE application is \$0 (nest.pl), not Config_heavy.pl"

print_residuals
ok "G14 nested exclusive + stmts=0 size"
exit 0

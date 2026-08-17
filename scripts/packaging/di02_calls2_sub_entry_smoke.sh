#!/usr/bin/env bash
# PR-B2 / DI-02 — Live calls=2 SUB_ENTRY 27 + CORE:print / CORE:match.
#
# Drives real `perl -d:NYTProfM` (product tree, not baseline/6.15/install)
# with NYTPROF file=<path>:calls=2 on fixtures/v5/calls2-default/workload.pl
# (same leaf/mid shape). Inspects produced NYTProf 5 bytes with shipped dump/report.
#
# Binding (do not claim 27 without CORE: names; do not lower 27):
#   calls=2 → sub_entry_events=27 AND SUB_RETURN includes
#     main::CORE:print and warnings::CORE:match
#   calls=1 → sub_entry_events=0; leaf 15 / mid 3 / mid→leaf 15
#   slowops=0 → no CORE: SUB_RETURN names
#   no double main::leaf RETURN (calls=1 and calls=2)
#
# Does NOT invoke DB::emit_* from the workload. Does NOT rewrite dual_path.
# collection_default stays v5. Not full opcode / DI-03 / not S2.
#
# Exit 0: DI-02 pass, or honest skip (no CC / no XS headers).
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
WORKLOAD="$ROOT/fixtures/v5/calls2-default/workload.pl"

usage() {
  cat <<'EOF'
Usage: di02_calls2_sub_entry_smoke.sh

DI-02 live calls=2: sub_entry_events=27 + CORE:print / CORE:match;
calls=1 still 0 SUB_ENTRY; leaf/mid 15/3; no double leaf RETURN.
EOF
}

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

# g17 overlays entersub=1 (this smoke hardcodes file= only).
nytprof_attach() {
  local spec="$1"
  if [[ -n "${NYTPROF_ATTACH_OPTS:-}" ]]; then
    printf '%s:%s' "$spec" "${NYTPROF_ATTACH_OPTS}"
  else
    printf '%s' "$spec"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    *) printf 'ERROR: unknown flag: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

echo "di02_calls2_sub_entry_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary until S2"
echo "never crates/ on PERL5LIB"
echo "DI-02 live attach NYTPROF calls=2; not DI-03 / not S2"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$NYTP_XS" ]] || fail "missing $NYTP_XS"
[[ -f "$NYTP_PM_SRC" ]] || fail "missing $NYTP_PM_SRC"
[[ -f "$NYTP_CORE_SRC" ]] || fail "missing $NYTP_CORE_SRC"
[[ -f "$WORKLOAD" ]] || fail "missing calls2-default workload $WORKLOAD"
grep -q 'xs-nytprof' "$MAKEFILE" || fail "Makefile missing xs-nytprof target"
grep -q 'emit_sub_entry' "$NYTP_XS" || fail "NYTProf.xs missing emit_sub_entry"
grep -q 'PRODUCT_CALLS' "$NYTP_PM_SRC" || fail "NYTProfM.pm missing PRODUCT_CALLS stamp"
grep -E -q 'sub leaf|mid' "$WORKLOAD" || fail "workload.pl missing leaf/mid shape"
ok "DI-02 sources and calls2-default workload present"

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
  echo "NOT-YET: full 6.15 opcode/entersub / DISCOUNT 818 / previous-statement ticks"
  echo "NOT-YET: mid-deflate continue-in-child / TEST-018 / S2"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain — DI-02 debugger .so not built"
  print_residuals
  ok "di02_calls2_sub_entry_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present"
  print_residuals
  ok "di02_calls2_sub_entry_smoke completed (skip — no XS headers)"
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
  fail "no shipped dump/report"
fi
echo "dump/report CLI: ${CLI_CMD[*]}"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-di02-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

dump_and_report() {
  local profile="$1" dump="$2" json="$3"
  set +e
  "${CLI_CMD[@]}" dump "$profile" >"$dump" 2>"$dump.err"
  local rc=$?
  set -e
  if [[ "$rc" -ne 0 ]]; then
    cat "$dump.err" >&2 || true
    fail "nytprof-cli dump failed on $profile (rc=$rc)"
  fi
  set +e
  "${CLI_CMD[@]}" report --json "$profile" >"$json" 2>"$json.err"
  rc=$?
  set -e
  if [[ "$rc" -ne 0 ]]; then
    cat "$json.err" >&2 || true
    fail "nytprof-cli report --json failed on $profile (rc=$rc)"
  fi
}

# --- calls=2 ---
PROFILE2="$WORKDIR/calls2.out"
DUMP2="$WORKDIR/calls2.jsonl"
JSON2="$WORKDIR/calls2.json"

echo "running: NYTPROF=file=…:calls=2 perl -I${NYTP_DEST} -d:NYTProfM <calls2-default workload>"
set +e
RUN2="$(
  cd "$WORKDIR" && NYTPROF="$(nytprof_attach "file=${PROFILE2}:calls=2")" perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" 2>&1
)"
RC2=$?
set -e
printf '%s\n' "$RUN2"
[[ "$RC2" -eq 0 ]] || fail "perl -d:NYTProfM calls=2 exited $RC2"
grep -E -q '^total=' <<<"$RUN2" || fail "calls=2 workload did not print total="
[[ -f "$PROFILE2" ]] || fail "calls=2 did not write profile"
magic="$(head -c 9 "$PROFILE2" || true)"
[[ "$magic" == "NYTProf 5" ]] || fail "calls=2 bytes must start with NYTProf 5"
ok "live perl -d:NYTProfM ran calls=2 workload"

dump_and_report "$PROFILE2" "$DUMP2" "$JSON2"

COUNTS2="$(perl - "$DUMP2" "$JSON2" <<'PERL'
use strict;
use warnings;
use JSON::PP;
my ($dump, $json) = @ARGV;
my $rep = do { open my $fh, "<", $json or die $!; local $/; decode_json(<$fh>) };
my $se = $rep->{sub_entry_events} // -1;
my $leaf = $rep->{leaf_returns} // -1;
my $mid = $rep->{mid_returns} // -1;
my $edge = $rep->{mid_leaf_edge} // -1;
my %ret;
open my $fh, "<", $dump or die $!;
while (<$fh>) {
    my $j = decode_json($_);
    next unless ($j->{tag} // "") eq "SUB_RETURN";
    my $name = $j->{args}[3] // "";
    $ret{$name}++;
}
print "sub_entry_events=$se\n";
print "leaf_returns=$leaf\n";
print "mid_returns=$mid\n";
print "mid_leaf_edge=$edge\n";
print "leaf_return_count=", ($ret{"main::leaf"} // 0), "\n";
print "has_core_print=", ($ret{"main::CORE:print"} ? 1 : 0), "\n";
print "has_core_match=", ($ret{"warnings::CORE:match"} ? 1 : 0), "\n";
print "core_print=", ($ret{"main::CORE:print"} // 0), "\n";
print "core_match=", ($ret{"warnings::CORE:match"} // 0), "\n";
PERL
)"
printf '%s\n' "$COUNTS2"
echo "$COUNTS2" | grep -E -q '^sub_entry_events=27$' \
  || fail "calls=2 sub_entry_events must be 27 (got $(echo "$COUNTS2" | grep '^sub_entry_events='))"
echo "$COUNTS2" | grep -E -q '^leaf_returns=15$' || fail "calls=2 leaf_returns must be 15"
echo "$COUNTS2" | grep -E -q '^mid_returns=3$' || fail "calls=2 mid_returns must be 3"
echo "$COUNTS2" | grep -E -q '^mid_leaf_edge=15$' || fail "calls=2 mid_leaf_edge must be 15"
echo "$COUNTS2" | grep -E -q '^leaf_return_count=15$' \
  || fail "calls=2 main::leaf SUB_RETURN must be 15 (no double count)"
echo "$COUNTS2" | grep -E -q '^has_core_print=1$' \
  || fail "calls=2 missing SUB_RETURN main::CORE:print"
echo "$COUNTS2" | grep -E -q '^has_core_match=1$' \
  || fail "calls=2 missing SUB_RETURN warnings::CORE:match"
ok "calls=2: sub_entry_events=27 + CORE:print + CORE:match; 15/3/15; no double leaf"

# --- calls=1 (default) ---
PROFILE1="$WORKDIR/calls1.out"
DUMP1="$WORKDIR/calls1.jsonl"
JSON1="$WORKDIR/calls1.json"
echo "running: NYTPROF=file=… (calls=1 default) perl -I${NYTP_DEST} -d:NYTProfM"
set +e
RUN1="$(
  cd "$WORKDIR" && NYTPROF="$(nytprof_attach "file=${PROFILE1}")" perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" 2>&1
)"
RC1=$?
set -e
printf '%s\n' "$RUN1"
[[ "$RC1" -eq 0 ]] || fail "perl -d:NYTProfM calls=1 exited $RC1"
dump_and_report "$PROFILE1" "$DUMP1" "$JSON1"
COUNTS1="$(perl - "$DUMP1" "$JSON1" <<'PERL'
use strict;
use warnings;
use JSON::PP;
my ($dump, $json) = @ARGV;
my $rep = do { open my $fh, "<", $json or die $!; local $/; decode_json(<$fh>) };
my $se = $rep->{sub_entry_events} // -1;
my $leaf = $rep->{leaf_returns} // -1;
my $mid = $rep->{mid_returns} // -1;
my %ret;
open my $fh, "<", $dump or die $!;
while (<$fh>) {
    my $j = decode_json($_);
    next unless ($j->{tag} // "") eq "SUB_RETURN";
    $ret{$j->{args}[3] // ""}++;
}
print "sub_entry_events=$se\n";
print "leaf_returns=$leaf\n";
print "mid_returns=$mid\n";
print "leaf_return_count=", ($ret{"main::leaf"} // 0), "\n";
PERL
)"
printf '%s\n' "$COUNTS1"
echo "$COUNTS1" | grep -E -q '^sub_entry_events=0$' \
  || fail "calls=1 sub_entry_events must be 0"
echo "$COUNTS1" | grep -E -q '^leaf_returns=15$' || fail "calls=1 leaf_returns must be 15"
echo "$COUNTS1" | grep -E -q '^mid_returns=3$' || fail "calls=1 mid_returns must be 3"
echo "$COUNTS1" | grep -E -q '^leaf_return_count=15$' \
  || fail "calls=1 main::leaf SUB_RETURN must stay 15"
ok "calls=1: sub_entry_events=0; leaf/mid 15/3; no double leaf"

# --- slowops=0 ---
PROFILE0="$WORKDIR/slow0.out"
DUMP0="$WORKDIR/slow0.jsonl"
JSON0="$WORKDIR/slow0.json"
echo "running: NYTPROF=file=…:calls=2:slowops=0"
set +e
RUN0="$(
  cd "$WORKDIR" && NYTPROF="$(nytprof_attach "file=${PROFILE0}:calls=2:slowops=0")" perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" 2>&1
)"
RC0=$?
set -e
printf '%s\n' "$RUN0"
[[ "$RC0" -eq 0 ]] || fail "perl -d:NYTProfM slowops=0 exited $RC0"
dump_and_report "$PROFILE0" "$DUMP0" "$JSON0"
if grep -E -q 'CORE:(print|match)' "$DUMP0"; then
  fail "slowops=0 must not emit CORE:print / CORE:match"
fi
ok "slowops=0: no CORE: SUB_RETURN names"

print_residuals
ok "DI-02 calls=2 SUB_ENTRY 27 live attach"

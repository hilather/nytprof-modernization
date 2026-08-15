#!/usr/bin/env bash
# PR-G04 — Live product attach parity on a default-calls1-shaped workload.
#
# Drives real `perl -d:NYTProfM` (product tree, not baseline/6.15/install)
# with NYTPROF file=<path> on fixtures/v5/default-calls1/workload.pl
# (mid ×3 × leaf ×5). Inspects the produced NYTProf 5 bytes with a shipped
# dump/report. Counts come from those bytes (leaf 15 / mid 3 / mid→leaf 15).
#
# Does NOT invoke DB::emit_* from the workload. Does NOT rewrite dual_path
# (stays oracle-primary). collection_default stays v5. G03a trivial -e
# still writes no nytprof.out. Default (no blocks=) stays TIME_LINE
# (no TIME_BLOCK). PR-3 finish emits SRC_LINE + SUB_INFO (savesrc default on).
# G05 format=v6 / G06 fork / full opcode / DI-02 27 remain residual.
#
# When CC + Perl XS headers exist:
#   make -C collector xs-nytprof, isolated product @INC, live attach + dump.
# When missing: honest SKIP: after source-file asserts (exit 0).
#
# Exit 0: G04 attach-parity pass, or honest skip (no CC / no XS headers).
# Exit 1: attach / dump / count failure.
# Exit 2: wrapper misuse or crates/ on PERL5LIB.
#
# Never puts crates/ on PERL5LIB. Not wired into dual_path or offline_gate.
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
WORKLOAD="$ROOT/fixtures/v5/default-calls1/workload.pl"

usage() {
  cat <<'EOF'
Usage: g04_v5_parity_smoke.sh

G04 live-attach parity: real perl -d:NYTProfM on default-calls1-shaped
work, dump/report of produced NYTProf 5 bytes, leaf 15 / mid 3 / edge 15.
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

echo "g04_v5_parity_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary until S2"
echo "never crates/ on PERL5LIB"
echo "G04 live attach via NYTPROF file= + DB::sub; not G05/G06/full opcode"

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
[[ -f "$WORKLOAD" ]] || fail "missing default-calls1 workload $WORKLOAD"
grep -q 'xs-nytprof' "$MAKEFILE" || fail "Makefile missing xs-nytprof target"
grep -q 'libnytp_sink_v5.a' "$MAKEFILE" || fail "Makefile missing libnytp_sink_v5.a (D1-B link)"
grep -q 'nytp_emit_sub_return' "$NYTP_XS" || fail "NYTProf.xs missing nytp_emit_sub_return wrapper"
grep -q 'nytp_emit_sub_callers' "$NYTP_XS" || fail "NYTProf.xs missing nytp_emit_sub_callers wrapper"
grep -q 'sub sub' "$NYTP_PM_SRC" || fail "NYTProf.pm missing DB::sub hook"
grep -q 'file=' "$NYTP_PM_SRC" || fail "NYTProf.pm missing NYTPROF file= parse"
grep -q '0x01' "$NYTP_PM_SRC" || fail "NYTProf.pm missing \$^P 0x01 (DB::sub enter/exit)"
grep -E -q 'sub leaf|mid' "$WORKLOAD" || fail "workload.pl missing leaf/mid shape"
ok "G04 debugger sources, emit_sub_callers, DB::sub, and fixture workload present"

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
  echo "G05 options/format: g05_options_format_smoke.sh"
  echo "G06 fork/addpid: g06_fork_addpid_smoke.sh"
  echo "NOT-YET: mid-deflate continue-in-child / TEST-018"
  echo "DI-01 blocks=1 780/810 + elapsed TIME_BLOCK: di01_blocks_780_smoke.sh"
  echo "G07 Getopt/Exporter compile-safe: g07_getopt_compile_smoke.sh"
  echo "G08 CORE:print/match times: g08_slowops_times_smoke.sh"
  echo "G09 tokenize excl split: g09_tokenize_excl_smoke.sh"
  echo "NOT-YET: full 6.15 opcode/entersub / DI-02 SUB_ENTRY 27"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — G04 debugger .so not built"
  echo "  (honest skip; live attach requires xs-nytprof)"
  print_residuals
  ok "g04_v5_parity_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present — G04 debugger .so not built"
  echo "  (honest skip; live attach requires xs-nytprof)"
  print_residuals
  ok "g04_v5_parity_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "perl + EXTERN.h present"

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
[[ -f "$NYTP_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM.pm"
[[ -f "$NYTP_DEST/Devel/NYTProfM/Core.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM/Core.pm"
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

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g04-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

# Isolated product @INC only. Never baseline/6.15/install, never crates/.
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

PROFILE="$WORKDIR/nytprof.out"
DUMP="$WORKDIR/dump.jsonl"
REPORT_TXT="$WORKDIR/report.txt"
REPORT_JSON="$WORKDIR/report.json"

echo "workdir: $WORKDIR"
echo "PERL5LIB=$PERL5LIB"
echo "workload: $WORKLOAD"
echo "running: NYTPROF=file=${PROFILE} perl -I${NYTP_DEST} -d:NYTProfM <default-calls1 workload>"

# Stamp probe: file= must flip PRODUCT_XS_ATTACH for this session.
STAMP_PATH="$WORKDIR/stamp.nytprof"
set +e
STAMP_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${STAMP_PATH}" perl -I"$NYTP_DEST" -d:NYTProfM -e '
    my $inc = $INC{"Devel/NYTProfM.pm"} // "";
    print "INC=", $inc, "\n";
    my $load = ($Devel::NYTProfM::PRODUCT_XS_LOAD ? 1 : 0);
    my $attach = (defined $Devel::NYTProfM::PRODUCT_XS_ATTACH && $Devel::NYTProfM::PRODUCT_XS_ATTACH) ? 1 : 0;
    print "PRODUCT_XS_LOAD=", $load, "\n";
    print "PRODUCT_XS_ATTACH=", $attach, "\n";
    die "PRODUCT_XS_LOAD stamp missing\n" unless $load;
    die "PRODUCT_XS_ATTACH must be true when NYTPROF file= is set\n" unless $attach;
    print "G04_STAMP_OK\n";
  ' 2>&1
)"
STAMP_RC=$?
set -e
printf '%s\n' "$STAMP_OUT"
[[ "$STAMP_RC" -eq 0 ]] || fail "G04 stamp probe exited $STAMP_RC (want 0)"
INC_LINE="$(printf '%s\n' "$STAMP_OUT" | grep -E '^INC=' | tail -n1 || true)"
[[ -n "$INC_LINE" ]] || fail "stamp probe did not print INC="
if grep -F -q 'baseline/6.15/install' <<<"$INC_LINE"; then
  fail "loaded Devel/NYTProfM.pm is the 6.15 oracle pin: $INC_LINE"
fi
if ! grep -F -q 'collector/build/xs-nytprof' <<<"$INC_LINE"; then
  fail "loaded Devel/NYTProfM.pm is not the product dest (want collector/build/xs-nytprof): $INC_LINE"
fi
grep -F -q 'PRODUCT_XS_ATTACH=1' <<<"$STAMP_OUT" \
  || fail "PRODUCT_XS_ATTACH must be 1 when NYTPROF file= is set"
grep -F -q 'G04_STAMP_OK' <<<"$STAMP_OUT" || fail "missing G04_STAMP_OK"
ok "product module path; file= session sets PRODUCT_XS_ATTACH=1"

# Live attach: real default-calls1-shaped program. No DB::emit_* in the workload.
set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}" perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"
[[ "$RUN_RC" -eq 0 ]] || fail "perl -d:NYTProfM workload exited $RUN_RC (want 0)"
grep -E -q '^total=' <<<"$RUN_OUT" || fail "workload did not print total="
ok "live perl -d:NYTProfM ran default-calls1-shaped workload"

[[ -f "$PROFILE" ]] || fail "NYTPROF file= did not write $PROFILE"
magic="$(head -c 9 "$PROFILE" || true)"
[[ "$magic" == "NYTProf 5" ]] || fail "produced bytes must start with NYTProf 5 (got $(printf %q "$magic"))"
ok "produced bytes start with NYTProf 5"

dump_profile() {
  local profile="$1"
  local out="$2"
  set +e
  "${CLI_CMD[@]}" dump "$profile" >"$out" 2>"$out.err"
  local rc=$?
  set -e
  if [[ "$rc" -ne 0 ]]; then
    cat "$out.err" >&2 || true
    fail "nytprof-cli dump failed on $profile (rc=$rc)"
  fi
}

dump_profile "$PROFILE" "$DUMP"

has_tag() {
  local dump="$1"
  local tag="$2"
  grep -E -q "\"tag\":[[:space:]]*\"${tag}\"" "$dump"
}

has_tag "$DUMP" "SUB_RETURN" || fail "dump missing SUB_RETURN (from produced bytes)"
has_tag "$DUMP" "SUB_CALLERS" || fail "dump missing SUB_CALLERS (from produced bytes)"
has_tag "$DUMP" "TIME_LINE" || fail "dump missing TIME_LINE (default attach must stay TIME_LINE)"
if has_tag "$DUMP" "TIME_BLOCK"; then
  fail "default attach must not emit TIME_BLOCK (blocks=0)"
fi
ok "dump JSONL has SUB_RETURN + SUB_CALLERS + TIME_LINE (no TIME_BLOCK)"

set +e
"${CLI_CMD[@]}" report "$PROFILE" >"$REPORT_TXT" 2>"$REPORT_TXT.err"
REPORT_RC=$?
set -e
if [[ "$REPORT_RC" -ne 0 ]]; then
  cat "$REPORT_TXT.err" >&2 || true
  fail "nytprof-cli report failed on produced profile (rc=$REPORT_RC)"
fi

set +e
"${CLI_CMD[@]}" report --json "$PROFILE" >"$REPORT_JSON" 2>"$REPORT_JSON.err"
JSON_RC=$?
set -e
if [[ "$JSON_RC" -ne 0 ]]; then
  cat "$REPORT_JSON.err" >&2 || true
  fail "nytprof-cli report --json failed on produced profile (rc=$JSON_RC)"
fi

# Counts from the produced profile (shipped report), not hardcoded smoke constants.
# report --json leaf_returns / mid_returns / mid_leaf_edge are derived from
# ProfileModel A5 SUB_RETURN + A7 SUB_CALLERS on those bytes.
LEAF="$(perl -ne 'print $1 if /"leaf_returns"\s*:\s*(\d+)/' "$REPORT_JSON")"
MID="$(perl -ne 'print $1 if /"mid_returns"\s*:\s*(\d+)/' "$REPORT_JSON")"
EDGE="$(perl -ne 'print $1 if /"mid_leaf_edge"\s*:\s*(\d+)/' "$REPORT_JSON")"
echo "report --json: leaf_returns=${LEAF:-?} mid_returns=${MID:-?} mid_leaf_edge=${EDGE:-?}"
[[ "$LEAF" == "15" ]] || fail "leaf_returns=$LEAF (want 15) from produced profile"
[[ "$MID" == "3" ]] || fail "mid_returns=$MID (want 3) from produced profile"
[[ "$EDGE" == "15" ]] || fail "mid_leaf_edge=$EDGE (want 15) from produced profile"
grep -F -q 'main::leaf  returns=15' "$REPORT_TXT" \
  || fail "text report missing main::leaf  returns=15 (from produced profile)"
grep -F -q 'main::mid  returns=3' "$REPORT_TXT" \
  || fail "text report missing main::mid  returns=3 (from produced profile)"
ok "shipped report of produced bytes: leaf 15 / mid 3 / mid→leaf 15"

if grep -F -q 'HASH(' "$DUMP" "$REPORT_TXT"; then
  fail "caller/sub names must not stringify as HASH( (product_sub_stack frames)"
fi
ok "no HASH( callers in dump/report"

LEAF_INCL="$(perl -ne 'print $1 if /main::leaf\s+returns=15\s+excl=\S+\s+incl=(\d+)/' "$REPORT_TXT")"
echo "main::leaf incl=${LEAF_INCL:-?}"
[[ -n "$LEAF_INCL" ]] || fail "text report missing main::leaf incl="
[[ "$LEAF_INCL" -gt 0 ]] || fail "main::leaf incl=$LEAF_INCL (want > 0; live clock)"
ok "main::leaf incl=$LEAF_INCL (> 0)"

perl -e '
  my $n = 0;
  my $nonzero = 0;
  my $not_one = 0;
  while (<>) {
    next unless /"tag":"TIME_LINE"/;
    $n++;
    if (/"args":\[(\d+)/) {
      $nonzero++ if $1 > 0;
      $not_one++ if $1 != 1;
    }
  }
  die "no TIME_LINE in dump\n" unless $n;
  die "all TIME_LINE ticks are 1 (still visit-count emit)\n" unless $not_one;
  print "TIME_LINE events=$n nonzero=$nonzero not_one=$not_one\n";
' "$DUMP" || fail "TIME_LINE ticks still look like visit counts"
ok "TIME_LINE ticks are elapsed (not identically 1)"

set +e
"${CLI_CMD[@]}" verify "$PROFILE" >"$WORKDIR/verify.txt" 2>"$WORKDIR/verify.err"
VERIFY_RC=$?
set -e
if [[ "$VERIFY_RC" -ne 0 ]]; then
  cat "$WORKDIR/verify.err" >&2 || true
  fail "nytprof-cli verify failed on produced profile (rc=$VERIFY_RC) — PID pair?"
fi
grep -q '^OK:' "$WORKDIR/verify.txt" \
  || fail "verify missing OK: (see $WORKDIR/verify.txt)"
ok "verify OK (PID_START/PID_END balanced)"

has_tag "$DUMP" "SRC_LINE" || fail "dump missing SRC_LINE (finish_profiler savesrc)"
has_tag "$DUMP" "SUB_INFO" || fail "dump missing SUB_INFO (finish_profiler %DB::sub)"
SRC_N="$(perl -ne 'END { print $n+0 } $n++ if /\"tag\":[[:space:]]*\"SRC_LINE\"/' "$DUMP")"
SUBINFO_N="$(perl -ne 'END { print $n+0 } $n++ if /\"tag\":[[:space:]]*\"SUB_INFO\"/' "$DUMP")"
echo "dump SRC_LINE=$SRC_N SUB_INFO=$SUBINFO_N"
[[ "$SRC_N" -gt 0 ]] || fail "src_line events=$SRC_N (want > 0)"
[[ "$SUBINFO_N" -gt 0 ]] || fail "sub_info events=$SUBINFO_N (want > 0)"
ok "dump has SRC_LINE ($SRC_N) and SUB_INFO ($SUBINFO_N)"

grep -E -q '"tag":[[:space:]]*"SUB_INFO".*main::leaf|"args":\[[^]]*main::leaf' "$DUMP" \
  || grep -F -q 'main::leaf' "$DUMP" \
  || fail "dump SUB_INFO missing main::leaf"
perl -e '
  my ($leaf, $mid) = (0, 0);
  while (<>) {
    next unless /"tag":[[:space:]]*"SUB_INFO"/;
    $leaf++ if /main::leaf/;
    $mid++  if /main::mid/;
  }
  die "dump SUB_INFO missing main::leaf\n" unless $leaf;
  die "dump SUB_INFO missing main::mid\n" unless $mid;
  print "dump SUB_INFO names leaf=$leaf mid=$mid\n";
' "$DUMP" || fail "dump SUB_INFO names"
ok "dump SUB_INFO includes main::leaf and main::mid"

SRC_JSON="$(perl -ne 'print $1 if /"src_line_events"\s*:\s*(\d+)/' "$REPORT_JSON")"
LEAF_DEF="$(perl -ne 'print $1 if /"sub_def_leaf"\s*:\s*(null|\{[^}]*\})/' "$REPORT_JSON")"
MID_DEF="$(perl -ne 'print $1 if /"sub_def_mid"\s*:\s*(null|\{[^}]*\})/' "$REPORT_JSON")"
echo "report --json src_line_events=${SRC_JSON:-?} sub_def_leaf=${LEAF_DEF:-?} sub_def_mid=${MID_DEF:-?}"
[[ -n "$SRC_JSON" && "$SRC_JSON" -gt 0 ]] \
  || fail "report src_line_events=$SRC_JSON (want > 0)"
[[ -n "$LEAF_DEF" && "$LEAF_DEF" != "null" ]] \
  || fail "report sub_def_leaf missing/null (want main::leaf SUB_INFO)"
[[ -n "$MID_DEF" && "$MID_DEF" != "null" ]] \
  || fail "report sub_def_mid missing/null (want main::mid SUB_INFO)"
echo "$LEAF_DEF" | grep -E -q '"first_line"|"fid"' \
  || fail "sub_def_leaf not an object: $LEAF_DEF"
echo "$MID_DEF" | grep -E -q '"first_line"|"fid"' \
  || fail "sub_def_mid not an object: $MID_DEF"
ok "report/model sub_def for main::leaf and main::mid; src_line_events=$SRC_JSON"

# savesrc=0 skips file SRC_LINE; SUB_INFO + 15/3/15 + verify still hold.
SAVESRC0="$WORKDIR/savesrc0.nytprof"
SAVESRC0_DUMP="$WORKDIR/savesrc0.jsonl"
SAVESRC0_JSON="$WORKDIR/savesrc0.json"
set +e
SAVESRC0_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${SAVESRC0}:savesrc=0" \
    perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" 2>&1
)"
SAVESRC0_RC=$?
set -e
[[ "$SAVESRC0_RC" -eq 0 ]] || fail "savesrc=0 workload exited $SAVESRC0_RC"
[[ -f "$SAVESRC0" ]] || fail "savesrc=0 did not write $SAVESRC0"
dump_profile "$SAVESRC0" "$SAVESRC0_DUMP"
if has_tag "$SAVESRC0_DUMP" "SRC_LINE"; then
  fail "savesrc=0 must skip file SRC_LINE"
fi
has_tag "$SAVESRC0_DUMP" "SUB_INFO" || fail "savesrc=0 must still emit SUB_INFO"
set +e
"${CLI_CMD[@]}" report --json "$SAVESRC0" >"$SAVESRC0_JSON" 2>"$SAVESRC0_JSON.err"
SAVESRC0_JSON_RC=$?
set -e
[[ "$SAVESRC0_JSON_RC" -eq 0 ]] || fail "savesrc=0 report --json failed"
S0_LEAF="$(perl -ne 'print $1 if /"leaf_returns"\s*:\s*(\d+)/' "$SAVESRC0_JSON")"
S0_MID="$(perl -ne 'print $1 if /"mid_returns"\s*:\s*(\d+)/' "$SAVESRC0_JSON")"
S0_EDGE="$(perl -ne 'print $1 if /"mid_leaf_edge"\s*:\s*(\d+)/' "$SAVESRC0_JSON")"
S0_SRC="$(perl -ne 'print $1 if /"src_line_events"\s*:\s*(\d+)/' "$SAVESRC0_JSON")"
[[ "$S0_LEAF" == "15" && "$S0_MID" == "3" && "$S0_EDGE" == "15" ]] \
  || fail "savesrc=0 counts leaf=$S0_LEAF mid=$S0_MID edge=$S0_EDGE (want 15/3/15)"
[[ "${S0_SRC:-1}" == "0" ]] || fail "savesrc=0 src_line_events=$S0_SRC (want 0)"
set +e
"${CLI_CMD[@]}" verify "$SAVESRC0" >"$WORKDIR/savesrc0.verify" 2>"$WORKDIR/savesrc0.verify.err"
S0_VRC=$?
set -e
[[ "$S0_VRC" -eq 0 ]] || fail "savesrc=0 verify failed (rc=$S0_VRC)"
grep -q '^OK:' "$WORKDIR/savesrc0.verify" || fail "savesrc=0 verify missing OK:"
ok "savesrc=0 skips SRC_LINE; SUB_INFO + 15/3/15 + verify hold"

# G03a: trivial -e without file= still must not write nytprof.out.
LOAD_CWD="$(mktemp -d "$WORKDIR/g03a-load-XXXXXX")"
set +e
LOAD_OUT="$(
  cd "$LOAD_CWD" && env -u NYTPROF perl -I"$NYTP_DEST" -d:NYTProfM -e 'print "ok\n"' 2>&1
)"
LOAD_RC=$?
set -e
[[ "$LOAD_RC" -eq 0 ]] || fail "G03a trivial -e exited $LOAD_RC (want 0)"
grep -F -q 'ok' <<<"$LOAD_OUT" || fail "G03a trivial -e missing stdout ok"
if [[ -e "$LOAD_CWD/nytprof.out" ]]; then
  fail "G03a must not write nytprof.out (found $LOAD_CWD/nytprof.out)"
fi
ok "G03a trivial -e still writes no nytprof.out"

print_residuals
echo "product_xs_attach=1"
ok "G04 attach parity"
exit 0

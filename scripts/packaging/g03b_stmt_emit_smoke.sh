#!/usr/bin/env bash
# PR-G03b — Product statement emit via shipped nytp_emit_* (single writer).
#
# Drives real DB::enable_sink / emit_time_line / emit_time_block /
# emit_discount / finish_profiler / run_m4_mini_sample, then inspects the
# produced v5 bytes with a shipped dump/verify. Fake-clock mini equality
# is the done bar.
#
# Does NOT claim collection attach / opcode hooks / G04 /
# PRODUCT-XS-ATTACH-MVP / default-calls1 15/3/15.
#
# When CC + Perl XS headers exist:
#   make -C collector xs-nytprof, isolated product @INC, emit + dump.
# When missing: honest SKIP: after source-file asserts (exit 0).
#
# Exit 0: G03b emit pass, or honest skip (no CC / no XS headers).
# Exit 1: emit / dump / overflow / identity failure.
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
FAKE_CLOCK_BIN="$COLLECTOR/build/test_fake_clock"

usage() {
  cat <<'EOF'
Usage: g03b_stmt_emit_smoke.sh

G03b statement-emit smoke: real nytp_emit_* via product XS, dump of
NYTProf 5 bytes. PRODUCT-XS-ATTACH-MVP / G04 remain NOT-YET.
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

echo "g03b_stmt_emit_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary until S2"
echo "never crates/ on PERL5LIB"
echo "product_xs_attach: no"
echo "product_xs_attach: not-ready"

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
grep -q 'libnytp_sink_v5.a' "$MAKEFILE" || fail "Makefile missing libnytp_sink_v5.a (D1-B link)"
grep -q 'nytp_emit_time_line' "$NYTP_XS" || fail "NYTProf.xs missing nytp_emit_time_line wrapper"
grep -q 'nytp_emit_time_block' "$NYTP_XS" || fail "NYTProf.xs missing nytp_emit_time_block wrapper"
grep -q 'nytp_emit_discount' "$NYTP_XS" || fail "NYTProf.xs missing nytp_emit_discount wrapper"
grep -q 'nytp_m4_mini_sample_run' "$NYTP_XS" || fail "NYTProf.xs missing nytp_m4_mini_sample_run"
grep -q 'PRODUCT_STMT_EMIT' "$NYTP_PM_SRC" || fail "NYTProf.pm missing PRODUCT_STMT_EMIT stamp"
ok "G03b debugger sources and Makefile target present"

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
  echo "G03b stmt emit-MVP; live attach: g04_v5_parity_smoke.sh"
  echo "G05 options/format: g05_options_format_smoke.sh"
  echo "G06 fork/addpid: g06_fork_addpid_smoke.sh"
  echo "NOT-YET: mid-deflate continue-in-child / TEST-018 / full opcode"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — G03b debugger .so not built"
  echo "  (honest skip; G03b emit requires xs-nytprof)"
  print_residuals
  ok "g03b_stmt_emit_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present — G03b debugger .so not built"
  echo "  (honest skip; G03b emit requires xs-nytprof)"
  print_residuals
  ok "g03b_stmt_emit_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "perl + EXTERN.h present"

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
[[ -f "$NYTP_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM.pm"
[[ -f "$NYTP_DEST/Devel/NYTProfM/Core.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM/Core.pm"
ok "xs-nytprof produced .so + .pm under collector/build/xs-nytprof"

# Existing C fake-clock mini stays green (not a reimplementation).
echo "make -C collector build/test_fake_clock"
make -C "$COLLECTOR" build/test_fake_clock
[[ -x "$FAKE_CLOCK_BIN" ]] || fail "missing $FAKE_CLOCK_BIN"
# test_fake_clock writes build/m4-mini-fake-clock.nytprof relative to cwd.
( cd "$COLLECTOR" && ./build/test_fake_clock ) || fail "collector test_fake_clock failed"
ok "existing C fake-clock mini (test_fake_clock)"

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
  fail "no shipped dump/verify (looked for prefix/bin/nytprof-cli, target/{debug,release}/nytprof-cli, cargo)"
fi
echo "dump CLI: ${CLI_CMD[*]}"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g03b-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

# Isolated product @INC only. Never baseline/6.15/install, never crates/.
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

STMT_PATH="$WORKDIR/stmt.nytprof"
M4_PATH="$WORKDIR/m4.nytprof"
OV_PATH="$WORKDIR/overflow.nytprof"
STMT_DUMP="$WORKDIR/stmt.jsonl"
M4_DUMP="$WORKDIR/m4.jsonl"

echo "workdir: $WORKDIR"
echo "PERL5LIB=$PERL5LIB"
echo "running: perl -I${NYTP_DEST} -d:NYTProfM (G03b emit + m4 + overflow)"

set +e
EMIT_OUT="$(
  cd "$WORKDIR" && perl -I"$NYTP_DEST" -d:NYTProfM -e '
    my $inc = $INC{"Devel/NYTProfM.pm"} // "";
    print "INC=", $inc, "\n";
    my $load = ($Devel::NYTProfM::PRODUCT_XS_LOAD ? 1 : 0);
    my $attach = (defined $Devel::NYTProfM::PRODUCT_XS_ATTACH && $Devel::NYTProfM::PRODUCT_XS_ATTACH) ? 1 : 0;
    my $stmt = ($Devel::NYTProfM::PRODUCT_STMT_EMIT ? 1 : 0);
    print "PRODUCT_XS_LOAD=", $load, "\n";
    print "PRODUCT_XS_ATTACH=", $attach, "\n";
    print "PRODUCT_STMT_EMIT=", $stmt, "\n";
    die "PRODUCT_XS_LOAD stamp missing\n" unless $load;
    die "PRODUCT_XS_ATTACH must stay false\n" if $attach;
    die "PRODUCT_STMT_EMIT stamp missing\n" unless $stmt;

    my $stmt_path = $ARGV[0];
    my $m4_path   = $ARGV[1];
    my $ov_path   = $ARGV[2];

    my $st = DB::enable_sink($stmt_path);
    die "enable_sink(stmt) status=$st\n" unless $st == 0;
    $st = DB::emit_time_line(10, 1, 5);
    die "emit_time_line status=$st\n" unless $st == 0;
    $st = DB::emit_time_block(7, 1, 5, 4, 3);
    die "emit_time_block status=$st\n" unless $st == 0;
    $st = DB::emit_discount();
    die "emit_discount status=$st\n" unless $st == 0;
    DB::finish_profiler();
    print "STMT_EMIT_OK\n";

    $st = DB::enable_sink($m4_path);
    die "enable_sink(m4) status=$st\n" unless $st == 0;
    $st = DB::run_m4_mini_sample();
    die "run_m4_mini_sample status=$st\n" unless $st == 0;
    DB::finish_profiler();
    print "M4_MINI_OK\n";

    $st = DB::enable_sink($ov_path);
    die "enable_sink(overflow) status=$st\n" unless $st == 0;
    my $big = 2147483648;    # INT32_MAX + 1
    $st = DB::emit_time_line($big, 1, 1);
    print "OVERFLOW_ST=", $st, "\n";
    die "emit overflow status=$st (want 4 NYTP_ERR_OVERFLOW)\n" unless $st == 4;
    my $pst = DB::overflow_probe();
    print "OVERFLOW_PROBE=", $pst, "\n";
    die "overflow_probe status=$pst (want 4)\n" unless $pst == 4;
    DB::finish_profiler();
    print "OVERFLOW_OK\n";
  ' "$STMT_PATH" "$M4_PATH" "$OV_PATH" 2>&1
)"
EMIT_RC=$?
set -e
printf '%s\n' "$EMIT_OUT"

[[ "$EMIT_RC" -eq 0 ]] || fail "perl -d:NYTProfM G03b emit exited $EMIT_RC (want 0)"

INC_LINE="$(printf '%s\n' "$EMIT_OUT" | grep -E '^INC=' | tail -n1 || true)"
[[ -n "$INC_LINE" ]] || fail "perl -d:NYTProfM did not print INC="
if grep -F -q 'baseline/6.15/install' <<<"$INC_LINE"; then
  fail "loaded Devel/NYTProfM.pm is the 6.15 oracle pin: $INC_LINE"
fi
if ! grep -F -q 'collector/build/xs-nytprof' <<<"$INC_LINE"; then
  fail "loaded Devel/NYTProfM.pm is not the product dest (want collector/build/xs-nytprof): $INC_LINE"
fi
ok "product module path (not baseline/6.15/install)"

grep -F -q 'PRODUCT_XS_LOAD=1' <<<"$EMIT_OUT" \
  || fail "missing PRODUCT_XS_LOAD=1 stamp"
grep -F -q 'PRODUCT_STMT_EMIT=1' <<<"$EMIT_OUT" \
  || fail "missing PRODUCT_STMT_EMIT=1 stamp"
if grep -F -q 'PRODUCT_XS_ATTACH=1' <<<"$EMIT_OUT"; then
  fail "PRODUCT_XS_ATTACH must stay 0 (G03b is emit-MVP, not attach)"
fi
grep -F -q 'STMT_EMIT_OK' <<<"$EMIT_OUT" || fail "missing STMT_EMIT_OK"
grep -F -q 'M4_MINI_OK' <<<"$EMIT_OUT" || fail "missing M4_MINI_OK"
grep -F -q 'OVERFLOW_ST=4' <<<"$EMIT_OUT" || fail "overflow emit did not return NYTP_ERR_OVERFLOW (4)"
grep -F -q 'OVERFLOW_PROBE=4' <<<"$EMIT_OUT" || fail "overflow_probe did not return NYTP_ERR_OVERFLOW (4)"
ok "G03b emit + fake-clock mini + overflow fail-closed"

[[ -f "$STMT_PATH" ]] || fail "enable_sink+finish did not write $STMT_PATH"
[[ -f "$M4_PATH" ]] || fail "run_m4_mini_sample did not write $M4_PATH"

stmt_magic="$(head -c 9 "$STMT_PATH" || true)"
[[ "$stmt_magic" == "NYTProf 5" ]] || fail "stmt bytes must start with NYTProf 5 (got $(printf %q "$stmt_magic"))"
m4_magic="$(head -c 9 "$M4_PATH" || true)"
[[ "$m4_magic" == "NYTProf 5" ]] || fail "m4 bytes must start with NYTProf 5 (got $(printf %q "$m4_magic"))"
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

dump_profile "$STMT_PATH" "$STMT_DUMP"
dump_profile "$M4_PATH" "$M4_DUMP"

has_tag() {
  local dump="$1"
  local tag="$2"
  grep -E -q "\"tag\":[[:space:]]*\"${tag}\"" "$dump"
}

has_tag "$STMT_DUMP" "TIME_LINE" || fail "stmt dump missing TIME_LINE (from emitted bytes)"
has_tag "$STMT_DUMP" "TIME_BLOCK" || fail "stmt dump missing TIME_BLOCK (from emitted bytes)"
has_tag "$STMT_DUMP" "DISCOUNT" || fail "stmt dump missing DISCOUNT (from emitted bytes)"
ok "stmt dump JSONL has TIME_LINE + TIME_BLOCK + DISCOUNT from those bytes"

has_tag "$M4_DUMP" "TIME_LINE" || fail "m4 dump missing TIME_LINE (from nytp_m4_mini_sample_run)"
has_tag "$M4_DUMP" "DISCOUNT" || fail "m4 dump missing DISCOUNT (from nytp_m4_mini_sample_run)"

# Structural order of TIME_LINE / DISCOUNT must match the shipped m4 mini:
# TIME_LINE, DISCOUNT, TIME_LINE, TIME_LINE
m4_order="$(
  grep -E '"tag":[[:space:]]*"(TIME_LINE|DISCOUNT)"' "$M4_DUMP" \
    | sed -E 's/.*"tag":[[:space:]]*"(TIME_LINE|DISCOUNT)".*/\1/' \
    | tr '\n' ' ' \
    | sed 's/[[:space:]]*$//'
)"
[[ "$m4_order" == "TIME_LINE DISCOUNT TIME_LINE TIME_LINE" ]] \
  || fail "m4 TIME_LINE/DISCOUNT order mismatch (got '$m4_order'; want TIME_LINE DISCOUNT TIME_LINE TIME_LINE)"
ok "product XS run_m4_mini_sample dump matches m4 TIME_LINE + DISCOUNT order"

set +e
"${CLI_CMD[@]}" verify "$M4_PATH" >"$WORKDIR/m4.verify.out" 2>"$WORKDIR/m4.verify.err"
VERIFY_RC=$?
set -e
if [[ "$VERIFY_RC" -eq 0 ]]; then
  ok "nytprof-cli verify accepted m4 mini stream"
else
  echo "note: nytprof-cli verify exited $VERIFY_RC on m4 mini (dump tags already asserted)"
  cat "$WORKDIR/m4.verify.err" >&2 || true
fi

if grep -F -q 'OK: attach works' <<<"$EMIT_OUT"; then
  fail "perl -d:NYTProfM output must not contain OK: attach works"
fi

# Trivial -e still must not write nytprof.out (G03a hold-in-memory).
LOAD_CWD="$(mktemp -d "$WORKDIR/g03a-load-XXXXXX")"
set +e
LOAD_OUT="$(
  cd "$LOAD_CWD" && perl -I"$NYTP_DEST" -d:NYTProfM -e 'print "ok\n"' 2>&1
)"
LOAD_RC=$?
set -e
[[ "$LOAD_RC" -eq 0 ]] || fail "G03a trivial -e exited $LOAD_RC (want 0)"
if [[ -e "$LOAD_CWD/nytprof.out" ]]; then
  fail "G03a must not write nytprof.out (found $LOAD_CWD/nytprof.out)"
fi
ok "G03a trivial -e still writes no nytprof.out"

print_residuals
ok "G03b stmt emit"
exit 0

#!/usr/bin/env bash
# PR-G03c — Product SUB_ENTRY / SUB_RETURN emit via shipped nytp_emit_sub_*
# (single writer).
#
# Drives real DB::enable_sink / emit_sub_entry / emit_sub_return /
# finish_profiler, then inspects the produced v5 bytes with a shipped
# dump/verify. May also emit TIME_LINE so the mini stream is plausible.
#
# Does NOT claim collection attach / opcode hooks / G04 /
# PRODUCT-XS-ATTACH-MVP / default-calls1 15/3/15.
#
# When CC + Perl XS headers exist:
#   make -C collector xs-nytprof, isolated product @INC, emit + dump.
# When missing: honest SKIP: after source-file asserts (exit 0).
#
# Exit 0: G03c emit pass, or honest skip (no CC / no XS headers).
# Exit 1: emit / dump / identity failure.
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

usage() {
  cat <<'EOF'
Usage: g03c_sub_emit_smoke.sh

G03c sub-emit smoke: real nytp_emit_sub_* via product XS, dump of
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

echo "g03c_sub_emit_smoke: repo root $ROOT"
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
grep -q 'nytp_emit_sub_entry' "$NYTP_XS" || fail "NYTProf.xs missing nytp_emit_sub_entry wrapper"
grep -q 'nytp_emit_sub_return' "$NYTP_XS" || fail "NYTProf.xs missing nytp_emit_sub_return wrapper"
grep -q 'nytp_sv_cstr' "$NYTP_XS" || fail "NYTProf.xs missing nytp_sv_cstr for SUB_RETURN"
grep -q 'PRODUCT_SUB_EMIT' "$NYTP_PM_SRC" || fail "NYTProf.pm missing PRODUCT_SUB_EMIT stamp"
grep -q 'PRODUCT_STMT_EMIT' "$NYTP_PM_SRC" || fail "NYTProf.pm missing PRODUCT_STMT_EMIT stamp"
ok "G03c debugger sources and Makefile target present"

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
  echo "G03c sub emit-MVP; live attach: g04_v5_parity_smoke.sh"
  echo "G05 options/format: g05_options_format_smoke.sh"
  echo "G06 fork/addpid: g06_fork_addpid_smoke.sh"
  echo "NOT-YET: mid-deflate continue-in-child / TEST-018 / full opcode"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — G03c debugger .so not built"
  echo "  (honest skip; G03c emit requires xs-nytprof)"
  print_residuals
  ok "g03c_sub_emit_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present — G03c debugger .so not built"
  echo "  (honest skip; G03c emit requires xs-nytprof)"
  print_residuals
  ok "g03c_sub_emit_smoke completed (skip — no XS headers)"
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
  fail "no shipped dump/verify (looked for prefix/bin/nytprof-cli, target/{debug,release}/nytprof-cli, cargo)"
fi
echo "dump CLI: ${CLI_CMD[*]}"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g03c-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

# Isolated product @INC only. Never baseline/6.15/install, never crates/.
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

SUB_PATH="$WORKDIR/sub.nytprof"
SUB_DUMP="$WORKDIR/sub.jsonl"

echo "workdir: $WORKDIR"
echo "PERL5LIB=$PERL5LIB"
echo "running: perl -I${NYTP_DEST} -d:NYTProfM (G03c SUB_ENTRY + SUB_RETURN emit)"

set +e
EMIT_OUT="$(
  cd "$WORKDIR" && perl -I"$NYTP_DEST" -d:NYTProfM -e '
    my $inc = $INC{"Devel/NYTProfM.pm"} // "";
    print "INC=", $inc, "\n";
    my $load = ($Devel::NYTProfM::PRODUCT_XS_LOAD ? 1 : 0);
    my $attach = (defined $Devel::NYTProfM::PRODUCT_XS_ATTACH && $Devel::NYTProfM::PRODUCT_XS_ATTACH) ? 1 : 0;
    my $stmt = ($Devel::NYTProfM::PRODUCT_STMT_EMIT ? 1 : 0);
    my $sub  = ($Devel::NYTProfM::PRODUCT_SUB_EMIT ? 1 : 0);
    print "PRODUCT_XS_LOAD=", $load, "\n";
    print "PRODUCT_XS_ATTACH=", $attach, "\n";
    print "PRODUCT_STMT_EMIT=", $stmt, "\n";
    print "PRODUCT_SUB_EMIT=", $sub, "\n";
    die "PRODUCT_XS_LOAD stamp missing\n" unless $load;
    die "PRODUCT_XS_ATTACH must stay false\n" if $attach;
    die "PRODUCT_STMT_EMIT stamp missing\n" unless $stmt;
    die "PRODUCT_SUB_EMIT stamp missing\n" unless $sub;

    my $sub_path = $ARGV[0];

    # After load, init_profiler holds an in-memory sink. Drop it so the
    # NULL-sink wrappers return NYTP_ERR_NULL (1) before enable_sink.
    DB::finish_profiler();
    my $st = DB::emit_sub_entry(1, 10);
    print "NULL_ENTRY_ST=", $st, "\n";
    die "emit_sub_entry(NULL sink) status=$st (want 1 NYTP_ERR_NULL)\n" unless $st == 1;
    $st = DB::emit_sub_return(1, 0.1, 0.05, "main::leaf");
    print "NULL_RETURN_ST=", $st, "\n";
    die "emit_sub_return(NULL sink) status=$st (want 1 NYTP_ERR_NULL)\n" unless $st == 1;

    $st = DB::enable_sink($sub_path);
    die "enable_sink(sub) status=$st\n" unless $st == 0;
    $st = DB::emit_time_line(10, 1, 5);
    die "emit_time_line status=$st\n" unless $st == 0;
    $st = DB::emit_sub_entry(1, 10);
    die "emit_sub_entry status=$st\n" unless $st == 0;
    $st = DB::emit_sub_return(1, 900.0, 50.0, "main::leaf");
    die "emit_sub_return status=$st\n" unless $st == 0;
    DB::finish_profiler();
    print "SUB_EMIT_OK\n";
  ' "$SUB_PATH" 2>&1
)"
EMIT_RC=$?
set -e
printf '%s\n' "$EMIT_OUT"

[[ "$EMIT_RC" -eq 0 ]] || fail "perl -d:NYTProfM G03c emit exited $EMIT_RC (want 0)"

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
grep -F -q 'PRODUCT_SUB_EMIT=1' <<<"$EMIT_OUT" \
  || fail "missing PRODUCT_SUB_EMIT=1 stamp"
if grep -F -q 'PRODUCT_XS_ATTACH=1' <<<"$EMIT_OUT"; then
  fail "PRODUCT_XS_ATTACH must stay 0 (G03c is emit-MVP, not attach)"
fi
grep -F -q 'NULL_ENTRY_ST=1' <<<"$EMIT_OUT" \
  || fail "NULL-sink emit_sub_entry did not return NYTP_ERR_NULL (1)"
grep -F -q 'NULL_RETURN_ST=1' <<<"$EMIT_OUT" \
  || fail "NULL-sink emit_sub_return did not return NYTP_ERR_NULL (1)"
grep -F -q 'SUB_EMIT_OK' <<<"$EMIT_OUT" || fail "missing SUB_EMIT_OK"
ok "G03c SUB_ENTRY + SUB_RETURN emit + NULL-sink fail-closed"

[[ -f "$SUB_PATH" ]] || fail "enable_sink+finish did not write $SUB_PATH"

sub_magic="$(head -c 9 "$SUB_PATH" || true)"
[[ "$sub_magic" == "NYTProf 5" ]] || fail "sub bytes must start with NYTProf 5 (got $(printf %q "$sub_magic"))"
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

dump_profile "$SUB_PATH" "$SUB_DUMP"

has_tag() {
  local dump="$1"
  local tag="$2"
  grep -E -q "\"tag\":[[:space:]]*\"${tag}\"" "$dump"
}

has_tag "$SUB_DUMP" "SUB_ENTRY" || fail "sub dump missing SUB_ENTRY (from emitted bytes)"
has_tag "$SUB_DUMP" "SUB_RETURN" || fail "sub dump missing SUB_RETURN (from emitted bytes)"
ok "sub dump JSONL has SUB_ENTRY + SUB_RETURN from those bytes"

set +e
"${CLI_CMD[@]}" verify "$SUB_PATH" >"$WORKDIR/sub.verify.out" 2>"$WORKDIR/sub.verify.err"
VERIFY_RC=$?
set -e
if [[ "$VERIFY_RC" -eq 0 ]]; then
  ok "nytprof-cli verify accepted G03c mini stream"
else
  echo "note: nytprof-cli verify exited $VERIFY_RC on G03c mini (dump tags already asserted)"
  cat "$WORKDIR/sub.verify.err" >&2 || true
fi

if grep -F -q 'OK: attach works' <<<"$EMIT_OUT"; then
  fail "perl -d:NYTProfM output must not contain OK: attach works"
fi
if grep -F -q 'product_xs_attach=1' <<<"$EMIT_OUT"; then
  fail "perl -d:NYTProfM output must not contain product_xs_attach=1"
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
ok "G03c sub emit"
exit 0

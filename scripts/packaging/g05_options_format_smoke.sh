#!/usr/bin/env bash
# PR-G05 — Product NYTPROF options + format=v6 D1-A / D1-B.
#
# Drives real `perl -d:NYTProfM` (product tree, not baseline/6.15/install):
#   - unknown option fail-closed
#   - use_db_sub=2 / wrap=2 / entersub=2 fail-closed (0/1 only)
#   - use_db_sub=1 / wrap=1 / entersub=1 load (stamps 1) with no hook change
#   - format=dual rejected
#   - D1-B format=v6 fail-closed (v6_collect rebuild message; no NYTPROF6 file)
#   - default / format=v5 live attach still leaf 15 / mid 3 / mid→leaf 15
#   - D1-A (xs-nytprof-v6) format=v6 writes NYTPROF6 when zstd/lz4 exist
#
# Counts come from produced v5 bytes (not DB::emit_* probes).
# collection_default stays v5. dual_path stays oracle-primary. G06 residual.
#
# Exit 0: G05 pass, or honest skip (no CC / no XS headers).
# Exit 1: option / format / parity failure.
# Exit 2: wrapper misuse or crates/ on PERL5LIB.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

COLLECTOR="$ROOT/collector"
MAKEFILE="$COLLECTOR/Makefile"
NYTP_DEST="$COLLECTOR/build/xs-nytprof"
NYTP_SO="$NYTP_DEST/auto/Devel/NYTProfM/NYTProfM.so"
NYTP_V6_DEST="$COLLECTOR/build/xs-nytprof-v6"
NYTP_V6_SO="$NYTP_V6_DEST/auto/Devel/NYTProfM/NYTProfM.so"
NYTP_PM_SRC="$COLLECTOR/xs/Devel/NYTProfM.pm"
NYTP_CORE_SRC="$COLLECTOR/xs/Devel/NYTProfM/Core.pm"
NYTP_XS="$COLLECTOR/xs/NYTProf.xs"
WORKLOAD="$ROOT/fixtures/v5/default-calls1/workload.pl"
V6_MSG="format=v6 requires v6-enabled build (install v6_collect package or rebuild with --with v6_collect)"

usage() {
  cat <<'EOF'
Usage: g05_options_format_smoke.sh

G05 options+format: unknown/dual fail-closed; wrap/entersub/use_db_sub 0/1;
D1-B format=v6 fail-closed; default/format=v5 live attach 15/3/15;
D1-A format=v6 → NYTPROF6.
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

echo "g05_options_format_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary until S2"
echo "never crates/ on PERL5LIB"
echo "G05 options + format=v6 D1-A/D1-B; not G06/full opcode"

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
grep -q 'xs-nytprof-v6' "$MAKEFILE" || fail "Makefile missing xs-nytprof-v6 (D1-A)"
grep -q 'libnytp_sink_v5.a' "$MAKEFILE" || fail "Makefile missing libnytp_sink_v5.a (D1-B)"
grep -q 'enable_sink_v6' "$NYTP_XS" || fail "NYTProf.xs missing enable_sink_v6"
grep -q 'product_v6_collect' "$NYTP_XS" || fail "NYTProf.xs missing product_v6_collect"
grep -q 'NYTPROF_V6_COLLECT' "$NYTP_XS" || fail "NYTProf.xs missing NYTPROF_V6_COLLECT ifdef"
grep -q '_product_parse_nytprof' "$NYTP_PM_SRC" || fail "NYTProf.pm missing NYTPROF parser"
grep -F -q 'unknown NYTPROF option' "$NYTP_PM_SRC" || fail "NYTProf.pm missing unknown-option croak"
grep -F -q 'format=dual is rejected' "$NYTP_PM_SRC" || fail "NYTProf.pm missing format=dual reject"
grep -F -q 'v6_collect' "$NYTP_PM_SRC" || fail "NYTProf.pm missing v6_collect fail-closed text"
# DI-03 E0: wrap/entersub must be known (else E1a entersub=1 dies unknown).
grep -F -q 'wrap entersub' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing wrap/entersub in %PRODUCT_NYTPROF_KNOWN"
grep -q 'PRODUCT_USE_DB_SUB' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing PRODUCT_USE_DB_SUB stamp"
grep -q 'PRODUCT_ENTERSUB' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing PRODUCT_ENTERSUB stamp"
grep -E -q '\$Devel::NYTProfM::PRODUCT_WRAP[[:space:]]*=' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing PRODUCT_WRAP stamp"
ok "G05 sources, parser, D1-A Makefile target present"

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
  echo "G06 fork/addpid: g06_fork_addpid_smoke.sh"
  echo "NOT-YET: mid-deflate continue-in-child / TEST-018"
  echo "DI-01 blocks=1 780/810: di01_blocks_780_smoke.sh"
  echo "DI-03 opcode/entersub: in progress, not done (E0 parse/stamp only)"
  echo "NOT-YET: full 6.15 opcode/entersub install / DI-02 SUB_ENTRY 27"
  echo "NOT-YET: PRODUCT-V6-COLLECT-EL8 / CPAN-TRIAL / EL8 RPM"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — G05 debugger .so not built"
  echo "  (honest skip; options/format require xs-nytprof)"
  print_residuals
  ok "g05_options_format_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present — G05 debugger .so not built"
  echo "  (honest skip; options/format require xs-nytprof)"
  print_residuals
  ok "g05_options_format_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "perl + EXTERN.h present"

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
[[ -f "$NYTP_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM.pm"
ok "D1-B xs-nytprof produced .so + .pm"

# D1-B must not pull zstd/lz4.
if command -v ldd >/dev/null 2>&1; then
  LDD_B="$(ldd "$NYTP_SO" 2>/dev/null || true)"
  if grep -E -q 'libzstd|liblz4' <<<"$LDD_B"; then
    fail "D1-B NYTProfM.so must not link zstd/lz4: $LDD_B"
  fi
  ok "D1-B .so is -lz only (no libzstd/liblz4)"
fi

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

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g05-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

echo "workdir: $WORKDIR"
echo "PERL5LIB=$PERL5LIB (D1-B)"

# --- unknown option fail-closed ---
UNK_PATH="$WORKDIR/unknown.out"
set +e
UNK_OUT="$(
  cd "$WORKDIR" && NYTPROF="notanoption=1:file=${UNK_PATH}" \
    perl -I"$NYTP_DEST" -d:NYTProfM -e 'print "UNEXPECTED_OK\n"' 2>&1
)"
UNK_RC=$?
set -e
printf '%s\n' "$UNK_OUT"
[[ "$UNK_RC" -ne 0 ]] || fail "unknown NYTPROF option must fail-closed (got exit 0)"
grep -F -q 'unknown NYTPROF option' <<<"$UNK_OUT" \
  || fail "unknown-option croak missing greppable text"
if grep -F -q 'UNEXPECTED_OK' <<<"$UNK_OUT"; then
  fail "unknown option must abort before the program runs"
fi
if [[ -e "$UNK_PATH" ]]; then
  fail "unknown option must not write $UNK_PATH"
fi
ok "unknown NYTPROF option fail-closed (no file)"

# --- DI-03 E0: wrap / entersub / use_db_sub 0/1 only ---
assert_opt_oor() {
  local key="$1"
  local path="$WORKDIR/${key}-oor.out"
  set +e
  local out rc
  out="$(
    cd "$WORKDIR" && NYTPROF="${key}=2:file=${path}" \
      perl -I"$NYTP_DEST" -d:NYTProfM -e 'print "UNEXPECTED_OK\n"' 2>&1
  )"
  rc=$?
  set -e
  printf '%s\n' "$out"
  [[ "$rc" -ne 0 ]] || fail "${key}=2 must fail-closed (got exit 0)"
  grep -F -q "unknown NYTPROF option: ${key}" <<<"$out" \
    || fail "${key}=2 missing greppable out-of-range croak"
  if grep -F -q 'UNEXPECTED_OK' <<<"$out"; then
    fail "${key}=2 must abort before the program runs"
  fi
  if [[ -e "$path" ]]; then
    fail "${key}=2 must not write $path"
  fi
  ok "${key}=2 fail-closed (no file)"
}
assert_opt_oor use_db_sub
assert_opt_oor wrap
assert_opt_oor entersub

# --- format=dual rejected ---
DUAL_PATH="$WORKDIR/dual.out"
set +e
DUAL_OUT="$(
  cd "$WORKDIR" && NYTPROF="format=dual:file=${DUAL_PATH}" \
    perl -I"$NYTP_DEST" -d:NYTProfM -e 'print "UNEXPECTED_OK\n"' 2>&1
)"
DUAL_RC=$?
set -e
printf '%s\n' "$DUAL_OUT"
[[ "$DUAL_RC" -ne 0 ]] || fail "format=dual must be rejected (got exit 0)"
grep -F -q 'format=dual is rejected' <<<"$DUAL_OUT" \
  || fail "format=dual reject missing greppable text"
if [[ -e "$DUAL_PATH" ]]; then
  fail "format=dual must not write $DUAL_PATH"
fi
ok "format=dual rejected (no file)"

# --- D1-B format=v6 fail-closed ---
V6B_PATH="$WORKDIR/d1b-v6.out"
set +e
V6B_OUT="$(
  cd "$WORKDIR" && NYTPROF="format=v6:file=${V6B_PATH}" \
    perl -I"$NYTP_DEST" -d:NYTProfM -e 'print "UNEXPECTED_OK\n"' 2>&1
)"
V6B_RC=$?
set -e
printf '%s\n' "$V6B_OUT"
[[ "$V6B_RC" -ne 0 ]] || fail "D1-B format=v6 must fail-closed (got exit 0)"
grep -F -q 'v6_collect' <<<"$V6B_OUT" \
  || fail "D1-B format=v6 missing v6_collect rebuild message"
grep -F -q "$V6_MSG" <<<"$V6B_OUT" \
  || fail "D1-B format=v6 missing DoD rebuild string"
if [[ -e "$V6B_PATH" ]]; then
  fail "D1-B format=v6 must not write $V6B_PATH"
fi
if compgen -G "$WORKDIR/*v6*" > /dev/null; then
  for f in "$WORKDIR"/*v6*; do
    if [[ -f "$f" ]] && head -c 8 "$f" 2>/dev/null | grep -q NYTPROF6; then
      fail "D1-B format=v6 wrote NYTPROF6 file $f"
    fi
  done
fi
ok "D1-B format=v6 fail-closed (v6_collect message; no v6 file)"

# D1-B stamp: PRODUCT_V6_COLLECT=0
set +e
STAMP_B="$(
  cd "$WORKDIR" && env -u NYTPROF perl -I"$NYTP_DEST" -d:NYTProfM -e '
    my $inc = $INC{"Devel/NYTProfM.pm"} // "";
    print "INC=", $inc, "\n";
    my $v6 = ($Devel::NYTProfM::PRODUCT_V6_COLLECT ? 1 : 0);
    print "PRODUCT_V6_COLLECT=", $v6, "\n";
    die "D1-B PRODUCT_V6_COLLECT must be 0\n" if $v6;
    print "D1B_STAMP_OK\n";
  ' 2>&1
)"
STAMP_B_RC=$?
set -e
printf '%s\n' "$STAMP_B"
[[ "$STAMP_B_RC" -eq 0 ]] || fail "D1-B stamp probe exited $STAMP_B_RC"
grep -F -q 'collector/build/xs-nytprof' <<<"$STAMP_B" \
  || fail "D1-B stamp not product dest"
if grep -F -q 'baseline/6.15/install' <<<"$STAMP_B"; then
  fail "D1-B loaded 6.15 oracle pin"
fi
grep -F -q 'PRODUCT_V6_COLLECT=0' <<<"$STAMP_B" || fail "D1-B PRODUCT_V6_COLLECT must be 0"
ok "D1-B PRODUCT_V6_COLLECT=0"

# DI-03 E0: default stamps 0; =1 stamps 1; attach stays wrap + C DBSTATE.
assert_attach_stamps() {
  local label="$1"
  local nytprof="$2"
  local want_use="$3"
  local want_wrap="$4"
  local want_ent="$5"
  local profile="$6"
  set +e
  local out rc
  out="$(
    cd "$WORKDIR" && NYTPROF="$nytprof" perl -I"$NYTP_DEST" -d:NYTProfM -e '
      my $inc = $INC{"Devel/NYTProfM.pm"} // "";
      print "INC=", $inc, "\n";
      print "PRODUCT_USE_DB_SUB=", ($Devel::NYTProfM::PRODUCT_USE_DB_SUB ? 1 : 0), "\n";
      print "PRODUCT_WRAP=", ($Devel::NYTProfM::PRODUCT_WRAP ? 1 : 0), "\n";
      print "PRODUCT_ENTERSUB=", ($Devel::NYTProfM::PRODUCT_ENTERSUB ? 1 : 0), "\n";
      print "P_SUB=", (($^P & 0x01) ? 1 : 0), "\n";
      print "DBSTATE_LINE=", ($Devel::NYTProfM::PRODUCT_DBSTATE_LINE ? 1 : 0), "\n";
      print "XS_ATTACH=", ($Devel::NYTProfM::PRODUCT_XS_ATTACH ? 1 : 0), "\n";
      if (DB->can("install_product_entersub")) {
        die "E0 must not expose DB::install_product_entersub\n";
      }
      print "STAMP_OK\n";
    ' 2>&1
  )"
  rc=$?
  set -e
  printf '%s\n' "$out"
  [[ "$rc" -eq 0 ]] || fail "$label: stamp probe exited $rc"
  grep -F -q 'collector/build/xs-nytprof' <<<"$out" \
    || fail "$label: stamp not product dest"
  if grep -F -q 'baseline/6.15/install' <<<"$out"; then
    fail "$label: loaded 6.15 oracle pin"
  fi
  grep -F -q "PRODUCT_USE_DB_SUB=${want_use}" <<<"$out" \
    || fail "$label: PRODUCT_USE_DB_SUB want $want_use"
  grep -F -q "PRODUCT_WRAP=${want_wrap}" <<<"$out" \
    || fail "$label: PRODUCT_WRAP want $want_wrap"
  grep -F -q "PRODUCT_ENTERSUB=${want_ent}" <<<"$out" \
    || fail "$label: PRODUCT_ENTERSUB want $want_ent"
  grep -F -q 'P_SUB=1' <<<"$out" \
    || fail "$label: PERLDBf_SUB (0x01) must stay on (wrap attach unchanged)"
  grep -F -q 'DBSTATE_LINE=1' <<<"$out" \
    || fail "$label: C OP_DBSTATE must stay installed"
  grep -F -q 'XS_ATTACH=1' <<<"$out" \
    || fail "$label: PRODUCT_XS_ATTACH must be 1 with file="
  grep -F -q 'STAMP_OK' <<<"$out" || fail "$label: missing STAMP_OK"
  [[ -f "$profile" ]] || fail "$label: missing profile $profile"
  local magic
  magic="$(head -c 9 "$profile" || true)"
  [[ "$magic" == "NYTProf 5" ]] || fail "$label: want NYTProf 5 (got $(printf %q "$magic"))"
  ok "$label: stamps + wrap/DBSTATE attach unchanged"
}

assert_attach_stamps "default stamps 0" \
  "file=${WORKDIR}/stamp-default.nytprof" \
  0 0 0 "$WORKDIR/stamp-default.nytprof"
assert_attach_stamps "use_db_sub=1" \
  "use_db_sub=1:file=${WORKDIR}/stamp-usedb.nytprof" \
  1 1 0 "$WORKDIR/stamp-usedb.nytprof"
assert_attach_stamps "wrap=1" \
  "wrap=1:file=${WORKDIR}/stamp-wrap.nytprof" \
  0 1 0 "$WORKDIR/stamp-wrap.nytprof"
assert_attach_stamps "entersub=1" \
  "entersub=1:file=${WORKDIR}/stamp-entersub.nytprof" \
  0 0 1 "$WORKDIR/stamp-entersub.nytprof"
# wrap wins over entersub: stamp both; still wrap attach (no opcode).
assert_attach_stamps "wrap=1:entersub=1" \
  "wrap=1:entersub=1:file=${WORKDIR}/stamp-both.nytprof" \
  0 1 1 "$WORKDIR/stamp-both.nytprof"

assert_v5_parity() {
  local label="$1"
  local nytprof="$2"
  local profile="$3"
  local dest="$4"
  set +e
  local out
  out="$(
    cd "$WORKDIR" && NYTPROF="$nytprof" perl -I"$dest" -d:NYTProfM "$WORKLOAD" 2>&1
  )"
  local rc=$?
  set -e
  printf '%s\n' "$out"
  [[ "$rc" -eq 0 ]] || fail "$label: perl -d:NYTProfM exited $rc"
  [[ -f "$profile" ]] || fail "$label: missing profile $profile"
  local magic
  magic="$(head -c 9 "$profile" || true)"
  [[ "$magic" == "NYTProf 5" ]] || fail "$label: want NYTProf 5 (got $(printf %q "$magic"))"
  local json="$profile.json"
  set +e
  "${CLI_CMD[@]}" report --json "$profile" >"$json" 2>"$json.err"
  local jrc=$?
  set -e
  if [[ "$jrc" -ne 0 ]]; then
    cat "$json.err" >&2 || true
    fail "$label: nytprof-cli report --json failed (rc=$jrc)"
  fi
  local leaf mid edge
  leaf="$(perl -ne 'print $1 if /"leaf_returns"\s*:\s*(\d+)/' "$json")"
  mid="$(perl -ne 'print $1 if /"mid_returns"\s*:\s*(\d+)/' "$json")"
  edge="$(perl -ne 'print $1 if /"mid_leaf_edge"\s*:\s*(\d+)/' "$json")"
  echo "$label report --json: leaf_returns=${leaf:-?} mid_returns=${mid:-?} mid_leaf_edge=${edge:-?}"
  [[ "$leaf" == "15" ]] || fail "$label: leaf_returns=$leaf (want 15)"
  [[ "$mid" == "3" ]] || fail "$label: mid_returns=$mid (want 3)"
  [[ "$edge" == "15" ]] || fail "$label: mid_leaf_edge=$edge (want 15)"
  ok "$label: NYTProf 5 + leaf 15 / mid 3 / mid→leaf 15"
}

assert_v5_parity "default file=" "file=${WORKDIR}/default.nytprof" \
  "$WORKDIR/default.nytprof" "$NYTP_DEST"
assert_v5_parity "format=v5" "format=v5:file=${WORKDIR}/v5.nytprof" \
  "$WORKDIR/v5.nytprof" "$NYTP_DEST"

# Known residual option must not fail-closed (stmts=1 is advertised work).
assert_v5_parity "stmts=1:file=" "stmts=1:file=${WORKDIR}/stmts.nytprof" \
  "$WORKDIR/stmts.nytprof" "$NYTP_DEST"

# DI-03 E0: wrap=1 + entersub=1 must not change attach counts (no opcode yet).
assert_v5_parity "wrap=1:entersub=1" \
  "wrap=1:entersub=1:file=${WORKDIR}/wrap-entersub.nytprof" \
  "$WORKDIR/wrap-entersub.nytprof" "$NYTP_DEST"

# G03a: no file= still no nytprof.out
LOAD_CWD="$(mktemp -d "$WORKDIR/g03a-load-XXXXXX")"
set +e
LOAD_OUT="$(
  cd "$LOAD_CWD" && env -u NYTPROF perl -I"$NYTP_DEST" -d:NYTProfM -e 'print "ok\n"' 2>&1
)"
LOAD_RC=$?
set -e
[[ "$LOAD_RC" -eq 0 ]] || fail "G03a trivial -e exited $LOAD_RC"
grep -F -q 'ok' <<<"$LOAD_OUT" || fail "G03a trivial -e missing stdout ok"
if [[ -e "$LOAD_CWD/nytprof.out" ]]; then
  fail "G03a must not write nytprof.out"
fi
ok "G03a trivial -e still writes no nytprof.out"

# --- D1-A format=v6 ---
have_v6_headers=0
if [[ -f /usr/include/zstd.h && -f /usr/include/lz4.h ]]; then
  have_v6_headers=1
fi

if [[ "$have_v6_headers" -ne 1 ]]; then
  echo "SKIP: zstd.h/lz4.h absent — D1-A xs-nytprof-v6 not built"
  echo "  (honest skip; D1-A test path is make xs-nytprof-v6 + format=v6)"
  grep -q 'xs-nytprof-v6' "$MAKEFILE" || fail "D1-A skip requires xs-nytprof-v6 target in Makefile"
  grep -q 'enable_sink_v6' "$NYTP_XS" || fail "D1-A skip requires enable_sink_v6 in XS"
else
  echo "make -C collector xs-nytprof-v6"
  make -C "$COLLECTOR" xs-nytprof-v6
  [[ -f "$NYTP_V6_SO" ]] || fail "xs-nytprof-v6 did not produce $NYTP_V6_SO"
  [[ -f "$NYTP_V6_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof-v6 did not copy .pm"
  ok "D1-A xs-nytprof-v6 produced .so + .pm"

  V6A_PATH="$WORKDIR/d1a-v6.out"
  set +e
  V6A_OUT="$(
    cd "$WORKDIR" && PERL5LIB="$NYTP_V6_DEST" NYTPROF="format=v6:file=${V6A_PATH}" \
      perl -I"$NYTP_V6_DEST" -d:NYTProfM -e '
        my $inc = $INC{"Devel/NYTProfM.pm"} // "";
        print "INC=", $inc, "\n";
        my $v6 = ($Devel::NYTProfM::PRODUCT_V6_COLLECT ? 1 : 0);
        print "PRODUCT_V6_COLLECT=", $v6, "\n";
        die "D1-A PRODUCT_V6_COLLECT must be 1\n" unless $v6;
        print "hello\n";
      ' 2>&1
  )"
  V6A_RC=$?
  set -e
  printf '%s\n' "$V6A_OUT"
  [[ "$V6A_RC" -eq 0 ]] || fail "D1-A format=v6 perl -d:NYTProfM exited $V6A_RC"
  grep -F -q 'xs-nytprof-v6' <<<"$V6A_OUT" || fail "D1-A INC not xs-nytprof-v6"
  grep -F -q 'PRODUCT_V6_COLLECT=1' <<<"$V6A_OUT" || fail "D1-A PRODUCT_V6_COLLECT must be 1"
  [[ -f "$V6A_PATH" ]] || fail "D1-A format=v6 did not write $V6A_PATH"
  magic6="$(head -c 8 "$V6A_PATH" || true)"
  [[ "$magic6" == "NYTPROF6" ]] || fail "D1-A format=v6 want NYTPROF6 (got $(printf %q "$magic6"))"
  ok "D1-A format=v6 wrote NYTPROF6"
fi

print_residuals
ok "G05 options+format"
exit 0

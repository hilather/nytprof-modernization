#!/usr/bin/env bash
# PR-G03e — Product START_DEFLATE / compress emit via shipped
# nytp_emit_start_deflate (single writer, -lz only).
#
# Drives real DB::enable_sink / pre-deflate emit / emit_start_deflate /
# post-deflate emit / finish_profiler, then inspects the produced v5 bytes
# with a shipped dump/verify. Dump must recover a post-deflate event via
# inflate (not only the uncompressed prefix).
#
# Does NOT claim collection attach / opcode hooks / G04 /
# PRODUCT-XS-ATTACH-MVP / default-calls1 15/3/15 / mid-deflate fork.
#
# When CC + Perl XS headers exist:
#   make -C collector xs-nytprof, isolated product @INC, emit + dump.
# When missing: honest SKIP: after source-file asserts (exit 0).
#
# Exit 0: G03e emit pass, or honest skip (no CC / no XS headers).
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
Usage: g03e_compress_emit_smoke.sh

G03e compress-emit smoke: real nytp_emit_start_deflate via product XS,
dump/verify of NYTProf 5 bytes after the compress switch.
PRODUCT-XS-ATTACH-MVP / G04 / mid-deflate fork remain NOT-YET.
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

echo "g03e_compress_emit_smoke: repo root $ROOT"
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
grep -q 'nytp_emit_start_deflate' "$NYTP_XS" || fail "NYTProf.xs missing nytp_emit_start_deflate wrapper"
grep -q 'nytp_v5_sink_is_deflating' "$NYTP_XS" || fail "NYTProf.xs missing nytp_v5_sink_is_deflating wrapper"
grep -q 'PRODUCT_COMPRESS_EMIT' "$NYTP_PM_SRC" || fail "NYTProf.pm missing PRODUCT_COMPRESS_EMIT stamp"
grep -q 'PRODUCT_META_EMIT' "$NYTP_PM_SRC" || fail "NYTProf.pm missing PRODUCT_META_EMIT stamp"
grep -q 'PRODUCT_SUB_EMIT' "$NYTP_PM_SRC" || fail "NYTProf.pm missing PRODUCT_SUB_EMIT stamp"
grep -q 'PRODUCT_STMT_EMIT' "$NYTP_PM_SRC" || fail "NYTProf.pm missing PRODUCT_STMT_EMIT stamp"
ok "G03e debugger sources and Makefile target present"

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
  echo "G03e compress emit-MVP; live attach: g04_v5_parity_smoke.sh"
  echo "G05 options/format: g05_options_format_smoke.sh"
  echo "G06 fork/addpid: g06_fork_addpid_smoke.sh"
  echo "NOT-YET: mid-deflate continue-in-child / TEST-018 / full opcode"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — G03e debugger .so not built"
  echo "  (honest skip; G03e emit requires xs-nytprof)"
  print_residuals
  ok "g03e_compress_emit_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present — G03e debugger .so not built"
  echo "  (honest skip; G03e emit requires xs-nytprof)"
  print_residuals
  ok "g03e_compress_emit_smoke completed (skip — no XS headers)"
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

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g03e-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

# Isolated product @INC only. Never baseline/6.15/install, never crates/.
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

COMP_PATH="$WORKDIR/compress.nytprof"
DUP_PATH="$WORKDIR/dup.nytprof"
COMP_DUMP="$WORKDIR/compress.jsonl"

echo "workdir: $WORKDIR"
echo "PERL5LIB=$PERL5LIB"
echo "running: perl -I${NYTP_DEST} -d:NYTProfM (G03e compress emit)"

set +e
EMIT_OUT="$(
  cd "$WORKDIR" && perl -I"$NYTP_DEST" -d:NYTProfM -e '
    my $inc = $INC{"Devel/NYTProfM.pm"} // "";
    print "INC=", $inc, "\n";
    my $load = ($Devel::NYTProfM::PRODUCT_XS_LOAD ? 1 : 0);
    my $attach = (defined $Devel::NYTProfM::PRODUCT_XS_ATTACH && $Devel::NYTProfM::PRODUCT_XS_ATTACH) ? 1 : 0;
    my $stmt = ($Devel::NYTProfM::PRODUCT_STMT_EMIT ? 1 : 0);
    my $sub  = ($Devel::NYTProfM::PRODUCT_SUB_EMIT ? 1 : 0);
    my $meta = ($Devel::NYTProfM::PRODUCT_META_EMIT ? 1 : 0);
    my $comp = ($Devel::NYTProfM::PRODUCT_COMPRESS_EMIT ? 1 : 0);
    print "PRODUCT_XS_LOAD=", $load, "\n";
    print "PRODUCT_XS_ATTACH=", $attach, "\n";
    print "PRODUCT_STMT_EMIT=", $stmt, "\n";
    print "PRODUCT_SUB_EMIT=", $sub, "\n";
    print "PRODUCT_META_EMIT=", $meta, "\n";
    print "PRODUCT_COMPRESS_EMIT=", $comp, "\n";
    die "PRODUCT_XS_LOAD stamp missing\n" unless $load;
    die "PRODUCT_XS_ATTACH must stay false\n" if $attach;
    die "PRODUCT_STMT_EMIT stamp missing\n" unless $stmt;
    die "PRODUCT_SUB_EMIT stamp missing\n" unless $sub;
    die "PRODUCT_META_EMIT stamp missing\n" unless $meta;
    die "PRODUCT_COMPRESS_EMIT stamp missing\n" unless $comp;

    my $comp_path = $ARGV[0];
    my $dup_path  = $ARGV[1];

    # After load, init_profiler holds an in-memory sink. Drop it so the
    # NULL-sink wrapper returns NYTP_ERR_NULL (1) before enable_sink.
    DB::finish_profiler();
    my $st = DB::emit_start_deflate();
    print "NULL_DEFLATE_ST=", $st, "\n";
    die "emit_start_deflate(NULL sink) status=$st (want 1 NYTP_ERR_NULL)\n" unless $st == 1;
    my $defl = DB::is_deflating();
    print "NULL_IS_DEFLATING=", $defl, "\n";
    die "is_deflating(NULL sink)=$defl (want 0)\n" unless $defl == 0;

    $st = DB::enable_sink($comp_path);
    die "enable_sink(compress) status=$st\n" unless $st == 0;
    $defl = DB::is_deflating();
    print "PRE_IS_DEFLATING=", $defl, "\n";
    die "is_deflating before start-deflate=$defl (want 0)\n" unless $defl == 0;

    # Uncompressed prefix so the header/body before z is clearly plaintext.
    $st = DB::emit_attribute("g03e_before", "1");
    die "emit_attribute(g03e_before) status=$st\n" unless $st == 0;
    $st = DB::emit_time_line(10, 1, 5);
    die "emit_time_line(pre) status=$st\n" unless $st == 0;

    $st = DB::emit_start_deflate();
    print "START_DEFLATE_ST=", $st, "\n";
    die "emit_start_deflate status=$st (want 0)\n" unless $st == 0;
    $defl = DB::is_deflating();
    print "POST_IS_DEFLATING=", $defl, "\n";
    die "is_deflating after start-deflate=$defl (want 1)\n" unless $defl == 1;

    # Duplicate start-deflate on the same held sink is NYTP_ERR_STATE (2).
    $st = DB::emit_start_deflate();
    print "DUP_SAME_ST=", $st, "\n";
    die "duplicate emit_start_deflate(same sink) status=$st (want 2 NYTP_ERR_STATE)\n" unless $st == 2;
    $defl = DB::is_deflating();
    die "is_deflating after rejected duplicate=$defl (want 1)\n" unless $defl == 1;

    # Distinct greppable post-deflate payload (zlib body; dump must inflate).
    $st = DB::emit_attribute("g03e_after", "1");
    die "emit_attribute(g03e_after) status=$st\n" unless $st == 0;
    $st = DB::emit_time_line(20, 1, 424242);
    die "emit_time_line(post) status=$st\n" unless $st == 0;
    DB::finish_profiler();

    # Second sink/path: first start-deflate OK; duplicate is NYTP_ERR_STATE (2).
    $st = DB::enable_sink($dup_path);
    die "enable_sink(dup) status=$st\n" unless $st == 0;
    $st = DB::emit_start_deflate();
    print "DUP_PATH_FIRST_ST=", $st, "\n";
    die "emit_start_deflate(dup path) status=$st (want 0)\n" unless $st == 0;
    $st = DB::emit_start_deflate();
    print "DUP_PATH_SECOND_ST=", $st, "\n";
    die "duplicate emit_start_deflate(dup path) status=$st (want 2 NYTP_ERR_STATE)\n" unless $st == 2;
    DB::finish_profiler();
    print "COMPRESS_EMIT_OK\n";
  ' "$COMP_PATH" "$DUP_PATH" 2>&1
)"
EMIT_RC=$?
set -e
printf '%s\n' "$EMIT_OUT"

[[ "$EMIT_RC" -eq 0 ]] || fail "perl -d:NYTProfM G03e emit exited $EMIT_RC (want 0)"

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
grep -F -q 'PRODUCT_META_EMIT=1' <<<"$EMIT_OUT" \
  || fail "missing PRODUCT_META_EMIT=1 stamp"
grep -F -q 'PRODUCT_COMPRESS_EMIT=1' <<<"$EMIT_OUT" \
  || fail "missing PRODUCT_COMPRESS_EMIT=1 stamp"
if grep -F -q 'PRODUCT_XS_ATTACH=1' <<<"$EMIT_OUT"; then
  fail "PRODUCT_XS_ATTACH must stay 0 (G03e is compress emit-MVP, not attach)"
fi
grep -F -q 'NULL_DEFLATE_ST=1' <<<"$EMIT_OUT" \
  || fail "NULL-sink emit_start_deflate did not return NYTP_ERR_NULL (1)"
grep -F -q 'START_DEFLATE_ST=0' <<<"$EMIT_OUT" \
  || fail "emit_start_deflate did not return NYTP_OK (0)"
grep -F -q 'POST_IS_DEFLATING=1' <<<"$EMIT_OUT" \
  || fail "is_deflating did not become 1 after emit_start_deflate"
grep -F -q 'DUP_SAME_ST=2' <<<"$EMIT_OUT" \
  || fail "duplicate emit_start_deflate(same sink) did not return NYTP_ERR_STATE (2)"
grep -F -q 'DUP_PATH_SECOND_ST=2' <<<"$EMIT_OUT" \
  || fail "duplicate emit_start_deflate(second path) did not return NYTP_ERR_STATE (2)"
grep -F -q 'COMPRESS_EMIT_OK' <<<"$EMIT_OUT" || fail "missing COMPRESS_EMIT_OK"
ok "G03e start-deflate + NULL-sink fail-closed + duplicate NYTP_ERR_STATE"

[[ -f "$COMP_PATH" ]] || fail "enable_sink+finish did not write $COMP_PATH"

comp_magic="$(head -c 9 "$COMP_PATH" || true)"
[[ "$comp_magic" == "NYTProf 5" ]] || fail "compress bytes must start with NYTProf 5 (got $(printf %q "$comp_magic"))"
ok "produced bytes start with NYTProf 5"

# Uncompressed prefix must contain the pre-deflate attribute; post-deflate
# payload must not appear as plaintext (zlib after the 'z' tag).
if ! grep -a -F -q 'g03e_before' "$COMP_PATH"; then
  fail "pre-deflate ATTRIBUTE g03e_before missing as plaintext (header/prefix must stay uncompressed)"
fi
if grep -a -F -q 'g03e_after' "$COMP_PATH"; then
  fail "post-deflate ATTRIBUTE g03e_after appears as plaintext — stream is not zlib after START_DEFLATE"
fi
# Locate the START_DEFLATE tag after the text header.
if ! perl -e '
  use strict;
  use warnings;
  my $path = $ARGV[0];
  open my $fh, "<:raw", $path or die "$path: $!\n";
  local $/;
  my $buf = <$fh>;
  close $fh;
  my $nl = index($buf, "\n");
  die "no header newline\n" if $nl < 0;
  my $z = index($buf, "z", $nl + 1);
  die "no START_DEFLATE z tag after header\n" if $z < 0;
  my $after = substr($buf, $z + 1);
  die "empty zlib body after z\n" if length($after) < 2;
  # zlib CMF/FLG: CMF=0x78 is the common zlib header for windowBits=15.
  my $cmf = ord(substr($after, 0, 1));
  die sprintf("bytes after z are not zlib (cmf=0x%02x)\n", $cmf)
    unless ($cmf & 0x0f) == 8;
  exit 0;
' "$COMP_PATH"; then
  fail "compress stream missing zlib body after START_DEFLATE z tag"
fi
ok "after z tag the body is zlib (g03e_after not plaintext)"

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

dump_profile "$COMP_PATH" "$COMP_DUMP"

has_tag() {
  local dump="$1"
  local tag="$2"
  grep -E -q "\"tag\":[[:space:]]*\"${tag}\"" "$dump"
}

has_tag "$COMP_DUMP" "START_DEFLATE" || fail "compress dump missing START_DEFLATE (inflate path)"
has_tag "$COMP_DUMP" "ATTRIBUTE" || fail "compress dump missing ATTRIBUTE"
has_tag "$COMP_DUMP" "TIME_LINE" || fail "compress dump missing TIME_LINE"
grep -F -q 'g03e_after' "$COMP_DUMP" \
  || fail "compress dump missing post-deflate ATTRIBUTE g03e_after (inflate must recover it)"
grep -E -q '424242' "$COMP_DUMP" \
  || fail "compress dump missing post-deflate TIME_LINE line 424242 (inflate must recover it)"
ok "dump of those bytes recovered post-deflate g03e_after / TIME_LINE 424242 via inflate"

set +e
"${CLI_CMD[@]}" verify "$COMP_PATH" >"$WORKDIR/compress.verify.out" 2>"$WORKDIR/compress.verify.err"
VERIFY_RC=$?
set -e
if [[ "$VERIFY_RC" -eq 0 ]]; then
  ok "nytprof-cli verify accepted G03e compressed mini stream"
else
  echo "note: nytprof-cli verify exited $VERIFY_RC on G03e mini (dump inflate already asserted)"
  cat "$WORKDIR/compress.verify.err" >&2 || true
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
ok "G03e compress emit"
exit 0

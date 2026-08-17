#!/usr/bin/env bash
# PR-B5 / DI-09 — Advertised-options subset on live perl -d:NYTProfM.
#
# Drives the real debugger (product xs-nytprof dest):
#   - slowops omit / =2 does not croak
#   - slowops=0: no CORE:print / CORE:match SUB_RETURN
#   - slowops=1: fail-closed residual message
#   - unknown option fail-closed
#   - compress=1: live attach writes START_DEFLATE; shipped dump inflates
#
# Dual_path stays oracle-primary. collection_default stays v5. Not opcode.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

NYTP_DEST="$ROOT/collector/build/xs-nytprof"
NYTP_SO="$NYTP_DEST/auto/Devel/NYTProfM/NYTProfM.so"
NYTP_PM="$ROOT/collector/xs/Devel/NYTProfM.pm"
WORKLOAD="$ROOT/fixtures/v5/default-calls1/workload.pl"
SLOW1_MSG="slowops=1 (collapsed CORE:: package) is residual until full opcode attach"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "di09_options_subset_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary until S2"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$NYTP_PM" ]] || fail "missing $NYTP_PM"
[[ -f "$WORKLOAD" ]] || fail "missing $WORKLOAD"
grep -E -q 'PRODUCT_COMPRESS\s*=' "$NYTP_PM" \
  || fail "NYTProfM.pm missing PRODUCT_COMPRESS stamp (compress=1 live wire)"
grep -F -q 'emit_start_deflate' "$NYTP_PM" \
  || fail "NYTProfM.pm missing live emit_start_deflate for compress=1"

if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1; then
  echo "SKIP: no C compiler"
  ok "di09 layout (compile skipped)"
  exit 0
fi

echo "make -C collector xs-nytprof"
make -C "$ROOT/collector" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof missing .so"

CLI_CMD=()
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI_CMD=("${NYTPROF_NATIVE_CLI}")
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/prefix/bin/nytprof-cli")
elif [[ -x "$ROOT/target/debug/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-cli")
elif command -v cargo >/dev/null 2>&1; then
  CLI_CMD=(cargo run -q -p nytprof-cli --)
else
  fail "no shipped dump/verify CLI"
fi

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-di09-XXXXXX")"
trap 'rm -rf "$WORKDIR"' EXIT
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

run_attach() {
  local env="$1" out="$2"
  set +e
  NYTPROF="$env" perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" \
    >"$out.stdout" 2>"$out.stderr"
  echo $? >"$out.rc"
  set -e
}

# --- unknown fail-closed ---
run_attach "file=$WORKDIR/unk.out:not_a_real_opt=1" "$WORKDIR/unk"
[[ "$(cat "$WORKDIR/unk.rc")" != "0" ]] || fail "unknown option must fail-closed"
grep -E -q 'unknown NYTPROF option' "$WORKDIR/unk.stderr" \
  || fail "unknown option missing fail-closed text"
[[ ! -f "$WORKDIR/unk.out" ]] || fail "unknown option must not write a profile"
ok "unknown option fail-closed"

# --- slowops=1 fail-closed ---
run_attach "file=$WORKDIR/s1.out:slowops=1" "$WORKDIR/s1"
[[ "$(cat "$WORKDIR/s1.rc")" != "0" ]] || fail "slowops=1 must fail-closed"
grep -F -q "$SLOW1_MSG" "$WORKDIR/s1.stderr" \
  || fail "slowops=1 missing residual fail-closed message"
[[ ! -f "$WORKDIR/s1.out" ]] || fail "slowops=1 must not write a profile"
ok "slowops=1 fail-closed residual"

# --- slowops omit / =2 do not croak ---
run_attach "file=$WORKDIR/s2.out:slowops=2" "$WORKDIR/s2"
[[ "$(cat "$WORKDIR/s2.rc")" == "0" ]] || fail "slowops=2 must not croak: $(cat "$WORKDIR/s2.stderr")"
run_attach "file=$WORKDIR/som.out" "$WORKDIR/som"
[[ "$(cat "$WORKDIR/som.rc")" == "0" ]] || fail "slowops omit must not croak: $(cat "$WORKDIR/som.stderr")"
ok "slowops omit/=2 do not croak"

# --- slowops=0: no CORE: names ---
run_attach "file=$WORKDIR/s0.out:calls=2:slowops=0" "$WORKDIR/s0"
[[ "$(cat "$WORKDIR/s0.rc")" == "0" ]] || fail "slowops=0 attach failed: $(cat "$WORKDIR/s0.stderr")"
set +e
"${CLI_CMD[@]}" dump "$WORKDIR/s0.out" >"$WORKDIR/s0.jsonl" 2>"$WORKDIR/s0.dump.err"
set -e
if grep -E -q 'CORE:(print|match)' "$WORKDIR/s0.jsonl"; then
  fail "slowops=0 must not emit CORE:print / CORE:match"
fi
ok "slowops=0: no CORE: events"

# --- compress=1 live START_DEFLATE ---
run_attach "file=$WORKDIR/c1.out:compress=1" "$WORKDIR/c1"
[[ "$(cat "$WORKDIR/c1.rc")" == "0" ]] || fail "compress=1 attach failed: $(cat "$WORKDIR/c1.stderr")"
[[ -f "$WORKDIR/c1.out" ]] || fail "compress=1 did not write a profile"
magic="$(head -c 9 "$WORKDIR/c1.out" || true)"
[[ "$magic" == "NYTProf 5" ]] || fail "compress=1 magic not NYTProf 5"
perl -e '
  my $p = $ARGV[0];
  open my $fh, "<:raw", $p or die $!;
  my $hdr = <$fh>;
  die "bad header\n" unless $hdr && $hdr =~ /^NYTProf 5/;
  my $rest;
  { local $/; $rest = <$fh> // "" }
  my $z = index($rest, "z");
  die "no START_DEFLATE z tag after header\n" if $z < 0;
  my $body = substr($rest, $z + 1);
  die "no zlib body after z\n" if length($body) < 2;
  my $b0 = ord(substr($body, 0, 1));
  die sprintf("body after z is not zlib (0x%02x)\n", $b0)
    unless $b0 == 0x78;
  print "ZLIB_AFTER_Z=1\n";
' "$WORKDIR/c1.out"
set +e
"${CLI_CMD[@]}" dump "$WORKDIR/c1.out" >"$WORKDIR/c1.jsonl" 2>"$WORKDIR/c1.dump.err"
DUMP_RC=$?
"${CLI_CMD[@]}" verify "$WORKDIR/c1.out" >"$WORKDIR/c1.verify" 2>"$WORKDIR/c1.verify.err"
VER_RC=$?
set -e
[[ "$DUMP_RC" -eq 0 ]] || fail "compress=1 dump failed (inflate): $(cat "$WORKDIR/c1.dump.err")"
[[ "$VER_RC" -eq 0 ]] || fail "compress=1 verify failed: $(cat "$WORKDIR/c1.verify.err")"
grep -E -q '"tag":[[:space:]]*"START_DEFLATE"' "$WORKDIR/c1.jsonl" \
  || fail "compress=1 dump missing START_DEFLATE"
grep -E -q '"tag":[[:space:]]*"TIME_LINE"|"tag":[[:space:]]*"SUB_RETURN"' "$WORKDIR/c1.jsonl" \
  || fail "compress=1 dump missing post-deflate events (inflate path)"
ok "compress=1 live attach: START_DEFLATE + dump/verify inflate"

echo "NOT-YET: findcaller/evals/full slowops.h / S2"
echo "E3: leave=1 opt-in (default stays 0); see g19_leave_discount_smoke.sh"
ok "DI-09 advertised-options subset"

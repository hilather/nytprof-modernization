#!/usr/bin/env bash
# D2 — durable sealed publish: force-seal then kill -9 leaves a decoder-ready
# nytprof.out. Drives real perl -d:NYTProfM + shipped dump/verify.
#
#   durable=1 + DB::durable_seal_now + kill -9 → dump TIME_LINE>=1
#   compress=1:durable=1 → dump inflates (not torn-z salvage discard)
#   durable=0:compress=1 + kill during sleep → verify is not OK:
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

NYTP_DEST="$ROOT/collector/build/xs-nytprof"
NYTP_SO="$NYTP_DEST/auto/Devel/NYTProfM/NYTProfM.so"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1; then
  echo "SKIP: no C compiler"
  ok "di_durable_kill_smoke (compile skipped)"
  exit 0
fi

make -C "$ROOT/collector" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof missing .so"

CLI_CMD=()
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI_CMD=("${NYTPROF_NATIVE_CLI}")
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/prefix/bin/nytprof-cli")
elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-dump")
elif [[ -x "$ROOT/target/debug/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-cli")
elif command -v cargo >/dev/null 2>&1; then
  CLI_CMD=(cargo run -q -p nytprof-cli --)
else
  fail "no shipped dump/verify CLI"
fi

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-durable-XXXXXX")"
trap 'rm -rf "$WORKDIR"' EXIT
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

write_child() {
  local dest="$1"
  cat >"$dest" <<'PL'
use strict;
use warnings;
my $ready = $ARGV[0] or die "usage: child READY\n";
for my $i (1 .. 200) {
    my $x = $i * 3 + 1;
    $x = $x + $i;
}
die "DB::durable_seal_now missing\n" unless defined &DB::durable_seal_now;
my $st = DB::durable_seal_now();
die "durable_seal_now status=$st\n" unless $st == 0;
open my $fh, '>', $ready or die "ready: $!";
print {$fh} "ready\n";
close $fh;
sleep 30;
PL
}

run_kill_after_seal() {
  local nytprof="$1" profile="$2" label="$3"
  local child="$WORKDIR/${label}-child.pl"
  local ready="$WORKDIR/${label}.ready"
  write_child "$child"
  rm -f "$ready" "$profile"
  set +e
  NYTPROF="$nytprof" perl -I"$NYTP_DEST" -d:NYTProfM "$child" "$ready" \
    >"$WORKDIR/${label}.stdout" 2>"$WORKDIR/${label}.stderr" &
  local pid=$!
  set -e
  local i
  for i in $(seq 1 100); do
    if [[ -s "$ready" ]]; then
      break
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      fail "$label child exited before ready: $(cat "$WORKDIR/${label}.stderr" 2>/dev/null || true)"
    fi
    sleep 0.05
  done
  [[ -s "$ready" ]] || fail "$label never wrote ready file"
  kill -9 "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  [[ -s "$profile" ]] || fail "$label missing nytprof.out after kill"
  set +e
  "${CLI_CMD[@]}" dump "$profile" >"$WORKDIR/${label}.jsonl" 2>"$WORKDIR/${label}.dump.err"
  local dump_rc=$?
  "${CLI_CMD[@]}" verify "$profile" >"$WORKDIR/${label}.verify" 2>"$WORKDIR/${label}.verify.err"
  local ver_rc=$?
  set -e
  [[ "$dump_rc" -eq 0 ]] || fail "$label dump failed: $(cat "$WORKDIR/${label}.dump.err")"
  grep -E -q '"tag":[[:space:]]*"TIME_LINE"' "$WORKDIR/${label}.jsonl" \
    || fail "$label dump missing TIME_LINE (got $(head -5 "$WORKDIR/${label}.jsonl"))"
  if [[ "$nytprof" == *compress=1* ]] || [[ "$nytprof" != *compress=0* ]]; then
    grep -E -q '"tag":[[:space:]]*"START_DEFLATE"' "$WORKDIR/${label}.jsonl" \
      || fail "$label compressed dump missing START_DEFLATE"
    # KD-D8: OPTION/PID_START stay uncompressed; z is inserted at header_end.
    perl -e '
      use strict;
      use warnings;
      my $p = $ARGV[0];
      open my $fh, "<:raw", $p or die "$p: $!";
      my $b = do { local $/; <$fh> };
      die "no magic\n" unless $b =~ /^NYTProf 5 0\n/;
      my $z = index($b, "z");
      die "no z tag on disk\n" if $z < 0;
      my $pre = substr($b, 0, $z);
      die "PID_START not before z (header_end too early)\n" unless $pre =~ /P/;
      die "OPTION not before z\n" unless $pre =~ /!/;
    ' "$profile" || fail "$label KD-D8 uncompressed header before z"
  fi
  # Sealed mid-run has no PID_END → verify must not claim complete OK.
  if grep -q '^OK:' "$WORKDIR/${label}.verify"; then
    fail "$label verify printed OK: on a mid-run seal (want incomplete): $(cat "$WORKDIR/${label}.verify")"
  fi
  ok "$label: dump TIME_LINE after kill -9 (verify not OK: rc=$ver_rc)"
}

# durable=1 + compress=0 (no z)
run_kill_after_seal "file=$WORKDIR/d0.out:compress=0:durable=1" \
  "$WORKDIR/d0.out" "dur1-uncomp"

# durable=1 + compress=1 (sealed z + Z_FINISH)
run_kill_after_seal "file=$WORKDIR/d1.out:compress=1:durable=1" \
  "$WORKDIR/d1.out" "dur1-zlib"

# durable=0 + compress=1: kill during sleep without force-seal → torn or
# unfinished zlib must not verify OK.
run_torn_live_deflate() {
  local profile="$WORKDIR/livez.out"
  local child="$WORKDIR/livez-child.pl"
  local pid
  cat >"$child" <<'PL'
use strict;
sleep 30;
PL
  set +e
  NYTPROF="file=$profile:compress=1:durable=0" \
    perl -I"$NYTP_DEST" -d:NYTProfM "$child" \
    >"$WORKDIR/livez.stdout" 2>"$WORKDIR/livez.stderr" &
  pid=$!
  set -e
  sleep 0.2
  kill -9 "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  if [[ -s "$profile" ]]; then
    set +e
    "${CLI_CMD[@]}" verify "$profile" >"$WORKDIR/livez.verify" 2>"$WORKDIR/livez.verify.err"
    set -e
    if grep -q '^OK:' "$WORKDIR/livez.verify"; then
      fail "durable=0:compress=1 kill printed verify OK: (torn zlib must not be complete)"
    fi
    ok "durable=0:compress=1 kill: verify not OK:"
  else
    ok "durable=0:compress=1 kill: no file (also not verify OK:)"
  fi
}
run_torn_live_deflate

ok "di_durable_kill_smoke"

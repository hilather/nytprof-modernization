#!/usr/bin/env bash
# Pure-Perl JsonlData PID_START / PID_END smoke (PERL-PID-JSONL).
#
# Spec: docs/schemas/perl-jsonl-data-mvp-v0.md
#
# 1. Prove golden JSONL path: pid_start_count >= 1, pid_end_count >= 1,
#    start pid matches end pid (default-calls1 golden observes 2975381)
# 2. Optional: regenerate dump via native CLI and re-query PID counts
#
# Does NOT put crates/ on oracle PERL5LIB. Does NOT require oracle Devel::NYTProf.
# Does NOT invent PIDs — asserts dump-derived values only.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_pid_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="fixtures/v5/default-calls1"
GOLDEN="$FIXTURE_DIR/readstream.jsonl"
PROFILE="$FIXTURE_DIR/nytprof.out"
PM="perl/lib/Devel/NYTProf/JsonlData.pm"
STREAM_PM="perl/lib/Devel/NYTProf/JsonlReadStream.pm"
T="perl/t/jsonl_data_pid_default_calls1.t"
LIB="perl/lib"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$GOLDEN" ]] || fail "missing golden dump $GOLDEN"
[[ -f "$ROOT/$PM" ]] || fail "missing $PM"
[[ -f "$ROOT/$STREAM_PM" ]] || fail "missing $STREAM_PM"
[[ -f "$ROOT/$T" ]] || fail "missing $T"

# ---------------------------------------------------------------------------
# 1. Pure-Perl test against committed golden JSONL (no native CLI required)
# ---------------------------------------------------------------------------
echo "=== JsonlData PID: golden JSONL path ==="
prove -I"$LIB" "$T" || fail "prove $T failed"
ok "prove $T (golden readstream.jsonl)"

# Explicit aggregation print for operator evidence
AGG_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
    my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
    my $sc = $d->pid_start_count;
    my $ec = $d->pid_end_count;
    my $starts = $d->pid_starts;
    my $ends   = $d->pid_ends;
    my $sp = @$starts ? $starts->[0]{pid} : "";
    my $ep = @$ends   ? $ends->[0]{pid}   : "";
    printf "pid_start_count=%d\n", $sc;
    printf "pid_end_count=%d\n",   $ec;
    printf "start_pid=%s\n", $sp;
    printf "end_pid=%s\n",   $ep;
    printf "pids=%s\n", join(",", @{ $d->pids });
    printf "records_seen=%d\n", $d->records_seen;
  ' "$GOLDEN"
)"
echo "$AGG_OUT"
echo "$AGG_OUT" | grep -qE '^pid_start_count=[1-9]' \
  || fail "golden pid_start_count < 1"
echo "$AGG_OUT" | grep -qE '^pid_end_count=[1-9]' \
  || fail "golden pid_end_count < 1"
# start and end pid must match and be non-empty (dump-derived)
START_PID="$(echo "$AGG_OUT" | sed -n 's/^start_pid=//p' | head -1)"
END_PID="$(echo "$AGG_OUT" | sed -n 's/^end_pid=//p' | head -1)"
[[ -n "$START_PID" ]] || fail "golden start_pid empty"
[[ -n "$END_PID" ]] || fail "golden end_pid empty"
[[ "$START_PID" == "$END_PID" ]] || fail "start_pid ($START_PID) != end_pid ($END_PID)"
# Golden fixture observes 2975381 (do not invent; assert dump value)
[[ "$START_PID" == "2975381" ]] || fail "expected golden pid 2975381, got $START_PID"
ok "golden JsonlData: start_count>=1 end_count>=1 pid=$START_PID matched"

# ---------------------------------------------------------------------------
# 2. Native CLI dump path (when cargo or a built binary is available)
# ---------------------------------------------------------------------------
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT
NATIVE_JSONL="$TMPDIR_SMOKE/native_dump.jsonl"

find_cli() {
  if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
    echo "path:${NYTPROF_NATIVE_CLI}"
    return 0
  fi
  for p in \
    prefix/bin/nytprof-cli \
    prefix/bin/nytprof-dump \
    target/release/nytprof-dump \
    target/debug/nytprof-dump
  do
    if [[ -x "$ROOT/$p" || -f "$ROOT/$p" ]]; then
      echo "path:$ROOT/$p"
      return 0
    fi
  done
  if command -v cargo >/dev/null 2>&1; then
    echo "cargo"
    return 0
  fi
  return 1
}

if CLI_SPEC="$(find_cli)"; then
  echo "=== JsonlData PID: native CLI dump path ($CLI_SPEC) ==="
  [[ -f "$ROOT/$PROFILE" ]] || fail "missing profile $PROFILE"

  set +e
  if [[ "$CLI_SPEC" == cargo ]]; then
    cargo run -q -p nytprof-cli -- dump "$PROFILE" >"$NATIVE_JSONL" 2>"$TMPDIR_SMOKE/dump.err"
    DUMP_RC=$?
  else
    CLI_PATH="${CLI_SPEC#path:}"
    "$CLI_PATH" dump "$PROFILE" >"$NATIVE_JSONL" 2>"$TMPDIR_SMOKE/dump.err"
    DUMP_RC=$?
  fi
  set -e

  if [[ "$DUMP_RC" -ne 0 ]]; then
    cat "$TMPDIR_SMOKE/dump.err" >&2 || true
    fail "native dump failed (rc=$DUMP_RC)"
  fi
  [[ -s "$NATIVE_JSONL" ]] || fail "native dump produced empty JSONL"

  NATIVE_AGG="$(
    perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
      my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
      my $sc = $d->pid_start_count;
      my $ec = $d->pid_end_count;
      my $starts = $d->pid_starts;
      my $ends   = $d->pid_ends;
      my $sp = @$starts ? $starts->[0]{pid} : "";
      my $ep = @$ends   ? $ends->[0]{pid}   : "";
      printf "pid_start_count=%d\n", $sc;
      printf "pid_end_count=%d\n",   $ec;
      printf "start_pid=%s\n", $sp;
      printf "end_pid=%s\n",   $ep;
      printf "pids=%s\n", join(",", @{ $d->pids });
    ' "$NATIVE_JSONL"
  )"
  echo "$NATIVE_AGG"
  echo "$NATIVE_AGG" | grep -qE '^pid_start_count=[1-9]' \
    || fail "native dump pid_start_count < 1"
  echo "$NATIVE_AGG" | grep -qE '^pid_end_count=[1-9]' \
    || fail "native dump pid_end_count < 1"
  N_START="$(echo "$NATIVE_AGG" | sed -n 's/^start_pid=//p' | head -1)"
  N_END="$(echo "$NATIVE_AGG" | sed -n 's/^end_pid=//p' | head -1)"
  [[ -n "$N_START" && "$N_START" == "$N_END" ]] \
    || fail "native start_pid ($N_START) != end_pid ($N_END)"
  # Native dump of the same profile must observe the same process id
  [[ "$N_START" == "2975381" ]] || fail "native dump expected pid 2975381, got $N_START"
  ok "native dump JsonlData: start_count>=1 end_count>=1 pid=$N_START matched"
else
  echo "NOTE: no native CLI / cargo; skipped live dump path (golden path still required)"
fi

ok "perl JsonlData PID packaging smoke passed"
exit 0

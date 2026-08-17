#!/usr/bin/env bash
# PR-10 — Do not wrap CORE::require. DateTime::Duration /
# namespace::autoclean / B::Hooks::EndOfScope::XS need compile-time
# %^H magic (Variable::Magic on the hints hash).
#
# Symptom (before this fix):
#   Can't use string ("#pod\n") as an ARRAY ref while "strict refs" in use
#   at .../B/Hooks/EndOfScope/XS.pm line 39.
#   BEGIN failed--compilation aborted at .../DateTime/Duration.pm line 5.
#
# Cause: (1) CORE::GLOBAL::require wrapping CORE::require, and/or
# (2) DB::sub intercepting B::Hooks::EndOfScope::XS::on_scope_end.
# Either one makes Variable::Magic::getdata(%^H) return a source
# fragment ("#pod\n") instead of the wizard array. Fix: no require
# wrap; preload hint-magic modules and CvNODEBUG their CVs before
# $^P 0x01. Do not defer 0x01 to INIT (g04 SUB_RETURN → 0).
#
# Drives real `perl -d:NYTProfM` (product tree, not baseline/6.15/install)
# with NYTPROF file=<path>. Isolated product @INC. Never crates/.
# collection_default stays v5. Not wired into dual_path or offline_gate.
#
# Always: source assert (no CORE::GLOBAL::require wrap) + live attach
# that `defined &CORE::GLOBAL::require` is false.
# When DateTime loads: also construct DateTime + DateTime::Duration.
#
# Extra DateTime @INC (optional): NYTPROF_SMOKE_EXTRA_INC=dir[:dir]
#
# Exit 0: pass, or honest skip (no CC / no XS headers).
# Exit 1: compile / attach / DateTime / wrap failure.
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

usage() {
  cat <<'EOF'
Usage: g10_datetime_hints_smoke.sh

PR-10: no CORE::GLOBAL::require wrap; DateTime::Duration compiles
under live perl -d:NYTProfM when DateTime is installed.
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

echo "g10_datetime_hints_smoke: repo root $ROOT"
echo "collection_default remains v5; never crates/ on PERL5LIB"
echo "PR-10: no CORE::GLOBAL::require wrap (DateTime / namespace::autoclean %^H)"

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

if grep -E '^[[:space:]]*\*CORE::GLOBAL::require[[:space:]]*=' "$NYTP_PM_SRC"; then
  fail "NYTProfM.pm must not assign CORE::GLOBAL::require (breaks %^H / DateTime::Duration)"
fi
if grep -q '_product_install_require_rebind' "$NYTP_PM_SRC"; then
  fail "NYTProfM.pm must not define _product_install_require_rebind"
fi
grep -q '_product_nodebug_hint_magic' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing _product_nodebug_hint_magic"
grep -q 'nodebug_stash' "$NYTP_XS" \
  || fail "NYTProf.xs missing nodebug_stash"
ok "PR-10 debugger sources: no CORE::GLOBAL::require wrap; hint-magic CvNODEBUG"

print_residuals() {
  echo "G07 Getopt/Exporter compile-safe: g07_getopt_compile_smoke.sh"
  echo "G08 slowops times: g08_slowops_times_smoke.sh"
  echo "NOT-YET: full 6.15 opcode/entersub / XSUB / leavesub"
}

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

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — G10 debugger .so not built"
  echo "  (honest skip; live attach requires xs-nytprof)"
  print_residuals
  ok "g10_datetime_hints_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present — G10 debugger .so not built"
  echo "  (honest skip; live attach requires xs-nytprof)"
  print_residuals
  ok "g10_datetime_hints_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "perl + EXTERN.h present"

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
[[ -f "$NYTP_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM.pm"
ok "xs-nytprof produced .so + .pm under collector/build/xs-nytprof"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g10-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"
EXTRA_INC="${NYTPROF_SMOKE_EXTRA_INC:-}"
if [[ -n "$EXTRA_INC" ]]; then
  case ":${EXTRA_INC}:" in
    *"/crates/"*) fail2 "NYTPROF_SMOKE_EXTRA_INC must not contain /crates/" ;;
  esac
  export PERL5LIB="${EXTRA_INC}:${PERL5LIB}"
fi

PROFILE="$WORKDIR/nytprof.out"
WORKLOAD="$WORKDIR/no_require_wrap.pl"

cat >"$WORKLOAD" <<'PL'
use strict;
use warnings;
die "CORE::GLOBAL::require must not be installed (breaks %^H / DateTime)\n"
  if defined &CORE::GLOBAL::require;
print "ok-nowrap\n";
PL

echo "workdir: $WORKDIR"
echo "PERL5LIB=$PERL5LIB"
echo "running: NYTPROF=file=${PROFILE} perl -d:NYTProfM <no CORE::GLOBAL::require>"

set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="$(nytprof_attach "file=${PROFILE}")" perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"

if grep -F -q 'as an ARRAY ref' <<<"$RUN_OUT"; then
  fail "stdout/stderr contain ARRAY ref hints error (CORE::require wrap still active)"
fi
[[ "$RUN_RC" -eq 0 ]] || fail "perl -d:NYTProfM no-require-wrap exited $RUN_RC (want 0)"
grep -F -q 'ok-nowrap' <<<"$RUN_OUT" || fail "workload did not print ok-nowrap"
ok "live perl -d:NYTProfM did not install CORE::GLOBAL::require"

[[ -f "$PROFILE" ]] || fail "NYTPROF file= did not write $PROFILE"
magic="$(head -c 9 "$PROFILE" || true)"
[[ "$magic" == "NYTProf 5" ]] || fail "produced bytes must start with NYTProf 5 (got $(printf %q "$magic"))"
ok "produced bytes start with NYTProf 5"

HAVE_DT=0
if perl -e 'require DateTime; require DateTime::Duration; 1' >/dev/null 2>&1; then
  HAVE_DT=1
fi
echo "host DateTime loadable: $HAVE_DT"

if [[ "$HAVE_DT" -eq 1 ]]; then
  DT_PROFILE="$WORKDIR/nytprof-dt.out"
  DT_WORKLOAD="$WORKDIR/datetime_duration.pl"
  cat >"$DT_WORKLOAD" <<'PL'
use strict;
use warnings;
use DateTime;
use DateTime::Duration;
die "CORE::GLOBAL::require must not be installed\n"
  if defined &CORE::GLOBAL::require;
my $dt = DateTime->new(
  year => 2020, month => 1, day => 1, time_zone => 'UTC',
);
my $dur = DateTime::Duration->new( days => 1, hours => 2 );
$dt->add_duration($dur);
my $clone = $dur->clone;
my $sum   = $dur + DateTime::Duration->new( days => 1 );
die "ymd"   unless $dt->ymd eq '2020-01-02';
die "days"  unless $clone->delta_days == 1;
die "sum"   unless $sum->delta_days == 2;
print "ok-datetime\n";
PL

  echo "running: NYTPROF=file=${DT_PROFILE} perl -d:NYTProfM <DateTime::Duration>"
  set +e
  DT_OUT="$(
    cd "$WORKDIR" && NYTPROF="$(nytprof_attach "file=${DT_PROFILE}")" perl -I"$NYTP_DEST" -d:NYTProfM "$DT_WORKLOAD" 2>&1
  )"
  DT_RC=$?
  set -e
  printf '%s\n' "$DT_OUT"
  if grep -F -q 'EndOfScope/XS.pm' <<<"$DT_OUT"; then
    fail "DateTime::Duration still dies in B::Hooks::EndOfScope::XS"
  fi
  if grep -F -q 'as an ARRAY ref' <<<"$DT_OUT"; then
    fail "DateTime attach still hits %^H ARRAY-ref error"
  fi
  [[ "$DT_RC" -eq 0 ]] || fail "perl -d:NYTProfM DateTime exited $DT_RC (want 0)"
  grep -F -q 'ok-datetime' <<<"$DT_OUT" || fail "DateTime workload did not print ok-datetime"
  [[ -f "$DT_PROFILE" ]] || fail "DateTime attach did not write $DT_PROFILE"
  ok "live perl -d:NYTProfM compiled DateTime::Duration and did date math"
else
  echo "SKIP: DateTime not installed — live Duration half not run"
  echo "  (source + CORE::GLOBAL::require live assert still bind)"
fi

print_residuals
ok "G10 DateTime / %^H hints-safe attach"
exit 0

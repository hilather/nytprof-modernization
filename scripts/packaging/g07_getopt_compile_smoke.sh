#!/usr/bin/env bash
# PR-7 — Compile-safe start + goto &$raw so Getopt::Long / Exporter /
# Time::HiRes compile under live `perl -d:NYTProfM`.
#
# Drives real `perl -d:NYTProfM` (product tree, not baseline/6.15/install)
# with NYTPROF file=<path> on a tiny script that `use`s Getopt::Long
# (and Time::HiRes when the module loads on this host). Must exit 0,
# print ok, write NYTProf 5, and not emit `heavy_(eval)` or
# `Global symbol "$VERSION"`.
#
# Why this exists: `$DB::single = 1` at file= enable ran DB::DB during
# `use Getopt::Long` compile (`use vars qw($VERSION)` + strict). Product
# `DB::sub` wrapping Exporter with `&$raw` (not `goto`) turned
# `goto &heavy_*` into `heavy_(eval)`. `vars`/`constant`/`overload`
# `import` use `caller` — wrap would install into DB. PR-7 defers
# `$DB::single` to INIT, goto-all until then, and `goto &$raw` for
# Exporter / Getopt / vars / constant / overload. Normal workload
# subs still use the hash-stack `&$raw` wrap (g04 15/3/15 + leaf incl>0).
#
# Host vs EL8: this host (Perl 5.38 / Getopt::Long 2.54) reproduced the
# same `$VERSION` strict abort as EL8 Perl 5.26 before the fix. Land the
# smoke even on hosts that do not reproduce — it would have caught it.
#
# Does NOT rewrite dual_path. collection_default stays v5. Not wired
# into dual_path or offline_gate.
#
# When CC + Perl XS headers exist:
#   make -C collector xs-nytprof, isolated product @INC, live attach.
# When missing: honest SKIP: after source-file asserts (exit 0).
#
# Exit 0: G07 compile-safe pass, or honest skip (no CC / no XS headers).
# Exit 1: compile / attach / magic failure.
# Exit 2: wrapper misuse or crates/ on PERL5LIB.
#
# Never puts crates/ on PERL5LIB.
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
Usage: g07_getopt_compile_smoke.sh

PR-7 compile-safe attach: real perl -d:NYTProfM use Getopt::Long
(+ Time::HiRes when present), NYTProf 5, no heavy_(eval) / $VERSION abort.
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

echo "g07_getopt_compile_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary until S2"
echo "never crates/ on PERL5LIB"
echo "PR-7: INIT \$DB::single + compile-time goto-all + Exporter/Getopt/vars/constant; not full opcode"

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
grep -q 'libnytp_sink_v5.a' "$MAKEFILE" || fail "Makefile missing libnytp_sink_v5.a"
grep -q 'sub sub' "$NYTP_PM_SRC" || fail "NYTProf.pm missing DB::sub hook"
grep -q '_product_needs_goto' "$NYTP_PM_SRC" || fail "NYTProf.pm missing _product_needs_goto"
grep -q 'Exporter' "$NYTP_PM_SRC" || fail "NYTProf.pm missing Exporter goto class"
grep -E -q '^INIT \{' "$NYTP_PM_SRC" || fail "NYTProf.pm missing INIT { for compile-safe \$DB::single"
if grep -n '\$DB::single[[:space:]]*=' "$NYTP_PM_SRC" | grep -v 'INIT' | grep -q .; then
  # Allow comments; reject an enable-time assignment outside INIT.
  if awk '
    BEGIN { in_init=0; bad=0 }
    /^INIT \{/ { in_init=1 }
    /^\}/ && in_init { in_init=0 }
    /\$DB::single[[:space:]]*=/ && $0 !~ /^[[:space:]]*#/ && !in_init { bad=1 }
    END { exit bad }
  ' "$NYTP_PM_SRC"; then
    :
  else
    fail "NYTProf.pm must not assign \$DB::single outside INIT (compile-safe start)"
  fi
fi
ok "PR-7 debugger sources: INIT \$DB::single + _product_needs_goto present"

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
  echo "G04 attach 15/3/15: g04_v5_parity_smoke.sh"
  echo "NOT-YET: Rocky demo still profiles core-only scanner (ack retry later)"
  echo "NOT-YET: full 6.15 opcode/entersub / XSUB / leavesub"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — G07 debugger .so not built"
  echo "  (honest skip; live attach requires xs-nytprof)"
  print_residuals
  ok "g07_getopt_compile_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present — G07 debugger .so not built"
  echo "  (honest skip; live attach requires xs-nytprof)"
  print_residuals
  ok "g07_getopt_compile_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "perl + EXTERN.h present"

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
[[ -f "$NYTP_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM.pm"
[[ -f "$NYTP_DEST/Devel/NYTProfM/Core.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM/Core.pm"
ok "xs-nytprof produced .so + .pm under collector/build/xs-nytprof"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g07-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

# Isolated product @INC only. Never baseline/6.15/install, never crates/.
unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

PROFILE="$WORKDIR/nytprof.out"
WORKLOAD="$WORKDIR/getopt_compile.pl"

# Time::HiRes: include the use only when the host can load it without -d.
HAVE_HIRES=0
if perl -e 'require Time::HiRes; print $Time::HiRes::VERSION // "ok"' \
    >/dev/null 2>&1; then
  HAVE_HIRES=1
fi
echo "host Time::HiRes loadable without -d: $HAVE_HIRES"

{
  cat <<'PL'
use strict;
use warnings;
use Getopt::Long;
PL
  if [[ "$HAVE_HIRES" -eq 1 ]]; then
    cat <<'PL'
use Time::HiRes;
PL
  fi
  cat <<'PL'
my $x;
Getopt::Long::GetOptionsFromArray(["--x", "1"], "x=i" => \$x);
die "PRODUCT_XS_ATTACH must be 1 when file= is set\n"
  unless $Devel::NYTProfM::PRODUCT_XS_ATTACH;
die "need C TIME_LINE (PRODUCT_DBSTATE_LINE) or \$DB::single after INIT\n"
  unless $Devel::NYTProfM::PRODUCT_DBSTATE_LINE || $DB::single;
die "DB::single must be 0 when C OP_DBSTATE TIME_LINE is installed\n"
  if $Devel::NYTProfM::PRODUCT_DBSTATE_LINE && $DB::single;
print "ok\n";
PL
} >"$WORKLOAD"

echo "workdir: $WORKDIR"
echo "PERL5LIB=$PERL5LIB"
echo "workload: $WORKLOAD"
echo "running: NYTPROF=file=${PROFILE} perl -I${NYTP_DEST} -d:NYTProfM <getopt compile>"

set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}" perl -I"$NYTP_DEST" -d:NYTProfM "$WORKLOAD" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"

if grep -F -q 'heavy_(eval)' <<<"$RUN_OUT"; then
  fail "stdout/stderr contain heavy_(eval) (Exporter goto was wrapped with &\$raw)"
fi
if grep -F -q 'Global symbol "$VERSION"' <<<"$RUN_OUT"; then
  fail "stdout/stderr contain Global symbol \"\$VERSION\" (compile-safe start failed)"
fi
[[ "$RUN_RC" -eq 0 ]] || fail "perl -d:NYTProfM Getopt compile exited $RUN_RC (want 0)"
grep -F -q 'ok' <<<"$RUN_OUT" || fail "workload did not print ok"
ok "live perl -d:NYTProfM compiled Getopt::Long and printed ok"

[[ -f "$PROFILE" ]] || fail "NYTPROF file= did not write $PROFILE"
magic="$(head -c 9 "$PROFILE" || true)"
[[ "$magic" == "NYTProf 5" ]] || fail "produced bytes must start with NYTProf 5 (got $(printf %q "$magic"))"
ok "produced bytes start with NYTProf 5"

print_residuals
ok "G07 Getopt/Exporter compile-safe attach"
exit 0

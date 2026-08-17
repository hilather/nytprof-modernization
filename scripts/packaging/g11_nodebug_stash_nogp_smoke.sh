#!/usr/bin/env bash
# PR-11 — DB::nodebug_stash / rebind_stash_slowops must not SEGV on a
# GP-less stash GV. GvCV() is ((CV*)GvGP(gv)->gp_cv); isGV() is not
# enough. v0.2.12 walked Variable::Magic / BHES / Package::Stash::XS
# with isGV + GvCV and core-dumped when a slot had no GP.
#
# Drives the shipped XSUBs (DB::nodebug_stash, DB::rebind_stash_slowops)
# and live `perl -d:NYTProfM` with NYTPROF file=. Isolated product @INC.
# Never crates/. collection_default stays v5. Not in dual_path / offline_gate.
#
# Plants a GP-less SVt_PVGV via a tiny helper .so (the only way to create
# that slot from Perl). Pre-fix: exit 139. Post-fix: skip the slot, exit 0.
#
# Exit 0: pass, or honest skip (no CC / no XS headers).
# Exit 1: compile / SEGV / attach failure.
# Exit 2: wrapper misuse or crates/ on PERL5LIB.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

COLLECTOR="$ROOT/collector"
MAKEFILE="$COLLECTOR/Makefile"
NYTP_DEST="$COLLECTOR/build/xs-nytprof"
NYTP_SO="$NYTP_DEST/auto/Devel/NYTProfM/NYTProfM.so"
NYTP_PM_SRC="$COLLECTOR/xs/Devel/NYTProfM.pm"
NYTP_XS="$COLLECTOR/xs/NYTProf.xs"

usage() {
  cat <<'EOF'
Usage: g11_nodebug_stash_nogp_smoke.sh

PR-11: GP-less stash GV must not SEGV nodebug_stash / rebind_stash_slowops.
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

echo "g11_nodebug_stash_nogp_smoke: repo root $ROOT"
echo "collection_default remains v5; never crates/ on PERL5LIB"
echo "PR-11: isGV_with_GP before GvCV (GP-less stash GV is a SEGV)"

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

grep -q 'product_stash_val_cv' "$NYTP_XS" \
  || fail "NYTProf.xs missing product_stash_val_cv (GP-safe stash CV lookup)"
grep -q 'isGV_with_GP' "$NYTP_XS" \
  || fail "NYTProf.xs missing isGV_with_GP (required before GvCV)"
grep -q 'CvISXSUB(cv)' "$NYTP_XS" \
  || fail "NYTProf.xs missing CvISXSUB skip in product_rebind_cv"
# The only GvCV on a stash HE value must sit after isGV_with_GP.
if grep -n 'GvCV(' "$NYTP_XS" | grep -v 'product_stash_val_cv' | grep -v 'return GvCV' | grep -q .; then
  fail "NYTProf.xs has a GvCV() outside product_stash_val_cv (GP-less GV SEGV)"
fi
ok "PR-11 sources: GP-safe stash walk + XSUB op-walk skip"

print_residuals() {
  echo "G10 DateTime / %^H hints-safe attach: g10_datetime_hints_smoke.sh"
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
  echo "SKIP: no C toolchain (cc/gcc/clang) — G11 debugger .so not built"
  echo "  (honest skip; live attach + plant helper require a compiler)"
  print_residuals
  ok "g11_nodebug_stash_nogp_smoke completed (skip — no CC)"
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
  echo "SKIP: perl XS headers (EXTERN.h) not present — G11 debugger .so not built"
  echo "  (honest skip; live attach requires xs-nytprof)"
  print_residuals
  ok "g11_nodebug_stash_nogp_smoke completed (skip — no XS headers)"
  exit 0
fi
ok "perl + EXTERN.h present"
export CC="$CC_BIN"

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
[[ -f "$NYTP_DEST/Devel/NYTProfM.pm" ]] || fail "xs-nytprof did not copy Devel/NYTProfM.pm"
ok "xs-nytprof produced .so + .pm under collector/build/xs-nytprof"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g11-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

# Tiny helper: the only reliable way to create a GP-less SVt_PVGV.
cat >"$WORKDIR/NoGpPlant.xs" <<'EOF'
#define PERL_NO_GET_CONTEXT
#include "EXTERN.h"
#include "perl.h"
#include "XSUB.h"

MODULE = NoGpPlant PACKAGE = NoGpPlant

void
plant(name)
    const char *name
  CODE:
    {
        HV *stash;
        SV *gv;
        stash = gv_stashpv(name, GV_ADD);
        gv = newSV_type(SVt_PVGV);
        (void)hv_store(stash, "empty", 5, gv, 0);
    }
EOF
cat >"$WORKDIR/NoGpPlant.pm" <<'PM'
package NoGpPlant;
use strict;
use warnings;
our $VERSION = '0.01';
require DynaLoader;
our @ISA = qw(DynaLoader);
bootstrap NoGpPlant $VERSION;
1;
PM

(
  cd "$WORKDIR"
  perl -MExtUtils::ParseXS -e '
    ExtUtils::ParseXS::process_file(
      filename => "NoGpPlant.xs", output => "NoGpPlant.c", prototypes => 0);
  '
  perl -MConfig -e '
    my $cc = $ENV{CC} || $Config{cc};
    my $core = "$Config{archlibexp}/CORE";
    my $compile = "$cc $Config{ccflags} $Config{cccdlflags} -I$core -c -o NoGpPlant.o NoGpPlant.c";
    my $link    = "$cc $Config{lddlflags} -o NoGpPlant.so NoGpPlant.o";
    system($compile) == 0 or die "compile plant: $compile";
    system($link) == 0 or die "link plant: $link";
  '
)
mkdir -p "$WORKDIR/auto/NoGpPlant"
cp "$WORKDIR/NoGpPlant.so" "$WORKDIR/auto/NoGpPlant/NoGpPlant.so"
[[ -f "$WORKDIR/auto/NoGpPlant/NoGpPlant.so" ]] || fail "plant helper .so missing"
ok "plant helper built (GP-less SVt_PVGV)"

unset PERL5OPT || true
export PERL5LIB="$WORKDIR:$NYTP_DEST"
export NYTP_DEST WORKDIR

PROFILE="$WORKDIR/nytprof.out"
WORKLOAD="$WORKDIR/nogp.pl"
cat >"$WORKLOAD" <<'PL'
use strict;
use warnings;
use lib $ENV{NYTP_DEST};
use lib $ENV{WORKDIR};
require NoGpPlant;
{
    no strict 'refs';
    *{"NoGpStash::alive"} = sub { 1 };
}
NoGpPlant::plant("NoGpStash");
my $n = DB::nodebug_stash("NoGpStash");
die "nodebug_stash should mark at least NoGpStash::alive (got "
  . ( defined $n ? $n : 'undef' ) . ")\n"
  unless defined $n && $n >= 1;
my $r = DB::rebind_stash_slowops("NoGpStash");
die "rebind_stash_slowops returned undef\n" unless defined $r;
die "alive sub broken after nodebug/rebind\n" unless NoGpStash::alive();
print "ok-nogp nodebug=$n rebind=$r\n";
PL

echo "workdir: $WORKDIR"
echo "running: NYTPROF=file=${PROFILE} perl -d:NYTProfM <GP-less stash>"

set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="file=${PROFILE}" perl -I"$NYTP_DEST" -I"$WORKDIR" -d:NYTProfM "$WORKLOAD" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"

if [[ "$RUN_RC" -eq 139 || "$RUN_RC" -eq 134 ]]; then
  fail "perl -d:NYTProfM core-dumped on GP-less stash GV (exit $RUN_RC) — GvCV without isGV_with_GP"
fi
[[ "$RUN_RC" -eq 0 ]] || fail "perl -d:NYTProfM GP-less stash exited $RUN_RC (want 0; 139 = SEGV)"
grep -F -q 'ok-nogp' <<<"$RUN_OUT" || fail "workload did not print ok-nogp"
ok "shipped DB::nodebug_stash + rebind_stash_slowops skipped GP-less GV (no SEGV)"

[[ -f "$PROFILE" ]] || fail "NYTPROF file= did not write $PROFILE"
magic="$(head -c 9 "$PROFILE" || true)"
[[ "$magic" == "NYTProf 5" ]] || fail "produced bytes must start with NYTProf 5 (got $(printf %q "$magic"))"
ok "produced bytes start with NYTProf 5"

print_residuals
ok "G11 nodebug_stash / rebind GP-less GV fail-closed"
exit 0

#!/usr/bin/env bash
# I01 — cargo-free install of product Devel::NYTProf (D1-B XS + .pm).
#
# Builds collector xs-nytprof (libnytp_sink_v5.a + -lz only) and copies
# Devel/NYTProfM.pm, Core.pm, and NYTProfM.so into:
#   $PREFIX/lib/perl5/Devel/NYTProfM.pm
#   $PREFIX/lib/perl5/Devel/NYTProfM/Core.pm
#   $PREFIX/lib/perl5/auto/Devel/NYTProfM/NYTProfM.so
#
# Cargo is never invoked. Does not put crates/ on PERL5LIB.
# Does not install oracle XS under baseline/6.15.
# Not BUILD-003-FULL / not CPAN-TRIAL / not EL8 RPM.
#
# Usage:
#   ./scripts/packaging/install_product_xs.sh
#   NYTPROF_PREFIX=/path/to/prefix ./scripts/packaging/install_product_xs.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

COLLECTOR="$ROOT/collector"
SRC_DEST="$COLLECTOR/build/xs-nytprof"
SRC_PM="$SRC_DEST/Devel/NYTProfM.pm"
SRC_CORE="$SRC_DEST/Devel/NYTProfM/Core.pm"
SRC_SO="$SRC_DEST/auto/Devel/NYTProfM/NYTProfM.so"

[[ -f "$COLLECTOR/Makefile" ]] || fail "missing $COLLECTOR/Makefile"
[[ -f "$COLLECTOR/xs/NYTProf.xs" ]] || fail "missing collector/xs/NYTProf.xs"
[[ -f "$COLLECTOR/xs/Devel/NYTProfM.pm" ]] || fail "missing collector/xs/Devel/NYTProfM.pm"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*)
      fail "PERL5LIB must not contain /crates/: $PERL5LIB"
      ;;
  esac
fi

# shellcheck source=resolve_packaging_prefix.sh
source "$ROOT/scripts/packaging/resolve_packaging_prefix.sh"
PREFIX="$(resolve_packaging_prefix "$ROOT")"
LIB="$PREFIX/lib/perl5"

echo "install_product_xs: repo root $ROOT"
echo "prefix: $PREFIX"
echo "never crates/ on PERL5LIB; cargo is not invoked"
echo "full_build003=0; not CPAN-TRIAL; D1-B (-lz only)"

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$SRC_SO" ]] || fail "xs-nytprof did not produce $SRC_SO"
[[ -f "$SRC_PM" ]] || fail "xs-nytprof did not copy Devel/NYTProfM.pm"
[[ -f "$SRC_CORE" ]] || fail "xs-nytprof did not copy Devel/NYTProfM/Core.pm"

mkdir -p "$LIB/Devel/NYTProfM" "$LIB/auto/Devel/NYTProfM"
cp -f "$SRC_PM" "$LIB/Devel/NYTProfM.pm"
cp -f "$SRC_CORE" "$LIB/Devel/NYTProfM/Core.pm"
cp -f "$SRC_SO" "$LIB/auto/Devel/NYTProfM/NYTProfM.so"

stamp="$PREFIX/nytprof-product-xs.install"
{
  echo "installed_from=$ROOT"
  echo "prefix=$PREFIX"
  echo "libperl5=$LIB"
  echo "packaging_i01=1"
  echo "full_build003=0"
  echo "cargo_required=0"
  echo "d1_flavor=B"
  echo "product_xs=Devel::NYTProfM"
  date -u +'installed_utc=%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || echo "installed_utc=unknown"
} >"$stamp"

ok "installed product Devel::NYTProfM → $LIB"
printf '%s\n' "$LIB"
exit 0

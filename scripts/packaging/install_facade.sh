#!/usr/bin/env bash
# Install pure-Perl nytprof-engine facade into a stable on-disk prefix.
#
# BUILD-003-depth (toward full MakeMaker↔Cargo dual-build; not full XS CPAN):
#   prefix/bin/nytprof-engine
#   prefix/lib/Devel/NYTProf/*.pm
#
# Cargo is never required. Does not put crates/ on PERL5LIB.
# Does not install oracle XS under baseline/6.15.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/install_facade.sh
#   NYTPROF_PREFIX=/path/to/prefix ./scripts/packaging/install_facade.sh
#
# Prefer NYTPROF_PREFIX over bare PREFIX: ExtUtils::MakeMaker defines PREFIX
# and will rewrite an exported PREFIX when recipes run under `make`.
# Root resolution is shared with install_native.sh (resolve_packaging_prefix.sh)
# so dual-install cannot split CLI and facade.
#
# With default prefix=$REPO/prefix, the installed engine still discovers the
# workspace root (Cargo.toml walk) for native CLI lookup under prefix/bin.
# For an external prefix, set NYTPROF_MOD_ROOT to the repo and optionally
# NYTPROF_NATIVE_CLI to the native binary.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

SRC_BIN="$ROOT/perl/bin/nytprof-engine"
SRC_LIB="$ROOT/perl/lib/Devel/NYTProf"

[[ -f "$SRC_BIN" ]] || fail "missing facade script: $SRC_BIN"
[[ -d "$SRC_LIB" ]] || fail "missing facade lib dir: $SRC_LIB"

# shellcheck source=resolve_packaging_prefix.sh
source "$ROOT/scripts/packaging/resolve_packaging_prefix.sh"
PREFIX="$(resolve_packaging_prefix "$ROOT")"
BIN_DIR="$PREFIX/bin"
LIB_DEST="$PREFIX/lib/Devel/NYTProf"

mkdir -p "$BIN_DIR" "$LIB_DEST"

cp -f "$SRC_BIN" "$BIN_DIR/nytprof-engine"
chmod +x "$BIN_DIR/nytprof-engine"

# Copy only the shipped pure-Perl modules (no XS .so / no oracle tree).
shopt -s nullglob
mods=( "$SRC_LIB"/*.pm )
[[ ${#mods[@]} -gt 0 ]] || fail "no .pm files under $SRC_LIB"
for pm in "${mods[@]}"; do
  cp -f "$pm" "$LIB_DEST/"
done
shopt -u nullglob

# Stamp for depth smokes / clean-prefix honesty.
stamp="$PREFIX/nytprof-facade.install"
{
  echo "installed_from=$ROOT"
  echo "prefix=$PREFIX"
  echo "packaging_depth=BUILD-003-depth-v0"
  echo "full_build003=0"
  echo "not_full_xs_cpan=1"
  date -u +'installed_utc=%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || echo "installed_utc=unknown"
} >"$stamp"

ok "installed pure-Perl facade → $BIN_DIR/nytprof-engine"
ok "modules → $LIB_DEST ($(ls -1 "$LIB_DEST"/*.pm | wc -l) .pm)"
printf '%s\n' "$BIN_DIR/nytprof-engine"
exit 0

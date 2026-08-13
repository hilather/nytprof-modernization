#!/usr/bin/env bash
# I03 — cargo-free install of product report scripts + EngineDispatch.
#
# Copies EngineDispatch / Jsonl* / LegacyBridge (and thin Data/ReadStream if
# present) plus nytprof-engine and familiar wrappers into:
#   $PREFIX/lib/perl5/Devel/NYTProf/{EngineDispatch,JsonlData,...}.pm
#   $PREFIX/bin/{nytprof-engine,nytprofhtml,nytprofcsv,nytprofcg}
#
# Same product lib/perl5 layout as I01. Does NOT overwrite I01 debugger
# files (NYTProf.pm, Core.pm, NYTProf.so). Does NOT copy 6.15 nytprofhtml.
# Cargo is never invoked. Does not put crates/ on PERL5LIB. No CC required.
# Not BUILD-003-FULL / not CPAN-TRIAL / not COMPAT-007 / not S2.
#
# Usage:
#   ./scripts/packaging/install_product_scripts.sh
#   NYTPROF_PREFIX=/path/to/prefix ./scripts/packaging/install_product_scripts.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

SRC_BIN="$ROOT/perl/bin"
SRC_LIB="$ROOT/perl/lib/Devel/NYTProf"
SRC_ENGINE="$SRC_BIN/nytprof-engine"

[[ -f "$SRC_ENGINE" ]] || fail "missing engine script: $SRC_ENGINE"
[[ -d "$SRC_LIB" ]] || fail "missing product lib dir: $SRC_LIB"

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
BIN_DIR="$PREFIX/bin"
LIB="$PREFIX/lib/perl5"
LIB_DEST="$LIB/Devel/NYTProf"

echo "install_product_scripts: repo root $ROOT"
echo "prefix: $PREFIX"
echo "never crates/ on PERL5LIB; cargo is not invoked"
echo "full_build003=0; not CPAN-TRIAL; not COMPAT-007"

mkdir -p "$BIN_DIR" "$LIB_DEST"

# Product EngineDispatch + JSONL query stack (required).
required_pm=(
  EngineDispatch.pm
  JsonlData.pm
  JsonlReadStream.pm
  LegacyBridge.pm
)
for pm in "${required_pm[@]}"; do
  [[ -f "$SRC_LIB/$pm" ]] || fail "missing $SRC_LIB/$pm"
  cp -f "$SRC_LIB/$pm" "$LIB_DEST/$pm"
done

# Thin product Data/ReadStream (PERL-XS-DATA-READSTREAM-MVP) when present.
# Do not copy I01 debugger files (NYTProf.pm / Core.pm live elsewhere).
for pm in Data.pm ReadStream.pm; do
  if [[ -f "$SRC_LIB/$pm" ]]; then
    cp -f "$SRC_LIB/$pm" "$LIB_DEST/$pm"
  fi
done

cp -f "$SRC_ENGINE" "$BIN_DIR/nytprof-engine"
chmod +x "$BIN_DIR/nytprof-engine"

# Familiar wrappers: exec sibling nytprof-engine (not 6.15 oracle scripts).
for wrap in nytprofhtml nytprofcsv nytprofcg; do
  [[ -f "$SRC_BIN/$wrap" ]] || fail "missing wrapper $SRC_BIN/$wrap"
  cp -f "$SRC_BIN/$wrap" "$BIN_DIR/$wrap"
  chmod +x "$BIN_DIR/$wrap"
done

stamp="$PREFIX/nytprof-product-scripts.install"
{
  echo "installed_from=$ROOT"
  echo "prefix=$PREFIX"
  echo "libperl5=$LIB"
  echo "packaging_i03=1"
  echo "full_build003=0"
  echo "cargo_required=0"
  echo "product_scripts=nytprof-engine,nytprofhtml,nytprofcsv,nytprofcg"
  date -u +'installed_utc=%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || echo "installed_utc=unknown"
} >"$stamp"

ok "installed product scripts → $BIN_DIR"
ok "modules → $LIB_DEST"
printf '%s\n' "$BIN_DIR/nytprof-engine"
exit 0

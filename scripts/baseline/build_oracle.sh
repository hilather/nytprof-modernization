#!/usr/bin/env bash
# Clean-build the pinned 6.15 oracle into baseline/6.15/install (BASE-001).
set -euo pipefail
# shellcheck source=common.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

if [[ ! -f "$SRC_DIR/Makefile.PL" ]]; then
  echo "Source missing; run scripts/baseline/fetch_oracle.sh first" >&2
  exit 1
fi

# Isolation: do not use candidate modules
unset PERL5LIB
export PERL5LIB=""
# Ensure we do not pick up a site-local NYTProf accidentally during build tools
export PERL_LOCAL_LIB_ROOT=""
export PERL_MB_OPT=""
export PERL_MM_OPT=""

BUILD_LOG="$LOG_DIR/build_oracle.log"
rm -rf "$INSTALL_DIR"
mkdir -p "$INSTALL_DIR" "$LOG_DIR"

cd "$SRC_DIR"
# Clean any prior in-tree build
if [[ -f Makefile ]]; then
  make distclean >/dev/null 2>&1 || true
fi

# macOS / Xcode: zlib.h lives under the SDK (not bare /usr/include). NYTProf's
# Makefile.PL only searches INCLUDE + a few fixed dirs; without the SDK path it
# silently builds without HAS_ZLIB and later fails on compressed fixtures
# ("File uses compression but compression is not supported by this build").
if [[ "$(uname -s 2>/dev/null || true)" == "Darwin" ]]; then
  SDKROOT="${SDKROOT:-}"
  if [[ -z "$SDKROOT" ]] && command -v xcrun >/dev/null 2>&1; then
    SDKROOT="$(xcrun --show-sdk-path 2>/dev/null || true)"
  fi
  if [[ -n "$SDKROOT" && -f "$SDKROOT/usr/include/zlib.h" ]]; then
    export INCLUDE="${INCLUDE:+$INCLUDE:}$SDKROOT/usr/include"
    # Help the linker resolve -lz from the same SDK when needed.
    if [[ -d "$SDKROOT/usr/lib" ]]; then
      export LIBRARY_PATH="${LIBRARY_PATH:+$LIBRARY_PATH:}$SDKROOT/usr/lib"
    fi
    echo "build_oracle: Darwin SDK zlib via INCLUDE=$SDKROOT/usr/include"
  else
    echo "build_oracle: WARNING: Darwin zlib.h not found under SDKROOT=${SDKROOT:-<unset>}; oracle may lack compression" >&2
  fi
  # Homebrew zlib (optional extra) when present.
  if command -v brew >/dev/null 2>&1; then
    BZ="$(brew --prefix zlib 2>/dev/null || true)"
    if [[ -n "$BZ" && -f "$BZ/include/zlib.h" ]]; then
      export INCLUDE="${INCLUDE:+$INCLUDE:}$BZ/include"
      export LIBRARY_PATH="${LIBRARY_PATH:+$LIBRARY_PATH:}$BZ/lib"
      echo "build_oracle: also using brew zlib at $BZ"
    fi
  fi
fi

{
  echo "=== build_oracle $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
  echo "ROOT=$ROOT"
  echo "SRC_DIR=$SRC_DIR"
  echo "INSTALL_DIR=$INSTALL_DIR"
  echo "PERL5LIB=${PERL5LIB-<unset>}"
  echo "INCLUDE=${INCLUDE-<unset>}"
  echo "LIBRARY_PATH=${LIBRARY_PATH-<unset>}"
  which perl
  perl -V
  echo "=== perl Makefile.PL ==="
  perl Makefile.PL INSTALL_BASE="$INSTALL_DIR" POLLUTE=0
  echo "=== make ==="
  make -j"$(nproc 2>/dev/null || echo 2)"
  echo "=== make install ==="
  make install
} 2>&1 | tee "$BUILD_LOG"

# Fail closed if zlib was not enabled: compressed v5 fixtures are required by the
# offline gate dual_path smoke. Makefile.PL prints "Found deflateInit2 in zlib.h".
if ! grep -q 'Found deflateInit2 in zlib.h' "$BUILD_LOG" \
  && ! grep -qE -- '-DHAS_ZLIB' "$BUILD_LOG"; then
  echo "ERROR: oracle build did not enable HAS_ZLIB (zlib not detected).
  Compressed fixtures will fail on this host. On macOS, ensure Xcode CLI tools
  provide zlib.h (xcrun --show-sdk-path) or install brew zlib and re-run.
  Log: $BUILD_LOG" >&2
  exit 1
fi

# Locate installed module
MOD_PATH="$(find "$INSTALL_DIR" -path '*/Devel/NYTProf.pm' | head -1)"
if [[ -z "$MOD_PATH" ]]; then
  echo "ERROR: Devel/NYTProf.pm not found under $INSTALL_DIR" >&2
  exit 1
fi
echo "Installed module: $MOD_PATH"
# Smoke-load with only install tree on PERL5LIB
ARCH_LIB="$(find "$INSTALL_DIR" -type d -name 'auto' | head -1 | xargs -I{} dirname {} | head -1)"
# Prefer standard local::lib layout
export PERL5LIB="$(find "$INSTALL_DIR" -type d -name 'lib' | tr '\n' ':' | sed 's/:$//')"
# Also add arch-specific lib if present
while IFS= read -r d; do
  case ":$PERL5LIB:" in
    *":$d:"*) ;;
    *) PERL5LIB="${PERL5LIB:+$PERL5LIB:}$d" ;;
  esac
done < <(find "$INSTALL_DIR" -type d \( -name 'lib' -o -path '*/lib/perl5/*' \) 2>/dev/null | head -50)

# Build a clean PERL5LIB of all lib dirs under install
PERL5LIB="$(python3 - <<'PY'
import os
from pathlib import Path
install = Path(os.environ["INSTALL_DIR"])
libs = sorted({str(p.parent) for p in install.rglob("NYTProf.pm") if "Devel" in p.parts})
# parent of Devel is lib root
roots = sorted({str(Path(p).parent.parent) for p in libs})
# also include any */lib/perl5 and */lib/perl5/arch
extra = []
for p in install.rglob("*"):
    if p.is_dir() and p.name in ("perl5",) and "lib" in p.parts:
        extra.append(str(p))
    if p.is_dir() and (p / "auto").is_dir():
        extra.append(str(p))
print(":".join(dict.fromkeys(roots + extra)))
PY
)"
export PERL5LIB

perl -MDevel::NYTProf -e 'print "Devel::NYTProf $Devel::NYTProf::VERSION loaded from ", $INC{"Devel/NYTProf.pm"}, "\n"'
LOADED_PATH="$(perl -MDevel::NYTProf -e 'print $INC{"Devel/NYTProf.pm"}')"
case "$LOADED_PATH" in
  "$INSTALL_DIR"/*) echo "OK: oracle module loaded from install tree" ;;
  *)
    echo "ERROR: loaded module not from install tree: $LOADED_PATH" >&2
    exit 1
    ;;
esac

# Refuse candidate contamination
if [[ "$LOADED_PATH" == *"/crates/"* ]] || [[ "$LOADED_PATH" == *"/perl/lib/"* ]]; then
  echo "ERROR: candidate path contamination" >&2
  exit 1
fi

printf '%s\n' "$PERL5LIB" > "$BASELINE_DIR/oracle-perl5lib.txt"
printf '%s\n' "$LOADED_PATH" > "$BASELINE_DIR/oracle-module-path.txt"
echo "Build complete. Log: $BUILD_LOG"

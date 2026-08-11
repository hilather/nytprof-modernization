#!/usr/bin/env bash
# Install native nytprof CLI into a stable on-disk prefix for Perl dispatch.
#
# Spec: docs/schemas/native-install-mvp-v0.md
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/install_native.sh
#   NYTPROF_PREFIX=/path/to/prefix ./scripts/packaging/install_native.sh
#   PREFIX=/path/to/prefix ./scripts/packaging/install_native.sh   # also accepted
#   NATIVE_RELEASE=1 ./scripts/packaging/install_native.sh
#
# Prefer NYTPROF_PREFIX over bare PREFIX when invoking via `make` (MakeMaker
# defines PREFIX and rewrites exported PREFIX in recipe environments).
# Root resolution is shared with install_facade.sh (resolve_packaging_prefix.sh)
# so dual-install cannot split CLI and facade.
#
# Never mutates oracle PERL5LIB or puts crates/ on the module path.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml (need workspace)"
[[ -d "$ROOT/crates/nytprof-cli" ]] || fail "missing crates/nytprof-cli"
command -v cargo >/dev/null 2>&1 || fail "cargo required for install_native.sh"

# shellcheck source=resolve_packaging_prefix.sh
source "$ROOT/scripts/packaging/resolve_packaging_prefix.sh"
PREFIX="$(resolve_packaging_prefix "$ROOT")"
BIN_DIR="$PREFIX/bin"

if [[ "${NATIVE_RELEASE:-0}" == "1" ]]; then
  BUILD_PROFILE=release
  cargo build -q --release -p nytprof-cli
  SRC="$ROOT/target/release/nytprof-dump"
else
  BUILD_PROFILE=debug
  cargo build -q -p nytprof-cli
  SRC="$ROOT/target/debug/nytprof-dump"
fi

[[ -f "$SRC" ]] || fail "built binary missing at $SRC"
[[ -x "$SRC" ]] || fail "built binary not executable: $SRC"

mkdir -p "$BIN_DIR"
cp -f "$SRC" "$BIN_DIR/nytprof-cli"
cp -f "$SRC" "$BIN_DIR/nytprof-dump"
chmod +x "$BIN_DIR/nytprof-cli" "$BIN_DIR/nytprof-dump"

# Stamp for depth smokes (shared-root checks with facade install).
stamp="$PREFIX/nytprof-native.install"
{
  echo "installed_from=$ROOT"
  echo "prefix=$PREFIX"
  echo "packaging_depth=BUILD-003-depth-v0"
  echo "full_build003=0"
  echo "not_full_xs_cpan=1"
  echo "profile=$BUILD_PROFILE"
  date -u +'installed_utc=%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || echo "installed_utc=unknown"
} >"$stamp"

ok "installed native CLI ($BUILD_PROFILE) → $BIN_DIR/nytprof-cli"
ok "alias also at $BIN_DIR/nytprof-dump"
printf '%s\n' "$BIN_DIR/nytprof-cli"
exit 0

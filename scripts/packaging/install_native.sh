#!/usr/bin/env bash
# Install native nytprof CLI into a stable on-disk prefix for Perl dispatch.
#
# Spec: docs/schemas/native-install-mvp-v0.md
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/install_native.sh
#   PREFIX=/path/to/prefix ./scripts/packaging/install_native.sh
#   NATIVE_RELEASE=1 ./scripts/packaging/install_native.sh
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

PREFIX="${PREFIX:-$ROOT/prefix}"
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

ok "installed native CLI ($BUILD_PROFILE) → $BIN_DIR/nytprof-cli"
ok "alias also at $BIN_DIR/nytprof-dump"
printf '%s\n' "$BIN_DIR/nytprof-cli"
exit 0

#!/usr/bin/env bash
# Build an unsigned Rocky 8 / EL8 x86_64 nytprof-cli for the module RPM.
# Cargo runs in rockylinux:8 (not in mock %build). Test-drive: no GPG.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
OUT_DIR="$ROOT/packaging/prebuilt/el8-x86_64"
IMAGE="${NYTPROF_EL8_IMAGE:-rockylinux:8}"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

command -v docker >/dev/null 2>&1 || fail "docker required to build the EL8 prebuilt"
[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -d "$ROOT/crates/nytprof-cli" ]] || fail "missing crates/nytprof-cli"

mkdir -p "$OUT_DIR"
echo "building nytprof-cli in $IMAGE (release, strip)"
docker run --rm \
  -v "$ROOT:/src:rw" \
  -w /src \
  -e CARGO_HOME=/tmp/nytprof-el8-cargo \
  -e RUSTUP_HOME=/tmp/nytprof-el8-rustup \
  "$IMAGE" \
  bash -lc '
    set -euo pipefail
    yum -y install gcc gcc-c++ make openssl-devel curl ca-certificates binutils
    if [[ ! -x "${CARGO_HOME}/bin/rustc" ]]; then
      curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal --no-modify-path
    fi
    export PATH="${CARGO_HOME}/bin:${PATH}"
    cargo build --release -p nytprof-cli
    strip -s target/release/nytprof-dump
    install -m 755 target/release/nytprof-dump /src/packaging/prebuilt/el8-x86_64/nytprof-cli
    rustc --version
    ldd /src/packaging/prebuilt/el8-x86_64/nytprof-cli || true
  '

[[ -x "$OUT_DIR/nytprof-cli" ]] || fail "docker build did not write $OUT_DIR/nytprof-cli"
file "$OUT_DIR/nytprof-cli"
ldd "$OUT_DIR/nytprof-cli" || true
# Freshness marker: release-el8-rpm.yml fails closed when the committed marker
# does not match the tag's source hash (ADR-0010 test-drive gate).
"$ROOT/scripts/packaging/cli_source_sha256.sh" > "$OUT_DIR/nytprof-cli.source-sha256"
ok "EL8 prebuilt → $OUT_DIR/nytprof-cli"
ok "source hash marker → $OUT_DIR/nytprof-cli.source-sha256 ($(cat "$OUT_DIR/nytprof-cli.source-sha256"))"
printf '%s\n' "$OUT_DIR/nytprof-cli"

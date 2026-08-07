#!/usr/bin/env bash
# Optional native (Cargo) packaging smoke.
#
# This path is NOT required for oracle rebuild or legacy-only installs.
# Skip entirely when cargo is absent — that is a successful "not applicable"
# outcome for Hybrid/standalone tool jobs only when Cargo is expected.
#
# Usage:
#   ./scripts/packaging/native_optional_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

if [[ ! -f "$ROOT/Cargo.toml" ]] || [[ ! -d "$ROOT/crates" ]]; then
  echo "SKIP: crates/ Cargo workspace not present (native tools optional)"
  exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "SKIP: cargo not on PATH (native smoke is optional; legacy-only is enough)"
  exit 0
fi

ok "cargo available: $(cargo --version 2>/dev/null || echo unknown)"
ok "running optional workspace package tests (format-v5, model, report, cli)"

# Focused packages used by offline native tools; does not touch oracle PERL5LIB.
cargo test \
  -p nytprof-format-v5 \
  -p nytprof-model \
  -p nytprof-report \
  -p nytprof-cli

ok "native optional packaging smoke passed"
exit 0

#!/usr/bin/env bash
# Content hash of the nytprof-cli source inputs (crates/ + workspace manifests).
#
# Single source of truth for the EL8 prebuilt freshness contract:
#   - build_el8_nytprof_cli.sh writes the output into
#     packaging/prebuilt/el8-x86_64/nytprof-cli.source-sha256 after each build
#   - .github/workflows/release-el8-rpm.yml recomputes it on the tag checkout
#     and fails closed when it differs from the committed marker
#
# Hashes the working tree (tracked + untracked, not ignored) because the docker
# build mounts the working tree, not a git ref. Output: one hex sha256 line.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
[[ -d "$ROOT/crates/nytprof-cli" ]] || { echo "ERROR: missing crates/nytprof-cli" >&2; exit 1; }
git ls-files -z --cached --others --exclude-standard -- crates Cargo.toml Cargo.lock \
  | sort -z \
  | xargs -0 sha256sum \
  | sha256sum \
  | cut -d' ' -f1

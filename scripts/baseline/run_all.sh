#!/usr/bin/env bash
# Full BASE-001 pipeline: fetch → build → test → manifest.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
"$DIR/fetch_oracle.sh"
"$DIR/build_oracle.sh"
"$DIR/test_oracle.sh"
"$DIR/write_manifest.sh"
echo "BASE-001 pipeline complete."

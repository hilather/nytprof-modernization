#!/usr/bin/env bash
# Backward-compatible entry: full normalize+compare harness.
# Prefer: ./tools/oracle/selftest_harness.sh
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$DIR/selftest_harness.sh"

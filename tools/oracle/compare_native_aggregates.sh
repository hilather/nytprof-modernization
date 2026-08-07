#!/usr/bin/env bash
# Compare native ProfileModel aggregates to committed aggregates.oracle.json.
#
# Preferred gate is cargo test in nytprof-model:
#   cargo test -p nytprof-model native_matches_aggregates_oracle_json
#
# This script is a thin wrapper so operators can re-run the same check from
# tools/oracle without memorizing the filter. Requires a Rust toolchain.
#
# Optional non-fatal note when baseline nytprofcsv is present (layout not
# required to match; contracted field values only — see aggregate-comparison-v0).
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
cd "$ROOT"

log() { printf '%s\n' "$*"; }

if ! command -v cargo >/dev/null 2>&1; then
  log "ERROR: cargo not found; install Rust or run the Python oracle selftest only:" >&2
  log "  ./tools/oracle/selftest_aggregates.sh" >&2
  exit 1
fi

log "=== native vs aggregates.oracle.json (cargo test) ==="
cargo test -p nytprof-model native_matches_aggregates_oracle_json -- --nocapture

# Optional: mention legacy nytprofcsv without requiring a pass.
NYTPROFCSV="$ROOT/baseline/6.15/install/bin/nytprofcsv"
if [[ -x "$NYTPROFCSV" ]]; then
  log ""
  log "NOTE: legacy nytprofcsv is available at:"
  log "  $NYTPROFCSV"
  log "  Spot-check contracted sub/edge fields only if desired;"
  log "  byte-identical CSV layout is NOT required (aggregate-comparison-v0)."
else
  log ""
  log "NOTE: baseline nytprofcsv not present (optional; non-fatal)."
fi

log ""
log "compare_native_aggregates: PASS"
exit 0

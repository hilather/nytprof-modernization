#!/usr/bin/env bash
# DECODE-FUZZ-MVP: smoke that shipped decode/verify fuzz batteries stay green.
#
# Schema: docs/schemas/decode-fuzz-mvp-v0.md
# Board: DECODE-FUZZ-MVP
#
# Runs the named Cargo test filters for:
#   - nytprof-format-v5: decode_fuzz_no_panic_* + fuzz_truncated_mutations
#   - nytprof-report:    decode_fuzz_no_panic_* + fuzz_truncated_mutations_verify
#
# Does not reimplement the decoder. Does not require oracle Perl / PERL5LIB.
#
# Usage:
#   bash tools/oracle/selftest_decode_fuzz.sh
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
cd "$ROOT"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

FIXTURE="$ROOT/fixtures/v5/default-calls1/nytprof.out"
[[ -f "$FIXTURE" ]] || fail "missing fixture $FIXTURE"

if ! command -v cargo >/dev/null 2>&1; then
  fail "cargo required for decode-fuzz cargo tests (no cargo on PATH)"
fi

log "selftest_decode_fuzz: format-v5 decode_fuzz / fuzz_truncated_mutations"
cargo test -q -p nytprof-format-v5 decode_fuzz_no_panic -- --nocapture
cargo test -q -p nytprof-format-v5 fuzz_truncated_mutations -- --nocapture
ok "nytprof-format-v5 decode fuzz battery"

log "selftest_decode_fuzz: report verify fuzz / fuzz_truncated_mutations_verify"
cargo test -q -p nytprof-report decode_fuzz_no_panic -- --nocapture
cargo test -q -p nytprof-report fuzz_truncated_mutations_verify -- --nocapture
ok "nytprof-report verify fuzz battery"

log "selftest_decode_fuzz: PASS"
exit 0

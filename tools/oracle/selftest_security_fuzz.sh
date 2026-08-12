#!/usr/bin/env bash
# SEC-FUZZ-HARDENING-MVP: offline security/fuzz package smoke.
#
# Contract: docs/contracts/SECURITY_FUZZ_HARDENING_PACKAGE_v0.md
# Schema:   docs/schemas/security-fuzz-hardening-mvp-v0.md
# Board:    SEC-FUZZ-HARDENING-MVP
#
# Runs:
#   1. v5 format + report decode-fuzz batteries (DECODE-FUZZ-MVP)
#   2. v6 C-fixture decode-fuzz battery (V6-DECODE-FUZZ)
#   3. collector unit suite when CC present (batch SV lifetime + fork state MVP)
#
# Does not reimplement decoders. Does not require oracle Perl / PERL5LIB.
# Not full SEC-002 continuous fuzz; not COL-015 complete.
#
# Usage:
#   bash tools/oracle/selftest_security_fuzz.sh
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
cd "$ROOT"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }
note() { printf 'NOTE: %s\n' "$*"; }

V5_FIXTURE="$ROOT/fixtures/v5/default-calls1/nytprof.out"
[[ -f "$V5_FIXTURE" ]] || fail "missing fixture $V5_FIXTURE"

for f in absolute packing dict; do
  [[ -f "$ROOT/fixtures/v6/from-c/${f}.nytprof" ]] \
    || fail "missing C fixture fixtures/v6/from-c/${f}.nytprof"
done

if ! command -v cargo >/dev/null 2>&1; then
  fail "cargo required for security-fuzz cargo batteries (no cargo on PATH)"
fi

# ---------------------------------------------------------------------------
# 1. v5 decode / verify batteries (reuse DECODE-FUZZ-MVP filters)
# ---------------------------------------------------------------------------
log "selftest_security_fuzz: format-v5 decode_fuzz / fuzz_truncated_mutations"
cargo test -q -p nytprof-format-v5 decode_fuzz_no_panic -- --nocapture
cargo test -q -p nytprof-format-v5 fuzz_truncated_mutations -- --nocapture
ok "nytprof-format-v5 decode fuzz battery"

log "selftest_security_fuzz: report verify fuzz"
cargo test -q -p nytprof-report decode_fuzz_no_panic -- --nocapture
cargo test -q -p nytprof-report fuzz_truncated_mutations_verify -- --nocapture
ok "nytprof-report verify fuzz battery"

# ---------------------------------------------------------------------------
# 2. v6 always-inflate EVENT decode-fuzz on C fixtures
# ---------------------------------------------------------------------------
log "selftest_security_fuzz: format-v6 v6_decode_fuzz / fuzz_truncated_mutations_v6"
cargo test -q -p nytprof-format-v6 v6_decode_fuzz -- --nocapture
cargo test -q -p nytprof-format-v6 fuzz_truncated_mutations_v6 -- --nocapture
ok "nytprof-format-v6 v6 decode fuzz battery"

# ---------------------------------------------------------------------------
# 3. Collector threat evidence (batch SV lifetime + fork state MVP)
# ---------------------------------------------------------------------------
if command -v make >/dev/null 2>&1 && command -v cc >/dev/null 2>&1; then
  log "selftest_security_fuzz: make -C collector test (batch/fork threat evidence)"
  make -C "$ROOT/collector" test
  ok "collector unit suite (includes test_sv_lifetime + test_fork_split_seq_reset)"
elif command -v make >/dev/null 2>&1 && command -v gcc >/dev/null 2>&1; then
  log "selftest_security_fuzz: make -C collector test (gcc)"
  make -C "$ROOT/collector" test
  ok "collector unit suite (includes test_sv_lifetime + test_fork_split_seq_reset)"
else
  note "C toolchain (cc/gcc) not on PATH — collector batch/fork unit evidence skipped"
  note "  (honest skip; Rust decode batteries still green)"
fi

log "selftest_security_fuzz: PASS"
log "NOTE: not full SEC-002 continuous fuzz; COL-015 full fork suite residual"
exit 0

#!/usr/bin/env bash
# SEC-002 continuous-fuzz job MVP (P02).
#
# Thin wrapper: invoke the shipped security-fuzz entry
#   tools/oracle/selftest_security_fuzz.sh
# (v5 + v6 decode_fuzz batteries). Does not reimplement decode.
#
# Honest SKIP: when cargo is absent (selftest_security_fuzz.sh requires cargo).
# Never crates/ on oracle PERL5LIB.
#
# Not full SEC-002 (cargo-fuzz / AFL / scheduled deep corpus).
# Not SEC-012 independent sign-off / GA marketing.
#
# Usage:
#   bash scripts/ci/sec002_continuous_fuzz_mvp.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

SELFTEST="$ROOT/tools/oracle/selftest_security_fuzz.sh"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "sec002_continuous_fuzz_mvp: repo root $ROOT"
echo "invokes shipped selftest_security_fuzz.sh / decode_fuzz; not cargo-fuzz/AFL"
echo "never crates/ on PERL5LIB; not SEC-012 complete; not GA marketing"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$SELFTEST" ]] || fail "missing shipped fuzz entry: $SELFTEST"
[[ -x "$SELFTEST" || -r "$SELFTEST" ]] || fail "unreadable shipped fuzz entry: $SELFTEST"

if ! command -v cargo >/dev/null 2>&1; then
  echo "SKIP: no cargo on PATH — SEC-002 job MVP cannot run decode_fuzz batteries"
  echo "NOTE: not full SEC-002 continuous fuzz; not independent SEC-012 sign-off"
  ok "sec002_continuous_fuzz_mvp honest skip (no cargo)"
  exit 0
fi

echo "running shipped: $SELFTEST"
exec bash "$SELFTEST"

#!/usr/bin/env bash
# A5a — stub GPG key + unsigned-bootstrap yum runbook. No live key. No COPR.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

KEY="$ROOT/packaging/rpm/RPM-GPG-KEY-nytprofm"
README="$ROOT/packaging/rpm/README.md"
MIG="$ROOT/docs/MIGRATION_DROP_IN_v0.md"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "a5a_gpg_stub_runbook_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$KEY" ]] || fail "missing $KEY"
[[ -f "$README" ]] || fail "missing $README"
[[ -f "$MIG" ]] || fail "missing $MIG"

grep -F -q 'NYTPROFM-GPG-STUB' "$KEY" \
  || fail "stub key missing NYTPROFM-GPG-STUB sentinel"
if grep -F -q 'BEGIN PGP PUBLIC KEY BLOCK' "$KEY"; then
  fail "A5a stub must not contain a live PGP public key block"
fi
grep -Eiq 'NOT A LIVE KEY|not a live key' "$KEY" \
  || fail "stub key must say it is not a live key"
ok "stub key is a sentinel, not a live pubkey"

grep -Eiq 'unsigned internal' "$README" \
  || fail "README missing unsigned internal bootstrap title"
grep -F -q 'gpgcheck=0' "$README" \
  || fail "README missing gpgcheck=0 temporary recipe"
grep -Eiq 'not a production policy' "$README" \
  || fail "README must say gpgcheck=0 is not a production policy"
if grep -Eiq 'dnf copr enable' "$README"; then
  fail "README must not teach dnf copr enable before A5b"
fi
ok "README unsigned-bootstrap runbook"

grep -F -q 'nytprofhtml' "$MIG" \
  || fail "MIG01 missing product nytprofhtml in the module RPM"
if grep -Eiq 'module RPM is attach-only' "$MIG"; then
  fail "MIG01 still says the module RPM is attach-only (scripts now ship)"
fi
grep -Eiq 'unsigned internal bootstrap|not a production policy' "$MIG" \
  || fail "MIG01 missing unsigned-bootstrap / not-production-policy wording"
grep -F -q 'gpgcheck=0' "$MIG" \
  || fail "MIG01 missing gpgcheck=0 bootstrap recipe"
ok "MIG01 product nytprofhtml in module RPM + unsigned bootstrap"

echo "NOT-YET: A5b live rpmsign / COPR / gpgcheck=1"
echo "NOT-YET: C1 signed nytprof-cli pipeline"
ok "A5A-GPG-STUB-RUNBOOK"
exit 0

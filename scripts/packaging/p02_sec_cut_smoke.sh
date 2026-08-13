#!/usr/bin/env bash
# P02 — SEC-012 checklist + SEC-002 job MVP (not GA marketing sign-off).
#
# Reads the real SEC-012 checklist. Asserts the real SEC-002 workflow +
# wrapper invoke shipped selftest_security_fuzz.sh / decode_fuzz.
# Drives the shipped wrapper (honest SKIP without cargo).
# Never crates/ on oracle PERL5LIB.
#
# Exit 0: P02 pass. Exit 1: checklist/job failure. Exit 2: misuse / crates/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

CHECKLIST="$ROOT/docs/contracts/SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md"
SCHEMA="$ROOT/docs/schemas/p02-sec-cut-mvp-v0.md"
WRAPPER="$ROOT/scripts/ci/sec002_continuous_fuzz_mvp.sh"
WORKFLOW="$ROOT/.github/workflows/sec002-fuzz-mvp.yml"
SELFTEST="$ROOT/tools/oracle/selftest_security_fuzz.sh"

usage() {
  cat <<'EOF'
Usage: p02_sec_cut_smoke.sh

P02: real SEC-012 checklist + real SEC-002 job/script invoking
shipped selftest_security_fuzz.sh / decode_fuzz (or honest SKIP).
Not independent sign-off / not GA marketing / not full continuous fuzz.
EOF
}

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    *) printf 'ERROR: unknown flag: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

echo "p02_sec_cut_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; not independent sign-off; not GA marketing"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$CHECKLIST" ]] || fail "missing real SEC-012 checklist: $CHECKLIST"
[[ -f "$SCHEMA" ]] || fail "missing P02 schema: $SCHEMA"
[[ -f "$WRAPPER" ]] || fail "missing SEC-002 wrapper: $WRAPPER"
[[ -f "$WORKFLOW" ]] || fail "missing SEC-002 workflow: $WORKFLOW"
[[ -f "$SELFTEST" ]] || fail "missing shipped fuzz entry: $SELFTEST"
[[ -x "$WRAPPER" ]] || fail "not executable: $WRAPPER"

# --- real checklist (not a smoke-only string dump) ---
# Allow optional markdown ** around words so bold table cells still match.
BLOB="$(cat "$CHECKLIST")"
for needle in \
  'covered surfaces' \
  'residual' \
  'SEC-012' \
  'SEC-002' \
  'decode_fuzz' \
  'v5' \
  'v6' \
  'selftest_security_fuzz.sh'
do
  grep -F -q -- "$needle" <<<"$BLOB" \
    || fail "SEC-012 checklist missing required honesty string: $needle"
done
grep -Eiq 'not([[:space:]]|\*\*)+independent[[:space:]]+sign-off' "$CHECKLIST" \
  || fail "checklist must deny independent sign-off"
grep -Eiq 'not([[:space:]]|\*\*)+GA[[:space:]]+marketing' "$CHECKLIST" \
  || fail "checklist must deny GA marketing"
if grep -Eiq 'SEC-012 complete GA|SEC-012 is done|independent sign-off is complete|GA marketing complete' "$CHECKLIST"; then
  fail "checklist must not claim SEC-012 complete or GA marketing"
fi
ok "real SEC-012 checklist has covered surfaces, residuals, and non-claims"

# --- workflow exists and job/script invokes shipped fuzz entry ---
JOB_BLOB="$(cat "$WORKFLOW" "$WRAPPER")"
if ! grep -F -q 'selftest_security_fuzz.sh' <<<"$JOB_BLOB" \
  && ! grep -E -q 'cargo test.*decode_fuzz|decode_fuzz' <<<"$JOB_BLOB"; then
  fail "SEC-002 workflow/script must invoke shipped selftest_security_fuzz.sh or cargo test decode_fuzz"
fi
grep -F -q 'selftest_security_fuzz.sh' "$WRAPPER" \
  || fail "SEC-002 wrapper must name shipped selftest_security_fuzz.sh"
grep -F -q 'sec002_continuous_fuzz_mvp.sh' "$WORKFLOW" \
  || fail "workflow must invoke scripts/ci/sec002_continuous_fuzz_mvp.sh"
grep -F -q 'selftest_security_fuzz.sh' "$WORKFLOW" \
  || fail "workflow must name shipped selftest_security_fuzz.sh"
ok "SEC-002 workflow + wrapper invoke shipped selftest_security_fuzz.sh / decode_fuzz"

# --- drive the real SEC-002 MVP entry ---
echo "running shipped SEC-002 MVP: $WRAPPER"
set +e
WRAP_OUT="$(bash "$WRAPPER" 2>&1)"
WRAP_RC=$?
set -e
printf '%s\n' "$WRAP_OUT"
[[ "$WRAP_RC" -eq 0 ]] || fail "sec002_continuous_fuzz_mvp.sh failed (rc=$WRAP_RC)"
if grep -E -q 'SKIP: no cargo' <<<"$WRAP_OUT"; then
  ok "SEC-002 honest SKIP without cargo"
else
  grep -E -q 'selftest_security_fuzz: PASS|decode fuzz battery' <<<"$WRAP_OUT" \
    || fail "wrapper did not run shipped selftest_security_fuzz / decode_fuzz"
  ok "SEC-002 drove shipped selftest_security_fuzz.sh / decode_fuzz"
fi

echo "NOT-YET: independent SEC-012 sign-off / GA marketing complete"
echo "NOT-YET: full SEC-002 cargo-fuzz / AFL / deep corpus"
echo "NOT-YET: BUILD-003-FULL / PRODUCT-V6-COLLECT-EL8 / S2 / R3-R4 flip"
ok "P02-SEC-CUT"
exit 0

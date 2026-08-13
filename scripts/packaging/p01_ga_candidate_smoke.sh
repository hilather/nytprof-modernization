#!/usr/bin/env bash
# P01 — GA-candidate drop-in honesty cut (not final GA marketing).
#
# Reads the real GA-candidate notes + Changes. Drives real capability
# (collection_default v5) and shipped G04 attach 15/3/15 when CC/XS exist.
# Never crates/ on oracle PERL5LIB. Does not claim SEC-012 / PAUSE / R3–R4.
#
# Exit 0: P01 pass. Exit 1: notes/attach failure. Exit 2: misuse / crates/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

NOTES="$ROOT/docs/RELEASE_NOTES_GA_CANDIDATE_v0.md"
CHANGES="$ROOT/Changes"
G04="$ROOT/scripts/packaging/g04_v5_parity_smoke.sh"

usage() {
  cat <<'EOF'
Usage: p01_ga_candidate_smoke.sh

P01: real GA-candidate notes + capability v5 + G04 attach 15/3/15.
Not SEC-012 / not PAUSE / not R3-R4 flip.
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

echo "p01_ga_candidate_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; not SEC-012 complete; not R3/R4 flip"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$NOTES" ]] || fail "missing $NOTES"
[[ -f "$CHANGES" ]] || fail "missing $CHANGES"
[[ -x "$G04" ]] || fail "missing shipped G04: $G04"

BLOB="$(cat "$NOTES" "$CHANGES")"
for needle in \
  'Devel::NYTProf' \
  '7.00' \
  'collection drop-in preview' \
  'D1-B' \
  'WAIVE' \
  'tablesorter' \
  'entersub' \
  'collection_default' \
  'v5' \
  'PRODUCT-V6-COLLECT-EL8'
do
  grep -F -q -- "$needle" <<<"$BLOB" \
    || fail "GA-candidate notes/Changes missing required string: $needle"
done
grep -Eiq 'Rocky.*D1-B|D1-B only|default RPM = D1-B' <<<"$BLOB" \
  || fail "notes must say Rocky default RPM is D1-B only"
grep -Eiq 'not uploaded to PAUSE' <<<"$BLOB" \
  || fail "notes must say not uploaded to PAUSE"
grep -Eiq 'not SEC-012 complete|Not claimed' "$NOTES" \
  || fail "notes must not claim SEC-012 complete (must deny it)"
if grep -Eiq 'SEC-012 complete GA|SEC-012 is done|R3 flipped|R4 flipped|R4 default flip executed' "$NOTES"; then
  fail "notes must not claim SEC-012 complete or R3/R4 flipped"
fi
if grep -Eiq 'cpan-upload succeeded|uploaded the TRIAL to PAUSE' "$NOTES"; then
  fail "notes must not claim PAUSE uploaded"
fi
ok "real GA-candidate notes have flavor honesty and residuals"

NATIVE=""
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  NATIVE="${NYTPROF_NATIVE_CLI}"
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  NATIVE="$ROOT/prefix/bin/nytprof-cli"
elif command -v nytprof-cli >/dev/null 2>&1; then
  NATIVE="$(command -v nytprof-cli)"
fi

if [[ -n "$NATIVE" ]]; then
  echo "running: $NATIVE capability --json"
  set +e
  CAP="$("$NATIVE" capability --json 2>&1)"
  CAPRC=$?
  set -e
  printf '%s\n' "$CAP"
  [[ "$CAPRC" -eq 0 ]] || fail "capability --json failed (rc=$CAPRC)"
  grep -F -q '"collection_default":"v5"' <<<"$CAP" \
    || fail "capability --json missing collection_default v5"
  ok "capability --json collection_default=v5"
else
  echo "SKIP: no nytprof-cli — notes asserts hold"
fi

echo "running shipped G04 attach (D1-B 15/3/15)"
set +e
G04_OUT="$(bash "$G04" 2>&1)"
G04_RC=$?
set -e
printf '%s\n' "$G04_OUT"
[[ "$G04_RC" -eq 0 ]] || fail "g04_v5_parity_smoke.sh failed (rc=$G04_RC)"
if grep -E -q 'SKIP: no C toolchain|SKIP: perl XS headers' <<<"$G04_OUT"; then
  ok "G04 honest skip (no CC/XS) — notes asserts hold"
else
  grep -E -q 'leaf_returns=15' <<<"$G04_OUT" \
    || fail "G04 did not report leaf_returns=15 from produced bytes"
  grep -E -q 'mid_returns=3' <<<"$G04_OUT" \
    || fail "G04 did not report mid_returns=3"
  grep -E -q 'mid_leaf_edge=15' <<<"$G04_OUT" \
    || fail "G04 did not report mid_leaf_edge=15"
  ok "D1-B live attach 15/3/15 via shipped G04"
fi

echo "NOT-YET: SEC-012 complete GA marketing / independent sign-off"
echo "NOT-YET: BUILD-003-FULL / PRODUCT-V6-COLLECT-EL8 / S2 / R3-R4 flip"
ok "P01-GA-CANDIDATE"
exit 0

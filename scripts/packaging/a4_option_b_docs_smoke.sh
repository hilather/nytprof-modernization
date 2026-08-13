#!/usr/bin/env bash
# A4 / RPM-08 — Option B operator-doc identity.
#
# Greps the real operator-facing files (MIG01, BUILD S0–S3, board EL8-RPM-MODULE
# rows, graft annex C, PRODUCT_COMPLETION banner, ADR-0010 Recommends, R1 live
# status pointers, EL8 module/tools schemas). Fails if leftover product-path
# recipes still teach `-d:NYTProf`, `perl-Devel-NYTProf` ≥ 7.00, or
# `perl-Devel-NYTProf.spec` as the shipped **product** names.
#
# Oracle / stock / history mentions of `-d:NYTProf` are allowed (P-ORACLE,
# “operators switch from stock”). Frozen rev-4 KD-16/17 body in
# PRODUCT_COMPLETION_DROP_IN_v0.md is historical and is not rewritten.
#
# Does not rewrite dual_path. Does not claim S2 / BUILD-003-FULL / PAUSE.
# Never puts crates/ on oracle PERL5LIB. Cargo-free.
#
# Exit 0: A4 pass. Exit 1: leftover product recipe. Exit 2: misuse / crates/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

MIG="$ROOT/docs/MIGRATION_DROP_IN_v0.md"
BUILD="$ROOT/docs/BUILD_SUPPORT_POLICY.md"
BOARD="$ROOT/docs/FIRST_SLICE_BOARD.md"
ANNEX="$ROOT/docs/schemas/product-xs-graft-annex-v0.md"
COMPLETION="$ROOT/docs/PRODUCT_COMPLETION_DROP_IN_v0.md"
ADR="$ROOT/docs/adrs/0010-signed-ci-prebuilt-native-cli.md"
RUNBOOK="$ROOT/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md"
MOD_SCHEMA="$ROOT/docs/schemas/el8-module-rpm-mvp-v0.md"
TOOLS_SCHEMA="$ROOT/docs/schemas/el8-tools-rpm-mvp-v0.md"
MODULE_SPEC="$ROOT/packaging/rpm/perl-NYTProfM.spec"

usage() {
  cat <<'EOF'
Usage: a4_option_b_docs_smoke.sh

A4: real operator docs teach Option B (NYTProfM / -d:NYTProfM / perl-NYTProfM
6.15). Fails on leftover product-path recipes. Frozen rev-4 KDs stay historical.
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

echo "a4_option_b_docs_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; cargo is not required"
echo "frozen rev-4 KD-16/17 body is historical; Option B is the shipped name"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

for f in "$MIG" "$BUILD" "$BOARD" "$ANNEX" "$COMPLETION" "$ADR" "$RUNBOOK" \
         "$MOD_SCHEMA" "$TOOLS_SCHEMA" "$MODULE_SPEC"; do
  [[ -f "$f" ]] || fail "missing $f"
done

# Leftover *product install recipes* (not stock/oracle mentions).
leftover_recipes() {
  local file="$1"
  local label="$2"
  if grep -E -q 'cpanm[[:space:]]+Devel::NYTProf([^M]|$)' "$file"; then
    fail "$label: leftover product recipe 'cpanm Devel::NYTProf' (want NYTProfM / Devel::NYTProfM)"
  fi
  if grep -E -q 'dnf[[:space:]]+install[[:space:]]+perl-Devel-NYTProf([^M]|$)' "$file"; then
    fail "$label: leftover product recipe 'dnf install perl-Devel-NYTProf' (want perl-NYTProfM)"
  fi
  if grep -E -q 'dnf[[:space:]]+downgrade[[:space:]]+perl-Devel-NYTProf' "$file"; then
    fail "$label: leftover pre-Option-B rollback 'dnf downgrade perl-Devel-NYTProf' (want dnf remove perl-NYTProfM)"
  fi
  if grep -E -q 'perl[[:space:]]+-d:NYTProf[[:space:]]+your_script' "$file"; then
    fail "$label: leftover product example 'perl -d:NYTProf your_script' (want -d:NYTProfM)"
  fi
}

# --- MIG01 operator guide ---
leftover_recipes "$MIG" "MIG01"
grep -F -q 'perl-NYTProfM' "$MIG" \
  || fail "MIG01 missing perl-NYTProfM"
grep -F -q -- '-d:NYTProfM' "$MIG" \
  || fail "MIG01 missing -d:NYTProfM"
grep -F -q 'dnf install perl-NYTProfM' "$MIG" \
  || fail "MIG01 missing 'dnf install perl-NYTProfM'"
grep -E -q 'cpanm[[:space:]]+(NYTProfM|Devel::NYTProfM)' "$MIG" \
  || fail "MIG01 missing cpanm NYTProfM / Devel::NYTProfM"
if grep -E -q '≥[[:space:]]*7\.00|>=[[:space:]]*7\.00' "$MIG"; then
  fail "MIG01 still teaches product ≥ 7.00 (Option B is 6.15, parallel to stock)"
fi
if grep -Eiq 'product keeps the \*\*same RPM name\*\*|product keeps the same RPM name' "$MIG"; then
  fail "MIG01 still teaches same-RPM-name EVR upgrade (Option B is parallel perl-NYTProfM)"
fi
# Forbidding stock Provides is allowed; teaching it as the product recipe is not
# (covers both `Provides:` and a table-cell value without the colon).
if grep -E 'perl\(Devel::NYTProf\)[^M]' "$MIG" \
    | grep -Eiv 'do[[:space:]]+\*\*not\*\*|do not|must not|not Provides|forbids|No Provides'; then
  fail "MIG01 still teaches Provides perl(Devel::NYTProf) (Option B forbids it)"
fi
grep -Eiq 'parallel|operators switch|switch from stock' "$MIG" \
  || fail "MIG01 must say product is parallel / operators switch (Option B)"
grep -F -q '6.15' "$MIG" \
  || fail "MIG01 missing product \$VERSION 6.15"
ok "MIG01 teaches Option B install / attach / rollback"

# --- BUILD S0–S3 identity (extract the section only; history above may keep old names) ---
S03="$(awk '
  /^## Three isolation profiles \+ smoke migration S0/ {p=1}
  /^## Explicit non-goals/ {p=0}
  p {print}
' "$BUILD")"
[[ -n "$S03" ]] || fail "BUILD_SUPPORT_POLICY.md missing S0–S3 section"
if grep -F -q 'perl-Devel-NYTProf.spec' <<<"$S03"; then
  fail "BUILD S0–S3 still names perl-Devel-NYTProf.spec (want perl-NYTProfM.spec)"
fi
grep -F -q 'perl-NYTProfM.spec' <<<"$S03" \
  || fail "BUILD S0–S3 missing perl-NYTProfM.spec"
if grep -E -q 'Devel::NYTProf([^M]|$).*≥[[:space:]]*7\.00|≥[[:space:]]*7\.00' <<<"$S03"; then
  fail "BUILD S0–S3 still teaches J01 Devel::NYTProf ≥ 7.00 (want NYTProfM 6.15)"
fi
leftover_recipes <(printf '%s\n' "$S03") "BUILD S0–S3"
# Product-path -d:NYTProf leftovers fail; stock / P-ORACLE mentions are allowed.
if grep -E -- '-d:NYTProf([^M]|$)' <<<"$S03" \
    | grep -Eiv 'stock|oracle|P-ORACLE|baseline/6\.15|switch from'; then
  fail "BUILD S0–S3 still teaches product -d:NYTProf (want -d:NYTProfM)"
fi
grep -F -q -- '-d:NYTProfM' <<<"$S03" \
  || fail "BUILD S0–S3 missing -d:NYTProfM"
grep -F -q 'P-ORACLE' <<<"$S03" \
  || fail "BUILD S0–S3 missing P-ORACLE (oracle isolation stays valid)"
grep -E -q 'Do \*\*not\*\* change `dual_path_smoke.sh`|Do not change `dual_path_smoke.sh`' <<<"$S03" \
  || fail "BUILD S0–S3 must keep dual_path rewrite-before-S2 hard rule"
ok "BUILD S0–S3 product identity is -d:NYTProfM / perl-NYTProfM.spec"

# --- Board EL8-RPM-MODULE rows (current spec path, not G03a history) ---
EL8_ROWS="$(grep -E 'EL8-RPM-MODULE' "$BOARD" || true)"
[[ -n "$EL8_ROWS" ]] || fail "FIRST_SLICE_BOARD missing EL8-RPM-MODULE rows"
if grep -F -q 'perl-Devel-NYTProf.spec' <<<"$EL8_ROWS"; then
  fail "board EL8-RPM-MODULE still cites perl-Devel-NYTProf.spec (want perl-NYTProfM.spec)"
fi
if grep -E -q '≥[[:space:]]*7\.00|>=[[:space:]]*7\.00' <<<"$EL8_ROWS"; then
  fail "board EL8-RPM-MODULE still teaches ≥ 7.00 (want 6.15)"
fi
grep -F -q 'perl-NYTProfM.spec' <<<"$EL8_ROWS" \
  || fail "board EL8-RPM-MODULE missing perl-NYTProfM.spec"
ok "board EL8-RPM-MODULE cites perl-NYTProfM.spec / 6.15"

# --- Graft annex C ---
ANNEX_C="$(awk '
  /^## Annex C/ {p=1}
  /^## Related/ {p=0}
  p {print}
' "$ANNEX")"
[[ -n "$ANNEX_C" ]] || fail "graft annex missing Annex C"
leftover_recipes <(printf '%s\n' "$ANNEX_C") "annex C"
if grep -E -q '^Name:[[:space:]]+perl-Devel-NYTProf([^M]|$)' <<<"$ANNEX_C"; then
  fail "annex C still shows product Name: perl-Devel-NYTProf"
fi
if grep -E -q 'Provides:[[:space:]]+perl\(Devel::NYTProf\)[^M]' <<<"$ANNEX_C"; then
  fail "annex C still teaches Provides perl(Devel::NYTProf)"
fi
if grep -E -q '≥[[:space:]]*7\.00|>=[[:space:]]*7\.00' <<<"$ANNEX_C"; then
  fail "annex C still teaches product ≥ 7.00"
fi
grep -F -q 'perl-NYTProfM' <<<"$ANNEX_C" \
  || fail "annex C missing perl-NYTProfM"
grep -Eiq 'do not Provides|must not Provides|not Provides' <<<"$ANNEX_C" \
  || fail "annex C must say product does not Provides stock perl(Devel::NYTProf)"
ok "annex C names perl-NYTProfM / no Provides stock"

# --- PRODUCT_COMPLETION banner only (do not rewrite frozen KD body) ---
grep -Eiq 'identity superseded|superseded by Option B' "$COMPLETION" \
  || fail "PRODUCT_COMPLETION_DROP_IN_v0.md missing Option B superseded banner"
grep -F -q 'DROP_IN_RPM_COMPLETION_v0.md' "$COMPLETION" \
  || fail "PRODUCT_COMPLETION banner must point at DROP_IN_RPM_COMPLETION_v0.md"
# Frozen rev-4 KD body must still be present (not silently rewritten).
grep -E -q 'KD-16|KD-17' "$COMPLETION" \
  || fail "PRODUCT_COMPLETION lost frozen KD-16/17 body"
grep -E -q '≥ 7\.00|>= 7\.00' "$COMPLETION" \
  || fail "PRODUCT_COMPLETION lost frozen rev-4 ≥ 7.00 KD text (must stay historical)"
ok "PRODUCT_COMPLETION has Option B banner; frozen rev-4 KDs remain"

# --- ADR-0010 Recommends + module package name ---
# Do not use grep -q on the left of a pipe (it would starve the consumer).
if grep -Ei 'Recommends:|Suggests:' "$ADR" | grep -E -q 'perl-Devel-NYTProf([^M]|$)'; then
  fail "ADR-0010 still Recommends/Suggests perl-Devel-NYTProf (want perl-NYTProfM)"
fi
if ! grep -Ei 'Recommends:|Suggests:' "$ADR" | grep -E -q 'perl-NYTProfM'; then
  fail "ADR-0010 must Recommends/Suggests perl-NYTProfM"
fi
if awk '/^### 7\. Module RPM/,/^### 8\./' "$ADR" | grep -E -q '`perl-Devel-NYTProf`'; then
  fail "ADR-0010 §7 still names default EL8 module perl-Devel-NYTProf"
fi
ok "ADR-0010 Recommends perl-NYTProfM"

# --- R1 live status pointers (do not rewrite G03a history prose) ---
if grep -E 'EL8-RPM-MODULE' "$RUNBOOK" | grep -F -q 'perl-Devel-NYTProf.spec'; then
  fail "R1 runbook EL8-RPM-MODULE still links perl-Devel-NYTProf.spec"
fi
grep -E 'EL8-RPM-MODULE' "$RUNBOOK" | grep -F -q 'perl-NYTProfM.spec' \
  || fail "R1 runbook EL8-RPM-MODULE missing perl-NYTProfM.spec"
if grep -E 'J01-CPAN-HYGIENE' "$RUNBOOK" | grep -E -q 'Devel-NYTProf`? \*\*7\.00\*\*|Devel-NYTProf \*\*7\.00\*\*|7\.00'; then
  fail "R1 runbook J01 still teaches Devel-NYTProf 7.00 as current identity"
fi
grep -Eiq 'Option B|perl-NYTProfM|-d:NYTProfM' "$RUNBOOK" \
  || fail "R1 runbook missing Option B / MIG01 pointer"
ok "R1 live status pointers are Option B (history prose untouched)"

# --- EL8 schemas (current contract, not history) ---
if grep -F -q 'perl-Devel-NYTProf.spec' "$MOD_SCHEMA"; then
  fail "el8-module-rpm-mvp-v0.md still cites perl-Devel-NYTProf.spec"
fi
if grep -E -q '≥[[:space:]]*7\.00|>=[[:space:]]*7\.00' "$MOD_SCHEMA"; then
  fail "el8-module-rpm-mvp-v0.md still teaches ≥ 7.00"
fi
if grep -E -q 'Provides `perl\(Devel::NYTProf\)`|Provides:[[:space:]]*perl\(Devel::NYTProf\)[^M]' "$MOD_SCHEMA"; then
  fail "el8-module-rpm-mvp-v0.md still teaches Provides stock perl(Devel::NYTProf)"
fi
grep -F -q 'perl-NYTProfM.spec' "$MOD_SCHEMA" \
  || fail "el8-module-rpm-mvp-v0.md missing perl-NYTProfM.spec"
grep -F -q 'perl-NYTProfM' "$TOOLS_SCHEMA" \
  || fail "el8-tools-rpm-mvp-v0.md Recommends must be perl-NYTProfM"
if grep -E -q 'Recommends \| `perl-Devel-NYTProf`' "$TOOLS_SCHEMA"; then
  fail "el8-tools-rpm-mvp-v0.md still Recommends perl-Devel-NYTProf"
fi
ok "EL8 module/tools schemas use perl-NYTProfM"

# --- shipped spec still matches the docs ---
grep -E -q '^Name:[[:space:]]+perl-NYTProfM' "$MODULE_SPEC" \
  || fail "shipped spec Name is not perl-NYTProfM"
grep -E -q '^Version:[[:space:]]+6\.15' "$MODULE_SPEC" \
  || fail "shipped spec Version is not 6.15"
if grep -E '^Provides:' "$MODULE_SPEC" | grep -E -q 'perl\(Devel::NYTProf\)[^M]'; then
  fail "shipped spec must not Provides stock perl(Devel::NYTProf)"
fi
ok "shipped perl-NYTProfM.spec matches Option B docs"

echo "NOT-YET: S2 dual_path primary rewrite"
echo "NOT-YET: BUILD-003-FULL / PAUSE upload / A3 maintainer-mock / A5b COPR"
ok "A4-OPTION-B-DOCS"
exit 0

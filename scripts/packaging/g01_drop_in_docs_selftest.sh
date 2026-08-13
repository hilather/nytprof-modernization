#!/usr/bin/env bash
# G01 drop-in docs + skeleton smoke regression.
#
# Drives the real shipped product_* smoke scripts (no reimplementation).
# Fails if G01 skeletons claim attach works, or if frozen KD strings vanish
# from the binding contracts.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

PACK="$ROOT/scripts/packaging"
ATTACH="$PACK/product_attach_smoke.sh"
LEGACY="$PACK/product_legacy_smoke.sh"
DUAL="$PACK/dual_path_smoke.sh"

DOD="$ROOT/docs/contracts/DROP_IN_DOD_v0.md"
ANNEX="$ROOT/docs/schemas/product-xs-graft-annex-v0.md"
DESIGN="$ROOT/docs/PRODUCT_COMPLETION_DROP_IN_v0.md"
SMOKE_SCHEMA="$ROOT/docs/schemas/product-attach-smoke-mvp-v0.md"
BOARD="$ROOT/docs/FIRST_SLICE_BOARD.md"
MATRIX="$ROOT/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

assert_file() {
  [[ -f "$1" ]] || fail "missing required file: $1"
}

assert_contains() {
  local haystack="$1"
  local needle="$2"
  local label="$3"
  if ! grep -F -q -- "$needle" <<<"$haystack"; then
    fail "$label: expected to contain $(printf %q "$needle")"
  fi
}

assert_not_contains() {
  local haystack="$1"
  local needle="$2"
  local label="$3"
  if grep -F -q -- "$needle" <<<"$haystack"; then
    fail "$label: must not contain ready-claim $(printf %q "$needle")"
  fi
}

assert_skip_or_notyet() {
  local haystack="$1"
  local label="$2"
  if grep -E -q 'SKIP:|NOT-YET:' <<<"$haystack"; then
    return 0
  fi
  fail "$label: expected SKIP: or NOT-YET: in output"
}

run_smoke() {
  local script="$1"
  shift
  local out rc
  set +e
  out="$(bash "$script" "$@" 2>&1)"
  rc=$?
  set -e
  printf '%s\n' "$out"
  return "$rc"
}

assert_file "$ATTACH"
assert_file "$LEGACY"
assert_file "$DUAL"
assert_file "$DOD"
assert_file "$ANNEX"
assert_file "$DESIGN"
assert_file "$SMOKE_SCHEMA"
assert_file "$BOARD"
assert_file "$MATRIX"
[[ -x "$ATTACH" ]] || fail "not executable: $ATTACH"
[[ -x "$LEGACY" ]] || fail "not executable: $LEGACY"

# --- dual_path still requires legacy_only as first half (static; do not rewrite) ---
grep -q 'legacy_only_smoke.sh' "$DUAL" \
  || fail "dual_path_smoke.sh must still name legacy_only_smoke.sh as required first half"
if ! grep -E -q 'Always run legacy_only_smoke|legacy-only \(required\)|LEGACY=.*legacy_only_smoke' "$DUAL"; then
  fail "dual_path_smoke.sh must still treat legacy_only_smoke.sh as the required first half"
fi
ok "dual_path_smoke.sh still requires legacy_only_smoke.sh (oracle-primary)"

# --- contract KD strings ---
CONTRACT_BLOB="$(cat "$DOD" "$ANNEX" "$DESIGN" "$SMOKE_SCHEMA")"
for kd in \
  'Devel::NYTProf' \
  'NYTProfM' \
  'Devel::NYTProfM' \
  '6.15' \
  'libnytp_sink_v5' \
  'P-ORACLE' \
  'P-PRODUCT-LEGACY' \
  'P-PRODUCT-DUAL' \
  'S0' \
  'S3' \
  'WAIVE' \
  'CPAN primary' \
  'Rocky' \
  'signed CI' \
  'fail-closed' \
  'FileHandle' \
  'KD-2' \
  'KD-13' \
  'KD-21' \
  'KD-24'
do
  grep -F -q -- "$kd" <<<"$CONTRACT_BLOB" \
    || fail "contract files missing required KD string: $kd"
done
# tablesorter WAIVE is the M01/Q4 freeze
grep -E -q 'tablesorter.*WAIVE|WAIVE.*tablesorter' <<<"$CONTRACT_BLOB" \
  || fail "contract files missing tablesorter WAIVE (M01/Q4)"
# KD-24: full archive must not be the v5-only product link
grep -F -q 'libnytp_sink.a' <<<"$CONTRACT_BLOB" \
  || fail "contract files missing libnytp_sink.a (KD-24 test-only full archive)"
# Clock / discount + 6.15 pin must stay in the annex
grep -E -q 'DISCOUNT|discount' <<<"$CONTRACT_BLOB" \
  || fail "contract files missing clock/discount non-negotiable"
grep -F -q 'baseline/6.15/src' "$ANNEX" \
  || fail "annex missing 6.15 pin path (ADR-0004)"
grep -E -q 'do not.*edit|Do not edit' "$ANNEX" \
  || fail "annex missing 6.15 do-not-edit pin (ADR-0004)"
ok "contract files exist and contain frozen KD strings"

# --- honesty: G04 attach-MVP + G05 options + G06 fork/addpid done; legacy residual ---
HONESTY_BLOB="$(cat "$BOARD" "$MATRIX")"
grep -F -q 'PRODUCT-XS-ATTACH-MVP' <<<"$HONESTY_BLOB" \
  || fail "honesty docs missing PRODUCT-XS-ATTACH-MVP row"
if grep -E 'PRODUCT-XS-ATTACH-MVP' "$BOARD" | grep -E -q 'residual / not-ready'; then
  fail "FIRST_SLICE_BOARD still marks PRODUCT-XS-ATTACH-MVP residual / not-ready after G04"
fi
grep -E 'PRODUCT-XS-ATTACH-MVP' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing PRODUCT-XS-ATTACH-MVP done (MVP)"
grep -E -q 'residual / not-ready|\*\*residual|not-ready|not ready' "$BOARD" \
  || fail "FIRST_SLICE_BOARD missing residual honesty (legacy)"
grep -E -q 'residual / not-ready|\*\*residual' "$MATRIX" \
  || fail "residual matrix missing residual honesty (legacy)"
grep -E 'PRODUCT-OPTIONS-MATRIX' "$BOARD" | grep -E -q 'done \(docs \+ tests\)' \
  || fail "FIRST_SLICE_BOARD missing PRODUCT-OPTIONS-MATRIX done (docs + tests)"
grep -E 'PRODUCT-FORK-ADDPID-MVP' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing PRODUCT-FORK-ADDPID-MVP done (MVP)"
grep -E 'PRODUCT-LEGACY-SMOKE' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing PRODUCT-LEGACY-SMOKE done (MVP)"
grep -E 'I02-MAKEMAKER-NATIVE' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing I02-MAKEMAKER-NATIVE done (MVP)"
grep -E 'I03-DIST-SCRIPTS' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing I03-DIST-SCRIPTS done (MVP)"
grep -E 'J01-CPAN-HYGIENE' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing J01-CPAN-HYGIENE done (MVP)"
grep -E '^\| CPAN-TRIAL-READY ' "$BOARD" | grep -E -q 'done \(notes-ready' \
  || fail "FIRST_SLICE_BOARD missing CPAN-TRIAL-READY done (notes-ready"
if grep -E '^\| CPAN-TRIAL-READY ' "$BOARD" | grep -Eiq 'cpan-upload succeeded|indexed on PAUSE'; then
  fail "FIRST_SLICE_BOARD CPAN-TRIAL-READY must not claim PAUSE uploaded"
fi
grep -E '^\| CPAN-TRIAL-READY ' "$BOARD" | grep -Eiq 'Not PAUSE uploaded|not uploaded' \
  || fail "FIRST_SLICE_BOARD CPAN-TRIAL-READY must say not PAUSE uploaded"
J02_SMOKE="$PACK/j02_cpan_trial_notes_smoke.sh"
assert_file "$J02_SMOKE"
[[ -x "$J02_SMOKE" ]] || fail "not executable: $J02_SMOKE"
assert_file "$ROOT/docs/RELEASE_NOTES_CPAN_TRIAL_v0.md"
grep -F -q 'Devel::NYTProf' "$ROOT/docs/RELEASE_NOTES_CPAN_TRIAL_v0.md" \
  || fail "TRIAL notes missing Devel::NYTProf"
grep -Eiq 'not uploaded to PAUSE' "$ROOT/docs/RELEASE_NOTES_CPAN_TRIAL_v0.md" \
  || fail "TRIAL notes must say not uploaded to PAUSE"
grep -E 'MIG01-MIGRATION-GUIDE' "$BOARD" | grep -E -q 'done \(docs\)' \
  || fail "FIRST_SLICE_BOARD missing MIG01-MIGRATION-GUIDE done (docs)"
grep -E 'K03-PREBUILT-CLI-ADR' "$BOARD" | grep -E -q 'done \(docs\)' \
  || fail "FIRST_SLICE_BOARD missing K03-PREBUILT-CLI-ADR done (docs)"
grep -E 'EL8-RPM-MODULE' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing EL8-RPM-MODULE done (MVP)"
grep -E 'EL8-RPM-TOOLS' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing EL8-RPM-TOOLS done (MVP)"
grep -E 'P01-GA-CANDIDATE' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing P01-GA-CANDIDATE done (MVP)"
P01_SMOKE="$PACK/p01_ga_candidate_smoke.sh"
assert_file "$P01_SMOKE"
[[ -x "$P01_SMOKE" ]] || fail "not executable: $P01_SMOKE"
assert_file "$ROOT/docs/RELEASE_NOTES_GA_CANDIDATE_v0.md"
grep -F -q 'Devel::NYTProf' "$ROOT/docs/RELEASE_NOTES_GA_CANDIDATE_v0.md" \
  || fail "GA-candidate notes missing Devel::NYTProf"
grep -Eiq 'D1-B' "$ROOT/docs/RELEASE_NOTES_GA_CANDIDATE_v0.md" \
  || fail "GA-candidate notes must say Rocky default is D1-B"
if grep -q 'p01_ga_candidate' "$DUAL"; then
  fail "dual_path_smoke.sh must not require P01 (S2 not claimed)"
fi
grep -E 'P02-SEC-CUT' "$BOARD" | grep -E -q 'done \(MVP / checklist / job\)' \
  || fail "FIRST_SLICE_BOARD missing P02-SEC-CUT done (MVP / checklist / job)"
grep -E 'E3-MIXED-RESIDUAL' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing E3-MIXED-RESIDUAL done (MVP)"
grep -E 'E4-01-ORACLE-PAIR-MVP' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing E4-01-ORACLE-PAIR-MVP done (MVP)"
grep -E 'TOOL-CONVERT-LOSSY-MVP' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing TOOL-CONVERT-LOSSY-MVP done (MVP)"
if grep -E 'TOOL-CONVERT-LOSSY-MVP' "$BOARD" | grep -Eiq 'lossy is default|strict is gone|full TEST-008 done'; then
  fail "FIRST_SLICE_BOARD L01 must not claim lossy default / TEST-008 complete"
fi
L01_SMOKE="$PACK/l01_lossy_convert_smoke.sh"
assert_file "$L01_SMOKE"
[[ -x "$L01_SMOKE" ]] || fail "not executable: $L01_SMOKE"
grep -F -q -- '--allow-lossy' "$ROOT/crates/nytprof-cli/src/main.rs" \
  || fail "CLI source missing --allow-lossy"
assert_file "$ROOT/docs/schemas/convert-lossy-mvp-v0.md"
if grep -q 'allow-lossy' "$DUAL"; then
  fail "dual_path_smoke.sh must not require L01 (S2 not claimed)"
fi
grep -E 'TOOL-MERGE-AGGREGATE-SUM-MVP' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing TOOL-MERGE-AGGREGATE-SUM-MVP done (MVP)"
if grep -E 'TOOL-MERGE-AGGREGATE-SUM-MVP' "$BOARD" | grep -Eiq 'full nytprofmerge option parity done|concat removed|S2 dual_path rewrite executed'; then
  fail "FIRST_SLICE_BOARD L02 must not claim full nytprofmerge options / concat removed / S2"
fi
L02_SMOKE="$PACK/l02_aggregate_sum_merge_smoke.sh"
assert_file "$L02_SMOKE"
[[ -x "$L02_SMOKE" ]] || fail "not executable: $L02_SMOKE"
grep -F -q -- '--aggregate-sum' "$ROOT/crates/nytprof-cli/src/main.rs" \
  || fail "CLI source missing --aggregate-sum"
assert_file "$ROOT/docs/schemas/merge-aggregate-sum-mvp-v0.md"
if grep -q 'aggregate-sum' "$DUAL"; then
  fail "dual_path_smoke.sh must not require L02 (S2 not claimed)"
fi
if grep -E 'E4-01-ORACLE-PAIR-MVP' "$BOARD" | grep -Eiq 'full TEST-008 done|format=dual shipped|S2 dual_path rewrite executed'; then
  fail "FIRST_SLICE_BOARD E4-01 must not claim full TEST-008 / format=dual / S2"
fi
assert_file "$ROOT/fixtures/e4/oracle-pair/default_calls1_v5.nytprof"
assert_file "$ROOT/fixtures/e4/oracle-pair/default_calls1_v6.nytprof"
h5="$(head -c 12 "$ROOT/fixtures/e4/oracle-pair/default_calls1_v5.nytprof" | tr -d '\0' || true)"
h6="$(head -c 8 "$ROOT/fixtures/e4/oracle-pair/default_calls1_v6.nytprof" | tr -d '\0' || true)"
[[ "$h5" == "NYTProf 5"* ]] || fail "E4-01 v5 missing NYTProf 5 magic"
[[ "$h6" == "NYTPROF6" ]] || fail "E4-01 v6 missing NYTPROF6 magic"
grep -F -q 'oracle-pair' "$ROOT/scripts/packaging/e4_v5_v6_semantic_smoke.sh" \
  || fail "e4_v5_v6_semantic_smoke.sh must name oracle-pair"
grep -E 'E4-02-ORACLE-PAIR-MVP' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing E4-02-ORACLE-PAIR-MVP done (MVP)"
if grep -E 'E4-02-ORACLE-PAIR-MVP' "$BOARD" | grep -Eiq 'full TEST-008 done|format=dual shipped|opcode/entersub shipped|A4 780 live attach|S2 dual_path rewrite executed'; then
  fail "FIRST_SLICE_BOARD E4-02 must not claim full TEST-008 / format=dual / opcode / 780 attach / S2"
fi
assert_file "$ROOT/fixtures/e4/oracle-pair/blocks_calls1_v5.nytprof"
assert_file "$ROOT/fixtures/e4/oracle-pair/blocks_calls1_v6.nytprof"
hb5="$(head -c 12 "$ROOT/fixtures/e4/oracle-pair/blocks_calls1_v5.nytprof" | tr -d '\0' || true)"
hb6="$(head -c 8 "$ROOT/fixtures/e4/oracle-pair/blocks_calls1_v6.nytprof" | tr -d '\0' || true)"
[[ "$hb5" == "NYTProf 5"* ]] || fail "E4-02 v5 missing NYTProf 5 magic"
[[ "$hb6" == "NYTPROF6" ]] || fail "E4-02 v6 missing NYTPROF6 magic"
grep -F -q 'blocks_calls1' "$ROOT/scripts/packaging/e4_v5_v6_semantic_smoke.sh" \
  || fail "e4_v5_v6_semantic_smoke.sh must name blocks_calls1"
if grep -F -q -- '--allow-lossy' "$ROOT/scripts/packaging/e4_v5_v6_semantic_smoke.sh"; then
  fail "e4_v5_v6_semantic_smoke.sh must not pass --allow-lossy"
fi
grep -E 'E4-03-ORACLE-PAIR-MVP' "$BOARD" | grep -E -q 'done \(MVP\)' \
  || fail "FIRST_SLICE_BOARD missing E4-03-ORACLE-PAIR-MVP done (MVP)"
if grep -E 'E4-03-ORACLE-PAIR-MVP' "$BOARD" | grep -Eiq 'full TEST-008 done|format=dual shipped|SUB_ENTRY 27 live attach|S2 dual_path rewrite executed'; then
  fail "FIRST_SLICE_BOARD E4-03 must not claim full TEST-008 / format=dual / SUB_ENTRY 27 attach / S2"
fi
assert_file "$ROOT/fixtures/e4/oracle-pair/calls2_default_v5.nytprof"
assert_file "$ROOT/fixtures/e4/oracle-pair/calls2_default_v6.nytprof"
hc5="$(head -c 12 "$ROOT/fixtures/e4/oracle-pair/calls2_default_v5.nytprof" | tr -d '\0' || true)"
hc6="$(head -c 8 "$ROOT/fixtures/e4/oracle-pair/calls2_default_v6.nytprof" | tr -d '\0' || true)"
[[ "$hc5" == "NYTProf 5"* ]] || fail "E4-03 v5 missing NYTProf 5 magic"
[[ "$hc6" == "NYTPROF6" ]] || fail "E4-03 v6 missing NYTPROF6 magic"
grep -F -q 'calls2_default' "$ROOT/scripts/packaging/e4_v5_v6_semantic_smoke.sh" \
  || fail "e4_v5_v6_semantic_smoke.sh must name calls2_default"
if grep -E 'E3-MIXED-RESIDUAL' "$BOARD" | grep -Eiq 'TEST-008 done|COL-008 baseline|collection_default: v6|S2 dual_path rewrite executed'; then
  fail "FIRST_SLICE_BOARD E3-MIXED-RESIDUAL must not claim TEST-008 / COL-008 / v6 collection default / S2"
fi
assert_file "$ROOT/fixtures/v6/from-c/mixed.nytprof"
head_c="$(head -c 8 "$ROOT/fixtures/v6/from-c/mixed.nytprof" | tr -d '\0' || true)"
[[ "$head_c" == "NYTPROF6" ]] || fail "mixed.nytprof missing NYTPROF6 magic"
grep -F -q 'e3_decode_mixed_writer_bytes' "$ROOT/tools/oracle/e3_c_writer_parity.sh" \
  || grep -F -q 'e3_c_mixed' "$ROOT/tools/oracle/e3_c_writer_parity.sh" \
  || fail "e3_c_writer_parity.sh must name e3_c_mixed / mixed decode"
if grep -E 'P02-SEC-CUT' "$BOARD" | grep -Eiq 'independent sign-off is complete|GA marketing complete|SEC-012 complete GA'; then
  fail "FIRST_SLICE_BOARD P02-SEC-CUT must not claim independent sign-off or GA marketing"
fi
grep -E 'P02-SEC-CUT' "$BOARD" | grep -Eiq 'Not.*independent sign-off|not independent sign-off' \
  || fail "FIRST_SLICE_BOARD P02-SEC-CUT must deny independent sign-off"
P02_SMOKE="$PACK/p02_sec_cut_smoke.sh"
assert_file "$P02_SMOKE"
[[ -x "$P02_SMOKE" ]] || fail "not executable: $P02_SMOKE"
assert_file "$ROOT/docs/contracts/SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md"
grep -Eiq 'not([[:space:]]|\*\*)+independent[[:space:]]+sign-off' "$ROOT/docs/contracts/SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md" \
  || fail "SEC-012 checklist must say not independent sign-off"
grep -Eiq 'not([[:space:]]|\*\*)+GA[[:space:]]+marketing' "$ROOT/docs/contracts/SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md" \
  || fail "SEC-012 checklist must say not GA marketing"
assert_file "$ROOT/scripts/ci/sec002_continuous_fuzz_mvp.sh"
assert_file "$ROOT/.github/workflows/sec002-fuzz-mvp.yml"
grep -F -q 'selftest_security_fuzz.sh' "$ROOT/scripts/ci/sec002_continuous_fuzz_mvp.sh" \
  || fail "SEC-002 wrapper must invoke selftest_security_fuzz.sh"
grep -F -q 'selftest_security_fuzz.sh' "$ROOT/.github/workflows/sec002-fuzz-mvp.yml" \
  || fail "SEC-002 workflow must name selftest_security_fuzz.sh"
if grep -q 'p02_sec_cut' "$DUAL"; then
  fail "dual_path_smoke.sh must not require P02 (S2 not claimed)"
fi
K02_SMOKE="$PACK/k02_el8_tools_rpm_smoke.sh"
assert_file "$K02_SMOKE"
[[ -x "$K02_SMOKE" ]] || fail "not executable: $K02_SMOKE"
assert_file "$ROOT/packaging/rpm/nytprof-cli.spec"
grep -E -q '^Name:[[:space:]]+nytprof-cli' "$ROOT/packaging/rpm/nytprof-cli.spec" \
  || fail "K02 spec Name is not nytprof-cli"
grep -Eiq 'Recommends:[[:space:]]*perl-NYTProfM' "$ROOT/packaging/rpm/nytprof-cli.spec" \
  || fail "K02 spec must Recommends perl-NYTProfM"
if grep -q 'k02_el8_tools_rpm' "$DUAL"; then
  fail "dual_path_smoke.sh must not require K02 (S2 not claimed)"
fi
K01_SMOKE="$PACK/k01_el8_module_rpm_smoke.sh"
assert_file "$K01_SMOKE"
[[ -x "$K01_SMOKE" ]] || fail "not executable: $K01_SMOKE"
assert_file "$ROOT/packaging/rpm/perl-NYTProfM.spec"
grep -E -q '^Name:[[:space:]]+perl-NYTProfM' "$ROOT/packaging/rpm/perl-NYTProfM.spec" \
  || fail "K01 spec Name is not perl-NYTProfM"
if grep -q 'k01_el8_module_rpm' "$DUAL"; then
  fail "dual_path_smoke.sh must not require K01 (S2 not claimed)"
fi
grep -E 'M01-HTML-JS-WAIVE' "$BOARD" | grep -E -q 'done \(docs\)' \
  || fail "FIRST_SLICE_BOARD missing M01-HTML-JS-WAIVE done (docs)"
assert_file "$ROOT/docs/MIGRATION_DROP_IN_v0.md"
assert_file "$ROOT/docs/adrs/0010-signed-ci-prebuilt-native-cli.md"
I02_SMOKE="$PACK/i02_makemaker_native_smoke.sh"
assert_file "$I02_SMOKE"
[[ -x "$I02_SMOKE" ]] || fail "not executable: $I02_SMOKE"
I03_SMOKE="$PACK/i03_dist_scripts_smoke.sh"
I03_INSTALL="$PACK/install_product_scripts.sh"
assert_file "$I03_SMOKE"
assert_file "$I03_INSTALL"
[[ -x "$I03_SMOKE" ]] || fail "not executable: $I03_SMOKE"
[[ -x "$I03_INSTALL" ]] || fail "not executable: $I03_INSTALL"
J01_SMOKE="$PACK/j01_cpan_hygiene_smoke.sh"
assert_file "$J01_SMOKE"
[[ -x "$J01_SMOKE" ]] || fail "not executable: $J01_SMOKE"
grep -F -q 'Makefile.PL' "$J01_SMOKE" \
  || fail "j01_cpan_hygiene_smoke.sh must drive real Makefile.PL"
grep -F -q 'Devel::NYTProf' "$J01_SMOKE" \
  || fail "j01_cpan_hygiene_smoke.sh must assert Devel::NYTProf"
assert_file "$ROOT/MANIFEST.SKIP"
if grep -q 'i03_dist_scripts_smoke' "$DUAL"; then
  fail "dual_path_smoke.sh must not require I03 (S2 not claimed)"
fi
grep -F -q 'NYTPROF_NATIVE=1' "$I02_SMOKE" \
  || fail "i02_makemaker_native_smoke.sh must drive NYTPROF_NATIVE=1"
grep -F -q 'Makefile.PL' "$I02_SMOKE" \
  || fail "i02_makemaker_native_smoke.sh must drive real Makefile.PL"
G04_SMOKE="$PACK/g04_v5_parity_smoke.sh"
G05_SMOKE="$PACK/g05_options_format_smoke.sh"
G06_SMOKE="$PACK/g06_fork_addpid_smoke.sh"
assert_file "$G04_SMOKE"
assert_file "$G05_SMOKE"
assert_file "$G06_SMOKE"
[[ -x "$G04_SMOKE" ]] || fail "not executable: $G04_SMOKE"
[[ -x "$G05_SMOKE" ]] || fail "not executable: $G05_SMOKE"
[[ -x "$G06_SMOKE" ]] || fail "not executable: $G06_SMOKE"
grep -F -q -- '-d:NYTProfM' "$G04_SMOKE" \
  || fail "g04_v5_parity_smoke.sh must drive real perl -d:NYTProfM"
grep -F -q -- '-d:NYTProfM' "$G05_SMOKE" \
  || fail "g05_options_format_smoke.sh must drive real perl -d:NYTProfM"
grep -F -q 'format=v6' "$G05_SMOKE" \
  || fail "g05_options_format_smoke.sh must exercise format=v6"
grep -E -q 'leaf_returns|returns=15' "$G04_SMOKE" \
  || fail "g04_v5_parity_smoke.sh must assert leaf 15 from produced bytes"
grep -F -q -- '-d:NYTProfM' "$G06_SMOKE" \
  || fail "g06_fork_addpid_smoke.sh must drive real perl -d:NYTProfM"
grep -E -q 'addpid=1|addpid' "$G06_SMOKE" \
  || fail "g06_fork_addpid_smoke.sh must exercise addpid=1"
grep -F -q 'fork' "$G06_SMOKE" \
  || fail "g06_fork_addpid_smoke.sh must drive a real fork"
ok "honesty docs mark attach-MVP + options + fork/addpid done; G04/G05/G06 smokes present"

# --- attach smoke: default flavor B (G03a load; attach still residual) ---
set +e
ATTACH_DEFAULT="$(run_smoke "$ATTACH")"
ATTACH_RC=$?
set -e
[[ "$ATTACH_RC" -eq 0 ]] || fail "product_attach_smoke.sh default exit $ATTACH_RC (want 0)"
assert_skip_or_notyet "$ATTACH_DEFAULT" "product_attach_smoke.sh default"
assert_contains "$ATTACH_DEFAULT" "flavor_stub:" "product_attach_smoke.sh default"
assert_contains "$ATTACH_DEFAULT" "flavor_stub: d1-b" "product_attach_smoke.sh default B"
assert_contains "$ATTACH_DEFAULT" "phase: S0/S1" "product_attach_smoke.sh default"
assert_contains "$ATTACH_DEFAULT" "product_xs_attach: no" "product_attach_smoke.sh default"
assert_contains "$ATTACH_DEFAULT" "g04_v5_parity_smoke.sh" "product_attach_smoke.sh default"
assert_not_contains "$ATTACH_DEFAULT" "OK: attach works" "product_attach_smoke.sh default"
assert_not_contains "$ATTACH_DEFAULT" "product_xs_attach=1" "product_attach_smoke.sh default"
if grep -E -q 'SKIP: no C toolchain|SKIP: perl XS headers' <<<"$ATTACH_DEFAULT"; then
  ok "product_attach_smoke.sh default (flavor B) honest skip (no CC/XS)"
else
  assert_contains "$ATTACH_DEFAULT" "OK: G03a load" "product_attach_smoke.sh default"
  assert_contains "$ATTACH_DEFAULT" "-d:NYTProfM" "product_attach_smoke.sh default"
  ok "product_attach_smoke.sh default (flavor B) G03a load (no file=)"
fi

# --- attach smoke: --flavor=d1-a ---
set +e
ATTACH_A="$(run_smoke "$ATTACH" --flavor=d1-a)"
ATTACH_A_RC=$?
set -e
[[ "$ATTACH_A_RC" -eq 0 ]] || fail "product_attach_smoke.sh --flavor=d1-a exit $ATTACH_A_RC (want 0)"
assert_skip_or_notyet "$ATTACH_A" "product_attach_smoke.sh --flavor=d1-a"
assert_contains "$ATTACH_A" "flavor_stub: d1-a" "product_attach_smoke.sh --flavor=d1-a"
assert_contains "$ATTACH_A" "g04_v5_parity_smoke.sh" "product_attach_smoke.sh --flavor=d1-a"
assert_not_contains "$ATTACH_A" "OK: attach works" "product_attach_smoke.sh --flavor=d1-a"
assert_not_contains "$ATTACH_A" "product_xs_attach=1" "product_attach_smoke.sh --flavor=d1-a"
if grep -E -q 'SKIP: no C toolchain|SKIP: perl XS headers' <<<"$ATTACH_A"; then
  ok "product_attach_smoke.sh --flavor=d1-a honest skip (no CC/XS)"
else
  assert_contains "$ATTACH_A" "OK: G03a load" "product_attach_smoke.sh --flavor=d1-a"
  assert_contains "$ATTACH_A" "-d:NYTProfM" "product_attach_smoke.sh --flavor=d1-a"
  ok "product_attach_smoke.sh --flavor=d1-a G03a load (no file=)"
fi

# --- legacy smoke: default + d1-a (I01 install+attach when CC/XS) ---
assert_file "$PACK/install_product_xs.sh"
[[ -x "$PACK/install_product_xs.sh" ]] || fail "not executable: $PACK/install_product_xs.sh"
grep -q 'xs-nytprof' "$PACK/install_product_xs.sh" \
  || fail "install_product_xs.sh must build xs-nytprof"
grep -F -q 'cargo' "$PACK/install_product_xs.sh" \
  || fail "install_product_xs.sh must mention cargo is not invoked"

set +e
LEGACY_DEFAULT="$(run_smoke "$LEGACY")"
LEGACY_RC=$?
set -e
[[ "$LEGACY_RC" -eq 0 ]] || fail "product_legacy_smoke.sh default exit $LEGACY_RC (want 0)"
assert_contains "$LEGACY_DEFAULT" "flavor_stub: d1-b" "product_legacy_smoke.sh default B"
assert_contains "$LEGACY_DEFAULT" "PRODUCT-LEGACY-SMOKE" "product_legacy_smoke.sh default"
assert_not_contains "$LEGACY_DEFAULT" "OK: attach works" "product_legacy_smoke.sh default"
if grep -E -q 'SKIP: no C toolchain|SKIP: perl XS headers|SKIP: P-PRODUCT-LEGACY' <<<"$LEGACY_DEFAULT"; then
  assert_skip_or_notyet "$LEGACY_DEFAULT" "product_legacy_smoke.sh default skip"
  ok "product_legacy_smoke.sh default (flavor B) honest skip (no CC/XS)"
else
  assert_contains "$LEGACY_DEFAULT" "phase: I01" "product_legacy_smoke.sh default"
  assert_contains "$LEGACY_DEFAULT" "OK: P-PRODUCT-LEGACY install+attach" "product_legacy_smoke.sh default"
  assert_contains "$LEGACY_DEFAULT" "NYTProf 5" "product_legacy_smoke.sh default"
  assert_contains "$LEGACY_DEFAULT" "leaf_returns=15" "product_legacy_smoke.sh default"
  assert_contains "$LEGACY_DEFAULT" "-d:NYTProfM" "product_legacy_smoke.sh default"
  assert_not_contains "$LEGACY_DEFAULT" "NOT-YET: P-PRODUCT-LEGACY" "product_legacy_smoke.sh default"
  if grep -E '^INC=' <<<"$LEGACY_DEFAULT" | grep -F -q 'baseline/6.15/install'; then
    fail "product_legacy_smoke.sh default INC= must not be the 6.15 oracle pin"
  fi
  ok "product_legacy_smoke.sh default (flavor B) I01 install+attach"
fi

set +e
LEGACY_A="$(run_smoke "$LEGACY" --flavor=d1-a)"
LEGACY_A_RC=$?
set -e
[[ "$LEGACY_A_RC" -eq 0 ]] || fail "product_legacy_smoke.sh --flavor=d1-a exit $LEGACY_A_RC (want 0)"
assert_contains "$LEGACY_A" "flavor_stub: d1-a" "product_legacy_smoke.sh --flavor=d1-a"
assert_not_contains "$LEGACY_A" "OK: attach works" "product_legacy_smoke.sh --flavor=d1-a"
if grep -E -q 'SKIP: no C toolchain|SKIP: perl XS headers|SKIP: P-PRODUCT-LEGACY' <<<"$LEGACY_A"; then
  assert_skip_or_notyet "$LEGACY_A" "product_legacy_smoke.sh --flavor=d1-a skip"
  ok "product_legacy_smoke.sh --flavor=d1-a honest skip (no CC/XS)"
else
  assert_contains "$LEGACY_A" "OK: P-PRODUCT-LEGACY install+attach" "product_legacy_smoke.sh --flavor=d1-a"
  ok "product_legacy_smoke.sh --flavor=d1-a I01 install+attach"
fi

# --- wrapper misuse fail-closed (not missing XS) ---
set +e
ATTACH_BAD="$(bash "$ATTACH" --not-a-real-flag 2>&1)"
ATTACH_BAD_RC=$?
set -e
[[ "$ATTACH_BAD_RC" -eq 2 ]] || fail "product_attach_smoke.sh unknown flag exit $ATTACH_BAD_RC (want 2)"
ok "product_attach_smoke.sh unknown flag fail-closed (exit 2)"

set +e
LEGACY_BAD="$(bash "$LEGACY" --not-a-real-flag 2>&1)"
LEGACY_BAD_RC=$?
set -e
[[ "$LEGACY_BAD_RC" -eq 2 ]] || fail "product_legacy_smoke.sh unknown flag exit $LEGACY_BAD_RC (want 2)"
ok "product_legacy_smoke.sh unknown flag fail-closed (exit 2)"

ok "g01_drop_in_docs_selftest passed"
exit 0

#!/usr/bin/env bash
# Thin fail-fast roll-up of pure-Perl JsonlData packaging smokes.
#
# Used by CI-OFFLINE-GATE-EXPAND (scripts/ci/offline_gate.sh step 5).
# Spec: docs/schemas/perl-jsonl-data-mvp-v0.md
#
# Order (each child owns isolation; never puts crates/ on oracle PERL5LIB):
#   1. perl_jsonl_data_smoke.sh      — returns / edges (default-calls1)
#   2. perl_line_totals_smoke.sh     — line_calls A4 (blocks-calls1)
#   3. perl_subdefs_smoke.sh         — sub_defs + files A9
#   4. perl_source_smoke.sh          — source_lines A8
#   5. perl_a4b_smoke.sh             — block_line_totals A4b
#   6. perl_meta_smoke.sh            — ATTRIBUTE / OPTION metadata
#   7. perl_pid_smoke.sh             — PID_START / PID_END process lifecycle
#   8. perl_stream_complete_smoke.sh — COMPAT-010 stream completeness
#   9. perl_discount_smoke.sh        — DISCOUNT A3 event multiplicity
#  10. perl_sub_entry_smoke.sh       — SUB_ENTRY event multiplicity
#
# Does NOT re-run packaging_gate / dual_path / engine smokes.
# Does NOT require oracle Devel::NYTProf on PERL5LIB.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_jsonl_data_all_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

PACK="$ROOT/scripts/packaging"

banner() {
  echo
  echo "----------------------------------------------------------------"
  echo " JSONL DATA ALL: $*"
  echo "----------------------------------------------------------------"
}

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

run_required() {
  local name="$1"
  local path="$2"
  banner "$name"
  if [[ ! -f "$path" ]]; then
    fail "required script missing: $path"
  fi
  bash "$path"
  ok "step: $name"
}

echo "perl_jsonl_data_all_smoke: repo root $ROOT"
echo "perl_jsonl_data_all_smoke: fail-fast pure-Perl JsonlData roll-up"
echo "perl_jsonl_data_all_smoke: never puts crates/ on oracle PERL5LIB"

run_required "perl_jsonl_data_smoke (returns/edges)" "$PACK/perl_jsonl_data_smoke.sh"
run_required "perl_line_totals_smoke (A4)" "$PACK/perl_line_totals_smoke.sh"
run_required "perl_subdefs_smoke (A9)" "$PACK/perl_subdefs_smoke.sh"
run_required "perl_source_smoke (A8)" "$PACK/perl_source_smoke.sh"
run_required "perl_a4b_smoke (A4b)" "$PACK/perl_a4b_smoke.sh"
run_required "perl_meta_smoke (ATTRIBUTE/OPTION)" "$PACK/perl_meta_smoke.sh"
run_required "perl_pid_smoke (PID_START/PID_END)" "$PACK/perl_pid_smoke.sh"
run_required "perl_stream_complete_smoke (COMPAT-010)" "$PACK/perl_stream_complete_smoke.sh"
run_required "perl_discount_smoke (A3 DISCOUNT)" "$PACK/perl_discount_smoke.sh"
run_required "perl_sub_entry_smoke (SUB_ENTRY)" "$PACK/perl_sub_entry_smoke.sh"

banner "ALL PASSED"
ok "perl_jsonl_data_all_smoke completed successfully"
exit 0

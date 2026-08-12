#!/usr/bin/env bash
# CI-OFFLINE-GATE / CI-OFFLINE-GATE-EXPAND / CI-CAPABILITY-GATE /
# CI-QUERY-JSON-GATE — single documented fail-fast gate for critical
# offline R1 checks.
#
# Steps (in order; exit non-zero on first failure):
#   1. cargo test of offline native packages (honest skip if cargo absent)
#   2. tools/oracle/selftest_harness.sh (required)
#   3. scripts/packaging/dual_path_smoke.sh (primary packaging path)
#   4. scripts/packaging/engine_auto_fallback_smoke.sh (ENGINE-AUTO-FALLBACK)
#   5. scripts/packaging/perl_jsonl_data_all_smoke.sh (pure-Perl JsonlData roll-up, incl. stream_complete / discount)
#   6. scripts/packaging/perl_query_json_smoke.sh (QUERY-JSON-MVP / QUERY-JSON-EXPAND;
#      pure-Perl golden --jsonl; no cargo required) — CI-QUERY-JSON-GATE
#   6b. scripts/packaging/json_sub_entry_smoke.sh (JSON-SUB-ENTRY-MVP: sub_entry_events
#       0/27 on both surfaces; pure-Perl golden required; native when available)
#   6c. scripts/packaging/json_blocks_smoke.sh (JSON-BLOCKS-MVP: line_calls_1_5=780 /
#       block_line_calls_1_4=810 on blocks-calls1; pure-Perl golden; optional native)
#   6d. scripts/packaging/json_subdef_source_smoke.sh (JSON-SUBDEF-SOURCE-MVP:
#       sub_def_leaf 1/3–7, sub_def_mid 1/8–12, source_line_1_5 $x++/1 .. 50;
#       pure-Perl golden required; optional native)
#   6e. scripts/packaging/json_meta_files_smoke.sh (JSON-META-FILES-MVP:
#       attribute_ticks_per_sec / option_calls / file_1 on default-calls1;
#       pure-Perl golden required; optional native)
#   6f. scripts/packaging/json_time_block_smoke.sh (JSON-TIME-BLOCK-MVP:
#       time_block_events 0 on default-calls1 / 916 on blocks-calls1;
#       pure-Perl golden required; optional native + golden TIME_BLOCK recount)
#   6g. scripts/packaging/json_file_basename_smoke.sh (JSON-FILE-BASENAME-MVP:
#       file_1_basename equals/contains workload.pl on default-calls1;
#       pure-Perl golden required; optional native)
#   6h. scripts/packaging/json_event_counts_smoke.sh (JSON-EVENT-COUNTS-MVP:
#       sub_return/new_fid/sub_callers/src_line/sub_info 27/3/13/632/31 on
#       default-calls1; pure-Perl golden required; optional native + golden recount)
#   6i. scripts/packaging/json_total_basetime_smoke.sh (JSON-TOTAL-EVENTS-MVP /
#       JSON-ATTR-BASETIME-MVP: total_events 2474 + attribute_basetime on
#       default-calls1; pure-Perl golden required; optional native)
#   7. scripts/packaging/native_agg_json_smoke.sh when native available (optional)
#      + scripts/packaging/json_native_stream_smoke.sh (JSON-NATIVE-STREAM-MVP)
#      + scripts/packaging/json_report_incomplete_smoke.sh
#        (JSON-REPORT-INCOMPLETE-FAILCLOSED; COMPAT-010 report --json)
#   8. scripts/packaging/native_query_json_cross_smoke.sh when native available
#      (NATIVE-QUERY-JSON-CROSS / CROSS-EXPAND / CROSS-BLOCKS / CROSS-META /
#       CROSS-TIMEBLOCK / CROSS-COUNTS / CROSS-TOTAL: total_events 2474 +
#       attribute_basetime + event counts 27/3/13/632/31 + basename)
#       CROSS-TIMEBLOCK: native report --json vs Perl query --json)
#   9. scripts/packaging/capability_selftest_smoke.sh when cargo or prefix/target
#      native CLI exists (honest skip when native unavailable; same pattern as
#      packaging_gate). dual_path with cargo usually installs prefix/bin first.
#  10. scripts/packaging/collector_sink_smoke.sh (COL-001..007 scaffold:
#      isolation asserts always; make -C collector test when CC present;
#      honest skip without C toolchain). Real v5 wire + absolute/packing v6.
#  11. tools/oracle/e3_c_writer_parity.sh when cargo present (COL-007 product
#      E3-EVENT with C fixtures under fixtures/v6/from-c/**; honest skip if no
#      cargo). E3-mixed residual.
#  12. scripts/packaging/e4_v5_v6_semantic_smoke.sh --full when native CLI
#      available (E4 product CLI smoke on C dual-sink pairs under
#      fixtures/e4/dual-sink/**; honest skip when native unavailable). Full
#      oracle dual residual (TEST-003/TEST-008).
#
# Primary packaging choice: dual_path_smoke.sh (BUILD dual-path policy entry).
# Alternatives not re-run here (document only):
#   ./scripts/packaging/packaging_gate.sh          # broader packaging suite
#   ./scripts/packaging/makemaker_dual_path_smoke.sh  # MakeMaker facade
#
# Non-goals: multi-OS CI matrix (BUILD-006), full packaging_gate breadth.
# Isolation: never puts crates/ or collector/ install on oracle PERL5LIB
# (parent does not source oracle env; child smokes own isolation).
#
# Policy: docs/BUILD_SUPPORT_POLICY.md
# Board:  CI-OFFLINE-GATE / CI-OFFLINE-GATE-EXPAND / CI-CAPABILITY-GATE /
#         CI-QUERY-JSON-GATE / NATIVE-QUERY-JSON-CROSS / COL-001-SINK-MVP
#         (docs/FIRST_SLICE_BOARD.md)
#
# Usage (from repo root or any cwd):
#   ./scripts/ci/offline_gate.sh
# Optional Make target (after perl Makefile.PL):
#   make offline-gate
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

HARNESS="$ROOT/tools/oracle/selftest_harness.sh"
PACKAGING="$ROOT/scripts/packaging/dual_path_smoke.sh"
ENGINE_AUTO_FALLBACK="$ROOT/scripts/packaging/engine_auto_fallback_smoke.sh"
JSONL_DATA_ALL="$ROOT/scripts/packaging/perl_jsonl_data_all_smoke.sh"
QUERY_JSON_SMOKE="$ROOT/scripts/packaging/perl_query_json_smoke.sh"
JSON_SUB_ENTRY_SMOKE="$ROOT/scripts/packaging/json_sub_entry_smoke.sh"
JSON_BLOCKS_SMOKE="$ROOT/scripts/packaging/json_blocks_smoke.sh"
JSON_SUBDEF_SOURCE_SMOKE="$ROOT/scripts/packaging/json_subdef_source_smoke.sh"
JSON_META_FILES_SMOKE="$ROOT/scripts/packaging/json_meta_files_smoke.sh"
JSON_TIME_BLOCK_SMOKE="$ROOT/scripts/packaging/json_time_block_smoke.sh"
JSON_FILE_BASENAME_SMOKE="$ROOT/scripts/packaging/json_file_basename_smoke.sh"
JSON_EVENT_COUNTS_SMOKE="$ROOT/scripts/packaging/json_event_counts_smoke.sh"
JSON_TOTAL_BASETIME_SMOKE="$ROOT/scripts/packaging/json_total_basetime_smoke.sh"
CAPABILITY_SMOKE="$ROOT/scripts/packaging/capability_selftest_smoke.sh"
COLLECTOR_SINK_SMOKE="$ROOT/scripts/packaging/collector_sink_smoke.sh"
E3_C_WRITER_PARITY="$ROOT/tools/oracle/e3_c_writer_parity.sh"
E4_V5_V6_SMOKE="$ROOT/scripts/packaging/e4_v5_v6_semantic_smoke.sh"

banner() {
  echo
  echo "================================================================"
  echo " OFFLINE GATE: $*"
  echo "================================================================"
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

# True when native CLI can be exercised (cargo or prefix/target / NYTPROF_NATIVE_CLI).
# Same condition as packaging_gate capability step (CI-CAPABILITY-GATE).
native_cli_available() {
  command -v cargo >/dev/null 2>&1 \
    || [[ -x "$ROOT/prefix/bin/nytprof-cli" ]] \
    || [[ -x "$ROOT/prefix/bin/nytprof-dump" ]] \
    || [[ -x "$ROOT/target/debug/nytprof-dump" ]] \
    || [[ -x "$ROOT/target/release/nytprof-dump" ]] \
    || [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]
}

echo "offline_gate: repo root $ROOT"
echo "offline_gate: fail-fast; never puts crates/ on oracle PERL5LIB"
echo "offline_gate: board CI-OFFLINE-GATE-EXPAND / CI-CAPABILITY-GATE / CI-QUERY-JSON-GATE / NATIVE-QUERY-JSON-CROSS; policy docs/BUILD_SUPPORT_POLICY.md"

# ---------------------------------------------------------------------------
# 1. Focused cargo tests for offline native packages (skip if cargo absent)
# ---------------------------------------------------------------------------
banner "cargo test (offline packages)"
if ! command -v cargo >/dev/null 2>&1; then
  echo "SKIP: cargo not on PATH — offline native package tests not run"
  echo "  (honest skip; harness + packaging still required)"
  echo "  To exercise cargo: install rustc/cargo, then re-run this gate"
elif [[ ! -f "$ROOT/Cargo.toml" ]] || [[ ! -d "$ROOT/crates" ]]; then
  echo "SKIP: crates/ Cargo workspace not present — offline native package tests not run"
  echo "  (honest skip; harness + packaging still required)"
else
  ok "cargo available: $(cargo --version 2>/dev/null || echo unknown)"
  # Focused packages used by offline native tools; does not touch oracle PERL5LIB.
  cargo test \
    -p nytprof-format-v5 \
    -p nytprof-format-v6 \
    -p nytprof-model \
    -p nytprof-report \
    -p nytprof-cli
  ok "step: cargo test (nytprof-format-v5 format-v6 model report cli)"
fi

# ---------------------------------------------------------------------------
# 2. Oracle differential harness (required)
# ---------------------------------------------------------------------------
run_required "selftest_harness" "$HARNESS"

# ---------------------------------------------------------------------------
# 3. Primary packaging path: dual-path smoke (legacy required; native if cargo)
# ---------------------------------------------------------------------------
run_required "dual_path_smoke (primary packaging)" "$PACKAGING"

# ---------------------------------------------------------------------------
# 4. ENGINE-AUTO-FALLBACK: auto prefer-native / fall-back-legacy (required)
# ---------------------------------------------------------------------------
run_required "engine_auto_fallback_smoke (ENGINE-AUTO-FALLBACK)" "$ENGINE_AUTO_FALLBACK"

# ---------------------------------------------------------------------------
# 5. Pure-Perl JsonlData roll-up (returns/edges/line_totals/subdefs/source/a4b/meta/pid/stream_complete/discount)
# ---------------------------------------------------------------------------
run_required "perl_jsonl_data_all_smoke (JsonlData pure-Perl)" "$JSONL_DATA_ALL"

# ---------------------------------------------------------------------------
# 6. QUERY-JSON-MVP / QUERY-JSON-EXPAND: structured query --json via pure-Perl
#    golden --jsonl (CI-QUERY-JSON-GATE). Required; no cargo. Fail-fast if script missing.
# ---------------------------------------------------------------------------
run_required "perl_query_json_smoke (QUERY-JSON-MVP / QUERY-JSON-EXPAND / CI-QUERY-JSON-GATE)" "$QUERY_JSON_SMOKE"

# ---------------------------------------------------------------------------
# 6b. JSON-SUB-ENTRY-MVP: sub_entry_events 0 (default-calls1) / 27 (calls2-default)
#     on Perl query --json (golden) + native report --json when available.
# ---------------------------------------------------------------------------
run_required "json_sub_entry_smoke (JSON-SUB-ENTRY-MVP)" "$JSON_SUB_ENTRY_SMOKE"

# ---------------------------------------------------------------------------
# 6c. JSON-BLOCKS-MVP: greppable A4/A4b on query --json (blocks-calls1 780/810).
#     Pure-Perl golden required; optional native report --json when CLI present.
# ---------------------------------------------------------------------------
run_required "json_blocks_smoke (JSON-BLOCKS-MVP)" "$JSON_BLOCKS_SMOKE"

# ---------------------------------------------------------------------------
# 6d. JSON-SUBDEF-SOURCE-MVP: greppable A9 sub_def samples + A8 source_line_1_5
#     on query --json (default-calls1 leaf 1/3–7, mid 1/8–12, hot-loop text).
#     Pure-Perl golden required; optional native report --json when CLI present.
# ---------------------------------------------------------------------------
run_required "json_subdef_source_smoke (JSON-SUBDEF-SOURCE-MVP)" "$JSON_SUBDEF_SOURCE_SMOKE"

# ---------------------------------------------------------------------------
# 6e. JSON-META-FILES-MVP: greppable ATTRIBUTE/OPTION/NEW_FID samples on
#     query --json (default-calls1 ticks_per_sec / calls / file_1 workload.pl).
#     Pure-Perl golden required; optional native report --json when CLI present.
# ---------------------------------------------------------------------------
run_required "json_meta_files_smoke (JSON-META-FILES-MVP)" "$JSON_META_FILES_SMOKE"

# ---------------------------------------------------------------------------
# 6f. JSON-TIME-BLOCK-MVP: time_block_events 0 (default-calls1) / 916
#     (blocks-calls1) on query --json; optional native report --json.
# ---------------------------------------------------------------------------
run_required "json_time_block_smoke (JSON-TIME-BLOCK-MVP)" "$JSON_TIME_BLOCK_SMOKE"

# ---------------------------------------------------------------------------
# 6g. JSON-FILE-BASENAME-MVP: file_1_basename equals/contains workload.pl
#     on query --json (default-calls1); pure-Perl golden required; optional native.
# ---------------------------------------------------------------------------
run_required "json_file_basename_smoke (JSON-FILE-BASENAME-MVP)" "$JSON_FILE_BASENAME_SMOKE"

# ---------------------------------------------------------------------------
# 6h. JSON-EVENT-COUNTS-MVP: stream tag multiplicities on query --json
#     (default-calls1 27/3/13/632/31); pure-Perl golden; optional native.
# ---------------------------------------------------------------------------
run_required "json_event_counts_smoke (JSON-EVENT-COUNTS-MVP)" "$JSON_EVENT_COUNTS_SMOKE"

# ---------------------------------------------------------------------------
# 6i. JSON-TOTAL-EVENTS-MVP / JSON-ATTR-BASETIME-MVP: total_events 2474 +
#     attribute_basetime on query --json; pure-Perl golden; optional native.
# ---------------------------------------------------------------------------
run_required "json_total_basetime_smoke (JSON-TOTAL-EVENTS-MVP / JSON-ATTR-BASETIME-MVP)" "$JSON_TOTAL_BASETIME_SMOKE"

# ---------------------------------------------------------------------------
# 7. NATIVE-AGG-JSON + JSON-NATIVE-STREAM-MVP
#    (optional when smoke present + native CLI available)
# ---------------------------------------------------------------------------
NATIVE_AGG_SMOKE="$ROOT/scripts/packaging/native_agg_json_smoke.sh"
STREAM_SMOKE="$ROOT/scripts/packaging/json_native_stream_smoke.sh"
banner "native_agg_json_smoke (NATIVE-AGG-JSON, optional)"
if [[ -f "$NATIVE_AGG_SMOKE" ]] && native_cli_available; then
  bash "$NATIVE_AGG_SMOKE"
  ok "step: native_agg_json_smoke"
elif [[ -f "$NATIVE_AGG_SMOKE" ]]; then
  echo "SKIP: native_agg_json_smoke present but no cargo/prefix/target native CLI"
else
  echo "SKIP: native_agg_json_smoke.sh not present"
fi
banner "json_native_stream_smoke (JSON-NATIVE-STREAM-MVP, optional when native)"
if [[ -f "$STREAM_SMOKE" ]] && native_cli_available; then
  bash "$STREAM_SMOKE"
  ok "step: json_native_stream_smoke"
elif [[ -f "$STREAM_SMOKE" ]]; then
  echo "SKIP: json_native_stream_smoke present but no cargo/prefix/target native CLI"
else
  echo "SKIP: json_native_stream_smoke.sh not present"
fi

JSON_INCOMPLETE_SMOKE="$ROOT/scripts/packaging/json_report_incomplete_smoke.sh"
banner "json_report_incomplete_smoke (JSON-REPORT-INCOMPLETE-FAILCLOSED, optional when native)"
if [[ -f "$JSON_INCOMPLETE_SMOKE" ]] && native_cli_available; then
  bash "$JSON_INCOMPLETE_SMOKE"
  ok "step: json_report_incomplete_smoke"
elif [[ -f "$JSON_INCOMPLETE_SMOKE" ]]; then
  echo "SKIP: json_report_incomplete_smoke present but no cargo/prefix/target native CLI"
else
  echo "SKIP: json_report_incomplete_smoke.sh not present"
fi

# ---------------------------------------------------------------------------
# 8. NATIVE-QUERY-JSON-CROSS: native report --json vs Perl query --json
#    (shared fields 15/3/15 + discount + time_block when both expose).
#    Needs native CLI; pure-Perl query --jsonl alone is covered by step 6.
# ---------------------------------------------------------------------------
CROSS_SMOKE="$ROOT/scripts/packaging/native_query_json_cross_smoke.sh"
banner "native_query_json_cross_smoke (NATIVE-QUERY-JSON-CROSS / CROSS-TOTAL, optional when native)"
if [[ -f "$CROSS_SMOKE" ]] && native_cli_available; then
  bash "$CROSS_SMOKE"
  ok "step: native_query_json_cross_smoke"
elif [[ -f "$CROSS_SMOKE" ]]; then
  echo "SKIP: native_query_json_cross_smoke present but no cargo/prefix/target native CLI"
  echo "  (pure-Perl query --json covered by step 6 perl_query_json_smoke)"
else
  echo "SKIP: native_query_json_cross_smoke.sh not present"
fi

# ---------------------------------------------------------------------------
# 9. CAPABILITY-SELFTEST (+ JSON): run when native CLI can be exercised
#    (CI-CAPABILITY-GATE). dual_path with cargo typically installs prefix/bin
#    so this step usually runs after step 3 on developer hosts with cargo.
# ---------------------------------------------------------------------------
banner "capability_selftest_smoke (CI-CAPABILITY-GATE)"
if [[ ! -f "$CAPABILITY_SMOKE" ]]; then
  fail "required script missing: $CAPABILITY_SMOKE"
fi
if native_cli_available; then
  bash "$CAPABILITY_SMOKE"
  ok "step: capability_selftest_smoke"
else
  echo "SKIP: no cargo/prefix/target native CLI — CAPABILITY-SELFTEST not run"
  echo "  (honest skip; same condition as packaging_gate capability step)"
  echo "  looked for: cargo, prefix/bin/{nytprof-cli,nytprof-dump},"
  echo "  target/{debug,release}/nytprof-dump, \$NYTPROF_NATIVE_CLI"
  echo "  To exercise: install rustc/cargo and re-run, or ./scripts/packaging/install_native.sh"
fi

# ---------------------------------------------------------------------------
# 10. COL-001..007 collector scaffold (honest skip without CC)
# ---------------------------------------------------------------------------
banner "collector_sink_smoke (COL-001..007 scaffold + fake-clock)"
if [[ ! -f "$COLLECTOR_SINK_SMOKE" ]]; then
  fail "required script missing: $COLLECTOR_SINK_SMOKE"
fi
bash "$COLLECTOR_SINK_SMOKE"
ok "step: collector_sink_smoke"

# ---------------------------------------------------------------------------
# 11. COL-007 product E3-EVENT with C fixtures (when cargo present)
# ---------------------------------------------------------------------------
banner "e3_c_writer_parity (COL-007 product E3-EVENT)"
if [[ ! -f "$E3_C_WRITER_PARITY" ]]; then
  fail "required script missing: $E3_C_WRITER_PARITY"
fi
if command -v cargo >/dev/null 2>&1; then
  bash "$E3_C_WRITER_PARITY"
  ok "step: e3_c_writer_parity"
else
  echo "SKIP: cargo not on PATH — product E3 e3_c_* equality not run"
  echo "  (honest skip; committed fixtures under fixtures/v6/from-c/ still required present)"
  # Soft presence check without cargo decode.
  for f in absolute packing packing_lz4 dict packing_dict mid_stream mid_stream_dict; do
    [[ -f "$ROOT/fixtures/v6/from-c/${f}.nytprof" ]] \
      || fail "missing C fixture fixtures/v6/from-c/${f}.nytprof"
  done
  ok "step: e3_c fixture presence (cargo skipped)"
fi

# ---------------------------------------------------------------------------
# 12. E4 product CLI smoke: real CLIs on v5+v6 dual-sink pairs (when native)
#     C dual-sink fixtures are committed under fixtures/e4/dual-sink/**.
# ---------------------------------------------------------------------------
banner "e4_v5_v6_semantic_smoke --full (E4 product CLI; PR-B12b)"
if [[ ! -f "$E4_V5_V6_SMOKE" ]]; then
  fail "required script missing: $E4_V5_V6_SMOKE"
fi
if native_cli_available; then
  # Require committed C dual-sink pairs (native+C product path).
  for stem in m4 default_calls1 blocks_calls1 calls2_default; do
    [[ -f "$ROOT/fixtures/e4/dual-sink/${stem}_v5.nytprof" ]] \
      || fail "missing E4 dual v5 fixtures/e4/dual-sink/${stem}_v5.nytprof"
    [[ -f "$ROOT/fixtures/e4/dual-sink/${stem}_v6.nytprof" ]] \
      || fail "missing E4 dual v6 fixtures/e4/dual-sink/${stem}_v6.nytprof"
  done
  bash "$E4_V5_V6_SMOKE" --full
  ok "step: e4_v5_v6_semantic_smoke --full"
else
  echo "SKIP: no cargo/prefix/target native CLI — E4 product CLI smoke not run"
  echo "  (honest skip; same condition as capability / packaging-native steps)"
  echo "  Dual-sink fixtures still required present:"
  for stem in m4 default_calls1 blocks_calls1 calls2_default; do
    [[ -f "$ROOT/fixtures/e4/dual-sink/${stem}_v5.nytprof" ]] \
      || fail "missing E4 dual v5 fixtures/e4/dual-sink/${stem}_v5.nytprof"
    [[ -f "$ROOT/fixtures/e4/dual-sink/${stem}_v6.nytprof" ]] \
      || fail "missing E4 dual v6 fixtures/e4/dual-sink/${stem}_v6.nytprof"
  done
  ok "step: e4 dual-sink fixture presence (native CLI skipped)"
  echo "  To exercise full E4 product smoke: install rustc/cargo and re-run,"
  echo "  or: ./scripts/packaging/e4_v5_v6_semantic_smoke.sh --full"
fi

banner "ALL PASSED"
ok "offline_gate completed successfully"
echo "NOTE: broader packaging_gate / makemaker_dual_path_smoke are not part of this gate"
echo "NOTE: COL-007 E3-EVENT done with C; E4 product CLI smoke done when native (step 12); residuals: E3-mixed / full oracle dual (TEST-008) / CLI v6 collection default / COL-008"
echo "NOTE: E4 model-only: ./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only"
exit 0

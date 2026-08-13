#!/usr/bin/env bash
# PR-B09 / COL-007 product E3 — C writer bytes ↔ Rust always-inflate decode.
#
# Evidence path for board COL-007 done (EVENT matrix):
#   absolute + packing + FOOTER dict + mid-stream packing continuity
#
# Product E3 loads committed C-produced fixtures under fixtures/v6/from-c/**
# (never Rust stand-in encode). Optional regenerate when CC present:
#   NYTPROF_REGEN_E3_C=1 ./tools/oracle/e3_c_writer_parity.sh
#
# Isolation: never puts crates/ or collector/ on oracle PERL5LIB.
#
# Usage (from repo root or any cwd):
#   ./tools/oracle/e3_c_writer_parity.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="$ROOT/fixtures/v6/from-c"
COLLECTOR="$ROOT/collector"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
banner() { printf '\n=== %s ===\n' "$*"; }
note() { printf 'NOTE: %s\n' "$*"; }

banner "e3_c_writer_parity (COL-007 product E3-EVENT + E3-mixed C bytes)"

# Isolation: never put crates/ or collector/ on oracle PERL5LIB.
assert_no_bad_perl5lib() {
  local label="$1"
  local p5="${2-}"
  case ":${p5}:" in
    *"/crates/"*)
      fail "$label PERL5LIB must not contain /crates/: $p5"
      ;;
    *"/collector"*|*"collector/"*)
      fail "$label PERL5LIB must not contain collector path: $p5"
      ;;
  esac
  ok "$label: no crates/ or collector/ on PERL5LIB"
}
assert_no_bad_perl5lib "parent env" "${PERL5LIB-}"

MATRIX=(
  absolute.nytprof
  packing.nytprof
  packing_lz4.nytprof
  dict.nytprof
  packing_dict.nytprof
  mid_stream.nytprof
  mid_stream_dict.nytprof
)

MIXED=(
  mixed.nytprof
)

# Optional regenerate from C sink (product bytes must stay C-produced).
if [[ "${NYTPROF_REGEN_E3_C:-0}" == "1" ]]; then
  banner "regenerate C fixtures (NYTPROF_REGEN_E3_C=1)"
  command -v cc >/dev/null 2>&1 || fail "cc required to regenerate E3-C fixtures"
  [[ -f "$COLLECTOR/Makefile" ]] || fail "missing collector/Makefile"
  make -C "$COLLECTOR" gen-e3-fixtures OUTDIR="$FIXTURE_DIR"
  ok "regenerated fixtures under $FIXTURE_DIR"
fi

banner "committed C fixture matrix present"
[[ -d "$FIXTURE_DIR" ]] || fail "missing $FIXTURE_DIR (run make -C collector gen-e3-fixtures)"
for f in "${MATRIX[@]}" "${MIXED[@]}"; do
  path="$FIXTURE_DIR/$f"
  [[ -f "$path" ]] || fail "missing C fixture $path"
  [[ -s "$path" ]] || fail "empty C fixture $path"
  # Magic NYTPROF6
  head_c="$(head -c 8 "$path" | tr -d '\0' || true)"
  [[ "$head_c" == "NYTPROF6" ]] || fail "$path: bad magic (not NYTPROF6)"
  ok "fixture $f ($(wc -c <"$path") bytes)"
done

# Engineering dual-path decode examples (when cargo present).
if command -v cargo >/dev/null 2>&1; then
  banner "engineering dual-path decode_c_b08 on C fixtures"
  cargo run -q -p nytprof-format-v6 --example decode_c_b08 -- \
    "$FIXTURE_DIR/absolute.nytprof"
  cargo run -q -p nytprof-format-v6 --example decode_c_b08 -- \
    "$FIXTURE_DIR/packing.nytprof"
  cargo run -q -p nytprof-format-v6 --example decode_c_b08 -- \
    "$FIXTURE_DIR/packing_lz4.nytprof"
  cargo run -q -p nytprof-format-v6 --example decode_c_b08 -- \
    "$FIXTURE_DIR/dict.nytprof" --dict
  cargo run -q -p nytprof-format-v6 --example decode_c_b08 -- \
    "$FIXTURE_DIR/packing_dict.nytprof" --dict
  cargo run -q -p nytprof-format-v6 --example decode_c_b08 -- \
    "$FIXTURE_DIR/mid_stream.nytprof"
  cargo run -q -p nytprof-format-v6 --example decode_c_b08 -- \
    "$FIXTURE_DIR/mid_stream_dict.nytprof" --dict
  ok "decode_c_b08 accepted all C fixtures"

  banner "product E3 cargo tests (e3_c_*)"
  cargo test -q -p nytprof-format-v6 e3_c_ -- --nocapture
  ok "cargo test -p nytprof-format-v6 e3_c_"
  ok "E3-mixed SOURCE/INDEX/SUMMARY decoded from C mixed.nytprof (e3_c_mixed_*)"
else
  note "skip cargo E3 decode (no cargo) — fixture presence still checked"
  # Without cargo we cannot run product E3 equality; fail closed if this is
  # the only evidence path operators expected. Offline gate step 1 already
  # runs format-v6 when cargo present; this script is required when cargo
  # is available (caller offline_gate only invokes when cargo present).
  if [[ "${NYTPROF_E3_REQUIRE_CARGO:-1}" == "1" ]]; then
    fail "cargo required for product E3 e3_c_* equality (set NYTPROF_E3_REQUIRE_CARGO=0 to soft-skip)"
  fi
fi

note "E3-EVENT ready with C (absolute/packing/dict/mid-stream matrix)"
note "E3-mixed MVP: SOURCE/INDEX/SUMMARY product C fixture mixed.nytprof"
note "Not TEST-008; not COL-008; not CLI v6 collection default; not S2; not R3/R4 flip"

banner "e3_c_writer_parity PASSED"
ok "COL-007 product E3-EVENT + E3-mixed with real C bytes"
exit 0

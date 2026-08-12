#!/usr/bin/env bash
# COL-001..007-abs / PR-B02..B06 — collector sink + lifecycle/seq + batch/fast +
# fake-clock + real v5 wire + absolute v6 wire smoke.
#
# When a C toolchain is present: build + unit-test the overlay sink (needs zlib).
# When absent: honest skip (offline_gate remains green).
# Always: isolation asserts — collector/ must never appear as oracle PERL5LIB.
# Optional: when cargo + nytprof-dump available, verify mini wire artifact.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/collector_sink_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

COLLECTOR="$ROOT/collector"
MAKEFILE="$COLLECTOR/Makefile"
BASELINE="$ROOT/baseline/6.15"
PERL5LIB_FILE="$BASELINE/oracle-perl5lib.txt"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
banner() { printf '\n=== %s ===\n' "$*"; }

banner "collector_sink_smoke (COL-001..007-pack + fake-clock + v5/v6 wire)"

# ---------------------------------------------------------------------------
# Tree present (this smoke is only meaningful after PR-B02 lands sources)
# ---------------------------------------------------------------------------
[[ -d "$COLLECTOR" ]] || fail "missing collector/ overlay (ADR-0004 / COL-001)"
[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$COLLECTOR/include/nytp_sink.h" ]] || fail "missing nytp_sink.h"
[[ -f "$COLLECTOR/include/nytp_clock.h" ]] || fail "missing nytp_clock.h (PR-B03)"
[[ -f "$COLLECTOR/include/nytp_batch.h" ]] || fail "missing nytp_batch.h (PR-B04)"
[[ -f "$COLLECTOR/include/nytp_event.h" ]] || fail "missing nytp_event.h (PR-B04)"
[[ -f "$COLLECTOR/include/nytp_sink_v5.h" ]] || fail "missing nytp_sink_v5.h"
[[ -f "$COLLECTOR/src/nytp_sink_v5.c" ]] || fail "missing v5 wire adapter"
[[ -f "$COLLECTOR/include/nytp_sink_v6.h" ]] || fail "missing nytp_sink_v6.h (PR-B06)"
[[ -f "$COLLECTOR/src/nytp_sink_v6.c" ]] || fail "missing v6 absolute wire adapter (PR-B06)"
[[ -f "$COLLECTOR/include/nytprof_v6_ids.h" ]] || fail "missing nytprof_v6_ids.h lockfile header"
[[ -f "$COLLECTOR/src/nytp_clock.c" ]] || fail "missing nytp_clock.c"
[[ -f "$COLLECTOR/src/nytp_batch.c" ]] || fail "missing nytp_batch.c (PR-B04)"
[[ -f "$COLLECTOR/t/test_v5_wire.c" ]] || fail "missing test_v5_wire.c (PR-B05)"
[[ -f "$COLLECTOR/t/test_v6_abs_wire.c" ]] || fail "missing test_v6_abs_wire.c (PR-B06)"
[[ -f "$COLLECTOR/t/test_v6_codec_chunk_crc.c" ]] || fail "missing test_v6_codec_chunk_crc.c (PR-B07)"
[[ -f "$COLLECTOR/t/test_v6_packing_footer.c" ]] || fail "missing test_v6_packing_footer.c (PR-B08)"
ok "collector/ overlay tree present (B0-A; COL-001..007-abs)"

# ---------------------------------------------------------------------------
# Isolation: never put collector/ (or crates/) on oracle PERL5LIB
# ---------------------------------------------------------------------------
assert_no_bad_perl5lib() {
  local label="$1"
  local p5="${2-}"
  case ":${p5}:" in
    *"/crates/"*)
      fail "$label PERL5LIB must not contain /crates/: $p5"
      ;;
  esac
  # Path-component asserts for collector/ (ADR-0004 §3 / COL-001).
  if [[ -n "$p5" ]]; then
    local IFS=':'
    local part base
    for part in $p5; do
      [[ -n "$part" ]] || continue
      case "$part" in
        *"/collector"|*"/collector/"*|*/collector/install*|*/prefix/collector*)
          fail "$label PERL5LIB must not contain collector overlay path: $part"
          ;;
      esac
      # Bare relative entry (PERL5LIB=collector) and basename component.
      base="$(basename -- "$part")"
      if [[ "$part" == "collector" ]] || [[ "$base" == "collector" ]]; then
        fail "$label PERL5LIB must not contain collector path component: $part"
      fi
      if [[ "$part" == *"/collector/"* ]] || [[ "$part" == *"/collector" ]]; then
        fail "$label PERL5LIB contains collector path component: $part"
      fi
    done
  fi
  ok "$label: no crates/ or collector/ on PERL5LIB"
}

# Parent process env (gate parent must not leak collector onto PERL5LIB).
assert_no_bad_perl5lib "parent env" "${PERL5LIB-}"

# Oracle pin file, if present, must also be clean.
if [[ -f "$PERL5LIB_FILE" ]]; then
  oracle_p5="$(tr -d '\r' <"$PERL5LIB_FILE" | head -1 || true)"
  assert_no_bad_perl5lib "oracle-perl5lib.txt" "$oracle_p5"
else
  ok "oracle-perl5lib.txt absent (oracle not built here; isolation still checked on parent env)"
fi

# Refuse if someone exported a collector install into PERL5LIB for this smoke.
case ":${PERL5LIB-}:" in
  *"${COLLECTOR}"*)
    fail "PERL5LIB must not include collector tree path: $PERL5LIB"
    ;;
esac

# ---------------------------------------------------------------------------
# Honest skip if no C toolchain
# ---------------------------------------------------------------------------
resolve_cc() {
  if [[ -n "${CC-}" ]] && command -v "$CC" >/dev/null 2>&1; then
    printf '%s\n' "$CC"
    return 0
  fi
  for c in cc gcc clang; do
    if command -v "$c" >/dev/null 2>&1; then
      printf '%s\n' "$c"
      return 0
    fi
  done
  return 1
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — collector sink unit tests not run"
  echo "  (honest skip; legacy-only / dual-path half still independent of collector/)"
  echo "  To exercise: install a C compiler and re-run this smoke"
  ok "collector_sink_smoke completed (skip — no CC)"
  exit 0
fi
ok "C toolchain: $CC_BIN"

# zlib/zstd/lz4 required for COL-006 v5 + COL-007 v6 codecs
if ! echo 'int main(void){return 0;}' | "$CC_BIN" -x c - -lz -lzstd -llz4 -o /tmp/nytp_zcheck_$$ 2>/dev/null; then
  fail "zlib+zstd+lz4 (-lz -lzstd -llz4) required for COL-006/007 wire writers; install zlib/zstd/lz4 dev packages"
fi
rm -f /tmp/nytp_zcheck_$$
ok "zlib/zstd/lz4 link available"

# ---------------------------------------------------------------------------
# Build + unit test (real entry: collector/Makefile)
# ---------------------------------------------------------------------------
banner "make -C collector clean test"
make -C "$COLLECTOR" clean
make -C "$COLLECTOR" test CC="$CC_BIN"

[[ -x "$COLLECTOR/build/test_sink_api" ]] || fail "test binary missing after make test"
[[ -x "$COLLECTOR/build/test_lifecycle_seq" ]] || fail "test_lifecycle_seq missing"
[[ -x "$COLLECTOR/build/test_fake_clock" ]] || fail "test_fake_clock missing"
[[ -x "$COLLECTOR/build/test_batch_fast" ]] || fail "test_batch_fast missing (PR-B04)"
[[ -x "$COLLECTOR/build/test_v5_wire" ]] || fail "test_v5_wire missing (PR-B05)"
[[ -x "$COLLECTOR/build/test_v6_abs_wire" ]] || fail "test_v6_abs_wire missing (PR-B06)"
# Re-run shipped binaries from collector/ so relative build/*.nytprof paths work.
(
  cd "$COLLECTOR"
  ./build/test_sink_api
  ./build/test_lifecycle_seq
  ./build/test_fake_clock
  ./build/test_batch_fast
  ./build/test_v5_wire
  ./build/test_v6_abs_wire
)
ok "collector unit tests (sink + lifecycle/seq + fake-clock mini M4 + batch/fast + v5 + v6-abs wire)"

# Mini wire artifact from test_v5_wire
WIRE="$COLLECTOR/build/m4_mini_wire.nytprof"
[[ -f "$WIRE" ]] || fail "expected mini wire artifact $WIRE after test_v5_wire"
# bash $(...) strips trailing newlines — compare via cmp on raw bytes
printf 'NYTProf 5 0\n' | cmp -n 12 - "$WIRE" >/dev/null 2>&1 \
  || fail "mini wire missing NYTProf 5 0 header"
ok "mini wire artifact present with v5 header ($WIRE)"

# Absolute v6 mini from test_v6_abs_wire
WIRE6="$COLLECTOR/build/m4_mini_v6.nytprof"
[[ -f "$WIRE6" ]] || fail "expected absolute v6 mini artifact $WIRE6 after test_v6_abs_wire"
printf 'NYTPROF6' | cmp -n 8 - "$WIRE6" >/dev/null 2>&1 \
  || fail "v6 mini missing NYTPROF6 magic"
ok "absolute v6 mini artifact present with NYTPROF6 magic ($WIRE6)"


# Codec / multi-chunk artifacts from test_v6_codec_chunk_crc
for art in v6_zlib_one.nytprof v6_zstd_one.nytprof v6_lz4_one.nytprof \
           v6_zlib_multi.nytprof v6_zstd_multi.nytprof v6_lz4_multi.nytprof; do
  ART="$COLLECTOR/build/$art"
  [[ -f "$ART" ]] || fail "expected codec artifact $ART after test_v6_codec_chunk_crc"
  printf 'NYTPROF6' | cmp -n 8 - "$ART" >/dev/null 2>&1 \
    || fail "codec artifact $art missing NYTPROF6 magic"
done
ok "v6 codec/multi-chunk artifacts present with NYTPROF6 magic"

# Optional: Rust always-inflate decode on C absolute + compressed artifacts.
if command -v cargo >/dev/null 2>&1; then
  banner "Rust v6 always-inflate decode (COL-007 dual-path check)"
  for art in m4_mini_v6.nytprof \
             v6_zlib_one.nytprof v6_zstd_one.nytprof v6_lz4_one.nytprof \
             v6_zlib_multi.nytprof v6_zstd_multi.nytprof v6_lz4_multi.nytprof; do
    ART="$COLLECTOR/build/$art"
    if (cd "$ROOT" && cargo run -q -p nytprof-format-v6 --example decode_abs_c_mini -- "$ART" --require-crc); then
      ok "Rust decode accepted C artifact $art"
    else
      fail "Rust decode failed on C artifact $art"
    fi
  done
else
  echo "NOTE: skip Rust v6 decode (no cargo) — C self-tests still cover wire"
fi


# Optional: independent Rust v5 decoder when already built or cargo available.
resolve_dump() {
  if [[ -n "${NYTPROF_NATIVE_CLI-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
    printf '%s\n' "$NYTPROF_NATIVE_CLI"
    return 0
  fi
  if [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
    printf '%s\n' "$ROOT/target/debug/nytprof-dump"
    return 0
  fi
  if [[ -x "$ROOT/prefix/bin/nytprof-dump" ]]; then
    printf '%s\n' "$ROOT/prefix/bin/nytprof-dump"
    return 0
  fi
  return 1
}

if DUMP_BIN="$(resolve_dump)"; then
  banner "Rust v5 decoder verify (COL-006 dual-path check)"
  "$DUMP_BIN" verify "$WIRE" || fail "nytprof-dump verify failed on mini wire"
  ok "nytprof-dump verify accepted mini wire ($DUMP_BIN)"
elif command -v cargo >/dev/null 2>&1; then
  banner "Rust v5 decoder verify via cargo build -p nytprof-cli"
  (cd "$ROOT" && cargo build -q -p nytprof-cli)
  [[ -x "$ROOT/target/debug/nytprof-dump" ]] || fail "nytprof-dump missing after cargo build"
  "$ROOT/target/debug/nytprof-dump" verify "$WIRE" || fail "nytprof-dump verify failed"
  ok "nytprof-dump verify accepted mini wire (cargo build)"
else
  echo "NOTE: skip Rust decoder verify (no nytprof-dump / cargo) — C self-tests still cover wire"
fi

# Residual honesty banner.
echo "NOTE: COL-006 real v5 wire on mini samples — full fixtures/v5/* oracle corpus is complete TEST-003 residual"
# Packing / dict / mid-stream artifacts from test_v6_packing_footer
for ART in   "$COLLECTOR/build/v6_pack_multi.nytprof"   "$COLLECTOR/build/v6_mid_stream.nytprof"   "$COLLECTOR/build/v6_dict.nytprof"   "$COLLECTOR/build/v6_pack_dict.nytprof"
do
  [[ -f "$ART" ]] || fail "expected packing/dict artifact $ART after test_v6_packing_footer"
done
ok "packing/dict/mid-stream artifacts present"

if command -v cargo >/dev/null 2>&1; then
  banner "Rust dual-path packing/dict (decode_c_b08)"
  cargo run -q -p nytprof-format-v6 --example decode_c_b08 --     "$COLLECTOR/build/v6_pack_multi.nytprof"
  cargo run -q -p nytprof-format-v6 --example decode_c_b08 --     "$COLLECTOR/build/v6_mid_stream.nytprof"
  cargo run -q -p nytprof-format-v6 --example decode_c_b08 --     "$COLLECTOR/build/v6_dict.nytprof" --dict
  cargo run -q -p nytprof-format-v6 --example decode_c_b08 --     "$COLLECTOR/build/v6_pack_dict.nytprof" --dict
  ok "Rust always-inflate accepted packing/dict C bytes"
fi

echo "NOTE: COL-007 ABS+CODEC+PACK scaffold; E3-C residual — board COL-007 not done; not live XS hooks"
echo "NOTE: M4 mini sample only — full oracle corpus under fake-clock needs complete TEST-003"
echo "NOTE: batch light microbench is engineering only — not BENCH-003/006 certification"
echo "NOTE: flush/compression discount timing vs BASE-003 remains residual"
echo "NOTE: nytp_ticks outside I32 fails closed (OI-003-01 overflow composition residual)"

banner "collector_sink_smoke PASSED"
ok "COL-001..007-abs + fake-clock + v5/v6-abs wire scaffold build + isolation"
exit 0

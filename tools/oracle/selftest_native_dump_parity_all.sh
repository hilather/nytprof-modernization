#!/usr/bin/env bash
# Native dump structural parity on the multi-fixture set (DUMP-PARITY-EXPAND).
#
# Spec: docs/schemas/native-dump-parity-mvp-v0.md
#
# Runs full structural parity (dump×2 + normalize + compare_jsonl + tag
# multiplicity) for:
#   - default-calls1   (TIME_LINE; TIME_BLOCK == 0)
#   - calls2-default   (TIME_LINE; calls=2)
#   - blocks-calls1    (TIME_BLOCK; TIME_LINE == 0)
#
# Multiplicity counts are loaded per fixture golden — not hard-coded from
# default-calls1.
#
# Usage:
#   bash tools/oracle/selftest_native_dump_parity_all.sh
#   ./tools/oracle/selftest_native_dump_parity_all.sh
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
cd "$ROOT"

exec bash "$DIR/selftest_native_dump_parity.sh" \
  default-calls1 \
  calls2-default \
  blocks-calls1

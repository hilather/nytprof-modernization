# E4-v0 model-level semantic equality MVP (v0)

**Board ID:** `E4-V0-MODEL-SEMANTIC-MVP`  
**Status:** implemented (PR-B10) — **not** full oracle dual equality; **not** wire freeze; **not** full CLI E5 / E4 product smoke  
**Depends on:** COL-014 dual-sink (PR-B10a, test/dev-only OQ-4); product v6→ProfileModel ingest (PR-B11a); E4 policy  
**Evidence:** `cargo test -p nytprof-model e4_v0_`; `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only`

## Goal

Enforce **E4-v0** without depending on full CLI E5 report surfaces:

```text
same-run v5 + v6 profile files
  → ProfileModel::from_path (dual dispatch)
  → e4_v0_aggregates_equal(v5_model, v6_model)
```

Compare **decoded aggregates** (A1–A9 + stream completeness), not wire bytes or event order.

## Surfaces compared

| Surface | Equal? |
|---------|--------|
| TIME_LINE / TIME_BLOCK / DISCOUNT / SUB_ENTRY / SUB_RETURN / SUB_CALLERS / NEW_FID / SRC_LINE / SUB_INFO / PID_* multiplicities | **yes** |
| A4 `line_totals`, A4b `block_line_totals` | **yes** |
| A5 returns + incl/excl (f64_close; integer-tick fixtures) | **yes** |
| A7 call edges (count/sites/depth + times) | **yes** |
| A8 source lines, A9 sub_defs, files/attributes/options | **yes** |
| `is_stream_complete` | **yes** |
| Wire bytes / event order | **not required** |
| Absolute vs packing v6 of same logical sample | **yes** (also covered via C E3 fixtures) |

## Fixture pairs

Committed under [`fixtures/e4/dual-sink/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/e4/dual-sink/):

| Stem | Role |
|------|------|
| `m4` | M4 mini under dual |
| `default_calls1` | leaf 15 / mid 3 / edge 15 pattern (scaled DISCOUNT×15) |
| `blocks_calls1` | TIME_BLOCK A4/A4b pattern |
| `calls2_default` | SUB_ENTRY×9 pattern |

Produced by COL-014 `test_dual_sink` (integer-tick SUB_RETURN/SUB_CALLERS). **Not** full oracle counts.

## API

| Symbol | Location |
|--------|----------|
| `e4_v0_aggregates_equal(a, b, compare_total_events)` | `nytprof_model` |
| `ProfileModel::from_path` | dual v5/v6 dispatch (PR-B11a) |

## Smoke

```sh
./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only
# optional regenerate:
NYTPROF_REGEN_E4_DUAL=1 ./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only
```

Honest skip when cargo missing (fixture presence still checked). Full CLI path **not** implemented (PR-B12b).

## Non-claims / residuals

- Not full `fixtures/v5/*` oracle dual equality (TEST-003 / TEST-008)
- Not E4 product smoke in offline_gate (PR-B12b)
- Not wire freeze / CLI v6 default / product `format=dual`
- Dual-sink remains **test/dev-only** (OQ-4)
- Fractional wall-NV dual pairs are **not** E4-equal under v6 u64 truncation; dual fixtures use integer ticks by design

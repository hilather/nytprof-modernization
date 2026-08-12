# E4-v0 dual-sink same-run pairs (v5 + v6)

**Status:** E4-v0 model-level evidence (PR-B10) — **not** full oracle dual equality; **not** wire freeze  
**Producer:** COL-014 dual-sink harness (`collector/t/test_dual_sink.c`) — test/dev-only (OQ-4)  
**Consumer:** `ProfileModel::from_path` dual dispatch + `e4_v0_aggregates_equal`  
**Evidence:** `cargo test -p nytprof-model e4_v0_` · `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only`

These files are **same-run** v5 + absolute-v6 wires from the dual-sink fan-out of primary-fixture-**shaped** synthetic streams (multiplicity patterns). They are **not** full `fixtures/v5/*` oracle corpora.

## Matrix

| Stem | Intent | Advertised pattern samples |
|------|--------|----------------------------|
| `m4` | M4 mini sample under dual | TIME_LINE×3, DISCOUNT×1, leaf returns 1 |
| `default_calls1` | calls=1 shape | leaf **15**, mid **3**, mid→leaf **15**, SUB_ENTRY **0**, DISCOUNT×15 (scaled; not oracle 818) |
| `blocks_calls1` | blocks path | TIME_BLOCK×12 → A4 line5 calls 12, A4b block 1:4 calls 12 |
| `calls2_default` | calls=2 shape | SUB_ENTRY×9, leaf returns 9 |

Each stem has `{stem}_v5.nytprof`, `{stem}_v6.nytprof`, and optional `{stem}_meta.json` (out-of-band COL-014 compare meta; not on either wire).

## Integer-tick policy

`SUB_RETURN` / `SUB_CALLERS` incl/excl/reci use **integer tick** values so v5 NV wire and v6 `nv_to_u64` truncation stay E4-equal (see policy absolute-ticks note). Fractional wall NV minis would truncate to 0 on v6 and fail model time equality.

## Regenerate

Requires a C toolchain (zlib, zstd, lz4) and `make`:

```sh
make -C collector test
# copies dual_* artifacts:
NYTPROF_REGEN_E4_DUAL=1 ./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only
```

Committed binaries keep `cargo test -p nytprof-model e4_v0_` green without regenerating.

## Residuals

| Residual | Notes |
|----------|--------|
| Full oracle `fixtures/v5/*` dual pairs | complete TEST-003 + TEST-008 M6 |
| E4 product CLI smoke / offline_gate | PR-B12b |
| Wire freeze / CLI v6 default | after E3/E4 product evidence |
| Product `format=dual` | **rejected** (OQ-4) — dual-sink remains test/dev-only |

## Schema / policy

- [`docs/schemas/e4-v0-model-semantic-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-v0-model-semantic-mvp-v0.md)
- [`docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md)

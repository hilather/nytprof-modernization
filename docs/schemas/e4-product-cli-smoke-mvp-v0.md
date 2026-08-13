# E4 product CLI smoke MVP (v0)

**Board ID:** `E4-PRODUCT-CLI-SMOKE-MVP`  
**Status:** implemented (PR-B12b) — **not** full oracle dual equality (TEST-003/TEST-008); **not** CLI v6 collection default (R4); **not** product `format=dual`  
**Depends on:** E4-v0 model semantic ([`e4-v0-model-semantic-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-v0-model-semantic-mvp-v0.md)); CLI E5 v6 opt-in ([`cli-e5-v6-opt-in-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cli-e5-v6-opt-in-mvp-v0.md)); COL-014 dual-sink fixtures  
**Evidence:** `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --full`; `cargo test -p nytprof-cli e4_product_`; offline_gate step 12 when native CLI available

## Goal

Enforce **E4 product** semantic equality with **real native CLIs** on both formats of same-run dual-sink pairs:

```text
fixtures/e4/dual-sink/{stem}_v5.nytprof
fixtures/e4/dual-sink/{stem}_v6.nytprof
  → nytprof-cli verify / report --json / report / csv / folded / callgrind
  → advertised aggregates equal (after dropping path-only `profile`)
```

This is the offline_gate product path for class **E4** (beyond model-only E4-v0).

## Modes

| Mode | Command | What runs |
|------|---------|-----------|
| **full** (default) | `./scripts/packaging/e4_v5_v6_semantic_smoke.sh` or `--full` / `--cli` | Model `e4_v0_*` + real CLI product stage |
| model-only | `--model-only` | ProfileModel / `e4_v0_*` only (PR-B10 path) |

## Product stage checks

| Check | Stems | Rule |
|-------|-------|------|
| Magic / presence | all dual pairs | v5 `NYTProf 5 0`; v6 `NYTPROF6`; non-empty |
| `verify` | all pairs, both formats | exit 0 |
| `report --json` equality | all pairs | JSON objects equal after removing `profile` |
| report text / csv / folded / callgrind | `default_calls1` both formats | greppable leaf **15** / mid **3** / edge **15** |
| Isolation | always | no `crates/` or `collector/` on `PERL5LIB` |

## offline_gate integration

| Condition | Behavior |
|-----------|----------|
| `native_cli_available` (cargo / prefix / target / `NYTPROF_NATIVE_CLI`) **and** dual-sink fixtures present | **required** step 12: `e4_v5_v6_semantic_smoke.sh --full` |
| native CLI unavailable | honest **skip** of product stage; dual-sink fixture **presence** still required |

C dual-sink wires are **committed** under [`fixtures/e4/dual-sink/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/e4/dual-sink/) (COL-014 / PR-B10a). Regenerating them still needs a C toolchain (`NYTPROF_REGEN_E4_DUAL=1`).

## Fixtures

Same scaled dual-sink pairs as E4-v0:

| Stem | Pattern samples |
|------|-----------------|
| `m4` | mini dual |
| `default_calls1` | leaf **15** / mid **3** / edge **15** |
| `blocks_calls1` | TIME_BLOCK A4/A4b pattern |
| `calls2_default` | SUB_ENTRY pattern |

**Not** full oracle `fixtures/v5/*` counts (DISCOUNT 818 / TL 916 residual).

**E4-01 / E4-02 / E4-03:** `fixtures/e4/oracle-pair/{default_calls1,blocks_calls1,calls2_default}_{v5,v6}.nytprof` — smoke asserts pair presence + `NYTProf 5` / `NYTPROF6` and, when a native CLI exists, shipped `report --json` leaf/mid/edge equality. Not full JSON equality. Not A4 **780** / SUB_ENTRY **27** / `format=dual` / `--allow-lossy`.

## Tests

| Evidence | Command |
|----------|---------|
| Packaging smoke full | `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --full` |
| Packaging smoke model | `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only` |
| CLI regression | `cargo test -p nytprof-cli e4_product_` |
| offline_gate | `./scripts/ci/offline_gate.sh` (step 12 when native) |

## Non-claims / residuals

- **Not** full oracle dual pairs (TEST-003 / TEST-008)
- **Not** collection `format=v6` default (R4)
- **Not** product `format=dual` UX (OQ-4; dual-sink remains test/dev-only)
- **Not** convert / merge tooling
- **Not** E3-mixed multi-kind product path
- Dual fixtures use integer-tick SUB_RETURN/SUB_CALLERS for v5 NV ↔ v6 u64 equality

## Related

- E4 policy: [`docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md)
- Dual-equality readiness: [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md)
- E4-v0 model: [`e4-v0-model-semantic-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-v0-model-semantic-mvp-v0.md)
- CLI E5: [`cli-e5-v6-opt-in-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cli-e5-v6-opt-in-mvp-v0.md)
- Board: [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md) (`E4-PRODUCT-CLI-SMOKE-MVP`)

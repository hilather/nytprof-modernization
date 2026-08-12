# E4 v5↔v6 semantic equality policy (provisional) — v0

**Status:** provisional policy for dual-equality class **E4** — **E4-v0 model enforcement ready** (PR-B10); **E4 product CLI smoke ready** (PR-B12b); **not** dual-equality freeze; **not** full oracle dual  
**Board IDs:** `E4-V5-V6-SEMANTIC-EQUALITY-POLICY-PROVISIONAL`, `E4-V5-V6-SEMANTIC-EQUALITY-POLICY-MVP`, `E4-V0-MODEL-SEMANTIC-MVP`, `E4-PRODUCT-CLI-SMOKE-MVP`  
**Depends on:** [`DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md); packing ADR [`0001`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md); offline R1-preview residual matrix  
**Gate:** policy draft **before** COL-007 product claim; **E4-v0** model + **E4 product CLI** on dual-sink pairs (PR-B10/B12b); full oracle dual residual

---

## Purpose

Define which **advertised** semantic surfaces must match when the same workload is profiled as **v5 (oracle 6.15)** and as **v6 (COL-007 collector)** under absolute and packing body policies.

## Equality classes (reminder)

| Class | Scope |
|-------|--------|
| E1 | v5 native ↔ oracle/JSONL (R1-preview ready) |
| E2 | v6 encode↔decode (preflight ready) |
| E3 | C writer bytes ↔ Rust decode (EVENT ready; mixed residual) |
| **E4** | **v5 workload ↔ v6 workload semantic aggregates** (this doc) |
| E5 | CLI product path on v6 (ready PR-B12; collection default residual) |

## Required equal surfaces (default-calls1 / blocks-calls1 / calls2-default fixtures)

When both v5 and v6 profiles of the **same workload script and NYTPROF options** are available:

| Surface | Must equal | Notes |
|---------|------------|-------|
| Sub return counts (leaf/mid) | **yes** | Advertised leaf **15** / mid **3** on default-calls1 |
| Call-edge mid→leaf | **yes** | **15** on default-calls1 |
| A4 line_totals calls (blocks fixtures) | **yes** | line5 **780** on blocks-calls1 |
| A4b block_line_totals | **yes** when blocks enabled | **810** on blocks-calls1 `"1:4"` |
| SUB_ENTRY multiplicity | **yes** | **0** calls1 / **27** calls2 |
| DISCOUNT multiplicity | **yes** | **818** default-calls1 |
| Stream complete + pid balance | **yes** | COMPAT-010 |
| A9 sub_defs leaf/mid ranges | **yes** | leaf 1/3–7, mid 1/8–12 |
| Source line samples | **yes** | workload hot loop text |
| Attribute/option greppable samples | **yes** when both expose | ticks_per_sec, calls, basetime samples |
| Absolute ticks / wall times | **policy** | Integer ticks equal after normalize; wall volatiles follow COMPAT-002 |
| Event wire order | **not required** | Packing/run forms may reorder wire without changing logical aggregates |
| Absolute vs packed v6 bodies | **logical equal** | Same workload v6 absolute vs packing encode must yield equal E2 decode then equal E4 aggregates |

## Packing policy interaction

- v6 profiles may use absolute EVENT bodies or packing forms under ADR-0001.
- E4 compares **decoded logical / aggregate semantics**, not wire bytes.
- If packing expands TIME_*_RUN, multiplicity of logical TIME_LINE/TIME_BLOCK events must match the absolute encoding of the same logical stream.

## Enforcement stages

| Stage | Status | How |
|-------|--------|-----|
| **E4-v0 (model-level)** | **ready (PR-B10)** | `ProfileModel::from_path` on same-run dual-sink pairs → `e4_v0_aggregates_equal`. Fixtures [`fixtures/e4/dual-sink/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/e4/dual-sink/); schema [`e4-v0-model-semantic-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-v0-model-semantic-mvp-v0.md); smoke `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only` |
| **E4 product (CLI)** | **ready (PR-B12b)** | Real native CLIs on both formats: verify + `report --json` equality + E5 surfaces on `default_calls1`; smoke `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --full`; offline_gate step 12 when native; schema [`e4-product-cli-smoke-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-product-cli-smoke-mvp-v0.md) |
| Full oracle dual pairs | **open** (TEST-003/TEST-008) | Full `fixtures/v5/*` workloads under dual |

### E4-v0 fixture honesty

- Dual pairs are **primary-fixture-shaped synthetic** streams (leaf 15 / mid 3 / edge 15 **patterns**), **not** full oracle DISCOUNT **818** / TIME_LINE **916**.
- `SUB_RETURN` / `SUB_CALLERS` times use **integer ticks** so v5 NV wire and v6 `nv_to_u64` truncation stay equal (fractional wall NV truncates to 0 on v6).
- Dual-sink remains **test/dev-only** (OQ-4).

## Open enforcement residuals

| Item | Status |
|------|--------|
| COL-007 C emitter producing v6 profiles of **full oracle** fixtures | **open** (product E3-EVENT mini matrix done; full oracle workload residual) |
| Full oracle same-workload → v5 + v6 pair | **open** (COL-014 dual-sink provides same-run fan-out + E4-v0 on scaled shapes; full oracle residual) |
| COL-014 dual-sink same-run logical equality (test/dev-only, OQ-4) | **harness ready** — `test_dual_sink` on M4 + primary-fixture-shaped streams; not product UX |
| Automated E4 **product** smoke in offline_gate | **ready (PR-B12b)** — step 12 when native CLI available; dual-sink scaled pairs only |
| Tick / basetime volatile normalize for dual profiles | follow COMPAT-002/003 |

## Non-claims

- Not full E4 aggregate enforcement on **oracle** fixture pairs; not CLI v6 **collection** default.
- COL-014 dual-sink is **test/dev-only** (OQ-4) — not advertised product `format=dual`.
- Not full REPORT HTML DOM parity or XS Data fidelity.
- E4 product smoke uses **scaled** dual-sink shapes, not oracle DISCOUNT 818 / TL 916.
- First-slice R1-preview remains v5-only advertised product collection path.

## Evidence paths

- R1 residual matrix advertised ready rows for count samples.
- Dual-equality readiness: [`DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md)
- COL-014 dual-sink schema: [`docs/schemas/collector-dual-sink-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-dual-sink-mvp-v0.md)
- E4-v0 schema: [`docs/schemas/e4-v0-model-semantic-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-v0-model-semantic-mvp-v0.md)
- E4 product CLI schema: [`docs/schemas/e4-product-cli-smoke-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-product-cli-smoke-mvp-v0.md)
- E4 fixtures: [`fixtures/e4/dual-sink/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/e4/dual-sink/)
- E4 smoke: `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --full` (or `--model-only`)
- offline_gate step 12 (when native): E4 product CLI smoke
- E3 harness (writer-bytes → Rust decode): `crates/nytprof-format-v6` module `dual_equality`
- Model API: `nytprof_model::e4_v0_aggregates_equal`
- CLI tests: `cargo test -p nytprof-cli e4_product_`

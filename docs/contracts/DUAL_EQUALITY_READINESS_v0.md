# Dual-equality readiness contract (provisional) — v0

**Status:** readiness checklist — **R2-preview opt-in ready** (PR-B13 honesty cut); **COL-007 product E3-EVENT with C is done** (PR-B09); **wire freeze ADR-0006 accepted** (major=6 IDs); **COL-009 C baseline reaffirmed** (ADR-0007); **not** full dual-equality product freeze (E3-mixed / full oracle E4 residual); **not** R2-stable / R3 / R4  
**Board ID:** `DUAL-EQUALITY-READINESS-MVP` (authoritative — contract shipped + residual honesty; no separate PROVISIONAL board row); packaging cut `R2-PREVIEW-READINESS-CUT`  
**Depends on:** offline R0 / R1-preview residual matrix; **accepted** packing ADR [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md); **accepted** string-pool ADR [`docs/adrs/0002-v6-string-pool-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md); **accepted** wire freeze ADR [`docs/adrs/0006-v6-wire-freeze.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md); **accepted** COL-009 C-baseline ADR [`docs/adrs/0007-production-v6-writer-backend-c-baseline.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0007-production-v6-writer-backend-c-baseline.md); ID lockfile [`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md) (status **frozen**); `nytprof-format-v6`; C `nytp_sink_v6` (PR-B06..B08); E3-C fixtures (PR-B09); E4-v0 (PR-B10); CLI E5 (PR-B12); E4 product (PR-B12b)

---

## Purpose

Define what **dual-equality** means on the path to **R2-preview** (v6 collection/report **opt-in**) and which gates remain **open** after product COL-007 E3-EVENT + wire freeze + CLI E5. This document freezes **readiness structure**; wire numeric IDs are frozen by ADR-0006; production writer backend is C (ADR-0007).

## Maintainer OQ-1 (2026-08-11) + wire freeze (PR-B11)

| Item | Decision |
|------|----------|
| ADR-0001 packing | **Accepted as-is** (proposed→accepted). Do **not** supersede. COL-007 packing implements ADR-0001 intent. |
| ADR-0002 FOOTER string-pool | **Accepted as-is** (FOOTER-local). Do **not** supersede. COL-007 dict emit implements ADR-0002 intent. |
| Wire freeze | **Accepted** (ADR-0006) after E3-EVENT(C) + E4-v0; golden vectors under `fixtures/v6/vectors/`. OQ-5 seq policy frozen; OQ-6 global pool deferred. |

## Plan FMT-002..010 deviation (closed for freeze class)

Plan COL-007 listed dependencies **FMT-002 through FMT-010**. This program **implemented COL-007 against the provisional ID lockfile**, then **promoted** formal freeze after E3/E4-v0 (ADR-0006). Details: [`V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md); catalog [`v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md).

## Dual-equality classes

| Class | Meaning | Independent check |
|-------|---------|-------------------|
| **E1 — v5 semantic surfaces** | Native v5 read/report vs oracle / pure-Perl JSONL bridges on advertised fixtures (already R1-preview ready for offline scope) | offline_gate + packaging smokes; residual matrix “Advertised ready” |
| **E2 — v6 encode↔decode** | Rust (or C) encode → always-inflate decode recovers equal logical events/sites/seq/strings under packing/absolute policies | `cargo test -p nytprof-format-v6` always-inflate packing/mid-stream/multi-chunk tests |
| **E3 — C writer ↔ Rust decode** | COL-007 C emitter produces streams that Rust always-inflate path decodes with E2 equality on golden workloads | **ready (EVENT)** — C fixtures `fixtures/v6/from-c/**` + `e3_c_*` tests + `tools/oracle/e3_c_writer_parity.sh`. Stand-in harness remains engineering only. **E3-mixed residual.** |
| **E4 — v5↔v6 semantic** | Same workload profiled as v5 and v6 yields equal advertised aggregates / dump structure after normalize | **E4-v0 ready (PR-B10)** + **E4 product CLI ready (PR-B12b)** — policy + dual-sink pairs + model equality + real CLI `report --json`/E5 surfaces + offline_gate step 12; **COL-014** logical equality harness (PR-B10a); full oracle pairs residual (TEST-008) |
| **E5 — CLI product path** | CLI report/verify on v6 files as product surface (opt-in, not default) | **ready (PR-B12)** — report/html/csv/folded/callgrind/dump/verify on v6; capability `v6_decode`/`v6_report`; convert/merge **false**; `collection_default: v5`; schema [`cli-e5-v6-opt-in-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cli-e5-v6-opt-in-mvp-v0.md) |

## Readiness matrix

| Surface / gate | Equality class | Status | Notes |
|----------------|----------------|--------|-------|
| Offline R0 / R1-preview v5 dump/report/JSON/cross | E1 | **ready** (advertised) | Keep green; not a COL-007 substitute |
| Absolute v6 mini/multi-chunk encode↔decode | E2 | **ready** + golden vectors | IDs frozen ADR-0006 |
| Packing site-delta / seq / TIME_*_RUN / multi-chunk / mid-stream continuity | E2 | **intent accepted** (ADR-0001) + IDs frozen | ADR-0006 |
| FOOTER string-dictionary resolve | E2 | **intent accepted** (ADR-0002 FOOTER-local) + IDs frozen | OQ-6 global pool deferred |
| ID lockfile (Rust + C mirror) | — | **frozen** (ADR-0006 / PR-B11) | Path `V6_PROVISIONAL_ID_LOCKFILE_v0` historical |
| Golden vectors `fixtures/v6/vectors/` | E2 | **ready** | FMT-012 class; `golden_vector_*` tests |
| Auto-VERSION header/body align | E2 | **ready** (OQ-5 in ADR-0006 §3) | Seq optional; VERSION may carry FLAG_HAS_SEQ when dual-output active |
| Default `parse_chunk_frame` inflate/CRC | — | **residual** (stays non-inflating) | Product policy ADR if default flips |
| E3 harness (writer bytes → Rust decode) | E3 | **ready** (stand-in + product C path) | Stand-in is **not** product evidence; product path is `e3_c_*` |
| C COL-007 absolute / codecs / packing / dict / mid-stream | E3 runway | **done** (PR-B06..B08 scaffold + PR-B09 E3-C) | Board COL-007 **done** for EVENT product path |
| C COL-007 product E3-EVENT fixtures | E3 | **ready** | `fixtures/v6/from-c/**`; absolute+packing+dict+mid-stream matrix |
| E3-mixed multi-kind C fixtures | E3 | **residual** | SOURCE/INDEX/SUMMARY product C matrix open |
| Batched Rust COL-008 writer | E3/E4 | **deferred** (non-baseline) | COL-009 / ADR-0007 reaffirms C; re-open only with dual-equality + measurement ADR |
| v5↔v6 semantic equality policy | E4 | **policy draft ready** | Surfaces + packing interaction documented |
| COL-014 same-run dual writer (test/dev-only, OQ-4) | E4/M6 runway | **ready (harness)** | Fan-out v5+v6; logical equality on M4 + primary-fixture-shaped streams; **not** product UX; full oracle dual residual |
| E4-v0 model-level aggregate equality | E4 | **ready (PR-B10)** | Dual-sink pairs → ProfileModel → `e4_v0_aggregates_equal`; smoke `--model-only`; schema [`e4-v0-model-semantic-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-v0-model-semantic-mvp-v0.md). Scaled shapes only; full oracle residual |
| E4 product CLI smoke / offline_gate | E4 | **ready (PR-B12b)** | Real CLIs on both formats; smoke `--full`; offline_gate step 12 when native; schema [`e4-product-cli-smoke-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-product-cli-smoke-mvp-v0.md). Scaled dual-sink only; full oracle residual |
| Wire freeze FMT-002..010 | — | **done** (ADR-0006) | After E3-EVENT(C) + E4-v0; golden vectors |
| Product v6→ProfileModel ingest | E5 runway | **ready (MVP)** | PR-B11a: dual-dispatch `from_path`; dump/verify prelim; schema [`product-v6-profilemodel-ingest-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-v6-profilemodel-ingest-mvp-v0.md). Full E5 surfaces: PR-B12 |
| CLI v6 opt-in report/verify | E5 | **ready (PR-B12)** | Full product surfaces + capability honesty; schema [`cli-e5-v6-opt-in-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cli-e5-v6-opt-in-mvp-v0.md); tests `cli_e5_v6` |
| Convert / merge / salvage tooling | E5 residual | **residual (R2-preview)** | Capability `convert`/`merge` **false** on this branch; Phase C (PR-C01/C02 class) may land on other tracks — **do not claim** until capability + tools green |
| Production writer backend (COL-009) | — | **done (ADR-0007)** | C baseline reaffirmed; COL-008 deferred non-baseline |
| R2-preview packaging honesty cut | E1–E5 roll-up | **ready (PR-B13)** | Release notes [`RELEASE_NOTES_R2_PREVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_PREVIEW.md); opt-in only; dual-path legacy unchanged |
| CLI v6 / format default flip (R4) | E5 | **out of scope** | Field window + ADR; capability asserts `collection_default: v5` |
| Full R1 HTML DOM / XS Data / FFI | E1 product depth | **residual** (not R1-preview claim) | Separate full-R1 work |

## Explicit open gates after COL-007 E3-EVENT + wire freeze + R2-preview cut

1. **E3-mixed:** multi-kind SOURCE/INDEX/SUMMARY product C fixtures (EVENT path is ready).
2. **E4 full oracle dual:** E4-v0 model + E4 product CLI smoke are **ready** on dual-sink scaled pairs (PR-B10/B12b; offline_gate step 12). Remaining: full oracle v5+v6 pairs (TEST-003/TEST-008). Policy: [`E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md).
3. **Wire freeze** — **done** (ADR-0006 + golden vectors). OQ-5 seq policy frozen in ADR-0006 §3.
4. **OI-002** full ATTRIBUTE/OPTION key vocabulary — residual (not numeric ID freeze).
5. **CLI v6 full E5** — **ready (PR-B12)** for offline report surfaces + capability honesty; residual: collection default flip (R4), convert/merge claims (PR-C01+ until green).
6. **COL-008** batched Rust writer remains deferred / non-baseline (reaffirmed COL-009 / ADR-0007).
7. **OQ-6 / COL-010** global string pool — deferred (FOOTER-local only).
8. **R2-stable** gates (convert/merge when claimed, COL-015 fork, security/fuzz, P1/P2, platform as advertised) — **not** part of R2-preview.
9. **R3/R4** product default flips — **not started**.

## First-slice vs full-R1 vs R0–R5 (honesty)

| Horizon | Status relative to this doc |
|---------|----------------------------|
| First-slice / offline R0 + R1-preview | **Complete** for advertised surfaces; COL-007 E3-EVENT **done**; COL-008 deferred |
| Full product R1 | **Not complete** on this branch’s residual table (FFI/XS Data, full nytprofhtml DOM, multi-OS CI, perf cert, R3 engine default — Track A may close some on other branches) |
| **R2-preview** v6 opt-in | **Ready (opt-in only)** — ADR-0001/0002 accepted; **wire freeze ADR-0006**; **COL-009 C baseline ADR-0007**; C writer + product E3-EVENT green; COL-014 dual-sink test/dev harness ready (OQ-4); **E4-v0 model ready**; **E4 product CLI ready (PR-B12b)**; **CLI E5 ready (PR-B12)**; packaging honesty PR-B13; **convert/merge residual**; E3-mixed / full oracle E4 residual |
| R2-stable | **Not claimed** — Phase C certification depth |
| R3–R5 defaults / retirement | **Not started** |

## Non-claims

Do **not** treat this document as:

- full dual-equality product freeze (E3-mixed / full oracle E4 still residual) or CLI v6 **collection** default / default-parse always-inflate;
- E3-mixed multi-kind complete;
- E4 full oracle dual complete (TEST-008); E4 product offline_gate smoke **is** complete on dual-sink scaled pairs (PR-B12b);
- convert/merge tooling complete (capability correctly **false** on this branch — honest residual even if convert lands on a parallel track before merge);
- COL-008 done;
- R2-stable, R3, or R4 claims;
- full R1 HTML/XS/FFI or multi-OS CI (unless closed elsewhere with evidence).

Wire **numeric IDs** for major=6 **are** frozen (ADR-0006). Production writer backend is **C** (ADR-0007). Those claims are **not** “product dual-equality complete.”

## Evidence paths

- Residual matrix: [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md)
- Operator runbook: [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md)
- Packing ADR (accepted): [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md)
- String-pool ADR (accepted): [`docs/adrs/0002-v6-string-pool-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md)
- Wire freeze ADR: [`docs/adrs/0006-v6-wire-freeze.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md)
- COL-009 C baseline ADR: [`docs/adrs/0007-production-v6-writer-backend-c-baseline.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0007-production-v6-writer-backend-c-baseline.md)
- R2-preview release notes: [`docs/RELEASE_NOTES_R2_PREVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_PREVIEW.md)
- Frozen ID catalog: [`docs/schemas/v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md)
- ID lockfile (status frozen): [`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md)
- C header: [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h)
- Golden vectors: [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/); `cargo test -p nytprof-format-v6 golden_vector_`
- E3-C schema: [`docs/schemas/collector-v6-e3-c-fixtures-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-e3-c-fixtures-mvp-v0.md)
- E4 policy: [`docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md)
- COL-014 dual-sink (test/dev-only): [`docs/schemas/collector-dual-sink-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-dual-sink-mvp-v0.md); `collector/include/nytp_sink_dual.h`; `make -C collector test` (`test_dual_sink`); smoke step 10
- E3 harness: `crates/nytprof-format-v6/src/dual_equality.rs` (engineering stand-in + `e3_decode_writer_bytes`); product `e3_c_*` in `crates/nytprof-format-v6/tests/e3_c.rs`. Evidence: `cargo test -p nytprof-format-v6 e3_c_` / `e3_harness`; `./tools/oracle/e3_c_writer_parity.sh`
- Product v6→ProfileModel ingest: schema [`docs/schemas/product-v6-profilemodel-ingest-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-v6-profilemodel-ingest-mvp-v0.md); `cargo test -p nytprof-model` (`v6_*`); CLI dump/verify on `fixtures/v6/from-c/**`
- CLI E5 v6 opt-in: schema [`docs/schemas/cli-e5-v6-opt-in-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cli-e5-v6-opt-in-mvp-v0.md); `cargo test -p nytprof-cli --test cli_e5_v6`; capability honesty `v6_decode`/`v6_report` / no convert/merge / `collection_default: v5`
- E4-v0 model semantic: schema [`docs/schemas/e4-v0-model-semantic-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-v0-model-semantic-mvp-v0.md); fixtures [`fixtures/e4/dual-sink/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/e4/dual-sink/); `cargo test -p nytprof-model e4_v0_`; `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --model-only`
- E4 product CLI smoke: schema [`docs/schemas/e4-product-cli-smoke-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-product-cli-smoke-mvp-v0.md); `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --full`; `cargo test -p nytprof-cli e4_product_`
- Offline gate: `./scripts/ci/offline_gate.sh` (step 11 E3 when cargo; step 12 E4 product when native)

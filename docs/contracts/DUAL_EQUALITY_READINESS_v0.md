# Dual-equality readiness contract (provisional) — v0

**Status:** provisional readiness checklist — **COL-007 product E3-EVENT with C is done**; **not** a dual-equality product freeze / wire freeze  
**Board ID:** `DUAL-EQUALITY-READINESS-MVP` (authoritative — contract shipped + residual honesty; no separate PROVISIONAL board row)  
**Depends on:** offline R0 / R1-preview residual matrix; **accepted** packing ADR [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md); **accepted** string-pool ADR [`docs/adrs/0002-v6-string-pool-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md); provisional ID lockfile [`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md); `nytprof-format-v6` preflight stack; C `nytp_sink_v6` (PR-B06..B08); E3-C fixtures (PR-B09)

---

## Purpose

Define what **dual-equality** means on the path to R2 (v6 collection opt-in) and which gates remain **open** after product COL-007 E3-EVENT. This document freezes **readiness structure**, not wire IDs.

## Maintainer OQ-1 (2026-08-11)

| Item | Decision |
|------|----------|
| ADR-0001 packing | **Accepted as-is** (proposed→accepted). Do **not** supersede. COL-007 packing implements ADR-0001 intent. |
| ADR-0002 FOOTER string-pool | **Accepted as-is** (FOOTER-local). Do **not** supersede. COL-007 dict emit implements ADR-0002 intent. |
| Wire freeze | Still **open** — provisional ID lockfile interim; freeze after E3 + E4. |

## Plan FMT-002..010 deviation (explicit)

Plan COL-007 lists dependencies **FMT-002 through FMT-010**. This program **intentionally implements COL-007 against the provisional ID lockfile**, then promotes formal freeze after E3/E4. Agents must **not** block COL-007 waiting for full FMT freeze. Details: [`V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md).

## Dual-equality classes

| Class | Meaning | Independent check |
|-------|---------|-------------------|
| **E1 — v5 semantic surfaces** | Native v5 read/report vs oracle / pure-Perl JSONL bridges on advertised fixtures (already R1-preview ready for offline scope) | offline_gate + packaging smokes; residual matrix “Advertised ready” |
| **E2 — v6 encode↔decode** | Rust (or C) encode → always-inflate decode recovers equal logical events/sites/seq/strings under packing/absolute policies | `cargo test -p nytprof-format-v6` always-inflate packing/mid-stream/multi-chunk tests |
| **E3 — C writer ↔ Rust decode** | COL-007 C emitter produces streams that Rust always-inflate path decodes with E2 equality on golden workloads | **ready (EVENT)** — C fixtures `fixtures/v6/from-c/**` + `e3_c_*` tests + `tools/oracle/e3_c_writer_parity.sh`. Stand-in harness remains engineering only. **E3-mixed residual.** |
| **E4 — v5↔v6 semantic** | Same workload profiled as v5 and v6 yields equal advertised aggregates / dump structure after normalize | **Open (runway advanced)** — policy draft shipped; **COL-014 dual-sink test/dev harness** (PR-B10a) proves same-run **logical** equality on M4 + primary-fixture-shaped streams; full oracle aggregate enforcement + fixture pairs residual |
| **E5 — CLI product path** | CLI report/verify on v6 files as product surface (opt-in, not default) | **Open** — CLI v6 default residual; opt-in path not claimed done |

## Readiness matrix

| Surface / gate | Equality class | Status | Notes |
|----------------|----------------|--------|-------|
| Offline R0 / R1-preview v5 dump/report/JSON/cross | E1 | **ready** (advertised) | Keep green; not a COL-007 substitute |
| Absolute v6 mini/multi-chunk encode↔decode | E2 | **preflight ready** | Not wire freeze |
| Packing site-delta / seq / TIME_*_RUN / multi-chunk / mid-stream continuity | E2 | **intent accepted** (ADR-0001) + preflight ready | Wire freeze still open |
| FOOTER string-dictionary resolve | E2 | **intent accepted** (ADR-0002 FOOTER-local) + preflight ready | Wire freeze still open |
| Provisional ID lockfile (Rust + C mirror) | — | **ready** (lockfile shipped B01) | Not wire freeze |
| Auto-VERSION header/body align | E2 | **preflight ready** | Full dual-output VERSION policy (OI-001-03) open |
| Default `parse_chunk_frame` inflate/CRC | — | **residual** (stays non-inflating) | Product policy ADR if default flips |
| E3 harness (writer bytes → Rust decode) | E3 | **ready** (stand-in + product C path) | Stand-in is **not** product evidence; product path is `e3_c_*` |
| C COL-007 absolute / codecs / packing / dict / mid-stream | E3 runway | **done** (PR-B06..B08 scaffold + PR-B09 E3-C) | Board COL-007 **done** for EVENT product path |
| C COL-007 product E3-EVENT fixtures | E3 | **ready** | `fixtures/v6/from-c/**`; absolute+packing+dict+mid-stream matrix |
| E3-mixed multi-kind C fixtures | E3 | **residual** | SOURCE/INDEX/SUMMARY product C matrix open |
| Batched Rust COL-008 writer | E3/E4 | **deferred** (non-baseline) | After dual-equality + ADR re-open |
| v5↔v6 semantic equality policy | E4 | **policy draft ready** | Enforcement open until fixture pairs |
| COL-014 same-run dual writer (test/dev-only, OQ-4) | E4/M6 runway | **ready (harness)** | Fan-out v5+v6; logical equality on M4 + primary-fixture-shaped streams; **not** product UX; full oracle dual residual |
| Wire freeze FMT-002..010 | — | **open** (deviated as COL-007 hard dep) | After E2/E3 evidence + freeze ADR |
| CLI v6 opt-in report/verify | E5 | **open** | After E3/E4 gates |
| CLI v6 / format default flip (R4) | E5 | **out of scope** | Field window + ADR |
| Full R1 HTML DOM / XS Data / FFI | E1 product depth | **residual** (not R1-preview claim) | Separate full-R1 work |

## Explicit open gates after COL-007 E3-EVENT

1. **E3-mixed:** multi-kind SOURCE/INDEX/SUMMARY product C fixtures (EVENT path is ready).
2. **E4 enforcement:** v5+v6 fixture pairs with documented normalize policy ([`E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md)). COL-014 dual-sink (test/dev-only) is **harness ready** for same-run logical equality; full oracle aggregate pairs residual.
3. **Wire freeze ADR** (or explicit provisional product flag with major/minor negotiation) — **not** claimed by ADR-0001/0002 or the lockfile alone.
4. **OI-001-03 / dual-output sequence-number freeze** if product requires permanent seq policy beyond preflight `FLAG_HAS_SEQ`.
5. **CLI v6 opt-in / default** (E5) — residual.
6. **COL-008** batched Rust writer remains deferred / non-baseline.

## First-slice vs full-R1 vs R0–R5 (honesty)

| Horizon | Status relative to this doc |
|---------|----------------------------|
| First-slice / offline R0 + R1-preview | **Complete** for advertised surfaces; COL-007 E3-EVENT **done**; COL-008 deferred |
| Full product R1 | **Not complete** — residual: FFI/XS Data, full nytprofhtml DOM, multi-OS CI, perf cert, R3 engine default |
| R2 v6 collection opt-in | **Runway advanced** — ADR-0001/0002 accepted; provisional ID lockfile; C writer + product E3-EVENT green; COL-014 dual-sink test/dev harness ready (OQ-4); E3-mixed / E4 enforcement / wire freeze open |
| R3–R5 defaults / retirement | **Not started** |

## Non-claims

Do **not** treat this document as:

- dual-equality product freeze or wire freeze / CLI v6 default / default-parse always-inflate;
- E3-mixed multi-kind complete;
- E4 enforcement complete;
- COL-008 done;
- full R1 HTML/XS/FFI or multi-OS CI or R3/R4 default flips.

## Evidence paths

- Residual matrix: [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md)
- Operator runbook: [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md)
- Packing ADR (accepted): [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md)
- String-pool ADR (accepted): [`docs/adrs/0002-v6-string-pool-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md)
- Provisional ID lockfile: [`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md)
- C header stub: [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h)
- E3-C schema: [`docs/schemas/collector-v6-e3-c-fixtures-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-e3-c-fixtures-mvp-v0.md)
- E4 policy: [`docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md)
- COL-014 dual-sink (test/dev-only): [`docs/schemas/collector-dual-sink-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-dual-sink-mvp-v0.md); `collector/include/nytp_sink_dual.h`; `make -C collector test` (`test_dual_sink`); smoke step 10
- E3 harness: `crates/nytprof-format-v6/src/dual_equality.rs` (engineering stand-in + `e3_decode_writer_bytes`); product `e3_c_*` in `crates/nytprof-format-v6/tests/e3_c.rs`. Evidence: `cargo test -p nytprof-format-v6 e3_c_` / `e3_harness`; `./tools/oracle/e3_c_writer_parity.sh`
- Offline gate: `./scripts/ci/offline_gate.sh` (step 11 when cargo)

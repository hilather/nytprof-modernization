# Dual-equality readiness contract (provisional) — v0

**Status:** provisional readiness checklist for R2 / COL-007 runway — **not** a dual-equality product freeze; **not** COL-007 done  
**Board ID:** `DUAL-EQUALITY-READINESS-MVP` (authoritative — contract shipped + residual honesty; no separate PROVISIONAL board row)  
**Depends on:** offline R0 / R1-preview residual matrix; **accepted** packing ADR [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md); **accepted** string-pool ADR [`docs/adrs/0002-v6-string-pool-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md); provisional ID lockfile [`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md); `nytprof-format-v6` preflight stack  
**Gate:** done **before COL-007** product claim (C v6 writer)

---

## Purpose

Define what **dual-equality** means on the path to R2 (v6 collection opt-in) and which gates remain **open** before a product COL-007 claim. This document freezes **readiness structure**, not wire IDs or writer completion.

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
| **E2 — v6 encode↔decode** | Rust (or future C) encode → always-inflate decode recovers equal logical events/sites/seq/strings under packing/absolute policies | `cargo test -p nytprof-format-v6` always-inflate tests |
| **E3 — C writer ↔ Rust decode** | COL-007 C emitter produces streams that Rust always-inflate path decodes with E2 equality on golden workloads | **Open** — C scaffold (PR-B06 absolute + PR-B07 codecs/multi-chunk/CRC via `nytp_sink_v6`) emits EVENT profiles Rust always-inflate accepts; product E3-C fixtures + board COL-007 flip remain PR-B09 |
| **E4 — v5↔v6 semantic** | Same workload profiled as v5 and v6 yields equal advertised aggregates / dump structure after normalize | **Open** — needs COL-007 + fixture pairs + policy enforcement |
| **E5 — CLI product path** | CLI report/verify on v6 files as product surface (opt-in, not default) | **Open** — CLI v6 default residual; opt-in path not claimed done |

## Readiness matrix

| Surface / gate | Equality class | Status | Open before COL-007 product claim |
|----------------|----------------|--------|-----------------------------------|
| Offline R0 / R1-preview v5 dump/report/JSON/cross | E1 | **ready** (advertised) | Keep green; not a COL-007 substitute |
| Absolute v6 mini/multi-chunk encode↔decode | E2 | **preflight ready** | Not wire freeze |
| Packing site-delta / seq / TIME_*_RUN / multi-chunk / mid-stream continuity | E2 | **intent accepted** (ADR-0001) | Preflight encode paths + wire freeze still open |
| FOOTER string-dictionary resolve | E2 | **intent accepted** (ADR-0002 FOOTER-local) | Dict preflight encode + wire freeze still open |
| Provisional ID lockfile (Rust + C mirror) | — | **ready** (lockfile shipped B01) | Not wire freeze; COL-007 implements against it |
| Auto-VERSION header/body align | E2 | **preflight ready** | Full dual-output VERSION policy (OI-001-03) open |
| Default `parse_chunk_frame` inflate/CRC | — | **residual** (stays non-inflating) | Product policy ADR if default flips |
| E3 harness (writer bytes → Rust decode) | E3 | **open / runway** | COL-007 C producer + harness |
| C COL-007 absolute MVP | E3 runway | **partial (PR-B06)** | Absolute EVENT bodies; sealed CRC default after B07 |
| C COL-007 codecs/multi-chunk/CRC | E3 runway | **partial (PR-B07)** | NONE/ZLIB/ZSTD/LZ4 + multi-chunk; not packing/dict/mid-stream switch; not E3-C product |
| C COL-007 product (board done) | E3 | **deferred** | PR-B09 after B08 packing+dict; use lockfile + ADR-0001/0002 |
| Batched Rust COL-008 writer | E3/E4 | **deferred** (non-baseline) | After dual-equality + ADR re-open |
| v5↔v6 semantic equality policy | E4 | **open** | Policy + enforcement after COL-007 |
| Wire freeze FMT-002..010 | — | **open** (deviated as COL-007 hard dep) | After E2/E3 evidence + freeze ADR |
| CLI v6 opt-in report/verify | E5 | **open** | After E3/E4 gates |
| CLI v6 / format default flip (R4) | E5 | **out of scope** | Field window + ADR |
| Full R1 HTML DOM / XS Data / FFI | E1 product depth | **residual** (not R1-preview claim) | Separate full-R1 work |

## Explicit open gates before COL-007 product claim

1. **COL-007 C v6 writer** implemented against **accepted** packing intent (ADR-0001), **accepted** string-pool intent (ADR-0002), absolute baseline, and the **provisional ID lockfile**.
2. **E3 evidence with C bytes:** feed COL-007-produced profiles into E3 harness (Rust always-inflate decode + logical equality).
3. **E4 enforcement:** v5+v6 fixture pairs with documented normalize policy.
4. **Wire freeze ADR** (or explicit provisional product flag with major/minor negotiation) — **not** claimed by ADR-0001/0002 or the lockfile alone.
5. **OI-001-03 / dual-output sequence-number freeze** if product requires permanent seq policy beyond preflight `FLAG_HAS_SEQ`.
6. Residual honesty: COL-007 board row remains **deferred** until E3 evidence with C bytes lands.

## First-slice vs full-R1 vs R0–R5 (honesty)

| Horizon | Status relative to this doc |
|---------|----------------------------|
| First-slice / offline R0 + R1-preview | **Complete** for advertised surfaces; COL-007/008 deferred |
| Full product R1 | **Not complete** — residual: FFI/XS Data, full nytprofhtml DOM, multi-OS CI, perf cert, R3 engine default |
| R2 v6 collection opt-in | **Runway** — ADR-0001/0002 **accepted**; provisional ID lockfile shipped; absolute C MVP scaffold only (PR-B06); product COL-007 / E3-C open (PR-B09) |
| R3–R5 defaults / retirement | **Not started** |

## Non-claims

Do **not** treat this document as:

- dual-equality product freeze or COL-007/COL-008 done;
- wire freeze / CLI v6 default / default-parse always-inflate;
- full R1 HTML/XS/FFI or multi-OS CI or R3/R4 default flips.

## Evidence paths

- Residual matrix: [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md)
- Operator runbook: [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md)
- Packing ADR (accepted): [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md)
- String-pool ADR (accepted): [`docs/adrs/0002-v6-string-pool-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md)
- Provisional ID lockfile: [`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md)
- C header stub: [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h)
- Crate tests: `cargo test -p nytprof-format-v6`
- Offline gate: `./scripts/ci/offline_gate.sh`

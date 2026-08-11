# Dual-equality readiness contract (provisional) — v0

**Status:** provisional readiness checklist for R2 / COL-007 runway — **not** a dual-equality product freeze; **not** COL-007 done  
**Board IDs:** `DUAL-EQUALITY-READINESS-PROVISIONAL` (contract), `DUAL-EQUALITY-READINESS-MVP` (doc shipped + residual honesty)  
**Depends on:** offline R0 / R1-preview residual matrix; packing ADR candidate [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md); `nytprof-format-v6` preflight stack  
**Gate:** done **before COL-007** product claim (C v6 writer)

---

## Purpose

Define what **dual-equality** means on the path to R2 (v6 collection opt-in) and which gates remain **open** before a product COL-007 claim. This document freezes **readiness structure**, not wire IDs or writer completion.

## Dual-equality classes

| Class | Meaning | Independent check |
|-------|---------|-------------------|
| **E1 — v5 semantic surfaces** | Native v5 read/report vs oracle / pure-Perl JSONL bridges on advertised fixtures (already R1-preview ready for offline scope) | offline_gate + packaging smokes; residual matrix “Advertised ready” |
| **E2 — v6 encode↔decode** | Rust (or future C) encode → always-inflate decode recovers equal logical events/sites/seq/strings under packing/absolute policies | `cargo test -p nytprof-format-v6` always-inflate packing/mid-stream/multi-chunk tests |
| **E3 — C writer ↔ Rust decode** | COL-007 C emitter produces streams that Rust always-inflate path decodes with E2 equality on golden workloads | **Harness shipped** (`crates/nytprof-format-v6` `dual_equality`); C writer still **deferred** |
| **E4 — v5↔v6 semantic** | Same workload profiled as v5 and v6 yields equal advertised aggregates / dump structure after normalize | **Policy draft shipped** ([`E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md)); enforcement **open** (needs COL-007 + fixture pairs) |
| **E5 — CLI product path** | CLI report/verify on v6 files as product surface (opt-in, not default) | **Open** — CLI v6 default residual; opt-in path not claimed done |

## Readiness matrix

| Surface / gate | Equality class | Status | Open before COL-007 product claim |
|----------------|----------------|--------|-----------------------------------|
| Offline R0 / R1-preview v5 dump/report/JSON/cross | E1 | **ready** (advertised) | Keep green; not a COL-007 substitute |
| Absolute v6 mini/multi-chunk encode↔decode | E2 | **preflight ready** | Not wire freeze |
| Packing site-delta / seq / TIME_*_RUN / multi-chunk / mid-stream continuity | E2 | **preflight ready** (intent in ADR-0001) | Permanent packing ADR acceptance + wire freeze still open |
| FOOTER string-dictionary resolve | E2 | **preflight ready** | Permanent string-pool ADR **proposed** (ADR-0002) |
| Auto-VERSION header/body align | E2 | **preflight ready** | Full dual-output VERSION policy (OI-001-03) open |
| Default `parse_chunk_frame` inflate/CRC | — | **residual** (stays non-inflating) | Product policy ADR if default flips |
| E3 harness (writer bytes → Rust decode) | E3 | **ready** (stand-in absolute/packing/**string-dict**/`expect_string_dict`/mid-stream packing continuity) | Stand-in is **not** product E3 evidence; COL-007 C producer still deferred |
| C COL-007 writer | E3 | **deferred** | Implementer; plug bytes into E3 harness |
| Batched Rust COL-008 writer | E3/E4 | **deferred** (non-baseline) | After dual-equality + ADR |
| v5↔v6 semantic equality policy | E4 | **policy draft ready** | Enforcement open until COL-007 + fixture pairs |
| Wire freeze FMT-002..010 | — | **open** | After E2/E3 evidence + ADR acceptance |
| CLI v6 opt-in report/verify | E5 | **open** | After E3/E4 gates |
| CLI v6 / format default flip (R4) | E5 | **out of scope** | Field window + ADR |
| Full R1 HTML DOM / XS Data / FFI | E1 product depth | **residual** (not R1-preview claim) | Separate full-R1 work |

## Explicit open gates before COL-007 product claim

1. **COL-007 C v6 writer** implemented against accepted packing intent (ADR-0001 or successor), string-pool intent (ADR-0002 or successor), and absolute baseline.
2. **E3 evidence with C bytes:** feed COL-007-produced profiles into shipped E3 harness (`dual_equality::e3_decode_writer_bytes` / `e3_assert_logical_equal`, including `expect_string_dict=true` and mid-stream packing continuity). Harness + stand-in writers are **shipped**; stand-in tests are **not** product dual-equality evidence; C producer is not.
3. **E4 enforcement:** apply [`E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md) with v5+v6 fixture pairs (policy draft **shipped**; automation open).
4. **Wire freeze ADR** (or explicit provisional product flag with major/minor negotiation) — not claimed by ADR-0001/0002 alone.
5. **OI-001-03 / dual-output sequence-number freeze** if product requires permanent seq policy beyond preflight `FLAG_HAS_SEQ`.
6. **String-pool ADR acceptance** — ADR-0002 is **proposed** (FOOTER-local); global/cross-file pool still separate open ADR if needed.
7. Residual honesty: COL-007 board row remains deferred until E3 evidence with C bytes lands.

## First-slice vs full-R1 vs R0–R5 (honesty)

| Horizon | Status relative to this doc |
|---------|----------------------------|
| First-slice / offline R0 + R1-preview | **Complete** for advertised surfaces; COL-007/008 deferred |
| Full product R1 | **Not complete** — residual: FFI/XS Data, full nytprofhtml DOM, multi-OS CI, perf cert, R3 engine default |
| R2 v6 collection opt-in | **Runway only** — packing ADR-0001 + string-pool ADR-0002 proposed; E3 harness + E4 policy shipped; no COL-007 writer |
| R3–R5 defaults / retirement | **Not started** |

## Non-claims

Do **not** treat this document as:

- dual-equality product freeze or COL-007/COL-008 done;
- wire freeze / CLI v6 default / default-parse always-inflate;
- full R1 HTML/XS/FFI or multi-OS CI or R3/R4 default flips.

## Evidence paths

- Residual matrix: [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md)
- Operator runbook: [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md)
- Packing ADR candidate: [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md)
- String-pool ADR candidate: [`docs/adrs/0002-v6-string-pool-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md)
- E4 policy: [`docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md)
- E3 harness: `crates/nytprof-format-v6/src/dual_equality.rs` — `e3_decode_writer_bytes` (`expect_string_dict`), stand-in absolute/packing/string-dict/mid-stream writers (`e3_standin_*`). Evidence: `cargo test -p nytprof-format-v6 e3_harness`. **Stand-in is not product dual-equality evidence.**
- Crate tests: `cargo test -p nytprof-format-v6`
- Offline gate: `./scripts/ci/offline_gate.sh`

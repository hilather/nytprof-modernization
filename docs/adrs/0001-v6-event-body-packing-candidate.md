# ADR-0001 - Format v6 event-body packing candidate design

- **Status:** proposed (provisional intent freeze — **not** wire freeze; **not** accepted permanent flag-bit freeze)
- **Date:** 2026-08-11
- **Owners/approvers:** format architect (proposed); pending maintainers acceptance
- **Related ADR-Q:** packing / OI-001-03 runway items in [`docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md)
- **Related tasks/risks/gates:** COL-007 runway preflights (`FMT-V6-EVENT-BODY-SITE-DELTA-*`, `FMT-V6-EVENT-BODY-SEQ-NUMBER-*`, `FMT-V6-EVENT-BODY-TIME-*-RUN-*`, `FMT-V6-*-SITE-DELTA-SEQ-*`, multi-chunk / mid-stream packing continuity); dual-equality readiness; **not** COL-007 C writer done
- **Decision scope/version:** candidate permanent packing design for format v6 EVENT bodies prior to wire freeze and COL-007 product writer

## Context

Offline R0 / R1-preview first-slice is complete for native **v5** read/report. Format v6 work in `nytprof-format-v6` is **COL-007 runway preflight only**: provisional codecs and packing forms exist with always-inflate decode tests, but wire IDs, flag bits, and C/Rust writers are **not** product-frozen.

Infinite composition of provisional packing slices does not unblock COL-007. A permanent packing ADR is required to freeze **intent** (what packing forms the product design will use) before dual-equality gates and the C v6 writer can proceed honestly.

This ADR freezes the **candidate design** drawn from shipped preflight evidence. It does **not** ratify wire bytes (FMT-002..010 class freeze) and does **not** mark COL-007/COL-008 done.

## Evidence

Preflight schemas (provisional, absolute links):

| Form | Schema |
|------|--------|
| Site deltas | [`docs/schemas/v6-event-body-site-delta-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-site-delta-provisional-v0.md) |
| Sequence numbers | [`docs/schemas/v6-event-body-seq-number-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-seq-number-provisional-v0.md) |
| TIME_LINE_RUN / TIME_BLOCK_RUN | [`docs/schemas/v6-event-body-time-line-run-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-time-line-run-provisional-v0.md), [`docs/schemas/v6-event-body-time-block-run-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-time-block-run-provisional-v0.md) |
| Site-delta + seq compose | [`docs/schemas/v6-event-body-site-delta-seq-compose-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-site-delta-seq-compose-provisional-v0.md) |
| Multi-chunk packing continuity | [`docs/schemas/v6-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-site-delta-seq-packing-provisional-v0.md) |
| TIME_*_RUN multi-chunk | [`docs/schemas/v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md) |
| Mid-stream packing continuity | [`docs/schemas/v6-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md) |

Shipped crate: `crates/nytprof-format-v6` (`PackingEncodeState`, `encode_event_body_with_site_deltas_and_seq[_continuing]`, always-inflate EVENT/mixed consumers). Gate evidence: `cargo test -p nytprof-format-v6`, `./scripts/ci/offline_gate.sh`.

## Decision

**Candidate permanent packing design (intent freeze):**

1. **Absolute baseline retained.** Default absolute EVENT body encode (no packing flags) remains valid and required for interoperability tests.

2. **Site deltas.** TIME_LINE / TIME_BLOCK / SUB_ENTRY may use provisional `FLAG_SITE_DELTA` with ZigZag site field deltas relative to a continuous `SiteCursor`. Decode reconstructs absolute sites.

3. **Logical sequence numbers.** Optional `FLAG_HAS_SEQ` + ULEB128 sequence after flags; monotonic logical sequences across expand (packed runs assign base..base+N-1). Full OI-001-03 / dual-output sequence policy freeze remains **open** (see dual-equality readiness).

4. **Packed same-site runs.** TIME_LINE_RUN and TIME_BLOCK_RUN expand to ordered absolute logical events retaining every ticks value; fail-closed empty/oversize/truncated mid-run.

5. **Compose.** Site-delta and seq may coexist on the same site-bearing record (`FLAG_SITE_DELTA | FLAG_HAS_SEQ`). Runs use absolute site on the run form + seq base only (no site-delta on run wire form); runs **advance** SiteCursor so following site-delta events are correct.

6. **Continuity.** Multi-chunk record-aligned partitions and mid-stream START_DEFLATE codec-switch regions that use packing **must** share packing state (`PackingEncodeState` / equivalent): site bases and next sequence **continue** across chunk and pre/post switch boundaries. Naive per-chunk packing reset is incorrect. Mid-run body span across chunk or switch is **not** required (record-aligned regions only).

7. **Always-inflate recovery.** Product packing decode path for v6 consumers is always-inflate join + body decode (as preflight always-inflate helpers). Default `parse_chunk_frame` remaining non-inflating is a separate residual; this ADR does **not** flip default parse policy.

8. **String pool.** FOOTER string-dictionary preflight is **orthogonal** packing of string payloads; permanent global string-pool ADR remains **open** (not decided here).

**Explicit non-decisions (still residual):**

- Wire freeze of numeric opcodes, flag bit assignments, and TLV catalogs (FMT-002..010 class).
- COL-007 C v6 writer / COL-008 batched Rust writer product claims.
- Full OI-001-03 dual-output sequence-number policy freeze.
- Permanent global string-pool / cross-file dictionary ADR.
- Default-parse always-inflate / CRC-on-by-default mutate.
- CLI v6 default; dual-equality product gate (see readiness contract).

## Exactness and compatibility consequences

- v5 read/report path is **unchanged** (oracle 6.15 remains collector).
- v6 packing is opt-in body encoding; absolute bodies remain the interoperability baseline until wire freeze + dual-equality close.
- Independent C / Rust / Perl writers must share the continuity rules in Decision §6 once accepted; until wire freeze, implementations treat schemas as provisional.

## Alternatives considered

| Alternative | Correctness/compatibility | Performance/storage | Security/reliability | Build/portability | Reason accepted/rejected |
|---|---|---|---|---|---|
| Absolute-only EVENT bodies forever | Highest interop simplicity | Larger profiles | Fewer packing edge cases | Simple | Rejected as permanent sole design — preflight shows packing continuity is workable and needed for R2 size goals |
| Pack without cross-chunk continuity | Incorrect absolute sites/seq after partition | Easy | Silent wrong analytics | — | Rejected as product design |
| Freeze wire IDs in this ADR | Premature without dual-equality / field window | — | Locks bugs into permanent IDs | — | Rejected — intent freeze only |
| Defer all packing ADR until COL-007 lands | Blocks honest R2 runway | — | Encourages infinite preflight | — | Rejected |

## Implementation and testing requirements

- Keep provisional schemas and `cargo test -p nytprof-format-v6` packing/mid-stream/multi-chunk tests green as evidence until wire freeze ADR supersedes them.
- When accepted: promote flag/opcode numeric tables via separate wire-freeze ADR; add immutable golden vectors (FMT-012 class).
- COL-007 C writer must implement Decision §1–§6 or document ADR deviation before dual-equality claim.

## Migration, rollout, and rollback

- No default format flip (R4 out of scope).
- Preflight-only profiles remain non-product; collectors stay 6.15/v5 until COL-007 + dual-equality.
- Rollback: leave status `proposed` or supersede with a new ADR if field evidence rejects packing forms.

## Revisit triggers

- Wire freeze ADR / dual-equality readiness gates closed or failed.
- Oracle or dual-equality evidence shows packing continuity bugs.
- Security finding on oversize run / delta overflow policy.
- Decision to abandon packing for absolute-only v6 EVENT bodies.

## Non-claims

This ADR does **not** mark COL-007, COL-008, wire freeze, CLI v6 default, default-parse always-inflate, full R1 HTML/XS/FFI, multi-OS CI, or R3/R4 default flips as done.

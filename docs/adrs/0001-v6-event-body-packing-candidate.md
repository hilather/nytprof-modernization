# ADR-0001 - Format v6 event-body packing candidate design

- **Status:** **accepted** (maintainer OQ-1 — as-is; **not** superseded; **not** wire freeze; **not** COL-007 done)
- **Date:** 2026-08-11
- **Accepted:** 2026-08-11 (maintainers — user OQ-1: promote proposed→accepted without superseding)
- **Owners/approvers:** format architect (authored); maintainers (accepted OQ-1)
- **Related ADR-Q:** packing / OI-001-03 runway items in [`docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md)
- **Related tasks/risks/gates:** COL-007 packing forms; dual-equality readiness E2/E3; provisional ID lockfile [`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md); **not** COL-007 C writer done
- **Decision scope/version:** permanent **intent** freeze for format v6 EVENT body packing forms prior to wire freeze and COL-007 product writer

## Context

Offline R0 / R1-preview first-slice is complete for native **v5** read/report. Format v6 work in `nytprof-format-v6` is **COL-007 runway preflight**: provisional codecs and absolute EVENT bodies exist with always-inflate decode paths, but wire IDs, flag bits, and C/Rust product writers are **not** frozen.

Infinite composition of provisional packing slices does not unblock COL-007. A permanent packing ADR freezes **intent** (what packing forms the product design will use) before dual-equality gates and the C v6 writer proceed honestly.

Maintainer decision **OQ-1** (2026-08-11): accept this ADR **as-is** (promote proposed→accepted). Do **not** supersede with a different packing model. COL-007 packing must implement Decision §1–§7.

This ADR freezes the **candidate design intent**. It does **not** ratify wire bytes (FMT-002..010 class freeze) and does **not** mark COL-007/COL-008 done. See plan **FMT-002..010 deviation** in the provisional ID lockfile: COL-007 implements against the lockfile; formal wire freeze follows E3/E4 evidence.

## Evidence

Shipped preflight (absolute EVENT + stream stack):

- Event-body opcodes (absolute): [`docs/schemas/v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md)
- Always-inflate EVENT/mixed consumers under `docs/schemas/v6-decoded-*-provisional-v0.md`
- Crate: `crates/nytprof-format-v6` — absolute `encode_event_body` / `decode_event_body`, always-inflate helpers
- Gate: `cargo test -p nytprof-format-v6`, `./scripts/ci/offline_gate.sh`

Packing forms (intent; product COL-007 / dual-equality implementers must match):

| Form | Intent |
|------|--------|
| Site deltas | `FLAG_SITE_DELTA` + ZigZag site field deltas vs continuous `SiteCursor` |
| Sequence numbers | Optional `FLAG_HAS_SEQ` + ULEB128 seq after flags |
| TIME_LINE_RUN / TIME_BLOCK_RUN | Packed same-site runs expand to ordered absolute logical events |
| Compose | Site-delta + seq may coexist on site-bearing records |
| Continuity | Multi-chunk and mid-stream codec-switch regions share packing state |

## Decision

**Permanent packing design intent (accepted):**

1. **Absolute baseline retained.** Default absolute EVENT body encode (no packing flags) remains valid and required for interoperability tests until wire freeze + dual-equality close.

2. **Site deltas.** TIME_LINE / TIME_BLOCK / SUB_ENTRY may use `FLAG_SITE_DELTA` with ZigZag site field deltas relative to a continuous `SiteCursor`. Decode reconstructs absolute sites.

3. **Logical sequence numbers.** Optional `FLAG_HAS_SEQ` + ULEB128 sequence after flags; monotonic logical sequences across expand (packed runs assign base..base+N-1). Full OI-001-03 / dual-output sequence policy freeze remains **open** (see dual-equality readiness).

4. **Packed same-site runs.** TIME_LINE_RUN and TIME_BLOCK_RUN expand to ordered absolute logical events retaining every ticks value; fail-closed empty/oversize/truncated mid-run.

5. **Compose.** Site-delta and seq may coexist on the same site-bearing record (`FLAG_SITE_DELTA | FLAG_HAS_SEQ`). Runs use absolute site on the run form + seq base only (no site-delta on run wire form); runs **advance** SiteCursor so following site-delta events are correct.

6. **Continuity.** Multi-chunk record-aligned partitions and mid-stream START_DEFLATE codec-switch regions that use packing **must** share packing state (`PackingEncodeState` / equivalent): site bases and next sequence **continue** across chunk and pre/post switch boundaries. Naive per-chunk packing reset is incorrect. Mid-run body span across chunk or switch is **not** required (record-aligned regions only).

7. **Always-inflate recovery.** Product packing decode path for v6 consumers is always-inflate join + body decode. Default `parse_chunk_frame` remaining non-inflating is a separate residual; this ADR does **not** flip default parse policy.

8. **String pool.** FOOTER string-dictionary is **orthogonal** (see accepted ADR-0002). Permanent global/cross-file string-pool remains a **separate open ADR** if product requires it.

**Explicit non-decisions (still residual):**

- Wire freeze of numeric opcodes, flag bit assignments, and TLV catalogs (FMT-002..010 class) — use provisional ID lockfile until E3/E4 + wire-freeze ADR.
- COL-007 C v6 writer / COL-008 batched Rust writer product claims.
- Full OI-001-03 dual-output sequence-number policy freeze.
- Permanent global string-pool / cross-file dictionary ADR.
- Default-parse always-inflate / CRC-on-by-default mutate.
- CLI v6 default; dual-equality product gate (see readiness contract).

## Exactness and compatibility consequences

- v5 read/report path is **unchanged** (oracle 6.15 remains collector until COL-007 + dual-equality).
- v6 packing is opt-in body encoding; absolute bodies remain the interoperability baseline until wire freeze + dual-equality close.
- Independent C / Rust / Perl writers must share the continuity rules in Decision §6 once productized; until wire freeze, numeric IDs follow the provisional ID lockfile (not a permanent freeze claim).

## Alternatives considered

| Alternative | Correctness/compatibility | Performance/storage | Security/reliability | Build/portability | Reason accepted/rejected |
|---|---|---|---|---|---|
| Absolute-only EVENT bodies forever | Highest interop simplicity | Larger profiles | Fewer packing edge cases | Simple | Rejected as permanent sole design — packing continuity is required for R2 size goals |
| Pack without cross-chunk continuity | Incorrect absolute sites/seq after partition | Easy | Silent wrong analytics | — | Rejected as product design |
| Freeze wire IDs in this ADR | Premature without dual-equality / field window | — | Locks bugs into permanent IDs | — | Rejected — intent freeze only; lockfile interim |
| Supersede with alternate packing model | — | — | — | — | **Rejected by OQ-1** — accept as-is |
| Defer all packing ADR until COL-007 lands | Blocks honest R2 runway | — | Encourages infinite preflight | — | Rejected |

## Implementation and testing requirements

- COL-007 C writer **must** implement Decision §1–§7 (or document an explicit ADR deviation before any dual-equality product claim).
- Promote flag/opcode numeric tables via separate **wire-freeze ADR** after E3 (C bytes) + E4 evidence; add immutable golden vectors (FMT-012 class).
- Keep `cargo test -p nytprof-format-v6` and offline gate green for absolute + packing preflights as they land.
- Provisional ID lockfile is the shared numeric source for COL-007 until wire freeze.

## Migration, rollout, and rollback

- No default format flip (R4 out of scope).
- Collectors stay 6.15/v5 until COL-007 + dual-equality.
- Rollback: supersede with a new ADR only if field evidence rejects packing forms (OQ-1 already rejected preemptive supersede).

## Revisit triggers

- Wire freeze ADR / dual-equality readiness gates closed or failed.
- Oracle or dual-equality evidence shows packing continuity bugs.
- Security finding on oversize run / delta overflow policy.
- Decision to abandon packing for absolute-only v6 EVENT bodies (requires superseding ADR).

## Non-claims

This ADR does **not** mark COL-007, COL-008, wire freeze, CLI v6 default, default-parse always-inflate, full R1 HTML/XS/FFI, multi-OS CI, or R3/R4 default flips as done.

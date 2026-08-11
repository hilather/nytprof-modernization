# ADR-0002 - Format v6 FOOTER string-pool / dictionary candidate design

- **Status:** **accepted** (maintainer OQ-1 — as-is; **not** superseded; FOOTER-local only; **not** wire freeze; **not** COL-007 done)
- **Date:** 2026-08-11
- **Accepted:** 2026-08-11 (maintainers — user OQ-1: promote proposed→accepted without superseding)
- **Owners/approvers:** format architect (authored); maintainers (accepted OQ-1)
- **Related ADR-Q:** string dictionary / FMT-008 class items in [`docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md)
- **Related tasks/risks/gates:** FOOTER dict emit for COL-007; dual-equality readiness E2/E3; packing ADR [`0001`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md); provisional ID lockfile; **not** COL-007 done
- **Decision scope/version:** permanent **intent** freeze for FOOTER-local string dictionary design prior to wire freeze and COL-007 product writer

## Context

Dual-equality readiness requires a permanent string-pool ADR before COL-007 product claim when FOOTER dict is productized. Preflight design uses a **local** FOOTER string-dictionary that resolves non-zero `string_id` on EVENT string-blobs (MARK, COMMENT, ATTRIBUTE keys/values, etc.).

Maintainer decision **OQ-1** (2026-08-11): accept this ADR **as-is** (FOOTER-local model). Do **not** supersede with a global/cross-file pool in this record. COL-007 dict emit must implement Decision §1–§5.

This ADR freezes **intent** for the FOOTER-local dictionary model. It does **not** decide a process-wide / cross-file intern pool, wire freeze of table layout, or COL-007 completion. Plan **FMT-002..010 deviation** still applies: implement against the provisional ID lockfile; formal freeze after E3/E4.

## Evidence

- Length-prefixed string-blob frame: [`docs/schemas/v6-string-blob-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-blob-provisional-v0.md)
- FOOTER body role: [`docs/schemas/v6-footer-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-footer-body-provisional-v0.md)
- Crate: `crates/nytprof-format-v6` string-blob + FOOTER body APIs (dictionary encode/decode/resolve land with packing/dict preflights and COL-007)
- Gate: `cargo test -p nytprof-format-v6`

## Decision

**Permanent FOOTER-local string dictionary (accepted intent):**

1. **Local table packaging.** The dictionary is carried as a FOOTER chunk payload (codec **NONE** for the table packaging preflight) after EVENT/SOURCE/INDEX/SUMMARY regions as required by stream layout.

2. **Wire table layout (candidate).**
   ```text
   entry_count : ULEB128
   entry*      : id ULEB128 || flags u8 || byte_length ULEB128 || bytes
   ```
   `id == 0` reserved for inline-only blobs — **not** a dictionary key. Duplicate ids **fail-closed**. Payload caps match string-blob limits; total table payload cap as preflight (64 MiB).

3. **Resolution policy.**
   | `string_id` | Result |
   |-------------|--------|
   | `0` | Use inline blob bytes |
   | non-zero, present | Use dictionary payload (inline may be empty) |
   | non-zero, missing | **Err** (`UnknownId`) |

4. **Orthogonality to packing.** Site-delta / seq / TIME_*_RUN packing (ADR-0001) is independent of string interning. Compose paths may combine both; string ids do not participate in site cursors.

5. **Scope limit.** This ADR freezes **FOOTER-local, single-profile** dictionary semantics. A **global / cross-file / process-lifetime** intern pool (COL-010 class) remains a **separate open ADR** if product requires it.

**Explicit non-decisions:**

- Wire freeze of FOOTER kind id / flags / multi-FOOTER policy (provisional ID lockfile until wire-freeze ADR).
- Cross-file or multi-process dictionary inheritance (fork/reset policy).
- COL-007 C writer completion; CLI v6 default; default-parse always-inflate.
- Permanent global pool (not accepted here).

## Exactness and compatibility consequences

- v5 path unchanged.
- Until wire freeze, FOOTER dict remains provisional numerically (lockfile IDs); always-inflate + resolve is the product-shaped decode path for dict profiles.
- C and Rust writers must share resolution policy once productized.

## Alternatives considered

| Alternative | Reason |
|-------------|--------|
| Inline strings only forever | Rejected for R2 size goals on repeated names |
| Global process pool in this ADR | Premature; fork/reset and packaging cost undecided — separate ADR |
| Supersede FOOTER-local with alternate model | **Rejected by OQ-1** — accept as-is |
| Defer all string-pool ADR until COL-007 | Blocks dual-equality open-gate clarity |

## Implementation and testing requirements

- COL-007 C writer must emit FOOTER-local dict when enabled and honor resolution policy.
- E3 harness must accept FOOTER-dict profiles when C writer emits them.
- Promote numeric FOOTER layout via wire-freeze ADR after E3/E4.
- Keep `cargo test -p nytprof-format-v6` string-dict + compose tests green as they land.

## Migration, rollout, and rollback

- No default format flip.
- Rollback: supersede only with a new ADR if field evidence rejects FOOTER-local model (OQ-1 rejected preemptive supersede).

## Revisit triggers

- COL-010 global pool decision; wire freeze; dual-equality E3/E4 failures on dict profiles; memory-limit security findings.

## Non-claims

Does **not** mark COL-007/008, wire freeze, CLI v6 default, global string pool, or full R1 product residuals as done.

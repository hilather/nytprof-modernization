# ADR-0002 - Format v6 FOOTER string-pool / dictionary candidate design

- **Status:** proposed (provisional intent freeze — **not** wire freeze; **not** permanent cross-file pool acceptance)
- **Date:** 2026-08-11
- **Owners/approvers:** format architect (proposed); pending maintainers acceptance
- **Related ADR-Q:** string dictionary / FMT-008 class items in [`docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md)
- **Related tasks/risks/gates:** `FMT-V6-STRING-DICTIONARY-*`, FOOTER dict compose preflights, dual-equality readiness E2/E3; packing ADR [`0001`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md); **not** COL-007 done
- **Decision scope/version:** candidate permanent FOOTER-local string dictionary design prior to wire freeze and COL-007 product writer

## Context

Preflight ships a **local** FOOTER string-dictionary that resolves non-zero `string_id` on EVENT string-blobs (MARK, COMMENT, ATTRIBUTE keys/values, etc.). Dual-equality readiness lists a permanent string-pool ADR as an open gate before COL-007 product claim when FOOTER dict is productized.

This ADR freezes **intent** for the FOOTER-local dictionary model. It does **not** decide a process-wide / cross-file intern pool, wire freeze of table layout, or COL-007 completion.

## Evidence

- Schema: [`docs/schemas/v6-string-dictionary-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-dictionary-provisional-v0.md)
- Compose preflights: string-dict + packing / multi-chunk / mid-stream schemas under `docs/schemas/v6-*-string-dict-*-provisional-v0.md`
- Crate: `crates/nytprof-format-v6` (`encode_string_dictionary`, `decode_string_dictionary`, `resolve_event_records`, `*_with_string_dict` always-inflate helpers)
- Gate: `cargo test -p nytprof-format-v6`

## Decision

**Candidate permanent FOOTER-local string dictionary (intent freeze):**

1. **Local table packaging.** The dictionary is carried as a FOOTER chunk payload (codec **NONE** for the table packaging preflight) after EVENT/SOURCE/INDEX/SUMMARY regions as required by stream layout.

2. **Wire table layout (candidate).** Matches preflight:
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

- Wire freeze of FOOTER kind id / flags / multi-FOOTER policy.
- Cross-file or multi-process dictionary inheritance (fork/reset policy).
- COL-007 C writer completion; CLI v6 default; default-parse always-inflate.
- Permanent packing ADR acceptance (see ADR-0001).

## Exactness and compatibility consequences

- v5 path unchanged.
- Until wire freeze, FOOTER dict remains provisional; always-inflate + resolve is the product-shaped decode path for dict profiles.
- C and Rust writers must share resolution policy once accepted.

## Alternatives considered

| Alternative | Reason |
|-------------|--------|
| Inline strings only forever | Rejected for R2 size goals on repeated names |
| Global process pool in this ADR | Premature; fork/reset and packaging cost undecided |
| Defer all string-pool ADR until COL-007 | Blocks dual-equality open-gate clarity |

## Implementation and testing requirements

- Keep `cargo test -p nytprof-format-v6` string-dict + compose tests green.
- E3 harness must accept FOOTER-dict profiles when C writer emits them.
- When accepted: promote numeric FOOTER layout via wire-freeze ADR.

## Migration, rollout, and rollback

- No default format flip.
- Rollback: leave `proposed` or supersede if field evidence rejects FOOTER-local model.

## Revisit triggers

- COL-010 global pool decision; wire freeze; dual-equality E3/E4 failures on dict profiles; memory-limit security findings.

## Non-claims

Does **not** mark COL-007/008, wire freeze, CLI v6 default, global string pool, or full R1 product residuals as done.

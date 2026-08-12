# ADR-0006 - Format v6 wire freeze (numeric IDs + core layouts)

- **Status:** **accepted**
- **Date:** 2026-08-11
- **Accepted:** 2026-08-11 (after E3-EVENT with C bytes + E4-v0 model-level equality)
- **Owners/approvers:** format architect; maintainers
- **Related ADR-Q:** plan FMT-002..010 class; design OQ-5 (OI-001-03 seq policy); design OQ-6 (global string pool)
- **Related tasks/risks/gates:** FMT-002..010 / FMT-012; provisional ID lockfile promotion; dual-equality E3-EVENT (PR-B09) + E4-v0 (PR-B10); ADR-0001 packing intent; ADR-0002 FOOTER-local dict; **not** COL-008; **not** CLI v6 default; **not** E3-mixed
- **Decision scope/version:** permanent freeze of format **v6 major=6** on-wire numeric IDs and the core layout contracts already shipped as provisional preflight

## Context

Plan tasks **FMT-002 through FMT-010** historically gated COL-007. This program **deviated** intentionally: COL-007 implemented against the provisional ID lockfile ([`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md)), then promotes freeze **after** dual-equality evidence.

Preconditions (satisfied):

| Gate | Evidence |
|------|----------|
| ADR-0001 packing intent accepted | OQ-1 |
| ADR-0002 FOOTER-local string dict accepted | OQ-1 |
| Provisional ID lockfile (Rust + C) | PR-B01 |
| E3-EVENT with **C** bytes | PR-B09 — `fixtures/v6/from-c/**`, `e3_c_*` |
| E4-v0 model-level v5↔v6 aggregates | PR-B10 — dual-sink pairs + `e4_v0_aggregates_equal` |

Freezing earlier (Rust-only preflight) was **rejected**. Freezing after E3-EVENT(C) + E4-v0 is the design rule (Key Decision 4 / B.5).

## Evidence

- C header mirror: [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h)
- Rust constants: `crates/nytprof-format-v6` (`MAGIC`, `chunk::{kind,codec}`, `event_body::opcode`, flags, `tlv::type_id`, caps)
- Product E3 C fixtures: [`fixtures/v6/from-c/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/from-c/)
- Immutable golden vectors (this freeze): [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/)
- Frozen ID catalog: [`docs/schemas/v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md)
- E4-v0: [`docs/schemas/e4-v0-model-semantic-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-v0-model-semantic-mvp-v0.md)

## Decision

### 1. Numeric IDs are frozen for major=6

All values catalogued in the provisional ID lockfile at freeze time are **permanent** for `SUPPORTED_MAJOR = 6`. Changing any frozen ID requires a **new major** (or a superseding ADR with explicit migration). Values:

| Domain | Frozen values |
|--------|----------------|
| Magic | ASCII `NYTPROF6` (8 bytes) |
| Chunk sync | ASCII `NYT6` as `u32 LE` (`0x3654594E`) |
| Chunk kinds | 0 RESERVED, 1 EVENT, 2 SOURCE, 3 INDEX, 4 SUMMARY, 5 FOOTER |
| Codecs | 0 NONE, 1 ZLIB, 2 ZSTD, 3 LZ4 |
| Event opcodes | 0–19 as lockfile (incl. TIME_LINE_RUN=18, TIME_BLOCK_RUN=19) |
| Event flags | `0x01` required opcode, `0x02` body length, `0x04` site delta, `0x08` has seq |
| String-blob | `FLAG_UTF8=0x01` |
| TLV type_id | 0 RESERVED, 1 PRODUCER, 2 TICKS_PER_SEC, `0x7E` END |
| Caps | header 1 MiB; chunk payload 64 MiB; event body 64 MiB; string 16 MiB; TLV value 16 MiB; TLV region 64 MiB; TIME_*_RUN N ≤ 1_048_576; dict entries ≤ 1_048_576; dict total 64 MiB |

Full tables: [`docs/schemas/v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md). Rust and C mirrors **must** stay aligned in the same change set.

### 2. Core layouts elevated from provisional → frozen (ID + frame contracts)

The following layout contracts are **frozen for major=6** at the level needed for independent C/Rust agreement (byte order little-endian; ULEB128/ZigZag as shipped):

| Area | Normative preflight schema (still the detailed layout home) | Freeze class |
|------|-------------------------------------------------------------|--------------|
| Fixed header | `v6-fixed-header-provisional-v0.md` | **frozen IDs + 36-byte full header** |
| Chunk frame | `v6-chunk-frame-provisional-v0.md` | **frozen IDs + 40-byte header** |
| ULEB128 / ZigZag | `v6-varint-uleb128-provisional-v0.md`, `v6-svarint-zigzag-provisional-v0.md` | **frozen algorithms** |
| String-blob | `v6-string-blob-provisional-v0.md` | **frozen frame** |
| Header TLV / region | `v6-header-tlv-provisional-v0.md`, `v6-tlv-region-provisional-v0.md` | **frozen known type_ids + END** |
| Event body opcodes | `v6-event-body-provisional-v0.md` + packing siblings | **frozen opcodes/flags**; packing intent ADR-0001 |
| FOOTER string dict table | `v6-string-dictionary-provisional-v0.md` + ADR-0002 | **frozen table layout** (FOOTER-local) |

Schema filenames may retain `provisional-v0` for link stability; **status banners** and this ADR mark the ID/frame freeze. New feature bits / type_ids / opcodes for major=6 require minor negotiation or a superseding ADR — reserved zero and unknown-required fail-closed rules remain.

### 3. OQ-5 — permanent dual-output sequence policy (OI-001-03)

**Resolved for major=6:**

1. `FLAG_HAS_SEQ` (`0x08`) is a **permanent optional** flag bit: when set, a ULEB128 logical sequence number follows flags and precedes the typed body.
2. Absolute encode **without** `FLAG_HAS_SEQ` remains valid and is the interoperability baseline for records that do not need dual-output seq.
3. When a dual-output / COL-003 gapless sequence is emitted, **all logical recovered events** in the EVENT stream — including dump-visible prelude **VERSION** and control **START_DEFLATE** — **may** carry `FLAG_HAS_SEQ` and **must** use the same monotonic numbering space as TIME_* / SUB_* samples. Packed TIME_*_RUN expands to N logical seqs from a base.
4. Sequence participation is therefore **optional per profile** but **uniform when present**: decoders must not invent seqs; dual-equality uses recovered seq when flags set.
5. Default product absolute writers may omit seq; packing writers (ADR-0001) may include site-delta + seq compose.

This closes design **OQ-5** for wire freeze. Full COL-003 lifecycle ownership remains a collector concern; this ADR freezes **wire meaning** only.

### 4. OQ-6 — global string pool (COL-010)

**Deferred** beyond this freeze. Product major=6 string interning remains **FOOTER-local, single-profile** per ADR-0002. A process-wide / cross-file pool requires a **separate ADR** and must not renumber frozen FOOTER kind, string-blob flags, or dictionary table layout without a new major.

### 5. Immutable golden vectors (FMT-012 class)

Committed under [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/):

- Primitive frames (ULEB, ZigZag, fixed header, empty EVENT chunk, TLV region, string dict table)
- Event bodies (TIME_LINE, DISCOUNT, TIME_LINE_RUN, site-delta+seq, dual-output sequence)
- Mini absolute profile (prefix + EVENT NONE)

Product E3 C streams under `fixtures/v6/from-c/**` remain **cross-language product evidence** (not re-encoded by Rust stand-ins). Regenerating vectors requires dual-path evidence and must not silently change frozen IDs.

### 6. Plan FMT-002..010 deviation closed for freeze class

COL-007 product E3-EVENT already shipped against the provisional lockfile. This ADR **promotes** that lockfile to **frozen** status for major=6 and records FMT-002..010 class acceptance for the frozen catalog above. Residual **E3-mixed**, full oracle E4, E4 product CLI smoke, convert/merge, CLI v6 **default**, and COL-008 remain out of scope.

## Exactness and compatibility consequences

- v5 path unchanged.
- Existing C writer and Rust decoder must continue to interoperate on frozen IDs (E3 harness).
- New major is required to renumber opcodes/kinds/codecs/flags.
- Default-parse always-inflate is **not** flipped by this ADR (product consumers use always-inflate helpers / model ingest explicitly).
- CLI v6 opt-in report surfaces remain PR-B12; collection default remains v5 until R4 ADR.

## Alternatives considered

| Alternative | Reason |
|-------------|--------|
| Freeze from Rust-only preflight before E3/E4 | **Rejected** — locks bugs into permanent IDs |
| Keep IDs provisional indefinitely after E3+E4-v0 | **Rejected** — blocks convert tooling and R2-stable |
| Supersede packing / FOOTER dict models | **Rejected** — OQ-1 accepted ADR-0001/0002 as-is |
| Require FLAG_HAS_SEQ on every record | **Rejected** — absolute baseline without seq remains valid |
| Freeze global string pool now | **Rejected** — OQ-6 deferred; ADR-0002 FOOTER-local only |

## Implementation and testing requirements

1. Promote lockfile document status to **frozen** (historical path retained).
2. Align C header + Rust comments with ADR-0006; keep numeric values identical.
3. Commit `fixtures/v6/vectors/**` + SHA256SUMS; Rust tests load and decode vectors.
4. Board / residual matrix / dual-equality readiness / runbook: wire freeze **done**; residual honesty for E3-mixed / CLI default / COL-008.
5. Do not mark CLI v6 default or full R1 DOM/FFI from this ADR alone.

## Migration, rollout, and rollback

- Files already produced by COL-007 against the provisional lockfile remain valid under the freeze (IDs unchanged).
- Rollback of the **claim** is documentation-only if a critical wire bug is found before R2-stable; a **true** ID change still requires major bump.
- Feature negotiation continues via header required/optional feature bits (all-zero default remains valid).

## Revisit triggers

- Discovery that C and Rust disagree on a frozen vector after independent reimplementation.
- Security issue requiring layout change (prefer fail-closed + major bump).
- Product need for global string pool (new ADR; not silent renumber).
- OI-002 full ATTRIBUTE/OPTION key vocabulary freeze (separate from numeric IDs).

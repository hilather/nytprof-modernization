# Format v6 ID lockfile — v0 (promoted to frozen)

**Status:** **frozen** for `SUPPORTED_MAJOR = 6` — promoted by [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md) (2026-08-11)  
**Historical name:** path retains `V6_PROVISIONAL_ID_LOCKFILE_v0` for stable links; values are **no longer provisional**  
**Authoritative frozen catalog:** [`docs/schemas/v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md)  
**Board IDs:** `FMT-V6-PROVISIONAL-ID-LOCKFILE` (runway, done); `FMT-V6-WIRE-FREEZE` (promotion, done)  
**Depends on:** accepted ADR-0001, ADR-0002; E3-EVENT(C); E4-v0  
**Gate:** COL-007 implemented against this lockfile; **formal wire freeze accepted** after E3 + E4-v0

---

## Purpose

Provide a single, language-mirrored catalog of v6 numeric IDs so independent C and Rust implementers share the same constants.

| This lockfile is | This lockfile is **not** |
|------------------|--------------------------|
| Shared MAGIC / major / chunk kind / codec / opcode / flag / TLV ids | CLI v6 default or default-parse always-inflate |
| Implementation baseline that COL-007 shipped against | E3-mixed multi-kind product claim |
| **Frozen** major=6 IDs (ADR-0006) | COL-008 batched Rust writer |
| Historical path for pre-freeze links | Permission to renumber without major bump |

## Plan FMT-002..010 deviation (closed for freeze class)

Plan task **COL-007** listed dependencies **FMT-002 through FMT-010**. This program **implemented COL-007 against the provisional catalog**, then **promoted** formal freeze after dual-equality **E3** (C writer bytes → Rust always-inflate) and **E4-v0** (model-level v5↔v6 semantic).

1. ~~Implement COL-007 against provisional IDs~~ **done** (PR-B06..B09).
2. ~~Do not block COL-007 on full FMT freeze~~ **done**.
3. **Promote** formal wire freeze — **done** (ADR-0006 + golden vectors PR-B11).
4. Freezing stable IDs from Rust-only preflight before E3/E4 remains **rejected** (historical rule).

Related: dual-equality readiness [`DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md); freeze ADR [`0006-v6-wire-freeze.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md).

## Source of truth (mirror pair)

| Language | Path |
|----------|------|
| Rust (crate constants) | `crates/nytprof-format-v6` (`MAGIC`, `SUPPORTED_MAJOR`, `chunk::{kind,codec}`, `event_body::opcode`, flags, `tlv::type_id`, …) |
| C header (COL-007) | [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h) |

**Rule:** frozen IDs must not diverge between Rust and C. Any future major bump updates **both** in the same change set and supersedes ADR-0006.

Collector tree path (`collector/`) is the overlay layout (ADR-0004 / PR-B00).

## Locked constants (identical to freeze v1)

Values match `nytprof-format-v6` / `nytprof_v6_ids.h`. All multi-byte integers on wire are **little-endian** unless noted (ULEB128 for varints).

### Fixed header

| Constant | Value | Rust |
|----------|------:|------|
| `MAGIC` | ASCII `NYTPROF6` (8 bytes) | `nytprof_format_v6::MAGIC` |
| `SUPPORTED_MAJOR` | `6` | `SUPPORTED_MAJOR` |
| `HEADER_LEN_MIN` | `16` | `HEADER_LEN_MIN` |
| `HEADER_LEN_FULL` | `36` | `HEADER_LEN_FULL` |
| `MAX_HEADER_LEN` | `1 MiB` | `MAX_HEADER_LEN` |

### Chunk frame

| Constant | Value | Rust |
|----------|------:|------|
| `CHUNK_SYNC` | ASCII `NYT6` as `u32 LE` (`0x3654594E`) | `chunk::CHUNK_SYNC` |
| `CHUNK_HEADER_LEN` | `40` | `CHUNK_HEADER_LEN` |
| `MAX_CHUNK_PAYLOAD` | `64 MiB` | `MAX_CHUNK_PAYLOAD` |
| `FLAG_KIND_REQUIRED` | `0x0001` | `FLAG_KIND_REQUIRED` |

#### Chunk kinds (`chunk::kind`)

| Value | Name |
|------:|------|
| 0 | `RESERVED` (invalid) |
| 1 | `EVENT` |
| 2 | `SOURCE` |
| 3 | `INDEX` |
| 4 | `SUMMARY` |
| 5 | `FOOTER` |

#### Codecs (`chunk::codec`)

| Value | Name |
|------:|------|
| 0 | `NONE` |
| 1 | `ZLIB` |
| 2 | `ZSTD` |
| 3 | `LZ4` |

### Event-body opcodes (`event_body::opcode`)

| Value | Name |
|------:|------|
| 0 | `RESERVED` |
| 1 | `MARK` |
| 2 | `TIME_LINE` |
| 3 | `TIME_BLOCK` |
| 4 | `SUB_ENTRY` |
| 5 | `SUB_RETURN` |
| 6 | `SUB_INFO` |
| 7 | `SRC_LINE` |
| 8 | `NEW_FID` |
| 9 | `PID_START` |
| 10 | `PID_END` |
| 11 | `SUB_CALLERS` |
| 12 | `DISCOUNT` |
| 13 | `ATTRIBUTE` |
| 14 | `OPTION` |
| 15 | `COMMENT` |
| 16 | `START_DEFLATE` |
| 17 | `VERSION` |
| 18 | `TIME_LINE_RUN` (ADR-0001 packing) |
| 19 | `TIME_BLOCK_RUN` (ADR-0001 packing) |

### Event-body flags

| Bit | Name | Notes |
|----:|------|-------|
| `0x01` | `FLAG_OPCODE_REQUIRED` | Unknown opcode → Err |
| `0x02` | `FLAG_BODY_LENGTH` | Length-framed unknown optional skip |
| `0x04` | `FLAG_SITE_DELTA` | ADR-0001 site deltas |
| `0x08` | `FLAG_HAS_SEQ` | OQ-5 permanent optional seq (ADR-0006 §3) |

### String-blob

| Constant | Value |
|----------|------:|
| `FLAG_UTF8` | `0x01` |
| `MAX_STRING_BYTES` | `16 MiB` |

### Header TLV type ids (`tlv::type_id`)

| Value | Name |
|------:|------|
| 0 | `RESERVED` |
| 1 | `PRODUCER` |
| 2 | `TICKS_PER_SEC` |
| `0x7e` | `END` (region terminator) |

| Flag | Value |
|------|------:|
| `FLAG_TYPE_REQUIRED` | `0x01` |

### Caps (fail-closed before large alloc)

| Cap | Value |
|-----|------:|
| Event body | `64 MiB` |
| TLV value | `16 MiB` |
| TLV region | `64 MiB` |
| TIME_*_RUN length `N` | `1_048_576` |

## ADR alignment

| ADR | Status | Lockfile role |
|-----|--------|---------------|
| [ADR-0001](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md) | **accepted** | Packing forms use `FLAG_SITE_DELTA` / `FLAG_HAS_SEQ` / TIME_*_RUN opcodes |
| [ADR-0002](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md) | **accepted** | FOOTER kind `5` + string-blob flags; FOOTER-local dict |
| [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md) | **accepted** | Promotes this catalog to permanent major=6 freeze |

## Promotion path (completed)

1. E3: COL-007 C bytes feed Rust always-inflate decode with logical equality — **done** (PR-B09).
2. E4-v0: v5↔v6 semantic equality on dual-sink fixtures — **done** (PR-B10).
3. Wire-freeze ADR + FMT-012 golden vectors — **done** (PR-B11 / ADR-0006).
4. This document remains the stable link target; frozen tables also live in `v6-wire-ids-frozen-v1.md`.

## Non-claims

- Not CLI v6 default; not default-parse always-inflate.
- Not multi-OS CI, full R1 HTML/XS/FFI, or performance certification.
- Not E3-mixed multi-kind C fixtures; not full oracle E4 product smoke.
- Not COL-008; not global string pool (OQ-6 deferred).

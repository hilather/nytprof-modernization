# Format v6 provisional ID lockfile — v0

**Status:** provisional shared numeric constants for COL-007 runway — **not** a v6 wire freeze (not FMT-002..010 ratification)  
**Board IDs:** `FMT-V6-PROVISIONAL-ID-LOCKFILE`  
**Depends on:** accepted ADR-0001 (packing intent), accepted ADR-0002 (FOOTER-local string dict), `nytprof-format-v6` preflight constants  
**Gate:** done **before COL-007** product implementation (C writer implements against this lockfile)

---

## Purpose

Provide a single, language-mirrored catalog of **provisional** v6 numeric IDs so independent C and Rust implementers share the same constants **without** claiming a permanent wire freeze.

| This lockfile is | This lockfile is **not** |
|------------------|--------------------------|
| Shared MAGIC / major / chunk kind / codec / opcode / flag / TLV ids from shipped preflight | FMT-002..010 class ratification |
| Implementation baseline for COL-007 C writer | Permission to mark COL-007 **done** |
| Documented plan deviation for COL-007 dependencies | Immutable golden freeze (FMT-012) |
| Interim until E3 (C bytes) + E4 evidence | CLI v6 default or default-parse always-inflate |

## Plan FMT-002..010 deviation (explicit)

Plan task **COL-007** lists dependencies **FMT-002 through FMT-010** (format freeze class) in [`docs/plan/05_COLLECTOR_AND_C_XS_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/05_COLLECTOR_AND_C_XS_TASKS.md).

**This program intentionally deviates:**

1. **Implement COL-007 against this provisional ID lockfile** (and accepted ADR-0001 / ADR-0002 intent).
2. **Do not block** COL-007 on full FMT-002..010 wire freeze.
3. **Promote** formal wire freeze (new ADR + immutable vectors) **after** dual-equality **E3** (C writer bytes → Rust always-inflate decode) and **E4** (v5↔v6 semantic) evidence.
4. Freezing stable v6 wire IDs from Rust-only preflight before E3/E4 is **rejected** (locks bugs into permanent IDs).

Agents must **not** wait for FMT freeze to start COL-007 against the lockfile. Agents must **not** claim wire freeze when only this lockfile exists.

Related: dual-equality readiness [`DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md).

## Source of truth (mirror pair)

| Language | Path |
|----------|------|
| Rust (shipped crate constants) | `crates/nytprof-format-v6` (`MAGIC`, `SUPPORTED_MAJOR`, `chunk::{kind,codec}`, `event_body::opcode`, flags, `tlv::type_id`, …) |
| C header stub (COL-007) | [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h) |

**Rule:** when provisional IDs change before wire freeze, update **both** the Rust crate and the C header in the same change set, then refresh this document’s tables if values move.

Collector tree path (`collector/`) is the preferred overlay layout (PR-B00 packaging ADR). If packaging relocates collector sources, move the header with the tree and keep this absolute link current.

## Locked provisional constants (v0)

Values match `nytprof-format-v6` at acceptance of this lockfile. All multi-byte integers on wire are **little-endian** unless noted (ULEB128 for varints).

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
| 18 | `TIME_LINE_RUN` (ADR-0001 packing; provisional) |
| 19 | `TIME_BLOCK_RUN` (ADR-0001 packing; provisional) |

### Event-body flags

| Bit | Name | Notes |
|----:|------|-------|
| `0x01` | `FLAG_OPCODE_REQUIRED` | Unknown opcode → Err |
| `0x02` | `FLAG_BODY_LENGTH` | Length-framed unknown optional skip |
| `0x04` | `FLAG_SITE_DELTA` | ADR-0001 site deltas (provisional) |
| `0x08` | `FLAG_HAS_SEQ` | ADR-0001 / OI-001-03 seq field (provisional) |

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
| TIME_*_RUN length `N` | `1_048_576` (provisional packing) |

## ADR alignment

| ADR | Status | Lockfile role |
|-----|--------|---------------|
| [ADR-0001](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md) | **accepted** | Packing forms use `FLAG_SITE_DELTA` / `FLAG_HAS_SEQ` / TIME_*_RUN opcodes above |
| [ADR-0002](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md) | **accepted** | FOOTER kind `5` + string-blob flags; dict table layout per ADR (not renumbered here) |

## Promotion path (after E3/E4)

1. E3: COL-007 C bytes feed Rust always-inflate decode with logical equality.
2. E4: v5↔v6 semantic equality on primary fixtures (policy + automation).
3. Wire-freeze ADR + FMT-012 immutable vectors promote selected IDs from this lockfile to permanent.
4. This document remains historical or is superseded by the freeze ADR index.

## Non-claims

- Not COL-007/008 done; not CLI v6 default; not default-parse always-inflate.
- Not multi-OS CI, full R1 HTML/XS/FFI, or performance certification.
- Opcode 18/19 and packing flags are **reserved** in crate (`event_body`) + C header; packing encode/decode remains residual until packing preflight / COL-007.

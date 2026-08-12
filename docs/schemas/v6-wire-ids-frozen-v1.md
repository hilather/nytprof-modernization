# Format v6 wire IDs — frozen v1

**Status:** **frozen** for `SUPPORTED_MAJOR = 6` by [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md)  
**Board IDs:** `FMT-V6-WIRE-FREEZE`, `FMT-V6-GOLDEN-VECTORS`  
**Supersedes (status only):** provisional catalog in [`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md) — path retained; values identical  
**Depends on:** E3-EVENT(C) PR-B09; E4-v0 PR-B10; ADR-0001; ADR-0002  
**Gate:** after dual-equality E3 + E4-v0; **before** convert tooling (PR-C01) and R2-stable

---

## Purpose

Authoritative **frozen** numeric ID catalog for independent C and Rust implementations. Layout detail remains in the historical `docs/schemas/v6-*-provisional-v0.md` documents; those schemas’ **numeric IDs and core frames** are elevated by ADR-0006 even when filenames retain `provisional-v0` for link stability.

| This document is | This document is **not** |
|------------------|--------------------------|
| Permanent major=6 ID freeze | Permission to flip CLI v6 default |
| FMT-002..010 class ratification for catalogued IDs | E3-mixed multi-kind C fixture claim |
| Index for golden vectors under `fixtures/v6/vectors/` | COL-008 batched Rust writer |
| OQ-5 seq policy + OQ-6 deferral pointer | Full OI-002 ATTRIBUTE/OPTION vocabulary |

## Source of truth (mirror pair)

| Language | Path |
|----------|------|
| Rust | `crates/nytprof-format-v6` |
| C | [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h) |

**Rule:** ID changes for major=6 are **forbidden** without ADR supersession + major bump. Comments may say “frozen”; values must match.

## Fixed header

| Constant | Value | Rust |
|----------|------:|------|
| `MAGIC` | ASCII `NYTPROF6` (8 bytes) | `MAGIC` |
| `SUPPORTED_MAJOR` | `6` | `SUPPORTED_MAJOR` |
| `HEADER_LEN_MIN` | `16` | `HEADER_LEN_MIN` |
| `HEADER_LEN_FULL` | `36` | `HEADER_LEN_FULL` |
| `MAX_HEADER_LEN` | `1 MiB` | `MAX_HEADER_LEN` |

Little-endian multi-byte integers. Detailed field map: [`v6-fixed-header-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-fixed-header-provisional-v0.md).

## Chunk frame

| Constant | Value | Rust |
|----------|------:|------|
| `CHUNK_SYNC` | `0x3654594E` (`NYT6` LE) | `chunk::CHUNK_SYNC` |
| `CHUNK_HEADER_LEN` | `40` | `CHUNK_HEADER_LEN` |
| `MAX_CHUNK_PAYLOAD` | `64 MiB` | `MAX_CHUNK_PAYLOAD` |
| `FLAG_KIND_REQUIRED` | `0x0001` | `FLAG_KIND_REQUIRED` |

### Kinds

| Value | Name |
|------:|------|
| 0 | RESERVED |
| 1 | EVENT |
| 2 | SOURCE |
| 3 | INDEX |
| 4 | SUMMARY |
| 5 | FOOTER |

### Codecs

| Value | Name |
|------:|------|
| 0 | NONE |
| 1 | ZLIB |
| 2 | ZSTD |
| 3 | LZ4 |

## Event-body opcodes

| Value | Name |
|------:|------|
| 0 | RESERVED |
| 1 | MARK |
| 2 | TIME_LINE |
| 3 | TIME_BLOCK |
| 4 | SUB_ENTRY |
| 5 | SUB_RETURN |
| 6 | SUB_INFO |
| 7 | SRC_LINE |
| 8 | NEW_FID |
| 9 | PID_START |
| 10 | PID_END |
| 11 | SUB_CALLERS |
| 12 | DISCOUNT |
| 13 | ATTRIBUTE |
| 14 | OPTION |
| 15 | COMMENT |
| 16 | START_DEFLATE |
| 17 | VERSION |
| 18 | TIME_LINE_RUN |
| 19 | TIME_BLOCK_RUN |

### Flags

| Bit | Name | Frozen meaning |
|----:|------|----------------|
| `0x01` | `FLAG_OPCODE_REQUIRED` | Unknown opcode → Err |
| `0x02` | `FLAG_BODY_LENGTH` | Length-framed unknown optional skip |
| `0x04` | `FLAG_SITE_DELTA` | ZigZag site deltas (ADR-0001) |
| `0x08` | `FLAG_HAS_SEQ` | ULEB seq after flags (OQ-5 / ADR-0006 §3) |

## String-blob / dictionary

| Constant | Value |
|----------|------:|
| `FLAG_UTF8` | `0x01` |
| `MAX_STRING_BYTES` | `16 MiB` |
| Dict table | `entry_count` ULEB + entries `id || flags u8 || len ULEB || bytes` |
| `id == 0` | Reserved for inline-only |
| `MAX_DICT_ENTRIES` | `1_048_576` |
| `MAX_DICT_TOTAL_BYTES` | `64 MiB` |

FOOTER-local packaging per ADR-0002. Global pool (OQ-6 / COL-010) **deferred**.

## Header TLV

| Value | Name |
|------:|------|
| 0 | RESERVED |
| 1 | PRODUCER |
| 2 | TICKS_PER_SEC |
| `0x7e` | END |

| Flag | Value |
|------|------:|
| `FLAG_TYPE_REQUIRED` | `0x01` |

## Caps (fail-closed before large alloc)

| Cap | Value |
|-----|------:|
| Event body | `64 MiB` |
| TLV value | `16 MiB` |
| TLV region | `64 MiB` |
| TIME_*_RUN `N` | `1_048_576` |

## Golden vectors

| Path | Role |
|------|------|
| [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/) | Immutable primitive / event / mini-profile vectors |
| [`fixtures/v6/from-c/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/from-c/) | Product E3 C streams (cross-language) |

Tests: `cargo test -p nytprof-format-v6 wire_freeze_` · `cargo test -p nytprof-format-v6 golden_vector_`

## Residual honesty

- **E3-mixed** multi-kind SOURCE/INDEX/SUMMARY product C fixtures still residual.
- **CLI v6 default** / collection default format remain residual (R4).
- **Default-parse always-inflate** not flipped.
- **COL-008** deferred non-baseline.
- Full oracle E4 pairs + E4 product offline_gate residual (PR-B12b).
- Complete OI-002 key vocabulary not frozen here.

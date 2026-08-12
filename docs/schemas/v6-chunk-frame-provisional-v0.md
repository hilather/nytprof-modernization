# Format v6 chunk frame (provisional) — v0

**Status:** numeric IDs / core frame **frozen** for major=6 by [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md) + catalog [`v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md); filename retains `provisional-v0` for link stability; **not** CLI v6 default / E3-mixed / COL-008  
**Board IDs:** `FMT-V6-CHUNK-PROVISIONAL` (contract), `FMT-V6-CHUNK-PARSE-MVP` (shipped parse + tests)  
**Depends on:** plan draft [`docs/plan/04_FILE_FORMAT_V6_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/04_FILE_FORMAT_V6_TASKS.md) §4; fixed-header preflight [`v6-fixed-header-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-fixed-header-provisional-v0.md)  
**Gate:** IDs frozen after E3-EVENT(C)+E4-v0 (ADR-0006). Residual: CLI v6 default; default-parse non-inflate; E3-mixed; COL-008; full OI-002 vocabulary  

---

## Scope and non-claims

This document is the detailed layout home for **chunk frame (40-byte header + kind/codec ids)**. Numeric IDs and the core frame described here are **frozen for major=6** by ADR-0006 (see frozen catalog). Filename retains `provisional-v0` for stable links.

It is **not**:

- permission to flip CLI v6 / collection default (still v5 until R4 ADR);
- E3-mixed multi-kind product C fixture claim;
- COL-008 batched Rust writer;
- default-parse always-inflate / CRC default flip;
- complete OI-002 ATTRIBUTE/OPTION key vocabulary;
- a new major without ADR supersession (renumbering requires major bump).

Independent C/Rust implementations must match the frozen IDs and this layout. Golden vectors: [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/).

## Chunk frame layout (little-endian)

All multi-byte integers are **little-endian**.

### Fixed chunk header (40 bytes)

| Offset | Size | Field | Type | Notes |
|-------:|-----:|-------|------|-------|
| 0 | 4 | Sync word | `u32 LE` | Resync marker (see below) |
| 4 | 1 | Chunk kind | `u8` | Event / source / index / summary / footer / … |
| 5 | 1 | Codec | `u8` | None / zlib / zstd / LZ4 / reserved |
| 6 | 2 | Flags | `u16 LE` | Required-kind bit and provisional flags |
| 8 | 8 | Sequence | `u64 LE` | Monotonic chunk sequence |
| 16 | 8 | First logical event seq | `u64 LE` | Canonical ordering anchor |
| 24 | 4 | Logical event count | `u32 LE` | Exact expansion count (0 if N/A) |
| 28 | 4 | Uncompressed length | `u32 LE` | Decoded payload size (bound allocation) |
| 32 | 4 | Compressed length | `u32 LE` | On-wire payload size immediately after this header |
| 36 | 4 | Payload checksum | `u32 LE` | Placeholder; **MVP parse does not verify** |

**`CHUNK_HEADER_LEN` = 40.**

**On-wire frame** = fixed chunk header ‖ payload of exactly `compressed_length` bytes.

### Sync word (provisional)

```text
ASCII bytes (in file order): N Y T 6
u32 LE value:                0x3654594E
```

Constant: `CHUNK_SYNC` / `nytprof_format_v6::chunk::CHUNK_SYNC`.

### Provisional chunk kinds

| Value | Name | Notes |
|------:|------|-------|
| 0 | reserved | Invalid as kind → **Err** |
| 1 | `EVENT` | Event stream chunk (payload opaque in this MVP) |
| 2 | `SOURCE` | Source blob |
| 3 | `INDEX` | Index |
| 4 | `SUMMARY` | Optional summary |
| 5 | `FOOTER` | Footer |
| other | unknown | See required-kind flag |

### Provisional codecs

| Value | Name |
|------:|------|
| 0 | `NONE` (payload is raw / already uncompressed) |
| 1 | `ZLIB` |
| 2 | `ZSTD` |
| 3 | `LZ4` |
| other | unknown codec — **recorded only**; body inflate is residual |

### Flags

| Bit | Name | Meaning |
|----:|------|---------|
| 0 (`0x0001`) | `FLAG_KIND_REQUIRED` | If set and `kind` is not a known provisional kind → **Err** (fail closed). If clear, unknown kind → **Ok** with `known_kind = false` (skip-with-honesty for future readers). |

Other flag bits are reserved (must be accepted without error in this MVP).

### Length bounds (fail closed)

| Rule | Result |
|------|--------|
| `buf.len() < 40` | **Err** truncated (cannot read full chunk header) |
| sync ≠ `CHUNK_SYNC` | **Err** bad sync |
| `kind == 0` | **Err** invalid kind |
| `compressed_length > MAX_CHUNK_PAYLOAD` (64 MiB provisional) | **Err** oversize compressed length |
| `uncompressed_length > MAX_CHUNK_PAYLOAD` | **Err** oversize uncompressed length |
| `buf.len() < 40 + compressed_length` | **Err** truncated (payload not fully present) |
| unknown kind **and** `FLAG_KIND_REQUIRED` | **Err** unknown required kind |
| otherwise | **Ok** |

No payload allocation is required to parse the frame: the MVP only **bounds-checks** declared lengths against the input slice.

---

## Fail-closed policy

- never panic on crafted byte vectors in the shipped parse path;
- never treat bad sync / truncated / oversize / unknown **required** kind as a valid frame;
- payload checksum is **not** verified in `FMT-V6-CHUNK-PARSE-MVP` (placeholder).

---

## Shipped parse entry point

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::chunk::{parse_chunk_frame, ChunkFrame, CHUNK_SYNC, CHUNK_HEADER_LEN};

let frame: ChunkFrame = parse_chunk_frame(&bytes)?;
// frame.payload is a subslice of the input (length = compressed_length)
```

- Pure byte-slice API (no filesystem I/O).
- Tests craft vectors in-process: valid minimal frame (codec none, empty payload), bad sync, truncated, oversize length, unknown required kind.

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status under this schema |
|------|--------------------------|
| Provisional fixed header | done (`FMT-V6-HEADER-*`) |
| Provisional chunk frame contract | **done** (`FMT-V6-CHUNK-PROVISIONAL`) |
| Shipped fail-closed chunk parse + tests | **done** (`FMT-V6-CHUNK-PARSE-MVP`) |
| C v6 writer (COL-007) | **still deferred** |
| Payload codecs / event stream / dictionaries | residual (FMT-002..010) |
| COL-008 batched Rust writer | residual |
| CLI default v6 report/dump | not claimed |

---

## Open items (honest residual)

1. ADR ratification of sync word / kind / codec enums (FMT-002).
2. Payload checksum verification (CRC32C or stronger).
3. Payload inflate and event opcode decode.
4. Dictionary / delta state reset rules across chunks.
5. Immutable golden byte vectors (FMT-012) after freeze.

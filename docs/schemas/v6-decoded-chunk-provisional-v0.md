# Format v6 decoded-chunk consumer path (always inflate + optional CRC) — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-DECODED-CHUNK-PROVISIONAL` (contract), `FMT-V6-DECODED-CHUNK-MVP` (shipped encode/decode + tests)  
**Depends on:** chunk-frame parse; payload codecs NONE/ZLIB/ZSTD/LZ4; optional payload CRC verify  
**Gate:** COL-007 runway preflight only — **before** dictionaries / C v6 writer / CLI v6 default

---

## Scope and non-claims

This document freezes a **provisional consumer path** that always recovers **plain body bytes** from a sealed chunk:

```text
wire bytes
  → parse_chunk_frame          # non-inflating, non-CRC (unchanged)
  → [optional] verify_chunk_payload_crc on on-wire payload
  → decode_chunk_payload       # NONE identity / ZLIB / ZSTD / LZ4 inflate
  → plain body
```

Rules:

- Codecs **NONE / ZLIB / ZSTD / LZ4** (ids 0–3) are supported on this path.
- CRC verify is **optional** (`verify_crc: bool`); when enabled, it checks the **on-wire** payload before inflate.
- Low-level default **`parse_chunk_frame` stays non-inflating and non-CRC-verifying** (wire-level frame parse only).
- Composition is pure byte-slice / `Vec` — no I/O.

It is **not**:

- a change to default `parse_chunk_frame` semantics;
- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- dictionaries; dual-equality vs C; certified perf claims.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Bad sync / truncated frame | **Err** (chunk layer) |
| CRC mismatch when `verify_crc` | **Err** (CRC layer) |
| Corrupt compressed payload | **Err** (payload layer) |
| Inflated len ≠ `uncompressed_len` | **Err** (`SizeMismatch`) |
| Unsupported codec | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::decoded_chunk::{decode_chunk, decode_chunk_frame_plain};
use nytprof_format_v6::chunk::parse_chunk_frame;

// Buffer path: parse + optional CRC + always inflate.
let (decoded, n) = decode_chunk(&wire, /* verify_crc */ true)?;
assert_eq!(decoded.plain, expected_plain);

// Frame path (after non-inflating parse).
let frame = parse_chunk_frame(&wire)?;
let plain = decode_chunk_frame_plain(&frame, true)?;
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Wire-level `parse_chunk_frame` (non-inflating) | prior `FMT-V6-CHUNK-*` — **unchanged** |
| Explicit payload inflate helpers | prior `FMT-V6-PAYLOAD-*` |
| Optional payload CRC | prior `FMT-V6-CRC-*` |
| Always-inflate consumer path + optional CRC | **done** (`FMT-V6-DECODED-CHUNK-*`) |
| Multi-chunk always-inflate stream | **done** separately (`FMT-V6-DECODED-STREAM-*`) |
| Default parse mutate to always inflate | **not** done (compat residual) |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. Multi-chunk stream consumer: sibling `FMT-V6-DECODED-STREAM-*`.
2. Mutating default `parse_chunk_frame` to inflate/CRC in place (explicitly out of this preflight).
2. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.
3. Full COL-007 C writer / COL-008 batched Rust writer.

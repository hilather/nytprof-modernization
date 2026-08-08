# Format v6 decoded prefix+chunk stream (always inflate + optional CRC) — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-DECODED-STREAM-PROVISIONAL` (contract), `FMT-V6-DECODED-STREAM-MVP` (shipped encode/decode + tests)  
**Depends on:** prefix+chunk stream; decoded-chunk consumer (`FMT-V6-DECODED-CHUNK-*`); payload codecs NONE/ZLIB/ZSTD/LZ4; optional payload CRC  
**Gate:** COL-007 runway preflight only — **before** dictionaries / C v6 writer / CLI v6 default

---

## Scope and non-claims

This document freezes a **provisional multi-chunk consumer path** over a file-prefix + chunk stream that always recovers **ordered plain body bytes** per chunk:

```text
wire bytes
  → decode_prefix_chunk_stream     # non-inflating frames (unchanged)
  → for each frame:
        [optional] verify_chunk_payload_crc
        → decode_chunk_payload     # NONE / ZLIB / ZSTD / LZ4
  → ordered DecodedChunk.plain list
```

Rules:

- Codecs **NONE / ZLIB / ZSTD / LZ4** (ids 0–3) are supported per chunk.
- CRC verify is **optional** (`verify_crc: bool`) and applies **per chunk** on the on-wire payload before inflate.
- Low-level default **`parse_chunk_frame` / `decode_prefix_chunk_stream` stay non-inflating** (wire-level only).
- Composition reuses shipped decoded-chunk helpers; pure byte-slice / `Vec` — no I/O.

It is **not**:

- a change to default `parse_chunk_frame` semantics;
- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- dictionaries; dual-equality vs C; certified perf claims.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Bad magic / truncated prefix | **Err** (stream/prefix) |
| Truncated mid-chunk after prefix | **Err** (stream/chunk) |
| CRC mismatch when `verify_crc` | **Err** (decoded-chunk CRC) |
| Corrupt compressed payload | **Err** (payload) |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::decoded_stream::{
    decode_prefix_chunk_stream_plain, encode_prefix_sealed_chunks,
};

let wire = encode_prefix_sealed_chunks(6, 0, 0, 0, 0, &[], &[&frame0, &frame1])?;
let (stream, n) = decode_prefix_chunk_stream_plain(&wire, /* verify_crc */ true)?;
// stream.chunks[i].plain is always-inflated body for chunk i
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Prefix+chunk stream (non-inflating) | prior `FMT-V6-PREFIX-CHUNK-STREAM-*` |
| Single-chunk always-inflate + optional CRC | prior `FMT-V6-DECODED-CHUNK-*` |
| Multi-chunk always-inflate stream consumer | **done** (`FMT-V6-DECODED-STREAM-*`) |
| Logical EVENT recovery over stream | **done** separately (`FMT-V6-DECODED-EVENT-*`) |
| Default parse mutate to always inflate | **not** done (compat residual) |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. Mutating default `parse_chunk_frame` to inflate/CRC in place (explicitly out of this preflight).
2. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.
3. Full COL-007 C writer / COL-008 batched Rust writer.

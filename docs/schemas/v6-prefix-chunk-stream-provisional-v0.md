# Format v6 prefix + chunk stream (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-PREFIX-CHUNK-STREAM-PROVISIONAL` (contract), `FMT-V6-PREFIX-CHUNK-STREAM-MVP` (shipped compose encode/decode + tests)  
**Depends on:** file prefix [`v6-file-prefix-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-file-prefix-provisional-v0.md); chunk frame [`v6-chunk-frame-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-chunk-frame-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** payload inflate / event codecs / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** minimal v6 **file layout** composition:

```text
[ file prefix ][ chunk frame ][ chunk frame ]…
```

where **file prefix** = fixed header + multi-TLV region ending with **END**, and each **chunk frame** is the existing 40-byte header + payload (`v6-chunk-frame-provisional-v0.md`).

It is **not**:

- a permanent wire freeze or ADR-ratified FMT file layout;
- permission to mark **COL-007** (C v6 writer) or **COL-008** done;
- payload inflate (zlib / zstd / LZ4), event opcodes, dictionaries, or location deltas;
- header / payload **CRC verification** freeze;
- streaming I/O, mmap readers, or default CLI report/dump of v6 profiles.

Layout and framing may change under future ADR + golden vectors.

---

## Layout

| Region | Content | Schema |
|--------|---------|--------|
| File prefix | Fixed header + multi-TLV … **END** | `v6-file-prefix-provisional-v0.md` |
| Chunk stream | Zero or more 40-byte frames + payload | `v6-chunk-frame-provisional-v0.md` |

### Provisional rules (MVP)

| Rule | Detail |
|------|--------|
| Empty stream | Valid prefix with **zero** chunks is **Ok** (no footer required) |
| Codec | **NONE** (`codec = 0`) only for this MVP; other codec ids may be recorded on frames but bodies are **not** inflated |
| Payload | Opaque bytes of length `compressed_len`; no event decode |
| Exhaustion | After the prefix, walk frames until the buffer is fully consumed |
| No trailing garbage | Leftover bytes that do not form a complete frame → fail closed |

### Fail-closed composition

| Condition | Result |
|-----------|--------|
| Bad magic / unsupported major / truncated prefix / missing END | **Err** from file-prefix parse |
| Truncated mid-chunk (after valid prefix) | **Err** from chunk parse (`Truncated`) |
| Bad chunk sync (`NYT6` mismatch) | **Err** from chunk parse (`BadSync`) |
| Oversize / reserved kind / unknown **required** kind | **Err** from existing chunk rules |
| Never panic on crafted streams | Required |

Header CRC and payload checksum remain **placeholders** (not verified).

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::stream::{
    encode_prefix_chunk_stream, decode_prefix_chunk_stream, ChunkSpec,
};
use nytprof_format_v6::chunk::{kind, codec};

let bytes = encode_prefix_chunk_stream(
    6, 0, 0, 0, 0,
    &[],
    &[ChunkSpec {
        kind: kind::EVENT,
        codec: codec::NONE,
        flags: 0,
        sequence: 0,
        first_logical_seq: 0,
        logical_event_count: 0,
        uncompressed_len: 4,
        payload: b"data",
        payload_checksum: 0,
    }],
);
let (stream, n) = decode_prefix_chunk_stream(&bytes)?;
// stream.prefix, stream.chunks; n == bytes.len()
```

- Composes **shipped** `encode_file_prefix` / `decode_file_prefix` and `encode_chunk_frame` / `parse_chunk_frame`.
- Pure byte-slice / `Vec` APIs (no I/O). Not wired into default CLI.

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Fixed header + multi-TLV + file-prefix | done |
| Single chunk frame parse/encode | done |
| Prefix + chunk stream composition | **done** (`FMT-V6-PREFIX-CHUNK-STREAM-*`) |
| Event-body opcode codec (codec NONE payload) | **done** separately (`FMT-V6-EVENT-BODY-*`) |
| Payload inflate / full event catalog / dictionaries | residual |
| C v6 writer (**COL-007**) | **still deferred** |
| Full freeze / CRC verify / golden full-file corpus | residual |

---

## Open items (honest residual)

1. ADR freeze of full-file layout (footer requirement, padding, multi-section files).
2. Payload inflate (zlib/zstd/LZ4); event-body opcode preflight is `FMT-V6-EVENT-BODY-*` (not full catalog).
3. Header / payload CRC verification.
4. String dictionaries, location deltas, dual-equality vs C encoder.
5. Golden full-file corpus (FMT-012) and default CLI v6 read path.

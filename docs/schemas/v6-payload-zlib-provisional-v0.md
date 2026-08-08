# Format v6 payload codec ZLIB (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-PAYLOAD-ZLIB-PROVISIONAL` (contract), `FMT-V6-PAYLOAD-ZLIB-MVP` (shipped deflate/inflate + tests)  
**Depends on:** chunk frame [`v6-chunk-frame-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-chunk-frame-provisional-v0.md); optional CRC [`v6-crc-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-crc-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** zstd/LZ4 / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** rule for chunk **codec ZLIB** (`codec = 1`):

- On-wire chunk payload bytes are **zlib-compressed** (zlib wrapper around DEFLATE).
- `uncompressed_len` is the exact inflated size and **bounds** inflate allocation / verification.
- Codec **NONE** (`0`) remains identity: wire payload equals uncompressed body.
- Default `parse_chunk_frame` stays **non-inflating** (compat); inflate is an **explicit** helper.

It is **not**:

- a permanent wire freeze or product default CLI v6 path;
- permission to mark **COL-007** / **COL-008** done;
- payload codecs **zstd** / **LZ4**;
- streaming inflate across chunks or dictionary compression;
- multi-OS CI or public performance claims.

---

## Codec rules (MVP)

| Codec | Wire payload | Decode helper |
|------:|--------------|---------------|
| `NONE` (0) | Raw / already uncompressed | Identity; require `uncompressed_len == payload.len()` |
| `ZLIB` (1) | zlib stream | Inflate; require inflated len == `uncompressed_len` |
| `ZSTD` / `LZ4` / other | residual | **Err** `UnsupportedCodec` on this MVP path |

### Fail-closed inflate

| Condition | Result |
|-----------|--------|
| Corrupt zlib stream | **Err** |
| Inflated length ≠ declared `uncompressed_len` | **Err** (`SizeMismatch`) |
| `uncompressed_len` > 64 MiB (`MAX_CHUNK_PAYLOAD`) | **Err** (`Oversize`) |
| Never panic | Required |

### CRC note (provisional)

When sealing a ZLIB chunk via shipped helpers, `payload_checksum` is CRC32 of the **on-wire compressed** payload (same scope as CRC preflight). Optional; default parse still does not verify CRC.

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::payload_codec::{
    deflate_zlib, inflate_zlib, decode_chunk_payload, encode_chunk_frame_zlib,
};
use nytprof_format_v6::{parse_chunk_frame, chunk::kind};

let wire = deflate_zlib(b"hello")?;
let plain = inflate_zlib(&wire, 5)?;

let frame_bytes = encode_chunk_frame_zlib(kind::EVENT, 0, 0, 0, 0, b"event-body")?;
let frame = parse_chunk_frame(&frame_bytes)?;
let body = decode_chunk_payload(&frame)?; // inflates when codec=ZLIB
```

- Uses workspace **`flate2`** (same as v5). Pure byte-slice / `Vec` APIs (no I/O).
- Not wired into default CLI.

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Chunk codec id space (NONE/ZLIB/…) | prior frame preflight |
| ZLIB deflate/inflate + chunk composition | **done** (`FMT-V6-PAYLOAD-ZLIB-*`) |
| zstd / LZ4 | residual |
| Always-on inflate in default parse | residual (compat: explicit only) |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. ADR freeze of compression levels and checksum scope for compressed payloads.
2. zstd / LZ4 codecs.
3. Streaming multi-chunk inflate and dictionary modes.
4. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.

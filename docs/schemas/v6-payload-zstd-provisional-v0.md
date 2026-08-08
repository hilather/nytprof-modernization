# Format v6 payload codec ZSTD (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-PAYLOAD-ZSTD-PROVISIONAL` (contract), `FMT-V6-PAYLOAD-ZSTD-MVP` (shipped compress/decompress + tests)  
**Depends on:** chunk frame [`v6-chunk-frame-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-chunk-frame-provisional-v0.md); ZLIB sibling [`v6-payload-zlib-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-payload-zlib-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** dictionaries / always-on inflate / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** rule for chunk **codec ZSTD** (`codec = 2`):

- On-wire chunk payload bytes are a **Zstandard frame** (standard zstd frame, no dictionary required for this MVP).
- `uncompressed_len` is the exact inflated size and **bounds** inflate allocation / verification.
- Codec **NONE** (`0`) remains identity; **ZLIB** (`1`) and **LZ4** (`3`) are separate codec paths.
- Default `parse_chunk_frame` stays **non-inflating** (compat); inflate is an **explicit** helper.

It is **not**:

- a permanent wire freeze or product default CLI v6 path;
- permission to mark **COL-007** / **COL-008** done;
- zstd **dictionaries**, multi-frame streams, or certified perf claims;
- always-on inflate inside default `parse_chunk_frame`;
- multi-OS CI or dual-equality vs C.

---

## Codec rules (MVP)

| Codec | Id | Wire payload | Decode helper |
|------:|---:|---------------|---------------|
| `NONE` | 0 | Raw / already uncompressed | Identity; require `uncompressed_len == payload.len()` |
| `ZLIB` | 1 | zlib stream | See ZLIB preflight |
| `ZSTD` | 2 | zstd frame | Inflate; require inflated len == `uncompressed_len` |
| `LZ4` | 3 | LZ4 block | See LZ4 preflight |
| other | — | residual | **Err** `UnsupportedCodec` |

### Fail-closed inflate

| Condition | Result |
|-----------|--------|
| Corrupt zstd stream | **Err** |
| Inflated length ≠ declared `uncompressed_len` | **Err** (`SizeMismatch`) |
| `uncompressed_len` > 64 MiB (`MAX_CHUNK_PAYLOAD` / `MAX_INFLATE_BYTES`) | **Err** (`Oversize`) |
| Never panic | Required |

### CRC note (provisional)

When sealing a ZSTD chunk via shipped helpers, `payload_checksum` is CRC32 of the **on-wire compressed** payload (same scope as CRC preflight). Optional; default parse still does not verify CRC.

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::payload_codec::{
    compress_zstd, decompress_zstd, decode_chunk_payload, encode_chunk_frame_zstd,
};
use nytprof_format_v6::{parse_chunk_frame, chunk::kind};

let wire = compress_zstd(b"hello")?;
let plain = decompress_zstd(&wire, 5)?;

let frame_bytes = encode_chunk_frame_zstd(kind::EVENT, 0, 0, 0, 0, b"event-body")?;
let frame = parse_chunk_frame(&frame_bytes)?;
let body = decode_chunk_payload(&frame)?; // inflates when codec=ZSTD
```

- Uses workspace **`zstd`**. Pure byte-slice / `Vec` APIs (no I/O).
- Not wired into default CLI; default `parse_chunk_frame` does not inflate.

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Chunk codec id `ZSTD = 2` | prior frame preflight |
| ZSTD compress/decompress + chunk composition | **done** (`FMT-V6-PAYLOAD-ZSTD-*`) |
| Dictionaries / always-on default inflate | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. ADR freeze of compression levels, dictionaries, and checksum scope for compressed payloads.
2. Always-on inflate in default parse (compat residual: explicit only).
3. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.

# Format v6 CRC32 (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-CRC-PROVISIONAL` (contract), `FMT-V6-CRC-MVP` (shipped compute/verify + tests)  
**Depends on:** fixed header [`v6-fixed-header-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-fixed-header-provisional-v0.md); chunk frame [`v6-chunk-frame-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-chunk-frame-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** permanent CRC algorithm ADR / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** **CRC-32/IEEE** (ISO-HDLC / zlib polynomial) for:

1. **Fixed-header `header_crc`** — checksum over a documented header byte range excluding the CRC field itself.
2. **Chunk `payload_checksum`** — checksum over **payload bytes only** (not the 40-byte chunk header).

It is **not**:

- a permanent irrevocable CRC algorithm ADR or full FMT freeze;
- permission to mark **COL-007** / **COL-008** done;
- mandatory always-on verify for all CLI / default decode paths (parse remains non-verifying for compat);
- payload inflate (zlib / zstd / LZ4);
- multi-OS CI or public performance claims.

---

## Algorithm (provisional)

| Item | Value |
|------|--------|
| Name | CRC-32/IEEE (ISO-HDLC, zlib) |
| Polynomial (reflected) | `0xEDB88320` |
| Init | `0xFFFFFFFF` |
| XorOut | `0xFFFFFFFF` |
| Check (`"123456789"`) | `0xCBF43926` |
| Empty input | `0` |

Shipped: `crc32_ieee(data: &[u8]) -> u32`.

---

## Header CRC covered range

For a full provisional fixed header (`HEADER_LEN_FULL = 36`):

| Item | Rule |
|------|------|
| Covered bytes | `[0, 32)` = `magic … optional_features` |
| Stored field | `header_crc` at offset 32 (4 bytes LE) |
| Excluded | The 4-byte `header_crc` field itself |

Helpers: `compute_header_crc`, `fill_header_crc`, `verify_header_crc`, `encode_fixed_header_full_sealed`.

Default `parse_fixed_header` still does **not** verify CRC (compat). Call `verify_header_crc` explicitly for fail-closed mismatch.

---

## Chunk payload CRC scope

| Item | Rule |
|------|------|
| Covered bytes | Chunk payload only (`compressed_len` bytes after the 40-byte chunk header) |
| Stored field | `payload_checksum` at offset 36 of the chunk header (4 bytes LE) |
| Excluded | Chunk header bytes (sync, kind, codec, lengths, …) |

Helpers: `compute_payload_crc`, `verify_payload_crc`, `verify_chunk_payload_crc`, `encode_chunk_frame_sealed`.

Default `parse_chunk_frame` still does **not** verify the checksum (compat).

---

## Fail-closed optional verify

| Condition | Result |
|-----------|--------|
| CRC match | **Ok** |
| CRC mismatch | **Err** (`Mismatch`) |
| Truncated header for verify | **Err** (`Truncated` / `HeaderTooShort`) |
| Never panic | Required |

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::crc::{
    crc32_ieee, fill_header_crc, verify_header_crc,
    compute_payload_crc, verify_payload_crc,
    encode_fixed_header_full_sealed, encode_chunk_frame_sealed,
    verify_chunk_payload_crc,
};

let mut hdr = nytprof_format_v6::encode_fixed_header_full(6, 0, 0, 0, 0);
fill_header_crc(&mut hdr)?;
verify_header_crc(&hdr)?;

let frame = encode_chunk_frame_sealed(/* kind, codec, … */, b"payload");
let parsed = nytprof_format_v6::parse_chunk_frame(&frame)?;
verify_chunk_payload_crc(&parsed)?;
```

- Pure byte-slice / `Vec` APIs (no I/O). Not wired into default CLI.
- Pure-Rust implementation (no extra crate dependency).

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Header/chunk CRC fields present as placeholders | prior preflight |
| CRC32 compute + optional verify | **done** (`FMT-V6-CRC-*`) |
| Always-on default decode verify | residual (compat keeps non-verify parse) |
| ZLIB payload codec | **done** separately (`FMT-V6-PAYLOAD-ZLIB-*`) |
| Permanent CRC ADR / dual-equality vs C | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. ADR freeze of CRC algorithm and covered ranges for all sections.
2. Optional always-on verify flag for profile readers.
3. TLV-region / multi-section CRCs if required by future freeze.
4. Dual-equality vs C encoder + FMT-012 golden corpus.

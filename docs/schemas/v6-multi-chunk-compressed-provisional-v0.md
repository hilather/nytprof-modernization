# Format v6 multi-chunk EVENT with compressed payloads (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-MULTI-CHUNK-COMPRESSED-PROVISIONAL` (contract), `FMT-V6-MULTI-CHUNK-COMPRESSED-MVP` (shipped encode/decode + tests)  
**Depends on:** multi-chunk EVENT [`v6-multi-chunk-event-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-event-provisional-v0.md); compressed mini-profile [`v6-compressed-profile-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-compressed-profile-provisional-v0.md); payload codecs ZLIB/ZSTD/LZ4  
**Gate:** COL-007 runway preflight only — **before** always-on inflate / dictionaries / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** composition path that splits an ordered event-body record stream across **one or more** EVENT chunks whose payloads may use:

| Codec | Id | Per-chunk wire payload |
|------:|---:|------------------------|
| `NONE` | 0 | Raw `encode_event_body(partition)` |
| `ZLIB` | 1 | zlib of that partition body |
| `ZSTD` | 2 | zstd frame of that partition body |
| `LZ4` | 3 | LZ4 raw block of that partition body |

Layout:

```text
[file prefix][EVENT codec C, partition 0…][EVENT codec C, partition 1…]…[optional FOOTER codec NONE]
```

Rules:

- Split uses shipped **`partition_event_records`** (records-per-chunk; **not** mid-record span).
- Each partition is an independent event-body; inflate + `decode_event_body` per EVENT; records concatenated in chunk order.
- Same `event_codec` for all EVENT chunks on this MVP path.
- `uncompressed_len` bounds each chunk’s inflate (fail-closed).
- Default `parse_chunk_frame` stays **non-inflating**; composition decode calls `decode_chunk_payload` explicitly.
- Empty events → zero EVENT chunks; FOOTER optional, codec NONE.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mid-record spanning across chunks; compressed SOURCE/INDEX/SUMMARY;
- dictionaries or always-on inflate inside default `parse_chunk_frame`;
- dual-equality vs C or certified perf claims.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Bad magic / truncated stream / bad sync | **Err** (stream) |
| Unsupported EVENT codec | **Err** |
| Corrupt compressed payload on any EVENT | **Err** |
| Inflated len ≠ `uncompressed_len` | **Err** (`SizeMismatch`) |
| Truncated / invalid event-body after inflate | **Err** |
| Never panic | Required |

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::multi_chunk_compressed::{
    decode_multi_chunk_compressed_profile, encode_multi_chunk_compressed_profile,
};
use nytprof_format_v6::chunk::codec;
use nytprof_format_v6::event_body::EventRecordSpec;

let events = [/* ≥2 records */];
let wire = encode_multi_chunk_compressed_profile(
    6, 0, 0, 0, 0, &[], codec::ZSTD, &events, 1, None,
)?;
let (prof, n) = decode_multi_chunk_compressed_profile(&wire)?;
assert!(prof.event_chunk_count >= 2);
```

- Composes shipped `partition_event_records`, `encode_event_body`, payload-codec frame helpers, `decode_prefix_chunk_stream`, `decode_chunk_payload`, `decode_event_body`.
- Not wired into default CLI.

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Multi-chunk EVENT codec NONE | prior `FMT-V6-MULTI-CHUNK-EVENT-*` |
| Single-chunk compressed mini-profile | prior `FMT-V6-COMPRESSED-PROFILE-*` |
| Multi-chunk + compressed EVENT | **done** (`FMT-V6-MULTI-CHUNK-COMPRESSED-*`) |
| Always-on default inflate / mid-record span | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. Mid-record spanning; mixed per-chunk codecs. Compressed SOURCE/INDEX/SUMMARY mixed: **done** separately (`FMT-V6-COMPRESSED-MIXED-*`).
2. Always-on inflate in default parse (compat residual: explicit only).
3. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.

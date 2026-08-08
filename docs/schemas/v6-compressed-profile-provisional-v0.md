# Format v6 compressed multi-codec mini-profile composition (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-COMPRESSED-PROFILE-PROVISIONAL` (contract), `FMT-V6-COMPRESSED-PROFILE-MVP` (shipped compose encode/decode + tests)  
**Depends on:** mini-profile [`v6-mini-profile-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-mini-profile-provisional-v0.md); event-body [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); payload codecs ZLIB/ZSTD/LZ4 (`v6-payload-*-provisional-v0.md`)  
**Gate:** COL-007 runway preflight only — **before** always-on inflate / dictionaries / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** composition path for a mini-profile whose **EVENT** chunk payload may be sealed with:

| Codec | Id | Wire EVENT payload |
|------:|---:|--------------------|
| `NONE` | 0 | Raw `encode_event_body` bytes |
| `ZLIB` | 1 | zlib stream of event-body |
| `ZSTD` | 2 | zstd frame of event-body |
| `LZ4` | 3 | LZ4 raw block of event-body |

Layout (same shape as mini-profile):

```text
[file prefix][EVENT chunk with chosen codec…][optional FOOTER codec NONE…]
```

Rules:

- Event-body is produced by shipped `encode_event_body` / recovered by `decode_event_body` after **explicit** `decode_chunk_payload`.
- `uncompressed_len` bounds inflate (fail-closed size mismatch / oversize / corrupt).
- Default `parse_chunk_frame` stays **non-inflating**; composition decode calls inflate helpers explicitly.
- Empty event list → no EVENT chunk (prefix-only), any codec argument ignored for that case.
- Optional FOOTER remains codec **NONE** opaque (or empty) payload on this MVP.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- multi-chunk compressed EVENT split, SOURCE/INDEX/SUMMARY compression, dictionaries;
- always-on inflate inside default `parse_chunk_frame`;
- dual-equality vs C or certified perf claims.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Bad magic / truncated stream / bad chunk sync | **Err** (stream layer) |
| Unsupported EVENT codec | **Err** |
| Corrupt compressed payload | **Err** |
| Inflated len ≠ `uncompressed_len` | **Err** (`SizeMismatch`) |
| Truncated / invalid event-body after inflate | **Err** (event-body layer) |
| Never panic | Required |

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::compressed_profile::{
    decode_compressed_mini_profile, encode_compressed_mini_profile,
};
use nytprof_format_v6::chunk::codec;
use nytprof_format_v6::event_body::EventRecordSpec;

let events = [EventRecordSpec::TimeLine { fid: 1, line: 5, ticks: 9 }];
let wire = encode_compressed_mini_profile(
    6, 0, 0, 0, 0, &[], codec::ZSTD, &events, None,
)?;
let (prof, n) = decode_compressed_mini_profile(&wire)?;
assert_eq!(n, wire.len());
assert_eq!(prof.event_codec, codec::ZSTD);
```

- Composes shipped `encode_file_prefix`, `encode_event_body`, `encode_chunk_frame_{zlib,zstd,lz4}` / NONE frame, `decode_prefix_chunk_stream`, `decode_chunk_payload`, `decode_event_body`.
- Not wired into default CLI.

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Per-codec payload helpers | prior `FMT-V6-PAYLOAD-*` |
| Compressed mini-profile composition | **done** (`FMT-V6-COMPRESSED-PROFILE-*`) |
| Multi-chunk EVENT with compressed payloads | **done** separately (`FMT-V6-MULTI-CHUNK-COMPRESSED-*`) |
| Compressed multi-kind mixed (SOURCE/INDEX/SUMMARY) | **done** separately (`FMT-V6-COMPRESSED-MIXED-*`) |
| Always-on default inflate | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. Multi-chunk compressed EVENT: **done** (`FMT-V6-MULTI-CHUNK-COMPRESSED-*`); compressed multi-kind mixed: **done** (`FMT-V6-COMPRESSED-MIXED-*`).
2. Always-on inflate in default parse (compat residual: explicit only).
3. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.

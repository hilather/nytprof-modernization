# Format v6 compressed multi-kind mixed profile (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-COMPRESSED-MIXED-PROVISIONAL` (contract), `FMT-V6-COMPRESSED-MIXED-MVP` (shipped encode/decode + tests)  
**Depends on:** mixed kind composition (`encode_mixed_kind_profile` path); event/source/index/summary/footer bodies; payload codecs ZLIB/ZSTD/LZ4; compressed mini-profile helpers  
**Gate:** COL-007 runway preflight only — **before** always-on inflate / dictionaries / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** mixed-kind composition path whose **EVENT / SOURCE / INDEX / SUMMARY** chunk payloads may use:

| Codec | Id |
|------:|---:|
| `NONE` | 0 |
| `ZLIB` | 1 |
| `ZSTD` | 2 |
| `LZ4` | 3 |

Layout (when non-empty kinds are present):

```text
[file prefix][EVENT…][SOURCE…][INDEX…][SUMMARY…][optional FOOTER codec NONE last]
```

Rules:

- Body bytes from shipped `encode_event_body` / `encode_source_body` / `encode_index_body` / `encode_summary_body`.
- Same `payload_codec` for all compressible kinds on this MVP path.
- **FOOTER** remains codec **NONE** and must be **last** when present (body via `encode_footer_body`).
- Decode: non-inflating stream parse → `decode_chunk_payload` per compressible kind → matching body decoder.
- `uncompressed_len` bounds inflate (fail-closed).
- Empty kinds omit that chunk; empty profile = prefix-only.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mid-record span; dictionaries (per-kind codecs: sibling `FMT-V6-PER-KIND-CODEC-*`);
- always-on inflate inside default `parse_chunk_frame`;
- dual-equality vs C or certified perf claims.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Bad magic / truncated / bad sync | **Err** |
| Unsupported payload codec | **Err** |
| Corrupt compressed payload | **Err** |
| Inflated len ≠ `uncompressed_len` | **Err** (`SizeMismatch`) |
| Truncated / invalid body after inflate | **Err** |
| FOOTER not last / FOOTER not codec NONE | **Err** |
| Never panic | Required |

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::compressed_mixed::{
    decode_compressed_mixed_profile, encode_compressed_mixed_profile,
};
use nytprof_format_v6::chunk::codec;

let wire = encode_compressed_mixed_profile(
    6, 0, 0, 0, 0, &[],
    codec::ZSTD,
    &events, &sources, &indexes, &summaries,
    Some(&footer_recs),
)?;
let (prof, n) = decode_compressed_mixed_profile(&wire)?;
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Mixed kind codec NONE | prior mixed profile preflight |
| Compressed EVENT mini / multi-chunk | prior `FMT-V6-COMPRESSED-PROFILE-*` / `FMT-V6-MULTI-CHUNK-COMPRESSED-*` |
| Compressed EVENT+SOURCE(+INDEX/SUMMARY) mixed | **done** (`FMT-V6-COMPRESSED-MIXED-*`) |
| Always-on default inflate | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. Per-kind different codecs: **done** (`FMT-V6-PER-KIND-CODEC-*`); multi-chunk EVENT under mixed: **done** (`FMT-V6-MULTI-CHUNK-KIND-*`); multi-chunk SOURCE/INDEX/SUMMARY residual.
2. Always-on inflate in default parse (compat residual: explicit only).
3. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.

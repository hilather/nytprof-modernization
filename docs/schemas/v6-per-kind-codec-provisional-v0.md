# Format v6 per-kind payload codecs on mixed profiles (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-PER-KIND-CODEC-PROVISIONAL` (contract), `FMT-V6-PER-KIND-CODEC-MVP` (shipped encode/decode + tests)  
**Depends on:** compressed multi-kind mixed [`v6-compressed-mixed-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-compressed-mixed-provisional-v0.md); payload codecs NONE/ZLIB/ZSTD/LZ4  
**Gate:** COL-007 runway preflight only — **before** always-on inflate / dictionaries / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** extension of compressed multi-kind mixed composition so that **EVENT / SOURCE / INDEX / SUMMARY** may each use a **different** payload codec:

| Kind | Allowed codecs |
|------|----------------|
| EVENT | NONE (0), ZLIB (1), ZSTD (2), LZ4 (3) |
| SOURCE | same |
| INDEX | same |
| SUMMARY | same |
| FOOTER | **NONE only**, must be **last** |

Rules:

- Encode via shipped body encoders + `encode_kind_chunk` per kind with that kind’s codec.
- Decode: non-inflating stream parse → `decode_chunk_payload` **per frame** (honors frame.codec) → matching body decoder.
- Shared-codec path remains available as `KindCodecs::uniform(c)` / `encode_compressed_mixed_profile`.
- Default `parse_chunk_frame` stays **non-inflating**.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- multi-chunk SOURCE/INDEX/SUMMARY under compression; dictionaries (multi-chunk EVENT under mixed: sibling `FMT-V6-MULTI-CHUNK-KIND-*`);
- always-on inflate inside default `parse_chunk_frame`;
- dual-equality vs C or certified perf claims.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Unsupported codec on any compressible frame | **Err** |
| Corrupt compressed payload | **Err** |
| Inflated len ≠ `uncompressed_len` | **Err** (`SizeMismatch`) |
| FOOTER not last / not codec NONE | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::compressed_mixed::{
    decode_compressed_mixed_profile, encode_compressed_mixed_profile_per_kind, KindCodecs,
};
use nytprof_format_v6::chunk::codec;

let codecs = KindCodecs {
    event: codec::ZSTD,
    source: codec::LZ4,
    index: codec::NONE,
    summary: codec::NONE,
};
let wire = encode_compressed_mixed_profile_per_kind(
    6, 0, 0, 0, 0, &[], codecs, &events, &sources, &[], &[], None,
)?;
let (prof, n) = decode_compressed_mixed_profile(&wire)?;
assert_eq!(prof.kind_codecs.event, codec::ZSTD);
assert_eq!(prof.kind_codecs.source, codec::LZ4);
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Shared-codec compressed mixed | prior `FMT-V6-COMPRESSED-MIXED-*` |
| Per-kind different codecs | **done** (`FMT-V6-PER-KIND-CODEC-*`) |
| Multi-chunk EVENT under mixed | **done** separately (`FMT-V6-MULTI-CHUNK-KIND-*`) |
| Always-on default inflate / multi-chunk SOURCE/INDEX/SUMMARY | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. Multi-chunk EVENT under mixed: **done** (`FMT-V6-MULTI-CHUNK-KIND-*`); multi-chunk SOURCE: **done** (`FMT-V6-MULTI-CHUNK-SOURCE-*`); multi-chunk INDEX/SUMMARY residual.
2. Always-on inflate in default parse (compat residual: explicit only).
3. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.

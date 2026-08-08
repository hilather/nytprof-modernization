# Format v6 multi-chunk INDEX under compressed mixed profiles (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-MULTI-CHUNK-INDEX-PROVISIONAL` (contract), `FMT-V6-MULTI-CHUNK-INDEX-MVP` (shipped encode/decode + tests)  
**Depends on:** multi-chunk SOURCE under mixed [`v6-multi-chunk-source-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-source-provisional-v0.md); INDEX body; payload codecs NONE/ZLIB/ZSTD/LZ4  
**Gate:** COL-007 runway preflight only — **before** always-on inflate / dictionaries / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** composition path where **INDEX** may span **≥2** chunks under a payload codec from **NONE / ZLIB / ZSTD / LZ4**, co-present with EVENT and/or SOURCE (single- or multi-chunk).

Layout example:

```text
[file prefix]
  [EVENT …]
  [SOURCE …]
  [INDEX codec C_i, part 0…]
  [INDEX codec C_i, part 1…]
  …
  [SUMMARY…]
  [optional FOOTER codec NONE last]
```

Rules:

- INDEX split uses shipped **`partition_index_records`** (records-per-chunk; **not** mid-record span).
- Each INDEX partition: `encode_index_body` + kind-chunk seal with `codecs.index`.
- EVENT/SOURCE may still use multi-chunk via existing max_records params.
- FOOTER: codec **NONE**, last when present.
- Decode: non-inflating stream parse → `decode_chunk_payload` per frame → body decoder; INDEX records concatenated in order.
- Default `parse_chunk_frame` stays **non-inflating**.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mid-record INDEX span: sibling `FMT-V6-MID-RECORD-INDEX-*`; multi-chunk SUMMARY: sibling `FMT-V6-MULTI-CHUNK-SUMMARY-*`;
- dictionaries; always-on inflate inside default `parse_chunk_frame`;
- dual-equality vs C or certified perf claims.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Corrupt compressed payload on any INDEX partition | **Err** |
| Inflated len ≠ `uncompressed_len` | **Err** (`SizeMismatch`) |
| FOOTER not last / not codec NONE | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::compressed_mixed::{
    decode_compressed_mixed_profile, encode_multi_chunk_index_mixed_profile,
    partition_index_records, KindCodecs,
};
use nytprof_format_v6::chunk::codec;

let codecs = KindCodecs {
    event: codec::LZ4,
    source: codec::NONE,
    index: codec::ZSTD,
    summary: codec::NONE,
};
let wire = encode_multi_chunk_index_mixed_profile(
    6, 0, 0, 0, 0, &[], codecs,
    &events, 0, &[], 0, &indexes, 1, &[], None,
)?;
let (prof, n) = decode_compressed_mixed_profile(&wire)?;
assert!(prof.index_chunk_count >= 2);
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Multi-chunk SOURCE under mixed | prior `FMT-V6-MULTI-CHUNK-SOURCE-*` |
| Multi-chunk INDEX under mixed | **done** (`FMT-V6-MULTI-CHUNK-INDEX-*`) |
| Multi-chunk SUMMARY under mixed | **done** separately (`FMT-V6-MULTI-CHUNK-SUMMARY-*`) |
| Always-on inflate / mid-record span | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. Multi-chunk SUMMARY: **done** as preflight (`FMT-V6-MULTI-CHUNK-SUMMARY-*`).
2. Always-on inflate in default parse (compat residual: explicit only).
3. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.

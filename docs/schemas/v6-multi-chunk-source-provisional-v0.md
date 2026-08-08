# Format v6 multi-chunk SOURCE under compressed mixed profiles (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-MULTI-CHUNK-SOURCE-PROVISIONAL` (contract), `FMT-V6-MULTI-CHUNK-SOURCE-MVP` (shipped encode/decode + tests)  
**Depends on:** multi-chunk EVENT under mixed [`v6-multi-chunk-kind-mixed-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-kind-mixed-provisional-v0.md); SOURCE body; payload codecs NONE/ZLIB/ZSTD/LZ4  
**Gate:** COL-007 runway preflight only — **before** always-on inflate / dictionaries / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** composition path where **SOURCE** may span **≥2** chunks under a payload codec from **NONE / ZLIB / ZSTD / LZ4**, co-present with EVENT (single- or multi-chunk).

Layout example:

```text
[file prefix]
  [EVENT codec C_e …]           # 0..n partitions
  [SOURCE codec C_s, part 0…]
  [SOURCE codec C_s, part 1…]
  …
  [INDEX…][SUMMARY…]
  [optional FOOTER codec NONE last]
```

Rules:

- SOURCE split uses shipped **`partition_source_records`** (records-per-chunk; **not** mid-record span).
- Each SOURCE partition: `encode_source_body` + kind-chunk seal with `codecs.source`.
- EVENT may still use multi-chunk via `partition_event_records` / `max_event_records_per_chunk`.
- FOOTER: codec **NONE**, last when present.
- Decode: non-inflating stream parse → `decode_chunk_payload` per frame → body decoder; SOURCE records concatenated in order.
- Default `parse_chunk_frame` stays **non-inflating**.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mid-record SOURCE span: sibling `FMT-V6-MID-RECORD-SOURCE-*`; multi-chunk SUMMARY (residual); multi-chunk INDEX: sibling `FMT-V6-MULTI-CHUNK-INDEX-*`;
- dictionaries; always-on inflate inside default `parse_chunk_frame`;
- dual-equality vs C or certified perf claims.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Corrupt compressed payload on any SOURCE partition | **Err** |
| Inflated len ≠ `uncompressed_len` | **Err** (`SizeMismatch`) |
| FOOTER not last / not codec NONE | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::compressed_mixed::{
    decode_compressed_mixed_profile, encode_multi_chunk_source_mixed_profile,
    partition_source_records, KindCodecs,
};
use nytprof_format_v6::chunk::codec;

let codecs = KindCodecs {
    event: codec::ZSTD,
    source: codec::LZ4,
    index: codec::NONE,
    summary: codec::NONE,
};
let wire = encode_multi_chunk_source_mixed_profile(
    6, 0, 0, 0, 0, &[], codecs,
    &events, 0,      // single EVENT
    &sources, 1,     // ≥2 SOURCE if sources.len() ≥ 2
    &[], &[], None,
)?;
let (prof, n) = decode_compressed_mixed_profile(&wire)?;
assert!(prof.source_chunk_count >= 2);
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Multi-chunk EVENT under mixed | prior `FMT-V6-MULTI-CHUNK-KIND-*` |
| Multi-chunk SOURCE under mixed | **done** (`FMT-V6-MULTI-CHUNK-SOURCE-*`) |
| Multi-chunk INDEX under mixed | **done** separately (`FMT-V6-MULTI-CHUNK-INDEX-*`) |
| Multi-chunk SUMMARY; always-on inflate | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. Multi-chunk INDEX: **done** (`FMT-V6-MULTI-CHUNK-INDEX-*`); multi-chunk SUMMARY: **done** (`FMT-V6-MULTI-CHUNK-SUMMARY-*`).
2. Always-on inflate in default parse (compat residual: explicit only).
3. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.

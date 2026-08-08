# Format v6 multi-chunk per kind under compressed mixed profiles (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-MULTI-CHUNK-KIND-PROVISIONAL` (contract), `FMT-V6-MULTI-CHUNK-KIND-MVP` (shipped encode/decode + tests)  
**Depends on:** multi-chunk EVENT partition; compressed mixed / per-kind codecs; payload codecs NONE/ZLIB/ZSTD/LZ4  
**Gate:** COL-007 runway preflight only — **before** always-on inflate / dictionaries / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** composition path where **EVENT** may span **≥2** chunks under a payload codec from **NONE / ZLIB / ZSTD / LZ4**, co-present with optional **SOURCE** (and INDEX/SUMMARY) using their own codecs (including a different codec from EVENT).

Layout example:

```text
[file prefix]
  [EVENT codec C_e, partition 0…]
  [EVENT codec C_e, partition 1…]
  …
  [SOURCE codec C_s…]
  [INDEX…][SUMMARY…]
  [optional FOOTER codec NONE last]
```

Rules:

- EVENT split uses shipped **`partition_event_records`** (records-per-chunk; **not** mid-record span).
- Each EVENT partition: `encode_event_body` + kind-chunk seal with `codecs.event`.
- SOURCE: single chunk here; multi-chunk SOURCE is sibling `FMT-V6-MULTI-CHUNK-SOURCE-*`. INDEX/SUMMARY: single chunk each.
- FOOTER: codec **NONE**, last when present.
- Decode: non-inflating stream parse → `decode_chunk_payload` per frame → body decoder; EVENT records concatenated in order.
- Default `parse_chunk_frame` stays **non-inflating**.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mid-record spanning; multi-chunk INDEX/SUMMARY (optional residual); multi-chunk SOURCE: sibling preflight;
- dictionaries; always-on inflate inside default `parse_chunk_frame`;
- dual-equality vs C or certified perf claims.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Corrupt compressed payload on any EVENT partition | **Err** |
| Inflated len ≠ `uncompressed_len` | **Err** (`SizeMismatch`) |
| FOOTER not last / not codec NONE | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::compressed_mixed::{
    decode_compressed_mixed_profile, encode_multi_chunk_kind_mixed_profile, KindCodecs,
};
use nytprof_format_v6::chunk::codec;

let codecs = KindCodecs {
    event: codec::ZSTD,
    source: codec::LZ4,
    index: codec::NONE,
    summary: codec::NONE,
};
let wire = encode_multi_chunk_kind_mixed_profile(
    6, 0, 0, 0, 0, &[], codecs, &events, 1, &sources, &[], &[], None,
)?;
let (prof, n) = decode_compressed_mixed_profile(&wire)?;
assert!(prof.event_chunk_count >= 2);
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Multi-chunk EVENT alone | prior `FMT-V6-MULTI-CHUNK-COMPRESSED-*` |
| Per-kind single-chunk mixed | prior `FMT-V6-PER-KIND-CODEC-*` |
| Multi-chunk EVENT + mixed SOURCE | **done** (`FMT-V6-MULTI-CHUNK-KIND-*`) |
| Multi-chunk SOURCE under mixed | **done** separately (`FMT-V6-MULTI-CHUNK-SOURCE-*`) |
| Multi-chunk INDEX/SUMMARY; always-on inflate | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. Multi-chunk SOURCE/INDEX/SUMMARY: **done** (`FMT-V6-MULTI-CHUNK-SOURCE-*` / `FMT-V6-MULTI-CHUNK-INDEX-*` / `FMT-V6-MULTI-CHUNK-SUMMARY-*`).
2. Always-on inflate in default parse (compat residual: explicit only).
3. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.

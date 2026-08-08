# Format v6 multi-chunk SUMMARY under compressed mixed profiles (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-MULTI-CHUNK-SUMMARY-PROVISIONAL` (contract), `FMT-V6-MULTI-CHUNK-SUMMARY-MVP` (shipped encode/decode + tests)  
**Depends on:** multi-chunk INDEX under mixed [`v6-multi-chunk-index-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-index-provisional-v0.md); SUMMARY body; payload codecs NONE/ZLIB/ZSTD/LZ4  
**Gate:** COL-007 runway preflight only — **before** always-on inflate / dictionaries / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** composition path where **SUMMARY** may span **≥2** chunks under a payload codec from **NONE / ZLIB / ZSTD / LZ4**, co-present with EVENT and/or SOURCE/INDEX (single- or multi-chunk).

Layout example:

```text
[file prefix]
  [EVENT …]
  [SOURCE …]
  [INDEX …]
  [SUMMARY codec C_s, part 0…]
  [SUMMARY codec C_s, part 1…]
  …
  [optional FOOTER codec NONE last]
```

Rules:

- SUMMARY split uses shipped **`partition_summary_records`** (records-per-chunk; **not** mid-record span).
- Each SUMMARY partition: `encode_summary_body` + kind-chunk seal with `codecs.summary`.
- EVENT/SOURCE/INDEX may still use multi-chunk via existing max_records params.
- FOOTER: codec **NONE**, last when present.
- Decode: non-inflating stream parse → `decode_chunk_payload` per frame → body decoder; SUMMARY records concatenated in order.
- Default `parse_chunk_frame` stays **non-inflating**.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- mid-record SUMMARY span: sibling `FMT-V6-MID-RECORD-SUMMARY-*`; dictionaries;
- always-on inflate inside default `parse_chunk_frame`;
- dual-equality vs C or certified perf claims.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Corrupt compressed payload on any SUMMARY partition | **Err** |
| Inflated len ≠ `uncompressed_len` | **Err** (`SizeMismatch`) |
| FOOTER not last / not codec NONE | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::compressed_mixed::{
    decode_compressed_mixed_profile, encode_multi_chunk_summary_mixed_profile,
    partition_summary_records, KindCodecs,
};
use nytprof_format_v6::chunk::codec;

let codecs = KindCodecs {
    event: codec::LZ4,
    source: codec::NONE,
    index: codec::NONE,
    summary: codec::ZSTD,
};
let wire = encode_multi_chunk_summary_mixed_profile(
    6, 0, 0, 0, 0, &[], codecs,
    &events, 0, &[], 0, &[], 0, &summaries, 1, None,
)?;
let (prof, n) = decode_compressed_mixed_profile(&wire)?;
assert!(prof.summary_chunk_count >= 2);
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Multi-chunk INDEX under mixed | prior `FMT-V6-MULTI-CHUNK-INDEX-*` |
| Multi-chunk SUMMARY under mixed | **done** (`FMT-V6-MULTI-CHUNK-SUMMARY-*`) |
| Mid-record EVENT span | **done** separately (`FMT-V6-MID-RECORD-SPAN-*`) |
| Always-on inflate | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. Always-on inflate in default parse (compat residual: explicit only).
2. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.
3. Mid-record EVENT/SOURCE/INDEX/SUMMARY: **done** as preflight; always-on inflate residual.

# Format v6 mid-record SOURCE span on always-inflate multi-kind mixed path — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-PROVISIONAL` (contract), `FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-MVP` (shipped encode/decode + tests)  
**Depends on:** `FMT-V6-DECODED-MIXED-MID-RECORD-*` (EVENT mid-on-mixed); `split_source_body_bytes`; payload codecs NONE/ZLIB/ZSTD/LZ4; optional CRC  
**Gate:** COL-007 runway preflight only — **before** dictionaries / C v6 writer / CLI v6 default / dual-equality

---

## Scope and non-claims

This document freezes a **provisional SOURCE mid-record-spanning consumer path** on the always-inflate multi-kind mixed stack:

```text
wire bytes
  → encode: co-present EVENT (full) + encode_source_body mid-record split
             into ≥2 SOURCE chunks under NONE/ZLIB/ZSTD/LZ4
  → decode: decode_prefix_chunk_stream_plain (optional CRC)
  → join same-kind plains → decode_*_body per kind
```

Rules:

- **SOURCE** body bytes are split mid-record across ≥2 SOURCE chunk payloads (interior split: neither piece alone is a complete multi-record SOURCE body).
- At least **one other kind** is co-present (typically EVENT).
- Codecs **NONE / ZLIB / ZSTD / LZ4** per kind; FOOTER codec NONE when present.
- Decode reuses shipped [`decode_decoded_mixed_profile`] (join plains then body decode).
- Default `parse_chunk_frame` stays **non-inflating** / non-CRC.
- EVENT mid-record-on-mixed remains prior preflight; INDEX/SUMMARY mid-record-on-mixed are sibling preflights.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- dictionaries; dual-equality; FMT-012 golden freeze;
- mutating default `parse_chunk_frame` to always inflate;
- INDEX/SUMMARY mid-record-on-mixed (see sibling schemas; not this document's primary claim).

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Truncated mid-record SOURCE join | **Err** (source-body layer) |
| CRC mismatch when verify on | **Err** |
| Corrupt compressed payload on span-carrying SOURCE | **Err** |
| Invalid interior split on encode | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::decoded_mixed::{
    decode_decoded_mixed_profile, encode_decoded_mixed_mid_record_source_profile,
};
use nytprof_format_v6::mid_record_span::{default_mid_body_split, split_source_body_bytes};
use nytprof_format_v6::source_body::encode_source_body;
use nytprof_format_v6::compressed_mixed::KindCodecs;
use nytprof_format_v6::chunk::codec;

let body = encode_source_body(&sources);
let split = /* interior mid-record split */;
let codecs = KindCodecs {
    event: codec::NONE,
    source: codec::ZLIB,
    index: codec::NONE,
    summary: codec::NONE,
};
let wire = encode_decoded_mixed_mid_record_source_profile(
    6, 0, 0, 0, 0, &[], codecs, &events, &sources, split, &[], &[], None,
)?;
let (prof, n) = decode_decoded_mixed_profile(&wire, /* verify_crc */ true)?;
assert_eq!(prof.source_chunk_count, 2);
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Standalone SOURCE mid-record | prior `FMT-V6-MID-RECORD-SOURCE-*` |
| EVENT mid-record on mixed path | prior `FMT-V6-DECODED-MIXED-MID-RECORD-*` |
| SOURCE mid-record on always-inflate multi-kind path | **done** (`FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-*`) |
| INDEX mid-record-on-mixed | sibling `FMT-V6-DECODED-MIXED-MID-RECORD-INDEX-*` |
| SUMMARY mid-record-on-mixed | sibling `FMT-V6-DECODED-MIXED-MID-RECORD-SUMMARY-*` |
| Concurrent multi-kind mid-record-on-mixed | sibling `FMT-V6-DECODED-MIXED-MID-RECORD-CONCURRENT-*` |
| C writer / dual-equality / default-parse mutate | residual / deferred |

---

## Open items (honest residual)

1. Mutating default `parse_chunk_frame` to inflate/CRC in place.
2. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.
3. Full COL-007 C writer / COL-008 batched Rust writer.

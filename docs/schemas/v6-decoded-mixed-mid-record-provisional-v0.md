# Format v6 mid-record span recovery on always-inflate multi-kind mixed path — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-DECODED-MIXED-MID-RECORD-PROVISIONAL` (contract), `FMT-V6-DECODED-MIXED-MID-RECORD-MVP` (shipped encode/decode + tests)  
**Depends on:** `FMT-V6-DECODED-MIXED-*`; mid-record split helpers (`FMT-V6-MID-RECORD-*`); payload codecs NONE/ZLIB/ZSTD/LZ4; optional CRC  
**Gate:** COL-007 runway preflight only — **before** dictionaries / C v6 writer / CLI v6 default / dual-equality

---

## Scope and non-claims

This document freezes a **provisional mid-record-spanning consumer path** on the always-inflate multi-kind mixed stack:

```text
wire bytes
  → encode: encode_*_body + interior mid-record split of ≥1 kind
             + co-present other kind chunk(s) under NONE/ZLIB/ZSTD/LZ4
  → decode: decode_prefix_chunk_stream_plain (optional CRC)
  → join same-kind plains → decode_*_body per kind
```

Rules:

- At least **one kind** has body bytes **split mid-record** across ≥2 same-kind chunk payloads (interior split: neither piece alone is a complete multi-record body).
- At least **one other kind** is co-present so the profile is multi-kind.
- Codecs **NONE / ZLIB / ZSTD / LZ4** per kind; FOOTER codec NONE when present.
- Decode reuses shipped [`decode_decoded_mixed_profile`] (join plains then body decode).
- Default `parse_chunk_frame` stays **non-inflating** / non-CRC.
- Standalone mid-record-only profiles remain prior preflights (`FMT-V6-MID-RECORD-*`).

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- dictionaries; dual-equality; FMT-012 golden freeze;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Truncated mid-record join (missing continuation) | **Err** (body layer) |
| CRC mismatch when verify on | **Err** |
| Corrupt compressed payload on a span-carrying chunk | **Err** |
| Invalid interior split on encode | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::decoded_mixed::{
    decode_decoded_mixed_profile, encode_decoded_mixed_mid_record_event_profile,
};
use nytprof_format_v6::mid_record_span::{default_mid_body_split, split_event_body_bytes};
use nytprof_format_v6::event_body::encode_event_body;
use nytprof_format_v6::compressed_mixed::KindCodecs;
use nytprof_format_v6::chunk::codec;

let body = encode_event_body(&events);
let split = default_mid_body_split(&body).unwrap();
let codecs = KindCodecs {
    event: codec::ZLIB,
    source: codec::NONE,
    index: codec::NONE,
    summary: codec::NONE,
};
let wire = encode_decoded_mixed_mid_record_event_profile(
    6, 0, 0, 0, 0, &[], codecs, &events, split, &sources, &[], &[], None,
)?;
let (prof, n) = decode_decoded_mixed_profile(&wire, /* verify_crc */ true)?;
assert_eq!(prof.event_chunk_count, 2);
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Standalone mid-record EVENT/SOURCE/INDEX/SUMMARY | prior `FMT-V6-MID-RECORD-*` |
| Always-inflate multi-kind mixed | prior `FMT-V6-DECODED-MIXED-*` |
| Multi-chunk record-aligned on mixed path | prior `FMT-V6-DECODED-MIXED-MULTI-CHUNK-*` |
| Mid-record EVENT span recovery on always-inflate multi-kind path | **done** (`FMT-V6-DECODED-MIXED-MID-RECORD-*`) |
| Mid-record SOURCE span on multi-kind path | **done** (`FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-*`) |
| INDEX mid-record-on-mixed | **done** (`FMT-V6-DECODED-MIXED-MID-RECORD-INDEX-*`) |
| SUMMARY mid-record-on-mixed | **done** (`FMT-V6-DECODED-MIXED-MID-RECORD-SUMMARY-*`) |
| Concurrent multi-kind mid-record-on-mixed | **done** (`FMT-V6-DECODED-MIXED-MID-RECORD-CONCURRENT-*`) |
| C writer / dual-equality / default-parse mutate | residual / deferred |

---

## Open items (honest residual)

1. Mutating default `parse_chunk_frame` to inflate/CRC in place.
2. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.
3. Full COL-007 C writer / COL-008 batched Rust writer.

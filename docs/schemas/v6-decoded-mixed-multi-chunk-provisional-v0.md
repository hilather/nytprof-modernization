# Format v6 multi-chunk record-aligned recovery on always-inflate multi-kind mixed path — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-DECODED-MIXED-MULTI-CHUNK-PROVISIONAL` (contract), `FMT-V6-DECODED-MIXED-MULTI-CHUNK-MVP` (shipped encode/decode + tests)  
**Depends on:** `FMT-V6-DECODED-MIXED-*`; multi-chunk partitions; payload codecs NONE/ZLIB/ZSTD/LZ4; optional CRC  
**Gate:** COL-007 runway preflight only — **before** dictionaries / C v6 writer / CLI v6 default / dual-equality

---

## Scope and non-claims

This document freezes a **provisional multi-chunk record-aligned consumer path** on the always-inflate multi-kind mixed stack:

```text
wire bytes
  → encode: partition_* (record-aligned) + per-kind codec seal + co-present kinds
  → decode: decode_prefix_chunk_stream_plain (optional CRC)
  → join same-kind plains → decode_*_body per kind
```

Rules:

- At least **one kind** may occupy **≥2 chunks** (via shipped `partition_*` with `max_*_per_chunk ≥ 1`).
- At least **one other kind** is co-present (or FOOTER) so the profile is multi-kind.
- Codecs **NONE / ZLIB / ZSTD / LZ4** per kind; FOOTER codec NONE when present.
- Decode reuses shipped [`decode_decoded_mixed_profile`] (join plains then body decode).
- Default `parse_chunk_frame` stays **non-inflating** / non-CRC.
- This is **record-aligned** multi-chunk (not mid-record span as the primary claim; mid-record remains a separate prior preflight).

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- dictionaries; dual-equality; FMT-012 golden freeze;
- mutating default `parse_chunk_frame` to always inflate;
- a re-claim that mid-record spanning is the deliverable of this slice.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Truncated mid-stream | **Err** |
| CRC mismatch when verify on | **Err** |
| Corrupt compressed payload on a multi-chunk kind | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::decoded_mixed::{
    decode_decoded_mixed_profile, encode_decoded_mixed_multi_chunk_profile,
};
use nytprof_format_v6::compressed_mixed::KindCodecs;
use nytprof_format_v6::chunk::codec;

let codecs = KindCodecs {
    event: codec::ZLIB,
    source: codec::NONE,
    index: codec::NONE,
    summary: codec::NONE,
};
// max_event_records_per_chunk = 1 → ≥2 EVENT chunks when events.len() ≥ 2
let wire = encode_decoded_mixed_multi_chunk_profile(
    6, 0, 0, 0, 0, &[], codecs,
    &events, 1, &sources, 0, &[], 0, &[], 0, None,
)?;
let (prof, n) = decode_decoded_mixed_profile(&wire, /* verify_crc */ true)?;
assert!(prof.event_chunk_count >= 2);
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Single-chunk multi-kind always-inflate mixed | prior `FMT-V6-DECODED-MIXED-*` |
| Multi-chunk partitions under compressed mixed | prior `FMT-V6-MULTI-CHUNK-*-*` |
| Multi-chunk record-aligned recovery on always-inflate mixed path | **done** (`FMT-V6-DECODED-MIXED-MULTI-CHUNK-*`) |
| Mid-record span (separate) | prior `FMT-V6-MID-RECORD-*` |
| C writer / dual-equality / default-parse mutate | residual / deferred |

---

## Open items (honest residual)

1. Mutating default `parse_chunk_frame` to inflate/CRC in place.
2. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.
3. Full COL-007 C writer / COL-008 batched Rust writer.

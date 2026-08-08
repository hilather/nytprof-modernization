# Format v6 concurrent multi-kind mid-record spans on always-inflate mixed path — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-DECODED-MIXED-MID-RECORD-CONCURRENT-PROVISIONAL` (contract), `FMT-V6-DECODED-MIXED-MID-RECORD-CONCURRENT-MVP` (shipped encode/decode + tests)  
**Depends on:** per-kind mid-on-mixed preflights (`FMT-V6-DECODED-MIXED-MID-RECORD-*` siblings); `split_*_body_bytes`; payload codecs NONE/ZLIB/ZSTD/LZ4; optional CRC  
**Gate:** COL-007 runway preflight only — **before** dictionaries / C v6 writer / CLI v6 default / dual-equality

---

## Scope and non-claims

This document freezes a **provisional concurrent multi-kind mid-record consumer path** on the always-inflate multi-kind mixed stack:

```text
wire bytes
  → encode: ≥2 kinds each mid-record-split into ≥2 same-kind chunks
             under NONE/ZLIB/ZSTD/LZ4 (optional full co-present kinds)
  → decode: decode_prefix_chunk_stream_plain (optional CRC)
  → join same-kind plains → decode_*_body per kind
```

Rules:

- At least **two** of EVENT / SOURCE / INDEX / SUMMARY carry an interior mid-record body split (each kind ≥2 chunks).
- Other kinds may be full single chunks or omitted.
- Codecs **NONE / ZLIB / ZSTD / LZ4** per kind; FOOTER codec NONE when present.
- Decode reuses shipped [`decode_decoded_mixed_profile`] (join plains then body decode).
- Default `parse_chunk_frame` stays **non-inflating** / non-CRC.
- Per-kind single mid-on-mixed APIs remain prior preflights.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- dictionaries; dual-equality; FMT-012 golden freeze;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed encode / decode

| Condition | Result |
|-----------|--------|
| Fewer than 2 mid-split kinds on encode | **Err** (`NeedConcurrentMidRecordKinds`) |
| Invalid interior split on encode | **Err** (kind body layer) |
| Truncated mid-record join on decode | **Err** (kind body layer) |
| CRC mismatch when verify on | **Err** |
| Corrupt compressed span-carrying payload | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::decoded_mixed::{
    decode_decoded_mixed_profile, encode_decoded_mixed_mid_record_concurrent_profile,
    MidRecordKindSplits,
};
use nytprof_format_v6::compressed_mixed::KindCodecs;
use nytprof_format_v6::chunk::codec;

let splits = MidRecordKindSplits {
    event: Some(/* interior */),
    source: Some(/* interior */),
    index: None,
    summary: None,
};
let wire = encode_decoded_mixed_mid_record_concurrent_profile(
    6, 0, 0, 0, 0, &[], codecs, &events, &sources, &[], &[], splits, None,
)?;
let (prof, n) = decode_decoded_mixed_profile(&wire, /* verify_crc */ true)?;
assert_eq!(prof.event_chunk_count, 2);
assert_eq!(prof.source_chunk_count, 2);
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Per-kind mid-record-on-mixed (EVENT/SOURCE/INDEX/SUMMARY) | prior siblings **done** |
| Concurrent multi-kind mid-record-on-mixed | **done** (`FMT-V6-DECODED-MIXED-MID-RECORD-CONCURRENT-*`) |
| C writer / dual-equality / default-parse mutate | residual / deferred |

---

## Open items (honest residual)

1. Mutating default `parse_chunk_frame` to inflate/CRC in place.
2. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.
3. Full COL-007 C writer / COL-008 batched Rust writer.

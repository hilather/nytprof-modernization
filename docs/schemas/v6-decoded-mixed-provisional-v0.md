# Format v6 decoded multi-kind mixed profile (always-inflate + optional CRC) — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-DECODED-MIXED-PROVISIONAL` (contract), `FMT-V6-DECODED-MIXED-MVP` (shipped encode/decode + tests)  
**Depends on:** decoded-stream; per-kind body codecs; compressed-mixed encode helpers; optional CRC  
**Gate:** COL-007 runway preflight only — **before** dictionaries / C v6 writer / CLI v6 default / dual-equality

---

## Scope and non-claims

This document freezes a **provisional multi-kind consumer path**:

```text
wire bytes
  → decode_prefix_chunk_stream_plain (always inflate + optional CRC)
  → group plains by kind (EVENT/SOURCE/INDEX/SUMMARY) in file order
  → join same-kind plains → decode_*_body per kind
  → optional FOOTER codec NONE last → decode_footer_body
```

Rules:

- Any non-empty subset of EVENT / SOURCE / INDEX / SUMMARY may appear.
- Per-kind codecs **NONE / ZLIB / ZSTD / LZ4** (ids 0–3); FOOTER always codec NONE when present.
- Wire order when encoding via shipped helper: EVENT, SOURCE, INDEX, SUMMARY, optional FOOTER last.
- Default `parse_chunk_frame` stays **non-inflating** / non-CRC.
- Encode reuses `encode_compressed_mixed_profile_per_kind`; decode uses always-inflate stream path.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- dictionaries; dual-equality; FMT-012 golden freeze;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Truncated mid-stream | **Err** |
| CRC mismatch when verify on | **Err** |
| Corrupt compressed payload | **Err** |
| Truncated joined kind body | **Err** (body layer) |
| FOOTER not last / non-NONE FOOTER | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::decoded_mixed::{
    decode_decoded_mixed_profile, encode_decoded_mixed_profile,
};
use nytprof_format_v6::compressed_mixed::KindCodecs;
use nytprof_format_v6::chunk::codec;

let codecs = KindCodecs {
    event: codec::ZSTD,
    source: codec::LZ4,
    index: codec::ZLIB,
    summary: codec::NONE,
};
let wire = encode_decoded_mixed_profile(
    6, 0, 0, 0, 0, &[], codecs, &events, &sources, &indexes, &summaries, None,
)?;
let (prof, n) = decode_decoded_mixed_profile(&wire, /* verify_crc */ true)?;
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Per-kind DECODED-EVENT/SOURCE/INDEX/SUMMARY | prior preflights |
| Always-inflate stream | prior `FMT-V6-DECODED-STREAM-*` |
| Compressed multi-kind mixed (no CRC flag) | prior `FMT-V6-COMPRESSED-MIXED-*` |
| Multi-kind always-inflate + optional CRC consumer | **done** (`FMT-V6-DECODED-MIXED-*`) |
| Multi-chunk record-aligned on always-inflate mixed path | **done** separately (`FMT-V6-DECODED-MIXED-MULTI-CHUNK-*`) |
| Mid-record span on always-inflate multi-kind path | **done** separately (`FMT-V6-DECODED-MIXED-MID-RECORD-*`) |
| C writer / dual-equality / default-parse mutate | residual / deferred |

---

## Open items (honest residual)

1. Mutating default `parse_chunk_frame` to inflate/CRC in place.
2. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.
3. Full COL-007 C writer / COL-008 batched Rust writer.

# Format v6 decoded SUMMARY profile (stream → inflate → summary-body) — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-DECODED-SUMMARY-PROVISIONAL` (contract), `FMT-V6-DECODED-SUMMARY-MVP` (shipped encode/decode + tests)  
**Depends on:** decoded-stream (`FMT-V6-DECODED-STREAM-*`); summary-body; payload codecs NONE/ZLIB/ZSTD/LZ4; optional CRC; decoded-INDEX/SOURCE/EVENT pattern  
**Gate:** COL-007 runway preflight only — **before** full summary catalog freeze / dictionaries / C v6 writer / CLI v6 default

---

## Scope and non-claims

This document freezes a **provisional logical SUMMARY consumer path**:

```text
wire bytes
  → decode_prefix_chunk_stream_plain (always inflate + optional CRC)
  → collect SUMMARY plain payloads in order
  → join plains
  → decode_summary_body
  → ordered logical OwnedSummaryRecord list
```

Rules:

- SUMMARY chunks under **NONE / ZLIB / ZSTD / LZ4** (ids 0–3); optional FOOTER codec NONE last.
- Multi-chunk SUMMARY is **record-aligned** via shipped `partition_summary_records` (join plains then one `decode_summary_body`).
- Default `parse_chunk_frame` / non-inflating stream parse stay **unchanged**.
- Provisional SUMMARY records only (key_id/count/value + string-blob label) — not a full catalog freeze.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- full summary catalog / dictionaries / dual-equality / FMT-012 golden freeze;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Truncated stream / mid-chunk | **Err** |
| CRC mismatch when verify on | **Err** |
| Corrupt compressed SUMMARY payload | **Err** |
| Truncated joined summary-body | **Err** (summary-body layer) |
| Unexpected non-SUMMARY/FOOTER kind | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::decoded_summary::{
    decode_decoded_summary_profile, encode_decoded_summary_profile,
};
use nytprof_format_v6::chunk::codec;

let wire = encode_decoded_summary_profile(
    6, 0, 0, 0, 0, &[], codec::ZSTD, &summaries, /* max_per_chunk */ 0, None,
)?;
let (prof, n) = decode_decoded_summary_profile(&wire, /* verify_crc */ true)?;
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Always-inflate multi-chunk stream | prior `FMT-V6-DECODED-STREAM-*` |
| Summary-body encode/decode | prior `FMT-V6-SUMMARY-BODY-*` |
| Logical EVENT/SOURCE/INDEX recovery over stream | prior decoded-kind preflights |
| Logical SUMMARY recovery via always-inflate stream + optional CRC | **done** (`FMT-V6-DECODED-SUMMARY-*`) |
| Multi-kind always-inflate mixed consumer | **done** separately (`FMT-V6-DECODED-MIXED-*`) |
| Full catalog; C writer; dual-equality | residual / deferred |

---

## Open items (honest residual)

1. Mutating default `parse_chunk_frame` to inflate/CRC in place.
2. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.
3. Full COL-007 C writer / COL-008 batched Rust writer.

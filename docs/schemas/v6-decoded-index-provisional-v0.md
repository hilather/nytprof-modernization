# Format v6 decoded INDEX profile (stream → inflate → index-body) — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-DECODED-INDEX-PROVISIONAL` (contract), `FMT-V6-DECODED-INDEX-MVP` (shipped encode/decode + tests)  
**Depends on:** decoded-stream (`FMT-V6-DECODED-STREAM-*`); index-body; payload codecs NONE/ZLIB/ZSTD/LZ4; optional CRC; decoded-SOURCE/EVENT pattern  
**Gate:** COL-007 runway preflight only — **before** full index catalog freeze / dictionaries / C v6 writer / CLI v6 default

---

## Scope and non-claims

This document freezes a **provisional logical INDEX consumer path**:

```text
wire bytes
  → decode_prefix_chunk_stream_plain (always inflate + optional CRC)
  → collect INDEX plain payloads in order
  → join plains
  → decode_index_body
  → ordered logical OwnedIndexRecord list
```

Rules:

- INDEX chunks under **NONE / ZLIB / ZSTD / LZ4** (ids 0–3); optional FOOTER codec NONE last.
- Multi-chunk INDEX is **record-aligned** via shipped `partition_index_records` (join plains then one `decode_index_body`).
- Default `parse_chunk_frame` / non-inflating stream parse stay **unchanged**.
- Provisional INDEX records only (key_id/file_offset/length + string-blob label) — not a full catalog freeze.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- full index catalog / dictionaries / dual-equality / FMT-012 golden freeze;
- decoded SUMMARY: sibling `FMT-V6-DECODED-SUMMARY-*`;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Truncated stream / mid-chunk | **Err** |
| CRC mismatch when verify on | **Err** |
| Corrupt compressed INDEX payload | **Err** |
| Truncated joined index-body | **Err** (index-body layer) |
| Unexpected non-INDEX/FOOTER kind | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::decoded_index::{
    decode_decoded_index_profile, encode_decoded_index_profile,
};
use nytprof_format_v6::chunk::codec;

let wire = encode_decoded_index_profile(
    6, 0, 0, 0, 0, &[], codec::ZSTD, &indexes, /* max_per_chunk */ 0, None,
)?;
let (prof, n) = decode_decoded_index_profile(&wire, /* verify_crc */ true)?;
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Always-inflate multi-chunk stream | prior `FMT-V6-DECODED-STREAM-*` |
| Index-body encode/decode | prior `FMT-V6-INDEX-BODY-*` |
| Logical EVENT/SOURCE recovery over stream | prior `FMT-V6-DECODED-EVENT-*` / `FMT-V6-DECODED-SOURCE-*` |
| Logical INDEX recovery via always-inflate stream + optional CRC | **done** (`FMT-V6-DECODED-INDEX-*`) |
| Logical SUMMARY recovery over stream | **done** separately (`FMT-V6-DECODED-SUMMARY-*`) |
| Decoded SUMMARY | **done** separately (`FMT-V6-DECODED-SUMMARY-*`) |
| Full catalog; C writer | residual / deferred |

---

## Open items (honest residual)

1. Decoded SUMMARY: sibling `FMT-V6-DECODED-SUMMARY-*` (landed).
2. Mutating default `parse_chunk_frame` to inflate/CRC in place.
3. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.
4. Full COL-007 C writer / COL-008 batched Rust writer.

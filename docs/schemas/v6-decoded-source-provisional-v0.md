# Format v6 decoded SOURCE profile (stream → inflate → source-body) — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-DECODED-SOURCE-PROVISIONAL` (contract), `FMT-V6-DECODED-SOURCE-MVP` (shipped encode/decode + tests)  
**Depends on:** decoded-stream (`FMT-V6-DECODED-STREAM-*`); source-body; payload codecs NONE/ZLIB/ZSTD/LZ4; optional CRC; decoded-EVENT pattern  
**Gate:** COL-007 runway preflight only — **before** full SRC_LINE catalog freeze / dictionaries / C v6 writer / CLI v6 default

---

## Scope and non-claims

This document freezes a **provisional logical SOURCE consumer path**:

```text
wire bytes
  → decode_prefix_chunk_stream_plain (always inflate + optional CRC)
  → collect SOURCE plain payloads in order
  → join plains
  → decode_source_body
  → ordered logical OwnedSourceRecord list
```

Rules:

- SOURCE chunks under **NONE / ZLIB / ZSTD / LZ4** (ids 0–3); optional FOOTER codec NONE last.
- Multi-chunk SOURCE is **record-aligned** via shipped `partition_source_records` (join plains then one `decode_source_body`).
- Default `parse_chunk_frame` / non-inflating stream parse stay **unchanged**.
- Provisional SOURCE records only (fid/line + string-blob text) — not a full SRC_LINE catalog freeze.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- full SRC_LINE catalog / dictionaries / dual-equality / FMT-012 golden freeze;
- decoded INDEX/SUMMARY profiles (residual unless extended);
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Truncated stream / mid-chunk | **Err** |
| CRC mismatch when verify on | **Err** |
| Corrupt compressed SOURCE payload | **Err** |
| Truncated joined source-body | **Err** (source-body layer) |
| Unexpected non-SOURCE/FOOTER kind | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::decoded_source::{
    decode_decoded_source_profile, encode_decoded_source_profile,
};
use nytprof_format_v6::chunk::codec;

let wire = encode_decoded_source_profile(
    6, 0, 0, 0, 0, &[], codec::ZSTD, &sources, /* max_per_chunk */ 0, None,
)?;
let (prof, n) = decode_decoded_source_profile(&wire, /* verify_crc */ true)?;
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Always-inflate multi-chunk stream | prior `FMT-V6-DECODED-STREAM-*` |
| Source-body encode/decode | prior `FMT-V6-SOURCE-BODY-*` |
| Logical EVENT recovery over stream | prior `FMT-V6-DECODED-EVENT-*` |
| Logical SOURCE recovery via always-inflate stream + optional CRC | **done** (`FMT-V6-DECODED-SOURCE-*`) |
| Logical INDEX recovery over stream | **done** separately (`FMT-V6-DECODED-INDEX-*`) |
| Decoded INDEX | **done** separately (`FMT-V6-DECODED-INDEX-*`) |
| Decoded SUMMARY; full SRC_LINE catalog; C writer | residual / deferred |

---

## Open items (honest residual)

1. Decoded SUMMARY profile consumer path (INDEX: sibling `FMT-V6-DECODED-INDEX-*`).
2. Full SRC_LINE catalog freeze / Perl source-map parity.
3. Mutating default `parse_chunk_frame` to inflate/CRC in place.
4. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.
5. Full COL-007 C writer / COL-008 batched Rust writer.

# Format v6 decoded EVENT profile (stream → inflate → event-body) — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-DECODED-EVENT-PROVISIONAL` (contract), `FMT-V6-DECODED-EVENT-MVP` (shipped encode/decode + tests)  
**Depends on:** decoded-stream (`FMT-V6-DECODED-STREAM-*`); event-body; payload codecs NONE/ZLIB/ZSTD/LZ4; optional CRC  
**Gate:** COL-007 runway preflight only — **before** full opcode catalog freeze / dictionaries / C v6 writer / CLI v6 default

---

## Scope and non-claims

This document freezes a **provisional logical EVENT consumer path**:

```text
wire bytes
  → decode_prefix_chunk_stream_plain (always inflate + optional CRC)
  → collect EVENT plain payloads in order
  → join plains
  → decode_event_body
  → ordered logical OwnedEventRecord list
```

Rules:

- EVENT chunks under **NONE / ZLIB / ZSTD / LZ4** (ids 0–3); optional FOOTER codec NONE last.
- Multi-chunk EVENT is **record-aligned** via shipped `partition_event_records` (join plains then one `decode_event_body`).
- Default `parse_chunk_frame` / non-inflating stream parse stay **unchanged**.
- Provisional opcodes (MARK / TIME_LINE / TIME_BLOCK / SUB_ENTRY / SUB_RETURN / SUB_INFO / SRC_LINE / NEW_FID / PID_START / PID_END / SUB_CALLERS / DISCOUNT / ATTRIBUTE / OPTION / COMMENT / START_DEFLATE / VERSION) — not a full v5 opcode catalog freeze.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- full opcode catalog / dictionaries / dual-equality / FMT-012 golden freeze;
- mutating default `parse_chunk_frame` to always inflate.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Truncated stream / mid-chunk | **Err** |
| CRC mismatch when verify on | **Err** |
| Corrupt compressed EVENT payload | **Err** |
| Truncated joined event-body | **Err** (event-body layer) |
| Unexpected non-EVENT/FOOTER kind | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::decoded_event::{
    decode_decoded_event_profile, encode_decoded_event_profile,
};
use nytprof_format_v6::chunk::codec;

let wire = encode_decoded_event_profile(
    6, 0, 0, 0, 0, &[], codec::ZSTD, &events, /* max_per_chunk */ 0, None,
)?;
let (prof, n) = decode_decoded_event_profile(&wire, /* verify_crc */ true)?;
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Always-inflate multi-chunk stream | prior `FMT-V6-DECODED-STREAM-*` |
| Event-body encode/decode | prior `FMT-V6-EVENT-BODY-*` |
| Compressed mini-profile (no CRC flag) | prior `FMT-V6-COMPRESSED-PROFILE-*` |
| Logical EVENT recovery via always-inflate stream + optional CRC | **done** (`FMT-V6-DECODED-EVENT-*`) |
| Multi-kind always-inflate mixed consumer | **done** separately (`FMT-V6-DECODED-MIXED-*`) |
| Logical SOURCE recovery over stream | **done** separately (`FMT-V6-DECODED-SOURCE-*`) |
| Full opcode catalog / C writer | residual / deferred |

---

## Open items (honest residual)

1. Remaining stream-control / catalog items (full dual-output **sequence-number** freeze OI-001-03 — dump-aligned order recovery + chunk-framed mid-stream codec-switch + auto-emit VERSION + known-key ATTRIBUTE/OPTION preflights are done) beyond those preflights; complete ATTRIBUTE/OPTION key vocabularies (full OI-002-03/04 inventory); COL-015 fork re-init; exact DISCOUNT accounting.
2. Mutating default `parse_chunk_frame` to inflate/CRC in place.
3. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.
4. Full COL-007 C writer / COL-008 batched Rust writer.

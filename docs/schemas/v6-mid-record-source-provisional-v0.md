# Format v6 mid-record spanning across SOURCE chunks (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-MID-RECORD-SOURCE-PROVISIONAL` (contract), `FMT-V6-MID-RECORD-SOURCE-MVP` (shipped encode/decode + tests)  
**Depends on:** multi-chunk SOURCE + payload codecs NONE/ZLIB/ZSTD/LZ4; source-body encode/decode; EVENT mid-record pattern (`FMT-V6-MID-RECORD-SPAN-*`)  
**Gate:** COL-007 runway preflight only — **before** always-on inflate / dictionaries / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** composition path where a single source-body record may **span** two consecutive SOURCE chunk payloads:

```text
[file prefix]
  [SOURCE codec C, payload = body[0..split)]     # ends mid-record
  [SOURCE codec C, payload = body[split..)]      # continues record
  …
  [optional FOOTER codec NONE last]
```

Rules:

- Full logical body is produced by shipped **`encode_source_body`**.
- Body bytes are split at an interior offset (`0 < split_at < body.len()`), so at least one record is incomplete in each piece alone.
- Each piece is sealed as a SOURCE chunk under the same payload codec (**NONE / ZLIB / ZSTD / LZ4**).
- Decode: non-inflating stream parse → `decode_chunk_payload` per SOURCE → **concatenate** plain bytes → single **`decode_source_body`** over the joined buffer.
- `logical_event_count` on the first SOURCE is the full record count; continuation chunks use `0` (provisional).
- Default `parse_chunk_frame` stays **non-inflating**.

It is **not**:

- a wire freeze or product CLI v6 default;
- permission to mark **COL-007** / **COL-008** done;
- INDEX/SUMMARY mid-record: siblings `FMT-V6-MID-RECORD-INDEX-*` / `FMT-V6-MID-RECORD-SUMMARY-*`;
- dictionaries; always-on inflate inside default `parse_chunk_frame`;
- dual-equality vs C or certified perf claims.

---

## Fail-closed decode

| Condition | Result |
|-----------|--------|
| Joined body truncated mid-record | **Err** (source-body layer) |
| Corrupt compressed payload on any SOURCE | **Err** |
| Inflated len ≠ `uncompressed_len` | **Err** (`SizeMismatch`) |
| Fewer than 2 SOURCE chunks on this path | **Err** |
| Never panic | Required |

---

## Shipped API

```rust
use nytprof_format_v6::mid_record_span::{
    decode_mid_record_span_source_profile, encode_mid_record_span_source_profile,
    split_source_body_bytes,
};
use nytprof_format_v6::chunk::codec;

let wire = encode_mid_record_span_source_profile(
    6, 0, 0, 0, 0, &[], codec::ZSTD, &sources, split_at, None,
)?;
let (prof, n) = decode_mid_record_span_source_profile(&wire)?;
```

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Multi-chunk SOURCE (record-aligned partitions) | prior `FMT-V6-MULTI-CHUNK-SOURCE-*` |
| Mid-record span across EVENT chunks | prior `FMT-V6-MID-RECORD-SPAN-*` |
| Mid-record span across SOURCE chunks | **done** (`FMT-V6-MID-RECORD-SOURCE-*`) |
| Mid-record INDEX | **done** separately (`FMT-V6-MID-RECORD-INDEX-*`) |
| Mid-record SUMMARY | **done** separately (`FMT-V6-MID-RECORD-SUMMARY-*`) |
| Always-inflate consumer path | **done** separately (`FMT-V6-DECODED-CHUNK-*`) |
| Always-on inflate in default `parse_chunk_frame` | residual (default parse stays non-inflating) |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. Mid-record INDEX/SUMMARY: sibling preflights (landed).
2. Always-inflate consumer path: sibling `FMT-V6-DECODED-CHUNK-*`; default `parse_chunk_frame` still non-inflating.
3. Dual-equality vs C + FMT-012 golden corpus + default CLI v6.

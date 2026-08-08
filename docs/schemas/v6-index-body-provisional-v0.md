# Format v6 INDEX chunk body (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-INDEX-BODY-PROVISIONAL` (contract), `FMT-V6-INDEX-BODY-MVP` (shipped encode/decode + tests)  
**Depends on:** ULEB128 [`v6-varint-uleb128-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-varint-uleb128-provisional-v0.md); string/blob [`v6-string-blob-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-blob-provisional-v0.md); chunk frame INDEX kind [`v6-chunk-frame-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-chunk-frame-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** payload inflate / full index catalog / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** **INDEX chunk payload** layout under codec **NONE**:

```text
index-body = index-record*
index-record = ULEB128 key_id || ULEB128 file_offset || ULEB128 length
               || string_blob(id, flags, label)
```

It is **not**:

- a permanent wire freeze or full v5 index/catalog parity;
- permission to mark **COL-007** / **COL-008** done;
- payload inflate (zlib / zstd / LZ4);
- SUMMARY body freeze; string dictionaries; CRC verification freeze;
- default CLI report/dump of v6 profiles.

Semantics of `key_id` / `file_offset` / `length` are provisional (lookup anchors only).

---

## Record layout

| Field | Encoding | Notes |
|-------|----------|-------|
| key_id | strict ULEB128 | Provisional key (fid, sub id, …) |
| file_offset | strict ULEB128 | Byte offset into the profile (provisional) |
| length | strict ULEB128 | Span length or count (provisional) |
| label | length-prefixed string/blob | Optional; may be empty |

### Fail-closed rules

| Condition | Result |
|-----------|--------|
| Empty body | **Ok** — zero records |
| Truncated mid-record | **Err** |
| Oversize body (> 64 MiB) | **Err** |
| Never panic | Required |

### Role as codec NONE INDEX chunk payload

An INDEX chunk with `codec = NONE` may carry an index-body encoding. Mixed composition with EVENT + SOURCE (+ optional FOOTER) is provided as a thin helper.

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::index_body::{
    encode_index_body, decode_index_body, IndexRecordSpec,
    encode_mixed_kind_profile, decode_mixed_kind_profile,
};

let body = encode_index_body(&[IndexRecordSpec {
    key_id: 1, file_offset: 100, length: 50,
    string_id: 0, string_flags: 0, label: b"leaf",
}]);
let (recs, n) = decode_index_body(&body)?;

// Mixed EVENT + SOURCE + INDEX + optional FOOTER
let file = encode_mixed_kind_profile(6, 0, 0, 0, 0, &[], &events, &sources, &indexes, None);
let (prof, n) = decode_mixed_kind_profile(&file)?;
```

- Composes **shipped** `encode_u64` / `decode_u64` and `encode_string_blob` / `decode_string_blob`.
- Pure byte-slice / `Vec` APIs (no I/O). Not wired into default CLI.
- Pure-EVENT mini-profile / multi-chunk EVENT paths remain fail-closed on INDEX kind (unchanged honesty).

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Chunk kind INDEX (frame) | done (`FMT-V6-CHUNK-*`) |
| INDEX-body codec NONE | **done** (`FMT-V6-INDEX-BODY-*`) |
| EVENT + SOURCE + INDEX composition helper | **done** (same board MVP; now also SUMMARY via `encode_mixed_kind_profile`) |
| SUMMARY-body codec NONE | **done** separately (`FMT-V6-SUMMARY-BODY-*`) |
| Full catalog / CRC / inflate | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. ADR freeze of INDEX field semantics and multi-INDEX chunk policy.
2. SUMMARY payload schema.
3. Payload inflate + dual-equality vs C + FMT-012 golden corpus.
4. Default CLI v6 read path.

# Format v6 SUMMARY chunk body (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-SUMMARY-BODY-PROVISIONAL` (contract), `FMT-V6-SUMMARY-BODY-MVP` (shipped encode/decode + tests)  
**Depends on:** ULEB128 [`v6-varint-uleb128-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-varint-uleb128-provisional-v0.md); string/blob [`v6-string-blob-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-blob-provisional-v0.md); chunk frame SUMMARY kind [`v6-chunk-frame-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-chunk-frame-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** payload inflate / full summary catalog / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** **SUMMARY chunk payload** layout under codec **NONE**:

```text
summary-body = summary-record*
summary-record = ULEB128 key_id || ULEB128 count || ULEB128 value
                 || string_blob(id, flags, label)
```

It is **not**:

- a permanent wire freeze or full v5 aggregate/summary catalog parity;
- permission to mark **COL-007** / **COL-008** done;
- payload inflate (zlib / zstd / LZ4);
- string dictionaries; CRC verification freeze;
- default CLI report/dump of v6 profiles.

Semantics of `key_id` / `count` / `value` are provisional (aggregate anchors only).

---

## Record layout

| Field | Encoding | Notes |
|-------|----------|-------|
| key_id | strict ULEB128 | Provisional key (sub id, fid, bucket, …) |
| count | strict ULEB128 | Count (e.g. calls / events) |
| value | strict ULEB128 | Ticks or other aggregate value |
| label | length-prefixed string/blob | Optional; may be empty |

### Fail-closed rules

| Condition | Result |
|-----------|--------|
| Empty body | **Ok** — zero records |
| Truncated mid-record | **Err** |
| Oversize body (> 64 MiB) | **Err** |
| Never panic | Required |

### Role as codec NONE SUMMARY chunk payload

A SUMMARY chunk with `codec = NONE` may carry a summary-body encoding. Mixed composition with EVENT + SOURCE + INDEX (+ optional FOOTER) is provided via `encode_mixed_kind_profile` / `decode_mixed_kind_profile`.

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::summary_body::{
    encode_summary_body, decode_summary_body, SummaryRecordSpec,
};
use nytprof_format_v6::index_body::{encode_mixed_kind_profile, decode_mixed_kind_profile};

let body = encode_summary_body(&[SummaryRecordSpec {
    key_id: 1, count: 15, value: 1000,
    string_id: 0, string_flags: 0, label: b"main::leaf",
}]);
let (recs, n) = decode_summary_body(&body)?;

// Mixed EVENT + SOURCE + INDEX + SUMMARY + optional FOOTER
let file = encode_mixed_kind_profile(
    6, 0, 0, 0, 0, &[], &events, &sources, &indexes, &summaries, None,
);
let (prof, n) = decode_mixed_kind_profile(&file)?;
```

- Composes **shipped** `encode_u64` / `decode_u64` and `encode_string_blob` / `decode_string_blob`.
- Pure byte-slice / `Vec` APIs (no I/O). Not wired into default CLI.
- Pure-EVENT mini-profile / multi-chunk EVENT paths remain fail-closed on SUMMARY kind (unchanged honesty).

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Chunk kind SUMMARY (frame) | done (`FMT-V6-CHUNK-*`) |
| SUMMARY-body codec NONE | **done** (`FMT-V6-SUMMARY-BODY-*`) |
| EVENT + SOURCE + INDEX + SUMMARY composition | **done** (`encode_mixed_kind_profile`) |
| FOOTER-body codec NONE | **done** separately (`FMT-V6-FOOTER-BODY-*`) |
| Full aggregate catalog / CRC / inflate | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. ADR freeze of SUMMARY field semantics and multi-SUMMARY chunk policy.
2. Payload inflate + dual-equality vs C + FMT-012 golden corpus.
3. Default CLI v6 read path.

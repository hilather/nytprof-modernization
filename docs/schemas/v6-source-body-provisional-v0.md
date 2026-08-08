# Format v6 SOURCE chunk body (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-SOURCE-BODY-PROVISIONAL` (contract), `FMT-V6-SOURCE-BODY-MVP` (shipped encode/decode + tests)  
**Depends on:** ULEB128 [`v6-varint-uleb128-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-varint-uleb128-provisional-v0.md); string/blob [`v6-string-blob-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-blob-provisional-v0.md); chunk frame SOURCE kind [`v6-chunk-frame-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-chunk-frame-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** payload inflate / full source catalog / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** **SOURCE chunk payload** layout under codec **NONE**:

```text
source-body = source-record*
source-record = ULEB128 fid || ULEB128 line || string_blob(id, flags, text)
```

It is **not**:

- a permanent wire freeze or full v5 `SRC_LINE` catalog parity;
- permission to mark **COL-007** / **COL-008** done;
- payload inflate (zlib / zstd / LZ4);
- INDEX / SUMMARY full schemas; string dictionaries; CRC verification freeze;
- default CLI report/dump of v6 profiles.

---

## Record layout

| Field | Encoding | Notes |
|-------|----------|-------|
| fid | strict ULEB128 | File id |
| line | strict ULEB128 | Source line number |
| text | length-prefixed string/blob | Composes `encode_string_blob` / `decode_string_blob` |

### Fail-closed rules

| Condition | Result |
|-----------|--------|
| Empty body | **Ok** — zero records |
| Truncated mid-record (missing line/text or mid-string) | **Err** |
| Oversize body (> 64 MiB) | **Err** |
| Never panic on crafted bodies | Required |

### Role as codec NONE SOURCE chunk payload

A SOURCE chunk with `codec = NONE` may carry a source-body encoding as its payload. Mixed composition with EVENT (+ optional FOOTER) is provided as a thin helper (not a wire freeze of multi-kind layout).

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::source_body::{
    encode_source_body, decode_source_body, SourceRecordSpec,
    encode_event_source_profile, decode_event_source_profile,
};

let body = encode_source_body(&[SourceRecordSpec {
    fid: 1, line: 5, string_id: 0, string_flags: 0, text: b"$x++",
}]);
let (recs, n) = decode_source_body(&body)?;

// Mixed EVENT + SOURCE + optional FOOTER composition
let file = encode_event_source_profile(6, 0, 0, 0, 0, &[], &events, &sources, None);
let (prof, n) = decode_event_source_profile(&file)?;
```

- Composes **shipped** `encode_u64` / `decode_u64` and `encode_string_blob` / `decode_string_blob`.
- Pure byte-slice / `Vec` APIs (no I/O). Not wired into default CLI.
- Pure-EVENT mini-profile / multi-chunk EVENT paths remain fail-closed on SOURCE kind (unchanged honesty).

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Chunk kind SOURCE (frame) | done (`FMT-V6-CHUNK-*`) |
| SOURCE-body codec NONE | **done** (`FMT-V6-SOURCE-BODY-*`) |
| EVENT + SOURCE composition helper | **done** (same board MVP) |
| INDEX-body codec NONE | **done** separately (`FMT-V6-INDEX-BODY-*`) |
| SUMMARY body | residual |
| Full v5 SRC_LINE catalog / dictionaries | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. ADR freeze of SOURCE record fields and multi-SOURCE chunk policy.
2. INDEX / SUMMARY payload schemas.
3. Payload inflate + dual-equality vs C + FMT-012 golden corpus.
4. Default CLI v6 read path.

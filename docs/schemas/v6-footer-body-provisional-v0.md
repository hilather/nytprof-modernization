# Format v6 FOOTER chunk body (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-FOOTER-BODY-PROVISIONAL` (contract), `FMT-V6-FOOTER-BODY-MVP` (shipped encode/decode + tests)  
**Depends on:** ULEB128 [`v6-varint-uleb128-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-varint-uleb128-provisional-v0.md); string/blob [`v6-string-blob-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-blob-provisional-v0.md); chunk frame FOOTER kind [`v6-chunk-frame-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-chunk-frame-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** payload inflate / CRC freeze / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** **FOOTER chunk payload** layout under codec **NONE**:

```text
footer-body = footer-record*
footer-record = ULEB128 key_id || ULEB128 value || string_blob(id, flags, label)
```

It is **not**:

- a permanent wire freeze or full end-of-file catalog parity;
- permission to mark **COL-007** / **COL-008** done;
- payload inflate (zlib / zstd / LZ4);
- header/payload **CRC verification** freeze;
- default CLI report/dump of v6 profiles.

Semantics of `key_id` / `value` are provisional (end totals / markers only).

---

## Record layout

| Field | Encoding | Notes |
|-------|----------|-------|
| key_id | strict ULEB128 | Provisional key (e.g. total_events marker) |
| value | strict ULEB128 | Provisional counter/total/value |
| label | length-prefixed string/blob | Optional; may be empty |

### Fail-closed rules

| Condition | Result |
|-----------|--------|
| Empty body | **Ok** — zero records (compat with opaque empty FOOTER) |
| Truncated mid-record | **Err** |
| Oversize body (> 64 MiB) | **Err** |
| Never panic | Required |

### Last-chunk role

In mixed-kind profiles, FOOTER (when present) must be the **last** chunk. Anything after FOOTER is fail-closed (`InvalidFooter`). Empty FOOTER body remains valid.

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::footer_body::{
    encode_footer_body, decode_footer_body, FooterRecordSpec,
};
use nytprof_format_v6::index_body::{encode_mixed_kind_profile, decode_mixed_kind_profile};

let body = encode_footer_body(&[FooterRecordSpec {
    key_id: 1, value: 2474,
    string_id: 0, string_flags: 0, label: b"total_events",
}]);
let (recs, n) = decode_footer_body(&body)?;

// Mixed profile with structured FOOTER last
let file = encode_mixed_kind_profile(
    6, 0, 0, 0, 0, &[],
    &events, &sources, &indexes, &summaries,
    Some(&footers), // None = no FOOTER; Some(&[]) = empty FOOTER body
);
let (prof, n) = decode_mixed_kind_profile(&file)?;
// prof.footer_records, prof.has_footer
```

- Composes **shipped** `encode_u64` / `decode_u64` and `encode_string_blob` / `decode_string_blob`.
- Pure byte-slice / `Vec` APIs (no I/O). Not wired into default CLI.
- Mini-profile / multi-chunk EVENT paths may still use opaque FOOTER bytes separately; pure-EVENT paths remain fail-closed on non-EVENT/FOOTER kinds as before.

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Chunk kind FOOTER (frame) | done (`FMT-V6-CHUNK-*`) |
| FOOTER-body codec NONE | **done** (`FMT-V6-FOOTER-BODY-*`) |
| Mixed composition with structured FOOTER last | **done** (`encode_mixed_kind_profile`) |
| CRC32 optional compute/verify | **done** separately (`FMT-V6-CRC-*`; default parse still non-verify) |
| Permanent CRC ADR / always-on verify | residual |
| Full catalog / inflate / CLI v6 | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. ADR freeze of FOOTER field semantics and required totals set.
2. Payload/header CRC verification.
3. Dual-equality vs C + FMT-012 golden corpus.
4. Default CLI v6 read path.

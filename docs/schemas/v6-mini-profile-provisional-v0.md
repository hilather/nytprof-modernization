# Format v6 mini-profile composition (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-MINI-PROFILE-PROVISIONAL` (contract), `FMT-V6-MINI-PROFILE-MVP` (shipped compose encode/decode + tests)  
**Depends on:** file prefix [`v6-file-prefix-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-file-prefix-provisional-v0.md); prefix+chunk stream [`v6-prefix-chunk-stream-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-prefix-chunk-stream-provisional-v0.md); event-body [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); chunk frame [`v6-chunk-frame-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-chunk-frame-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** payload inflate / full catalog / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** minimal **complete v6 byte profile** composition:

```text
[ file prefix ][ EVENT chunk (codec NONE, event-body payload) … ][ optional FOOTER chunk ]
```

It is **not**:

- a permanent wire freeze or golden FMT-012 corpus freeze;
- permission to mark **COL-007** (C v6 writer) or **COL-008** done;
- payload inflate (zlib / zstd / LZ4);
- full v5-equivalent opcode catalog, dictionaries, location deltas, or CRC verification freeze;
- default CLI report/dump of v6 profiles.

Layout may change under future ADR + golden vectors.

---

## Layout rules (MVP)

| Region | Content |
|--------|---------|
| File prefix | Fixed header + multi-TLV … **END** |
| EVENT chunks | Zero or more; this encode MVP emits **0** when no events, else **one** EVENT chunk with codec **NONE** and payload = event-body encoding |
| FOOTER | Optional; at most one; must be **last**; codec **NONE**; payload opaque (may be empty) |

### Fail-closed composition

| Condition | Result |
|-----------|--------|
| Bad magic / truncated prefix / missing END | **Err** (file-prefix / stream) |
| Truncated mid-chunk / bad chunk sync | **Err** (chunk) |
| Truncated mid-event-body inside EVENT payload | **Err** (event-body) |
| EVENT with codec ≠ NONE | **Err** (`UnexpectedCodec`) |
| Chunk kind other than EVENT / FOOTER | **Err** (`UnexpectedKind`) |
| FOOTER not last / multiple FOOTERs | **Err** (`InvalidFooter`) |
| Never panic on crafted profiles | Required |

Header / payload CRC remain placeholders (not verified).

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::mini_profile::{encode_mini_profile, decode_mini_profile};
use nytprof_format_v6::event_body::EventRecordSpec;

let bytes = encode_mini_profile(
    6, 0, 0, 0, 0,
    &[],
    &[EventRecordSpec::TimeLine { fid: 1, line: 5, ticks: 42 }],
    Some(b""), // optional FOOTER
);
let (profile, n) = decode_mini_profile(&bytes)?;
// profile.prefix, profile.records, profile.has_footer
```

- Composes **shipped** `encode_prefix_chunk_stream` / `decode_prefix_chunk_stream`, `encode_event_body` / `decode_event_body`, and chunk frame APIs.
- Pure byte-slice / `Vec` APIs (no I/O). Not wired into default CLI.

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Prefix + chunk stream + event-body preflight | done |
| Mini-profile composition | **done** (`FMT-V6-MINI-PROFILE-*`) |
| Multi-chunk EVENT split/reassemble | **done** separately (`FMT-V6-MULTI-CHUNK-EVENT-*`) |
| Payload inflate / full catalog / dictionaries | compressed mini-profile preflight **done** separately (`FMT-V6-COMPRESSED-PROFILE-*`); full catalog / dictionaries residual |
| C v6 writer (**COL-007**) | **still deferred** |
| Golden full-file corpus freeze (FMT-012) | residual (this is preflight only) |

---

## Open items (honest residual)

1. ADR freeze of full-file layout (required FOOTER, multi-EVENT splitting policy, padding).
2. Multi-chunk EVENT splitting: **done** as preflight (`FMT-V6-MULTI-CHUNK-EVENT-*`); SOURCE/INDEX/SUMMARY kinds residual.
3. Compressed multi-codec mini-profile: **done** as preflight (`FMT-V6-COMPRESSED-PROFILE-*`); always-on default inflate + dual-equality vs C residual.
4. Golden full-file corpus (FMT-012) and default CLI v6 read path.

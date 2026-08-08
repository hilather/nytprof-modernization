# Format v6 multi-chunk EVENT body framing (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-MULTI-CHUNK-EVENT-PROVISIONAL` (contract), `FMT-V6-MULTI-CHUNK-EVENT-MVP` (shipped split/reassemble encode/decode + tests)  
**Depends on:** event-body [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); prefix+chunk stream [`v6-prefix-chunk-stream-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-prefix-chunk-stream-provisional-v0.md); mini-profile [`v6-mini-profile-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-mini-profile-provisional-v0.md); chunk frame [`v6-chunk-frame-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-chunk-frame-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** payload inflate / full catalog / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** rule for splitting one ordered **event-body** record stream across **two or more** codec-NONE **EVENT** chunks, and reassembling records on decode in **file/chunk sequence order**.

```text
[ file prefix ][ EVENT body₀ ][ EVENT body₁ ]…[ optional FOOTER ]
                 \___________/ \___________/
                 encode_event_body partitions (records-per-chunk)
```

It is **not**:

- a permanent wire freeze or mandatory multi-chunk policy for all writers;
- permission to mark **COL-007** / **COL-008** done;
- payload inflate (zlib / zstd / LZ4);
- mid-record split across chunks (each EVENT payload is a complete event-body);
- full opcode catalog, dictionaries, CRC freeze, or default CLI v6 read.

---

## Provisional split rule (records-per-chunk)

| Parameter | Behavior |
|-----------|----------|
| `max_records_per_chunk == 0` | Unlimited → **one** EVENT chunk for all records (single-chunk / mini-profile compat) |
| `max_records_per_chunk >= 1` | Consecutive windows of at most that many records; last window may be shorter |
| Empty record list | **Zero** EVENT chunks (prefix-only event section) |

Each partition is encoded with shipped `encode_event_body` as one EVENT payload (`codec = NONE`). Chunk `sequence` is `0 .. event_chunk_count-1` in order.

### Reassembly

Decode walks EVENT chunks in file order and appends `decode_event_body` records. Order of records after reassembly equals the original encode order.

### Fail-closed

| Condition | Result |
|-----------|--------|
| Bad magic / truncated prefix | **Err** (stream / prefix) |
| Truncated mid-chunk / bad sync | **Err** (chunk) |
| Truncated mid-record inside an EVENT body | **Err** (event-body) |
| EVENT with codec ≠ NONE | **Err** |
| Unexpected kind / FOOTER not last | **Err** |
| Never panic | Required |

Records are **not** split mid-record across chunks in this MVP.

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::multi_chunk_event::{
    encode_multi_chunk_event_profile, decode_multi_chunk_event_profile,
    partition_event_records,
};
use nytprof_format_v6::event_body::EventRecordSpec;

let bytes = encode_multi_chunk_event_profile(
    6, 0, 0, 0, 0,
    &[],
    &events,
    2,          // max records per EVENT chunk
    None,       // optional FOOTER
);
let (prof, n) = decode_multi_chunk_event_profile(&bytes)?;
// prof.records ordered; prof.event_chunk_count >= 2 when events.len() > 2
```

- Composes **shipped** `encode_event_body` / `decode_event_body` and `encode_prefix_chunk_stream` / `decode_prefix_chunk_stream`.
- Pure byte-slice / `Vec` APIs (no I/O). Not wired into default CLI.

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Single-chunk mini-profile | done (`FMT-V6-MINI-PROFILE-*`) |
| Multi-chunk EVENT split/reassemble | **done** (`FMT-V6-MULTI-CHUNK-EVENT-*`) |
| Mid-record spanning / byte-budget split | residual |
| Payload inflate / full catalog | residual |
| C v6 writer (**COL-007**) | **still deferred** |

---

## Open items (honest residual)

1. ADR freeze of split policy (records vs payload-byte budget vs fixed size).
2. Mid-record continuation frames (not in this MVP).
3. SOURCE body codec NONE: **done** separately (`FMT-V6-SOURCE-BODY-*`); INDEX/SUMMARY multi-chunk residual.
4. Payload inflate + dual-equality vs C + FMT-012 golden corpus.

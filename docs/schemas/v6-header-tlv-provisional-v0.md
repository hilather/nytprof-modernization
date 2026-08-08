# Format v6 header TLV frame (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-TLV-PROVISIONAL` (contract), `FMT-V6-TLV-MVP` (shipped encode/decode + tests)  
**Depends on:** plan draft [`docs/plan/04_FILE_FORMAT_V6_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/04_FILE_FORMAT_V6_TASKS.md) §3.2; fixed header [`v6-fixed-header-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-fixed-header-provisional-v0.md); ULEB128 [`v6-varint-uleb128-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-varint-uleb128-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** full TLV catalog / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** Type-Length-Value (TLV) frame for the region **after** the v6 fixed header (header TLVs / metadata acceleration).

It is **not**:

- a permanent wire freeze or ADR-ratified TLV catalog;
- permission to mark **COL-007** done;
- an exhaustive list of every candidate TLV (producer, clock, UUID, …) — a small provisional registry with ≥1 known type is enough;
- event-stream codecs or dictionaries.

---

## On-wire layout (one TLV)

```text
type_id        : ULEB128 u64
value_length   : ULEB128 u64   (number of following value bytes)
flags          : u8            (required-type bit + reserved)
value          : value_length octets
```

Integer fields use **strict canonical ULEB128**.

### Flags (provisional `u8`)

| Bit | Mask | Name | Meaning |
|----:|-----:|------|---------|
| 0 | `0x01` | `FLAG_TYPE_REQUIRED` | If set and `type_id` is **not** a known provisional type → **Err**. If clear, unknown type → **Ok** with `known_type = false` (skip-with-honesty). |
| 1–7 | | reserved | Accept and ignore unknown bits. |

### Length bounds (fail closed)

| Bound | Value |
|-------|------:|
| `MAX_TLV_VALUE_BYTES` | **16 MiB** |

| Condition | Result |
|-----------|--------|
| Truncated while reading type / length / flags | **Err** truncated |
| `value_length > MAX_TLV_VALUE_BYTES` | **Err** oversize |
| Remaining buffer shorter than `value_length` | **Err** truncated |
| Unknown type **and** `FLAG_TYPE_REQUIRED` | **Err** unknown required type |
| otherwise | **Ok** |

Decoders must not allocate based on `value_length` before the oversize check (MVP borrows the input slice).

### Provisional known type ids

| Value | Name | Notes |
|------:|------|-------|
| 0 | reserved | Invalid as type → **Err** |
| 1 | `PRODUCER` | Producer name/version hint (opaque bytes; often UTF-8 string blob later) |
| 2 | `TICKS_PER_SEC` | File-level ticks_per_sec hint (opaque / later integer) |
| other | unknown | See required flag |

This is intentionally small — full §3.2 catalog is residual.

---

## Encode / decode

| Operation | API |
|-----------|-----|
| Encode | `encode_tlv(type_id, flags, value) -> Vec<u8>` |
| Decode | `decode_tlv(buf, pos) -> Result<(Tlv, bytes_consumed)>` |

`Tlv` fields: `type_id`, `flags`, `known_type`, `value: &[u8]`.

---

## Shipped entry points

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::tlv::{encode_tlv, decode_tlv, type_id, FLAG_TYPE_REQUIRED};

let bytes = encode_tlv(type_id::PRODUCER, 0, b"nytprof-rust");
let (tlv, n) = decode_tlv(&bytes, 0)?;
```

- Pure byte-slice / `Vec` APIs (no I/O).
- Tests: known type empty/non-empty round-trip; truncated; oversize; unknown required type.

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Fixed header / chunk / varints / string preflight | done |
| Header TLV frame contract + API | **done** (`FMT-V6-TLV-*`) |
| C v6 writer (COL-007) | **still deferred** |
| Full TLV catalog / CRC / dictionaries / event codecs | residual |
| COL-008 / FFI / multi-OS CI / perf claims | residual |

---

## Open items (honest residual)

1. ADR freeze of type id space and flag bits.
2. Typed value payloads (e.g. string blob inside PRODUCER, integer for ticks).
3. Exhaustive §3.2 TLV list + golden vectors.
4. Header region terminator / end-of-TLVs marker (not required for single-TLV MVP).

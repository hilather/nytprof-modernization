# Format v6 length-prefixed string / byte blob (provisional) — v0

**Status:** numeric IDs / core frame **frozen** for major=6 by [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md) + catalog [`v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md); filename retains `provisional-v0` for link stability; **not** CLI v6 default / E3-mixed / COL-008  
**Board IDs:** `FMT-V6-STRING-PROVISIONAL` (contract), `FMT-V6-STRING-MVP` (shipped encode/decode + tests)  
**Depends on:** plan draft [`docs/plan/04_FILE_FORMAT_V6_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/04_FILE_FORMAT_V6_TASKS.md) §5.4; ULEB128 [`v6-varint-uleb128-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-varint-uleb128-provisional-v0.md)  
**Gate:** IDs frozen after E3-EVENT(C)+E4-v0 (ADR-0006). Residual: CLI v6 default; default-parse non-inflate; E3-mixed; COL-008; full OI-002 vocabulary  

---

## Scope and non-claims

This document is the detailed layout home for **length-prefixed string/blob frame + FLAG_UTF8**. Numeric IDs and the core frame described here are **frozen for major=6** by ADR-0006 (see frozen catalog). Filename retains `provisional-v0` for stable links.

It is **not**:

- permission to flip CLI v6 / collection default (still v5 until R4 ADR);
- E3-mixed multi-kind product C fixture claim;
- COL-008 batched Rust writer;
- default-parse always-inflate / CRC default flip;
- complete OI-002 ATTRIBUTE/OPTION key vocabulary;
- a new major without ADR supersession (renumbering requires major bump).

Independent C/Rust implementations must match the frozen IDs and this layout. Golden vectors: [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/).

## On-wire layout

```text
string_id      : ULEB128 u64   (dictionary id or provisional local id; 0 allowed)
byte_length    : ULEB128 u64   (number of following payload bytes)
flags          : u8            (see flag bits)
bytes          : byte_length octets
```

All multi-byte integer fields use **strict canonical ULEB128** (`encode_u64` / `decode_u64`).

### Flags (provisional `u8`)

| Bit | Mask | Name | Meaning |
|----:|-----:|------|---------|
| 0 | `0x01` | `FLAG_UTF8` | Payload is claimed UTF-8 text (semantic flag for Perl-facing callbacks later). **MVP does not validate UTF-8** when the bit is set. |
| 1–7 | | reserved | Must be accepted without error; ignore unknown bits for now. |

Constant: `FLAG_UTF8 = 0x01`.

### Length bounds (fail closed)

| Bound | Value |
|-------|------:|
| `MAX_STRING_BYTES` | **16 MiB** (`16 * 1024 * 1024`) |

| Condition | Result |
|-----------|--------|
| Truncated while reading id / length / flags | **Err** truncated (via ULEB128 or missing flag byte) |
| `byte_length > MAX_STRING_BYTES` | **Err** oversize |
| Remaining buffer shorter than `byte_length` | **Err** truncated payload |
| otherwise | **Ok** with borrowed payload subslice |

Decoders **must not** allocate based on `byte_length` before the oversize check (shipped MVP only borrows the input slice).

---

## Encode / decode

| Operation | API |
|-----------|-----|
| Encode | `encode_string_blob(id, flags, bytes) -> Vec<u8>` |
| Decode | `decode_string_blob(buf, pos) -> Result<(StringBlob, bytes_consumed)>` |

`StringBlob` fields: `id: u64`, `flags: u8`, `data: &[u8]` (length = declared `byte_length`).

### Examples

| Case | Notes |
|------|-------|
| Empty blob, id 0, flags 0 | id `00`, len `00`, flags `00`, no payload |
| UTF-8 claim, id 1, `"hi"` | id `01`, len `02`, flags `01`, bytes `68 69` |

---

## Shipped entry points

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::string::{encode_string_blob, decode_string_blob, FLAG_UTF8};

let bytes = encode_string_blob(1, FLAG_UTF8, b"hi");
let (blob, n) = decode_string_blob(&bytes, 0)?;
assert_eq!(blob.data, b"hi");
```

- Pure byte-slice / `Vec` APIs (no I/O).
- Tests: empty + non-empty round-trip; truncated; oversize declared length.

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Header / chunk / ULEB128 / ZigZag signed preflight | done |
| Length-prefixed string/blob contract + API | **done** (`FMT-V6-STRING-*`) |
| C v6 writer (COL-007) | **still deferred** |
| Local string-dictionary intern preflight | **done** (`FMT-V6-STRING-DICTIONARY-*`; not permanent global pool) |
| Permanent global pool / event-opcode catalog freezes | residual |
| COL-008 / FFI / multi-OS CI / perf claims | residual |

---

## Open items (honest residual)

1. ADR freeze of flag bits and max length.
2. UTF-8 validation policy when `FLAG_UTF8` is set.
3. Permanent global / cross-file string-pool ADR freeze (local dictionary intern preflight is shipped separately as `FMT-V6-STRING-DICTIONARY-*`).
4. Immutable golden vectors (FMT-012).

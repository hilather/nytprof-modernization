# Format v6 length-prefixed string / byte blob (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-STRING-PROVISIONAL` (contract), `FMT-V6-STRING-MVP` (shipped encode/decode + tests)  
**Depends on:** plan draft [`docs/plan/04_FILE_FORMAT_V6_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/04_FILE_FORMAT_V6_TASKS.md) §5.4; ULEB128 [`v6-varint-uleb128-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-varint-uleb128-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** string dictionaries / event codecs / C v6 writer

---

## Scope and non-claims

This document freezes a **provisional** on-wire form for **length-prefixed strings or opaque byte blobs** (definitions that may later be interned in a dictionary).

It is **not**:

- a permanent wire freeze or ADR-ratified string table;
- permission to mark **COL-007** done;
- permanent global string pools (local dictionary intern preflight is a sibling `FMT-V6-STRING-DICTIONARY-*`);
- a claim that every `string_id` is globally unique across files yet.

---

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

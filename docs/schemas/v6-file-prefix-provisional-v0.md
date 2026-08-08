# Format v6 file prefix (provisional) — v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-FILE-PREFIX-PROVISIONAL` (contract), `FMT-V6-FILE-PREFIX-MVP` (shipped compose encode/decode + tests)  
**Depends on:** fixed header [`v6-fixed-header-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-fixed-header-provisional-v0.md); multi-TLV region [`v6-tlv-region-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-tlv-region-provisional-v0.md)  
**Gate:** COL-007 runway preflight only — **before** event codecs / C v6 writer (prefix+chunk stream: `FMT-V6-PREFIX-CHUNK-STREAM-*`)

---

## Scope and non-claims

This document freezes a **provisional** rule for the **start of a v6 profile file**:

```text
[ fixed header ][ multi-TLV header region … END ][ … later: chunks … ]
```

It is **not** a wire freeze, full COL-007 encoder, or header CRC verification. Chunk stream after prefix is composed separately (`FMT-V6-PREFIX-CHUNK-STREAM-*` / `v6-prefix-chunk-stream-provisional-v0.md`).

---

## Layout

| Region | Content | Schema |
|--------|---------|--------|
| Fixed header | Magic `NYTPROF6`, major/minor, `header_len`, optional features/CRC fields | `v6-fixed-header-provisional-v0.md` |
| Multi-TLV region | Zero or more payload TLVs + **END** terminator | `v6-tlv-region-provisional-v0.md` |

### Provisional placement rule (`header_len`)

| Rule | Detail |
|------|--------|
| Fixed-header size | Declared by `header_len` (full provisional layout uses **36** = `HEADER_LEN_FULL`) |
| TLV region start | **Immediately after** the fixed header, at file offset **`header_len`** |
| Encode MVP | Always emits full 36-byte fixed header then `encode_tlv_region` |

Future freezes may allow `header_len > 36` with reserved padding before TLVs; this MVP does not require padding.

### Fail-closed composition

| Condition | Result |
|-----------|--------|
| Bad magic / unsupported major / invalid `header_len` / truncated fixed header | **Err** from fixed-header parse |
| Truncated mid-TLV / missing END / oversize region / unknown required type | **Err** from multi-TLV region parse |
| Never panic on crafted prefixes | Required |

Header CRC remains a **placeholder** (not verified).

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::file_prefix::{encode_file_prefix, decode_file_prefix};
use nytprof_format_v6::tlv::type_id;

let bytes = encode_file_prefix(6, 0, 0, 0, 0, &[
    (type_id::PRODUCER, 0, b"nytprof-rust"),
]);
let (prefix, n) = decode_file_prefix(&bytes)?;
// prefix.header, prefix.tlvs (no END); n includes END
```

- Composes **shipped** `encode_fixed_header_full` / `parse_fixed_header` and `encode_tlv_region` / `decode_tlv_region`.
- Pure byte-slice / `Vec` APIs (no I/O).

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Fixed header + multi-TLV region APIs | done |
| File-prefix composition | **done** (`FMT-V6-FILE-PREFIX-*`) |
| Chunk stream after prefix | **done** separately (`FMT-V6-PREFIX-CHUNK-STREAM-*`) |
| C v6 writer (COL-007) | **still deferred** |
| Full freeze / CRC verify / catalogs | residual |

---

## Open items (honest residual)

1. ADR freeze of `header_len` vs padding vs TLV start.
2. Header CRC verification.
3. Chunk stream after prefix: **done** as preflight (`FMT-V6-PREFIX-CHUNK-STREAM-*` / `v6-prefix-chunk-stream-provisional-v0.md`) — still no payload codecs / COL-007.
4. Golden full-file corpus (FMT-012).

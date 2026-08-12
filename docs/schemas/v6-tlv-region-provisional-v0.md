# Format v6 multi-TLV header region (provisional) — v0

**Status:** numeric IDs / core frame **frozen** for major=6 by [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md) + catalog [`v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md); filename retains `provisional-v0` for link stability; **not** CLI v6 default / E3-mixed / COL-008  
**Board IDs:** `FMT-V6-TLV-REGION-PROVISIONAL` (contract), `FMT-V6-TLV-REGION-MVP` (shipped encode/decode + tests)  
**Depends on:** single-TLV frame [`v6-header-tlv-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-header-tlv-provisional-v0.md); fixed header [`v6-fixed-header-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-fixed-header-provisional-v0.md)  
**Gate:** IDs frozen after E3-EVENT(C)+E4-v0 (ADR-0006). Residual: CLI v6 default; default-parse non-inflate; E3-mixed; COL-008; full OI-002 vocabulary  

---

## Scope and non-claims

This document is the detailed layout home for **multi-TLV region + END terminator**. Numeric IDs and the core frame described here are **frozen for major=6** by ADR-0006 (see frozen catalog). Filename retains `provisional-v0` for stable links.

It is **not**:

- permission to flip CLI v6 / collection default (still v5 until R4 ADR);
- E3-mixed multi-kind product C fixture claim;
- COL-008 batched Rust writer;
- default-parse always-inflate / CRC default flip;
- complete OI-002 ATTRIBUTE/OPTION key vocabulary;
- a new major without ADR supersession (renumbering requires major bump).

Independent C/Rust implementations must match the frozen IDs and this layout. Golden vectors: [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/).

## Region layout

```text
+------------------+
| TLV 0            |  (single-TLV frame; not type END)
+------------------+
| TLV 1            |
+------------------+
| ...              |
+------------------+
| END terminator   |  type_id = END (0x7E), value_length = 0, flags = 0
+------------------+
```

Each payload TLV uses the single-TLV frame from `v6-header-tlv-provisional-v0.md` (ULEB128 type/length, flags, value).

### Terminator

| Field | Provisional value |
|-------|-------------------|
| `type_id` | **`END` = `0x7E`** |
| `value_length` | **0** (empty value required) |
| `flags` | **0** (required bit must not create unknown-required failure — END is a **known** type) |

An END terminator with non-empty value → **Err** invalid terminator.

### Empty region

Zero payload TLVs + END only is valid (empty metadata region).

### Bounds (fail closed)

| Bound | Value |
|-------|------:|
| Per-TLV value | `MAX_TLV_VALUE_BYTES` (16 MiB) — inherited |
| Total region size | `MAX_TLV_REGION_BYTES` (**64 MiB**) including terminator |

| Condition | Result |
|-----------|--------|
| Truncated mid-TLV | **Err** truncated (via single-TLV) |
| EOF without END | **Err** missing terminator |
| Region length > `MAX_TLV_REGION_BYTES` | **Err** region oversize |
| Unknown type + `FLAG_TYPE_REQUIRED` | **Err** unknown required type |
| Reserved type 0 as payload | **Err** invalid type |

Optional unknown types (no required flag) are included in the payload list with `known_type = false`.

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::tlv::{encode_tlv_region, decode_tlv_region, type_id};

let bytes = encode_tlv_region(&[
    (type_id::PRODUCER, 0, b"nytprof-rust"),
    (type_id::TICKS_PER_SEC, 0, b"10000000"),
]);
let (tlvs, n) = decode_tlv_region(&bytes, 0)?;
// tlvs excludes END; n includes END
```

- Composes **shipped** `encode_tlv` / `decode_tlv` (no reimplementation).
- Pure byte-slice / `Vec` APIs (no I/O).

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Single-TLV frame | done (`FMT-V6-TLV-*`) |
| Multi-TLV region + END | **done** (`FMT-V6-TLV-REGION-*`) |
| C v6 writer (COL-007) | **still deferred** |
| Full catalog / CRC / event codecs | residual |

---

## Open items (honest residual)

1. ADR freeze of END id and region max.
2. Typed TLV value payloads.
3. Exhaustive §3.2 catalog + golden corpus.

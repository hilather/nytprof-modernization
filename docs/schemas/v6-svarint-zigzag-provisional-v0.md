# Format v6 signed varint — ZigZag + ULEB128 (provisional) — v0

**Status:** numeric IDs / core frame **frozen** for major=6 by [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md) + catalog [`v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md); filename retains `provisional-v0` for link stability; **not** CLI v6 default / E3-mixed / COL-008  
**Board IDs:** `FMT-V6-SVARINT-PROVISIONAL` (contract), `FMT-V6-SVARINT-MVP` (shipped encode/decode + tests)  
**Depends on:** plan draft [`docs/plan/04_FILE_FORMAT_V6_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/04_FILE_FORMAT_V6_TASKS.md) §5.2; unsigned ULEB128 [`v6-varint-uleb128-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-varint-uleb128-provisional-v0.md)  
**Gate:** IDs frozen after E3-EVENT(C)+E4-v0 (ADR-0006). Residual: CLI v6 default; default-parse non-inflate; E3-mixed; COL-008; full OI-002 vocabulary  

---

## Scope and non-claims

This document is the detailed layout home for **ZigZag+ULEB128 signed algorithm**. Numeric IDs and the core frame described here are **frozen for major=6** by ADR-0006 (see frozen catalog). Filename retains `provisional-v0` for stable links.

It is **not**:

- permission to flip CLI v6 / collection default (still v5 until R4 ADR);
- E3-mixed multi-kind product C fixture claim;
- COL-008 batched Rust writer;
- default-parse always-inflate / CRC default flip;
- complete OI-002 ATTRIBUTE/OPTION key vocabulary;
- a new major without ADR supersession (renumbering requires major bump).

Independent C/Rust implementations must match the frozen IDs and this layout. Golden vectors: [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/).

## ZigZag mapping (provisional)

Map signed `i64` to unsigned `u64` so small-magnitude negatives encode compactly:

| Signed `n` | ZigZag `u` |
|-----------:|-----------:|
| 0 | 0 |
| -1 | 1 |
| 1 | 2 |
| -2 | 3 |
| 2 | 4 |
| … | … |
| `i64::MIN` | `u64::MAX` (maps correctly under the formulas below) |

### Encode (signed → unsigned)

```text
u = ((n as u64) << 1) ^ ((n >> 63) as u64)   // arithmetic right shift on i64
```

Equivalent bit form used by the shipped API:  
`zigzag_encode_i64(n) = ((n << 1) ^ (n >> 63)) as u64`.

### Decode (unsigned → signed)

```text
n = ((u >> 1) as i64) ^ (-((u & 1) as i64))
```

Shipped: `zigzag_decode_i64(u)`.

---

## Wire bytes

| Step | API |
|------|-----|
| Encode signed | `encode_i64(n)` = `encode_u64(zigzag_encode_i64(n))` |
| Decode signed (strict) | `decode_i64(buf, pos)` = ZigZag-decode of `decode_u64(buf, pos)` |

Fail-closed behavior is **inherited** from strict ULEB128:

| Condition | Result |
|-----------|--------|
| Truncated ULEB128 | **Err** truncated |
| Too long (> 10 bytes) / overflow | **Err** too long / overflow |
| Non-canonical overlong (strict) | **Err** non-canonical |

Max on-wire length remains **`MAX_ULEB128_BYTES` = 10**.

### Examples (canonical)

| Signed | ZigZag u | Bytes (hex) |
|-------:|---------:|-------------|
| 0 | 0 | `00` |
| -1 | 1 | `01` |
| 1 | 2 | `02` |
| -2 | 3 | `03` |
| 127 | 254 | `fe 01` |
| -128 | 255 | `ff 01` |

---

## Residual: SLEB128

Plan §5.2 allows **ZigZag+ULEB128 or SLEB128** via ADR. This preflight **selects ZigZag+ULEB128 provisionally**. A competing SLEB128 dialect is **not** shipped here and remains residual until ADR + vectors.

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::varint::{encode_i64, decode_i64, zigzag_encode_i64, zigzag_decode_i64};

let bytes = encode_i64(-2);
let (v, n) = decode_i64(&bytes, 0)?; // strict via ULEB128 path
assert_eq!((v, n), (-2, bytes.len()));
```

- Pure functions; no I/O.
- Tests drive **shipped** `encode_i64` / `decode_i64` (round-trip negatives/zero/positives; truncated; overlong).

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Unsigned ULEB128 preflight | done (`FMT-V6-VARINT-*`) |
| Signed ZigZag+ULEB128 contract + API | **done** (`FMT-V6-SVARINT-*`) |
| C v6 writer (COL-007) | **still deferred** |
| SLEB128 alternative / ADR freeze | residual |
| Event opcodes / dictionaries / payload codecs | residual |
| COL-008 / FFI / multi-OS CI / perf claims | residual |

---

## Open items (honest residual)

1. ADR choice: ZigZag+ULEB128 vs SLEB128 (FMT-003).
2. Immutable golden vectors (FMT-012).
3. Use in event/TLV payloads (not wired to CLI yet).

# Format v6 unsigned LEB128 / ULEB128 (provisional) — v0

**Status:** numeric IDs / core frame **frozen** for major=6 by [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md) + catalog [`v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md); filename retains `provisional-v0` for link stability; **not** CLI v6 default / E3-mixed / COL-008  
**Board IDs:** `FMT-V6-VARINT-PROVISIONAL` (contract), `FMT-V6-VARINT-MVP` (shipped encode/decode + tests)  
**Depends on:** plan draft [`docs/plan/04_FILE_FORMAT_V6_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/04_FILE_FORMAT_V6_TASKS.md) §5.1; fixed-header / chunk preflight under `docs/schemas/v6-*-provisional-v0.md`  
**Gate:** IDs frozen after E3-EVENT(C)+E4-v0 (ADR-0006). Residual: CLI v6 default; default-parse non-inflate; E3-mixed; COL-008; full OI-002 vocabulary  

---

## Scope and non-claims

This document is the detailed layout home for **canonical ULEB128 algorithm + max width**. Numeric IDs and the core frame described here are **frozen for major=6** by ADR-0006 (see frozen catalog). Filename retains `provisional-v0` for stable links.

It is **not**:

- permission to flip CLI v6 / collection default (still v5 until R4 ADR);
- E3-mixed multi-kind product C fixture claim;
- COL-008 batched Rust writer;
- default-parse always-inflate / CRC default flip;
- complete OI-002 ATTRIBUTE/OPTION key vocabulary;
- a new major without ADR supersession (renumbering requires major bump).

Independent C/Rust implementations must match the frozen IDs and this layout. Golden vectors: [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/).

## Encoding (canonical)

ULEB128 encodes an unsigned integer as a sequence of bytes, least-significant 7-bit group first:

| Rule | Detail |
|------|--------|
| Payload bits | Each byte contributes the low **7** bits of the value group |
| Continuation | High bit **0x80** set on all bytes except the **last** |
| Canonical encode | Use the **minimum** number of bytes; never emit a trailing continuation-only zero group for a value that already fits |
| Empty | Not allowed — at least one byte |

### Max width (provisional)

| Bound | Value |
|-------|------:|
| `MAX_ULEB128_BYTES` | **10** |

Rationale: a `u64` needs at most \(\lceil 64/7 \rceil = 10\) bytes. Encoders of `u64` never emit more than 10 bytes. Decoders in **strict** mode reject any integer that requires more than 10 bytes (continuation still set after 10 bytes, or 10th byte with payload that would shift past 64 bits).

### Examples (canonical)

| Value | Bytes (hex) |
|------:|-------------|
| 0 | `00` |
| 1 | `01` |
| 127 | `7f` |
| 128 | `80 01` |
| 255 | `ff 01` |
| 300 | `ac 02` |
| \(2^{64}-1\) | `ff ff ff ff ff ff ff ff ff 01` |

---

## Decode (fail closed)

Given input slice and start offset, `decode_u64` returns `(value, bytes_consumed)` or **Err**.

| Condition | Result |
|-----------|--------|
| No bytes available at start | **Err** truncated |
| Continuation bit set and no following byte | **Err** truncated |
| More than `MAX_ULEB128_BYTES` needed / continuation after 10th byte | **Err** too long |
| **Strict mode:** any **non-canonical overlong** encoding | **Err** non-canonical |

### Non-canonical overlong (strict)

An encoding is **overlong** when it uses more bytes than the canonical encoding of the same value. Equivalent practical checks used by the shipped MVP:

1. **Zero-payload continuation in a non-final sense that pads:** after decoding, if any intermediate byte (all but the last) contributed only zeros *and* the total length exceeds the canonical length for the decoded value — reject.
2. **Shipped strict rule (primary):** if the encoding has length \(L > 1\) and the last byte is `0x00` (continuation cleared with zero payload) while a shorter prefix would represent the same value — reject. More generally: re-encode the decoded value with the canonical encoder; if the re-encoded byte sequence differs from the consumed prefix → **Err** non-canonical.

The MVP implements check (2): decode then compare to `encode_u64(value)` for the consumed length (strict path).

**Non-strict** mode (if exposed): accept overlong encodings that still fit in `u64` without overflow; still fail closed on truncated / too long / shift overflow. The provisional shipped default used by tests and the public `decode_u64` entry point is **strict**.

### Overflow

If shifting the next 7-bit group would exceed 64 bits of significance (value bits past bit 63), return **Err** overflow (fail closed).

---

## Shipped API

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::varint::{encode_u64, decode_u64, MAX_ULEB128_BYTES};

let bytes = encode_u64(300);
let (v, n) = decode_u64(&bytes, 0)?; // strict
assert_eq!((v, n), (300, bytes.len()));
```

- Pure byte-slice / `Vec` APIs (no I/O).
- Tests: round-trip of several magnitudes; truncated → Err; overlong → Err (strict).

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status |
|------|--------|
| Fixed header preflight | done (`FMT-V6-HEADER-*`) |
| Chunk frame preflight | done (`FMT-V6-CHUNK-*`) |
| ULEB128 contract + encode/decode | **done** (`FMT-V6-VARINT-*`) |
| C v6 writer (COL-007) | **still deferred** |
| Signed varints / event opcodes / dictionaries | residual |
| COL-008 / FFI / multi-OS CI / perf claims | residual |

---

## Open items (honest residual)

1. ADR freeze of max width / strict overlong policy (FMT-003).
2. Signed ZigZag + ULEB128 or SLEB128 choice + vectors.
3. Immutable golden byte corpus (FMT-012).
4. Use of varints in TLV / event payload codecs (not wired to CLI yet).

# Format v6 fixed file header (provisional) — v0

**Status:** numeric IDs + 36-byte full header **frozen** for major=6 by [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md); detailed layout text retained; not CLI v6 default  
**Board IDs:** `FMT-V6-HEADER-PROVISIONAL` (contract), `FMT-V6-HEADER-PARSE-MVP` (shipped parse + tests)  
**Depends on:** plan draft [`docs/plan/04_FILE_FORMAT_V6_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/04_FILE_FORMAT_V6_TASKS.md) §3.1  
**Gate:** COL-007 runway preflight only — **before** full C v6 encoder / ADR freeze

---

## Scope and non-claims

This document freezes a **provisional** fixed-header layout so Rust (and later C) can share fail-closed identification of a future v6 file.

It is **not**:

- a permanent wire freeze or ADR-ratified FMT-002;
- permission to mark **COL-007** (C v6 writer) done;
- a complete chunk/TLV/event codec specification;
- a claim that shipped CLI report/dump defaults read v6 profiles.

Exact magic/layout may change under future ADR + immutable vectors. Readers must treat this schema as **provisional**.

---

## Fixed header layout (little-endian)

All multi-byte integers are **little-endian**. Do not write native C/Perl memory layouts.

| Offset | Size | Field | Type | Notes |
|-------:|-----:|-------|------|-------|
| 0 | 8 | Magic | bytes | Unambiguous v6 identification (see below) |
| 8 | 2 | Major | `u16 LE` | Incompatible format generation |
| 10 | 2 | Minor | `u16 LE` | Backward-compatible additions |
| 12 | 4 | Header length | `u32 LE` | Total fixed-header byte length (incl. this field); skip future fixed fields |
| 16 | 8 | Required features | `u64 LE` | Reader must understand all set bits (provisional; may be all-zero) |
| 24 | 8 | Optional features | `u64 LE` | Reader may ignore unknown optional bits |
| 32 | 4 | Header CRC | `u32 LE` | Provisional placeholder; **MVP parse does not validate CRC** (documented residual) |

**Provisional full fixed-header length:** **36** bytes (`HEADER_LEN_FULL`).

**Minimum parse prefix:** first **16** bytes (magic + major + minor + header length) must be present before any further validation.

### Magic (provisional)

```text
ASCII: N Y T P R O F 6
Hex:   4e 59 54 50 52 4f 46 36
```

Constant name in code: `MAGIC` / `nytprof_format_v6::MAGIC`.

This is **not** the v5 text prelude (`NYTProf ...` line). V5 files must fail closed on this magic check.

### Supported major (provisional)

| Major | Meaning |
|------:|---------|
| **6** | Only major accepted by this provisional parse MVP |

Any other major → **Err** (unsupported major), fail closed, no panic.

Minor is recorded but not version-gated in this MVP (any `u16` accepted when major is 6).

### Header length rules (fail closed)

Let `header_len` be the `u32 LE` at offset 12, and `buf` the input slice.

| Condition | Result |
|-----------|--------|
| `buf.len() < 16` | **Err** truncated (cannot read length field) |
| magic ≠ provisional magic | **Err** bad magic |
| major ≠ 6 | **Err** unsupported major |
| `header_len < 16` | **Err** invalid header length (too small) |
| `header_len > MAX_HEADER_LEN` (1 MiB provisional) | **Err** invalid header length (too large) |
| `buf.len() < header_len` | **Err** truncated (declared length not fully present) |
| otherwise | **Ok** with parsed fields |

`MAX_HEADER_LEN` exists only as a fail-closed allocation/skip guard for future TLV readers; this MVP does not parse TLVs.

---

## Fail-closed policy

Aligned with COMPAT-010 spirit for corrupt/unsupported input:

- never panic on crafted byte vectors in the shipped parse path;
- never silently treat bad magic / truncated / unsupported major as a valid v6 header;
- return structured `Err` variants (see crate `nytprof-format-v6`).

CRC mismatch is **not** enforced in `FMT-V6-HEADER-PARSE-MVP` (placeholder field only). Future freeze may require CRC.

---

## Shipped parse entry point

Crate: [`crates/nytprof-format-v6`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6)

```rust
use nytprof_format_v6::{parse_fixed_header, FixedHeader, MAGIC, HEADER_LEN_FULL};

let hdr: FixedHeader = parse_fixed_header(&bytes)?;
```

- Pure byte-slice API (no filesystem I/O in the MVP entry point).
- Tests craft vectors in-process (valid full header, bad magic, truncated, unsupported major).

Evidence: `cargo test -p nytprof-format-v6`.

---

## Relation to COL-007 / full R1

| Item | Status under this schema |
|------|--------------------------|
| Provisional header contract | **done** (`FMT-V6-HEADER-PROVISIONAL`) |
| Shipped fail-closed parse + tests | **done** (`FMT-V6-HEADER-PARSE-MVP`) |
| C v6 writer (COL-007) | **still deferred** |
| Chunk frames (provisional parse) | done separately: `FMT-V6-CHUNK-*` / `v6-chunk-frame-provisional-v0.md` |
| Event codecs / dictionaries | residual (FMT-002..010) |
| COL-008 batched Rust writer | residual |
| CLI default v6 report/dump | not claimed |

---

## Open items (honest residual)

1. ADR ratification of magic / CRC algorithm / feature bit map (FMT-002).
2. Header CRC verification in parse (currently placeholder).
3. Header TLVs after fixed header: frame preflight **done** (`FMT-V6-TLV-*`); full catalog residual.
4. Payload codecs + event stream (rest of §3–§5 in format plan); chunk **frame** preflight is `FMT-V6-CHUNK-*`.
5. Immutable golden byte vectors corpus (FMT-012) after freeze.

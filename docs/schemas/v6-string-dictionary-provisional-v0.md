# Format v6 string-dictionary intern table — provisional v0

**Status:** numeric IDs / core frame **frozen** for major=6 by [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md) + catalog [`v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md); filename retains `provisional-v0` for link stability; **not** CLI v6 default / E3-mixed / COL-008  
**Board IDs:** `FMT-V6-STRING-DICTIONARY-PROVISIONAL` (contract), `FMT-V6-STRING-DICTIONARY-MVP` (shipped table + resolve + always-inflate tests)  
**Depends on:** [`v6-string-blob-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-blob-provisional-v0.md); event-body string-blobs; always-inflate EVENT/mixed consumers  
**Gate:** IDs frozen after E3-EVENT(C)+E4-v0 (ADR-0006). Residual: CLI v6 default; default-parse non-inflate; E3-mixed; COL-008; full OI-002 vocabulary  

---

## Scope and non-claims

This document is the detailed layout home for **FOOTER-local string dictionary table layout (ADR-0002)**. Numeric IDs and the core frame described here are **frozen for major=6** by ADR-0006 (see frozen catalog). Filename retains `provisional-v0` for stable links.

It is **not**:

- permission to flip CLI v6 / collection default (still v5 until R4 ADR);
- E3-mixed multi-kind product C fixture claim;
- COL-008 batched Rust writer;
- default-parse always-inflate / CRC default flip;
- complete OI-002 ATTRIBUTE/OPTION key vocabulary;
- a new major without ADR supersession (renumbering requires major bump).

Independent C/Rust implementations must match the frozen IDs and this layout. Golden vectors: [`fixtures/v6/vectors/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/vectors/).

## Shipped API

Crate: `crates/nytprof-format-v6`

| Item | Role |
|------|------|
| `encode_string_dictionary` / `decode_string_dictionary` | Table codec |
| `StringDictionary::resolve_to_owned` | id → bytes |
| `resolve_event_records` / `owned_event_from_borrowed_resolved` | Event-body resolve |
| `encode_decoded_event_profile_with_string_dict` | EVENT + FOOTER=dict |
| `decode_decoded_event_profile_with_string_dict` | Always-inflate + resolve |
| `encode_decoded_mixed_profile_with_string_dict` | Mixed + FOOTER=dict |
| `decode_decoded_mixed_profile_with_string_dict` | Mixed always-inflate + resolve |

Preflight composition places the dictionary table as **FOOTER** payload (codec NONE). That is a provisional packaging choice, not a permanent kind-catalog freeze.

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`string_dict_*`, `mixed_string_dict_*`).

- Body resolve: MARK / COMMENT / ATTRIBUTE with non-zero ids
- EVENT NONE/ZLIB/ZSTD/LZ4 with FOOTER dictionary
- Mixed EVENT+SOURCE under same codecs
- Unknown id / truncated dictionary fail-closed
- Default stream parse remains non-inflating for compressed EVENT payloads

---

## Open residual

1. Permanent global string-pool / cross-file dictionary ADR + wire freeze.
2. **Permanent** location-delta / run packing ADR (site-delta, TIME_LINE_RUN, TIME_BLOCK_RUN, site-delta+seq compose, and dictionary+packing compose preflights are siblings); full OI-001-03; complete OI-002 inventory.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

# Format v6 string-dictionary intern table — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-STRING-DICTIONARY-PROVISIONAL` (contract), `FMT-V6-STRING-DICTIONARY-MVP` (shipped table + resolve + always-inflate tests)  
**Depends on:** [`v6-string-blob-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-blob-provisional-v0.md); event-body string-blobs; always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** permanent global string-pool ADR / dual-equality / C v6 writer

---

## Scope and non-claims

Defines a provisional **local string dictionary** mapping non-zero `string_id` values to byte payloads for intern resolution of length-prefixed string-blobs.

### Table wire layout

```text
entry_count : ULEB128 u64
entry*      : id ULEB128 || flags u8 || byte_length ULEB128 || bytes
```

| Rule | Detail |
|------|--------|
| `id == 0` | **Reserved** for inline-only blobs — not allowed as a dictionary key |
| Duplicate ids | **Err** |
| Entry payload cap | same as string-blob (`MAX_STRING_BYTES`) |
| Total payload cap | 64 MiB |

### Resolution policy (preflight)

| `string_id` | Result |
|-------------|--------|
| `0` | Use inline blob `bytes` |
| non-zero, present in table | Use dictionary payload (inline may be empty) |
| non-zero, missing | **Err** (`UnknownId`) |

It is **not**:

- a permanent global / cross-file string pool ADR freeze;
- permanent location-delta packing ADR for TIME_* / SUB_ENTRY sites (site-delta preflight is a sibling `FMT-V6-EVENT-BODY-SITE-DELTA-*`);
- full OI-001-03 sequence-number freeze; complete OI-002 key inventory;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

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

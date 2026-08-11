# Format v6 composed string-dictionary + site-delta/seq packing — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-STRING-DICT-SITE-DELTA-SEQ-COMPOSE-PROVISIONAL` (contract), `FMT-V6-STRING-DICT-SITE-DELTA-SEQ-COMPOSE-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** string-dictionary [`v6-string-dictionary-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-dictionary-provisional-v0.md); site-delta+seq compose [`v6-event-body-site-delta-seq-compose-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-site-delta-seq-compose-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** permanent global string-pool ADR / permanent packing ADR / dual-equality / C v6 writer

---

## Scope and non-claims

Allows a **composed** provisional profile path:

1. **EVENT body** packed with site-delta + monotonic logical sequence numbers (`encode_event_body_with_site_deltas_and_seq`).
2. **String-bearing records** using non-zero `string_id` with empty inline payloads.
3. **FOOTER** carrying the provisional string-dictionary table (codec **NONE** packaging).
4. **Always-inflate** consumers recover absolute sites, per-event sequences, and dictionary-resolved string bytes.

### Shipped encode helpers

| Item | Role |
|------|------|
| `encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq` | EVENT + FOOTER dictionary compose (`max_events_per_chunk`; 0 = single-chunk) |
| `encode_decoded_mixed_profile_with_string_dict_and_site_deltas_and_seq` | Multi-kind + FOOTER dictionary compose (same `max_events_per_chunk` rule) |

Multi-chunk packing continuity **with** FOOTER dict is a sibling preflight: [`v6-string-dict-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-dict-multi-chunk-site-delta-seq-packing-provisional-v0.md).

### Decode (existing paths)

| Item | Role |
|------|------|
| `decode_decoded_event_profile_with_string_dict` | Always-inflate + `decode_event_body_full` + resolve |
| `decode_decoded_mixed_profile_with_string_dict` | Same for multi-kind EVENT join |

It is **not**:

- a permanent global string-pool / cross-file dictionary ADR;
- a permanent packing / flag-bit ADR;
- full OI-001-03 / COL-003 dual-output sequence-number policy freeze;
- complete OI-002 inventory freeze;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Unknown non-zero `string_id` | **Err** (`StringDictError::UnknownId`) |
| Truncated mid-seq / mid-delta | **Err** (varint / truncated) |
| Missing FOOTER dictionary | **Err** (`MissingStringDictionaryFooter`) |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`string_dict_and_site_delta_seq_compose_*`, `mixed_string_dict_and_site_delta_seq_compose_*`).

- Resolved dictionary string bytes **and** absolute TIME_LINE/SUB_ENTRY sites **and** monotonic seq
- EVENT NONE/ZLIB/ZSTD/LZ4; mixed + SOURCE co-kind; FOOTER dictionary codec NONE
- Default stream parse / `parse_chunk_frame` non-inflating for compressed EVENT payloads
- Prior pure dictionary, pure packing, and site-delta+seq compose tests remain green

---

## Open residual

1. Permanent global string-pool / packing ADR freezes (multi-chunk packing continuity is a sibling — see [`v6-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-site-delta-seq-packing-provisional-v0.md)).
2. Full OI-001-03 freeze; complete OI-002 freeze.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

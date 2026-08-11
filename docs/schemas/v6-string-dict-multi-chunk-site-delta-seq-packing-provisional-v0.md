# Format v6 FOOTER string-dictionary + multi-chunk site-delta/seq packing continuity — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-STRING-DICT-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-PROVISIONAL` (contract), `FMT-V6-STRING-DICT-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** string-dict + packing compose [`v6-string-dict-site-delta-seq-compose-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-dict-site-delta-seq-compose-provisional-v0.md); multi-chunk packing continuity [`v6-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-site-delta-seq-packing-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** permanent packing/string-pool ADRs / dual-equality / C v6 writer

---

## Scope and non-claims

Composes two prior preflights into one encode/decode path:

1. **Multi-chunk record-aligned** EVENT streams with site-delta + `FLAG_HAS_SEQ` packing whose **site bases and sequence numbers continue across chunk boundaries**.
2. **FOOTER** provisional string-dictionary (codec **NONE** packaging) so non-zero `string_id` on string-bearing records (e.g. MARK, COMMENT) resolve after always-inflate join.
3. Always-inflate join recovers the same ordered absolute sites, monotonic sequences, and dictionary-resolved string bytes as a **single-chunk** dict+packing compose of the same specs.

### Shipped encode helpers

| Item | Role |
|------|------|
| `encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq` | EVENT + FOOTER dict; `max_events_per_chunk` (0 = single; ≥1 multi-chunk continuity) |
| `encode_decoded_mixed_profile_with_string_dict_and_site_deltas_and_seq` | Multi-kind + FOOTER dict; same `max_events_per_chunk` rule on EVENT |
| `encode_decoded_event_profile_with_site_deltas_and_seq` | Underlying EVENT path (`max` + optional `dict_entries`) |

### Continuity + dictionary rule

```text
state = PackingEncodeState::new()
for part in partition_event_records(events, max_per_chunk):
    plain_i = encode_event_body_with_site_deltas_and_seq_continuing(part, &mut state)
FOOTER = encode_string_dictionary(dict_entries)  # codec NONE
join(plain_i) == encode_event_body_with_site_deltas_and_seq(events)
resolve(decode_event_body_full(join), dict) == single-chunk dict+packing compose
```

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

Evidence: `cargo test -p nytprof-format-v6` (`string_dict_multi_chunk_*`, `mixed_string_dict_multi_chunk_*`).

- Multi-chunk packing + FOOTER dict equals single-chunk dict+packing recovered records/seq/strings under NONE/ZLIB/ZSTD/LZ4
- Mixed multi-chunk EVENT + SOURCE co-kind + FOOTER dict
- Default stream parse / `parse_chunk_frame` non-inflating for compressed EVENT payloads
- Prior single-chunk dict+packing, multi-chunk packing without dict, absolute multi-chunk, TIME_*_RUN, known-key tests remain green

---

## Open residual

Triple compose (FOOTER dict + multi-chunk packing + TIME_*_RUN) is a sibling: [`v6-string-dict-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-dict-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md).

1. Permanent packing ADR / permanent string-pool ADR.
2. Full OI-001-03 freeze; complete OI-002 freeze.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

# Format v6 FOOTER string-dict + multi-chunk packing + TIME_*_RUN continuity — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-STRING-DICT-MULTI-CHUNK-TIME-RUN-SITE-DELTA-SEQ-PACKING-PROVISIONAL` (contract), `FMT-V6-STRING-DICT-MULTI-CHUNK-TIME-RUN-SITE-DELTA-SEQ-PACKING-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** string-dict multi-chunk packing [`v6-string-dict-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-dict-multi-chunk-site-delta-seq-packing-provisional-v0.md); multi-chunk packing + TIME_*_RUN [`v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** permanent packing/string-pool ADRs / dual-equality / C v6 writer

---

## Scope and non-claims

Composes three prior preflights into one encode/decode path:

1. **Multi-chunk record-aligned** EVENT packing with continuous site bases and sequence numbers across chunks (`PackingEncodeState`).
2. **TIME_LINE_RUN / TIME_BLOCK_RUN** coexisting with site-delta TIME_LINE / TIME_BLOCK / SUB_ENTRY; runs advance the site cursor so post-run site-delta reconstructs correctly **across a later partition**.
3. **FOOTER** provisional string-dictionary (codec **NONE**) so non-zero `string_id` on MARK/COMMENT (etc.) resolve after always-inflate join.

Always-inflate join recovers the same ordered absolute sites, monotonic sequences, and dictionary-resolved string bytes as a **single-chunk** dict+packing compose of the same specs.

Partitioning is **record-aligned** (whole run stays in one chunk; mid-run body span is not claimed).

### Shipped encode helpers

| Item | Role |
|------|------|
| `encode_decoded_event_profile_with_string_dict_and_site_deltas_and_seq` | EVENT + FOOTER dict; `max_events_per_chunk` (0 = single; ≥1 multi-chunk continuity) |
| `encode_decoded_mixed_profile_with_string_dict_and_site_deltas_and_seq` | Multi-kind + FOOTER dict; same `max_events_per_chunk` on EVENT |
| `PackingEncodeState` / continuing encode | Site/seq continuity; TIME_*_RUN advances site + seq by N |

### Continuity + dictionary rule

```text
state = PackingEncodeState::new()
for part in partition_event_records(events, max_per_chunk):
    plain_i = encode_event_body_with_site_deltas_and_seq_continuing(part, &mut state)
FOOTER = encode_string_dictionary(dict_entries)  # codec NONE
join(plain_i) == encode_event_body_with_site_deltas_and_seq(events)
resolve(decode_event_body_full(join), dict) == single-chunk dict+packing compose
```

It is **not**:

- a permanent global string-pool / packing ADR;
- full OI-001-03 / complete OI-002 freeze;
- mid-run body span across chunks;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Unknown non-zero `string_id` | **Err** (`StringDictError::UnknownId`) |
| Empty / oversize TIME_*_RUN | **Err** |
| Truncated mid-run / mid-seq / mid-delta | **Err** |
| Missing FOOTER dictionary | **Err** (`MissingStringDictionaryFooter`) |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`string_dict_multi_chunk_packing_with_time_runs_*`, `mixed_string_dict_multi_chunk_packing_with_time_runs_*`).

- Multi-chunk packing + TIME_*_RUN + FOOTER dict equals single-chunk recovered records/seq/strings under NONE/ZLIB/ZSTD/LZ4
- Post-run site-delta across chunk boundary lands on correct absolute site
- Mixed multi-chunk EVENT + SOURCE co-kind + FOOTER dict
- Default stream parse / `parse_chunk_frame` non-inflating for compressed EVENT payloads
- Prior dict multi-chunk, multi-chunk+run, single-chunk dict+packing tests remain green

---

## Open residual

Mid-stream packing + FOOTER string-dict compose is a sibling: [`v6-mid-stream-codec-switch-string-dict-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-mid-stream-codec-switch-string-dict-site-delta-seq-packing-provisional-v0.md).

Auto-VERSION + FOOTER dict + multi-chunk packing compose is a sibling: [`v6-auto-version-string-dict-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-auto-version-string-dict-multi-chunk-site-delta-seq-packing-provisional-v0.md).

1. Permanent packing ADR / permanent string-pool ADR.
2. Full OI-001-03 freeze; complete OI-002 freeze.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

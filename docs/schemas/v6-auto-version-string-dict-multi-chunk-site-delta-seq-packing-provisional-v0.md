# Format v6 auto-VERSION + FOOTER string-dict + multi-chunk packing continuity — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-AUTO-VERSION-STRING-DICT-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-PROVISIONAL` (contract), `FMT-V6-AUTO-VERSION-STRING-DICT-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** auto-VERSION multi-chunk packing [`v6-auto-version-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-auto-version-multi-chunk-site-delta-seq-packing-provisional-v0.md); string-dict multi-chunk packing (+run) [`v6-string-dict-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-string-dict-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** dual-equality freeze / permanent packing/string-pool ADRs / C v6 writer

---

## Scope and non-claims

Composes:

1. **Auto-emit / validate VERSION** matching fixed-header major/minor.
2. **FOOTER** provisional string-dictionary (codec **NONE**) resolving non-zero `string_id` (MARK/COMMENT, …).
3. **Multi-chunk record-aligned** site-delta + `FLAG_HAS_SEQ` packing with continuous site/seq bases across chunks (preferably with **TIME_*_RUN** so post-run site-delta across a later partition is correct).

Always-inflate auto-version + string-dict decode recovers the same ordered absolute sites, monotonic sequences, and resolved string bytes as a single-chunk compose of the same logical workload.

### Shipped helpers

| Item | Role |
|------|------|
| `encode_decoded_event_profile_auto_version_with_site_deltas_and_seq` | VERSION inject/validate + packing multi-chunk + optional `dict_entries` FOOTER |
| `decode_decoded_event_profile_auto_version_with_string_dict` | Always-inflate resolve + VERSION align |
| `encode_decoded_mixed_profile_auto_version_with_string_dict_and_site_deltas_and_seq` | Mixed compose |
| `decode_decoded_mixed_profile_auto_version_with_string_dict` | Mixed decode compose |

Partitioning is **record-aligned**. Absolute auto-version, auto-version packing without dict, and dict multi-chunk without auto-version remain available.

It is **not**:

- full dual-equality product freeze;
- permanent packing ADR / permanent string-pool ADR;
- full OI-001-03 / complete OI-002 freeze;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Body VERSION ≠ header major/minor | **Err** (`VersionHeaderMismatch`) |
| Unknown non-zero `string_id` | **Err** (`UnknownId`) |
| Empty / oversize TIME_*_RUN / truncated mid-field | **Err** |
| Missing FOOTER dictionary | **Err** (`MissingStringDictionaryFooter`) |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`auto_version_dict_multi_chunk_packing_*`, `mixed_auto_version_dict_multi_chunk_packing_*`).

- Auto-VERSION + FOOTER dict + multi-chunk packing (+ TIME_*_RUN) equals single-chunk recovered records/seq/strings under NONE/ZLIB/ZSTD/LZ4
- Post-run site-delta across chunk boundary lands on correct absolute site
- Mixed multi-chunk EVENT + SOURCE co-kind + FOOTER dict
- Default stream parse / `parse_chunk_frame` non-inflating for compressed EVENT payloads
- Prior auto-VERSION packing (no dict), string-dict multi-chunk (+run), absolute auto-version tests remain green

---

## Open residual

1. Full dual-equality policy freeze; permanent packing ADR / permanent string-pool ADR.
2. Full OI-001-03 freeze; complete OI-002 freeze.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; CLI v6 default.

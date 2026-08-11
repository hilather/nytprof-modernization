# Format v6 mid-stream codec-switch packing + FOOTER string-dict continuity — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-MID-STREAM-CODEC-SWITCH-STRING-DICT-SITE-DELTA-SEQ-PACKING-PROVISIONAL` (contract), `FMT-V6-MID-STREAM-CODEC-SWITCH-STRING-DICT-SITE-DELTA-SEQ-PACKING-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** mid-stream packing continuity [`v6-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md); FOOTER string-dictionary; always-inflate EVENT/mixed string-dict consumers  
**Gate:** COL-007 runway preflight only — **before** permanent packing/string-pool ADRs / dual-equality / C v6 writer

---

## Scope and non-claims

Composes:

1. **Mid-stream START_DEFLATE codec-switch** with shared [`PackingEncodeState`] across pre/post (site bases and sequence numbers continue across the switch).
2. **FOOTER** provisional string-dictionary (codec **NONE**) resolving non-zero `string_id` (MARK/COMMENT, …).
3. Preferably **TIME_*_RUN** in pre so a site-delta in post reconstructs correctly.

Always-inflate string-dict decode recovers the same ordered absolute sites, monotonic sequences, and resolved string bytes as continuous packing of `pre || post` plus FOOTER dict.

### Shipped helpers

| Item | Role |
|------|------|
| `encode_decoded_event_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq` | EVENT packing mid-stream + FOOTER dict |
| `encode_decoded_mixed_mid_stream_codec_switch_with_string_dict_and_site_deltas_and_seq` | Mixed (+ SOURCE) + FOOTER dict |
| `decode_decoded_event_profile_with_string_dict` / mixed sibling | Always-inflate resolve |

Absolute mid-stream and mid-stream packing without dict remain available.

It is **not**:

- a permanent packing ADR / permanent string-pool ADR;
- full dual-equality / OI-001-03 / complete OI-002 freeze;
- mid-run body span across the switch;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Missing `START_DEFLATE` in pre | **Err** |
| `pre_codec == post_codec` | **Err** |
| Unknown non-zero `string_id` | **Err** (`UnknownId`) |
| Missing FOOTER dictionary | **Err** (`MissingStringDictionaryFooter`) |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`mid_stream_codec_switch_dict_packing_*`, `mixed_mid_stream_codec_switch_dict_packing_*`).

- Mid-stream packing + FOOTER dict equals continuous packing of pre||post (+ resolved strings) under post codecs ZLIB/ZSTD/LZ4
- Post-run site-delta in post region lands on correct absolute site
- Mixed EVENT + SOURCE co-kind + FOOTER dict
- Default stream parse / `parse_chunk_frame` non-inflating for compressed post EVENT payloads
- Prior absolute mid-stream, mid-stream packing without dict, multi-chunk dict packing remain green

---

## Open residual

1. Permanent packing ADR / permanent string-pool ADR.
2. Full OI-001-03 freeze; complete OI-002 freeze; dual-equality freeze.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; CLI v6 default.

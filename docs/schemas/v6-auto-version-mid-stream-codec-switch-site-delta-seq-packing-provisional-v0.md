# Format v6 auto-VERSION + mid-stream packing continuity — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-AUTO-VERSION-MID-STREAM-CODEC-SWITCH-SITE-DELTA-SEQ-PACKING-PROVISIONAL` (contract), `FMT-V6-AUTO-VERSION-MID-STREAM-CODEC-SWITCH-SITE-DELTA-SEQ-PACKING-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** auto-VERSION preflight; mid-stream packing continuity [`v6-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md); always-inflate EVENT/mixed auto-version consumers  
**Gate:** COL-007 runway preflight only — **before** dual-equality freeze / permanent packing ADR / C v6 writer

---

## Scope and non-claims

Composes:

1. **Auto-emit / validate VERSION** matching fixed-header major/minor (inject at start of **pre** when pre||post omit VERSION; fail-closed on mismatch).
2. **Mid-stream START_DEFLATE codec-switch** with shared [`PackingEncodeState`] across pre/post (site bases and sequence numbers continue across the switch).
3. Preferably **TIME_*_RUN** in pre so a site-delta in post reconstructs correctly.

Always-inflate auto-version decode recovers the same ordered absolute sites and monotonic sequences as continuous packing of `VERSION||pre||post` (when VERSION was injected into pre).

### Shipped helpers

| Item | Role |
|------|------|
| `encode_decoded_event_mid_stream_codec_switch_auto_version_with_site_deltas_and_seq` | VERSION inject + packing mid-stream |
| `encode_decoded_mixed_mid_stream_codec_switch_auto_version_with_site_deltas_and_seq` | Mixed sibling |
| `decode_decoded_event_profile_auto_version` / mixed sibling | Always-inflate VERSION align |

FOOTER string-dict mid-stream packing is a sibling compose (not required here). Absolute mid-stream and mid-stream packing without auto-VERSION remain available.

It is **not**:

- full dual-equality product freeze;
- a permanent packing ADR / permanent string-pool ADR;
- full OI-001-03 / complete OI-002 freeze;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Body VERSION ≠ header major/minor | **Err** (`VersionHeaderMismatch`) |
| Missing `START_DEFLATE` in pre | **Err** |
| `pre_codec == post_codec` | **Err** |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`mid_stream_codec_switch_auto_version_packing_*`, `mixed_mid_stream_codec_switch_auto_version_packing_*`).

- Auto-VERSION + mid-stream packing equals continuous packing under post codecs ZLIB/ZSTD/LZ4
- Post-run site-delta in post region lands on correct absolute site
- Mixed EVENT + SOURCE co-kind
- Default stream parse / `parse_chunk_frame` non-inflating for compressed post EVENT payloads
- Prior absolute mid-stream, mid-stream packing (±dict), multi-chunk auto-VERSION packing remain green

---

## Open residual

1. Full dual-equality policy freeze; permanent packing ADR / permanent string-pool ADR.
2. Full OI-001-03 freeze; complete OI-002 freeze.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; CLI v6 default.

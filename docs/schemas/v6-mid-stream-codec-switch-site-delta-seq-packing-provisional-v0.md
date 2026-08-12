# Format v6 mid-stream codec-switch + site-delta/seq packing continuity — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-MID-STREAM-CODEC-SWITCH-SITE-DELTA-SEQ-PACKING-PROVISIONAL` (contract), `FMT-V6-MID-STREAM-CODEC-SWITCH-SITE-DELTA-SEQ-PACKING-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** mid-stream START_DEFLATE codec-switch preflight; multi-chunk packing continuity [`v6-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-site-delta-seq-packing-provisional-v0.md); multi-chunk packing + TIME_*_RUN [`v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** permanent packing ADR / dual-equality / C v6 writer

---

## Scope and non-claims

Composes:

1. **Mid-stream START_DEFLATE codec-switch** (chunk-framed preflight): EVENT chunk under `pre_codec` (body includes `START_DEFLATE`) then EVENT chunk under `post_codec` ≠ pre (typically NONE → ZLIB/ZSTD/LZ4).
2. **Site-delta + `FLAG_HAS_SEQ` packing** with one shared [`PackingEncodeState`] across pre and post regions so site bases and sequence numbers **continue across the codec switch** (not reset at post start).
3. Preferably **TIME_LINE_RUN / TIME_BLOCK_RUN** in pre so a site-delta event in post reconstructs correctly relative to the run site.

Always-inflate join recovers the same ordered absolute sites and monotonic sequences as a single continuous packing encode of `pre || post`.

### Shipped helpers

| Item | Role |
|------|------|
| `encode_decoded_event_mid_stream_codec_switch_with_site_deltas_and_seq` | EVENT packing mid-stream switch |
| `encode_decoded_mixed_mid_stream_codec_switch_with_site_deltas_and_seq` | Mixed (+ optional SOURCE) |
| Absolute mid-stream helpers | Unchanged (absolute bodies, no packing continuity) |

### Continuity rule

```text
state = PackingEncodeState::new()
pre_plain  = encode_event_body_with_site_deltas_and_seq_continuing(pre, &mut state)
post_plain = encode_event_body_with_site_deltas_and_seq_continuing(post, &mut state)
join(pre_plain, post_plain) == encode_event_body_with_site_deltas_and_seq(pre || post)
```

It is **not**:

- a permanent packing ADR / flag-bit freeze;
- full dual-equality / OI-001-03 / complete OI-002 freeze;
- mid-run body span across the switch record boundary (record-aligned regions only);
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Missing `START_DEFLATE` in pre | **Err** (`MissingStartDeflateMarker` / mixed sibling) |
| `pre_codec == post_codec` | **Err** (`MidStreamCodecsMustDiffer` / mixed sibling) |
| Empty pre or post | **Err** |
| Truncated mid-field / mid-run | **Err** |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`mid_stream_codec_switch_packing_*`, `mixed_mid_stream_codec_switch_packing_*`).

- Mid-stream packing equals continuous packing of pre||post under post codecs ZLIB/ZSTD/LZ4
- Post-run site-delta in post region lands on correct absolute site
- Mixed EVENT + SOURCE co-kind
- Default stream parse / `parse_chunk_frame` non-inflating for compressed post EVENT payloads
- Prior absolute mid-stream switch and multi-chunk packing tests remain green

---

## Open residual

Permanent packing **intent** is proposed in [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md) (status proposed — not wire freeze; dual-equality readiness: [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md)).

Auto-VERSION + mid-stream packing compose is a sibling: [`v6-auto-version-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-auto-version-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md).

Mid-stream packing + FOOTER string-dict compose is a sibling: [`v6-mid-stream-codec-switch-string-dict-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-mid-stream-codec-switch-string-dict-site-delta-seq-packing-provisional-v0.md).

1. Permanent packing ADR / permanent string-pool ADR.
2. Full OI-001-03 freeze; complete OI-002 freeze; dual-equality freeze.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; CLI v6 default.

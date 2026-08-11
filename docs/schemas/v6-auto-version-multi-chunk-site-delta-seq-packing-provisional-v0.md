# Format v6 auto-emit VERSION + multi-chunk site-delta/seq packing continuity — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-AUTO-VERSION-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-PROVISIONAL` (contract), `FMT-V6-AUTO-VERSION-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** auto-emit VERSION preflight; multi-chunk packing continuity [`v6-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-site-delta-seq-packing-provisional-v0.md); multi-chunk packing + TIME_*_RUN [`v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-multi-chunk-time-run-site-delta-seq-packing-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** dual-equality freeze / permanent packing ADR / C v6 writer

---

## Scope and non-claims

Composes:

1. **Auto-emit / validate VERSION** matching fixed-header major/minor (prepend when body omits; fail-closed on mismatch).
2. **Multi-chunk record-aligned** site-delta + `FLAG_HAS_SEQ` packing with continuous site bases and sequence numbers across chunks (`PackingEncodeState`).
3. Preferably **TIME_LINE_RUN / TIME_BLOCK_RUN** so post-run site-delta across a later partition remains correct.

Always-inflate auto-version decode recovers the same ordered absolute sites and monotonic sequences as a single-chunk packing compose of the same logical workload (with VERSION present). When body omits VERSION, decode may inject a synthetic VERSION and keep sequence list aligned.

### Shipped encode helpers

| Item | Role |
|------|------|
| `encode_decoded_event_profile_auto_version_with_site_deltas_and_seq` | VERSION inject/validate + packing multi-chunk (`max_events_per_chunk`) |
| `encode_decoded_mixed_profile_auto_version_with_site_deltas_and_seq` | Same for multi-kind (EVENT packing + SOURCE/INDEX/SUMMARY absolute) |
| `decode_decoded_event_profile_auto_version` / mixed sibling | Always-inflate + VERSION align |

Partitioning is **record-aligned** (whole run in one chunk). Absolute auto-version (non-packing) path remains available separately.

It is **not**:

- full dual-equality product freeze (header vs body VERSION policy beyond auto-emit preflight);
- a permanent packing ADR / flag-bit freeze;
- full OI-001-03 / complete OI-002 freeze;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Body VERSION ≠ header major/minor | **Err** (`VersionHeaderMismatch`) |
| Empty / oversize TIME_*_RUN | **Err** |
| Truncated mid-run / mid-seq / mid-delta | **Err** |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`auto_version_multi_chunk_packing_*`, `mixed_auto_version_multi_chunk_packing_*`).

- Auto-VERSION + multi-chunk packing (+ TIME_*_RUN) equals single-chunk recovered records/seq under NONE/ZLIB/ZSTD/LZ4
- Post-run site-delta across chunk boundary lands on correct absolute site
- Mixed multi-chunk EVENT + SOURCE co-kind
- Default stream parse / `parse_chunk_frame` non-inflating for compressed EVENT payloads
- Prior absolute auto-version, multi-chunk packing, multi-chunk+run, dict multi-chunk tests remain green

---

## Open residual

Auto-VERSION + mid-stream packing compose is a sibling: [`v6-auto-version-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-auto-version-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md).

Auto-VERSION + FOOTER dict + multi-chunk packing compose is a sibling: [`v6-auto-version-string-dict-multi-chunk-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-auto-version-string-dict-multi-chunk-site-delta-seq-packing-provisional-v0.md).

1. Full dual-equality policy freeze; permanent packing ADR / permanent string-pool ADR.
2. Full OI-001-03 freeze; complete OI-002 freeze.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; CLI v6 default.

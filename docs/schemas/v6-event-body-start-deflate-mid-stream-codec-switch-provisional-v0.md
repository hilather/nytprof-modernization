# Format v6 START_DEFLATE mid-stream payload codec-switch — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-START-DEFLATE-MID-STREAM-CODEC-SWITCH-PROVISIONAL` (contract), `FMT-V6-START-DEFLATE-MID-STREAM-CODEC-SWITCH-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-start-deflate-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-start-deflate-provisional-v0.md); always-inflate EVENT/mixed consumers; payload codecs NONE/ZLIB/ZSTD/LZ4  
**Gate:** COL-007 runway preflight only — **before** full OI-001-03 freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Models a **chunk-framed** mid-stream payload codec change **after** a `START_DEFLATE` marker in the pre-switch EVENT body:

```text
[file prefix]
[EVENT chunk, pre_codec (typically NONE)]  body includes … START_DEFLATE
[EVENT chunk, post_codec ∈ {ZLIB,ZSTD,LZ4}, post_codec ≠ pre_codec]  workload records
[optional other kinds / FOOTER]
```

Always-inflate consumers join EVENT plains in file order and recover a single ordered event-body record list.

This is **not** v5 mid-payload byte-stream deflate of a single continuous buffer. Provisional v6 remains chunk-framed; the switch is a **chunk codec change after the marker chunk**.

It is **not**:

- full **OI-001-03** dual-output sequence-number freeze (COL-003 / ADR + golden vectors);
- auto-emit VERSION freeze beyond shipped `FMT-V6-AUTO-EMIT-VERSION-*` preflight;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- complete ATTRIBUTE/OPTION key vocabulary freeze; COMPAT-002 comment normalize;
- wire freeze / dual-equality / CLI v6 default / FMT-012 golden corpus;
- permission to mark **COL-007** / **COL-008** done.

---

## Shipped API

Crate: `crates/nytprof-format-v6`

| Function | Role |
|----------|------|
| `encode_decoded_event_mid_stream_codec_switch_profile` | EVENT-only pre/post codec wire |
| `decode_decoded_event_profile` | Always-inflate join; **allows** per-EVENT-chunk codec change |
| `encode_decoded_mixed_mid_stream_codec_switch_profile` | EVENT pre/post + optional SOURCE co-kind |
| `decode_decoded_mixed_profile` | Always-inflate; EVENT may mid-stream switch; other kinds still uniform codec |

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Pre region missing `START_DEFLATE` | **Err** (encode) |
| `pre_codec == post_codec` | **Err** (encode) |
| Corrupt post-switch compressed payload | **Err** (decode) |
| Truncated / unknown opcodes in joined body | **Err** (unchanged MVP) |
| Never panic | Required |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`mid_stream_codec_switch_*`, `mixed_mid_stream_codec_switch_*`).

- NONE → ZLIB / ZSTD / LZ4 post-switch order+field round-trip
- Mixed path with co-present SOURCE
- Default stream / `parse_chunk_frame` remains non-inflating (post-switch payload ≠ plain body)

---

## Open residual

Mid-stream codec-switch + packing continuity is a sibling: [`v6-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-mid-stream-codec-switch-site-delta-seq-packing-provisional-v0.md).

1. Full dual-output **sequence-number** freeze (OI-001-03 / COL-003).
2. Broader dual-equality policy for header vs body VERSION beyond auto-emit preflight.
3. v5-style mid-payload stream deflate semantics (if ever required) vs chunk-framed switch.
4. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; key vocabularies.

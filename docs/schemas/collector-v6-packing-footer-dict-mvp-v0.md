# Collector v6 packing + FOOTER string-dict + mid-stream region — MVP v0 (COL-007 staged / PR-B08)

**Status:** provisional scaffolding — **not** a v6 wire freeze; **not** board COL-007 done (E3-C = PR-B09)  
**Task:** COL-007 packing / FOOTER dict / mid-stream codec region (PR-B08)  
**Depends on:** [collector-v6-codecs-multi-chunk-crc-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-codecs-multi-chunk-crc-mvp-v0.md) (PR-B07)  
**ADRs:** [0001 packing](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md) (accepted intent); [0002 FOOTER-local dict](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md) (accepted intent)  
**Layout:** [ADR-0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md) B0-A `collector/`  
**Sources:** [`collector/include/nytp_sink_v6.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytp_sink_v6.h), [`collector/src/nytp_sink_v6.c`](https://github.com/hilather/nytprof-modernization/blob/main/collector/src/nytp_sink_v6.c)  
**IDs:** [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h)  
**Independent reader:** `nytprof-format-v6` always-inflate `decode_decoded_event_profile` / `decode_decoded_event_profile_with_string_dict`

---

## Intent

Extend the C v6 writer with ADR-aligned packing and FOOTER-local dictionary emit:

| Capability | Rule |
|------------|------|
| Site-delta packing | TIME_LINE / TIME_BLOCK / SUB_ENTRY with `FLAG_SITE_DELTA` + ZigZag deltas vs continuous `SiteCursor` |
| Sequence packing | `FLAG_HAS_SEQ` + ULEB packing seq (compose with site-delta on site-bearing records) |
| Continuity | Multi-chunk partitions and mid-stream codec regions **share** packing state (do not reset site/seq) |
| TIME_LINE_RUN | Optional packed same-site run API; advances SiteCursor; seq base..base+N-1 |
| Mid-stream codec region | `begin_codec_region(next)`: emit empty START_DEFLATE → seal current EVENT region → switch codec |
| FOOTER string dict | When enabled: intern non-empty strings; EVENT blobs use non-zero `string_id` + empty inline; FOOTER table last (codec NONE) |

Default `create` remains absolute / no packing / no FOOTER dict (B06/B07 behavior).

## Surface (v0)

| Symbol | Role |
|--------|------|
| `nytp_v6_sink_options` | codec, max_records_per_chunk, `enable_packing`, `enable_string_dict` |
| `nytp_v6_sink_create_opts` | full create with options |
| `nytp_v6_sink_begin_codec_region` | mid-stream START_DEFLATE + region seal + codec switch; fail-closed on empty open body / non-OPEN|ACTIVE lifecycle; rolls back marker if seal fails |
| `nytp_v6_sink_emit_time_line_run` | TIME_LINE_RUN when packing enabled; lifecycle OPEN|ACTIVE; COL-003 seq advances by `n_ticks` |
| `nytp_v6_sink_packing_enabled` / `string_dict_enabled` / `has_footer_dict` | getters |

### Packing wire (compose, matches format-v6 preflight)

```text
TIME_LINE (packing):
  opcode ULEB || flags(SITE_DELTA|HAS_SEQ) || seq ULEB || Δfid ZigZag || Δline ZigZag || ticks ULEB

TIME_LINE_RUN:
  opcode ULEB || flags(HAS_SEQ) || seq_base ULEB || fid ULEB || line ULEB || N ULEB || ticks[N]

Other opcodes (packing):
  opcode ULEB || flags(HAS_SEQ) || seq ULEB || absolute typed body
```

Absolute path (packing off): flags byte `0` (B06/B07 unchanged).

### FOOTER dictionary table (ADR-0002)

```text
entry_count : ULEB128
entry*      : id ULEB128 || flags u8 || byte_length ULEB128 || bytes
```

`id == 0` reserved for inline-only blobs. Duplicate ids fail-closed at encode. Emitted only when `enable_string_dict` (including empty table).

### Mid-stream region

1. Emit empty `START_DEFLATE` into open body (marker only; not a body codec field).  
2. Seal open body as EVENT chunk(s) under **current** codec (region seal; packing continues).  
3. Subsequent chunk payloads use `next_codec` (must differ).  
4. Final `close` seals remaining post region + optional FOOTER.

## Acceptance for this MVP

| Check | Evidence |
|-------|----------|
| Site-delta + seq reconstruct continuous sites | `collector/t/test_v6_packing_footer.c` |
| Multi-chunk packing continuity (joined plains) | same |
| TIME_LINE_RUN + following site-delta | same |
| Mid-stream NONE→ZLIB with START_DEFLATE in pre | same |
| FOOTER dict intern + empty-inline EVENT blobs | same |
| Packing + dict multi-chunk | same |
| Prior absolute + codec suites green | `test_v6_abs_wire`, `test_v6_codec_chunk_crc` |
| Rust always-inflate dual-path | `cargo run -p nytprof-format-v6 --example decode_c_b08 -- <artifact> [--dict]` |

## Residuals / non-claims

| Residual | Owner |
|----------|-------|
| Board **COL-007 done** + E3-C fixtures (`e3_c_*`) | PR-B09 |
| Wire freeze / FMT-002..010 | after E3/E4 |
| TIME_BLOCK_RUN emit API | optional follow-on (TIME_LINE_RUN covered) |
| Global / cross-file string pool | separate ADR (not ADR-0002) |
| CLI v6 default / default-parse always-inflate | residual |
| Live Perl/XS hooks | later COL |
| BENCH writer cost | BENCH-004/006 |

## Tests

- `collector/t/test_v6_packing_footer.c` — packing continuity, run, mid-stream, FOOTER dict  
- `make -C collector test` includes packing suite after B07 codec suite

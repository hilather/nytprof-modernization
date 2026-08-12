# Collector v6 codecs + multi-chunk + CRC — MVP v0 (COL-007 staged / PR-B07)

**Status:** provisional scaffolding — **not** a v6 wire freeze; **not** board COL-007 done (E3-C = PR-B09)  
**Task:** COL-007 codecs / multi-chunk / CRC stage (PR-B07)  
**Depends on:** [collector-v6-absolute-wire-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-absolute-wire-mvp-v0.md) (PR-B06 absolute bodies)  
**Layout:** [ADR-0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md) B0-A `collector/`  
**Sources:** [`collector/include/nytp_sink_v6.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytp_sink_v6.h), [`collector/src/nytp_sink_v6.c`](https://github.com/hilather/nytprof-modernization/blob/main/collector/src/nytp_sink_v6.c)  
**IDs:** [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h)  
**Independent reader:** `nytprof-format-v6` `decode_decoded_event_profile` (always-inflate; optional CRC)

---

## Intent

Extend the **C absolute v6 writer** with:

| Capability | Rule |
|------------|------|
| EVENT payload codecs | `NONE` (0), `ZLIB` (1), `ZSTD` (2), `LZ4` (3) — chunk-framed inflate bounds by `uncompressed_len` |
| Multi-chunk EVENT | records-per-chunk partition (`max_records_per_chunk`: `0` = unlimited single chunk; `n≥1` windows) |
| Header CRC | CRC-32/IEEE over fixed-header bytes `[0,32)` sealed into `header_crc` |
| Chunk payload CRC | CRC-32/IEEE over **wire payload bytes only** sealed into `payload_checksum` |

Aligned with Rust provisional helpers (`payload_codec`, `crc`, `multi_chunk_compressed`, always-inflate `decoded_event`).

## Surface (v0)

| Symbol | Role |
|--------|------|
| `nytp_v6_sink_create` / `create_ex` | codec NONE, unlimited chunks; CRC always sealed |
| `nytp_v6_sink_create_codec` / `create_codec_ex` | select codec + `max_records_per_chunk` |
| `nytp_v6_sink_event_codec` | configured codec id |
| `nytp_v6_sink_max_records_per_chunk` | partition limit (`0` unlimited) |
| `nytp_v6_sink_event_chunk_count` | EVENT frames sealed on close |

`create_ex` / `create_codec_ex` **ignore** a stored `header_crc` argument — CRC is always recomputed (PR-B07 seal policy).

### Seal rules (on `close`)

1. Sticky `FAILED` → refuse product EVENT seal (prefix-only; same as B06).
2. Empty body → zero EVENT chunks.
3. Else partition committed records by `max_records_per_chunk`.
4. Each partition: compress (if needed) → CRC payload → frame with `sequence` 0..k-1, `logical_event_count` = partition size, `uncompressed_len` = plain length.
5. **Atomic multi-chunk seal:** snapshot `wire_len` (prefix) at seal entry; on any mid-loop failure rewind wire to that mark, leave `sealed=0` / `event_chunk_count=0` so retry cannot append a second sequence stream.

### Codec notes

| Codec | Wire payload | Level / notes |
|------:|--------------|---------------|
| NONE | identity | |
| ZLIB | zlib-wrapped DEFLATE | level 6 (`compress2`) |
| ZSTD | zstd frame | level 3 |
| LZ4 | raw LZ4 block (size from `uncompressed_len`) | `LZ4_compress_default` |

Unsupported codec id at create → `NULL`.

### CRC (provisional)

Matches [`v6-crc-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-crc-provisional-v0.md): polynomial `0xEDB88320`, check `"123456789"` → `0xCBF43926`. Implemented via zlib `crc32`.

## Acceptance for this MVP

| Check | Evidence |
|-------|----------|
| Header + payload CRC sealed | `collector/t/test_v6_codec_chunk_crc.c` |
| Multi-chunk NONE / ZLIB / ZSTD / LZ4 | same |
| Uneven last window (`max=2`, 3 records) | same |
| Atomic mid-seal rewind + retry (no duplicate chunks) | same (`test_fail_seal_after_chunks` + `test_try_seal`) |
| Inflate roundtrip in C | same |
| Rust always-inflate accepts C artifacts | `cargo run -p nytprof-format-v6 --example decode_abs_c_mini -- <path> --require-crc` (header + payload CRC) |
| Default create still codec NONE | `test_v6_abs_wire` green |
| Smoke | `scripts/packaging/collector_sink_smoke.sh` builds/runs both v6 suites |

## Residuals / non-claims

| Residual | Owner |
|----------|-------|
| Packing (site-delta, TIME_*_RUN, FLAG_HAS_SEQ) | PR-B08 / ADR-0001 |
| FOOTER string dictionary | PR-B08 / ADR-0002 |
| Mid-stream payload codec switch after START_DEFLATE | PR-B08 |
| Board **COL-007 done** + E3-C fixtures | PR-B09 |
| Wire freeze | after E3/E4 |
| Always-on default CLI CRC verify | residual (optional verify flag) |
| Live Perl/XS hooks | later COL |
| BENCH writer cost | BENCH-004/006 |

## Tests

- `collector/t/test_v6_codec_chunk_crc.c` — CRC, multi-chunk, four codecs, fail-closed create
- `collector/t/test_v6_abs_wire.c` — absolute bodies / empty / fail-closed (still green under sealed CRC)

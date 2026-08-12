# Collector absolute v6 wire sink — MVP v0 (COL-007 staged / PR-B06)

**Status:** provisional scaffolding — **not** a v6 wire freeze; **not** board COL-007 done (E3-C = PR-B09)  
**Task:** COL-007 absolute-first stage (PR-B06)  
**Layout:** [ADR-0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md) B0-A `collector/`  
**Sources:** [`collector/include/nytp_sink_v6.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytp_sink_v6.h), [`collector/src/nytp_sink_v6.c`](https://github.com/hilather/nytprof-modernization/blob/main/collector/src/nytp_sink_v6.c)  
**IDs:** [`collector/include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h) + [`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md)  
**Independent reader:** `nytprof-format-v6` `decode_mini_profile` (codec NONE EVENT)

---

## Intent

Implement a **C absolute v6 writer** behind the semantic sink API:

- Provisional **file prefix** (36-byte fixed header + empty multi-TLV END)
- Absolute **EVENT** body records (ULEB opcodes + flags `0` + typed fields)
- Single **codec NONE** EVENT chunk sealed on `close`
- Fail-closed length / string / tick / NV projections

This is the **absolute-first** COL-007 stage. Packing (ADR-0001), FOOTER dict (ADR-0002), multi-chunk, and payload codecs are **later PRs**.

## Surface (v0)

| Symbol | Role |
|--------|------|
| `nytp_v6_sink_create(path)` | Create sink; writes file-prefix immediately |
| `nytp_v6_sink_create_ex(...)` | Same; minor / features / stored header CRC |
| `nytp_v6_sink_wire` / `wire_len` | Borrow bytes (**decoder-ready only after `close`**) |
| `nytp_v6_sink_event_body` | Borrow open event-body before seal |
| `nytp_v6_sink_is_sealed` | 1 after EVENT chunk framed on close |
| `nytp_v6_sink_stats` | Counting-compatible multiplicities / seq ring |

### Flush vs close (decoder-ready)

| Call | Sealed? | Path / buffer meaning |
|------|---------|------------------------|
| `nytp_sink_flush` mid-stream | no | Prefix (+ unframed open body not on wire). **Not** a complete mini-profile. |
| `nytp_sink_close` | yes | Seals EVENT chunk (codec NONE) when body non-empty; **post-close** bytes are mini-profile shaped. |

Empty event stream → prefix only (no EVENT chunk), matching Rust `encode_mini_profile` with empty events.

### Absolute event mapping (semantic → opcode)

| Emit | Opcode | Encoding notes |
|------|--------|----------------|
| attribute | 13 | string-blob key + value |
| option | 14 | string-blob key + value |
| comment | 15 | string-blob text |
| time_line | 2 | ULEB fid, line, ticks (`u64`; negative ticks → OVERFLOW) |
| time_block | 3 | ULEB fid, line, block_line, ticks (**drops `sub_line`**) |
| discount | 12 | empty typed body |
| new_fid | 8 | ULEB fid + filename (**drops eval_*/flags/size/mtime**) |
| src_line | 7 | ULEB fid, line + text |
| sub_info | 6 | ULEB fid, first, last + name |
| sub_callers | 11 | seven ULEB + called + caller; NV→`u64` |
| pid_start / pid_end | 9 / 10 | ULEB ids + time as `u64` |
| sub_entry / sub_return | 4 / 5 | site or depth + integer times + name |
| start_deflate | 16 | **empty marker only** (no payload codec switch) |

All records use **flags = 0** (no `FLAG_SITE_DELTA` / `FLAG_HAS_SEQ`).  
String-blob: `ULEB id || ULEB len || u8 flags || bytes` (`FLAG_UTF8` from view).  
COL-003 sequence numbers are **not** written on the wire.

### Fail-closed rules

| Condition | Result |
|-----------|--------|
| `ptr == NULL && len > 0` | `NYTP_ERR_NULL` before wire write |
| string len > 16 MiB | `NYTP_ERR_OVERFLOW` |
| event body / chunk payload > 64 MiB | `NYTP_ERR_OVERFLOW` |
| negative ticks | `NYTP_ERR_OVERFLOW` |
| NaN / negative / out-of-u64 NV times | `NYTP_ERR_OVERFLOW` |
| emit after seal | `NYTP_ERR_STATE` |

## Acceptance for this MVP

| Check | Evidence |
|-------|----------|
| Lockfile ID constants compile-match | `nytprof_v6_ids.h` + `test_v6_abs_wire` |
| Absolute TIME_LINE / ATTRIBUTE unit vectors | `collector/t/test_v6_abs_wire.c` |
| Full-tag mini + self-decode | same |
| Fail-closed null / overflow | same |
| Rust `decode_mini_profile` accepts C artifact | engineering check on `build/m4_mini_v6.nytprof` |
| Smoke includes `test_v6_abs_wire` | `scripts/packaging/collector_sink_smoke.sh` |

## Residuals / non-claims

| Residual | Owner |
|----------|-------|
| Packing (site-delta, TIME_*_RUN, FLAG_HAS_SEQ) | PR-B08 / ADR-0001 |
| Multi-chunk EVENT + CRC + ZLIB/ZSTD/LZ4 | PR-B07 |
| FOOTER string dictionary | PR-B08 / ADR-0002 |
| Board **COL-007 done** + E3-C fixtures | PR-B09 |
| Wire freeze | after E3/E4 (lockfile deviation note) |
| Live Perl/XS hooks | later COL |
| NEW_FID / TIME_BLOCK field drop vs full COMPAT-001 | residual until packing/catalog freeze |
| BENCH writer cost | BENCH-004/006 |

## Tests

- `collector/t/test_v6_abs_wire.c` — lockfile IDs, ULEB vectors, absolute TIME_LINE/ATTRIBUTE bytes, full-tag mini, empty prefix-only, fail-closed, no packing flags

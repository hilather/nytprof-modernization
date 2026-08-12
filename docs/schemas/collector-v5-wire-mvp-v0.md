# Collector legacy v5 wire sink — MVP v0 (COL-006)

**Status:** provisional scaffolding (not a wire freeze; not full M4 oracle corpus)  
**Task:** COL-006 (PR-B05)  
**Layout:** [ADR-0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md) B0-A `collector/`  
**Sources:** [`collector/include/nytp_sink_v5.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytp_sink_v5.h), [`collector/src/nytp_sink_v5.c`](https://github.com/hilather/nytprof-modernization/blob/main/collector/src/nytp_sink_v5.c)  
**Protocol:** baseline 6.15 `FileHandle.xs` / `FileHandle.h` (packed u32, tags, NV doubles, zlib after `z`)  
**Independent reader:** `nytprof-format-v5` / `nytprof-dump verify|dump`

---

## Intent

Implement **real NYTProf v5 wire encoding** behind the semantic sink API so `format=v5` can route through the overlay writer. Mini samples must be accepted by the Rust v5 decoder (and, when tools are present, unmodified 6.15 readers).

## Surface (v0)

| Symbol | Role |
|--------|------|
| `nytp_v5_sink_create(path)` | Create wire sink; writes `NYTProf 5 0\n` immediately |
| `nytp_v5_sink_create_ex(path, compress_level)` | Same; zlib level 1..9 for `START_DEFLATE` (0 → default 6) |
| `nytp_v5_sink_wire` / `wire_len` | Borrow in-memory profile bytes (**decoder-ready only after `close`**) |
| `nytp_v5_sink_file_written` | 1 after successful path write on flush/close |
| `nytp_v5_sink_is_deflating` | 1 after `START_DEFLATE` (body is zlib) |
| `nytp_v5_sink_stats` | Counting-compatible multiplicities / seq ring |

### Flush vs close (decoder-ready)

| Call | Deflating? | Path / buffer meaning |
|------|------------|------------------------|
| `nytp_sink_flush` mid-stream | yes | **Unfinished** zlib snapshot (`Z_FINISH` not called). **Not** a complete profile; Rust/6.15 verify may fail. |
| `nytp_sink_flush` | no | Complete records so far (safe if no further emits required for your use). |
| `nytp_sink_close` | any | Finishes deflate if active; **only post-close bytes are decoder-ready** for a finished stream. |

Prefer reading `nytp_v5_sink_wire` / path after **`close`**.

### Wire mapping (semantic → tag)

| Emit | Tag | Encoding notes |
|------|-----|----------------|
| attribute | `:` | `:key=value\n` |
| option | `!` | `!key=value\n` |
| comment | `#` | `#text\n` |
| time_line | `+` | packed i32 ticks, u32 fid, u32 line |
| time_block | `*` | + block_line, sub_line |
| discount | `-` | tag only |
| new_fid | `@` | id, eval_*, flags, size, mtime, string |
| src_line | `S` | fid, line, string |
| sub_info | `s` | **wire** fid, name, first, last |
| sub_callers | `c` | **wire** fid, line, caller, count, NVs, depth, called |
| pid_start / pid_end | `P` / `p` | pid (+ ppid), NV time |
| sub_entry / sub_return | `>` / `<` | fid/line or depth + NVs + name |
| start_deflate | `z` | then zlib (windowBits=15); no COL-003 seq |

Strings: `'` or `"` (UTF-8 flag) + packed len + bytes.  
NV: 8-byte native double (LE on fixture platforms).  
COL-003 sequence numbers are **not** written on the wire.  
String views: `ptr == NULL && len > 0` is rejected with `NYTP_ERR_NULL` **before** any wire write (no half-written tag/len).

### Ticks projection (OI-003-01 residual)

`nytp_ticks` (`int64_t`) must fit in **I32** or the emit fails with `NYTP_ERR_OVERFLOW` (sticky-fail). Full I32+overflow-seconds composition remains open.

## Acceptance for this MVP

| Check | Evidence |
|-------|----------|
| Unit wire self-decode + packed-u32 boundaries | `collector/t/test_v5_wire.c` |
| M4 mini + zlib inflate/parse | same + `test_fake_clock` |
| Rust decoder accepts M4 mini | `nytprof-dump verify collector/build/m4_mini_wire.nytprof` (when built) |
| Smoke includes `test_v5_wire` | `scripts/packaging/collector_sink_smoke.sh` |
| Isolation / no hard CC dep | smoke honest skip without CC |

## Residuals / non-claims

| Residual | Owner |
|----------|-------|
| Full `fixtures/v5/*` oracle stream equality under fake-clock | complete TEST-003 (+ corpus harness) |
| Live Perl/XS opcode hooks | later COL / packaging |
| C v6 writer | COL-007 |
| Dual-sink product path | **rejected** (OQ-4) — COL-014 is test/dev-only; see [`collector-dual-sink-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-dual-sink-mvp-v0.md) |
| BENCH certification of writer cost | BENCH-004 |
| Byte-identical oracle files | optional diagnostic mode (not required) |
| Multi-OS CI matrix for collector | BUILD-006 residual |

## Tests

- `collector/t/test_v5_wire.c` — uncompressed full-tag mini, M4+zlib, overflow, null-string fail-closed (no partial write), mid-deflate flush residual, no seq on wire  
- `collector/t/test_fake_clock.c` — M4 via v5 wire (header present, file written)  
- Existing sink/lifecycle/batch tests still green with real writer backend  

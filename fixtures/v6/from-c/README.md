# C-produced v6 EVENT fixtures (product E3 / COL-007)

**Status:** product E3-EVENT evidence (PR-B09) — **not** a v6 wire freeze  
**Producer:** C collector sink `nytp_v6_sink_*` (`collector/t/gen_e3_c_fixtures.c`)  
**Consumer:** Rust always-inflate E3 harness (`crates/nytprof-format-v6` `dual_equality::e3_decode_writer_bytes`)  
**Evidence:** `cargo test -p nytprof-format-v6 e3_c_` · `./tools/oracle/e3_c_writer_parity.sh`

These files are **C-produced only**. Product E3 tests load them from disk and
must never re-encode via Rust stand-in writers (`e3_standin_*`).

## Matrix

| File | Mode | Logical intent |
|------|------|----------------|
| `absolute.nytprof` | absolute NONE single-chunk | TIME_LINE×2 + TIME_BLOCK + SUB_ENTRY |
| `packing.nytprof` | packing ZLIB multi-chunk (max 2) | same sample; continuous sites + FLAG_HAS_SEQ |
| `dict.nytprof` | FOOTER string-dict absolute | ATTRIBUTE basetime + COMMENT + TIME_LINE |
| `packing_dict.nytprof` | packing + FOOTER dict multi-chunk | COMMENT + TIME_LINE×2 + COMMENT |
| `mid_stream.nytprof` | packing mid-stream NONE→ZLIB | TIME_LINE + TIME_LINE_RUN + START_DEFLATE + post site-delta |
| `mid_stream_dict.nytprof` | packing mid-stream NONE→ZSTD + FOOTER dict | mid-stream + resolved comments |

## Regenerate

Requires a C toolchain (zlib, zstd, lz4) and `make`:

```sh
make -C collector gen-e3-fixtures OUTDIR="$(pwd)/fixtures/v6/from-c"
# or
NYTPROF_REGEN_E3_C=1 ./tools/oracle/e3_c_writer_parity.sh
```

Committed binaries keep `cargo test` green without regenerating.

## Residuals

| Residual | Notes |
|----------|--------|
| **E3-mixed** | SOURCE/INDEX/SUMMARY multi-kind product C fixtures not claimed here |
| Wire freeze FMT-002..010 | still open after E3/E4 |
| CLI v6 default | residual |
| E4 v5↔v6 semantic enforcement | policy draft; fixture pairs open |
| COL-008 batched Rust writer | deferred / non-baseline |
| Live Perl/XS collector hooks | later COL |

## Schema

[`docs/schemas/collector-v6-e3-c-fixtures-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-e3-c-fixtures-mvp-v0.md)

# Format v6 golden vectors (wire freeze / FMT-012 class)

**Status:** immutable golden bytes under [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md)  
**Catalog:** [`docs/schemas/v6-wire-ids-frozen-v1.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-wire-ids-frozen-v1.md)  
**Checksums:** [`SHA256SUMS`](SHA256SUMS) in this directory  
**Tests:** `cargo test -p nytprof-format-v6 golden_vector_` · `wire_freeze_`

These vectors lock **major=6** wire encodings for independent C/Rust agreement. They are **not** oracle 6.15 captures and **not** a claim of CLI v6 default.

## Layout

```text
fixtures/v6/vectors/
  primitives/     # ULEB, ZigZag, fixed header, empty chunk, TLV region, string dict
  event/          # EVENT body plains (absolute, run, packing compose, dual-output order)
  profiles/       # mini absolute profile (file prefix + EVENT NONE)
  SHA256SUMS
  README.md
  manifest.json
```

## Matrix

| Path | Intent |
|------|--------|
| `primitives/uleb_*.bin` | Canonical ULEB128 (0, 1, 127, 128, 300, max u64) |
| `primitives/zigzag_*.bin` | ZigZag+ULEB signed (0, ±1, −2) |
| `primitives/fixed_header_full.bin` | 36-byte `NYTPROF6` full fixed header |
| `primitives/chunk_event_none_empty.bin` | 40-byte EVENT/NONE empty payload frame |
| `primitives/tlv_region_producer_tps.bin` | PRODUCER + TICKS_PER_SEC + END |
| `primitives/string_dict_one_entry.bin` | FOOTER dict table one entry id=1 `"hello"` |
| `event/time_line_1_2_3.bin` | TIME_LINE fid=1 line=2 ticks=3 |
| `event/discount.bin` | DISCOUNT empty body |
| `event/time_line_run_n2.bin` | TIME_LINE_RUN N=2 ticks 10,20 |
| `event/site_delta_seq_tl_tl_se.bin` | site-delta + seq TIME_LINE×2 + SUB_ENTRY |
| `event/dual_output_sequence.bin` | VERSION→COMMENT→START_DEFLATE→PID… **order only** (no FLAG_HAS_SEQ) |
| `event/dual_output_seq_oq5.bin` | OQ-5: VERSION+START_DEFLATE+TIME_LINE+DISCOUNT with FLAG_HAS_SEQ monotonic 0..3 |
| `profiles/mini_absolute_none.bin` | Mini profile TIME_LINE + DISCOUNT |

## Product C streams (related, not re-encoded here)

Cross-language product E3 fixtures remain under
[`fixtures/v6/from-c/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/from-c/)
and are loaded only as C-produced bytes by `e3_c_*` tests.

## Regenerate

Requires Cargo. **Do not** regenerate casually — changing bytes is a freeze regression unless ADR-0006 is superseded.

The example writes `.bin` files, `SHA256SUMS`, and `manifest.json` in one pure-Rust pass (no shell glob quoting).

```sh
cargo build -p nytprof-format-v6 --example gen_wire_vectors   # compile smoke
cargo run -p nytprof-format-v6 --example gen_wire_vectors -- fixtures/v6/vectors
# then verify (SHA256SUMS paths are relative to this directory):
cargo test -p nytprof-format-v6 --test golden_vectors
(cd fixtures/v6/vectors && sha256sum -c SHA256SUMS)
```

## Residuals

| Residual | Notes |
|----------|--------|
| E3-mixed multi-kind C vectors | still open |
| Full oracle workload vectors | TEST-003 / TEST-008 residual |
| Corrupt / unknown-feature matrix expansion | SEC/FMT residual |
| CLI v6 default | residual until R4 |

# Collector v6 E3-C fixtures + product dual-equality — MVP v0 (COL-007 / PR-B09)

**Status:** product E3-EVENT evidence for board **COL-007 done** — **not** a v6 wire freeze  
**Task:** COL-007 product claim via C-produced EVENT fixtures + Rust always-inflate E3 harness  
**Depends on:** [collector-v6-packing-footer-dict-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-packing-footer-dict-mvp-v0.md) (PR-B08); E3 harness [dual_equality.rs](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v6/src/dual_equality.rs) (PR-B08.5)  
**ADRs:** [0001 packing](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md); [0002 FOOTER-local dict](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md)  
**Contract:** [DUAL_EQUALITY_READINESS_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md)

---

## Intent

Close board **COL-007** for the **EVENT** product path by:

1. Emitting deterministic C-only profiles through `nytp_v6_sink_*` (absolute, packing, FOOTER dict, mid-stream).
2. Committing them under `fixtures/v6/from-c/**`.
3. Driving product E3 via `e3_decode_writer_bytes` / `e3_assert_logical_equal` on those bytes (**no** Rust stand-in encode).

## Surface

| Artifact | Role |
|----------|------|
| `collector/t/gen_e3_c_fixtures.c` | C fixture generator |
| `make -C collector gen-e3-fixtures` | Build + write matrix |
| `fixtures/v6/from-c/*.nytprof` | Committed C product bytes |
| `e3_c_*` tests in `crates/nytprof-format-v6/tests/e3_c.rs` | Product E3 equality (C fixtures only) |
| `tools/oracle/e3_c_writer_parity.sh` | Operator / offline evidence smoke |

### Fixture matrix

| File | Options |
|------|---------|
| `absolute.nytprof` | default create (absolute NONE) |
| `packing.nytprof` | packing + ZLIB + max_records_per_chunk=2 |
| `packing_lz4.nytprof` | packing + LZ4 + max_records_per_chunk=2 |
| `dict.nytprof` | enable_string_dict |
| `packing_dict.nytprof` | packing + dict + ZLIB multi-chunk |
| `mid_stream.nytprof` | packing + `begin_codec_region(ZLIB)` after TIME_LINE_RUN |
| `mid_stream_dict.nytprof` | packing + dict + `begin_codec_region(ZSTD)` |
| `mixed.nytprof` | EVENT sample + SOURCE + INDEX + SUMMARY (codec NONE kinds) |

## Acceptance

| Check | Evidence |
|-------|----------|
| C fixtures present with `NYTPROF6` magic | `e3_c_fixture_matrix_present`; parity script |
| Absolute logical sites (hand-built expected) | `e3_c_absolute_event_logical_equal` |
| Packing multi-chunk continuous sites/seq | `e3_c_packing_multi_chunk_logical_equal` |
| Packing LZ4 multi-chunk same logical sample | `e3_c_packing_lz4_multi_chunk_logical_equal` |
| FOOTER dict resolve | `e3_c_dict_footer_resolve` |
| Packing + dict multi-chunk | `e3_c_packing_dict_multi_chunk` |
| Mid-stream packing continuity | `e3_c_mid_stream_packing_continuity` |
| Mid-stream + dict | `e3_c_mid_stream_dict_packing` |
| Truncated C bytes fail closed | `e3_c_truncated_fail_closed` |
| E3-mixed SOURCE/INDEX/SUMMARY C bytes | `e3_c_mixed_kinds_source_index_summary`; truncated `e3_c_mixed_truncated_fail_closed` |
| Offline gate when cargo | `./tools/oracle/e3_c_writer_parity.sh` |

## Residuals / non-claims

| Residual | Notes |
|----------|--------|
| **E3-mixed** | **done (MVP)** — `mixed.nytprof` C-produced; not TEST-008 / COL-008 / CLI v6 collection default |
| Wire freeze FMT-002..010 | open after E3/E4 |
| E4 v5↔v6 semantic enforcement | policy draft; automation open |
| CLI v6 product default | residual |
| COL-008 batched Rust writer | deferred / non-baseline |
| Live Perl/XS hooks | later COL |
| Full oracle corpus via C | complete TEST-003 residual |

## Board

| Row | Status after this MVP |
|-----|------------------------|
| **COL-007** | **done** (E3-EVENT with C; E3-mixed **MVP**) |
| **COL-008** | still **deferred** |
| E3 harness stand-in | remains engineering only (not product evidence) |

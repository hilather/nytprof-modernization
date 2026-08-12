
# Collector overlay (ADR-0004 B0-A) — COL-001..007 + COL-014 dual (test/dev) + COL-015 fork + fake-clock scaffold

**Status:** scaffolding + product E3-EVENT (PR-B02..**B05 v5 wire** + **B06 absolute v6** + **B07 codecs/multi-chunk/CRC** + **B08 packing/FOOTER dict/mid-stream** + **B09 E3-C fixtures / board COL-007 done** + **B10a COL-014 dual-sink test/dev-only OQ-4** + **C02b COL-015 fork/PID with buffered sinks**)  
**Layout decision:** [ADR-0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md)  
**Logical events:** [COMPAT-001](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md)  
**Schemas:** [collector-sink-api-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-sink-api-mvp-v0.md), [collector-lifecycle-seq-fake-clock-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-lifecycle-seq-fake-clock-mvp-v0.md), [collector-batch-fast-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-batch-fast-mvp-v0.md), [collector-v5-wire-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v5-wire-mvp-v0.md), [collector-v6-absolute-wire-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-absolute-wire-mvp-v0.md), [collector-v6-codecs-multi-chunk-crc-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-codecs-multi-chunk-crc-mvp-v0.md), [collector-v6-packing-footer-dict-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-packing-footer-dict-mvp-v0.md), [collector-v6-e3-c-fixtures-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-e3-c-fixtures-mvp-v0.md), [collector-dual-sink-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-dual-sink-mvp-v0.md), [collector-fork-pid-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-fork-pid-mvp-v0.md)  
**Timing notes:** [BASE-003](https://github.com/hilather/nytprof-modernization/blob/main/baseline/inventories/timing-lifecycle-notes.md)

Modernization C sources for the **semantic event sink** live here — **not** under `baseline/6.15/` (oracle pin remains immutable).

## Tree

```text
collector/
  include/     public C headers (sink API, types, clock, batch/event, fork, counting + v5/v6 wire + dual + v6 IDs)
  src/         sink wrappers + backends + fake-clock + batch/fast path + fork protocol + v5 + v6 + dual writers
  t/           unit tests (no Perl; pure C; zlib/zstd/lz4 for wire; POSIX fork stress)
  xs/          reserved for future XS glue (empty)
  Makefile     opt-in C build (links -lz -lzstd -llz4)
  build/       gitignored objects / test binaries / sample .nytprof
  README.md    this file
```

## What is delivered

| Piece | Role |
|-------|------|
| `nytp_sink` + `nytp_sink_ops` | Canonical vtable sink interface |
| `nytp_emit_*` | Semantic emit surface (COMPAT-001 + `start_deflate` control) |
| **COL-002 lifecycle** | `OPEN/ACTIVE/STOPPED/FINALIZING/CLOSED/FAILED/FORK_SPLIT` + legal transitions + emit gates |
| **COL-003 sequence** | Gapless logical `nytp_seq` on successful emits (not for `START_DEFLATE`); **not** on default v5/v6 wire |
| **COL-004 fast path** | `nytp_fast_emit_time_line` / `_time_block` + POD batch append — no malloc after create |
| **COL-005 batching** | Fixed event buffer + side arena; high-water / full flush; emergency oversized path; batch sink facade |
| Counting sink | Test dual companion — multiplicities, seq ring, last src/sub fingerprints |
| **COL-006 v5 wire** | Real FileHandle.xs protocol encode + optional zlib after `START_DEFLATE`; path and/or in-memory buffer |
| **COL-007-ABS v6 wire** | Absolute provisional v6 EVENT bodies + file-prefix; lockfile IDs |
| **COL-007-CODEC (PR-B07)** | EVENT codecs NONE/ZLIB/ZSTD/LZ4; multi-chunk seal; header + payload CRC32 |
| **COL-007-PACK (PR-B08)** | ADR-0001 packing continuity; mid-stream codec region; ADR-0002 FOOTER string dict |
| **COL-007 product E3-EVENT (PR-B09)** | C-only fixtures under `fixtures/v6/from-c/`; `gen_e3_c_fixtures`; product `e3_c_*` Rust always-inflate equality |
| **COL-014 dual-sink (PR-B10a, OQ-4)** | **Test/dev-only** fan-out to v5+v6; same-run logical equality; env probe `NYTPROF_DUAL_SINK` / `NYTPROF_FORMAT=dual` (**not** product UX) |
| **COL-015 fork/PID (PR-C02b)** | `nytp_fork_prepare/resume_*` protocol; batch preflush + child residual discard; addpid paths; v5/v6 child reinit; dual reinit; stress `test_fork_pid` |
| **Fake-clock harness** | Scripted ticks + BASE-003 stmt driver + M4 **mini** sample |
| `make -C collector test` | `test_sink_api` + `test_lifecycle_seq` + `test_fake_clock` + `test_batch_fast` + **`test_v5_wire`** + **`test_v6_abs_wire`** + **`test_v6_codec_chunk_crc`** + **`test_v6_packing_footer`** + **`test_dual_sink`** + **`test_fork_pid`** |
| `make -C collector gen-e3-fixtures` | Write product E3-EVENT C matrix to `OUTDIR` (default `../fixtures/v6/from-c`) |

## Explicit non-claims

- **Not full M4 oracle corpus** — mini sample only; full `fixtures/v5/*` v5-via-sink equality needs complete TEST-003  
- **Board COL-007 is done for product E3-EVENT** (`fixtures/v6/from-c/`, `e3_c_*`, `tools/oracle/e3_c_writer_parity.sh`; schema [collector-v6-e3-c-fixtures-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-v6-e3-c-fixtures-mvp-v0.md)). **Residuals:** **E3-mixed** multi-kind C fixtures; wire freeze; CLI v6 default; E4 enforcement; live XS hooks; COL-008  
- **COL-015 MVP done** (protocol + buffered ownership + stress); residual full TEST-018 oracle forkdepth/addpid/merge, live XS, mid-deflate continue-in-child, signal-safe finalize  
- **Not** hooked into live Perl opcode profiler yet  
- **Not** a default dependency of `make legacy-smoke` or dual-path legacy half  
- Dual-sink (COL-014) is **test/dev only** (OQ-4) — not advertised product `format=dual`; full fixtures dual equality residual  
- Fake-clock is **test/dev only** — must not be production default  
- Light microbench in `test_batch_fast` is **engineering only** — not BENCH-003/006 certification  
- Flush/compression **discount timing** vs BASE-003 remains open (timing ADR residual)  
- `nytp_ticks` outside I32 fails closed (OI-003-01 composition residual)  
- Byte-identical oracle files not required (canonical stream equality is the bar)  
- Product E3 fixture matrix covers NONE/ZLIB/ZSTD/LZ4 packing (+ mid-stream); unit codec suite remains authoritative for edge codec fail-closed paths

## Build / test

Requires a C toolchain (`cc` / `gcc` / `clang`) and **zlib + zstd + lz4** (`-lz -lzstd -llz4`). Honest skip in CI when CC absent.

```sh
# from repo root
make -C collector
make -C collector test
# product E3-EVENT C fixtures (COL-007 / PR-B09):
make -C collector gen-e3-fixtures OUTDIR="$(pwd)/fixtures/v6/from-c"
./tools/oracle/e3_c_writer_parity.sh
# or packaging smoke (isolation asserts + build/test when CC present):
./scripts/packaging/collector_sink_smoke.sh

# optional: independent Rust v5 decoder on mini wire artifact
cargo build -p nytprof-cli
./target/debug/nytprof-dump verify collector/build/m4_mini_wire.nytprof
./target/debug/nytprof-dump dump  collector/build/m4_mini_wire.nytprof
```

Candidate install prefixes (when added later) **must** be `collector/install/` or `prefix/collector/` — **never** `baseline/6.15/install`. Never put `collector/` on oracle `PERL5LIB`.

## Lifecycle (COL-002 summary)

```text
OPEN --activate--> ACTIVE --stop--> STOPPED --activate--> ACTIVE
                      |                 |
                      +--begin_finalize-+--> FINALIZING --> close --> CLOSED
                      |
                      +--begin_fork--> FORK_SPLIT --> parent ACTIVE / child OPEN (seq reset)
                      +--mark_failed--> FAILED --> close --> CLOSED
```

Prefer **`nytp_fork_prepare` / `nytp_fork_resume_parent` / `nytp_fork_resume_child`** (COL-015) so buffered sinks preflush and child residual is discarded. Wire sinks: `nytp_v5/v6_sink_fork_child_reinit` for addpid path + clean stream.

Emit: all kinds in `OPEN`/`ACTIVE`; finalization subset in `FINALIZING`; none in `STOPPED`/`FORK_SPLIT`/`FAILED`/`CLOSED`.

## Sequence (COL-003 summary)

- Assigned by public emit wrappers on success; starts at 0; gapless per process stream.  
- `START_DEFLATE` is control — **no** seq.  
- Default v5 path does **not** write seq on wire (COL-006 preserves).  
- Batch flush replays to child ops with the **batch-stamped** seq (no double assign).  
- Comparators: `nytp_seq_check_gapless`, counting seq ring.

## Batching + fast path (COL-004 / COL-005 summary)

```text
nytp_emit_* / nytp_fast_emit_*  -->  batch (fixed headers + arena)
                                         |
                                    high-water / full / explicit flush
                                         v
                                   child sink ops (counting / v5 wire / later dual)
```

| Rule | Detail |
|------|--------|
| Statement path | `TIME_LINE`/`TIME_BLOCK` copy POD only; `heap_allocs` must not grow after create |
| Strings | Copied once into bounded arena; never retain caller/Perl pointers |
| Order | Forced capacities `1..64` must match direct counting kind+seq rings |
| Failure | Failed flush retains pending events; no phantom logical commits on child |
| Oversized | Emergency direct child emit after flush attempt |

## v5 wire (COL-006 summary)

```text
nytp_emit_*  -->  v5 sink (packed tags / strings / NV)
                      |
                 optional START_DEFLATE ('z') --> zlib body
                      |
                 in-memory buffer (+ optional path write on flush/close)
                      v
           nytprof-format-v5 / 6.15 tools (when wire is complete)
```

| Rule | Detail |
|------|--------|
| Header | `NYTProf 5 0\n` on create |
| Protocol | Matches 6.15 `FileHandle.xs` (packed u32/i32, string tags, LE NV) |
| Deflate | `emit_start_deflate` writes `z` then compresses subsequent bytes (level default 6) |
| Ticks | Must fit I32; else `NYTP_ERR_OVERFLOW` (sticky) |
| Seq | Internal only — not on wire |
| Strings | `ptr==NULL && len>0` → `NYTP_ERR_NULL` before any wire write |
| Flush vs close | Mid-deflate **flush** path bytes are unfinished zlib — **not** decoder-ready; only post-**close** is complete |
| API | `nytp_v5_sink_wire` / `file_written` for tests and handoff |

## Event mapping (semantic → COMPAT-001 / v5 tag)

| Emit API | Logical event | v5 tag | COL-006 action |
|----------|---------------|--------|----------------|
| `nytp_emit_attribute` | `attribute` | `ATTRIBUTE` `:` | wire + count + seq |
| `nytp_emit_option` | `option` | `OPTION` `!` | wire + count + seq |
| `nytp_emit_comment` | `comment` | `COMMENT` `#` | wire + count + seq |
| `nytp_emit_time_line` | `time_line` | `TIME_LINE` `+` | wire + count + seq |
| `nytp_emit_time_block` | `time_block` | `TIME_BLOCK` `*` | wire + count + seq |
| `nytp_emit_discount` | `discount` | `DISCOUNT` `-` | wire + count + seq |
| `nytp_emit_new_fid` | `new_fid` | `NEW_FID` `@` | wire + count + seq |
| `nytp_emit_src_line` | `src_line` | `SRC_LINE` `S` | wire + count + seq |
| `nytp_emit_sub_info` | `sub_info` | `SUB_INFO` `s` | wire + count + seq |
| `nytp_emit_sub_callers` | `sub_callers` | `SUB_CALLERS` `c` | wire + count + seq |
| `nytp_emit_pid_start` | `pid_start` | `PID_START` `P` | wire + count + seq |
| `nytp_emit_pid_end` | `pid_end` | `PID_END` `p` | wire + count + seq |
| `nytp_emit_sub_entry` | `sub_entry` | `SUB_ENTRY` `>` | wire + count + seq |
| `nytp_emit_sub_return` | `sub_return` | `SUB_RETURN` `<` | wire + count + seq |
| `nytp_emit_start_deflate` | *(control)* | `START_DEFLATE` `z` | wire + zlib switch, **no** seq |

Hooks will call **emit**, never raw v5 bytes, once integrated. Dual/v6 sinks plug the same vtable.

## Follow-on work

| Task / PR | Continues |
|-----------|-----------|
| Complete TEST-003 | Full corpus fake-clock oracle match |
| COL-007 | C v6 writer product E3-EVENT **done** (PR-B09); E3-mixed residual |
| COL-014 | Dual-sink **test/dev harness done** (PR-B10a); full oracle dual residual (TEST-003/TEST-008) |
| COL-015 residual | Full TEST-018 oracle forkdepth/addpid/merge + signal-safe finalize / live XS (MVP protocol + unit stress landed in PR-C02b) |
| BENCH-003 / BENCH-004 | Certified statement-path / writer component gates |

## Isolation

| Path | On oracle `PERL5LIB`? |
|------|------------------------|
| `baseline/6.15/install` | Yes (oracle only) |
| `collector/`, `collector/install/`, `prefix/collector/` | **Never** |
| `crates/`, candidate `perl/` | **Never** (oracle context) |

## Provisional v6 ID lockfile (C)

| Path | Role |
|------|------|
| [`include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h) | Mirrored provisional MAGIC / kind / codec / opcode / flag constants for COL-007 |

Normative note: [`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md).

**Not** a wire freeze. Board COL-007 product E3-EVENT is **done** (PR-B09); E3-mixed / CLI v6 / live XS residual.

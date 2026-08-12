# Collector overlay (ADR-0004 B0-A) — COL-001..003 + fake-clock scaffold

**Status:** scaffolding (PR-B02 COL-001 + PR-B03 COL-002/003/TEST-003 slice)  
**Layout decision:** [ADR-0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md)  
**Logical events:** [COMPAT-001](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md)  
**Schemas:** [collector-sink-api-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-sink-api-mvp-v0.md), [collector-lifecycle-seq-fake-clock-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-lifecycle-seq-fake-clock-mvp-v0.md)  
**Timing notes:** [BASE-003](https://github.com/hilather/nytprof-modernization/blob/main/baseline/inventories/timing-lifecycle-notes.md)

Modernization C sources for the **semantic event sink** live here — **not** under `baseline/6.15/` (oracle pin remains immutable).

## Tree

```text
collector/
  include/     public C headers (sink API, types, clock, counting + stub v5)
  src/         sink wrappers + backends + fake-clock
  t/           unit tests (no Perl; pure C)
  xs/          reserved for future XS glue (empty)
  Makefile     opt-in C build
  build/       gitignored objects / test binaries
  README.md    this file
```

## What is delivered

| Piece | Role |
|-------|------|
| `nytp_sink` + `nytp_sink_ops` | Canonical vtable sink interface |
| `nytp_emit_*` | Semantic emit surface (COMPAT-001 + `start_deflate` control) |
| **COL-002 lifecycle** | `OPEN/ACTIVE/STOPPED/FINALIZING/CLOSED/FAILED/FORK_SPLIT` + legal transitions + emit gates |
| **COL-003 sequence** | Gapless logical `nytp_seq` on successful emits (not for `START_DEFLATE`); backends record via post-commit `on_logical_committed` |
| Counting sink | Test dual companion — multiplicities, seq ring, no I/O |
| Stub v5 adapter | Conceptual route for legacy v5 writes (**no wire encode yet**) |
| **Fake-clock harness** | Scripted ticks + BASE-003 stmt driver + M4 **mini** sample |
| `make -C collector test` | `test_sink_api` + `test_lifecycle_seq` + `test_fake_clock` |

## Explicit non-claims

- **Not COL-006** — real v5 wire encoding / oracle writer adaptation  
- **Not COL-007** — C v6 writer  
- **Not full M4 oracle gate** — mini sample only; full `fixtures/v5/*` v5-via-sink equality needs COL-006 + complete TEST-003  
- **Not COL-015** — full fork buffer ownership / signal-safe finalization matrix  
- **Not** hooked into live Perl opcode profiler yet  
- **Not** a default dependency of `make legacy-smoke` or dual-path legacy half  
- Fake-clock is **test/dev only** — must not be production default

## Build / test

Requires a C toolchain (`cc` / `gcc` / `clang`). Honest skip in CI when absent.

```sh
# from repo root
make -C collector
make -C collector test
# or packaging smoke (isolation asserts + build/test when CC present):
./scripts/packaging/collector_sink_smoke.sh
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

Emit: all kinds in `OPEN`/`ACTIVE`; finalization subset in `FINALIZING`; none in `STOPPED`/`FORK_SPLIT`/`FAILED`/`CLOSED`.

## Sequence (COL-003 summary)

- Assigned by public emit wrappers on success; starts at 0; gapless per process stream.  
- `START_DEFLATE` is control — **no** seq.  
- Default v5 path does **not** write seq on wire (COL-006 must preserve).  
- Comparators: `nytp_seq_check_gapless`, counting seq ring.

## Event mapping (semantic → COMPAT-001 / v5 tag)

| Emit API | Logical event | v5 tag (conceptual) | Stub v5 action today |
|----------|---------------|---------------------|----------------------|
| `nytp_emit_attribute` | `attribute` | `ATTRIBUTE` `:` | count + seq |
| `nytp_emit_option` | `option` | `OPTION` `!` | count + seq |
| `nytp_emit_comment` | `comment` | `COMMENT` `#` | count + seq |
| `nytp_emit_time_line` | `time_line` | `TIME_LINE` `+` | count + seq + last fields |
| `nytp_emit_time_block` | `time_block` | `TIME_BLOCK` `*` | count + seq + last fields |
| `nytp_emit_discount` | `discount` | `DISCOUNT` `-` | count + seq |
| `nytp_emit_new_fid` | `new_fid` | `NEW_FID` `@` | count + seq |
| `nytp_emit_src_line` | `src_line` | `SRC_LINE` `S` | count + seq |
| `nytp_emit_sub_info` | `sub_info` | `SUB_INFO` `s` | count + seq |
| `nytp_emit_sub_callers` | `sub_callers` | `SUB_CALLERS` `c` | count + seq |
| `nytp_emit_pid_start` | `pid_start` | `PID_START` `P` | count + seq |
| `nytp_emit_pid_end` | `pid_end` | `PID_END` `p` | count + seq |
| `nytp_emit_sub_entry` | `sub_entry` | `SUB_ENTRY` `>` | count + seq |
| `nytp_emit_sub_return` | `sub_return` | `SUB_RETURN` `<` | count + seq |
| `nytp_emit_start_deflate` | *(control)* | `START_DEFLATE` `z` | count, **no** seq |

Hooks will call **emit**, never v5 bytes, once integrated. Dual/v6 sinks plug the same vtable.

## Follow-on work

| Task / PR | Continues |
|-----------|-----------|
| COL-004 / COL-005 | Hot-path / batching |
| COL-006 | Real v5 writer behind this API + full M4 stream neutrality |
| Complete TEST-003 | Full corpus fake-clock oracle match |
| COL-007 | C v6 writer (separate) |
| COL-015 | Full fork / signal lifecycle matrix |

## Isolation

| Path | On oracle `PERL5LIB`? |
|------|------------------------|
| `baseline/6.15/install` | Yes (oracle only) |
| `collector/`, `collector/install/`, `prefix/collector/` | **Never** |
| `crates/`, candidate `perl/` | **Never** (oracle context) |

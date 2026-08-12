# Collector overlay (ADR-0004 B0-A) — COL-001 semantic sink

**Status:** scaffolding (PR-B02 / COL-001 slice)  
**Layout decision:** [ADR-0004](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md)  
**Logical events:** [COMPAT-001](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md)  
**Schema notes:** [docs/schemas/collector-sink-api-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/collector-sink-api-mvp-v0.md)

Modernization C sources for the **semantic event sink** live here — **not** under `baseline/6.15/` (oracle pin remains immutable).

## Tree

```text
collector/
  include/     public C headers (sink API, types, counting + stub v5)
  src/         sink wrappers + backends
  t/           unit tests (no Perl; pure C)
  xs/          reserved for future XS glue (empty in COL-001)
  Makefile     opt-in C build
  build/       gitignored objects / test binary
  README.md    this file
```

## What this PR delivers

| Piece | Role |
|-------|------|
| `nytp_sink` + `nytp_sink_ops` | Canonical vtable sink interface |
| `nytp_emit_*` | Semantic emit surface (COMPAT-001 mapped events + `start_deflate` control) |
| Counting sink | Test dual companion — multiplicities, no I/O |
| Stub v5 adapter | Conceptual route for legacy v5 writes (**no wire encode yet**) |
| `make -C collector test` | Unit tests for routing / null guards / field fingerprints |

## Explicit non-claims

- **Not COL-006** — real v5 wire encoding / oracle writer adaptation  
- **Not COL-007** — C v6 writer  
- **Not COL-002** — full lifecycle state machine freeze  
- **Not COL-003** — monotonic logical sequence numbers  
- **Not PR-B03 / TEST-003** — fake-clock  
- **Not** hooked into live Perl opcode profiler yet  
- **Not** a default dependency of `make legacy-smoke` or dual-path legacy half

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

## Event mapping (semantic → COMPAT-001 / v5 tag)

| Emit API | Logical event | v5 tag (conceptual) | Stub v5 action today |
|----------|---------------|---------------------|----------------------|
| `nytp_emit_attribute` | `attribute` | `ATTRIBUTE` `:` | count |
| `nytp_emit_option` | `option` | `OPTION` `!` | count |
| `nytp_emit_comment` | `comment` | `COMMENT` `#` | count |
| `nytp_emit_time_line` | `time_line` | `TIME_LINE` `+` | count + last fields |
| `nytp_emit_time_block` | `time_block` | `TIME_BLOCK` `*` | count + last fields |
| `nytp_emit_discount` | `discount` | `DISCOUNT` `-` | count |
| `nytp_emit_new_fid` | `new_fid` | `NEW_FID` `@` | count |
| `nytp_emit_src_line` | `src_line` | `SRC_LINE` `S` | count |
| `nytp_emit_sub_info` | `sub_info` | `SUB_INFO` `s` | count |
| `nytp_emit_sub_callers` | `sub_callers` | `SUB_CALLERS` `c` | count |
| `nytp_emit_pid_start` | `pid_start` | `PID_START` `P` | count |
| `nytp_emit_pid_end` | `pid_end` | `PID_END` `p` | count |
| `nytp_emit_sub_entry` | `sub_entry` | `SUB_ENTRY` `>` | count |
| `nytp_emit_sub_return` | `sub_return` | `SUB_RETURN` `<` | count |
| `nytp_emit_start_deflate` | *(control)* | `START_DEFLATE` `z` | count |

Hooks will call **emit**, never v5 bytes, once integrated. Dual/v6 sinks plug the same vtable.

## Follow-on work

| Task / PR | Continues |
|-----------|-----------|
| COL-002 | Lifecycle freeze + transition tests |
| COL-003 | Sequence numbers for dual compare |
| COL-004 / COL-005 | Hot-path / batching |
| COL-006 | Real v5 writer behind this API |
| COL-007 | C v6 writer (separate) |
| PR-B03 | Fake-clock (TEST-003) — not this PR |

## Isolation

| Path | On oracle `PERL5LIB`? |
|------|------------------------|
| `baseline/6.15/install` | Yes (oracle only) |
| `collector/`, `collector/install/`, `prefix/collector/` | **Never** |
| `crates/`, candidate `perl/` | **Never** (oracle context) |

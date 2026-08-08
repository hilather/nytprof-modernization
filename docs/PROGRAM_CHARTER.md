# Program Charter — Devel::NYTProf Modernization

**Status:** binding for this repository  
**Ratified:** 2026-08-07  
**Normative plan package:** [`docs/plan/`](plan/) (imported from the modernization architecture package)

## Mission

Modernize Devel::NYTProf so exact statement/call profiling remains exact while reducing storage, parse/report CPU, and peak report memory. Compatibility with Devel::NYTProf 6.15 behavior and tools is a release blocker; performance claims are secondary and must never weaken precision.

## Non-negotiables (summary)

Full text: [`docs/plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md).

1. No sampling as a replacement for exact instrumentation.
2. No dropped statement, block, call, source, process, or metadata events.
3. No pre-aggregation that replaces the ordered event stream.
4. Preserve event order, timing semantics, counts, call relationships, source association, fork/process boundaries, and configuration modes.
5. Keep a v5 write path that unmodified 6.15 tools can read.
6. Keep v5 read and compatibility adapters for existing Perl APIs and CLIs.
7. Performance claims require repeatable regression and benchmark gates after semantic equality.

`COMPAT-000` ratification is recorded in [`docs/governance/COMPAT-000_RATIFICATION.md`](governance/COMPAT-000_RATIFICATION.md).

## Separately promotable outcomes

| Level | Outcome | Default change? |
|-------|---------|-----------------|
| **R0** | Developer preview (experimental flags) | No |
| **R1** | Native v5 read/report engine opt-in | No |
| **R2** | v6 collection opt-in (`format=v6`) | No (v5 remains default) |
| **R3** | `engine=auto` prefers native reports | Yes, only after field window + ADR |
| **R4** | v6 output default on eligible tiers | Yes, only after field window + ADR |
| **R5** | Legacy retirement consideration | Separate ADRs only; never automatic |

Native reporting and v6 collection must not be coupled into one all-or-nothing release.

## Architecture baseline (in force)

- **Collector:** C/XS stays on the interpreter hot path; no per-event Rust/FFI.
- **v6 writer baseline:** C encoder (COL-007). Batched Rust writer (**COL-008**) is **deferred / non-baseline** until dual-equality with C is green and an ADR re-opens it.
- **Offline path:** Rust for v5/v6 decode, compact model, aggregation, convert/merge, reports, tools.
- **Optional Rust install:** legacy-only builds must remain installable without Cargo on supported tiers.

Canonical crate names:

```text
nytprof-types, nytprof-format-v5, nytprof-format-v6,
nytprof-model, nytprof-aggregate, nytprof-report-ir,
nytprof-html, nytprof-cli, nytprof-ffi, nytprof-testkit
```

## First value slice (90-day intent)

Ship **R1-oriented foundations** before freezing v6 wire IDs:

1. Pin 6.15 oracle and immutable fixtures.
2. Canonical comparator + provisional event contract.
3. Independent Rust v5 decoder (after inventories).
4. Compact model + one opt-in native report path.
5. Benchmark harness noise study (no public claims until certified).
6. Early packaging spike (optional Cargo + legacy-only install).

**v6 wire freeze is not allowed** until the event contract is frozen, report-side evidence exists, and format prototypes are measured (see Phase-0 exit criteria).

## Decision process

- Architecture Decision Records: [`docs/adrs/`](adrs/)  
- Process: [`docs/governance/ARCH-008_ADR_PROCESS.md`](governance/ARCH-008_ADR_PROCESS.md)  
- Open questions: [`docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md`](plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md)  
- Task index: [`docs/plan/TASK_INDEX.md`](plan/TASK_INDEX.md)

Agents and humans own **tasks**, not architectural truth. Specs, ADRs, and immutable fixtures override local implementation preferences.

**Agent quality bars (tests, docs, release notes, performance/size, benchmarks):** [`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/main/AGENTS.md).

## Explicit non-goals (this charter window)

- Statistical sampling modes as product defaults.
- Freezing stable v6 numeric/wire IDs in the first slice.
- COL-008 batched Rust collector encoder prototype.
- Full CPAN public release or R3/R4 default flips.
- Completing all 206 plan tasks in one cycle.

## Success definition for the program

Success is R1 (or stronger) field-usable native reporting with exact parity on advertised surfaces, plus a clear path to R2, without breaking v5 oracle workflows or legacy-only installs. Performance improvements are measured only after semantic equality gates pass.

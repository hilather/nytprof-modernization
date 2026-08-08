# Agent hints — Devel::NYTProf modernization

**Status:** binding for every agent session (main and subagents) in this repository  
**Audience:** coding agents and human implementers acting as agents  
**Does not replace:** [`docs/PROGRAM_CHARTER.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md), [`docs/plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md), or accepted ADRs

Read this file before implementing, reviewing, or shipping changes. Agents own **tasks**, not architectural truth — specs, ADRs, immutable fixtures, and the charter override local preferences.

---

## Mandatory quality bars (always)

### 1. Regression tests for every fix

| Rule | Detail |
|------|--------|
| **No fix without a test** | Every bug fix, fail-closed path, regression, or behavioral repair **must** land with a **regression test** that fails before the fix and passes after. |
| **Drive shipped code** | Tests must call the **real entry point** (CLI, library API, script) — no reimplementation of the code under test, no hardcoded “always pass” stubs, no starting past the broken path. |
| **Keep the gate green** | Prefer `./scripts/ci/offline_gate.sh` (or the focused package suite the gate runs) after non-trivial changes. Do not merge or claim done with a known red offline gate. |
| **Fixture honesty** | Do not edit golden fixtures solely to make a broken implementation pass. Fixture changes require an approved semantic reason and dual-path evidence. |

### 2. Performance and output size must stay optimal

| Rule | Detail |
|------|--------|
| **No silent bloat** | Prefer the smaller, faster design when correctness is equal. Avoid unnecessary copies, full-buffer re-aggregations, and oversized intermediate files. |
| **Bound allocations** | Length-prefixed and frame parsers must fail closed on oversize lengths **before** large allocations (see provisional v6 schemas under `docs/schemas/`). |
| **Output size** | Encoded profiles, dumps, reports, and packaged artifacts should not grow without cause. When changing codecs, headers, or report emitters, measure and document size impact. |
| **No unequal benchmarks** | Do not disable features on only one side of a comparison (native vs oracle / new vs old). See feature-parity and bench plan docs. |
| **Claims** | Public performance claims require certified gates (plan `BENCH-*`). Light harnesses (`tools/bench/light_bench.sh`) inform engineering; they are **not** release certification by themselves. |

### 3. Documentation must stay current

| Rule | Detail |
|------|--------|
| **Same change set** | When behavior, CLI flags, schemas, packaging, or operator guidance changes, **update the matching docs in the same change** (board, residual matrix, runbook, schemas, README, BUILD policy as applicable). |
| **No silent capability claims** | Do not mark board/parity rows done without implementation; do not ship behavior without updating status docs that claim readiness. |
| **Residual honesty** | Keep residual limitations explicit (FFI, XS Data, full DOM, COL-007 full writer, multi-OS CI, perf claims). |
| **Absolute links** | In README / docs / release notes, use **absolute HTTPS URLs** for cross-file links (relative links break outside the tree). |

Project global rule also applies: structured code review after non-trivial changes; docs with behavior (see maintainer/agent global rules).

### 4. Release notes must list all relevant changes

| Rule | Detail |
|------|--------|
| **Complete delta** | Every release (tag, R0/R1-preview cut, or versioned package) **must** have release notes covering **all relevant changes** since the previous release: features, fixes, schema/CLI changes, packaging, known residuals, and upgrade notes. |
| **Where to write** | Prefer a dated entry under project release notes / `Changes` / GitHub Release body for the tag. Pin absolute links to the **version tag** for cross-file refs. |
| **No “misc fixes” dump** | Group by theme (CLI, format, Perl facade, packaging, docs). Call out breaking or fail-closed behavior changes explicitly. |
| **Omit noise** | Skip pure formatting/typo-only churn unless it affects operators; never omit behavioral or security-relevant fixes. |

### 5. Benchmarks must stay up to date (Perl oracle and prior versions)

| Rule | Detail |
|------|--------|
| **Vs Perl / 6.15 oracle** | When changing decode, model, report, export, or collector-adjacent paths that affect wall time, CPU, peak memory, or output size, refresh or extend comparisons against the **pinned oracle** path (`baseline/6.15/`, isolated `PERL5LIB` — **never** put `crates/` on oracle `PERL5LIB`). |
| **Vs previous versions** | Keep engineering baselines current against **prior native builds/tags** (or the last recorded harness snapshot in `docs/BENCH_NOTES.md` / plan BENCH tasks) so regressions are visible over time. |
| **Same workload** | Use the same fixtures and feature options on both sides of a comparison. |
| **Record results** | Update `docs/BENCH_NOTES.md` (or the certified bench package when present) with command, host notes, and direction of change — without inventing certification claims. |
| **Harness** | Local light harness: `./tools/bench/light_bench.sh`. Full certification remains plan WP-13 / `docs/plan/11_BENCHMARKING_AND_PERFORMANCE_GATES.md`. |

### 6. Failed attempts and language semantics (save automatically)

Negative knowledge is part of the repo. Agents **must** automatically record failed implementation attempts and corrected language misunderstandings so later sessions do not re-pay the same cost.

**Shape:** keep the **default notes light** (one short table row — cheap to load into context). **Drill down** into `docs/agent-notes/details/<slug>.md` only when a later agent would need more than one line (benches, stack traces, multi-step postmortems).

| Rule | Detail |
|------|--------|
| **When to write (automatic)** | Do **not** wait for a dedicated “write notes” task. Append a light row as soon as you **abandon** an approach, **fail a gate** after real effort, or **correct** a Perl/Rust misunderstanding — ideally in the **same change set**, otherwise immediately after aborting. |
| **Failed attempts** | Abandoned or gate-failed approaches — especially **perf** optimizations that regressed or did not win, but also format/API experiments that broke parity, packaging/CI dead-ends, rewrites that lost simplicity without benefit. **Append one short row** to [`docs/agent-notes/failed-attempts.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/failed-attempts.md). |
| **Language semantics (Perl & Rust)** | When **Perl** or **Rust** (or oracle / XS / dual-engine) behavior was **misunderstood** and then corrected — or remains open — append a short row to [`docs/agent-notes/language-semantics.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/language-semantics.md). Prefer pointers to fixtures/contracts over restating whole manuals. |
| **Light by default** | One table row only: date, slug/topic, what was tried or wrong assumption, why it failed / correct rule, optional detail link. **No** session transcripts, full logs, or essay postmortems in the light ledgers. |
| **Drill-down when needed** | If the light row is insufficient, add `docs/agent-notes/details/<slug>.md` and link it from the row. Index + templates: [`docs/agent-notes/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/README.md). |
| **Same honesty bar** | Notes record *what failed or was wrong* — they do **not** override ADRs, fixtures, residual matrix, or the charter. Failed perf rows are **not** certification claims. |
| **Before proposing big rewrites** | Skim the two light ledgers (and any linked details) so you do not re-propose a known-failed approach without new evidence. |

**Examples that always get a light row**

| Situation | Ledger |
|-----------|--------|
| Perf tweak tried; wall time / peak RSS / size worse or no win | `failed-attempts.md` |
| Alternate codec / API / packaging path abandoned after red gate or wrong parity | `failed-attempts.md` |
| Misread Perl oracle option, call multiplicity, or `PERL5LIB` isolation | `language-semantics.md` (`perl`) |
| Misread Rust ownership, fail-closed parse, or crate API contract | `language-semantics.md` (`rust`) |

---

## Working rules (agents)

1. Prefer small, mergeable slices that keep the offline gate green.  
2. Never put `crates/` on oracle `PERL5LIB`.  
3. Derive counts and aggregates from dump/model/JsonlData — do not invent fixture constants detached from real loads.  
4. Provisional v6 preflight (`nytprof-format-v6`, `docs/schemas/v6-*-provisional-v0.md`) is **not** a wire freeze and does **not** complete COL-007 (C v6 writer).  
5. Stop and escalate when observed 6.15 behavior contradicts frozen specs; do not guess wire or timing semantics.  
6. Handoffs include commits, commands, artifacts, open questions, and known limitations.  
7. **Automatically** append light rows under `docs/agent-notes/` for abandoned attempts (incl. failed perf) and corrected Perl/Rust misunderstandings; open a `details/<slug>.md` only when the light row is not enough.

## Primary gates and docs

| Item | Path |
|------|------|
| Offline R1 gate | `./scripts/ci/offline_gate.sh` |
| Operator runbook | [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) |
| First-slice board | [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md) |
| Residual matrix | [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) |
| Agent work packages | [`docs/plan/14_AGENT_WORK_PACKAGES_AND_HANDOFFS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/14_AGENT_WORK_PACKAGES_AND_HANDOFFS.md) |
| Bench notes | [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md) |
| Agent notes (failed attempts + language) | [`docs/agent-notes/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/README.md) |

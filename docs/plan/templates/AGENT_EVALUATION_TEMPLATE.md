# Agent Evaluation - WP-XX / TASK-IDs

## Metadata

- **Evaluator:**
- **Date:**
- **Work package:**
- **Task IDs evaluated:**
- **Candidate repository/commit:**
- **Oracle version/build:** Devel::NYTProf v6.15 / pinned manifest ID
- **Status recommendation:** ready | blocked | split | defer | reject-with-ADR

## Scope decision

| Task ID | Accept, split, defer, or reject | Reason | Proposed owner | Blocking dependencies |
|---|---|---|---|---|
| | | | | |

State explicitly what is outside this evaluation. A rejected task requires an ADR or a replacement task; it must not disappear from the parity matrix.

## Dependency evidence reviewed

| Dependency/task/ADR | Artifact or commit | Checksum/version | Result | Gaps |
|---|---|---|---|---|
| | | | | |

Confirm that the legacy oracle and candidate build are isolated and that artifacts were not produced by a mixed module/library path.

## Legacy behavior and source touchpoints

Document the v6.15 files, functions, records, callbacks, APIs, CLIs, reports, tests, and fixtures that constrain this work. Separate observed behavior from source comments or proposed behavior.

## Exactness and compatibility analysis

Address all applicable points:

- canonical event types, fields, multiplicity, and order;
- clock reads, ticks, discounting, attribution, calls, and overflow;
- source/name bytes and UTF-8 flags;
- process, fork, start/stop, finalization, and partial files;
- v5 read/write and unmodified v6.15 consumer behavior;
- Perl object/callback behavior;
- CLI/report/artifact behavior;
- platform, Perl, compiler, codec, and legacy-only support;
- strict conversion/merge representability.

## Assumptions and ADR requirements

| Assumption/question | Why it matters | Evidence needed | ADR-Q or proposed ADR | Blocks which slice |
|---|---|---|---|---|
| | | | | |

Do not encode an unresolved assumption into stable wire bytes, public API, timing state, or defaults.

## Proposed implementation slices

Each slice must be independently reviewable and leave a passing legacy path.

| Slice | Task IDs | Observable change | Files/components | Tests before merge | Rollback/disable |
|---|---|---|---|---|---|
| 1 | | | | | |

## Regression plan

List exact matrix rows, fixtures, deterministic-clock scripts, canonical comparisons, API/CLI/report comparisons, malformed/fault cases, platform tiers, and commands. Identify the independent oracle for every assertion.

## Performance and storage plan

Specify feature-equivalent configurations, workloads, metrics, repetitions, noise controls, semantic hashes, and proposed gates. No measurement is admissible before equality checks pass.

## Security and reliability plan

Cover untrusted lengths/bytes, decompression, arithmetic, FFI, filesystem/HTML, resource limits, fork/finalization, partial writes, recovery, and supply chain as applicable.

## Risk update

| Existing/new risk | Likelihood | Impact | Trigger | Mitigation/task | Owner |
|---|---|---|---|---|---|
| | | | | | |

## Recommendation

State whether work may start, what must happen first, which slices can run in parallel, and the evidence required for the next review.

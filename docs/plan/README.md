# Devel::NYTProf Modernization Plan

**In-repo location:** this package lives at `docs/plan/` in the modernization repository. Program governance (charter, Phase-0 exit criteria, first-slice board, ADR process, COMPAT-000 ratification) lives one level up under `docs/`.

This package is an implementation-ready architecture and task plan for modernizing Devel::NYTProf 6.15 without reducing profiling precision or feature coverage. It contains 206 uniquely identified implementation, evaluation, certification, and rollout tasks, 16 directly assignable agent work packages, 30 tracked risks, and 26 blocking architecture-decision questions.

**COL-008** (batched Rust v6 writer) is **deferred / non-baseline**; the production writer path is the C encoder (COL-007).

The plan assumes a hybrid implementation:

- Retain C/XS for Perl interpreter hooks and the statement/call capture hot path.
- Introduce a canonical C event/sink boundary with bounded buffering and a C v6 writer as the baseline; evaluate a batched-Rust encoder only as a measured alternative, never through per-event FFI.
- Add a lossless, portable, independently chunked v6 profile format.
- Read both v5 and v6 profiles in the new implementation.
- Continue to emit v5 profiles for old tools and regression testing.
- Reimplement parsing, aggregation, conversion, merge, and report generation in Rust while retaining the legacy Perl/C implementation as an oracle and fallback during migration.

## Non-negotiable constraints

1. No sampling.
2. No dropped statement, block, call, source, process, or metadata events.
3. No pre-aggregation that replaces the ordered event stream.
4. Preserve event order, timing semantics, execution counts, call relationships, source association, fork/process boundaries, and all existing configuration modes.
5. Keep a v5 write path that old Devel::NYTProf 6.15 tools can read.
6. Keep a v5 read path and compatibility adapters for existing Perl APIs and command-line tools.
7. Make all performance claims pass repeatable regression and benchmark gates.

## How to use this package

Agents should begin with these files:

1. [`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/main/AGENTS.md) (repo root) — mandatory quality bars: regression tests for every fix, optimal performance/output size, current docs, complete release notes, benchmarks vs oracle and prior versions.
2. `01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`
3. `03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md`
4. `agent-work-packages/README.md` and the assigned `WP-*.md` brief.
5. `TASK_INDEX.md`, then every source workstream file containing assigned task IDs.
6. `10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`
7. `16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md`

Every executable task has an explicit status, size, dependency set, owner profile, work definition, deliverables, and acceptance criteria. Compatibility-sensitive implementation tasks also carry local regression gates; all tasks inherit the global differential and release gates. The task template provides optional rationale, rollback, and risk fields:

- **Goal**: outcome, not an implementation preference.
- **Rationale**: why the task exists.
- **Dependencies**: task IDs that must land first.
- **Work**: concrete implementation or investigation steps.
- **Deliverables**: files, code, fixtures, reports, or decisions expected.
- **Acceptance**: objective completion checks.
- **Regression gate**: compatibility tests that must remain green.
- **Risks/notes**: conditions an agent must not overlook.
- **Suggested owner**: the most suitable agent specialization.

Task status should use one of: `proposed`, `ready`, `in-progress`, `blocked`, `review`, `done`, `deferred`, or `rejected-with-ADR`.

## Document map

| File | Purpose |
|---|---|
| `00_EXECUTIVE_ARCHITECTURE.md` | Architectural summary and major decisions |
| `01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md` | Precision, API, format, CLI, and report compatibility contract |
| `02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md` | Baseline source map and optimization hypotheses |
| `03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md` | Target components, data flow, interfaces, and repository layout |
| `04_FILE_FORMAT_V6_TASKS.md` | Lossless v6 format design and implementation tasks |
| `05_COLLECTOR_AND_C_XS_TASKS.md` | Hot-path, buffering, timing, fork, and writer tasks |
| `06_RUST_CORE_AND_DATA_MODEL_TASKS.md` | Rust parser, model, aggregation, FFI, and concurrency tasks |
| `07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md` | Existing Perl/XS API preservation and fallback tasks |
| `08_REPORT_GENERATION_TASKS.md` | HTML, calls, flame, CSV, Callgrind, and deterministic rendering tasks |
| `09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md` | CLI parity, v5/v6 converters, validation, and mixed-format merge |
| `10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md` | Golden fixtures, semantic comparison, fuzzing, and matrix tests |
| `11_BENCHMARKING_AND_PERFORMANCE_GATES.md` | Collection, storage, report, memory, and compression benchmarks |
| `12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md` | Cargo/MakeMaker coexistence, CI, fallback, release, and distribution |
| `13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md` | Untrusted input, partial profiles, checksums, limits, and HTML safety |
| `14_AGENT_WORK_PACKAGES_AND_HANDOFFS.md` | Parallel agent packages, boundaries, and handoff protocol |
| `15_PHASES_DEPENDENCIES_AND_CRITICAL_PATH.md` | Phase gates, dependency graph, and integration order |
| `16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md` | Project and release definition of done |
| `17_RISK_REGISTER.md` | Risks, mitigations, triggers, and ownership |
| `18_OPEN_QUESTIONS_AND_ADR_QUEUE.md` | Decisions that must be resolved and recorded |
| `19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md` | Staged opt-in releases, default promotion, rollback, migration, and long-term support |
| `FEATURE_PARITY_MATRIX.md` | Seeded end-to-end feature, option, API, report, platform, and regression traceability matrix |
| `agent-work-packages/` | Sixteen directly assignable agent briefs with evaluation, execution, and handoff checklists |
| `templates/` | Agent evaluation, handoff, ADR, and evidence-bundle templates |
| `TASK_INDEX.md` | Consolidated human-readable index of all task IDs, dependencies, status, and source files |
| `TASK_INDEX.json` | Machine-readable task index for orchestration and automated dependency checks |
| `TASK_TEMPLATE.md` | Copyable task template |
| `SOURCES.md` | Primary-source references used to ground the plan |
| `MANIFEST.md` | Package file inventory, line counts, sizes, and document titles |
| `VALIDATION_REPORT.md` | Package consistency, dependency-cycle, link, task-reference, and artifact validation results |
| `SHA256SUMS` | Cryptographic integrity checksums for package files |

## Package navigation

- Start with `00_EXECUTIVE_ARCHITECTURE.md` for the target design and sequencing rationale.
- Use `TASK_INDEX.md` to locate any of the 206 tasks and its authoritative workstream file.
- Assign agents from `agent-work-packages/README.md`; each brief requires an evaluation artifact before implementation.
- Treat `01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`, accepted ADRs, immutable fixtures, and release gates as normative.
- Use `19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md` to keep native reporting, v6 collection, and default changes independently promotable and independently reversible.

## Recommended first execution slice

The lowest-risk, highest-information first slice is:

1. Freeze the compatibility contract and create canonical v5 fixtures.
2. Implement a standalone Rust v5 decoder and canonical event dumper.
3. Differentially compare it with the current C/Perl reader.
4. Implement a compact Rust data model and one report output behind an opt-in engine flag.
5. Benchmark report time and memory before modifying collection.
6. Only then freeze the v6 format and alter the collector.

This sequence creates a trusted oracle before changing the most timing-sensitive code.

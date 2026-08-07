# 14 - Agent Work Packages and Handoff Protocol

## Purpose

Turn the architecture into parallel, reviewable work packages that multiple engineering agents can execute without independently redefining compatibility, event semantics, or file-format rules.

Agents own tasks, not architectural truth. The compatibility contract, normative specs, accepted ADRs, and immutable fixtures are shared constraints.

## Global rules for every agent

1. Read `01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md` first.
2. Read the dependency task outputs and current ADRs before implementation.
3. Do not change a stable event/wire/API ID without a format/API ADR and migration plan.
4. Do not update golden fixtures to make a failing implementation pass without an approved semantic diff.
5. Every optimization includes correctness hashes and before/after measurements.
6. Every new parser/writer feature includes positive, boundary, malformed, truncation, and resource-limit tests.
7. Keep legacy and native paths independently selectable until the rollout plan retires one.
8. Handoffs must include exact commits, commands, artifacts, open questions, and known limitations.
9. Prefer small mergeable slices that retain a green oracle path.
10. Stop and escalate when observed v6.15 behavior contradicts the frozen specification; do not guess.

## Work package template

```text
Package ID:
Task IDs:
Objective:
Owner/specialty:
Inputs and frozen dependencies:
Files/components allowed to change:
Files/components read-only:
Expected deliverables:
Required tests:
Required benchmark/security evidence:
Handoff consumers:
Known risks:
Status:
```

## Package WP-00 - Program architecture and governance

- **Tasks:** ARCH-001 through ARCH-008; COMPAT-000 through COMPAT-013; ADR queue ownership.
- **Owner profile:** senior systems/performance architect with Perl internals and binary-format experience.
- **Objective:** Freeze canonical events, timing semantics, component boundaries, feature negotiation, and decision process.
- **Can run in parallel with:** baseline fixture generation after provisional schemas exist.
- **Blocks:** collector refactor, v6 freeze, native model compatibility, report replacement.
- **Required handoff:** approved event schema, sink lifecycle, v5 semantic spec, v6 draft, compatibility classification, ADR index.
- **Exit gate:** no unresolved ambiguity for a current v5 record/callback or timing field.

## Package WP-01 - v6.15 executable oracle and fixtures

- **Tasks:** BASE-001 through BASE-008; TEST-001; TEST-002; TEST-004.
- **Owner profile:** Perl/XS test engineer.
- **Objective:** Pin the old implementation and capture event, API, CLI, report, platform, and performance baselines.
- **Can run in parallel with:** architecture inventory, build-system spike.
- **Blocks:** independent v5 reader acceptance and all regression claims.
- **Required handoff:** immutable v5 files, regeneration scripts, callback traces, object/aggregate dumps, report trees/manifests, performance baseline.
- **Exit gate:** oracle is isolated and reproducible; fixture provenance/checksums complete.

## Package WP-02 - Differential comparison framework

- **Tasks:** TEST-001 through TEST-003; TOOL-002; TOOL-003.
- **Owner profile:** test framework/data comparison engineer.
- **Objective:** Build canonical schemas, streaming comparators, hashes, mismatch bundles, and minimization.
- **Inputs:** WP-00 event/normalization specs, WP-01 fixtures.
- **Blocks:** dual-output acceptance, native reader/model/report sign-off.
- **Required handoff:** comparator library/CLI, schemas, seeded mutation tests, documented normalization version.
- **Exit gate:** all intentional semantic mutations are detected and localized.

## Package WP-03 - Independent Rust v5 reader

- **Tasks:** RUST-001 through RUST-005; TEST-005; TEST-006; TEST-017.
- **Owner profile:** senior Rust parser engineer.
- **Objective:** Stream exact v5 events without Perl object materialization.
- **Inputs:** WP-00 v5 spec, WP-01 fixtures, WP-02 comparator.
- **Can run in parallel with:** compact model design and build packaging, once domain types stabilize.
- **Required handoff:** crate/API, coverage, platform/NV support matrix, canonical parity results, unresolved exotic representation notes.
- **Exit gate:** new-reader/old-writer event parity across required fixtures/platform tiers.

## Package WP-04 - Compact model and exact aggregation

- **Tasks:** RUST-006 through RUST-009; RUST-015; RUST-016; TEST-010; BENCH-008.
- **Owner profile:** Rust data-model engineer with profiler semantics knowledge.
- **Objective:** Replace Perl object amplification for report workloads while retaining all exact values.
- **Inputs:** WP-03 stream API, WP-00 timing/call specs, WP-01 aggregate oracle.
- **Can run in parallel with:** v6 format prototyping and report IR inventory.
- **Required handoff:** model schema/invariants, aggregate comparator results, memory layout, benchmark data, report IR API.
- **Exit gate:** exact aggregate parity and ratified memory/time evidence.

## Package WP-05 - Perl API/XS native adapter

- **Tasks:** PERL-001 through PERL-014; RUST-010; TEST-010; TEST-013.
- **Owner profile:** Perl XS + Rust FFI engineer.
- **Objective:** Preserve callback and object APIs using coarse native operations.
- **Inputs:** WP-03 reader, WP-04 model, WP-00 API inventory.
- **Can run in parallel with:** native report renderer after FFI handle/lifetime API is frozen.
- **Required handoff:** stable C ABI, XS adapter, engine selection, callback/object differential results, error map.
- **Exit gate:** same public test suite passes under legacy and native engines.

## Package WP-06 - Native report engine

- **Tasks:** REPORT-001 through REPORT-020; RUST-017; TEST-009; BENCH-009; BENCH-010.
- **Owner profile:** Rust report/rendering engineer plus report test specialist.
- **Objective:** Produce all existing reports faster and with lower RSS, deterministically.
- **Inputs:** WP-04 model/IR, WP-01 report oracle, WP-02 comparator.
- **Suggested split:**
  - WP-06A report inventory/IR/comparator;
  - WP-06B summary/source HTML;
  - WP-06C calls/flame/CSV/Callgrind/graph;
  - WP-06D deterministic parallelism/storage/optimization.
- **Required handoff:** report manifest, renderers, semantic/DOM parity, performance evidence, security escaping/path review.
- **Exit gate:** all selected outputs match semantics and report performance gates.

## Package WP-07 - v6 normative format and vectors

- **Tasks:** FMT-001 through FMT-015; RUST-011, RUST-012, RUST-014, RUST-018; TEST-014, TEST-015; SEC-003, SEC-004.
- **Owner profile:** binary-format architect plus independent parser reviewer.
- **Objective:** Freeze a portable, lossless, chunked v6 format with reversible compression features.
- **Inputs:** WP-00 event model, WP-01 real-size data, WP-02 comparator, WP-03 v5 semantics.
- **Can run in parallel with:** native v5 report path; should not block early report gains.
- **Required handoff:** normative spec, immutable vectors, reference encoder/decoder, codec/chunk benchmark ADR, corruption/recovery semantics.
- **Exit gate:** format review complete; independent implementations agree; no open required-field ambiguity.

## Package WP-08 - Collector event-sink refactor and v5 neutrality

- **Tasks:** COL-001 through COL-006; COL-016 through COL-018; TEST-003; TEST-007.
- **Owner profile:** senior C/XS profiler engineer.
- **Objective:** Introduce semantic sink boundary while keeping v5 behavior and overhead neutral.
- **Inputs:** WP-00 event/timing specs, WP-01 oracle, WP-02 comparator.
- **Blocks:** production v6 writer.
- **Required handoff:** event API, v5 adapter, fake-clock suite, hot-path baseline/assembly, byte/semantic parity.
- **Exit gate:** refactor gate C1/P1 passes before v6 encoding work merges onto main.

## Package WP-09 - C v6 writer and collector optimizations

- **Tasks:** COL-007 through COL-015; TEST-008, TEST-015, TEST-018, TEST-019; BENCH-005 through BENCH-007.
- **Owner profile:** C binary encoder/performance engineer.
- **Objective:** Emit exact v6 directly from the collector with dictionaries, deltas, chunks, checksums, and selected codec.
- **Inputs:** WP-07 stable format/vectors, WP-08 sink/tick layer, WP-02 comparator.
- **Suggested split:**
  - WP-09A writer skeleton/vectors;
  - WP-09B dictionaries/source blobs;
  - WP-09C deltas/chunks/codecs/buffering;
  - WP-09D fork/lifecycle/dual/fault tests.
- **Required handoff:** C writer, dual pairs, performance/size data, collector audit.
- **Exit gate:** same-run exact equality, old v5 path parity, storage/collector gates, no unresolved lifecycle issue.

## Package WP-10 - Conversion, merge, verification, and recovery tools

- **Tasks:** TOOL-001 through TOOL-016; RUST-013; TEST-011 through TEST-014; BENCH-012.
- **Owner profile:** Rust tooling/format engineer.
- **Objective:** Make v5/v6 interoperable and diagnosable without loss.
- **Inputs:** WP-03 reader, WP-07 v6, WP-02 comparator, WP-04 aggregation.
- **Required handoff:** strict converters, mixed merge, verify/inspect/repack/salvage commands, manifests, old-tool compatibility results.
- **Exit gate:** canonical hashes prove successful operations; unrepresentable conversion fails before publish.

## Package WP-11 - Build, packaging, platform, and CI

- **Tasks:** BUILD-001 through BUILD-015; PERL-012; TEST-017; TEST-020.
- **Owner profile:** CPAN/MakeMaker/Rust release engineer.
- **Objective:** Support native and legacy-only modes across explicit platform tiers.
- **Inputs:** early workspace/FFI prototypes; evolves throughout program.
- **Can run in parallel with:** all implementation lanes.
- **Required handoff:** tier policy, build modes, CI matrix, loader, offline/source packaging, artifact pipeline.
- **Exit gate:** clean installed artifacts pass required matrices; no hidden native dependency for legacy-only mode.

## Package WP-12 - Security and supply-chain hardening

- **Tasks:** SEC-001 through SEC-012; TEST-014; TEST-016; BUILD-009; BUILD-011.
- **Owner profile:** native parser/application security engineer independent of primary implementers.
- **Objective:** Harden inputs, codecs, FFI, report content/paths, native loading, and release artifacts.
- **Inputs:** designs early; code continuously.
- **Required handoff:** threat model, audit reports, fuzz corpora/results, security sign-off.
- **Exit gate:** no unresolved high-severity issue; required fuzz/security gates complete.

## Package WP-13 - Benchmarking and performance certification

- **Tasks:** BENCH-001 through BENCH-014; TOOL-011; BASE-007; BASE-008.
- **Owner profile:** performance engineer/statistician independent of feature owners.
- **Objective:** Produce reproducible evidence and reject semantically invalid speedups.
- **Inputs:** all lanes provide telemetry and correctness hashes.
- **Required handoff:** ratified gates, raw results, trend dashboard, certification.
- **Exit gate:** release claims supported; workload-level regressions reviewed.

## Package WP-14 - CLI compatibility and user migration

- **Tasks:** TOOL-010 through TOOL-016; PERL-002, PERL-010, PERL-014; BUILD-014; REL-001 through REL-013.
- **Owner profile:** Perl CLI/release/documentation engineer.
- **Objective:** Preserve existing commands, expose engine/format controls, and guide safe rollout.
- **Inputs:** native reports/tools/build capabilities.
- **Required handoff:** wrappers, help/docs, migration/conversion guides, rollback instructions.
- **Exit gate:** old workflows pass; new capabilities are explicit; legacy forcing works.

## Package WP-15 - Independent final certification

- **Tasks:** COMPAT-014, COL-018, RUST-018, REPORT-020, TOOL-016, BUILD-015, SEC-012, TEST-020, BENCH-013.
- **Owner profile:** release review group not primary implementers.
- **Objective:** Decide readiness for opt-in, native-report default, and v6-output default separately.
- **Inputs:** signed workstream evidence.
- **Required handoff:** release decision, exceptions/waivers, rollback triggers.
- **Exit gate:** all phase-specific definition-of-done clauses satisfied.

## Recommended parallel execution waves

### Wave 1 - Freeze truth

Run WP-00, WP-01, WP-02 foundations, WP-11 tier/build discovery, and WP-12 threat modeling.

### Wave 2 - Accelerate reports without changing collection

Run WP-03, WP-04, WP-05 foundations, WP-06 inventory/IR, and WP-13 baselines.

### Wave 3 - Freeze v6 and refactor collector safely

Run WP-07 and WP-08 in parallel with continued native report implementation.

### Wave 4 - Implement v6 and interoperability

Run WP-09 and WP-10; complete WP-06; expand WP-11/12/13 matrices.

### Wave 5 - Rollout and certification

Run WP-14 and WP-15 with full regression, performance, security, artifact, and migration evidence.

## Handoff artifact requirements

Every completed package supplies:

```text
HANDOFF.md
  scope completed
  task/ADR IDs
  commit hashes
  build/test commands
  exact tool paths/versions
  artifacts and checksums
  compatibility results
  benchmark/security results
  known limitations
  remaining questions
  downstream consumers/actions
```

Code without a reproducible handoff is not considered complete.

## Conflict-resolution protocol

When two agents discover conflicting assumptions:

1. Stop changes that would encode the assumption into stable API/wire bytes.
2. Create or update an item in `18_OPEN_QUESTIONS_AND_ADR_QUEUE.md`.
3. Attach source evidence, fixture behavior, alternatives, compatibility/performance/security consequences, and recommendation.
4. The architecture owner records an ADR decision.
5. Update specs/tests first, then implementations.

## Ownership boundaries

- Format agents may not alter collector timing semantics.
- Collector agents may not redefine canonical events to fit an encoder.
- Report agents may not recompute with different timing/call semantics.
- Compatibility agents may not silently normalize semantic differences.
- Performance agents may not waive correctness.
- Security agents may impose blocking limits/fixes; usability thresholds then require architecture review.

# 02 - Devel::NYTProf 6.15 Current-State Baseline and Hotspots

## Purpose

Establish a source-grounded and reproducible baseline before changing collection, storage, parsing, APIs, or reports. This document defines the oracle artifacts and baseline tasks referenced throughout the plan.

## Current architecture summary

### Collector and writer

The profiler's hot path is already implemented in XS/C. `NYTProf.xs` owns profiler state, statement and call hooks, clock handling, process/lifecycle behavior, and profile loading. `FileHandle.xs` owns binary event writing, variable-length integers, string/native-NV serialization, buffering, compression, and finalization. `FileHandle.h` defines event tags and native structures.

The statement hook runs before each breakable statement and attributes elapsed time to the prior statement/block. It also participates in discount handling. Any new event boundary or buffer flush must preserve the exact placement and meaning of these clock/state updates.

### V5 storage

The current format is version 5.0 and is already binary/compressed rather than naive text. It uses compact positive-integer encoding and large raw buffering before zlib. Remaining representation costs include:

- absolute/repeated file, line, block, sub, and depth values;
- repeated subroutine/path strings in call and aggregate records;
- raw native Perl `NV` memory images for some timing fields;
- limited-width statement tick fields and legacy overflow behavior;
- repeated source across profiles/processes;
- a stream not framed for independent chunk validation/indexed recovery.

These are optimization opportunities, but the individual logical events and their order remain required.

### Reader and data model

The C loader can stream callbacks or materialize nested Perl arrays, hashes, scalar values, references, and objects. `Data.pm` then post-processes/collapses eval/file structures and blesses objects. The resulting object amplification is a likely source of report peak RSS.

### Reports

`Reader.pm` processes report levels/files largely serially, constructs per-line hashes and statistical arrays, and renders source through Perl callbacks. `nytprofhtml` orchestrates HTML plus calls/flame/graph paths; other scripts generate merge, calls, CSV, and Callgrind outputs.

### Build and support

The distribution uses ExtUtils::MakeMaker, detects zlib, and supports old Perl environments. Rust therefore begins as optional tooling/acceleration rather than an unconditional build dependency.

## Source touchpoint map

| Area | Primary files | Baseline questions |
|---|---|---|
| Format/version/event tags | `NYTProf.xs`, `FileHandle.h` | Every record, field, version guard, state dependency |
| Statement timing | `NYTProf.xs` statement hook | Clock positions, attribution, discount, overflow, flush/I/O treatment |
| Calls/sub timing | `NYTProf.xs`, `FileHandle.xs` | Entry/return/depth, inclusive/exclusive/recursive, abnormal exits |
| Writer/buffering/compression | `FileHandle.xs` | copies, calls, buffer sizes, zlib cost, finalization |
| Streaming API | `ReadStream.pm` | callback names/order/arguments/scalar flags, incomplete streams |
| Data object model | `NYTProf.xs`, `Data.pm`, related classes | shapes, identity, eval collapse, memory amplification |
| HTML/report logic | `Reader.pm`, `nytprofhtml` | statistics, ordering, file/anchor/link contract, serial hotspots |
| Auxiliary tools | `nytprofcalls`, `nytprofmerge`, `nytprofcg`, `nytprofcsv` | exact formats/options/exit behavior |
| Build/tests | `Makefile.PL`, `MANIFEST`, `HACKING`, `t/` | support tiers, fixture coverage, missing branches |

## Optimization hypotheses to measure, not assume

| Hypothesis | Candidate benefit | Compatibility risk | Required evidence |
|---|---|---|---|
| Canonical sink plus bounded batching | Fewer calls/copies/writes | Clock/discount drift | Same-run deterministic clock and v5 canonical equality |
| String dictionaries and deltas | Smaller profiles/faster compression | Identity/lifetime/state bugs | Exact expansion plus call/eval/Unicode/fork fixtures |
| Integer ticks in v6 | Portability, precision, less conversion | V5/API projection | Cross-platform vectors and strict representability |
| Independent chunks | Recovery, parallel decode, indexing | Missing dictionary/delta state | Corruption/truncation/property tests |
| Source content dedup | Smaller fork/merge profiles | Source/eval identity conflation | Exact bytes/flags/callback/report comparison |
| Rust compact model | Lower report RSS | Missing API/report fields | Canonical aggregates and object materializer tests |
| Native deterministic rendering | Faster reports | Numeric/DOM/link drift | Parsed value, DOM, auxiliary and visual comparisons |
| Different codec/default level | Faster collection/read or smaller files | Build/size regressions | Equivalent-event codec/chunk matrix |

## Baseline tasks

### BASE-001 - Pin and reproduce the 6.15 oracle

- **Status:** done
- **Size:** L
- **Dependencies:** none
- **Suggested owner:** release/build engineer
- **Goal:** Create an immutable reference implementation for every differential test.
- **Work:** Record CPAN/tarball and repository tag/commit checksums; build in clean reference environments; capture Perl `-V`, compiler, zlib, OS, build flags, executable/module paths, upstream test logs; script isolated rebuilds.
- **Deliverables:** `baseline/6.15/manifest.json`, verified source archive reference, build/test scripts and logs. **In-repo:** `scripts/baseline/*`, `baseline/6.15/` (see `baseline/6.15/README.md`).
- **Acceptance:** A second clean environment reproduces a passing oracle and the runner proves it is not loading candidate modules.

### BASE-002 - Inventory the complete v5 event protocol

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BASE-001
- **Suggested owner:** C/XS reverse-engineering and format engineer
- **Goal:** Document every tag, field, writer condition, parser state, callback, and consumer.
- **Work:** Trace all constants from `FileHandle.h`; record widths/signedness/order/native layout/compression; include shutdown aggregates, source/process/attribute/comment/end records; map option predicates and state dependencies; add minimal fixtures for each reachable record.
- **Deliverables:** normative `v5-record-inventory.md/json`, writer-reader-callback-report mapping.
- **Acceptance:** Every tag has a reader disposition and fixture or a reviewed unreachable explanation.

### BASE-003 - Freeze timing, call, numeric, and lifecycle semantics

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BASE-001, BASE-002
- **Suggested owner:** Perl-internals/C and numerical systems engineers
- **Goal:** State exactly what is measured and how all timing values/counts are formed.
- **Work:** Trace statement and call clock reads/state transitions; discount and profiler-overhead handling; inclusive/exclusive/recursive calculations; calls=0/1/2; leave/slow-op behavior; exceptions, `goto &sub`, recursion, XSUB, start/stop, fork/PID/finalization; integer/NV widths, endian/layout, overflow/anomaly behavior.
- **Deliverables:** statement/call/lifecycle sequence diagrams, numeric-layout table, deterministic test plans/vectors.
- **Acceptance:** Controlled executions predict every timing/call event and report-visible aggregate; no collector refactor begins without review.

### BASE-004 - Inventory Perl APIs and object-model behavior

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BASE-001
- **Suggested owner:** senior Perl API/test engineer
- **Goal:** Freeze documented and de facto public package behavior.
- **Work:** Enumerate modules/classes/methods/functions; context, coercion, return values, AV/HV/SV shapes, blessings, scalar flags, identity/aliasing/mutation, eval/file/sub relationships, warnings/errors, callbacks, lifetime; scan bundled and downstream consumers.
- **Deliverables:** API inventory, structural dumper, executable contract tests.
- **Acceptance:** Every native facade method/object field has a testable legacy expectation or explicit classification.

### BASE-005 - Inventory CLI, report, and auxiliary-output contracts

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BASE-001, BASE-004
- **Suggested owner:** CLI/report compatibility engineers
- **Goal:** Define parity for commands and generated artifacts.
- **Work:** Exercise every executable option/default/alias/env/precedence; capture stdout/stderr/exit/file effects; inventory report files, anchors, links, source views, calculations, sorting, templates/assets, calls/flame, DOT/graphs, CSV, Callgrind, merge; identify volatile fields and script-consumed behavior.
- **Deliverables:** CLI contract, report artifact/semantic contract, normalized golden archive.
- **Acceptance:** Every output is assigned byte, parsed-semantic, DOM, visual, or approved-variable comparison.

### BASE-006 - Build feature-to-test traceability matrix

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-002 through BASE-005
- **Suggested owner:** test architect
- **Goal:** Find untested compatibility surfaces before implementation.
- **Work:** Map upstream tests to events/options/APIs/commands/reports/platforms; identify writer/reader/report branches and missing edge cases; prioritize fixtures.
- **Deliverables:** `test/coverage-matrix.md/json` and gap backlog.
- **Acceptance:** Every compatibility-contract item maps to an existing or planned test owner.

### BASE-007 - Capture representative fixtures and performance baselines

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BASE-001 through BASE-006
- **Suggested owner:** fixture and performance engineers
- **Goal:** Provide realistic, redistributable profiles/workloads and oracle metrics.
- **Work:** Collect micro, scale, and real workloads across statement/call/source/fork patterns; archive scripts/inputs/options/profiles/reports/checksums; measure unprofiled/v5 collection, parse, aggregate, report wall/CPU/RSS/bytes; retain raw samples/environment.
- **Deliverables:** versioned profile/workload corpus and baseline benchmark database.
- **Acceptance:** Format/report decisions can be tested on measured event distributions and reference runners reproduce accepted noise bounds.

### BASE-008 - Quantify object/report memory and CPU amplification

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-004, BASE-005, BASE-007
- **Suggested owner:** report/performance engineer
- **Goal:** Attribute report time and retained memory to concrete phases/structures.
- **Work:** Profile decode/decompression, C-to-Perl materialization, Data post-processing/eval collapse, per-line hashes/statistics, templates/escaping, call/flame/graph, output I/O; measure object counts and bytes per event/line/sub.
- **Deliverables:** ranked hotspot and memory-amplification report with raw profiles.
- **Acceptance:** At least 90 percent of large-fixture CPU and retained RSS is assigned to named components or explicitly unaccounted tooling limits.

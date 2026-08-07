# Regression, Differential Testing, and Fixture Task Plan

## 1. Objective

Build a regression system strong enough to prove that optimization does not reduce precision or feature coverage. Independent executions are insufficient for exact timing comparison, so the system combines frozen v5 fixtures, legacy/native readers on the same file, and same-run dual writer output.

## 2. Test oracles

Use multiple independent oracles:

1. Unmodified Devel::NYTProf 6.15 binaries and tests.
2. Legacy C/Perl path retained in the new tree.
3. `Devel::NYTProf::ReadStream` canonical callback dump.
4. Same-run `format=dual` v5/v6 stream comparison.
5. Legacy Data object snapshots and normalized aggregate tables.
6. Legacy report normalized DOM/data/auxiliary outputs.
7. Independent C and Rust v6 decoder test vectors.

No single implementation validates itself.

## 3. Canonical event comparison

Canonicalization must normalize only values that are legitimately different:

- map process IDs to stable process-order IDs when comparing separate runs;
- map dictionary/file/sub IDs by first semantic definition;
- normalize absolute paths through an explicit fixture root mapping;
- normalize basetime/wall timestamps only when the test is not about them;
- preserve event order, count, ticks, locations, names/source bytes, flags, and option values;
- record every normalization applied.

Same-run dual comparison should require exact ticks and event order after format projection. Separate-run tests compare structure/invariants unless they replay a deterministic clock.

## 4. Deterministic clock test mode

Add a development-only clock backend that returns a scripted tick sequence. It enables:

- exact expected statement/call timing;
- overflow and negative/anomalous tick cases;
- deterministic separate runs through old/new collector logic;
- precise tests around buffer flush, fork, stop/start, and finalization.

It must never be enabled in release production builds by default.

## 5. Fixture matrix

### 5.1 Collection features

- statement and block timing;
- `calls=0`, `calls=1`, `calls=2`;
- slow-op on/off;
- `leave` correction modes;
- `stmts`, `blocks`, `savesrc`, compression levels/codecs;
- compile, INIT, runtime, END, explicit start/stop;
- file switching and append/overwrite where supported;
- timestamps/basetime and selected clocks;
- `addpid`, `forkdepth`, child/parent paths;
- normal exit, die/eval, signal exit, `_exit` limitations.

### 5.2 Perl behavior

- recursion and mutual recursion;
- `goto &sub`;
- AUTOLOAD/AutoSplit/AutoLoader;
- anonymous subs, closures, redefinition;
- string and file evals, nested evals;
- XSUBs and overloaded operators;
- ties, magic, destructors/global destruction;
- exceptions/non-local exits;
- `.pm`/`.pmc`, missing source, changed source;
- Unicode identifiers/source, invalid/non-UTF-8 bytes where supported;
- very large line numbers/files and many unique files/subs.

### 5.3 Storage extremes

- one repeated hot line millions of times;
- alternating near/far line deltas;
- many unique subroutine names;
- long names/paths/source lines;
- empty/tiny profiles;
- durations at every varint boundary;
- durations beyond v5 32-bit tick range;
- dictionary/chunk boundary at every event type;
- compressed/uncompressed and all supported codecs;
- truncated at every byte position for minimal files;
- bit flips in headers/chunks/checksums/varints/IDs.

### 5.4 Platform fixtures

Capture v5 files and expected interpretation from:

- 32-bit and 64-bit Perl where support runners exist;
- common little-endian platforms;
- big-endian fixture generation/emulation if available;
- multiple Perl `NV` sizes/layouts;
- multiple supported Perl versions;
- zlib variants and compiler families;
- Windows and Unix path/newline/file-system behavior where supported.

## 6. Report comparison levels

1. **Data model:** exact integer counts/ticks and identity relationships.
2. **Rendered values:** normalized text/numeric cells.
3. **DOM semantics:** nodes, IDs, anchors, links, classes/data attributes.
4. **Artifact manifest:** files, paths, MIME/type, relative references.
5. **Auxiliary formats:** parsed CSV, Callgrind, folded stacks, DOT.
6. **Visual:** screenshots after volatile normalization.
7. **Accessibility:** navigation and semantic checks.

Byte equality is used only for deliberately deterministic artifacts.

## 7. Regression tasks

### TEST-001 - Build unified oracle test harness

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BASE-001, COMPAT-001, COMPAT-002
- **Agent:** test architecture engineer
- **Work:** invoke oracle/new builds, collect artifacts, normalize, compare, isolate temp roots, record environment and backend.
- **Deliverables:** harness with machine-readable result bundle.
- **Acceptance:** one command runs selected matrix rows and reports first meaningful mismatch.

### TEST-002 - Define versioned canonical event schema

- **Status:** proposed
- **Size:** L
- **Dependencies:** COMPAT-001, TOOL-002
- **Agent:** format/test engineer
- **Work:** event fields, bytes/UTF-8, ticks, IDs, process streams, extensions, normalization metadata.
- **Deliverables:** schema, examples, parser/validator.
- **Acceptance:** legacy v5, Rust v5, and Rust v6 paths emit valid equivalent documents.

### TEST-003 - Build deterministic clock backend and scripts

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-003, COL-001
- **Agent:** C test instrumentation engineer
- **Work:** scripted clock values, exhaustion/error behavior, per-process scripts, debug-only build controls.
- **Deliverables:** backend and exact timing fixtures.
- **Acceptance:** repeated runs produce exact expected ticks; production build excludes accidental activation.

### TEST-004 - Import and stabilize upstream test suite

- **Status:** proposed
- **Size:** M
- **Dependencies:** BASE-001, TEST-001
- **Agent:** Perl test engineer
- **Work:** run unchanged first; classify environment-sensitive tests; archive expected behavior; add backend/format matrix without weakening assertions.
- **Deliverables:** CI integration and baseline report.
- **Acceptance:** M1 passes and every skip has reason/owner.

### TEST-005 - Create v5 golden profile corpus

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BASE-002 through BASE-007, TEST-001
- **Agent:** fixture engineer
- **Work:** generate minimal per-event and combined real profiles; store source scripts, build metadata, checksums, expected canonical stream/model/report.
- **Deliverables:** immutable versioned corpus.
- **Acceptance:** every v5 event/option/report feature maps to fixtures.

### TEST-006 - Add v5 reader differential suite

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-004, RUST-005, TEST-002, TEST-005
- **Agent:** Rust/Perl differential test engineer
- **Work:** compare callbacks, canonical events, errors, partial files, native NV layouts.
- **Deliverables:** CI suite and mismatch diagnostics.
- **Acceptance:** C2/M3 pass on all supported fixtures.

### TEST-007 - Add collector v5 compatibility suite

- **Status:** proposed
- **Size:** L
- **Dependencies:** COL-006, TEST-005
- **Agent:** C/XS test engineer
- **Work:** new collector `format=v5` -> unmodified 6.15 readers/tools; canonical and report comparisons.
- **Deliverables:** compatibility matrix job.
- **Acceptance:** M4 passes; no unsupported default option combination.

### TEST-008 - Add same-run dual writer suite

- **Status:** proposed
- **Size:** XL
- **Dependencies:** COL-014, TOOL-003, TEST-003, TEST-005
- **Agent:** compatibility test engineer
- **Work:** decode v5/v6 from same event source; compare exact order/ticks/bytes/metadata; force buffer/chunk/dictionary boundaries.
- **Deliverables:** M6 suite.
- **Acceptance:** zero unexplained differences across full feature matrix.

### TEST-009 - Build normalized report comparison framework

- **Status:** proposed
- **Size:** XL
- **Dependencies:** REPORT-001, TEST-001
- **Agent:** web/report test engineer
- **Work:** artifact manifests, HTML parser/DOM normalization, link checker, numeric extraction, asset hashing, volatile-field rules.
- **Deliverables:** report diff tool.
- **Acceptance:** detects seeded count/time/link/source changes while ignoring only approved whitespace/timestamps.

### TEST-010 - Add Data/API structural snapshot suite

- **Status:** proposed
- **Size:** L
- **Dependencies:** PERL-001, TEST-001
- **Agent:** Perl API test engineer
- **Work:** deep shapes, classes, scalar flags, identity, contexts, mutation, errors/warnings.
- **Deliverables:** normalized snapshots and direct assertions.
- **Acceptance:** legacy/native backend differences are localized at field/reference level.

### TEST-011 - Add CLI black-box contract suite

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-005, TOOL-010
- **Agent:** CLI test engineer
- **Work:** argv/env, help, aliases, stdout/stderr, exit codes, file effects, overwrite/error paths.
- **Deliverables:** portable CLI harness.
- **Acceptance:** compatibility wrappers meet frozen contract.

### TEST-012 - Add conversion round-trip suite

- **Status:** proposed
- **Size:** L
- **Dependencies:** TOOL-004, TOOL-005, TEST-002, TEST-005
- **Agent:** format test engineer
- **Work:** v5->v6, v5->v6->v5, v6->v5 representability, raw NV provenance, overflow errors.
- **Deliverables:** M7/M8 tests.
- **Acceptance:** representable semantics round-trip; all nonrepresentable cases fail explicitly.

### TEST-013 - Add mixed merge differential suite

- **Status:** proposed
- **Size:** L
- **Dependencies:** TOOL-009, TEST-005
- **Agent:** merge test engineer
- **Work:** input order, duplicate sources, ID collisions, processes/runs, mixed clocks/options, checked sums, v5/v6 output.
- **Deliverables:** M9 tests.
- **Acceptance:** deterministic merged model and report parity under defined legacy behavior.

### TEST-014 - Add corruption/truncation recovery matrix

- **Status:** proposed
- **Size:** XL
- **Dependencies:** TOOL-007, FMT-010, SEC-003
- **Agent:** reliability test engineer
- **Work:** truncate/flip/duplicate/remove/reorder chunks; malformed varints/lengths/dictionaries; incomplete v5; salvage manifests.
- **Deliverables:** generated corpus and expected outcomes.
- **Acceptance:** no crash/UB/unbounded allocation; valid recovered ranges are exact and clearly incomplete.

### TEST-015 - Add property-based event-stream round trips

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-011, TEST-002, and the writer selected by COL-009
- **Agent:** property test engineer
- **Work:** generate state-valid and state-invalid streams, encode/decode, dictionary/delta/run boundaries, process streams.
- **Deliverables:** property suites in C/Rust where relevant.
- **Acceptance:** valid streams round-trip canonically; invalid streams fail in specified category.

### TEST-016 - Add parser/FFI fuzzing program

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-018, PERL-011, SEC-002
- **Agent:** security fuzz engineer
- **Work:** v5/v6 bytes, decompressors, converter, merge, C ABI handles, callback exceptions.
- **Deliverables:** fuzz targets, seed corpus, continuous/nightly jobs.
- **Acceptance:** budget passes with no crash, UB, leak trend, or limit bypass.

### TEST-017 - Add cross-platform v5 NV/endian corpus

- **Status:** proposed
- **Size:** XL
- **Dependencies:** RUST-005, BUILD-006
- **Agent:** portability engineer
- **Work:** generate profiles on real/emulated platforms; record Perl config and expected numeric interpretation; test fallback.
- **Deliverables:** licensed/redistributable binary fixtures and metadata.
- **Acceptance:** each supported layout has positive tests; each unsupported layout has deterministic diagnostic/fallback test.

### TEST-018 - Add fork/process stress suite

- **Status:** proposed
- **Size:** XL
- **Dependencies:** COL-015, TEST-003
- **Agent:** process/concurrency test engineer
- **Work:** nested forks, high event rate around fork, buffer near full, parent/child errors, addpid/forkdepth, merge.
- **Deliverables:** stress programs and race/fd checks.
- **Acceptance:** exact per-process streams, valid files, no deadlock/shared compressor corruption.

### TEST-019 - Add long-duration/overflow suite

- **Status:** proposed
- **Size:** L
- **Dependencies:** COMPAT-003, COL-011, RUST-007, RUST-008, TEST-003
- **Agent:** numerical test engineer
- **Work:** all varint/tick/count boundaries, v5 overflow behavior, v6 wide values, checked aggregate overflow, conversion diagnostics.
- **Deliverables:** boundary vectors.
- **Acceptance:** v6 retains values exactly; v5 path matches frozen behavior; conversion never silently clamps.

### TEST-020 - Add release compatibility matrix runner

- **Status:** proposed
- **Size:** L
- **Dependencies:** TEST-004 through TEST-019, BUILD-006
- **Agent:** release test engineer
- **Work:** execute M1-M10 across supported Perl/platform/backend/format combinations; aggregate evidence and artifacts.
- **Deliverables:** signed/versioned release gate report.
- **Acceptance:** release cannot promote default modes unless required rows pass with zero unexplained failures.

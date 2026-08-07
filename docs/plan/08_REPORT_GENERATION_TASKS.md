# Report Generation Architecture and Task Plan

## 1. Objective

Reimplement report generation around the compact native model so that reports are faster and use less memory while retaining all data, calculations, files, links, and user-visible capabilities of the 6.15 tools.

The first native report milestone consumes v5 profiles. This isolates report gains from collector/format changes and lets the existing output act as the oracle.

## 2. Target report pipeline

```text
v5/v6 profile
   -> bounded decoder
   -> exact streaming aggregation
   -> compact profile model
   -> deterministic report IR
   -> parallel independent render jobs
   -> temporary output tree
   -> validation manifest
   -> atomic publish
```

The report intermediate representation (IR) separates NYTProf calculations from HTML/text formatting. All renderers consume the same checked values.

## 3. Report IR requirements

The IR must represent:

- run/profile metadata and options;
- global totals and rankings;
- file/eval hierarchy and source availability;
- line, block, and subroutine metrics;
- median, MAD, percentages, and legacy display values;
- sub definitions and caller/callee/call-site data;
- exact folded stacks/flame inputs when requested;
- graph nodes/edges and external-tool inputs;
- warnings, incomplete-profile notices, and source mismatch notices;
- stable URLs, filenames, anchors, and navigation relations;
- asset manifest and renderer version.

Numbers remain integer ticks/counts in the IR when possible. Renderer-specific formatting converts them to seconds/percent strings using compatibility-tested rules.

## 4. Rendering modes

### 4.1 Compatibility mode

The initial/default native renderer reproduces the 6.15 artifact contract:

- same output file names and directories;
- same expected anchors and relative links;
- same report levels and optional artifacts;
- same data values and sort/tie behavior;
- semantically equivalent HTML/DOM and browser behavior.

### 4.2 Compact mode

After compatibility mode passes, an optional compact mode may reduce report-tree size through:

- shared immutable data/assets;
- content-addressed source payloads;
- less repeated inline markup/data;
- optional client-side hydration;
- optional compressed static assets where consumers support them.

Compact mode must retain every report feature and offline usability. It cannot become the default until browser, accessibility, script-consumer, and backward-link compatibility are approved by ADR.

## 5. Deterministic parallelism

Parallel work units may include:

- file-level source reports;
- block/sub-level variants;
- independent tables;
- call/flame aggregation after event replay;
- graph-input files;
- asset generation.

Rules:

- all visible sorting is deterministic with explicit tie breaks;
- output filenames are allocated before workers start;
- workers write unique temporary files;
- a coordinator writes the manifest and atomically publishes output;
- worker count does not alter numeric reduction order or output content;
- external tools are invoked in deterministic input order and failures are captured consistently.

## 6. Exact statistics

The old report computes statistics such as median/MAD and per-line summaries. Native implementations must freeze:

- input population and treatment of zero/unexecuted lines;
- integer-to-floating conversion point;
- sorting/tie behavior;
- odd/even median rule;
- MAD definition and scaling;
- percentage denominator;
- display precision and rounding;
- overflow/NaN/Inf behavior inherited from v5 data.

Use integer or rational arithmetic where practical, but output must remain compatible.

## 7. Source rendering and safety

Source is arbitrary profile-provided content. Renderers must:

- preserve exact visible source text and line mapping;
- honor Perl UTF-8 semantics while safely escaping HTML;
- avoid executing embedded source as markup/script;
- handle missing/mismatched source and evals as legacy reports do;
- avoid path traversal when constructing output names;
- not load current filesystem source unless the selected legacy behavior permits it.

## 8. Calls, flame, CSV, and Callgrind

All auxiliary outputs derive from the same native model/event stream:

- `nytprofcalls`: preserve exact call-event sequence semantics and output modes;
- flame data: preserve stack identity, counts/times, recursion, and escaping;
- Callgrind: preserve function/file/call-site mapping and event units;
- CSV: preserve columns, order, quoting, units, and precision;
- graph input/output: preserve node/edge values and options; retain external Graphviz compatibility initially.

No call stream may be discarded merely because HTML generation does not need it when the user requested a call-based output.

## 9. Report tasks

### REPORT-001 - Freeze the report artifact and semantic contract

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BASE-005, COMPAT-004, COMPAT-006
- **Agent:** reporting/test architect
- **Work:** catalog every file, anchor, link, table, statistic, template, callback, asset, optional output, error, and variable field.
- **Deliverables:** report contract and normalized golden corpus.
- **Acceptance:** each artifact has a comparison method and owner.

### REPORT-002 - Define deterministic report IR

- **Status:** proposed
- **Size:** L
- **Dependencies:** ARCH-004, RUST-006 through RUST-009, REPORT-001
- **Agent:** Rust/report architect
- **Work:** typed IR for index/file/sub/block/call/graph/assets; stable ordering and formatting inputs.
- **Deliverables:** schema and Rust types.
- **Acceptance:** all legacy report values/artifacts are representable without consulting Perl objects.

### REPORT-003 - Reimplement legacy numeric/statistical calculations

- **Status:** proposed
- **Size:** L
- **Dependencies:** REPORT-001, REPORT-002, BASE-003
- **Agent:** numerical/test engineer
- **Work:** totals, percentages, ranks, median, MAD, display conversion, special values.
- **Deliverables:** calculation library and boundary vectors.
- **Acceptance:** normalized values/strings match oracle across all fixtures and platform-specific policies.

### REPORT-004 - Implement native index/summary pages

- **Status:** proposed
- **Size:** L
- **Dependencies:** REPORT-002, REPORT-003
- **Agent:** Rust HTML engineer
- **Work:** profile summary, file/sub rankings, navigation, warnings, metadata, assets.
- **Deliverables:** first opt-in native HTML output.
- **Acceptance:** semantic DOM/value/link comparison passes for baseline corpus.

### REPORT-005 - Implement source line pages

- **Status:** proposed
- **Size:** XL
- **Dependencies:** REPORT-002, REPORT-003, RUST-009
- **Agent:** source-rendering engineer
- **Work:** exact source mapping, timing/count cells, annotations, tooltips, escaping, missing source, evals.
- **Deliverables:** file-level pages and fixtures.
- **Acceptance:** line-to-source and metric parity; XSS/path tests pass.

### REPORT-006 - Implement block and sub-level pages

- **Status:** proposed
- **Size:** L
- **Dependencies:** REPORT-005, RUST-007, RUST-008
- **Agent:** reporting engineer
- **Work:** reproduce report levels, expansion/navigation, block/sub attribution and statistics.
- **Deliverables:** pages and comparison tests.
- **Acceptance:** artifact set and all values match oracle in compatibility mode.

### REPORT-007 - Implement calls and folded-stack generation

- **Status:** proposed
- **Size:** XL
- **Dependencies:** RUST-008, RUST-015, REPORT-002
- **Agent:** call-stack algorithms engineer
- **Work:** stream/retain call events as needed, stack reconstruction, recursion, escaping, output filters/order.
- **Deliverables:** native equivalent of `nytprofcalls` data generation.
- **Acceptance:** normalized output exactly matches oracle for calls=1/2 fixtures.

### REPORT-008 - Preserve flame graph integration

- **Status:** proposed
- **Size:** L
- **Dependencies:** REPORT-007, BASE-005
- **Agent:** reporting/tool integration engineer
- **Work:** preserve bundled/external script invocation, options, input format, output filenames, errors; optionally add native SVG only behind separate equivalence gate.
- **Deliverables:** flame output path and tests.
- **Acceptance:** same folded input and semantically/visually equivalent output under pinned tool version.

### REPORT-009 - Implement graph data and Graphviz integration

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-008, REPORT-002
- **Agent:** graph/report engineer
- **Work:** nodes, edges, weights, labels, filtering, dot generation/invocation, failure behavior.
- **Deliverables:** graph artifacts and normalized DOT comparisons.
- **Acceptance:** graph metrics and links match; missing Graphviz behavior is compatible.

### REPORT-010 - Implement deterministic parallel render scheduler

- **Status:** proposed
- **Size:** L
- **Dependencies:** REPORT-004 through REPORT-009, RUST-017
- **Agent:** Rust concurrency engineer
- **Work:** DAG/jobs, fixed output allocation, worker pool, cancellation, ordered manifest, 1-worker fallback.
- **Deliverables:** scheduler and telemetry.
- **Acceptance:** 1..N workers produce normalized-identical output; failures do not publish partial trees.

### REPORT-011 - Implement atomic output publication

- **Status:** proposed
- **Size:** M
- **Dependencies:** REPORT-010, TOOL-001
- **Agent:** filesystem/reliability engineer
- **Work:** temporary directory, overwrite rules, fsync policy where applicable, rename/cross-device handling, cleanup.
- **Deliverables:** publisher and failure tests.
- **Acceptance:** interrupted or failed reports leave prior valid output intact according to documented platform behavior.

### REPORT-012 - Implement native CSV renderer

- **Status:** proposed
- **Size:** M
- **Dependencies:** REPORT-002, REPORT-003, BASE-005
- **Agent:** data-export engineer
- **Work:** columns, quoting, encoding, ordering, numeric units, options.
- **Deliverables:** CSV backend.
- **Acceptance:** parsed rows/cells match oracle; byte comparison where deterministic.

### REPORT-013 - Implement native Callgrind renderer

- **Status:** proposed
- **Size:** L
- **Dependencies:** REPORT-002, RUST-008, BASE-005
- **Agent:** profiling-format engineer
- **Work:** file/function/call-site mappings, events, positions, totals, escaping, options.
- **Deliverables:** Callgrind backend and parser-based comparison.
- **Acceptance:** normalized Callgrind model matches oracle and loads in representative tools.

### REPORT-014 - Implement renderer compatibility callbacks/fallback

- **Status:** proposed
- **Size:** L
- **Dependencies:** PERL-008, REPORT-004 through REPORT-006
- **Agent:** Perl/Rust report integration engineer
- **Work:** identify callbacks that can be batch-fed, those needing legacy path, and explicit capability diagnostics.
- **Deliverables:** bridge/fallback implementation.
- **Acceptance:** no supported callback is ignored; forced native fails clearly when unsupported rather than changing output.

### REPORT-015 - Reduce compatibility-mode output duplication

- **Status:** proposed
- **Size:** M
- **Dependencies:** REPORT-001, REPORT-004 through REPORT-006, BENCH-009
- **Agent:** web/performance engineer
- **Work:** measure repeated assets/markup/data; apply only DOM/link-compatible dedup such as shared assets, whitespace policy, and stable generated fragments.
- **Deliverables:** size report and safe optimizations.
- **Acceptance:** report semantic/visual/script-consumer tests pass; output size improves or task closes with evidence that compatibility prevents it.

### REPORT-016 - Design optional compact report mode

- **Status:** proposed
- **Size:** L
- **Dependencies:** REPORT-004 through REPORT-015, ADR report policy
- **Agent:** web architecture engineer
- **Work:** shared data store, content-addressed source, offline loading, deep links, accessibility, browser matrix, migration.
- **Deliverables:** prototype and ADR.
- **Acceptance:** full feature parity, exact values, no server requirement unless explicitly selected, documented compatibility limits.

### REPORT-017 - Add report phase/cache invalidation

- **Status:** proposed
- **Size:** M
- **Dependencies:** RUST-014, REPORT-002, TOOL-008
- **Agent:** caching/data-integrity engineer
- **Work:** optional sidecar/IR cache keyed by profile content, engine/schema/options/assets; exact invalidation and verification.
- **Deliverables:** cache format and controls.
- **Acceptance:** cached and uncached outputs are identical; corrupt/stale cache is ignored safely.

### REPORT-018 - Add report telemetry and hotspot regression

- **Status:** proposed
- **Size:** M
- **Dependencies:** RUST-016, REPORT-010
- **Agent:** performance engineer
- **Work:** time parse/aggregate/IR/render/external tools; track files, bytes, workers, cache, peak model sizes.
- **Deliverables:** optional JSON metrics and benchmark integration.
- **Acceptance:** disabled output remains unchanged and overhead is negligible.

### REPORT-019 - Add visual and accessibility regression suite

- **Status:** proposed
- **Size:** L
- **Dependencies:** REPORT-004 through REPORT-016, TEST-009
- **Agent:** browser/test engineer
- **Work:** pinned browser screenshots, keyboard navigation, ARIA/semantic checks, responsive layout, large-line behavior.
- **Deliverables:** visual baselines and accessibility report.
- **Acceptance:** approved thresholds pass; semantic value tests remain authoritative.

### REPORT-020 - Make native report engine independently selectable

- **Status:** proposed
- **Size:** M
- **Dependencies:** REPORT-004 through REPORT-014, PERL-002, TOOL-001
- **Agent:** integration engineer
- **Work:** `--engine`/configuration path, version manifest, legacy fallback, diagnostics.
- **Deliverables:** opt-in native report release path.
- **Acceptance:** same v5 profile can be rendered by both engines in one test job and compared automatically.

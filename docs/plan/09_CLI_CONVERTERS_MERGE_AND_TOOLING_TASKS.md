# CLI, Conversion, Merge, Inspection, and Tooling Task Plan

## 1. Objective

Provide stable command-line compatibility and new first-class tools for inspecting, validating, comparing, converting, and merging v5/v6 profiles. These tools are central to regression testing and backward compatibility, not optional utilities.

## 2. Command strategy

Existing executable names remain available:

```text
nytprofhtml
nytprofcalls
nytprofmerge
nytprofcg
nytprofcsv
```

New functionality may be exposed through a unified native executable plus thin wrappers, for example:

```text
nytprof inspect
nytprof verify
nytprof dump-events
nytprof compare
nytprof convert
nytprof merge
nytprof html
nytprof calls
nytprof cg
nytprof csv
```

Wrapper behavior, option parsing, exit codes, and output paths must match the existing tools. The internal command layout is not user-visible unless explicitly invoked.

## 3. Canonical semantic stream

The canonical dump is the primary cross-format oracle. Recommended forms:

- canonical NDJSON for human review and diffs;
- canonical CBOR or a compact binary form for large automated comparisons;
- stable string/source table remapping;
- explicit event sequence, process stream, event type, field units, bytes/UTF-8 representation;
- explicit normalization manifest for volatile values;
- raw provenance fields only when requested.

The canonical form is a test protocol, not necessarily a public long-term storage format. Version it nevertheless.

## 4. Conversion guarantees

### v5 to v6

- preserve ordered events and metadata;
- retain raw v5 numeric provenance when needed for strict round-trip;
- record producer/converter versions and source checksum;
- never infer unsupported native `NV` layout;
- optionally add exact derived indexes after raw conversion.

### v6 to v5

- default strict mode errors on unrepresentable values/features;
- diagnostics identify event sequence, field, value/range, and possible remediation;
- user-requested lossy mode, if implemented at all, is explicitly named and never used by compatibility tests;
- output must be readable by unmodified 6.15 tools for representable input.

## 5. Merge guarantees

- accept mixed v5 and v6 inputs;
- preserve independent run/process boundaries and metadata;
- remap IDs deterministically;
- deduplicate source/name content without conflating distinct semantic identity;
- checked arithmetic with explicit overflow errors;
- deterministic output given the same ordered input list/options;
- support v5 output only when the merged result is v5-representable;
- preserve existing `nytprofmerge` behavior in compatibility mode.

## 6. Validation and recovery

`verify` should support levels:

```text
header      - identify format/features and validate header
chunks      - validate framing, lengths, codecs, checksums, sequence
stream      - decode all logical events and validate state/IDs
semantic    - build aggregates and cross-check optional summaries
strict      - enforce canonical encodings and all invariants
```

`salvage` is a separate explicit operation. It copies only complete verified units and reports missing/corrupt ranges. It never pretends the result is a complete profile.

## 7. Tooling tasks

### TOOL-001 - Build unified native CLI framework

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-001, RUST-012, BASE-005
- **Agent:** Rust CLI engineer
- **Work:** subcommands, shared I/O/options/errors/logging, machine-readable diagnostics, version/capabilities.
- **Deliverables:** `nytprof` native executable.
- **Acceptance:** portable invocation, deterministic exit classes, no change to existing wrappers yet.

### TOOL-002 - Implement canonical event dump

- **Status:** proposed
- **Size:** L
- **Dependencies:** COMPAT-001, COMPAT-002, RUST-004, RUST-011
- **Agent:** compatibility tooling engineer
- **Work:** stable ID remapping, bytes/UTF-8 encoding, tick units, process sequences, normalization manifest, NDJSON/compact modes.
- **Deliverables:** schema, command, parser, golden outputs.
- **Acceptance:** v5 and same-run v6 dumps compare exactly after approved normalization.

### TOOL-003 - Implement semantic profile comparator

- **Status:** proposed
- **Size:** L
- **Dependencies:** TOOL-002
- **Agent:** test tooling engineer
- **Work:** streaming first-mismatch, context windows, field-aware diagnostics, optional aggregate/report checks, exit codes.
- **Deliverables:** `nytprof compare`.
- **Acceptance:** injected differences identify exact event sequence/field; large files compare with bounded memory.

### TOOL-004 - Implement v5 to v6 converter

- **Status:** proposed
- **Size:** XL
- **Dependencies:** RUST-004, RUST-005, RUST-011, FMT-013
- **Agent:** format compatibility engineer
- **Work:** stream events, preserve numeric provenance/options/source/order, write conversion manifest/checksum.
- **Deliverables:** converter and round-trip fixtures.
- **Acceptance:** canonical v5 input equals converted v6; unsupported layouts fail/fallback explicitly.
- **Regression gate:** M7.

### TOOL-005 - Implement strict v6 to v5 converter

- **Status:** proposed
- **Size:** XL
- **Dependencies:** RUST-011, COL-006 or Rust v5 encoder, FMT-013
- **Agent:** format compatibility engineer
- **Work:** representability analysis, v5 field projection, NV target description, compression/options, detailed errors.
- **Deliverables:** converter and compatibility report.
- **Acceptance:** unmodified 6.15 tools consume every representable converted fixture; no silent truncation.
- **Regression gate:** M8.

### TOOL-006 - Implement format inspector

- **Status:** proposed
- **Size:** M
- **Dependencies:** RUST-004, RUST-011, FMT-014
- **Agent:** tooling engineer
- **Work:** metadata, Perl/platform layout, options, codecs, chunks, event counts, source/dictionary stats, completion state.
- **Deliverables:** human and JSON output.
- **Acceptance:** works without full model construction where possible and handles corrupt metadata safely.

### TOOL-007 - Implement validator and salvage command

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-003, RUST-004, RUST-011, SEC-003
- **Agent:** reliability tooling engineer
- **Work:** verification levels, limits, checksum/state validation, complete-chunk salvage, incomplete marker/manifest.
- **Deliverables:** `verify` and `salvage`.
- **Acceptance:** corruption/truncation matrix yields deterministic result and never returns unverifiable events as valid.

### TOOL-008 - Implement optional index/summary builder

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-011, RUST-014
- **Agent:** data tooling engineer
- **Work:** append or sidecar modes, source profile hash, exact aggregate generation, atomic update.
- **Deliverables:** `index` command and cache schema.
- **Acceptance:** indexed/unstyled raw reports are identical; stale index ignored.

### TOOL-009 - Implement mixed v5/v6 merge command

- **Status:** proposed
- **Size:** XL
- **Dependencies:** RUST-013, BASE-005
- **Agent:** merge/data engineer
- **Work:** preserve legacy options/output, deterministic ID remap, metadata/source reconciliation, output format selection.
- **Deliverables:** native merge and compatibility wrapper.
- **Acceptance:** normalized merged model/report matches legacy for v5 inputs; mixed matrix passes.
- **Regression gate:** M9.

### TOOL-010 - Add existing CLI wrappers over native engine

- **Status:** proposed
- **Size:** L
- **Dependencies:** TOOL-001, REPORT-020, TOOL-009, REPORT-007, REPORT-012, REPORT-013
- **Agent:** Perl/Rust CLI integration engineer
- **Work:** preserve executable names, argv, help, environment, exit codes, stdout/stderr, default paths; allow `--engine`.
- **Deliverables:** wrappers and black-box tests.
- **Acceptance:** existing CLI contract passes in legacy mode and native-supported cases.

### TOOL-011 - Preserve `nytprofcalls` modes and streaming behavior

- **Status:** proposed
- **Size:** L
- **Dependencies:** REPORT-007, TOOL-010
- **Agent:** call tooling engineer
- **Work:** all filters, outputs, stack formatting, progress/errors, large-profile streaming.
- **Deliverables:** native command/wrapper.
- **Acceptance:** canonical folded/call output matches legacy and uses bounded memory where configured.

### TOOL-012 - Preserve `nytprofcg` and `nytprofcsv`

- **Status:** proposed
- **Size:** M
- **Dependencies:** REPORT-012, REPORT-013, TOOL-010
- **Agent:** export tooling engineer
- **Work:** option parity, stdout/file behavior, units, paths, errors.
- **Deliverables:** wrappers/native subcommands.
- **Acceptance:** parser-based output comparison passes.

### TOOL-013 - Add profile feature/capability negotiation command

- **Status:** proposed
- **Size:** S
- **Dependencies:** ARCH-006, TOOL-006
- **Agent:** tooling engineer
- **Work:** report whether installed engine can read/render/convert a profile and why not.
- **Deliverables:** `capabilities` output.
- **Acceptance:** packaging/fallback tests can make decisions without attempting a destructive operation.

### TOOL-014 - Add converter provenance and reproducibility manifest

- **Status:** proposed
- **Size:** M
- **Dependencies:** TOOL-004, TOOL-005
- **Agent:** release/data integrity engineer
- **Work:** input checksums, producer/converter versions, target NV description, codecs/features, warnings, normalized options.
- **Deliverables:** embedded or sidecar manifest.
- **Acceptance:** a converted fixture can be traced and reproduced; manifests never alter canonical events.

### TOOL-015 - Add tool-level resource limits

- **Status:** proposed
- **Size:** M
- **Dependencies:** SEC-001, TOOL-001
- **Agent:** security/CLI engineer
- **Work:** maximum uncompressed bytes, events, strings, source, recursion/depth, workers, output files; safe defaults and overrides.
- **Deliverables:** common limit configuration.
- **Acceptance:** malicious fixtures fail before resource exhaustion with precise errors.

### TOOL-016 - Add machine-readable exit/error taxonomy

- **Status:** proposed
- **Size:** S
- **Dependencies:** TOOL-001, PERL-003
- **Agent:** CLI API engineer
- **Work:** categories for unsupported, corrupt, incomplete, unrepresentable, I/O, resource limit, external tool, internal.
- **Deliverables:** documented exit codes/JSON error schema.
- **Acceptance:** wrappers preserve legacy codes where required and expose richer native diagnostics without ambiguity.

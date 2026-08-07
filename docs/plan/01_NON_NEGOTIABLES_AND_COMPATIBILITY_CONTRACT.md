# 01 - Non-Negotiables and Compatibility Contract

## Purpose

This document is the binding contract for design reviews, agent work, and release gates. An implementation that violates it must not merge without an approved architecture decision record (ADR) and explicit project-owner sign-off. The default disposition is rejection.

## A. Precision and event fidelity

### A1. Full event stream

The v6 profile must contain enough information to reconstruct, in order, every event visible through the v5 `Devel::NYTProf::ReadStream` interface, including all event arguments and process boundaries.

Required event categories include, at minimum:

- statement timing;
- block timing;
- discount markers;
- subroutine entry and return events for every supported `calls` mode;
- subroutine metadata;
- caller/callee aggregate records present in v5;
- file-ID definitions and file metadata;
- source lines and source association;
- process/PID/fork markers;
- attributes, comments, and end markers;
- slow-op or other option-dependent records supported by 6.15.

Unknown legacy records must either be preserved as opaque extension events or cause an explicit unsupported-record error. They must not be silently discarded.

### A2. Exact multiplicity and order

Encoding techniques may compress repeated values, but decoding must restore the exact number and order of events. A run-length record is permitted only when it is a reversible representation of identical consecutive event structure and retains each event's distinct timing value where timings differ.

### A3. Exact timing semantics

The collector must preserve the existing definition of:

- which statement an elapsed interval is attributed to;
- when profiler overhead is excluded;
- how discount records affect execution counts and time totals;
- inclusive, exclusive, and recursive subroutine timing;
- clock selection and tick frequency;
- behavior around exceptions, non-local exits, recursion, XSUBs, and process boundaries.

The v6 format should store native profiler ticks as integers wherever the collector has them. Display/API conversion to seconds is performed at the boundary that historically returns floating-point values.

### A4. No precision-reducing modes in project scope

The following are explicitly out of scope for this modernization:

- statistical sampling as a replacement for exact instrumentation;
- disabling statement, call, source, block, or slow-op features to claim improvement;
- replacing ordered events with only line/subroutine aggregates;
- quantizing, rounding, truncating, or bucketing timing values;
- omitting source bytes that the selected `savesrc` mode would currently retain;
- reducing call-stack depth or merging distinct call sites.

Existing user-selected options such as `calls=0`, `stmts=0`, or `savesrc=0` remain supported, but benchmark claims must compare equivalent configurations.

## B. File-format compatibility

### B1. v5 read support

The new engine must read the supported v5 format, including:

- compressed and uncompressed profiles;
- profiles with different pointer/integer/NV-size metadata;
- all event types emitted by 6.15;
- valid profiles with no optional end marker when legacy recovery allows them;
- profiles produced by forks or merged tools;
- source content with arbitrary bytes accepted by the legacy format.

### B2. v5 write support

The collector must keep a `format=v5` mode whose output is accepted by unmodified 6.15 readers and tools. During transition, this path should reuse the legacy writer unless and until an independently verified replacement is proven compatible.

### B3. v6 read/write support

The new engine reads and writes v6. The v6 decoder must accept future minor versions when all required features are understood and skip optional sections marked as skippable.

### B4. Conversion support

Required conversion paths:

```text
v5 -> v6
v6 -> v5
v5 -> canonical semantic stream
v6 -> canonical semantic stream
```

A v6-to-v5 conversion must fail loudly or emit an explicit compatibility warning when a v6 value cannot be represented in v5 without loss, such as a tick interval exceeding the v5 field limit. It must never silently clamp or discard data.

For v5-to-v6-to-v5 regression, opaque legacy numeric payloads may be retained in v6 provenance sections so the converter can reproduce the original v5 numeric bit pattern when requested.

### B5. Same-run dual output

A test/developer mode must capture each event once and send the same canonical event to both writers. The mode must include a monotonic event sequence number in diagnostic/canonical output so semantic mismatches can be localized exactly.

## C. Perl API compatibility

### C1. Public packages and methods

Create an inventory of all public or de facto public packages, constructors, methods, return values, exception texts, warnings, and object types in 6.15. At minimum include:

- `Devel::NYTProf`;
- `Devel::NYTProf::Data` and related data objects;
- `Devel::NYTProf::ReadStream`;
- `Devel::NYTProf::Reader`;
- file, subroutine, line, block, and call data objects exposed by the distribution;
- exported constants or utility entry points used by bundled tools.

No API is considered private merely because it lacks complete documentation if bundled tools or existing tests use it.

### C2. Observable compatibility

Tests must cover:

- scalar/list/void-context behavior where relevant;
- object class names and inheritance;
- key names and array positions in returned structures;
- numeric units and rounding;
- object identity and stable references;
- ordering of hashes where output depends on sorting logic;
- warnings, `croak`/`die` behavior, and exit status;
- callback order and callback argument types;
- mutation behavior if callers can alter returned structures.

### C3. Engine fallback

Until the native engine has completed release gates, users must be able to force the legacy path. `auto` must fail over only for documented capability/platform reasons, not silently after a data-corruption or semantic error.

## D. CLI compatibility

The following must be inventoried and preserved:

- executable names;
- option names, aliases, defaults, and precedence;
- environment-variable behavior;
- positional arguments;
- stdout/stderr format where scripts reasonably parse it;
- exit codes;
- report directory and file naming;
- overwrite behavior;
- missing/corrupt-input behavior;
- tool interoperability.

New options must not change legacy defaults during the compatibility period.

## E. Report compatibility

### E1. Semantic parity

For the same canonical event stream and configuration, old and new reports must agree on:

- statement counts and times;
- block-level counts and times;
- subroutine inclusive/exclusive/recursive times and calls;
- caller/callee relationships and call sites;
- totals, percentages, ranks, medians, MAD or other statistics;
- source-line association;
- flame/call-stack samples derived from exact call events;
- Callgrind and CSV values;
- merged-profile totals.

### E2. Navigation and artifact parity

Default compatibility rendering must preserve the expected artifact set, names, relative links, anchors, and navigation behavior. Visual styling may be refactored only if semantic DOM and visual-regression gates pass.

### E3. Determinism

Given the same profile, tool version, locale, timezone controls, and options, report data and ordering must be deterministic. Timestamps or version strings that are intentionally variable must be isolated for normalization.

## F. Platform and build compatibility

The existing distribution supports Perl installations that may not have Rust. Therefore:

- Rust must be optional during initial releases;
- legacy collection and report functionality must remain installable without Cargo on supported legacy platforms;
- a native engine may be built from source or supplied as a verified platform artifact where policy permits;
- failure to build the Rust accelerator must not produce a partially broken install unless the user explicitly requires it.

A later change to minimum Perl, compiler, or operating-system support requires a separate compatibility ADR and release policy.

## G. Regression comparison matrix

Every release candidate must pass this matrix:

| ID | Producer | Format | Consumer | Required result |
|---|---|---|---|---|
| M1 | 6.15 collector | v5 | 6.15 tools | Baseline remains green |
| M2 | 6.15 collector | v5 | New legacy engine | Same as baseline |
| M3 | 6.15 collector | v5 | New native engine | Canonical semantic equality |
| M4 | New collector | v5 | Unmodified 6.15 tools | Accepted and semantically equal |
| M5 | New collector | v6 | New native engine | Canonical semantic equality to same-run v5 |
| M6 | New collector dual mode | v5 and v6 | Canonical comparer | Event-for-event equality |
| M7 | v5 converter | v6 | New native engine | Equality to original v5 canonical stream |
| M8 | v6 converter | v5 | 6.15 tools | Equality within explicit v5 representability limits |
| M9 | Mixed v5/v6 inputs | merged output | New and legacy-compatible reports | Equal merged metrics |
| M10 | Truncated/corrupt inputs | v5/v6 | Validators/readers | Documented recovery or deterministic failure |

## H. Change-control rule

Any task that discovers an incompatible legacy behavior must:

1. Add a fixture reproducing it.
2. Record whether it is documented, tested, or accidental.
3. Propose one of: preserve, preserve behind compatibility mode, or intentionally change.
4. Obtain an ADR before modifying behavior.
5. Add migration and release-note text for any approved change.

## I. Compatibility governance tasks

### COMPAT-000 - Ratify the compatibility contract

- **Status:** done
- **Size:** S
- **Dependencies:** none
- **Suggested owner:** project maintainer/architect
- **Goal:** Make this document binding for implementation and review.
- **Work:** Review each non-negotiable, record any explicitly approved exception, assign compatibility/security/performance approvers, and define the ADR escalation path.
- **Deliverables:** approved contract revision, reviewer list, issue labels/checklists. **In-repo ratification:** `docs/governance/COMPAT-000_RATIFICATION.md`.
- **Acceptance:** No implementation phase that changes semantics begins without recorded approval or an explicitly provisional research status.

### COMPAT-001 - Define the canonical logical-event contract

- **Status:** in-progress (provisional taxonomy landed 2026-08-07)
- **Size:** XL
- **Dependencies:** BASE-001, BASE-002, BASE-003
- **Suggested owner:** Perl/XS and binary-format architect
- **Goal:** Define one exact event model that v5, v6, `ReadStream`, converters, and dual comparison share.
- **Work:** Specify every event type, field, units, byte/UTF-8 behavior, ordering, process/run association, unknown-event policy, and incomplete-stream behavior. Map v5 records and public callbacks to it.
- **Deliverables:** normative Markdown plus machine-readable schema and examples.
- **Provisional evidence:** `docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md`, `docs/contracts/logical-events.schema.json` (not a v6 wire freeze; open items OI-001/002/003 remain).
- **Acceptance:** Every record emitted by 6.15 is representable without ambiguity or loss; both v5/v6 decoders can target the schema.

### COMPAT-002 - Define volatile-field normalization

- **Status:** in-progress (provisional dump structural rules landed 2026-08-07)
- **Size:** M
- **Dependencies:** COMPAT-001, BASE-005
- **Suggested owner:** test architecture engineer
- **Goal:** Permit useful comparisons without hiding semantic differences.
- **Work:** Classify PIDs, absolute fixture roots, wall-clock basetime, generated timestamps, versions, compression bytes, HTML whitespace, and ordering. Explicitly forbid normalization of ticks, counts, event order, locations, source/name bytes, call edges, and report values.
- **Deliverables:** versioned normalization spec, library, and mutation tests.
- **Provisional evidence:** `docs/contracts/COMPAT-002_VOLATILE_NORMALIZATION.md`, `tools/oracle/normalize_jsonl.py`, `tools/oracle/selftest_normalize_compat.sh` (not full surface matrix: HTML/PID/report volatiles remain open).
- **Acceptance:** Every ignored field is traceable to a rule; seeded semantic changes always fail comparison.

### COMPAT-003 - Define precision and numeric-conversion policy

- **Status:** in-progress (provisional policy landed 2026-08-07)
- **Size:** L
- **Dependencies:** BASE-003, COMPAT-001
- **Suggested owner:** numerical systems engineer
- **Goal:** Preserve exact ticks internally while providing compatible v5/API/report projections.
- **Work:** Define tick domains, signedness, widths, checked accumulation, clock scale, native-NV layouts/provenance, conversion to seconds, display rounding, and strict v6-to-v5 representability behavior.
- **Deliverables:** precision ADR and boundary/cross-platform vectors.
- **Provisional evidence:** `docs/contracts/COMPAT-003_PRECISION_NUMERIC_POLICY.md`; dump float path via `normalize_jsonl.py` `normalize_number` / `%.17g` (OI-003-01/02, NV portability still open).
- **Acceptance:** No timing/count transformation lacks an exact or explicitly bounded definition; unsupported native layouts are never guessed.

### COMPAT-004 - Classify public and de facto public surfaces

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-004, BASE-005
- **Suggested owner:** Perl/API compatibility maintainer
- **Goal:** Prevent accidental removal of under-documented but used behavior.
- **Work:** Classify packages, methods, object fields, callbacks, CLI behavior, report files/anchors, diagnostics, and output formats as public, compatibility-supported, internal-but-observed, or changeable only by ADR.
- **Deliverables:** support matrix with owners and required tests.
- **Acceptance:** Every compatibility fixture and downstream consumer maps to a classified surface.

### COMPAT-005 - Freeze the cross-version compatibility matrix

- **Status:** proposed
- **Size:** M
- **Dependencies:** COMPAT-001 through COMPAT-004, BASE-005
- **Suggested owner:** compatibility lead
- **Goal:** Convert matrix M1-M10 into executable producer/consumer requirements.
- **Work:** Define exact versions, build isolation, options, expected artifacts, normalization, representability exceptions, and failure categories for every cell.
- **Deliverables:** machine-readable matrix specification.
- **Acceptance:** The release runner can execute each row without inferring missing expectations.

### COMPAT-006 - Freeze report and auxiliary-output parity rules

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-005, COMPAT-002, COMPAT-004
- **Suggested owner:** reporting compatibility architect
- **Goal:** Define value, DOM, artifact, link, CSV, Callgrind, folded-stack, and graph comparison requirements.
- **Work:** Inventory every baseline report and auxiliary artifact; define semantic fields, structural fields, volatile normalizers, comparison levels, and seeded mismatch cases for each output.
- **Deliverables:** report parity specification and approved variable-field list.
- **Acceptance:** Every baseline artifact has a parser/normalizer/comparator and seeded regressions are detected.

### COMPAT-007 - Freeze Perl object and callback fidelity rules

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-004, COMPAT-001, COMPAT-004
- **Suggested owner:** senior Perl/XS maintainer
- **Goal:** Specify classes, AV/HV/SV shapes, scalar flags, identity, mutation, context, callbacks, errors, and lifetime behavior.
- **Work:** Trace legacy object construction and callbacks; record class/shape/identity/scalar-flag/lifetime behavior; define structural snapshots and mutation/context/error probes.
- **Deliverables:** object/callback contract and structural snapshot schema.
- **Acceptance:** Native facade work has objective tests rather than relying on method-level similarity.

### COMPAT-008 - Freeze CLI and diagnostic compatibility rules

- **Status:** proposed
- **Size:** M
- **Dependencies:** BASE-005, COMPAT-004
- **Suggested owner:** CLI compatibility engineer
- **Goal:** Define options, precedence, help, streams, exit codes, paths, overwrite/error behavior, and allowable diagnostic evolution.
- **Work:** Capture every installed command, option, environment/config precedence rule, stream, exit code, output path, overwrite rule, help/version surface, and representative diagnostic.
- **Deliverables:** black-box CLI contract.
- **Acceptance:** Existing wrappers can be tested independently of implementation language.

### COMPAT-009 - Freeze support-tier and dependency compatibility

- **Status:** proposed
- **Size:** M
- **Dependencies:** BASE-001, COMPAT-004
- **Suggested owner:** release/build architect
- **Goal:** State what old Perl/platform/no-Rust environments retain and what requires an explicit policy change.
- **Work:** Map supported Perl/platform/build tiers and optional dependencies; define legacy-only, native-capable, and unsupported scenarios with installation and runtime expectations.
- **Deliverables:** provisional tier matrix feeding BUILD-001.
- **Acceptance:** Native work cannot accidentally make Rust or a new codec mandatory for legacy functionality.

### COMPAT-010 - Define error, fallback, and corruption policy

- **Status:** proposed
- **Size:** L
- **Dependencies:** COMPAT-005, COMPAT-008
- **Suggested owner:** reliability/compatibility architect
- **Goal:** Distinguish capability fallback from corruption, unsupported format/codec, resource limits, I/O, and internal errors.
- **Work:** Classify capability, format, codec, corruption, resource, I/O, compatibility, and internal failures; define when fallback is allowed, forbidden, or requires explicit user action.
- **Deliverables:** error taxonomy and fallback decision table.
- **Acceptance:** Automatic fallback cannot hide data corruption or a semantic failure.

### COMPAT-011 - Define legacy/no-Rust continuity requirements

- **Status:** proposed
- **Size:** M
- **Dependencies:** COMPAT-009, COMPAT-010
- **Suggested owner:** CPAN portability maintainer
- **Goal:** Keep v5 collection/report installation usable on supported legacy tiers throughout migration.
- **Work:** Specify and exercise no-Rust and legacy-tier installation/runtime paths for v5 collection, reading, reporting, conversion availability, and diagnostic behavior.
- **Deliverables:** required legacy feature matrix and install scenarios.
- **Acceptance:** A missing Rust toolchain is not itself a failure in optional-native phases.

### COMPAT-012 - Define cross-version fixture-runner isolation

- **Status:** proposed
- **Size:** M
- **Dependencies:** COMPAT-005, BASE-001
- **Suggested owner:** release test engineer
- **Goal:** Ensure old and candidate modules/binaries never contaminate one another during regression runs.
- **Work:** Design process, environment, library-path, module-path, temporary-directory, locale, and executable isolation so oracle and candidate runs cannot load one another’s components.
- **Deliverables:** process/environment/path isolation design and provenance fields.
- **Acceptance:** Every matrix artifact identifies exact executable/module paths and versions.

### COMPAT-013 - Define downstream-consumer validation policy

- **Status:** proposed
- **Size:** M
- **Dependencies:** COMPAT-004, COMPAT-005
- **Suggested owner:** ecosystem compatibility engineer
- **Goal:** Test real public consumers not represented by upstream tests.
- **Work:** Identify representative downstream consumers, pin versions/licenses, define smoke/deep validation cases, and establish how discovered assumptions become fixtures or ADRs.
- **Deliverables:** pinned/licensed corpus policy, smoke expectations, issue escalation path.
- **Acceptance:** Discovered assumptions become focused upstream fixtures or approved compatibility decisions.

### COMPAT-014 - Perform compatibility sign-off

- **Status:** proposed
- **Size:** M
- **Dependencies:** COMPAT-001 through COMPAT-013, TEST-020
- **Suggested owner:** compatibility reviewer independent of implementation leads
- **Goal:** Confirm a release/default change meets the complete contract.
- **Work:** Review the completed compatibility matrix, differential evidence, accepted ADRs, migration text, platform results, and unresolved exceptions independently of implementation owners.
- **Deliverables:** signed matrix and known-difference/ADR report.
- **Acceptance:** No unresolved blocker; every accepted difference has scope, test, migration text, owner, and review date.

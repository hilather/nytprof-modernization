# Perl API, XS Integration, and Compatibility Task Plan

## 1. Objective

Keep existing Perl applications and bundled scripts working while allowing the Rust model/report engine to replace expensive internals selectively. The compatibility facade is an adapter, not a second source of profiling semantics.

## 2. Backend selection

Required conceptual modes:

```text
legacy  - existing 6.15 C/Perl reader and report path
native  - Rust decoder/model/report path; fail if unavailable
 auto   - choose native only when format/platform/features are supported, otherwise explicit legacy fallback
```

Selection may be exposed through `NYTPROF`, CLI flags, environment variables, or constructor options after ADR review. It must be observable in diagnostics so a regression report can identify the backend.

## 3. Compatibility facade layers

### 3.1 Event streaming

`Devel::NYTProf::ReadStream` must receive the same logical callbacks in the same order with compatible Perl scalar flags and values. The Rust parser may send batches to XS, but XS invokes Perl callbacks according to the historical contract.

### 3.2 Data objects

`Devel::NYTProf::Data` and related classes need a native-backed implementation that can:

- answer common methods without materializing the entire Perl graph;
- materialize exact legacy AV/HV/SV structures when callers inspect them directly;
- preserve class names, reference types, ordering rules, and numeric units;
- avoid per-field FFI calls in large report loops.

A transitional approach can expose coarse native snapshots per file/sub/report section.

### 3.3 Reader/report facade

Existing constructor options and callbacks remain accepted. Native report generation may bypass `Reader.pm` internally, but the script/API facade must preserve output behavior. Any unsupported customization callback must route to the legacy renderer or an explicitly compatible bridge.

### 3.4 FileHandle and writer APIs

Inventory whether users instantiate or call `Devel::NYTProf::FileHandle` directly. Preserve documented/de facto methods or classify them through `COMPAT-004` before changing behavior.

## 4. Perl scalar fidelity

Compatibility tests must inspect more than displayed text:

- IV/UV/NV/string flags where observable;
- UTF-8 flag on strings;
- taint behavior if relevant to supported Perls;
- blessed class and inheritance;
- references and nested shape;
- `undef` versus missing key/empty string/zero;
- numeric conversion and formatting;
- mortal/refcount lifetime under callback exceptions.

## 5. Lazy materialization rules

Lazy native-backed objects are permitted only when:

- method results and exception timing remain compatible;
- object identity is stable;
- mutations either update the expected Perl view or force full materialization;
- direct hash dereference behavior is preserved for hash-based objects;
- fork/thread/interpreter lifetime is safe;
- native handles cannot outlive the owning interpreter or loaded library.

When these conditions are expensive or ambiguous, eagerly materialize that API surface and optimize higher-level report paths separately.

## 6. Perl/XS compatibility tasks

### PERL-001 - Complete the public API inventory and contract suite

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BASE-004, COMPAT-004
- **Agent:** senior Perl maintainer
- **Work:** black-box methods, contexts, object graphs, warnings/errors, scalar flags, mutability, bundled-tool usage.
- **Deliverables:** API matrix and executable tests against 6.15.
- **Acceptance:** every facade task references a covered contract entry.

### PERL-002 - Implement backend discovery and forcing

- **Status:** proposed
- **Size:** M
- **Dependencies:** BUILD-004, ARCH-005
- **Agent:** Perl/XS integration engineer
- **Work:** detect native library/binary and supported format/features; expose legacy/native/auto; provide version diagnostics.
- **Deliverables:** backend selector module and tests.
- **Acceptance:** forced modes never silently switch; auto fallback reason is inspectable; legacy-only install works.

### PERL-003 - Implement Rust error-to-Perl translation

- **Status:** proposed
- **Size:** M
- **Dependencies:** RUST-010, PERL-001
- **Agent:** XS engineer
- **Work:** map error categories, offsets, feature/codec errors, I/O, corruption, limits, and panics to compatible exceptions/warnings.
- **Deliverables:** error mapping table and tests.
- **Acceptance:** no panic crosses FFI; objects are cleaned up when callbacks die; CLI exit classes remain compatible.

### PERL-004 - Implement native-backed ReadStream

- **Status:** proposed
- **Size:** XL
- **Dependencies:** RUST-004, RUST-010, COMPAT-001, PERL-001
- **Agent:** Perl/XS engineer
- **Work:** batch transfer from Rust, scalar creation, callback dispatch, `$.`/sequence behavior, early termination, callback exceptions.
- **Deliverables:** selectable native ReadStream backend.
- **Acceptance:** callback type/order/value/scalar-flag tests match legacy over corpus.
- **Regression gate:** C2, M3.

### PERL-005 - Implement compact Data backend facade

- **Status:** proposed
- **Size:** XL
- **Dependencies:** RUST-006 through RUST-009, RUST-010, PERL-001
- **Agent:** Perl/Rust object-model engineer
- **Work:** map files, subs, lines, blocks, calls, runs, attributes, and source; choose eager/lazy boundaries.
- **Deliverables:** native Data implementation behind feature flag.
- **Acceptance:** API contract passes; common report use does not materialize full legacy graph.
- **Regression gate:** C4.

### PERL-006 - Implement legacy object graph materializer

- **Status:** proposed
- **Size:** XL
- **Dependencies:** PERL-005, BASE-004
- **Agent:** XS/Perl compatibility engineer
- **Work:** create exact AV/HV/SV/reference/blessing shape for callers that require internals; preserve UTF-8 and numeric units.
- **Deliverables:** coarse materialization APIs and snapshot tests.
- **Acceptance:** deep normalized dumps and direct-access tests match legacy.

### PERL-007 - Preserve eval collapse and file identity behavior

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-009, PERL-005
- **Agent:** Perl source/eval specialist
- **Work:** reproduce `Data.pm` post-processing, file IDs, eval naming, source parentage, anonymous sub treatment.
- **Deliverables:** compatibility functions and fixtures.
- **Acceptance:** Data/report results match for all eval/source fixtures.

### PERL-008 - Preserve Reader customization callbacks

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-004, REPORT-001, PERL-002
- **Agent:** Perl/reporting engineer
- **Work:** inventory callbacks/templates/hooks; define native support, batch bridge, or legacy fallback per hook.
- **Deliverables:** support matrix and implementation.
- **Acceptance:** no existing supported customization is silently ignored; fallback is explicit and tested.

### PERL-009 - Preserve FileHandle-facing APIs

- **Status:** proposed
- **Size:** M
- **Dependencies:** PERL-001, COL-006
- **Agent:** XS compatibility engineer
- **Work:** test constructor/method/options and direct consumers; maintain v5 behavior or route through sink abstraction.
- **Deliverables:** compatibility layer and tests.
- **Acceptance:** classified supported calls behave as 6.15.

### PERL-010 - Preserve configuration parsing and precedence

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-005, COL-001
- **Agent:** Perl configuration engineer
- **Work:** duplicate all `NYTPROF` parsing, abbreviations, quoting/path behavior, environment/CLI/constructor precedence, unknown-option handling; add format/backend options without ambiguity.
- **Deliverables:** shared parser tests and option contract.
- **Acceptance:** old options/defaults are unchanged; new options are opt-in during transition.

### PERL-011 - Validate reference counts and interpreter lifetime

- **Status:** proposed
- **Size:** L
- **Dependencies:** PERL-004 through PERL-006, RUST-010
- **Agent:** XS memory-safety engineer
- **Work:** callback exceptions, object destruction order, global destruction, repeated open/close, fork, multiple profiles, embedded interpreters where supported.
- **Deliverables:** stress tests under ASan/valgrind and Perl debugging builds.
- **Acceptance:** no leak, double free, stale native handle, or use after interpreter teardown.

### PERL-012 - Implement capability-aware auto fallback

- **Status:** proposed
- **Size:** M
- **Dependencies:** PERL-002, PERL-003, BUILD-005
- **Agent:** integration engineer
- **Work:** distinguish unavailable backend/codec/unsupported NV layout from corrupt profile or internal error.
- **Deliverables:** fallback decision table and logs.
- **Acceptance:** auto falls back only for approved capability reasons; corruption and semantic errors remain visible.

### PERL-013 - Add backend parity test runner

- **Status:** proposed
- **Size:** M
- **Dependencies:** PERL-002 through PERL-012, TEST-001
- **Agent:** Perl test engineer
- **Work:** run each API test against legacy and native, normalize allowed differences, produce first structural mismatch.
- **Deliverables:** `prove` integration and machine-readable report.
- **Acceptance:** backend parity is a required CI job.

### PERL-014 - Document compatibility and escape hatches

- **Status:** proposed
- **Size:** S
- **Dependencies:** PERL-002 through PERL-012
- **Agent:** documentation engineer
- **Work:** backend selection, supported platforms, troubleshooting, fallback, converter use, deprecation policy.
- **Deliverables:** POD and migration guide.
- **Acceptance:** a user can force legacy collection/report and identify which backend produced a result.

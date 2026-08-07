# Rust Core, Decoder, Aggregation, and Data Model Task Plan

## 1. Scope

The Rust core owns the new safe parsing and report-side computation path. It must support v5 input before v6 becomes a collector option so that correctness can be established against existing files and tools.

Rust does not redefine NYTProf semantics. It implements semantics frozen by the compatibility and baseline workstreams.

## 2. Workspace boundaries

```text
nytprof-types
  - IDs, ticks, byte strings, UTF-8 flags, logical events, limits, errors

nytprof-format-v5
  - streaming decoder, native-NV interpretation, canonical event output

nytprof-format-v6
  - chunk decoder/encoder, codecs, dictionaries, deltas, checksums, indexes

nytprof-model
  - compact exact profile structures and provenance

nytprof-aggregate
  - ordered replay, statement/block/sub/call aggregation, merge

nytprof-report-ir
  - deterministic presentation-independent report model

nytprof-html
  - HTML and auxiliary report generation (with nytprof-cli)

nytprof-cli
  - inspect, verify, convert, merge, html, calls

nytprof-ffi
  - coarse-grained C ABI and ownership handles

nytprof-testkit
  - fixtures, generators, canonical dumps, differential comparators
```

These names match `00_EXECUTIVE_ARCHITECTURE.md` and `03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md`.

## 3. Logical event model

Use a format-independent event enum or tagged union. Required design characteristics:

- borrowed `EventRef` for zero-copy streaming;
- owned `Event` for tests, queues, and retained call streams;
- original byte strings and semantic UTF-8 flag;
- explicit process stream ID and logical sequence;
- integer ticks plus clock domain metadata;
- opaque extension event for preserved unknown/skippable data where permitted;
- stable canonical serialization for regression tests;
- no exposure of v5 tag bytes in consumers.

## 4. v5 numeric portability

V5 writes some Perl `NV` values as native memory. The parser must use profile metadata and a supported-layout table. Requirements:

- explicitly identify size, byte order, and known floating representation;
- support common IEEE binary64 and verified long-double layouts;
- reject or route unsupported layouts to the legacy reader;
- preserve raw bytes in provenance for strict round trips;
- never reinterpret unknown bytes as `f64` by assumption;
- compare semantic values using exact decoded representation or documented rational/tick conversion, not rounded display strings.

## 5. Compact model strategy

The native report model should avoid a generic object per scalar. Candidate structures:

- dense vectors indexed by internal file/sub IDs;
- sparse sorted vectors or hash maps for executed lines, converted to dense only when beneficial;
- interned byte-string arenas;
- source blob table plus per-line slices;
- checked `i128` accumulators where sums can exceed `i64` before output validation;
- separate process/run metadata;
- call-site tables keyed by compact IDs;
- optional exact call-event storage only when a requested report requires it;
- provenance references for diagnostics/conversion.

The model must retain enough information to materialize legacy Perl objects and all reports.

## 6. Aggregation semantics

Aggregation consumes events in order and must match the legacy loader/report logic, including:

- discount treatment;
- line/block/sub attribution;
- eval file collapse rules;
- inclusive/exclusive/recursive call calculations;
- caller/callee counts and maximum depth;
- process/run boundaries;
- source definitions arriving before or after related metadata where v5 permits it;
- duplicate/redefined subroutine names;
- merged profiles with independent ID namespaces.

Derived summaries in v6 are hints. The aggregator validates their source sequence/schema and can recompute from raw events.

## 7. Concurrency model

Parsing a single ordered event stream remains sequential where state dependencies require it. Parallelism is introduced after safe partition points:

- independent compressed chunk decompression when dictionary/delta snapshots permit;
- per-process streams after global ordering requirements are resolved;
- per-file report IR construction;
- independent output rendering;
- independent callgraph transformations.

All reductions use deterministic merge order and checked arithmetic.

## 8. FFI model

Expose coarse operations, not per-field/per-event chatter:

```text
open_profile(path/options) -> opaque handle
stream_events(handle, callback/batch) -> status
build_model(handle/options) -> model handle
materialize_legacy(model, requested section) -> Perl-owned structures
render_report(model, options) -> result manifest
close(handle)
```

Rules:

- no Rust panic crosses FFI;
- all buffers have pointer+length and ownership contract;
- Perl scalars/references are created in XS on the Perl side unless a carefully audited API says otherwise;
- thread affinity and interpreter context are explicit;
- opaque handles have generation/magic validation;
- cancellation/error cleanup is idempotent.

## 9. Rust core tasks

### RUST-001 - Establish workspace, policy, and coding standards

- **Status:** proposed
- **Size:** M
- **Dependencies:** BUILD-001, ARCH-003
- **Agent:** Rust technical lead
- **Work:** create crates, lint rules, formatting, MSRV policy, dependency review, unsafe policy, error conventions, feature flags.
- **Deliverables:** compiling workspace and contributor guide.
- **Acceptance:** CI builds minimal/default/all features; unsafe is denied except named modules.

### RUST-002 - Implement canonical logical event types

- **Status:** proposed
- **Size:** L
- **Dependencies:** COMPAT-001, ARCH-003, RUST-001
- **Agent:** Rust API engineer
- **Work:** implement IDs, tick domains, borrowed/owned events, bytes+UTF-8 flags, process/sequence metadata, canonical equality.
- **Deliverables:** `nytprof-types` API and schema documentation.
- **Acceptance:** every inventoried v5 event is representable without loss; exhaustive enum tests exist.

### RUST-003 - Implement bounded streaming I/O primitives

- **Status:** proposed
- **Size:** M
- **Dependencies:** RUST-001, SEC-001
- **Agent:** Rust systems engineer
- **Work:** checked reads, offsets, varints, byte limits, decompression adapters, detailed errors, no untrusted-size allocation.
- **Deliverables:** shared parser primitives.
- **Acceptance:** malformed/truncated inputs never panic or allocate beyond configured limits.

### RUST-004 - Implement the v5 streaming decoder

- **Status:** proposed
- **Size:** XL
- **Dependencies:** FMT-001, RUST-002, RUST-003, BASE-002
- **Agent:** binary-format/Rust engineer
- **Work:** parse headers, attributes, compressed stream, every record, state, and callbacks; expose exact offsets and sequence.
- **Deliverables:** `nytprof-format-v5` decoder and event dump.
- **Acceptance:** canonical output equals legacy `ReadStream` over the complete v5 corpus.
- **Regression gate:** C2, M3.

### RUST-005 - Implement v5 native-NV decoding and provenance

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BASE-003, COMPAT-003, RUST-004
- **Agent:** numerical portability engineer
- **Work:** support verified layouts, raw-byte preservation, endian conversion, exact comparison/projection policy, unsupported-layout fallback.
- **Deliverables:** layout table, test vectors from real platforms, provenance API.
- **Acceptance:** no unsupported layout is guessed; known fixtures match legacy values and round-trip requirements.

### RUST-006 - Implement compact profile model

- **Status:** proposed
- **Size:** XL
- **Dependencies:** ARCH-004, RUST-002, BASE-004, BASE-008
- **Agent:** Rust data-layout engineer
- **Work:** implement file/source/sub/call/process tables, intern arenas, sparse line metrics, provenance, and memory instrumentation.
- **Deliverables:** `nytprof-model` plus size accounting.
- **Acceptance:** model represents all API/report inputs; large baseline uses substantially less RSS than Perl graph or has a documented optimization backlog.

### RUST-007 - Implement statement/block aggregation

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-004, RUST-006, BASE-003
- **Agent:** Rust correctness engineer
- **Work:** replay TIME_LINE/TIME_BLOCK/DISCOUNT semantics and checked totals.
- **Deliverables:** aggregator and differential tests.
- **Acceptance:** counts/ticks per line/block/sub match legacy on every fixture.
- **Regression gate:** C2/C4.

### RUST-008 - Implement subroutine/call aggregation

- **Status:** proposed
- **Size:** XL
- **Dependencies:** RUST-004, RUST-006, BASE-003, COMPAT-001
- **Agent:** callgraph algorithms engineer
- **Work:** inclusive/exclusive/recursive totals, caller/callee, depth, recursion, entry/return stream, abnormal stack behavior.
- **Deliverables:** call aggregator and canonical call tables.
- **Acceptance:** legacy Data/HTML/calls/flame/Callgrind values match across call fixture matrix.
- **Regression gate:** C2/C4.

### RUST-009 - Implement source/eval/sub-definition model

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-004, RUST-006, FMT-008
- **Agent:** source/eval specialist
- **Work:** file identity, eval parents/lines, source bytes, UTF-8 flags, anonymous/redefined subs, pmc/AutoLoader behavior.
- **Deliverables:** source model and compatibility views.
- **Acceptance:** source pages and API data match oracle; no path/name conflation.

### RUST-010 - Define and implement stable C ABI

- **Status:** proposed
- **Size:** XL
- **Dependencies:** ARCH-005, RUST-002, RUST-004, BUILD-002
- **Agent:** senior Rust/C FFI engineer
- **Work:** opaque handles, status/error objects, batch structures, callbacks, ownership, panic containment, version negotiation.
- **Deliverables:** C header, Rust implementation, C harness, ABI tests.
- **Acceptance:** valgrind/ASan/Miri-relevant tests pass; ABI version mismatch fails cleanly; no per-event FFI required in production paths.

### RUST-011 - Implement v6 encoder/decoder core

- **Status:** proposed
- **Size:** XL
- **Dependencies:** FMT-002 through FMT-012, RUST-002, RUST-003
- **Agent:** Rust format engineer
- **Work:** chunks, codecs, dictionaries, deltas, checksums, limits, unknown features, optional index.
- **Deliverables:** `nytprof-format-v6`, test vectors, round-trip/property tests.
- **Acceptance:** independent C/Rust vectors agree; strict decoder rejects all specified malformed forms.
- **Regression gate:** C3.

### RUST-012 - Implement mixed-format streaming abstraction

- **Status:** proposed
- **Size:** M
- **Dependencies:** RUST-004, RUST-011
- **Agent:** Rust API engineer
- **Work:** auto-detect format, expose one event iterator, carry format/provenance/capability metadata.
- **Deliverables:** profile source abstraction.
- **Acceptance:** consumers contain no v5/v6 semantic branching except explicit representability diagnostics.

### RUST-013 - Implement deterministic merge engine

- **Status:** proposed
- **Size:** XL
- **Dependencies:** RUST-006 through RUST-012, TOOL-004
- **Agent:** data integration engineer
- **Work:** remap file/sub/source/process IDs, reconcile metadata, preserve run boundaries, checked sum, stable ordering, mixed input.
- **Deliverables:** merge library and manifest.
- **Acceptance:** legacy-compatible totals and deterministic output independent of input discovery order when documented sorting applies.

### RUST-014 - Implement exact derived-summary validation

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-011, RUST-007, RUST-008, RUST-011
- **Agent:** data integrity engineer
- **Work:** validate schema, source sequence range/hash, counts, and optional spot/full replay; ignore invalid cache.
- **Deliverables:** summary reader/writer and validation modes.
- **Acceptance:** report from summary equals raw replay; corrupt/stale summary never changes results.

### RUST-015 - Implement bounded call-event retention modes

- **Status:** proposed
- **Size:** M
- **Dependencies:** RUST-008, REPORT-001, REPORT-002
- **Agent:** performance/data-model engineer
- **Work:** stream-only, retained compact, and indexed-on-disk modes selected by requested outputs.
- **Deliverables:** policy and implementation.
- **Acceptance:** standard reports do not retain call events unnecessarily; flame/calls output remains exact.

### RUST-016 - Add model memory and phase telemetry

- **Status:** proposed
- **Size:** M
- **Dependencies:** RUST-006
- **Agent:** performance engineer
- **Work:** account intern arenas, line tables, source blobs, call data, temporary buffers, parse/aggregate/render durations.
- **Deliverables:** optional JSON telemetry.
- **Acceptance:** disabled overhead is negligible; benchmark reports can attribute RSS and time.

### RUST-017 - Prove deterministic parallel reductions

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-006 through RUST-009, REPORT-002
- **Agent:** concurrency engineer
- **Work:** stable partition/merge order, checked integer sums, deterministic tie breaks, repeated stress runs.
- **Deliverables:** concurrency policy and determinism tests.
- **Acceptance:** output hashes after normalization are identical across 1..N worker counts.

### RUST-018 - Add property and fuzz targets for core crates

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-003 through RUST-015, SEC-002
- **Agent:** fuzz/test engineer
- **Work:** arbitrary event streams, round trips, malformed lengths/varints/chunks, dictionary state, truncation, merge algebra where valid.
- **Deliverables:** fuzz harnesses and corpus.
- **Acceptance:** agreed execution budget passes with no crash, UB, unbounded allocation, or semantic round-trip failure.

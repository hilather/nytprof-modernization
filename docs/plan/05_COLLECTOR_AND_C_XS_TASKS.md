# Collector and C/XS Architecture and Task Plan

## 1. Scope

This workstream changes the code that executes inside the profiled Perl process. It is the highest-risk area because even a small operation added to the statement hook can alter runtime overhead and because changes to clock placement can change attribution.

The collector remains exact. It must produce the same logical events as 6.15 for the same execution and configuration. The only permitted reductions are representational and operational overhead that does not remove information.

## 2. Collector invariants

1. The Perl interpreter hook remains in C/XS.
2. Common statement-event capture does not allocate from the general heap.
3. No Rust or other FFI transition occurs for each statement/call event.
4. Clock-read placement and overhead discounting are frozen by `BASE-003` and `COMPAT-003`.
5. The v5 sink remains available and readable by unmodified 6.15 tools.
6. `format=dual` captures one logical event and fans it out to both sinks.
7. Any buffering preserves exact order across statement, discount, call, metadata, source, and process events.
8. Flushes, compression, and I/O time are excluded or attributed exactly according to the legacy timing contract.
9. Fork/PID, start/stop, signal, normal exit, and partial-write behavior are explicit state-machine transitions.
10. A failed sink cannot silently continue with missing events.

## 3. Proposed collector layers

```text
Perl debugger/op hooks
        |
        v
legacy timing/call state machine
        |
        v
canonical C event emission API
        |
        +--> v5 sink (legacy-compatible wire encoding)
        |
        +--> v6 sink candidate A (C encoder)
        |
        +--> v6 sink candidate B (batched Rust encoder, no per-event FFI)
        |
        +--> dual/test/counting sinks
```

The v6 writer implementation language is deliberately an ADR decision. A C writer minimizes runtime/build disruption. A batched Rust writer may improve safety and code reuse with offline tooling. Both must consume the same C event API and must be benchmarked without changing semantics.

## 4. Canonical in-memory event representation

The collector API should not mirror v5 bytes. It should express logical values:

- signed 64-bit ticks where raw ticks exist;
- checked unsigned IDs/counts/depths;
- explicit byte-string pointer, length, and UTF-8 semantic flag;
- explicit process ID and event sequence;
- source location fields, never implicit global state at the sink boundary;
- option/attribute values with typed or byte-preserving representation.

A common statement event should fit in a small fixed structure. Variable-length payloads use one of:

- immediate synchronous encode;
- a bounded side arena that copies bytes once;
- a dictionary intern operation outside the statement-hot path.

No event may retain a borrowed Perl pointer beyond its guaranteed lifetime.

## 5. Buffering and flush design

### 5.1 Event buffer

Candidate design:

- fixed array of event headers;
- fixed or growable-but-bounded byte arena for strings/source fragments;
- sequence number on every logical event;
- high-water mark that triggers flush before capacity exhaustion;
- emergency path for an individual payload larger than the arena;
- reset only after all selected sinks acknowledge the batch.

### 5.2 Flush boundaries

Mandatory flush or state synchronization points include:

- buffer full/high-water mark;
- profiler stop;
- profile file switch;
- fork parent and child transition;
- PID change detection;
- final source/sub-caller summaries;
- normal close;
- signal/exit cleanup where safe;
- before discarding or replacing a failed sink.

### 5.3 Timing treatment

A buffer flush can be much more expensive than an event append. Tests must prove that its time is treated the same way as current serialization/compression work. Do not move a clock read around a flush without a dedicated timing ADR and oracle test.

## 6. v5 compatibility sink

The initial v5 sink should be a thin adaptation of the existing event writers. Requirements:

- same header/version/attributes;
- same event order and field projection;
- same zlib option behavior;
- same source and finalization behavior;
- same warnings/errors for overflow and I/O where feasible;
- no requirement that files be byte-identical, but canonical event streams must be equal;
- a byte-identical diagnostic mode is desirable for carefully controlled fixtures.

When the canonical event uses values wider than v5, `format=v5` follows the frozen 6.15 behavior. It must not silently adopt v6-only semantics.

## 7. v6 sink

Required hot-path properties:

- integer-tick serialization without `NV` conversion;
- reversible location/depth delta coding;
- dictionary references for repeated names;
- chunk buffering and checksum generation outside the smallest event append path;
- codec selection that is benchmarked on exact equivalent workloads;
- no global lock in a non-threaded interpreter path;
- bounded memory and explicit error status;
- complete chunks remain readable after abnormal termination.

## 8. String and source handling

Subroutine names, paths, attributes, and source content have different lifetimes and frequency. Do not use a single unbounded hash blindly.

Evaluate:

- pointer-identity cache plus byte verification for stable Perl symbols;
- content-hash dictionary for arbitrary/eval names;
- monotonic IDs per process/profile;
- dictionary reset only at explicit chunks with snapshots;
- source blob hashing outside the statement hot path;
- duplicate source sharing across fids without changing logical `SRC_LINE` callback order.

All byte content and UTF-8 flags must round-trip.

## 9. Process, fork, and lifecycle handling

The sink lifecycle must model:

```text
UNINITIALIZED -> OPEN -> ACTIVE -> STOPPED -> FINALIZING -> CLOSED
                       |          |
                       |          +-> ACTIVE (restart where supported)
                       +-> FORK_SPLIT -> parent/child OPEN or CLOSED
                       +-> FAILED
```

The state machine must cover:

- lazy initialization;
- compile/INIT/runtime/END start modes;
- explicit start/stop;
- `addpid` and file-name changes;
- parent and child behavior under `forkdepth`;
- child ownership of inherited buffers/compression state;
- no double-finalization;
- best-effort recovery without invoking unsafe operations from signal contexts;
- embedded/multiple-interpreter constraints already supported by 6.15.

## 10. Collector tasks

### COL-001 - Introduce the canonical sink interface

- **Status:** proposed
- **Size:** XL
- **Dependencies:** COMPAT-001, BASE-002, BASE-003, ARCH-001
- **Agent:** senior C/XS engineer
- **Work:** add semantic emit functions and a sink vtable or compile-time-specialized interface; adapt every current write site; keep old writer behavior behind the v5 sink.
- **Deliverables:** headers, implementation, event mapping table, unit tests, flame/assembly comparison of common path.
- **Acceptance:** all 6.15 tests pass; canonical v5 stream is unchanged; no new heap allocation appears in the common statement path.
- **Regression gate:** C1, M1, M4.
- **Risk:** indirect dispatch can cost more than direct calls; allow specialization/inlining for single-sink production mode.

### COL-002 - Freeze and test sink lifecycle

- **Status:** proposed
- **Size:** L
- **Dependencies:** ARCH-002, COL-001
- **Agent:** C systems engineer
- **Work:** implement explicit states and legal transitions; replace scattered implicit conditions where safe; add debug assertions and failure injection.
- **Deliverables:** state diagram, transition table, lifecycle tests.
- **Acceptance:** normal close, stop/restart, file switch, fork, failed write, and finalization are deterministic and leak-free.
- **Regression gate:** fork, start/stop, signal/exit fixtures.

### COL-003 - Add monotonic logical event sequence numbers

- **Status:** proposed
- **Size:** M
- **Dependencies:** COL-001
- **Agent:** C engineer
- **Work:** increment once per canonical logical event; expose to test/dual sinks; avoid writing it in v5 unless diagnostic metadata is explicitly enabled.
- **Deliverables:** sequence API and comparator hooks.
- **Acceptance:** sequence is gapless per process stream and permits exact first-mismatch reporting.
- **Regression gate:** no change to default v5 logical output.

### COL-004 - Build a no-allocation statement-event fast path

- **Status:** proposed
- **Size:** L
- **Dependencies:** COL-001, BASE-003, BENCH-003
- **Agent:** low-level performance engineer
- **Work:** minimize branches/copies, prebind the active sink, inline the common TIME_LINE/TIME_BLOCK append, and measure generated code.
- **Deliverables:** microbenchmark, disassembly notes, before/after counters.
- **Acceptance:** semantic equality and no median regression; target improvement is set by benchmark evidence.
- **Regression gate:** timing-attribution fixtures and all statement/block tests.

### COL-005 - Implement bounded event batching

- **Status:** proposed
- **Size:** XL
- **Dependencies:** COL-001, COL-003, BASE-003
- **Agent:** C systems/performance engineer
- **Work:** create fixed event buffer and side arena; define capacity, high-water behavior, oversized payload path, flush acknowledgment, and failure state.
- **Deliverables:** buffer module, metrics counters, stress tests.
- **Acceptance:** exact event order under all flush positions; bounded memory; no use-after-free of Perl data; flush overhead receives correct discount treatment.
- **Regression gate:** dual stream equality with forced capacities from 1 event to production size.

### COL-006 - Adapt the legacy v5 writer to the sink API

- **Status:** proposed
- **Size:** L
- **Dependencies:** COL-001, COL-005
- **Agent:** C/XS compatibility engineer
- **Work:** route semantic events through existing v5 encoding; preserve header, attributes, compression, source, and final records.
- **Deliverables:** v5 sink and mapping tests.
- **Acceptance:** unmodified 6.15 tools read new `format=v5`; canonical streams match the oracle across the corpus.
- **Regression gate:** M4.

### COL-007 - Prototype v6 writer candidate A in C

- **Status:** proposed
- **Size:** XL
- **Dependencies:** FMT-002 through FMT-010, COL-005
- **Agent:** C binary-format engineer
- **Work:** implement dictionaries, deltas, chunks, checksums, codec adapter, and integer ticks.
- **Deliverables:** C encoder and v6 vectors.
- **Acceptance:** Rust v6 decoder accepts output; all format vectors pass; no per-event general allocation.
- **Regression gate:** M5/M6.

### COL-008 - Prototype v6 writer candidate B with batched Rust FFI

- **Status:** deferred
- **Size:** XL
- **Dependencies:** FMT-002 through FMT-010, COL-005, RUST-010, BUILD-004
- **Agent:** C/Rust FFI engineer
- **Non-baseline:** The production architecture baseline is the **C v6 writer** (COL-007). COL-008 is an optional measured alternative only. Do not start COL-008 until dual-equality with the C writer is green and an ADR re-opens this task with evidence that a batched Rust encoder is worth the packaging and ABI cost. Never introduce per-event FFI.
- **Work:** pass batches through a stable C ABI; copy/borrow rules must be explicit; ensure no unwinding across FFI.
- **Deliverables:** optional encoder backend and benchmark comparison.
- **Acceptance:** exact output semantics, no per-event FFI, deterministic failure propagation, sanitizer-clean.
- **Regression gate:** M5/M6 and packaging fallback tests.

### COL-009 - Decide the production v6 writer backend

- **Status:** proposed
- **Size:** M
- **Dependencies:** COL-007, BENCH-006 (COL-008 only if re-opened), BUILD-004
- **Agent:** architecture review group
- **Work:** compare runtime, size, memory, binary size, portability, safety, maintenance, and build impact. Default recommendation is the C writer unless COL-008 evidence is re-opened and superior.
- **Deliverables:** ADR selecting default and fallback policy.
- **Acceptance:** decision cites raw measurements and support matrix; format remains implementation-independent.

### COL-010 - Implement dictionary interning for repeated names

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-005, COL-005
- **Agent:** C/Rust performance engineer according to selected backend
- **Work:** define byte/UTF-8 identity, deterministic IDs, collision handling, lifetime, fork reset/inheritance, and memory limits.
- **Deliverables:** dictionary module and telemetry.
- **Acceptance:** exact byte round-trip; no name conflation; measured size win on call-heavy profiles; acceptable collector overhead.
- **Regression gate:** anonymous/eval/AUTOLOAD/Unicode/redefinition fixtures.

### COL-011 - Move v6 timing serialization to integer ticks

- **Status:** proposed
- **Size:** L
- **Dependencies:** COMPAT-003, FMT-004, COL-006
- **Agent:** C numerical engineer
- **Work:** retain raw ticks through call/aggregate paths where currently converted to `NV`; use checked wide accumulators; keep v5 conversion isolated.
- **Deliverables:** tick types, conversion functions, overflow tests.
- **Acceptance:** no v6 timing is stored as native memory bytes; boundary and long-running fixtures do not discard intervals.
- **Regression gate:** dual equality after defined v5 projection; call/timing tests.

### COL-012 - Implement reversible delta and run encodings

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-006, FMT-007, COL-007 or COL-008
- **Agent:** format/performance engineer
- **Work:** encode location/depth locality and only approved exact runs; reset state at chunk boundaries.
- **Deliverables:** encoder paths and expansion tests.
- **Acceptance:** decoder reproduces event sequence/ticks exactly; feature can be toggled for A/B size benchmarks.
- **Regression gate:** randomized event-stream property tests.

### COL-013 - Implement source blob deduplication

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-008, COL-007 or COL-008
- **Agent:** source/eval specialist
- **Work:** hash exact bytes plus semantic flags, preserve fid/eval/line relationships, and avoid hot-path hashing.
- **Deliverables:** source dictionary/blob writer and collision tests.
- **Acceptance:** source callbacks and rendered source are identical; duplicate content stored once where valid.
- **Regression gate:** savesrc/eval/pmc/AutoLoader/Unicode/binary-line fixtures.

### COL-014 - Implement same-run dual writer

- **Status:** proposed
- **Size:** L
- **Dependencies:** COL-006, selected v6 writer, COL-003
- **Agent:** C compatibility engineer
- **Work:** fan out each canonical event; make finalization ordering identical; emit comparison metadata out-of-band.
- **Deliverables:** `format=dual` development mode and harness integration.
- **Acceptance:** decoded streams match event-for-event for all v5-representable fixtures.
- **Regression gate:** M6.

### COL-015 - Harden fork and PID transitions with buffered sinks

- **Status:** proposed
- **Size:** XL
- **Dependencies:** COL-002, COL-005, COL-014
- **Agent:** Perl/C process-lifecycle specialist
- **Work:** define pre/post-fork flush, inherited compressor state, file naming, sequence domains, dictionary state, and error behavior.
- **Deliverables:** fork protocol and stress suite.
- **Acceptance:** no duplicate/lost events, corrupt chunks, shared-FD races, or double close in parent/child matrices.
- **Regression gate:** all forkdepth/addpid/merge fixtures.

### COL-016 - Add collector observability counters

- **Status:** proposed
- **Size:** M
- **Dependencies:** COL-005
- **Agent:** performance engineer
- **Work:** count events by type, buffer flushes, raw/compressed bytes, dictionary hits/misses, oversized payloads, write calls, checksum/codec time, and failures.
- **Deliverables:** debug metadata or optional stderr/inspection report.
- **Acceptance:** counters have near-zero disabled cost and enable benchmark attribution.
- **Regression gate:** no default output changes.

### COL-017 - Preserve slow-op and leave-correction semantics

- **Status:** proposed
- **Size:** L
- **Dependencies:** COL-001, BASE-002, BASE-003
- **Agent:** Perl-internals specialist
- **Work:** trace option-specific op hooks and leave correction through the new event boundary; add dedicated fixtures.
- **Deliverables:** mapping and tests.
- **Acceptance:** all affected report metrics and callbacks match the oracle.
- **Regression gate:** slowops/leave feature matrix.

### COL-018 - Add collector fault injection

- **Status:** proposed
- **Size:** M
- **Dependencies:** COL-002, COL-005, selected v6 writer
- **Agent:** reliability engineer
- **Work:** simulate allocation failure, short write, ENOSPC, compressor error, checksum failure, close failure, and fork during near-full buffer.
- **Deliverables:** test hooks compiled only in development builds.
- **Acceptance:** deterministic error/finalization behavior with no memory corruption or silent event loss.
- **Regression gate:** SEC- and recovery suites.

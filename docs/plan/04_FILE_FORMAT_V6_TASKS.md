# Profile Format v6 Architecture and Task Plan

## 1. Design goals

Format v6 must:

1. preserve the complete ordered logical event stream;
2. represent time in portable integer ticks;
3. remove repeated strings from hot call records;
4. exploit location/depth locality through reversible deltas;
5. use independently verifiable chunks;
6. support streaming decode and bounded memory;
7. permit parallel or indexed report processing;
8. support safe recovery of complete chunks from a truncated file;
9. remain extensible without reusing ambiguous tags;
10. support exact comparison and conversion with v5.

## 2. Explicit rejection of lossy statement aggregation

A totals-only statement table is not a replacement for the event stream because `ReadStream` exposes individual `TIME_LINE`, `TIME_BLOCK`, and `DISCOUNT` events in order. V6 may add an optional derived summary, but the ordered events remain authoritative.

Allowed reversible compaction includes:

- dictionary IDs;
- location/depth deltas;
- encoding a repeated location once followed by a vector of exact per-event tick values, if no intervening logical event is hidden and the decoder re-emits the original sequence;
- chunk compression;
- source blob deduplication with ordered logical references;
- internal opcodes that expand to the same callback sequence.

## 3. Proposed top-level layout

```text
+------------------------------+
| fixed file header            |
+------------------------------+
| header TLVs                  |
+------------------------------+
| chunk 0                      |
+------------------------------+
| chunk 1                      |
+------------------------------+
| ...                          |
+------------------------------+
| optional index chunk         |
+------------------------------+
| optional footer              |
+------------------------------+
```

### 3.1 Fixed header draft

| Field | Draft type | Purpose |
|---|---:|---|
| Magic | 8 bytes | Unambiguous v6 identification. |
| Major | `u16 LE` | Incompatible format generation. |
| Minor | `u16 LE` | Backward-compatible additions. |
| Header length | `u32 LE` | Skip future header fields. |
| Required features | `u64 LE` | Reader must understand all set bits. |
| Optional features | `u64 LE` | Reader may ignore unknown sections. |
| Header CRC | `u32 LE` | Detect damaged metadata before allocation. |

Exact magic and layout require ADR approval and test vectors.

### 3.2 Header TLVs

Candidate TLVs:

- producer name/version;
- Perl version and configuration fingerprint;
- clock ID and ticks per second;
- profile/application metadata;
- default codec;
- limits/hints;
- source-storage mode;
- creation timestamp;
- format schema UUID/hash.

Options and attributes that are logically part of the event stream must still be exposed in their original order. A header copy may be an acceleration hint, not the sole source of truth, unless compatibility analysis proves otherwise.

## 4. Chunk framing

Draft chunk header:

| Field | Draft type | Notes |
|---|---:|---|
| Sync word | `u32 LE` | Resynchronization marker with low false-positive probability. |
| Chunk kind | `u8` | Event, source blob, index, summary, footer, etc. |
| Codec | `u8` | None, zlib, zstd, LZ4, reserved. |
| Flags | `u16 LE` | Required/optional semantics and reset flags. |
| Sequence | `u64 LE` | Monotonic chunk sequence. |
| First logical event sequence | `u64 LE` | Canonical ordering anchor. |
| Logical event count | `u32/u64` | Exact expansion count. |
| Uncompressed length | `u32/u64` | Validate allocation and decode. |
| Compressed length | `u32/u64` | Bounds and skipping. |
| Payload checksum | `u32/u64` | CRC32C initially; stronger optional hash may follow. |

Required properties:

- no chunk allocation before length limits are checked;
- dictionaries and delta state have explicit reset/snapshot rules;
- a complete chunk is independently verifiable;
- unknown optional chunk kinds can be skipped;
- unknown required chunk kinds fail cleanly;
- event sequence numbers detect missing/duplicated chunks.

## 5. Primitive encodings

### 5.1 Unsigned integers

Use a documented canonical ULEB128-like encoding. Reject non-canonical overlong encodings in strict mode to reduce ambiguity and fuzzing state space.

### 5.2 Signed integers

Use ZigZag + ULEB128 or canonical SLEB128. Choose through a benchmark/complexity ADR. Timing values must allow negative values because overhead subtraction or clock anomalies may expose them even if uncommon.

### 5.3 Fixed-width values

Use little-endian only for fixed header fields and checksums. Do not write native C/Perl memory layouts.

### 5.4 Strings

Represent strings as:

```text
string_id
byte_length
flags (UTF-8 validity/semantic flag, optional content class)
bytes
```

Definitions precede logical use. Decoders expose original bytes plus the same UTF-8 semantic flag expected by Perl callbacks.

### 5.5 Tick values

Canonical type: signed 64-bit integer tick count plus file-level `ticks_per_sec`.

- Statement elapsed ticks: signed `i64`.
- Call inclusive/exclusive ticks: signed `i64`.
- Aggregated caller inclusive/exclusive/recursive ticks: signed `i64` or checked wider accumulator during aggregation.
- Counts: `u64` internally, with compatibility checks when projecting to v5/Perl scalars.
- Depths and IDs: `u32` or `u64` wire varints with configured limits.

## 6. Logical event encoding

The wire opcode set may differ from v5 tags, but the decoder must yield the canonical logical events.

### 6.1 Metadata events

- `ATTRIBUTE(key_id, value)`
- `OPTION(key_id, value)`
- `COMMENT(string_id)`
- `PROCESS_START(pid, ppid, timestamp)`
- `PROCESS_END(pid, timestamp)`

Timestamps require a portable representation. A fixed integer nanosecond or seconds+nanos form is preferred over native floating `NV`.

### 6.2 File and source events

- `FILE_DEF(fid, eval_fid, eval_line, flags, size, mtime, path_id)`
- `SOURCE_BLOB_DEF(blob_id, encoding_flags, bytes)`
- `SOURCE_LINE_REF(fid, line, blob_id, offset, length, semantic_utf8_flag)`
- `SUB_DEF(fid, first_line, last_line, sub_id)`

Source blobs may combine contiguous lines or whole files. The logical decoder must emit `SRC_LINE` events in original order when compatibility callbacks are requested.

### 6.3 Timing events

- `TIME_LINE(ticks, fid_delta, line_delta)`
- `TIME_BLOCK(ticks, fid_delta, line_delta, block_delta, sub_delta)`
- `DISCOUNT`

A packed run opcode is permitted only when it expands to exactly the same ordered event list. Example:

```text
TIME_LINE_RUN(location, N, ticks[0..N-1])
```

This is valid only for consecutive logical `TIME_LINE` events at the same location with no hidden event between them. The run retains every tick value; storing only sum/count is not valid for the authoritative stream.

### 6.4 Call events

- `CALL_ENTRY(caller_fid_delta, caller_line_delta)`
- `CALL_RETURN(depth_delta, sub_id, inclusive_ticks, exclusive_ticks)`
- `SUB_CALLERS(fid, line, caller_sub_id, count, inclusive_ticks, exclusive_ticks, recursive_ticks, max_depth, called_sub_id)`

Subroutine names are dictionary references. The canonical decoder reconstructs original strings.

## 7. Dictionary strategy

Separate dictionaries may be used for:

- generic metadata strings;
- file paths;
- subroutine names;
- source blobs.

Evaluation criteria:

- hot-path lookup cost;
- memory bound and eviction policy;
- deterministic ID allocation;
- fork behavior;
- ability to recover after chunk boundaries;
- compression interaction;
- impact on tiny profiles.

The first implementation should use monotonic dictionaries without eviction. Add bounded/reset dictionaries only if real profiles demonstrate a memory problem and exact decoding remains straightforward.

## 8. Delta state

Maintain per-stream previous values for:

- file ID;
- line;
- block line;
- sub line;
- call depth;
- caller file/line;
- optional event timestamp baselines.

Chunk headers must state whether delta state resets. A reader starting from an indexed chunk must have either a reset or a state snapshot.

## 9. Compression strategy

The format identifies codecs; the implementation chooses defaults through benchmarks.

Initial support order:

1. `none` — debugging, deterministic tests, tiny profiles.
2. `zlib` — compatibility with an already available dependency.
3. `zstd` — candidate fast/default v6 codec if build and performance gates pass.
4. `lz4` — optional candidate for maximum collection speed.

Do not make zstd/LZ4 mandatory until packaging and old-platform implications are resolved. Codec selection must be explicit in metadata and errors.

## 10. Optional derived sections

Derived sections can accelerate standard reports without replacing raw events:

- line/block/sub totals;
- sub-call aggregate tables;
- file/source index;
- call-stream chunk index;
- per-file report metadata;
- bloom/filter maps for quick feature detection.

Every derived section must contain:

- source event sequence range;
- schema/version;
- checksum;
- enough metadata to decide whether it is valid;
- a way for readers to ignore and recompute it.

A report generated using summaries must be regression-tested against one generated by replaying raw events.

## 11. Conversion rules

### 11.1 v5 → v6

- preserve event order;
- preserve v5 `NV` bit-pattern-derived values as exactly as the supported decoder can interpret them;
- convert seconds-based aggregate fields back to ticks only when exact reconstruction is defined; otherwise preserve a typed legacy numeric event extension and mark it for compatibility projection;
- retain all options, attributes, comments, source, and process events.

This area requires careful investigation because v5 mixes tick and seconds representations.

### 11.2 v6 → v5

- succeed only when all fields fit v5 ranges and supported native `NV` representation;
- provide a strict default that errors on overflow or unrepresentable values;
- any `--allow-lossy` mode must be explicit, noisy, and excluded from regression oracles;
- preserve logical ordering and compression option where possible.

## 12. Versioning rules

- Major version changes when an old reader cannot safely parse the stream.
- Minor version adds optional chunk types, optional fields, or codecs with feature negotiation.
- Required feature bits prevent silent misinterpretation.
- Event and chunk namespaces reserve ranges for experimental/vendor use without colliding with standard IDs.
- Test vectors are versioned and immutable.

## 13. Format tasks

### FMT-001 — Write the normative v5 semantic specification

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BASE-002, BASE-003, COMPAT-001
- **Agent:** format engineer
- **Work:** Reverse-engineer every v5 tag, field encoding, state transition, unit, native-width value, callback, ordering rule, and malformed/truncated behavior from pinned source plus executable fixtures.
- **Deliverables:** complete event grammar, state machine, units, callback mapping, and edge cases.
- **Acceptance:** independent decoder implementation can pass the canonical v5 corpus.

### FMT-002 — Draft and approve v6 header/chunk specification

- **Status:** proposed
- **Size:** L
- **Dependencies:** ARCH-006, FMT-001
- **Agent:** binary-format architect
- **Work:** Specify magic/version prelude, endianness, header TLVs, process/run framing, chunk headers, feature negotiation, event sections, optional sections, finalization, and forward-compatibility behavior.
- **Deliverables:** byte-level tables, required/optional feature rules, sample hex dumps.
- **Acceptance:** two independent prototype decoders agree on test vectors.

### FMT-003 — Specify canonical varints and signed encoding

- **Status:** proposed
- **Size:** M
- **Dependencies:** FMT-002
- **Agent:** low-level encoding engineer
- **Work:** Select and specify canonical unsigned and signed varints, legal widths, overflow checks, overlong rejection, zig-zag or alternative mapping, and cross-language reference algorithms.
- **Deliverables:** normative byte tables, canonical/overlong rules, C/Rust reference routines, and cross-language boundary vectors.
- **Acceptance:** exhaustive boundary tests, malformed/overlong rejection tests, and cross-language vectors exist.

### FMT-004 — Specify integer tick model and overflow behavior

- **Status:** proposed
- **Size:** L
- **Dependencies:** COMPAT-003, BASE-003
- **Agent:** numerical systems engineer
- **Work:** Inventory all clock/tick domains and current floating conversions; define portable integer units, signedness/width, checked arithmetic, overflow behavior, and exact v5 projection rules.
- **Deliverables:** tick/clock schema, arithmetic and conversion rules, representability matrix, and fake-clock boundary vectors.
- **Acceptance:** all current clock backends and timing fields have exact mappings and checked arithmetic rules.

### FMT-005 — Design string/subroutine dictionaries

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-002, BENCH-002
- **Agent:** format/performance engineer
- **Work:** Prototype dictionary scopes and reset policies for names, paths, strings, and metadata; define byte equality, deterministic IDs, memory bounds, fork/chunk behavior, and adversarial cases.
- **Deliverables:** dictionary scope/lifecycle specification, bounded-memory design, collision/equality rules, and benchmark prototype.
- **Acceptance:** lookup, ID allocation, fork/reset, memory, and determinism policies are documented and benchmarked.

### FMT-006 — Design location and depth delta encoding

- **Status:** proposed
- **Size:** M
- **Dependencies:** FMT-003, BASE-007
- **Agent:** compression engineer
- **Work:** Measure file/line/depth locality; define exact signed deltas, bases, reset points, escape forms, chunk independence, and malformed-state rejection without changing logical event order.
- **Deliverables:** delta-state specification, reset rules, encoder/decoder prototype, property tests, and measured size results.
- **Acceptance:** canonical expansion tests prove exact event reconstruction; size benefit is measured.

### FMT-007 — Evaluate reversible run encodings

- **Status:** proposed
- **Size:** M
- **Dependencies:** COMPAT-001, BASE-007
- **Agent:** compression engineer
- **Deliverables:** candidate encoding definitions, exact expansion tests, collector/decode/size benchmarks, and adopt/reject ADR.
- **Work:** test same-location tick vectors and repeated metadata patterns without summing away events.
- **Acceptance:** adopt only encodings with measurable end-to-end benefit and simple exact decoding.

### FMT-008 — Design source blob and dedup representation

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-002, COMPAT-001
- **Agent:** source/eval specialist
- **Work:** Separate logical source identity from content blobs; specify exact bytes/flags, hashes and collision checks, dedup eligibility, eval/parent relationships, merge behavior, and privacy implications.
- **Deliverables:** logical-source versus blob schema, hash/collision rules, fork/merge behavior, and byte-exact source fixtures.
- **Acceptance:** source bytes, line mapping, UTF-8 flags, eval identity, and callback order round-trip exactly.

### FMT-009 — Implement codec-neutral chunk API

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-002
- **Agent:** C/Rust systems engineer
- **Work:** Define and implement codec-neutral framing/stream interfaces, required none/zlib support, optional codec registration, bounded decompression contracts, and capability negotiation.
- **Deliverables:** common chunk/codec interfaces in C and Rust, none/zlib implementations, capability registry hooks, and vectors.
- **Acceptance:** none/zlib codecs share framing; unsupported codec errors are deterministic.

### FMT-010 — Implement chunk checksums and truncation recovery

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-002, SEC-003
- **Agent:** reliability engineer
- **Work:** Choose checksum coverage and algorithms through security/performance evidence; specify strict validation, complete-chunk salvage, sequence gaps, diagnostics, and partial-file finalization semantics.
- **Deliverables:** checksum coverage specification, strict/salvage decoder behavior, corruption matrix, and recovery vectors.
- **Acceptance:** corruption is localized; salvage returns only complete verified chunks and reports missing event ranges.

### FMT-011 — Define optional index and summary schemas

- **Status:** proposed
- **Size:** L
- **Dependencies:** ARCH-004, RUST-006
- **Agent:** data-model engineer
- **Work:** Define optional offset indexes and exact derived summaries with source-range/provenance coverage; require independent validation or rebuild and make raw ordered events authoritative.
- **Deliverables:** index/summary schemas, coverage/provenance hash rules, trust/rebuild policy, and raw-versus-summary equality tests.
- **Acceptance:** raw-event replay and summary-based aggregation produce equal canonical aggregates.

### FMT-012 — Build immutable v6 test vectors

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-002 through FMT-010
- **Agent:** test engineer
- **Work:** Generate immutable positive/boundary/unknown-feature/corrupt vectors from the normative spec using independent implementations; publish expected canonical events and failure diagnostics.
- **Deliverables:** minimal, maximal, Unicode, eval, fork, call-heavy, corruption, and unknown-feature vectors.
- **Acceptance:** C and Rust implementations pass identical vectors.

### FMT-013 — Specify strict v5↔v6 conversion

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-001, FMT-004, TOOL-001
- **Agent:** compatibility engineer
- **Work:** Map every v5 and v6 field/event bidirectionally; define target-v5 representability checks, integer/NV conversion, unsupported-feature refusal, provenance, and canonical round-trip tests.
- **Deliverables:** field-by-field conversion map, target-v5 representability checker rules, diagnostics, and round-trip fixtures.
- **Acceptance:** every conversion failure mode is explicit; representable fixtures round-trip canonically.

### FMT-014 — Add format inspection metadata

- **Status:** proposed
- **Size:** S
- **Dependencies:** FMT-002
- **Agent:** tooling engineer
- **Work:** Define bounded metadata needed for fast inspection—format, features, codecs, chunks, process/run identity, counts, checksums, source/index presence—and text/JSON stability rules.
- **Deliverables:** inspect metadata schema, bounded fast-scan API, text/JSON examples, and unknown-feature behavior tests.
- **Acceptance:** `inspect` can report format, features, codecs, chunks, event counts, and checksums without full aggregation.

### FMT-015 — Benchmark codecs and chunk sizes on NYTProf data

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-007, BENCH-001
- **Agent:** performance engineer
- **Work:** Benchmark codec, level, chunk-size, dictionary, delta, exact-run, and source-dedup combinations on representative event distributions; include encode/decode CPU, RSS, size, random access, recovery, dependency, and platform costs.
- **Deliverables:** raw benchmark matrix, selected-default recommendation/ADR, platform/build analysis, and rejected alternatives.
- **Matrix:** codec level, 16/64/256/1024 KiB chunks, dictionary on/off, delta on/off, call-heavy/statement-heavy/source-heavy profiles.
- **Acceptance:** default recommendation includes collector CPU, reader CPU, size, RSS, truncation cost, and build implications.

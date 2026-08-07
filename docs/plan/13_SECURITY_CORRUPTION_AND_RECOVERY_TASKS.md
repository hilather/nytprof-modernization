# Security, Corruption, Resource Limits, and Recovery Task Plan

## 1. Threat model

Profile files can contain attacker-controlled or corrupted:

- lengths, varints, IDs, depths, counts, ticks, chunk sequences, and checksums;
- compressed data with extreme expansion ratios;
- paths, subroutine names, attributes, comments, and source bytes;
- HTML/script-like source content;
- malformed native-NV metadata;
- optional index/summary data designed to produce wrong reports;
- mixed profiles used by converters/merge;
- incomplete data from crashes, ENOSPC, killed processes, or fork races.

Report tools may run in developer/build environments with access to source trees and browsers. Treat input as untrusted unless the caller explicitly establishes trust.

## 2. Security invariants

1. Check every size and arithmetic operation before allocation/indexing.
2. Apply configurable and safe default limits to decoded and output resources.
3. Never trust compressed declared sizes or optional summaries without validation.
4. Never interpret unknown native floating layouts by guessing.
5. Never allow profile paths to escape the selected report output directory.
6. Escape all profile/source text for its output context.
7. Do not invoke a shell with profile-provided strings.
8. Do not cross FFI with panic/unwind or invalid ownership.
9. Preserve only complete verified v6 chunks during salvage.
10. Clearly mark incomplete/recovered profiles and missing event ranges.
11. Keep profile/source file permissions conservative because embedded source can be sensitive.
12. Do not hide corruption through automatic legacy fallback.

## 3. Limits model

Define limits for:

- profile/compressed/uncompressed bytes;
- chunk/header/TLV sizes;
- event count;
- dictionary entries and total string bytes;
- individual string/source line/blob size;
- files, subs, lines, call sites, runs, processes;
- call depth and recursion;
- output files and total report bytes;
- decompression ratio/work;
- worker count and memory budget;
- merge input count and aggregate widths.

Limits should be enforceable in library and CLI APIs, visible in diagnostics, and overridable by informed users.

## 4. Checksums and authenticity

Per-chunk checksums detect accidental corruption, not malicious forgery. The format should say so. Optional content hashes can support dedup/provenance. Cryptographic signatures are outside initial scope unless a deployment requires authenticity.

A checksum failure invalidates the chunk. A summary/index is valid only if its schema and source profile/range hash match.

## 5. Recovery policy

### V6

- scan only according to validated framing/sync rules;
- accept complete chunks with valid sequence/checksum/state snapshot;
- report gaps and dependent chunks that cannot be decoded;
- never fabricate missing events;
- emit a recovery manifest and incomplete marker;
- preserve original file unchanged unless output path explicitly provided.

### V5

V5 has fewer recovery boundaries. Match or improve legacy behavior, but label uncertainty. Do not claim that decompressed trailing data is complete without an end marker/state validation.

## 6. Security/reliability tasks

### SEC-001 - Define threat model and global resource-limit API

- **Status:** proposed
- **Size:** L
- **Dependencies:** ARCH-003, BUILD-001
- **Agent:** security architect
- **Work:** trust boundaries, defaults, library/CLI config, error categories, platform considerations.
- **Deliverables:** threat model and limit schema.
- **Acceptance:** every parser/model/report allocation maps to a checked limit or documented bounded source.

### SEC-002 - Establish continuous fuzzing strategy

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-003, BUILD-009
- **Agent:** security fuzz engineer
- **Work:** targets for v5/v6, codecs, conversion, merge, report escaping, FFI; corpus retention; budgets; crash triage.
- **Deliverables:** fuzz plan and jobs.
- **Acceptance:** critical parsers are covered by PR smoke and scheduled deep fuzzing.

### SEC-003 - Specify checksum, framing, and salvage rules

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-002, SEC-001
- **Agent:** binary reliability architect
- **Work:** checksum algorithm, coverage, sync false positives, sequence gaps, dictionary/delta dependencies, incomplete status.
- **Deliverables:** normative recovery section and vectors.
- **Acceptance:** independent implementations agree on every corruption fixture outcome.

### SEC-004 - Harden decompression and codec boundaries

- **Status:** proposed
- **Size:** L
- **Dependencies:** SEC-001, FMT-009, BUILD-008
- **Agent:** compression security engineer
- **Work:** expansion/work limits, streaming APIs, malformed frames, library error mapping, version/advisory policy.
- **Deliverables:** bounded codec adapters and tests.
- **Acceptance:** decompression-bomb corpus fails within resource limits; no partial unverified payload is consumed.

### SEC-005 - Audit integer arithmetic and indexing

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-003, RUST-006, FMT-004
- **Agent:** secure numerical engineer
- **Work:** checked add/mul/conversion, varint widths, offsets, counts, aggregate overflow, C casts, signed/unsigned boundaries.
- **Deliverables:** audit checklist, static assertions, boundary tests.
- **Acceptance:** no unchecked untrusted arithmetic reaches allocation or indexing; sanitizers/fuzz pass.

### SEC-006 - Harden HTML/source rendering

- **Status:** proposed
- **Size:** L
- **Dependencies:** REPORT-005, SEC-001
- **Agent:** application security engineer
- **Work:** context-specific escaping for text/attributes/URLs/JS/JSON/CSS, source bytes/UTF-8, CSP feasibility, unsafe template callbacks.
- **Deliverables:** escaping library and malicious-source fixtures.
- **Acceptance:** browser tests show no script execution or markup injection from profile content.

### SEC-007 - Harden filesystem paths and atomic output

- **Status:** proposed
- **Size:** L
- **Dependencies:** REPORT-011, TOOL-001, SEC-001
- **Agent:** filesystem security engineer
- **Work:** traversal, absolute/UNC paths, symlinks, race-resistant temp files, permissions, overwrite policy, cross-device rename, cleanup.
- **Deliverables:** safe output/path module.
- **Acceptance:** malicious path corpus cannot write outside output root or overwrite unrelated files.

### SEC-008 - Audit C/Rust/Perl FFI and unsafe code

- **Status:** proposed
- **Size:** XL
- **Dependencies:** RUST-010, PERL-011, BUILD-009
- **Agent:** senior memory-safety reviewer
- **Work:** ownership, lengths, lifetimes, thread/interpreter affinity, panic containment, callback exceptions, double close, ABI mismatch.
- **Deliverables:** documented unsafe invariants and review report.
- **Acceptance:** all unsafe blocks have rationale/tests; sanitizer/valgrind/stress suites pass.

### SEC-009 - Validate optional summary/index trust

- **Status:** proposed
- **Size:** M
- **Dependencies:** RUST-014, TOOL-008
- **Agent:** data integrity engineer
- **Work:** profile hash/range/schema validation, corruption/staleness, recompute fallback, cache permissions.
- **Deliverables:** validation policy/tests.
- **Acceptance:** modified summary never changes report values without detection.

### SEC-010 - Harden conversion and merge against hostile inputs

- **Status:** proposed
- **Size:** L
- **Dependencies:** TOOL-004, TOOL-005, TOOL-009, SEC-001
- **Agent:** security tooling engineer
- **Work:** many inputs, conflicting metadata, source/name bombs, deep calls, overflows, temp storage, output cleanup.
- **Deliverables:** hostile corpus and limit integration.
- **Acceptance:** bounded deterministic failure; no output presented as valid after partial conversion/merge.

### SEC-011 - Protect sensitive embedded source and metadata

- **Status:** proposed
- **Size:** M
- **Dependencies:** SEC-007, BUILD-001
- **Agent:** security/privacy engineer
- **Work:** default file permissions, temp files, report permissions, diagnostics that may print source/paths, provenance redaction controls without altering profile semantics.
- **Deliverables:** handling policy and tests.
- **Acceptance:** outputs are not made more permissive than legacy defaults without explicit opt-in; temporary sensitive files are cleaned safely.

### SEC-012 - Complete security/reliability release review

- **Status:** proposed
- **Size:** L
- **Dependencies:** SEC-001 through SEC-011, TEST-014 through TEST-019
- **Agent:** independent security reviewer
- **Work:** threat-model review, fuzz/sanitizer results, dependency advisories, recovery drills, unresolved risks.
- **Deliverables:** signed review and issue list.
- **Acceptance:** no unresolved critical/high issue; accepted residual risks have owner and release note.

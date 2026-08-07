# 16 - Acceptance Criteria and Definition of Done

## Purpose

Define objective completion criteria at task, workstream, phase, release, and project levels. A task is not done because code compiles or a benchmark improves. It is done when its specified artifact, compatibility, security, test, performance, documentation, and handoff obligations are complete.

## Task-level definition of done

Every implementation task must satisfy all applicable items:

### Scope and traceability

- Task ID is referenced in code review and changelog/work log.
- Dependencies are complete or explicitly waived by approved ADR.
- Changed compatibility-contract clauses, event/format fields, APIs, commands, reports, and platforms are identified.
- No unrelated generated/golden changes are bundled without explanation.

### Implementation

- Code follows approved component boundaries.
- Error and resource-limit paths are implemented, not left as TODOs.
- C ownership/lifetime and Rust `unsafe` invariants are documented.
- Common collector event path does not allocate or cross language boundaries unless the task explicitly proves and approves it.
- Stable IDs/schema/wire changes update the normative spec and generated consistency checks.

### Tests

- Positive, boundary, negative, malformed, truncation, and platform tests are present where applicable.
- Differential tests compare against the correct oracle.
- Seeded semantic mutations prove the comparator/gate is meaningful.
- New bug fixes add minimized permanent regression fixtures.
- Required old-reader/new-writer or old-writer/new-reader cells pass.
- Tests run against installed artifacts when packaging behavior is involved.

### Exactness and compatibility

- Canonical event stream is equal where the task touches collection/format/streaming.
- Canonical aggregates are equal where the task touches model/merge/reports.
- Public Perl object/method/callback behavior passes where applicable.
- CLI/report semantic manifests pass where applicable.
- No precision reduction, event omission, source-byte change, or call relationship loss.
- Any accepted difference has an approved ADR and user-facing note.

### Performance

- Baseline and candidate use equivalent feature/options and pass correctness P0.
- Raw measurements and provenance are retained.
- Collector/report/model/storage impact is reported even if expected neutral.
- No workload-specific regression exceeds its gate without approved exception.
- Speculative optimizations without measured benefit are removed or documented as required groundwork.

### Security and reliability

- Untrusted sizes/offsets/arithmetic are bounded and checked.
- Failure cannot produce a falsely complete/valid output.
- Relevant fuzz/sanitizer/fault-injection tests pass.
- Report/path/escaping/native-loading implications are reviewed.
- Temporary/output cleanup and atomicity are tested where applicable.

### Documentation and handoff

- Public/internal API/spec docs are updated.
- Build/runtime capability and failure behavior are documented.
- Handoff includes commands, exact versions/paths, artifacts/checksums, results, limitations, and next consumers.
- Task status is updated in `TASK_INDEX.md` (and `TASK_INDEX.json` when automation is used).

## Workstream-level acceptance

### Architecture/contracts

- Every v5 record/callback/API/report field is mapped or explicitly classified.
- Timing/call semantics are executable specifications, not prose only.
- ADR queue has no unresolved blocker for implementation being promoted.
- Specs and machine-readable schemas agree.

### Collector/C/XS

- Neutral v5 refactor passes byte/semantic and performance gates.
- Fake-clock suite proves exact attribution.
- v5 remains readable by unmodified v6.15 tools.
- v6 dual output equals v5 canonical stream exactly.
- Fork/source/calls/slow-op/lifecycle/incomplete/fault behavior passes.
- Hot path meets no-allocation/no-mandatory-FFI requirement.
- Independent C audit is signed.

### v6 format

- Normative spec, versioning rules, limits, recovery semantics, and extension rules are approved.
- C and Rust implementations agree on immutable vectors.
- Encode/decode properties and corruption tests pass.
- Storage/codec/chunk decisions have NYTProf-specific benchmark evidence.
- Raw ordered events remain present; summaries/indexes are additive and verifiable.

### Rust core/model

- v5/v6 event parity and aggregate parity pass.
- Parser resource limits/fuzzing/security audit pass.
- Compact model supports every required report/API field.
- Stable C ABI contains panic/ownership failures.
- RSS/time gates are certified.

### Perl/API compatibility

- Required packages, methods, callbacks, object shapes, options, errors, and contexts pass dual-engine tests.
- Engine selection/fallback is deterministic and explicit.
- Legacy-only supported tiers remain useful.
- Downstream smoke suite passes or approved exceptions exist.
- Compatibility sign-off is signed.

### Reports

- Full artifact catalog is implemented.
- Exact semantic manifest, normalized DOM, links/anchors, source, CSV, Callgrind, graph input, and call/flame data pass.
- Parallel output is deterministic and atomically published.
- Escaping/path security passes.
- Time/RSS/output-size measurements are certified.
- Legacy renderer remains force-selectable during rollout.

### Tools/conversion/merge

- Inspect/verify/dump/convert/merge/repack/salvage commands have versioned machine output.
- Successful conversions/repack have equal canonical hashes.
- v6-to-v5 refuses unrepresentable data before publish.
- Old v6.15 tools accept successful new v5 outputs.
- Mixed merge/recovery semantics pass.
- CLI/tooling sign-off is signed.

### Build/package/CI

- Platform tiers and native availability are explicit.
- Legacy-only and native installed-artifact tests pass.
- Offline/source/prebuilt policy is implemented.
- ABI/library/codec loading is validated.
- Release artifacts include provenance, checksums, dependency/license/SBOM data.
- Tier-1 matrix and package sign-off pass.

### Security/recovery

- Threat model is complete and current.
- Fuzz-hour, sanitizer, resource, corruption, path, escaping, loading, and supply-chain gates pass.
- No unresolved critical/high severity issue.
- Strict/incomplete/corrupt/unsupported/salvaged states are unambiguous.
- Security certification is signed.

### Benchmarking

- Noise study and ratified gates exist.
- Correctness P0 qualifies every performance result.
- Raw data is reproducible and workload-specific regressions are visible.
- Public claims are limited to certified configurations.
- Independent performance certification is signed.

## Release levels

### Level R0 - Developer preview

Required:

- explicit experimental flag/build;
- no default changes;
- basic unit/vector/differential tests;
- known limitations published;
- not recommended for production data without paired v5 output;
- legacy path fully available.

### Level R1 - Native v5 report opt-in

Required:

- v5 reader/model/API/report parity for advertised outputs;
- report security and atomicity;
- native packaging on advertised tiers;
- legacy fallback;
- P3/P4 results reported;
- regression/security sign-off scoped to native v5 reporting.

### Level R2 - v6 collection opt-in

Required:

- stable v6 spec/vectors;
- same-run dual exact equality;
- collector/fork/source/call/lifecycle audits;
- strict conversion, verify, inspect, recovery;
- P1/P2 results;
- old-tool interoperability through v5 mode/conversion;
- full scoped regression/security/platform certification.

### Level R3 - Native reporting preferred by `auto`

Required:

- successful R1 field window;
- full report/API/CLI compatibility and downstream suite;
- fallback/rollback telemetry and documentation;
- no unresolved high-severity issue;
- default-change ADR and release certification.

### Level R4 - v6 output default on eligible tiers

Required:

- successful R2 field window;
- stable format with no anticipated incompatible redesign;
- repeated performance/reliability certification;
- migration/conversion/old-tool workflow proven in released versions;
- legacy-only tier behavior defined;
- `format=v5` retained for documented compatibility window;
- default-change ADR and rollback plan.

### Level R5 - Legacy retirement consideration

Not implied by R4. Each legacy component requires separate deprecation/removal decision, usage evidence, support window, and migration path.

## Project-level success criteria

The modernization project is considered successful when all of the following hold for the accepted platform/configuration scope:

### Exactness

- No sampling or event omission was introduced.
- v6 retains the exact ordered logical event stream.
- Integer tick representation preserves collector precision.
- Call, source, process, statement, block, slow-op, metadata, and incomplete-stream semantics match the compatibility contract.

### Backward compatibility

- New collector can write v5 accepted by unmodified v6.15 tools.
- New engine reads supported old v5 profiles.
- v6 converts to v5 exactly when representable and refuses otherwise.
- Existing Perl/CLI/report workflows remain supported during the compatibility window.
- Legacy engine/fallback remains available according to release policy.

### Storage

- Ratified P2 profile-size target is met across the representative corpus or explicit workload exceptions are approved.
- Reductions come from reversible dictionaries/deltas/chunks/compression/source dedup, not lost information.
- Report output duplication is reduced only in compatibility-safe or opt-in modes.

### Runtime and memory

- Collector meets ratified P1 without precision/feature changes.
- Native read/model meets P3.
- Native reports meet P4 and deterministic parallelism.
- Tooling scales within defined resource budgets.

### Quality and operability

- Format/spec/test vectors are sufficient for independent implementations.
- Parser and collector security audits pass.
- Inspect/verify/convert/merge/repack/salvage tools are available.
- Build/package/platform behavior is explicit and reproducible.
- Regression/performance/security certificates accompany default promotions.

## Release-blocker categories

The following are blockers unless explicitly scoped out of the advertised feature/platform:

- any canonical event mismatch;
- timing/count/source/call/process semantic mismatch;
- old-reader rejection of required new v5 output;
- silent lossy conversion or merge;
- public API/callback/object/CLI/report semantic regression;
- corruption accepted as complete;
- unbounded parser allocation/decompression;
- memory safety/UB/panic across FFI;
- report injection/path escape;
- non-deterministic semantic output;
- unsupported native dependency with no promised fallback;
- performance gate failure on a critical workload without approved exception;
- missing artifact provenance or unreproducible oracle.

## Waiver policy

A waiver must include:

```text
blocker/criterion
scope and affected users/platforms
reproduction and evidence
why fixing before release is disproportionate
compatibility/security/data-loss analysis
workaround and documentation
owner and expiration/review release
approved ADR/reference
```

Precision loss, silent event loss, silent lossy conversion, or unresolved critical/high security issues are not waivable within this project scope.

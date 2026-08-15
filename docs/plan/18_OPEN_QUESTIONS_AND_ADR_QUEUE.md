# 18 - Open Questions and ADR Queue

## Purpose

Record decisions that affect stable semantics, wire bytes, platform support, packaging, compatibility, security, or defaults. Agents must not settle these independently inside implementation patches.

## ADR process

1. Open a question with evidence and affected task IDs.
2. Build the smallest experiment/fixture needed.
3. Compare alternatives across correctness, compatibility, collector cost, storage, decode/report cost, security, portability, and operational complexity.
4. Record decision, rejected alternatives, consequences, migration/versioning, and revisit trigger.
5. Update normative specs and tests before implementation is treated as stable.

## Decision status

- `open` - evidence incomplete.
- `proposed` - recommendation ready for review.
- `accepted` - binding; linked ADR exists.
- `superseded` - replaced by later ADR.
- `deferred` - not required for current release level.

## Blocking questions

### ADR-Q001 - Exact canonical event taxonomy

- **Status:** open
- **Blocks:** COMPAT-001, ARCH-001, FMT-001, COL-001
- **Question:** What is the complete semantic event set, including records exposed only as aggregate metadata or comments?
- **Evidence required:** v6.15 writer/tag inventory, `ReadStream` callbacks, merge/calls consumers, option matrix, incomplete files.
- **Recommended direction:** one canonical semantic event per observable v5 callback/record meaning; retain opaque optional extension events for round-trip where necessary.
- **Decision must specify:** fields, order, byte/numeric domains, process association, required/optional status, unknown behavior.

### ADR-Q002 - Tick signedness, width, and overflow policy

- **Status:** open
- **Blocks:** FMT-004, COL-011, COMPAT-003
- **Question:** Use signed i64, unsigned u64, wider software integer, or another representation for durations/totals?
- **Evidence required:** clock APIs, legacy arithmetic, long-run bounds, negative/anomaly behavior, platform support, v5 conversion.
- **Recommended direction:** signed 64-bit event ticks with checked arithmetic and explicit clock metadata unless evidence requires wider representation; aggregate accumulators may use wider checked types internally.
- **Decision must specify:** wire type, invalid/negative behavior, overflow failure, seconds conversion, merge domains.

### ADR-Q003 - v6 prelude, magic, and version negotiation

- **Status:** open
- **Blocks:** FMT-002, RUST-011, COL-007, COL-008
- **Question:** Exact magic bytes, major/minor/version layout, feature flags, and required/skippable extension rules?
- **Evidence required:** robust detection, streaming, future extensions, old-tool failure behavior, corruption recognition.
- **Recommended direction:** fixed endian-neutral magic/prelude, major/minor, header length, required/optional feature bitsets plus TLVs.
- **Decision must specify:** compatibility rules and when a major versus minor bump is required.

### ADR-Q004 - Canonical varint and signed-delta encoding

- **Status:** open
- **Blocks:** FMT-003, COL-012, RUST-011
- **Question:** LEB128, existing v5-style varint, prefix varint, or another canonical encoding?
- **Evidence required:** C/Rust speed, code size, canonical rejection of overlong forms, small-value distribution, fuzz/security complexity.
- **Recommended direction:** standard unsigned LEB128 with ZigZag for signed deltas and mandatory shortest representation, unless benchmarks justify otherwise.
- **Decision must specify:** maximum bytes, overlong handling, overflow, test vectors.

### ADR-Q005 - Dictionary scope and reset policy

- **Status:** partially accepted (FOOTER-local intent frozen); residual open for global/cross-file
- **Blocks:** FMT-005, COL-010, RUST-011
- **Accepted binding ADR:** [`docs/adrs/0002-v6-string-pool-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0002-v6-string-pool-candidate.md) (**accepted** OQ-1) — FOOTER-local, single-profile string dictionary intent for COL-007 dict emit + E3 dict cases.
- **Question (remaining open):** Global / cross-file / process-lifetime intern pool (COL-010 class), fork/reset inheritance across profiles, and multi-FOOTER policy beyond FOOTER-local.
- **Evidence required (residual):** hit rate, collector memory/CPU, chunk independence, recovery, fork/high-cardinality workloads for any **global** pool.
- **Recommended direction (residual):** keep FOOTER-local as product baseline until COL-010 evidence; do **not** re-litigate FOOTER-local without superseding ADR-0002.
- **Decision must specify (residual):** global definition ordering, reset, maximum entries/bytes, OOM/fallback, fork behavior — only if global pool is adopted later.

### ADR-Q006 - Reversible run/pattern records

- **Status:** accepted (packing intent frozen via ADR-0001); wire encode residual open
- **Blocks:** FMT-007, COL-012
- **Accepted binding ADR:** [`docs/adrs/0001-v6-event-body-packing-candidate.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0001-v6-event-body-packing-candidate.md) (**accepted** OQ-1) — site deltas, `FLAG_HAS_SEQ`, `TIME_LINE_RUN` / `TIME_BLOCK_RUN` expansion, multi-chunk/mid-stream packing continuity.
- **IDs:** opcodes 18/19 + flags `FLAG_SITE_DELTA`/`FLAG_HAS_SEQ` in [`V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md) — **frozen** for major=6 by [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md).
- **Question (closed for product intent):** Which repeated event patterns justify specialized records — answered by ADR-0001 (runs + site-delta + seq compose; absolute baseline retained).
- **Residual open:** E3-mixed multi-kind C fixtures; full oracle E4; convert tooling depth (not ID freeze).
- **Decision must specify (done in ADR-0001 + ADR-0006):** expansion semantics, continuity, limits; wire numeric freeze accepted after E3/E4-v0.

### ADR-Q007 - Source blob hashing and identity

- **Status:** open
- **Blocks:** FMT-008, COL-013
- **Question:** Which content hash, collision strategy, blob scope, and metadata represent source while preserving logical file/eval identity?
- **Evidence required:** source sizes, performance/security, fork/merge, identical content under distinct identities, binary bytes.
- **Recommended direction:** logical source events reference content-addressed blobs; full byte comparison verifies hash collisions; path/eval identity remains separate and ordered.
- **Decision must specify:** hash algorithm/version, collision handling, missing/external source, dedup scope.

### ADR-Q008 - Chunk size and boundary policy

- **Status:** open
- **Blocks:** FMT-009, FMT-010, FMT-015, COL-005, COL-007, COL-008, BENCH-007
- **Question:** Fixed uncompressed size, event-count threshold, time-based flush, lifecycle boundaries, or hybrid?
- **Evidence required:** collector latency/CPU, compression ratio, recovery granularity, parallel decode, memory, fork/shutdown.
- **Recommended direction:** bounded uncompressed byte target plus mandatory lifecycle/flush boundaries, with all state reset/snapshotted per spec.
- **Decision must specify:** default/limits, deterministic fixture mode, partial-final-chunk behavior.

### ADR-Q009 - v6 default codec and required codec set

- **Status:** open
- **Blocks:** FMT-009, FMT-015, COL-007, COL-008, BUILD-008
- **Question:** Should first stable v6 require only none/zlib, add zstd, add LZ4, or choose another default?
- **Evidence required:** NYTProf-specific collection CPU, size, decode speed, dependencies/licenses/platform availability/security.
- **Recommended direction:** format remains codec-neutral; first implementation supports none and zlib universally, with zstd/LZ4 only if packaging and benchmark evidence is strong. Default chosen after BENCH-006.
- **Decision must specify:** required decoder capabilities, producer default, fallback/error behavior.

### ADR-Q010 - Chunk checksum algorithm and coverage

- **Status:** open
- **Blocks:** FMT-010, SEC-003
- **Question:** CRC32C, XXH-style noncryptographic hash, cryptographic hash, or layered checks?
- **Evidence required:** corruption detection, speed/platform acceleration, dependency/security, source blob needs.
- **Recommended direction:** fast standardized per-chunk checksum over header-relevant fields plus uncompressed or compressed payload as explicitly specified; cryptographic source identity can be separate.
- **Decision must specify:** algorithm ID/version, covered bytes, footer/index protection, collision expectations.

### ADR-Q011 - Optional exact summary/index schemas and trust

- **Status:** open
- **Blocks:** FMT-011, RUST-014, TOOL-008
- **Question:** Which derived data is worth storing, and when may readers trust it?
- **Evidence required:** report startup gains, size, verification cost, incomplete files, stale/corrupt behavior.
- **Recommended direction:** optional footer index first; derived aggregate cache deferred or verified against raw event hash/sequence before use. Raw ordered events remain authoritative.
- **Decision must specify:** schema/version, coverage hash, validation, rebuild, skip rules.

### ADR-Q012 - Stable v6 byte determinism

- **Status:** open
- **Blocks:** FMT-012, RUST-011, COL-007, COL-008
- **Question:** Must production files be byte-deterministic for the same canonical stream, or only logical-deterministic?
- **Evidence required:** dictionary/chunk/compressor behavior, reproducible fixtures, merge/repack, performance cost.
- **Recommended direction:** deterministic uncompressed logical encoding and fixture mode are mandatory; production compressed bytes should be deterministic when codec/library permits but semantic hashes remain the compatibility contract.
- **Decision must specify:** required stable layers and allowed metadata differences.

### ADR-Q013 - Exotic v5 `NV` decoding strategy

- **Status:** open
- **Blocks:** RUST-005, TEST-017
- **Question:** Implement all declared `NV` representations in Rust, use a C compatibility shim, or fall back to legacy reader on rare tiers?
- **Evidence required:** actual supported distributions, fixture availability, format metadata sufficiency, safety/maintenance.
- **Recommended direction:** native Rust for common IEEE representations; isolated C shim or explicit legacy fallback for rare formats rather than unsafe guessing.
- **Decision must specify:** tier support and diagnostics.

### ADR-Q014 - v6-to-v5 representability rules

- **Status:** open
- **Blocks:** FMT-013, TOOL-005, COMPAT-003
- **Question:** Exact preconditions for lossless v5 output on a selected target Perl representation?
- **Evidence required:** every event/field, integer/NV/string limits, unknown extensions, clock conversion, incomplete semantics.
- **Recommended direction:** target-specific preflight; fail before output if any event cannot be represented exactly under the defined v5 semantic comparison.
- **Decision must specify:** target selection, exactness test, error context, provenance.

### ADR-Q015 - Collector sink dispatch mechanism

- **Status:** open
- **Blocks:** ARCH-001, COL-001, COL-004
- **Question:** Specialized direct functions, function-pointer vtable, tagged-union emit, compile-time writer specialization, or hybrid?
- **Evidence required:** assembly and event-heavy benchmarks, dual/test sink needs, maintainability.
- **Recommended direction:** semantic specialized functions with statically selected sink where possible; dual/test paths may use dispatch outside production default.
- **Decision must specify:** ABI/internal status and performance budget.

### ADR-Q016 - Native engine distribution model

- **Status:** open
- **Blocks:** BUILD-001, BUILD-003, BUILD-004, BUILD-012, COMPAT-011
- **Question:** Build Rust from source in CPAN distribution, ship optional companion, provide prebuilt artifacts, or a hybrid?
- **Evidence required:** platform/toolchain tiers, CPAN policies, offline installation, artifact provenance, maintenance cost.
- **Recommended direction:** hybrid with legacy-only fallback; exact mechanism chosen by platform evidence and release policy.
- **Decision must specify:** native availability guarantees and failure behavior.

### ADR-Q017 - Minimum supported Rust version and dependency policy

- **Status:** open
- **Blocks:** RUST-001, BUILD-001, BUILD-011
- **Question:** Which MSRV and vendoring/pinning/update policy balance portability/security?
- **Evidence required:** supported OS/toolchains, dependency MSRVs, source distribution constraints.
- **Recommended direction:** conservative explicitly tested MSRV with pinned lockfile and audited small dependency set.
- **Decision must specify:** update cadence and tier exceptions.

### ADR-Q018 - Native FFI versus subprocess for report operations

- **Status:** partial (full-R1 product disposition fixed; operation map open)
- **Blocks:** PERL-005, RUST-010, TOOL-010
- **Question:** Which operations run in-process through XS/FFI versus invoke a native CLI subprocess?
- **Evidence required:** API compatibility, startup overhead, failure isolation, packaging, old Perl, callback needs.
- **Recommended direction:** in-process coarse FFI for public Perl Data/ReadStream compatibility; CLI/subprocess acceptable for standalone report commands where it improves isolation and preserves behavior.
- **Decision must specify:** operation map and error/stream handling.
- **Full-R1 disposition (resolved by user OQ-2 / ADR-0003):** do **not** waive production FFI or XS Data/ReadStream for full R1. Close via **PR-A05** (`nytprof-ffi`) and **PR-A06** (XS Data / ReadStream). CLI subprocess remains the R0/R1-preview bridge and standalone report path. Normative: [`docs/adrs/0003-r1-full-residual-policy.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md).
- **Still open:** exact per-operation map (which report/query surfaces are in-process vs subprocess), error/stream handling, and packaging load rules for the dylib — decided when A05/A06 implement, not re-openable as “waive FFI/XS for full R1” without a superseding ADR.

### ADR-Q019 - Lazy Perl object materialization

- **Status:** open
- **Blocks:** PERL-005, PERL-006, TEST-010
- **Question:** Can legacy Data object graphs be lazily materialized without observable identity/mutation/error differences?
- **Evidence required:** object-shape/mutation/downstream tests and memory/time gains.
- **Recommended direction:** eager compatibility materialization first; introduce lazy behavior only after it is proven observationally equivalent or exposed as a new API.
- **Decision must specify:** fields/objects, cache/lifetime, mutation policy.

### ADR-Q020 - Report compatibility threshold

- **Status:** partial (full-R1 CLOSE/WAIVE map fixed; per-artifact comparison class open)
- **Blocks:** REPORT-001, REPORT-002, REPORT-009, TEST-009
- **Question:** Which HTML details require byte identity, normalized DOM identity, semantic identity, or may intentionally change?
- **Evidence required:** existing tests/downstream consumers, links/bookmarks/styles, user expectations.
- **Recommended direction:** exact data/filenames/anchors/links/source/order and normalized DOM semantics; byte identity only for machine formats or known consumers. Visual redesign is out of scope for default compatibility mode.
- **Decision must specify:** per-artifact comparison class.
- **Full-R1 disposition (resolved by ADR-0003 / PR-A04):** every HTML residual **class** is mapped to **CLOSE** (PR-A01–A03) or **WAIVE** (Graphviz, treemap, block/sub page modes, naming alias, presentation chrome, etc.). Semantic counts remain exact; full oracle DOM is not required for full R1. Normative map: [`docs/adrs/0003-r1-full-residual-policy.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) + inventory disposition column in [`REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md).
- **Still open:** finer comparison class per closed artifact (byte vs normalized DOM vs semantic-only) when each CLOSE PR lands; waived classes need no native comparison class until a superseding ADR re-opens them.

### ADR-Q021 - Default report worker count and memory budgeting

- **Status:** open
- **Blocks:** REPORT-010, RUST-017, BENCH-010
- **Question:** CPU-count based, memory-aware, fixed conservative, or user-only parallelism?
- **Evidence required:** report scaling/RSS/filesystem across tiny/large workloads/platforms.
- **Recommended direction:** bounded memory-aware default with single-thread fallback and explicit override, selected after BENCH-005.
- **Decision must specify:** cap algorithm and diagnostics.

### ADR-Q022 - Report output deduplication compatibility mode

- **Status:** open
- **Blocks:** REPORT-015, REPORT-016, BENCH-009
- **Question:** Which shared assets/data can change default tree layout without breaking file://, copying, archives, or downstream tools?
- **Evidence required:** artifact inventory, browser/tool tests, size savings.
- **Recommended direction:** shared static assets where existing paths can be preserved; more aggressive compact bundle remains opt-in initially.
- **Decision must specify:** default versus compact mode and portability behavior.

### ADR-Q023 - Mixed-profile clock-domain merge

- **Status:** open
- **Blocks:** RUST-013, TOOL-009
- **Question:** How to merge exact ticks from different clock frequencies/identities without order-dependent rounding?
- **Evidence required:** legacy behavior, report needs, rational arithmetic cost, v5/v6 combinations.
- **Recommended direction:** retain per-stream exact clock domains and use exact rational normalization at aggregate/presentation boundaries; refuse combinations whose semantics cannot be defined.
- **Decision must specify:** event-stream ordering and output clock metadata.

### ADR-Q024 - Native report default promotion criteria/field window

- **Status:** **criteria answered** by [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) (**PR-D02**); **product flip still gated** (not executed) until accepted field report recommends promote
- **Blocks:** BUILD-014, BUILD-015, BENCH-013, TEST-020 (flip execution still blocked without field promote)
- **Question:** What release duration/usage/issue/performance evidence is sufficient for `auto` to prefer native reports (and for product default to become `auto`)?
- **Evidence required:** R1 field data, platform success, fallback frequency, downstream reports, certifications — pack via [R3_FIELD_WINDOW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md); flip checklist [R3_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md)
- **Recommended direction:** at least one stable opt-in cycle plus full required gates and rollback mechanism — **accepted in ADR-0005**.
- **Decision must specify:** eligible tiers and fallback policy — **specified in ADR-0005**; flip procedure + one-step force-legacy rollback in `docs/R3_DEFAULT_FLIP.md`.

### ADR-Q025 - v6 output default promotion criteria/field window

- **Status:** **criteria answered** by [ADR-0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) (**PR-E02**); **product flip still gated** (not executed) until accepted field report recommends promote
- **Blocks:** BUILD-014, BUILD-015, BENCH-013, TEST-020 (flip execution still blocked without field promote)
- **Question:** What evidence is sufficient to change default profile format from v5 to v6 on eligible tiers?
- **Evidence required:** R2 field data, old-tool conversion usage, corruption/fork/long-run results, format stability, P1/P2 — pack via [R4_FIELD_WINDOW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_FIELD_WINDOW.md); flip checklist [R4_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md)
- **Recommended direction:** multiple opt-in releases/stability window and separate tier policy; retain `format=v5` — **accepted in ADR-0008**.
- **Decision must specify:** eligible tiers, compatibility window, rollback — **specified in ADR-0008**; flip procedure + force-v5 rollback in `docs/R4_DEFAULT_FLIP.md`.

### ADR-Q026 - Legacy code retirement policy

- **Status:** **governance answered** by [ADR-0009](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0009-r5-legacy-retirement-governance.md) (**PR-F01**); **no component retired**; component-specific deprecation/removal ADRs remain optional / deferred on evidence
- **Blocks:** none for modernization
- **Question:** Whether/when to retire legacy reader/report/writer or raise minimum Perl versions?
- **Evidence required:** long-term native field use, platform/ecosystem usage, maintenance/security cost, migration coverage — pack via [R5_RETIREMENT_REVIEW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R5_RETIREMENT_REVIEW.md)
- **Recommended direction:** no automatic retirement; separate decisions per component after deprecation — **accepted in ADR-0009**; absence of retirement is valid success.
- **Decision must specify:** warning period, alternatives, support end, file-format longevity — **process specified in ADR-0009** + per-component ADRs when (if ever) executed.

### ADR-Q027 - v5 in-process coalesced checkpoints (charter exception)

- **Status:** open (proposed vehicle: [ADR-0013](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0013-v5-coalesced-checkpoints.md))
- **Blocks:** item-3 implementation (PR-C1 / PR-C2)
- **Question:** May NYTProfM, under explicit `aggregate=1`, replace the ordered per-interval TIME_LINE / TIME_BLOCK / per-return SUB_CALLERS stream with in-memory maps and coalesced v5 records, violating charter #2–#4 and plan 01 A2/A4?
- **Evidence required:** charter + plan 01 cites; model `LineTotal.calls` increment; di01 780 occupancy; installed_attach per-tag edge count; field file sizes (engineering only); owner identity for sign-off.
- **Recommended direction:** allow **only** as an opt-in exception with default `aggregate=0`; dirty-delta emit; same-file v5 tags; no R4 flip.
- **Decision must specify:** project-owner name, lost counts vs kept totals, fail-closed caps, test bars that stay on `aggregate=0`.

### BUILD-LAYOUT - Collector packaging / source-tree overlay (design-program OQ-8)

- **Status:** accepted
- **Blocks:** COL-001 / PR-B02 (was blocking until accepted)
- **Question:** Where do modernization collector C/XS sources live relative to the BASE-001 oracle pin (`B0-A` overlay vs `B0-B` patch-in-pin)?
- **Decision:** **B0-A overlay** under repository-root `collector/`; oracle pin under `baseline/6.15/` remains archives + isolated install. See [`docs/adrs/0004-collector-packaging-source-tree.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md).
- **Not ADR-Q008:** plan **ADR-Q008** is chunk size/boundary policy (format/collector flush) — unrelated; do not close ADR-Q008 from this layout decision.
- **Design-program OQ-8:** external completion-architecture label for the same layout question; in-repo SoT is this entry + ADR-0004.

## ADR document template

```markdown
# ADR-NNN - Title

- Status:
- Date:
- Owners/reviewers:
- Related question/tasks/risks:

## Context and evidence

## Decision drivers

- correctness/precision
- backward compatibility
- collector performance
- storage
- decode/report performance
- security/recovery
- portability/packaging
- implementation/maintenance complexity

## Options considered

### Option A

Benefits, costs, measurements, risks.

### Option B

Benefits, costs, measurements, risks.

## Decision

Normative language and selected parameters.

## Consequences

Positive, negative, operational, migration, testing.

## Compatibility/versioning

## Required spec/test/code updates

## Revisit triggers
```

## ADR completion rule

An `accepted` ADR is complete only when:

- normative prose and machine-readable schema are updated;
- immutable vectors/fixtures are added where applicable;
- affected task acceptance criteria reference the decision;
- risks and migration/versioning consequences are recorded;
- no contradictory open question remains.

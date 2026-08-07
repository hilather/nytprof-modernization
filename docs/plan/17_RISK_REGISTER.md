# 17 - Risk Register

## Purpose

Track technical, compatibility, performance, security, packaging, and delivery risks throughout the modernization. Each risk has an owner, leading indicators, mitigation, contingency, and closure evidence.

## Scoring

- **Likelihood:** 1 rare, 2 unlikely, 3 possible, 4 likely, 5 very likely.
- **Impact:** 1 minor, 2 limited, 3 major, 4 severe, 5 project/release critical.
- **Score:** likelihood x impact.
- **Priority:** 15-25 critical, 8-14 high, 4-7 medium, 1-3 low.

Scores should be updated from evidence. A lower score is not closure.

## Active risks

### RSK-001 - Collector refactor changes timing attribution

- **Category:** correctness
- **Likelihood/impact/score:** 4/5/20 critical
- **Owner:** collector lead
- **Trigger/indicators:** fake-clock mismatch; changed event ordering; workload totals drift; assembly shows moved clock reads or discount boundaries.
- **Mitigation:** freeze state machine; implement fake clock; isolate semantic sink after timing; land v5-neutral refactor before v6; dual output.
- **Contingency:** revert hook refactor; keep legacy writer/hook path; move optimization offline only.
- **Closure evidence:** BASE-003, COL-001, COL-002, COL-005, TEST-003, TEST-008.

### RSK-002 - Per-event abstraction adds overhead

- **Category:** performance
- **Likelihood/impact/score:** 4/4/16 critical
- **Owner:** C performance lead
- **Trigger/indicators:** statement-heavy regression; added indirect calls/branches; compiler fails to inline; increased instruction/cache misses.
- **Mitigation:** compare specialized vs tagged APIs; static dispatch; assembly review; no mandatory FFI; benchmark each refactor slice.
- **Contingency:** generated/static writer calls or compile-time sink specialization; reduce abstraction in hot path while retaining semantic mapping tests.
- **Closure evidence:** COL-004, BENCH-003, BENCH-006.

### RSK-003 - v6 dictionaries cost more CPU than they save

- **Category:** performance/storage
- **Likelihood/impact/score:** 3/4/12 high
- **Owner:** format/collector performance lead
- **Trigger/indicators:** high-cardinality workloads regress; poor hit rate; hash allocation dominates; size improvement small.
- **Mitigation:** benchmark by string class; bounded specialized interners; optional per-class strategy selected by format rules; process/chunk scope study.
- **Contingency:** emit literals for low-benefit classes while retaining dictionary capability; keep format deterministic.
- **Closure evidence:** FMT-005, COL-010, BENCH-005, BENCH-006.

### RSK-004 - Stateful deltas amplify corruption or complicate recovery

- **Category:** reliability/format
- **Likelihood/impact/score:** 3/4/12 high
- **Owner:** v6 format lead
- **Trigger/indicators:** one bad record corrupts following decode; salvage cannot resume; complex decoder state bugs.
- **Mitigation:** reset all delta/dictionary state at chunks; per-chunk checksum; strict lengths; immutable corruption vectors.
- **Contingency:** reduce delta scope or use more absolute fields; smaller chunks for recovery.
- **Closure evidence:** FMT-006, FMT-010, FMT-012, TEST-014, TEST-015, SEC-003.

### RSK-005 - Integer ticks cannot reproduce all v5 `NV` behavior

- **Category:** compatibility/numerics
- **Likelihood/impact/score:** 3/5/15 critical
- **Owner:** precision/compatibility lead
- **Trigger/indicators:** unusual Perl `NV` width/type fixtures differ; v6-to-v5 conversion rounding mismatch; NaN/Inf/native representation quirks.
- **Mitigation:** inventory representations; preserve provenance; exact rational conversion; platform fixtures; strict representability checker; optional legacy C shim for exotic v5 decode.
- **Contingency:** support v5 read on affected tier through legacy engine; refuse v6-to-v5 conversion for unrepresentable values.
- **Closure evidence:** COMPAT-003, RUST-005, TOOL-005, TEST-012, TEST-017, TEST-019.

### RSK-006 - Legacy `ReadStream` order/behavior is underspecified

- **Category:** compatibility
- **Likelihood/impact/score:** 4/4/16 critical
- **Owner:** compatibility lead
- **Trigger/indicators:** third-party callback consumers fail; callback exception/return behavior differs; aggregate-only records omitted.
- **Mitigation:** source inventory; callback traces; downstream search/smoke; canonical contract; exact callback tests.
- **Contingency:** route `ReadStream` to legacy engine for unsupported corner until fixed.
- **Closure evidence:** COMPAT-001, COMPAT-007, COMPAT-013, PERL-004, TEST-006, TEST-010.

### RSK-007 - Compact model omits data needed by an obscure API/report

- **Category:** feature parity
- **Likelihood/impact/score:** 4/4/16 critical
- **Owner:** model/report architecture lead
- **Trigger/indicators:** legacy object field cannot be materialized; report provenance map has unmapped field; downstream consumer fails.
- **Mitigation:** exhaustive inventory/traceability; model acceptance before report port; raw stream remains available; compatibility materializer tests.
- **Contingency:** add typed field/index; retain targeted legacy path temporarily.
- **Closure evidence:** BASE-004, BASE-005, RUST-006 through RUST-009, PERL-005, PERL-006, REPORT-001, REPORT-002, TEST-010.

### RSK-008 - Rust parser supports common v5 files but misses exotic platforms

- **Category:** portability/compatibility
- **Likelihood/impact/score:** 3/4/12 high
- **Owner:** v5 reader/platform lead
- **Trigger/indicators:** failures on long-double/quad `NV`, endian/32-bit variants, unusual zlib/Perl builds.
- **Mitigation:** tier policy; cross-platform fixture exchange; declared metadata tests; independent C oracle/shim option; legacy fallback.
- **Contingency:** native v5 reader unsupported on affected tier while legacy path remains.
- **Closure evidence:** BUILD-001, BUILD-006, RUST-005, TEST-017.

### RSK-009 - Rust requirement reduces CPAN installability

- **Category:** packaging/ecosystem
- **Likelihood/impact/score:** 4/5/20 critical
- **Owner:** build/release lead
- **Trigger/indicators:** CPAN Testers failures; no suitable toolchain; offline builds fail; distribution size/dependency issues.
- **Mitigation:** legacy-only mode; explicit tiers; source/prebuilt strategy; offline packaging; no Rust in collector requirement.
- **Contingency:** ship native reports as optional companion/feature; retain full v5 legacy distribution functionality.
- **Closure evidence:** BUILD-001, BUILD-003, BUILD-012, PERL-012.

### RSK-010 - FFI lifecycle causes leaks, panics, or use-after-free

- **Category:** safety/reliability
- **Likelihood/impact/score:** 3/5/15 critical
- **Owner:** FFI lead
- **Trigger/indicators:** sanitizer failures; callbacks after free; Perl destruction order issues; panic crosses ABI.
- **Mitigation:** opaque handles; explicit ownership; panic containment; lifecycle state tests; independent audit.
- **Contingency:** restrict native API surface; use subprocess CLI for reports on affected path.
- **Closure evidence:** RUST-010, PERL-011, BUILD-009, SEC-008.

### RSK-011 - Parallel reports become nondeterministic

- **Category:** correctness/reproducibility
- **Likelihood/impact/score:** 4/3/12 high
- **Owner:** report concurrency lead
- **Trigger/indicators:** changing file/row/anchor order or hashes across runs/thread counts; races/partial output.
- **Mitigation:** immutable IR; explicit sort keys; bounded job manifest; per-file temp output; stable commit order; stress tests.
- **Contingency:** default to single-thread while preserving native model gains.
- **Closure evidence:** RUST-017, REPORT-010, TEST-009, BENCH-010.

### RSK-012 - Report semantic equality misses visual/navigation regression

- **Category:** compatibility/usability
- **Likelihood/impact/score:** 3/3/9 high
- **Owner:** report QA lead
- **Trigger/indicators:** broken links/anchors; unreadable source; browser differences despite data match.
- **Mitigation:** DOM/link/anchor checks, source block comparison, selected browser smoke/screenshots, artifact catalog.
- **Contingency:** keep legacy renderer default for affected report type.
- **Closure evidence:** REPORT-001, REPORT-009, REPORT-019, TEST-009.

### RSK-013 - HTML/path injection from profile content

- **Category:** security
- **Likelihood/impact/score:** 3/5/15 critical
- **Owner:** report security lead
- **Trigger/indicators:** payload changes DOM, executes script, escapes output root, invokes graph shell syntax.
- **Mitigation:** context-specific escaping; controlled filenames; direct argv; path/symlink protections; adversarial corpus.
- **Contingency:** disable affected renderer/integration until fixed; legacy path also audited if exposed.
- **Closure evidence:** SEC-006, SEC-007, REPORT-005, TEST-014.

### RSK-014 - New codecs introduce dependency/security/support burden

- **Category:** packaging/security
- **Likelihood/impact/score:** 3/4/12 high
- **Owner:** codec/build lead
- **Trigger/indicators:** unavailable libraries, licensing issues, CVEs, platform failures, larger binaries.
- **Mitigation:** codec-neutral format; `none`/zlib baseline; benchmark and dependency review; feature negotiation.
- **Contingency:** ship v6 with zlib/none only; add codec later without format break.
- **Closure evidence:** FMT-009, FMT-015, BUILD-008, BUILD-011, SEC-004.

### RSK-015 - v6 format freezes too early

- **Category:** architecture
- **Likelihood/impact/score:** 4/5/20 critical
- **Owner:** format architecture lead
- **Trigger/indicators:** unresolved event mapping, poor size/CPU results, conversion ambiguity, incompatible draft churn.
- **Mitigation:** report-side work first; prototype with experimental version; independent implementations; codec/chunk benchmarks; ADR queue closure before stable vectors.
- **Contingency:** do not call draft stable; bump experimental major/magic; continue v5 collection.
- **Closure evidence:** FMT-001 through FMT-015 and the phase 3 format review.

### RSK-016 - Old and new paths share code and repeat the same bug

- **Category:** testing
- **Likelihood/impact/score:** 3/4/12 high
- **Owner:** QA architecture lead
- **Trigger/indicators:** comparator/encoder/decoder derived from same implementation; mutation tests fail to detect errors.
- **Mitigation:** independent Rust/C implementations; legacy executable oracle; slow reference aggregator/comparator; property tests; seeded mutations.
- **Contingency:** add third implementation/reference for disputed component.
- **Closure evidence:** TEST-001, TEST-002, TEST-006, TEST-008, RUST-011.

### RSK-017 - Separate-run benchmarks misattribute workload variance

- **Category:** performance methodology
- **Likelihood/impact/score:** 4/3/12 high
- **Owner:** performance lead
- **Trigger/indicators:** unstable results; old/new input behavior differs; best-run cherry-picking.
- **Mitigation:** correctness hashes; interleaved repetitions; noise study; controlled inputs/host; raw distributions.
- **Contingency:** classify result advisory and avoid public claim.
- **Closure evidence:** BENCH-001, BENCH-013, BENCH-014, TEST-003, TEST-008.

### RSK-018 - File-size gains are offset by metadata/chunk overhead

- **Category:** storage
- **Likelihood/impact/score:** 3/3/9 high
- **Owner:** format performance lead
- **Trigger/indicators:** small profiles grow; worst-case streams exceed v5 materially; indexes/summaries duplicate data.
- **Mitigation:** benchmark tiny/large profiles; adaptive but deterministic chunking within spec; optional footer/index/summaries; compact header.
- **Contingency:** keep v5 default for tiny profiles or optimize header/chunk framing without semantic change; any automatic format choice needs ADR.
- **Closure evidence:** FMT-015, BENCH-005, BENCH-007.

### RSK-019 - Source dedup changes logical file/eval identity

- **Category:** correctness
- **Likelihood/impact/score:** 3/5/15 critical
- **Owner:** source-format/collector lead
- **Trigger/indicators:** reports merge distinct paths/evals; source association/order differs; changed-file behavior lost.
- **Mitigation:** content blob separate from logical source/file events; byte-exact identity; path/event metadata retained; dedicated fixtures.
- **Contingency:** disable dedup for ambiguous source class while preserving format capability.
- **Closure evidence:** FMT-008, COL-013, RUST-009, TEST-005.

### RSK-020 - Derived summaries become an unchecked source of truth

- **Category:** correctness
- **Likelihood/impact/score:** 3/5/15 critical
- **Owner:** model/format lead
- **Trigger/indicators:** report differs depending on summary presence; corrupt summary accepted; raw stream ignored.
- **Mitigation:** summaries additive; checksum/schema/provenance; verify against raw events before trust; strict mode can ignore/rebuild.
- **Contingency:** disable summary use by default; retain reindex/rebuild tool.
- **Closure evidence:** FMT-011, RUST-014, TOOL-008, REPORT-017, SEC-009.

### RSK-021 - v6-to-v5 conversion silently rounds or drops data

- **Category:** data integrity/compatibility
- **Likelihood/impact/score:** 3/5/15 critical
- **Owner:** conversion lead
- **Trigger/indicators:** old-tool output differs; values exceed v5; unsupported event hidden.
- **Mitigation:** full preflight representability; no lossy force mode; canonical hashes; old-tool tests; precise failure.
- **Contingency:** require new tools or recollect/write v5 dual for affected run.
- **Closure evidence:** TOOL-005, TEST-012, TEST-019, COMPAT-003.

### RSK-022 - Merge semantics across clock domains are ambiguous

- **Category:** correctness
- **Likelihood/impact/score:** 3/4/12 high
- **Owner:** merge/numerics lead
- **Trigger/indicators:** incompatible frequencies/clocks; totals depend on conversion order; legacy behavior unclear.
- **Mitigation:** normative merge spec; exact rational conversion or retain stream domains; strict refusal where semantics are undefined; fixtures.
- **Contingency:** limit mixed-clock merge mode and document; report separately per stream/process.
- **Closure evidence:** RUST-013, TOOL-009, TEST-013, and the mixed-clock-domain ADR.

### RSK-023 - Incomplete/corrupt profile recovery behavior regresses

- **Category:** reliability/compatibility
- **Likelihood/impact/score:** 3/4/12 high
- **Owner:** recovery lead
- **Trigger/indicators:** less data recoverable than legacy; corrupt file called valid; salvage guesses state.
- **Mitigation:** inventory legacy behavior; explicit states; chunk commit points/checksums; truncation matrix.
- **Contingency:** use legacy v5 recovery for v5; restrict v6 salvage to verified chunks.
- **Closure evidence:** SEC-003, TOOL-007, TEST-014.

### RSK-024 - Report output storage optimization breaks offline usage

- **Category:** compatibility/usability
- **Likelihood/impact/score:** 3/3/9 high
- **Owner:** report storage lead
- **Trigger/indicators:** file:// browser blocks loading; shared assets missing after copy/archive; hard links lost.
- **Mitigation:** compatibility mode preserves self-contained behavior; test file:// and simple server; measure archive/on-disk separately.
- **Contingency:** keep compact report opt-in.
- **Closure evidence:** REPORT-015, REPORT-016, REPORT-019, BENCH-009, TEST-009.

### RSK-025 - Long-running process exhausts dictionary/buffer memory

- **Category:** reliability/performance
- **Likelihood/impact/score:** 3/4/12 high
- **Owner:** collector data-structure lead
- **Trigger/indicators:** unbounded unique eval/sub/source names; RSS growth; OOM.
- **Mitigation:** bounded/interner memory policy; chunk/generation strategy; explicit OOM/error handling; long-run/high-cardinality tests.
- **Contingency:** fall back to literal encoding or dictionary reset at defined safe boundaries according to spec.
- **Closure evidence:** COL-005, COL-010, SEC-001, BENCH-005.

### RSK-026 - Fork/daemon behavior corrupts stream state

- **Category:** correctness/reliability
- **Likelihood/impact/score:** 4/5/20 critical
- **Owner:** collector lifecycle lead
- **Trigger/indicators:** parent/child shared dictionary/compressor/file offsets; duplicate sequences; invalid footer.
- **Mitigation:** explicit lifecycle state machine; fork detection/reinit; synchronized tests; fault injection; process generation IDs.
- **Contingency:** force v5 legacy collector for unsupported fork patterns until fixed.
- **Closure evidence:** COL-015, TEST-018, SEC-008.

### RSK-027 - Default changes outpace ecosystem migration

- **Category:** release/ecosystem
- **Likelihood/impact/score:** 3/5/15 critical
- **Owner:** release/product lead
- **Trigger/indicators:** users rely on old tools unavailable for v6; native binaries fail; support volume; conversion friction.
- **Mitigation:** separate opt-in/default phases; field window; v5 write mode/conversion; explicit engine/format controls; telemetry/issues; rollback.
- **Contingency:** revert default while continuing to read v6; extend compatibility window.
- **Closure evidence:** REL-005 through REL-009, COMPAT-014, BUILD-014.

### RSK-028 - Project scope becomes a wholesale rewrite with delayed value

- **Category:** delivery
- **Likelihood/impact/score:** 4/4/16 critical
- **Owner:** program architecture lead
- **Trigger/indicators:** native report work waits on v6; one long branch; no releasable slices; repeated redesign.
- **Mitigation:** v5 reader/report first; independent release levels; task packages; phase gates; small merges.
- **Contingency:** ship native v5 report companion; defer collector v6 while retaining benefits.
- **Closure evidence:** phase 1/2 releases and critical-path tracking.

### RSK-029 - Legacy code diverges and doubles maintenance cost

- **Category:** maintainability
- **Likelihood/impact/score:** 4/3/12 high
- **Owner:** maintenance lead
- **Trigger/indicators:** fixes applied to one engine only; tests duplicated inconsistently; fallback rots.
- **Mitigation:** shared canonical specs/fixtures; same compatibility suite; wrappers select engines; document support window; targeted common Perl facade.
- **Contingency:** narrow legacy path to stable v5 behavior after field-proven native path, only through separate retirement ADR.
- **Closure evidence:** TEST-020, REL-011, REL-012.

### RSK-030 - Performance improvements depend on unsupported hardware/features

- **Category:** portability/performance
- **Likelihood/impact/score:** 2/3/6 medium
- **Owner:** native performance/build lead
- **Trigger/indicators:** SIMD/CPU-specific binaries crash or regress; prebuilt mismatch.
- **Mitigation:** conservative baseline target; runtime feature detection; portable fallback; tier tests.
- **Contingency:** disable feature in release build; retain scalar implementation.
- **Closure evidence:** BUILD-006, BENCH-013, BENCH-014.

## Risk review cadence

- Review critical/high risks at every phase gate and release candidate.
- Update scores after benchmark, fixture, fuzz, field, or platform evidence.
- A task owner flags a risk immediately when a trigger appears; do not wait for scheduled review.
- Closed risks remain archived with evidence and may reopen after format/API/default changes.

## New risk template

```text
Risk ID/title:
Category:
Likelihood/impact/score:
Owner:
Description:
Trigger/leading indicators:
Mitigation/prevention:
Contingency/rollback:
Dependencies:
Closure evidence:
Status/history:
```

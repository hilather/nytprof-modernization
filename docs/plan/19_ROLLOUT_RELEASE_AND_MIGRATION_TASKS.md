# 19 - Rollout, Release, and Migration Task Plan

## Purpose

Introduce native reporting and v6 collection in reversible stages while preserving v5 workflows, old tools, explicit engine/format selection, and evidence-driven default changes. This workstream owns release levels, compatibility windows, migration/conversion guidance, field validation, telemetry/issue review, rollback, deprecation governance, and release communication.

## Rollout principles

1. Native reporting and v6 collection are separate releases/default decisions.
2. First value should come from reading/reporting existing v5 files.
3. v6 ships opt-in before it can become a default.
4. `format=v5` and `engine=legacy` remain explicit escape hatches during the documented window.
5. A default change must be reversible without losing the ability to read files already produced.
6. Field experience supplements but never replaces regression/security/performance certification.
7. No release note may claim generic speed/size gains beyond the certified configurations.
8. Conversion limitations and unsupported platform tiers are stated prominently.
9. Legacy retirement is not implied by modernization completion.
10. Release artifacts, specs, vectors, and compatibility matrices are versioned together.

## Release channel proposal

### Developer preview

- Feature/build flags required.
- Intended for fixture/tool authors.
- Dual output recommended/required for collector testing.
- No stability promise for experimental v6 wire bytes unless explicitly tagged stable.

### Experimental opt-in

- Native v5 report engine selectable.
- Stable public engine-selection controls.
- v6 may remain experimental until normative spec/vectors freeze.

### Stable opt-in

- Native v5 reporting and/or stable v6 collection advertised for production evaluation on eligible tiers.
- Full scoped certification complete.
- Legacy/v5 fallback documented.

### Preferred/default

- Separate decisions for native report engine and v6 output.
- Requires field window, certifications, rollback, and approved ADR.

## Task list

### REL-001 - Define release levels and compatibility windows

- **Status:** proposed
- **Size:** L
- **Dependencies:** BUILD-001, COMPAT-009 through COMPAT-011, `15_PHASES_DEPENDENCIES_AND_CRITICAL_PATH.md`
- **Suggested owner:** release/product lead
- **Goal:** Make support promises and promotion criteria explicit.
- **Work:**
  - Define R0-R5 levels from `16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md`.
  - Set initial duration/release-count windows for `format=v5`, legacy engine, v5 reading, conversion, and platform tiers.
  - Define which promises are format-level versus implementation-level.
  - Define policy for experimental versus stable v6 magic/version.
- **Deliverables:** release/support policy ADR and public matrix draft.
- **Acceptance:** Every shipped mode has stated stability/support/fallback expectations.
- **Regression gate:** No default/removal can bypass its promotion/deprecation criteria.

### REL-002 - Build migration and interoperability documentation

- **Status:** proposed
- **Size:** L
- **Dependencies:** COMPAT-005, COMPAT-006, COMPAT-008, COMPAT-009, TOOL-004 through TOOL-010, PERL-014
- **Suggested owner:** technical documentation + compatibility agent
- **Goal:** Let users move among old/new versions without losing precision or tool access.
- **Work:**
  - Document engine/format selection, v5 writing, v5/v6 detection, strict conversion, mixed merge, report generation, unsupported codec/feature behavior, and old-tool workflows.
  - Include exact examples for teams with mixed NYTProf versions.
  - Explain that v6-to-v5 can refuse unrepresentable data and has no lossy override.
  - Document backup/retention and conversion manifests.
- **Deliverables:** migration guide, compatibility cookbook, troubleshooting flowcharts.
- **Acceptance:** Every required producer-consumer matrix cell has a documented workflow.
- **Regression gate:** Commands are tested from installed release artifacts.

### REL-003 - Add release-visible capability and provenance reporting

- **Status:** proposed
- **Size:** M
- **Dependencies:** TOOL-006, TOOL-013, TOOL-014, BUILD-005, BUILD-013
- **Suggested owner:** diagnostics/release agent
- **Goal:** Make field reports actionable by exposing exact capabilities and components.
- **Work:**
  - Extend version/verbose output with Perl/module version, collector format support, native ABI/core version, codecs, engine selected, platform tier/build mode, spec version, and feature flags.
  - Add a machine-readable capability command/output.
  - Avoid leaking sensitive paths by default while allowing diagnostic mode.
- **Deliverables:** capability schema/output, support-bundle command or instructions.
- **Acceptance:** A bug report can identify the exact producer/reader/renderer/codecs without guesswork.
- **Regression gate:** Existing simple `--version` compatibility fields remain present.

### REL-004 - Build release-candidate artifact and evidence bundle

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BUILD-015, TEST-020, BENCH-013, SEC-012
- **Suggested owner:** release engineering lead
- **Goal:** Package source/native artifacts with auditable compatibility, security, and performance evidence.
- **Work:**
  - Include checksums/signatures/provenance, SBOM/licenses, platform/capability matrix, regression certificate, security report, performance report, spec/vector versions, known issues, and rollback guidance.
  - Verify installed artifacts in clean environments and mixed-version workflows.
  - Archive raw evidence separately from public summaries.
- **Deliverables:** release candidate bundle and verification report.
- **Acceptance:** Every public claim and supported tier references evidence in the bundle.
- **Regression gate:** Release is blocked if artifact tests differ from source-tree certification unexpectedly.

### REL-005 - Ship native v5 reporting as opt-in

- **Status:** proposed
- **Size:** L
- **Dependencies:** Phase 2 exit, REPORT-020, COMPAT-014, BUILD-015, SEC-012, TEST-020, BENCH-013
- **Suggested owner:** release lead
- **Goal:** Deliver report-time/RSS gains without changing collector output.
- **Work:**
  - Expose `engine=native` and retain legacy default or explicitly experimental `auto` behavior.
  - Publish platform availability, known limitations, fallback, performance data, and issue-report template.
  - Confirm all existing v5 files remain usable.
- **Deliverables:** release, notes, migration docs, support dashboard.
- **Acceptance:** Installed native/legacy paths pass the advertised matrix; rollback is engine selection only.
- **Regression gate:** No collector/default-format change in this release solely to bundle features.

### REL-006 - Evaluate and promote native reporting in `auto`

- **Status:** proposed
- **Size:** L
- **Dependencies:** REL-005 field window, ADR-Q024, COMPAT-014, TEST-020, BENCH-013
- **Suggested owner:** release review group
- **Goal:** Prefer native reports on eligible tiers only after production evidence.
- **Work:**
  - Review opt-in usage/issues, fallback frequency, platform failures, downstream results, security/performance trends, and support load.
  - Define eligible operations/formats/tiers and exact fallback rules.
  - Run release-candidate matrix with default selection.
  - Prepare one-step default rollback.
- **Deliverables:** default-change ADR, release decision, rollback trigger list.
- **Acceptance:** All R3 criteria pass; users can force legacy; fallback cannot hide corruption.
- **Regression gate:** Post-release monitoring and rollback owner assigned.

### REL-007 - Ship stable v6 collection as opt-in

- **Status:** proposed
- **Size:** XL
- **Dependencies:** Phase 4 exit, COL-009 through COL-015, TOOL-004 through TOOL-009, TOOL-013, TOOL-016, COMPAT-014, BUILD-015, SEC-012, TEST-020, BENCH-013
- **Suggested owner:** release lead with collector/format leads
- **Goal:** Allow production evaluation of exact compact v6 while v5 remains default.
- **Work:**
  - Expose `format=v6`; keep `format=v5`; keep dual mode developer/testing only.
  - Ship inspect/verify/convert/merge/repack/salvage tools and stable spec/vectors.
  - Document unsupported old-tool direct read and exact conversion workflow.
  - Provide long-running/fork/recovery examples and support template.
- **Deliverables:** stable opt-in release, v6 specification package, migration docs.
- **Acceptance:** R2 criteria pass on advertised tiers; old v6.15 tools accept new v5 mode and successful converted files.
- **Regression gate:** Default remains v5 until REL-008 approval.

### REL-008 - Evaluate and promote v6 output default

- **Status:** proposed
- **Size:** XL
- **Dependencies:** REL-007 field window, ADR-Q025, COMPAT-014, TEST-020, BENCH-013, SEC-012
- **Suggested owner:** independent release review group
- **Goal:** Change default output only when format, tooling, ecosystem, and rollback are mature.
- **Work:**
  - Review real profile size/overhead, corruption/recovery, fork/long-run behavior, conversion usage/failures, native availability, mixed-team friction, and format issue history.
  - Define eligible tiers and legacy-only behavior.
  - Verify all current/new tools auto-detect and read v6; verify v5 escape hatch.
  - Test default rollback while retaining v6 read support.
- **Deliverables:** default-format ADR, release plan, rollback plan, compatibility-window reaffirmation.
- **Acceptance:** All R4 criteria pass; no known incompatible redesign expected; `format=v5` remains supported as promised.
- **Regression gate:** Default is changed independently from legacy removal.

### REL-009 - Add post-release field validation and incident process

- **Status:** proposed
- **Size:** L
- **Dependencies:** REL-005 or REL-007
- **Suggested owner:** maintenance/release lead
- **Goal:** Detect correctness, compatibility, security, and performance regressions quickly in released use.
- **Work:**
  - Define issue templates requesting capability/provenance, profile verification output, canonical hashes where safe, commands/options, platform, and minimal reproduction.
  - Define severity and rollback triggers, including any event mismatch/data loss/corruption/security issue.
  - Provide safe profile minimization/redaction guidance; do not request sensitive source blindly.
  - Feed fixed field bugs into fixtures/fuzz corpus/risk register.
- **Deliverables:** incident runbook, templates, triage SLA, rollback procedure.
- **Acceptance:** A simulated critical regression follows the process and produces a release/fallback decision.
- **Regression gate:** Critical correctness/security incidents can trigger immediate default rollback.

### REL-010 - Define telemetry/privacy policy

- **Status:** proposed
- **Size:** M
- **Dependencies:** COL-016, REL-003
- **Suggested owner:** privacy/release agent
- **Goal:** Gather useful local diagnostics without exposing profile/source/application data.
- **Work:**
  - Keep telemetry local/off by default unless project explicitly adopts opt-in reporting.
  - Classify safe counters versus sensitive names/paths/source/call data.
  - Define support-bundle redaction and user review.
  - Document collection/retention if any external telemetry is ever proposed.
- **Deliverables:** telemetry/privacy policy and redaction tests.
- **Acceptance:** Default release performs no undisclosed data transmission; support artifacts are inspectable.
- **Regression gate:** Any future external telemetry needs separate consent/security/privacy review.

### REL-011 - Maintain cross-release compatibility tests

- **Status:** proposed
- **Size:** L
- **Dependencies:** COMPAT-012, BUILD-006, BUILD-012, TEST-020
- **Suggested owner:** release QA agent
- **Goal:** Test candidate releases against several supported old/new versions, not only v6.15.
- **Work:**
  - Pin a release matrix including v6.15 oracle and each stable v6-capable release.
  - Test v5/v6 reading, v5 writing, conversion, merge, reports, engine selection, upgrade/downgrade, and mixed installations.
  - Retain fixtures produced by each released stable v6 writer.
- **Deliverables:** rolling compatibility matrix and archived artifacts.
- **Acceptance:** Candidate release honors documented compatibility window.
- **Regression gate:** Required before each stable release.

### REL-012 - Run legacy retirement/deprecation review

- **Status:** deferred (execution); **governance ready** via [ADR-0009](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0009-r5-legacy-retirement-governance.md) + [`docs/R5_RETIREMENT_REVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R5_RETIREMENT_REVIEW.md) (**PR-F01** / ADR-Q026)
- **Size:** XL
- **Dependencies:** sustained R4 field use, ADR-Q026 (governance answered; component ADRs still required per retirement)
- **Suggested owner:** ecosystem/release review group
- **Goal:** Evaluate components independently rather than assuming removal.
- **Work:**
  - Measure usage/platform need/security/maintenance cost for legacy report engine, reader, writer, v5 format, and old Perl tiers.
  - Propose no change, narrower installation, deprecation, or retirement per component (catalog IDs in ADR-0009).
  - Provide migration tooling, warning period, support end, and archival reader guarantees.
- **Deliverables:** component-specific ADRs and deprecation plans if any; review packets per `R5_RETIREMENT_REVIEW.md`.
- **Acceptance:** No component is removed solely because native/v6 exists; **absence of retirement is valid success**.
- **Regression gate:** v5 archival readability requires an explicit long-term policy.

### REL-013 - Final modernization program review

- **Status:** proposed
- **Size:** L
- **Dependencies:** advertised target release level achieved, COMPAT-014, REL-004, REL-009, all relevant sign-offs
- **Suggested owner:** independent program review board
- **Goal:** Close the modernization program while preserving ongoing compatibility responsibilities.
- **Work:**
  - Review achieved storage/runtime/RSS goals, exactness, supported tiers, open risks/ADRs, field outcomes, maintenance ownership, and deferred retirement decisions.
  - Archive architecture/spec/test/evidence packages and assign long-term maintainers.
- **Deliverables:** final program report and maintenance backlog.
- **Acceptance:** Project success criteria are met for advertised scope; unresolved items have owners and are not misrepresented as complete.
- **Regression gate:** Ongoing cross-release/security/performance duties remain scheduled.

## Default rollback triggers

Immediate evaluation and likely rollback for an affected default if any occurs:

- canonical event/timing/count/source/call/process mismatch;
- silent loss during convert/merge/repack;
- corrupt profile accepted as complete;
- collector crash/corruption/fork bug attributable to v6 path;
- critical/high security vulnerability without immediate safe patch;
- widespread native load/install failure on an eligible tier;
- performance regression beyond ratified gate on common workload with no quick mitigation;
- report output corruption/injection/path escape;
- inability to read files produced by the default release.

Rollback changes selection defaults; it must not remove read support for files already produced.

## Release-note evidence checklist

- exact version/date and release level;
- default engine/format by tier;
- compatible input/output formats;
- old-tool workflows and conversion limitations;
- supported codecs/platforms;
- correctness/security/performance certification scope;
- measured claims with configurations;
- known issues and fallback commands;
- migration/rollback links;
- spec/vector/schema versions.

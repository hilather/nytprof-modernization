# WP-11 - Build, packaging, platform, and CI

## Required reading

1. [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](../01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md)
2. [`03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md`](../03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md)
3. [`TASK_INDEX.md`](../TASK_INDEX.md), followed by every source workstream file for the assigned task IDs
4. [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](../10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md)
5. [`16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md`](../16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md)
6. [`17_RISK_REGISTER.md`](../17_RISK_REGISTER.md) and [`18_OPEN_QUESTIONS_AND_ADR_QUEUE.md`](../18_OPEN_QUESTIONS_AND_ADR_QUEUE.md)

## Work-package definition

- **Tasks:** BUILD-001 through BUILD-015; PERL-012; TEST-017; TEST-020.
- **Owner profile:** CPAN/MakeMaker/Rust release engineer.
- **Objective:** Support native and legacy-only modes across explicit platform tiers.
- **Inputs:** early workspace/FFI prototypes; evolves throughout program.
- **Can run in parallel with:** all implementation lanes.
- **Required handoff:** tier policy, build modes, CI matrix, loader, offline/source packaging, artifact pipeline.
- **Exit gate:** clean installed artifacts pass required matrices; no hidden native dependency for legacy-only mode.

## Agent evaluation checklist

Before implementation, the assigned agent must produce an `EVALUATION.md` using [`AGENT_EVALUATION_TEMPLATE.md`](../templates/AGENT_EVALUATION_TEMPLATE.md), containing:

- the exact task IDs accepted, deferred, or rejected;
- dependency artifacts and versions/checksums reviewed;
- source locations and legacy behaviors that constrain the work;
- open ADR questions and experiments needed to resolve them;
- proposed implementation slices, each independently mergeable with legacy behavior green;
- required regression rows, platform tiers, security checks, and benchmark workloads;
- rollback or feature-selection mechanism;
- risks newly discovered or whose likelihood/impact changed.

An agent must stop before encoding an unresolved assumption into a stable wire format, public API, timing state machine, or default. Record the question in the ADR queue instead.

## Execution checklist

- Copy [`TASK_TEMPLATE.md`](../TASK_TEMPLATE.md) for implementation subtasks that need more granularity.
- Preserve the exact ordered event stream and exact timing semantics; derived aggregates or indexes may only supplement raw events.
- Keep legacy and native paths independently selectable for the work package's entire certification window.
- Add positive, boundary, malformed/failure, resource-limit, and lifecycle tests appropriate to the component.
- Attach canonical equality hashes before accepting performance measurements.
- Record raw benchmark/test/security outputs, environment, commands, repetitions, and artifact checksums.
- Do not update golden fixtures merely to make candidate behavior pass; require an approved semantic decision.

## Completion and handoff checklist

The handoff is incomplete until it includes:

- task status and linked commits for every assigned ID;
- exact build/test/benchmark commands and tool versions;
- generated specifications, schemas, fixtures, binaries, reports, and checksums;
- compatibility matrix rows and platform tiers executed;
- first-mismatch or failure bundles for any known difference;
- performance evidence for every optimization claim;
- security/reliability evidence for every untrusted-input or lifecycle boundary;
- accepted ADRs and remaining open questions;
- known limitations and one-step rollback/disable instructions;
- named downstream work packages and required actions.

Use [`HANDOFF_TEMPLATE.md`](../templates/HANDOFF_TEMPLATE.md) and the canonical handoff rules in the parent work-package document. Code without a reproducible evidence handoff is not complete.

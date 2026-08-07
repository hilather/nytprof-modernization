# Task Template

Copy this template into the appropriate workstream file or task tracker. Do not remove fields; use `not applicable` with a reason where needed.

## TASK-ID - Task title

- **Status:** proposed | ready | in-progress | blocked | review | done | rejected-with-ADR
- **Priority:** critical | high | medium | low
- **Size:** XS | S | M | L | XL
- **Suggested owner:** role/specialty
- **Assignee:**
- **Reviewers:**
- **Dependencies:** task IDs / ADRs / specs / fixtures
- **Blocks:** task IDs / phase gate
- **Related risks:** RSK IDs
- **Related ADR questions/decisions:** ADR-Q / ADR IDs
- **Compatibility clauses:** sections from `01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`

### Goal

One testable outcome. State what must be true, not just what code to write.

### Rationale

Why this task exists and what user/project risk it addresses.

### Scope

#### In scope

- concrete behaviors/components;
- supported inputs/options/platform tiers;
- required old/new paths.

#### Out of scope

- explicitly deferred work;
- tempting shortcuts or related changes that need separate tasks.

### Inputs and frozen assumptions

- normative spec/schema version;
- fixture/corpus version and checksums;
- oracle build/version;
- accepted ADRs;
- exact component APIs/lifetimes.

### Work

1. Investigation/design step.
2. Implementation step.
3. Positive/boundary/error/resource step.
4. Compatibility integration step.
5. Benchmark/security/documentation step.

### Deliverables

- source files/components;
- tests/fixtures/vectors;
- schemas/spec/docs;
- command/API/report outputs;
- benchmark/security evidence;
- handoff file.

### Acceptance criteria

- objective semantic/API/wire/report result;
- exact comparator/hash/old-tool cell that must pass;
- resource/error behavior;
- platform/build scope;
- performance threshold or neutral budget.

### Required regression gates

- canonical event equality;
- canonical aggregate equality;
- old reader/new writer;
- old writer/new reader;
- Perl object/method/callback parity;
- CLI/report semantic/DOM/link parity;
- conversion/merge round trip;
- fake clock/fork/source/calls/incomplete behavior;
- downstream consumer tests.

Mark non-applicable items and explain why.

### Required test cases

#### Positive

- minimal valid case;
- representative combined case;
- all relevant options/modes.

#### Boundaries

- zero/one/max and length/varint/tick/count limits;
- empty/binary/large strings/source;
- chunk/buffer/fork/finalization boundaries;
- 32/64-bit or platform variants.

#### Failure/security

- malformed/truncated/corrupt/unsupported;
- OOM/resource limit/decompression ratio;
- short write/disk full/close error;
- escaping/path/native loading/FFI lifetime as applicable.

#### Differential/property/fuzz

- exact oracle comparison;
- seeded mutation;
- encode/decode or aggregate properties;
- minimized regression fixture.

### Performance evidence

```text
Affected cost center:
Baseline/candidate commits and builds:
Equivalent feature/options proof:
Correctness qualification hashes:
Workloads and repetitions:
Wall/CPU/RSS/I/O/size results:
Worst workload regression:
Raw data path:
Decision:
```

### Security/reliability review

- trust boundary and untrusted fields;
- checked arithmetic/allocation;
- ownership/lifetime/unsafe invariants;
- failure and partial-output state;
- recovery/salvage behavior;
- escaping/path/external command behavior;
- fuzz/sanitizer/fault-injection status.

### Documentation updates

- normative spec/schema;
- API/CLI/help/migration;
- build/platform/capabilities;
- known limitations/error codes;
- task index/risk/ADR status.

### Handoff

```text
Commit(s):
Build/test commands:
Exact tool paths/versions:
Artifacts/checksums:
Compatibility results:
Benchmark/security results:
Known limitations:
Open questions:
Downstream tasks/owners:
```

### Completion checklist

- [ ] Dependencies/ADRs/specs are approved.
- [ ] Implementation and failure paths complete.
- [ ] Tests and seeded-mutation checks pass.
- [ ] Required old/new compatibility cells pass.
- [ ] Correctness qualifies performance data.
- [ ] Security/resource review complete.
- [ ] Documentation/spec/schema updated.
- [ ] Handoff reproducible.
- [ ] Master task index/status updated.
- [ ] Independent reviewer approved.

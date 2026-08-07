# Task Index

This index contains **206 uniquely identified implementation, evaluation, certification, and rollout tasks**. Detailed work, deliverables, acceptance tests, regression gates, and risks remain in the linked workstream files.

The package also contains **30 tracked risk items** (`RSK-*`) and **26 blocking architecture-decision questions** (`ADR-Q*`); those are maintained in the risk register and ADR queue rather than counted as executable tasks.

## Workstream counts

| Prefix | Tasks | Primary scope |
|---|---:|---|
| `ARCH` | 8 | Architecture and component boundaries |
| `BASE` | 8 | v6.15 oracle, contracts, and baselines |
| `COMPAT` | 15 | Compatibility governance and certification |
| `FMT` | 15 | Lossless v6 file format |
| `COL` | 18 | C/XS collector, buffering, and writers |
| `RUST` | 18 | Native parser, model, aggregation, and FFI |
| `PERL` | 14 | Perl/XS facade and public APIs |
| `REPORT` | 20 | HTML and auxiliary report generation |
| `TOOL` | 16 | CLI, conversion, merge, inspection, and validation |
| `TEST` | 20 | Differential regression, fixtures, property tests, and fuzzing |
| `BENCH` | 14 | Collection, storage, decode, memory, and report benchmarks |
| `BUILD` | 15 | Build, packaging, CI, dependency, and release artifacts |
| `SEC` | 12 | Security, corruption handling, limits, and recovery |
| `REL` | 13 | Staged rollout, migration, defaults, and long-term support |

## Status conventions

- `proposed`: scoped but not yet accepted by an assigned agent.
- `ready`: dependencies and evaluation artifact are complete.
- `in-progress`: implementation or investigation is active.
- `blocked`: a dependency, ADR, fixture, or platform issue prevents progress.
- `review`: deliverables exist and are awaiting independent acceptance.
- `done`: all task-specific and project-level gates pass.
- `deferred`: intentionally held for a later release/field window.
- `rejected-with-ADR`: not pursued, with evidence and an approved decision record.

## Tasks

### ARCH workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `ARCH-001` | Define the canonical C event API | proposed | L | COMPAT-001, BASE-002, BASE-003 | C/XS architect | [`03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md`](03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md) |
| `ARCH-002` | Define sink lifecycle and finalization state machine | proposed | L | ARCH-001 | C systems engineer | [`03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md`](03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md) |
| `ARCH-003` | Define Rust logical-event interfaces | proposed | M | COMPAT-001 | Rust API engineer | [`03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md`](03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md) |
| `ARCH-004` | Define compact aggregate model | proposed | L | BASE-004, BASE-008, ARCH-003 | Rust/data-layout engineer | [`03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md`](03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md) |
| `ARCH-005` | Define compatibility facade boundaries | proposed | M | ARCH-003, ARCH-004, COMPAT-004 | Perl/Rust integration engineer | [`03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md`](03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md) |
| `ARCH-006` | Define feature negotiation | proposed | M | FMT-001, BUILD-001 | format/build engineer | [`03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md`](03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md) |
| `ARCH-007` | Prototype dual-sink overhead | proposed | M | ARCH-001, COL-001 | performance engineer | [`03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md`](03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md) |
| `ARCH-008` | Establish ADR governance | done | S | none | technical lead | [`03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md`](03_TARGET_ARCHITECTURE_AND_COMPONENT_BOUNDARIES.md) |

### BASE workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `BASE-001` | Pin and reproduce the 6.15 oracle | done | L | none | release/build engineer | [`02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md`](02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md) |
| `BASE-002` | Inventory the complete v5 event protocol | in-progress | XL | BASE-001 | C/XS reverse-engineering and format engineer | [`02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md`](02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md) |
| `BASE-003` | Freeze timing, call, numeric, and lifecycle semantics | in-progress | XL | BASE-001, BASE-002 | Perl-internals/C and numerical systems engineers | [`02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md`](02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md) |
| `BASE-004` | Inventory Perl APIs and object-model behavior | proposed | XL | BASE-001 | senior Perl API/test engineer | [`02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md`](02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md) |
| `BASE-005` | Inventory CLI, report, and auxiliary-output contracts | proposed | XL | BASE-001, BASE-004 | CLI/report compatibility engineers | [`02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md`](02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md) |
| `BASE-006` | Build feature-to-test traceability matrix | proposed | L | BASE-002 through BASE-005 | test architect | [`02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md`](02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md) |
| `BASE-007` | Capture representative fixtures and performance baselines | proposed | XL | BASE-001 through BASE-006 | fixture and performance engineers | [`02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md`](02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md) |
| `BASE-008` | Quantify object/report memory and CPU amplification | proposed | L | BASE-004, BASE-005, BASE-007 | report/performance engineer | [`02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md`](02_V6_15_CURRENT_STATE_AND_HOTSPOTS.md) |

### COMPAT workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `COMPAT-000` | Ratify the compatibility contract | done | S | none | project maintainer/architect | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) |
| `COMPAT-001` | Define the canonical logical-event contract | in-progress | XL | BASE-001, BASE-002, BASE-003 | Perl/XS and binary-format architect | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md); provisional: [`docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md`](../contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md) |
| `COMPAT-002` | Define volatile-field normalization | in-progress | M | COMPAT-001, BASE-005 | test architecture engineer | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md); provisional dump rules: [`docs/contracts/COMPAT-002_VOLATILE_NORMALIZATION.md`](../contracts/COMPAT-002_VOLATILE_NORMALIZATION.md) |
| `COMPAT-003` | Define precision and numeric-conversion policy | in-progress | L | BASE-003, COMPAT-001 | numerical systems engineer | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md); provisional: [`docs/contracts/COMPAT-003_PRECISION_NUMERIC_POLICY.md`](../contracts/COMPAT-003_PRECISION_NUMERIC_POLICY.md) |
| `COMPAT-004` | Classify public and de facto public surfaces | proposed | L | BASE-004, BASE-005 | Perl/API compatibility maintainer | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) |
| `COMPAT-005` | Freeze the cross-version compatibility matrix | proposed | M | COMPAT-001 through COMPAT-004, BASE-005 | compatibility lead | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) |
| `COMPAT-006` | Freeze report and auxiliary-output parity rules | proposed | L | BASE-005, COMPAT-002, COMPAT-004 | reporting compatibility architect | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) |
| `COMPAT-007` | Freeze Perl object and callback fidelity rules | proposed | L | BASE-004, COMPAT-001, COMPAT-004 | senior Perl/XS maintainer | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) |
| `COMPAT-008` | Freeze CLI and diagnostic compatibility rules | proposed | M | BASE-005, COMPAT-004 | CLI compatibility engineer | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) |
| `COMPAT-009` | Freeze support-tier and dependency compatibility | proposed | M | BASE-001, COMPAT-004 | release/build architect | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) |
| `COMPAT-010` | Define error, fallback, and corruption policy | proposed | L | COMPAT-005, COMPAT-008 | reliability/compatibility architect | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) |
| `COMPAT-011` | Define legacy/no-Rust continuity requirements | proposed | M | COMPAT-009, COMPAT-010 | CPAN portability maintainer | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) |
| `COMPAT-012` | Define cross-version fixture-runner isolation | proposed | M | COMPAT-005, BASE-001 | release test engineer | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) |
| `COMPAT-013` | Define downstream-consumer validation policy | proposed | M | COMPAT-004, COMPAT-005 | ecosystem compatibility engineer | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) |
| `COMPAT-014` | Perform compatibility sign-off | proposed | M | COMPAT-001 through COMPAT-013, TEST-020 | compatibility reviewer independent of implementation leads | [`01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) |

### FMT workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `FMT-001` | Write the normative v5 semantic specification | proposed | XL | BASE-002, BASE-003, COMPAT-001 | format engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-002` | Draft and approve v6 header/chunk specification | proposed | L | ARCH-006, FMT-001 | binary-format architect | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-003` | Specify canonical varints and signed encoding | proposed | M | FMT-002 | low-level encoding engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-004` | Specify integer tick model and overflow behavior | proposed | L | COMPAT-003, BASE-003 | numerical systems engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-005` | Design string/subroutine dictionaries | proposed | L | FMT-002, BENCH-002 | format/performance engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-006` | Design location and depth delta encoding | proposed | M | FMT-003, BASE-007 | compression engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-007` | Evaluate reversible run encodings | proposed | M | COMPAT-001, BASE-007 | compression engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-008` | Design source blob and dedup representation | proposed | L | BASE-002, COMPAT-001 | source/eval specialist | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-009` | Implement codec-neutral chunk API | proposed | L | FMT-002 | C/Rust systems engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-010` | Implement chunk checksums and truncation recovery | proposed | L | FMT-002, SEC-003 | reliability engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-011` | Define optional index and summary schemas | proposed | L | ARCH-004, RUST-006 | data-model engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-012` | Build immutable v6 test vectors | proposed | L | FMT-002 through FMT-010 | test engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-013` | Specify strict v5↔v6 conversion | proposed | L | FMT-001, FMT-004, TOOL-001 | compatibility engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-014` | Add format inspection metadata | proposed | S | FMT-002 | tooling engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |
| `FMT-015` | Benchmark codecs and chunk sizes on NYTProf data | proposed | L | BASE-007, BENCH-001 | performance engineer | [`04_FILE_FORMAT_V6_TASKS.md`](04_FILE_FORMAT_V6_TASKS.md) |

### COL workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `COL-001` | Introduce the canonical sink interface | proposed | XL | COMPAT-001, BASE-002, BASE-003, ARCH-001 | senior C/XS engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-002` | Freeze and test sink lifecycle | proposed | L | ARCH-002, COL-001 | C systems engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-003` | Add monotonic logical event sequence numbers | proposed | M | COL-001 | C engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-004` | Build a no-allocation statement-event fast path | proposed | L | COL-001, BASE-003, BENCH-003 | low-level performance engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-005` | Implement bounded event batching | proposed | XL | COL-001, COL-003, BASE-003 | C systems/performance engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-006` | Adapt the legacy v5 writer to the sink API | proposed | L | COL-001, COL-005 | C/XS compatibility engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-007` | Prototype v6 writer candidate A in C | proposed | XL | FMT-002 through FMT-010, COL-005 | C binary-format engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-008` | Prototype v6 writer candidate B with batched Rust FFI | deferred | XL | FMT-002 through FMT-010, COL-005, RUST-010, BUILD-004 | C/Rust FFI engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-009` | Decide the production v6 writer backend | proposed | M | COL-007, BENCH-006 (COL-008 only if re-opened), BUILD-004 | architecture review group | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-010` | Implement dictionary interning for repeated names | proposed | L | FMT-005, COL-005 | C/Rust performance engineer according to selected backend | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-011` | Move v6 timing serialization to integer ticks | proposed | L | COMPAT-003, FMT-004, COL-006 | C numerical engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-012` | Implement reversible delta and run encodings | proposed | L | FMT-006, FMT-007, COL-007 or COL-008 | format/performance engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-013` | Implement source blob deduplication | proposed | L | FMT-008, COL-007 or COL-008 | source/eval specialist | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-014` | Implement same-run dual writer | proposed | L | COL-006, selected v6 writer, COL-003 | C compatibility engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-015` | Harden fork and PID transitions with buffered sinks | proposed | XL | COL-002, COL-005, COL-014 | Perl/C process-lifecycle specialist | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-016` | Add collector observability counters | proposed | M | COL-005 | performance engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-017` | Preserve slow-op and leave-correction semantics | proposed | L | COL-001, BASE-002, BASE-003 | Perl-internals specialist | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |
| `COL-018` | Add collector fault injection | proposed | M | COL-002, COL-005, selected v6 writer | reliability engineer | [`05_COLLECTOR_AND_C_XS_TASKS.md`](05_COLLECTOR_AND_C_XS_TASKS.md) |

### RUST workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `RUST-001` | Establish workspace, policy, and coding standards | done | M | BUILD-001, ARCH-003 | Rust technical lead | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-002` | Implement canonical logical event types | done | L | COMPAT-001, ARCH-003, RUST-001 | Rust API engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-003` | Implement bounded streaming I/O primitives | done | M | RUST-001, SEC-001 | Rust systems engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-004` | Implement the v5 streaming decoder | done | XL | FMT-001, RUST-002, RUST-003, BASE-002 | binary-format/Rust engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-005` | Implement v5 native-NV decoding and provenance | proposed | XL | BASE-003, COMPAT-003, RUST-004 | numerical portability engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-006` | Implement compact profile model | in-progress | XL | ARCH-004, RUST-002, BASE-004, BASE-008 | Rust data-layout engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) — first-slice MVP in `crates/nytprof-model` (A1–A6); full model/RSS not done |
| `RUST-007` | Implement statement/block aggregation | in-progress | L | RUST-004, RUST-006, BASE-003 | Rust correctness engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) — MVP TIME_LINE counts/ticks only (aggregate-comparison-v0) |
| `RUST-008` | Implement subroutine/call aggregation | in-progress | XL | RUST-004, RUST-006, BASE-003, COMPAT-001 | callgraph algorithms engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) — MVP SUB_RETURN totals only; not full callgraph |
| `RUST-009` | Implement source/eval/sub-definition model | proposed | L | RUST-004, RUST-006, FMT-008 | source/eval specialist | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-010` | Define and implement stable C ABI | proposed | XL | ARCH-005, RUST-002, RUST-004, BUILD-002 | senior Rust/C FFI engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-011` | Implement v6 encoder/decoder core | proposed | XL | FMT-002 through FMT-012, RUST-002, RUST-003 | Rust format engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-012` | Implement mixed-format streaming abstraction | proposed | M | RUST-004, RUST-011 | Rust API engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-013` | Implement deterministic merge engine | proposed | XL | RUST-006 through RUST-012, TOOL-004 | data integration engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-014` | Implement exact derived-summary validation | proposed | L | FMT-011, RUST-007, RUST-008, RUST-011 | data integrity engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-015` | Implement bounded call-event retention modes | proposed | M | RUST-008, REPORT-001, REPORT-002 | performance/data-model engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-016` | Add model memory and phase telemetry | proposed | M | RUST-006 | performance engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-017` | Prove deterministic parallel reductions | proposed | L | RUST-006 through RUST-009, REPORT-002 | concurrency engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |
| `RUST-018` | Add property and fuzz targets for core crates | proposed | L | RUST-003 through RUST-015, SEC-002 | fuzz/test engineer | [`06_RUST_CORE_AND_DATA_MODEL_TASKS.md`](06_RUST_CORE_AND_DATA_MODEL_TASKS.md) |

### PERL workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `PERL-001` | Complete the public API inventory and contract suite | proposed | XL | BASE-004, COMPAT-004 | senior Perl maintainer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-002` | Implement backend discovery and forcing | proposed | M | BUILD-004, ARCH-005 | Perl/XS integration engineer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-003` | Implement Rust error-to-Perl translation | proposed | M | RUST-010, PERL-001 | XS engineer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-004` | Implement native-backed ReadStream | proposed | XL | RUST-004, RUST-010, COMPAT-001, PERL-001 | Perl/XS engineer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-005` | Implement compact Data backend facade | proposed | XL | RUST-006 through RUST-009, RUST-010, PERL-001 | Perl/Rust object-model engineer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-006` | Implement legacy object graph materializer | proposed | XL | PERL-005, BASE-004 | XS/Perl compatibility engineer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-007` | Preserve eval collapse and file identity behavior | proposed | L | RUST-009, PERL-005 | Perl source/eval specialist | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-008` | Preserve Reader customization callbacks | proposed | L | BASE-004, REPORT-001, PERL-002 | Perl/reporting engineer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-009` | Preserve FileHandle-facing APIs | proposed | M | PERL-001, COL-006 | XS compatibility engineer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-010` | Preserve configuration parsing and precedence | proposed | L | BASE-005, COL-001 | Perl configuration engineer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-011` | Validate reference counts and interpreter lifetime | proposed | L | PERL-004 through PERL-006, RUST-010 | XS memory-safety engineer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-012` | Implement capability-aware auto fallback | proposed | M | PERL-002, PERL-003, BUILD-005 | integration engineer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-013` | Add backend parity test runner | proposed | M | PERL-002 through PERL-012, TEST-001 | Perl test engineer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |
| `PERL-014` | Document compatibility and escape hatches | proposed | S | PERL-002 through PERL-012 | documentation engineer | [`07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md`](07_PERL_API_AND_XS_COMPATIBILITY_TASKS.md) |

### REPORT workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `REPORT-001` | Freeze the report artifact and semantic contract | proposed | XL | BASE-005, COMPAT-004, COMPAT-006 | reporting/test architect | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) — first-slice `REPORT-MVP` (board, done) is text totals only via `crates/nytprof-report`; does not complete REPORT-001..020 |
| `REPORT-002` | Define deterministic report IR | proposed | L | ARCH-004, RUST-006 through RUST-009, REPORT-001 | Rust/report architect | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-003` | Reimplement legacy numeric/statistical calculations | proposed | L | REPORT-001, REPORT-002, BASE-003 | numerical/test engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-004` | Implement native index/summary pages | proposed | L | REPORT-002, REPORT-003 | Rust HTML engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-005` | Implement source line pages | proposed | XL | REPORT-002, REPORT-003, RUST-009 | source-rendering engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-006` | Implement block and sub-level pages | proposed | L | REPORT-005, RUST-007, RUST-008 | reporting engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-007` | Implement calls and folded-stack generation | proposed | XL | RUST-008, RUST-015, REPORT-002 | call-stack algorithms engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-008` | Preserve flame graph integration | proposed | L | REPORT-007, BASE-005 | reporting/tool integration engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-009` | Implement graph data and Graphviz integration | proposed | L | RUST-008, REPORT-002 | graph/report engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-010` | Implement deterministic parallel render scheduler | proposed | L | REPORT-004 through REPORT-009, RUST-017 | Rust concurrency engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-011` | Implement atomic output publication | proposed | M | REPORT-010, TOOL-001 | filesystem/reliability engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-012` | Implement native CSV renderer | proposed | M | REPORT-002, REPORT-003, BASE-005 | data-export engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-013` | Implement native Callgrind renderer | proposed | L | REPORT-002, RUST-008, BASE-005 | profiling-format engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-014` | Implement renderer compatibility callbacks/fallback | proposed | L | PERL-008, REPORT-004 through REPORT-006 | Perl/Rust report integration engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-015` | Reduce compatibility-mode output duplication | proposed | M | REPORT-001, REPORT-004 through REPORT-006, BENCH-009 | web/performance engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-016` | Design optional compact report mode | proposed | L | REPORT-004 through REPORT-015, ADR report policy | web architecture engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-017` | Add report phase/cache invalidation | proposed | M | RUST-014, REPORT-002, TOOL-008 | caching/data-integrity engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-018` | Add report telemetry and hotspot regression | proposed | M | RUST-016, REPORT-010 | performance engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-019` | Add visual and accessibility regression suite | proposed | L | REPORT-004 through REPORT-016, TEST-009 | browser/test engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |
| `REPORT-020` | Make native report engine independently selectable | proposed | M | REPORT-004 through REPORT-014, PERL-002, TOOL-001 | integration engineer | [`08_REPORT_GENERATION_TASKS.md`](08_REPORT_GENERATION_TASKS.md) |

### TOOL workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `TOOL-001` | Build unified native CLI framework | proposed | L | RUST-001, RUST-012, BASE-005 | Rust CLI engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-002` | Implement canonical event dump | proposed | L | COMPAT-001, COMPAT-002, RUST-004, RUST-011 | compatibility tooling engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-003` | Implement semantic profile comparator | proposed | L | TOOL-002 | test tooling engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-004` | Implement v5 to v6 converter | proposed | XL | RUST-004, RUST-005, RUST-011, FMT-013 | format compatibility engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-005` | Implement strict v6 to v5 converter | proposed | XL | RUST-011, COL-006 or Rust v5 encoder, FMT-013 | format compatibility engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-006` | Implement format inspector | proposed | M | RUST-004, RUST-011, FMT-014 | tooling engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-007` | Implement validator and salvage command | proposed | L | RUST-003, RUST-004, RUST-011, SEC-003 | reliability tooling engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-008` | Implement optional index/summary builder | proposed | L | FMT-011, RUST-014 | data tooling engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-009` | Implement mixed v5/v6 merge command | proposed | XL | RUST-013, BASE-005 | merge/data engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-010` | Add existing CLI wrappers over native engine | proposed | L | TOOL-001, REPORT-020, TOOL-009, REPORT-007, REPORT-012, REPORT-013 | Perl/Rust CLI integration engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-011` | Preserve `nytprofcalls` modes and streaming behavior | proposed | L | REPORT-007, TOOL-010 | call tooling engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-012` | Preserve `nytprofcg` and `nytprofcsv` | proposed | M | REPORT-012, REPORT-013, TOOL-010 | export tooling engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-013` | Add profile feature/capability negotiation command | proposed | S | ARCH-006, TOOL-006 | tooling engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-014` | Add converter provenance and reproducibility manifest | proposed | M | TOOL-004, TOOL-005 | release/data integrity engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-015` | Add tool-level resource limits | proposed | M | SEC-001, TOOL-001 | security/CLI engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |
| `TOOL-016` | Add machine-readable exit/error taxonomy | proposed | S | TOOL-001, PERL-003 | CLI API engineer | [`09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md) |

### TEST workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `TEST-001` | Build unified oracle test harness | done | XL | BASE-001, COMPAT-001, COMPAT-002 | test architecture engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-002` | Define versioned canonical event schema | done | L | COMPAT-001, TOOL-002 | format/test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-003` | Build deterministic clock backend and scripts | proposed | L | BASE-003, COL-001 | C test instrumentation engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-004` | Import and stabilize upstream test suite | proposed | M | BASE-001, TEST-001 | Perl test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-005` | Create v5 golden profile corpus | proposed | XL | BASE-002 through BASE-007, TEST-001 | fixture engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-006` | Add v5 reader differential suite | proposed | L | RUST-004, RUST-005, TEST-002, TEST-005 | Rust/Perl differential test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-007` | Add collector v5 compatibility suite | proposed | L | COL-006, TEST-005 | C/XS test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-008` | Add same-run dual writer suite | proposed | XL | COL-014, TOOL-003, TEST-003, TEST-005 | compatibility test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-009` | Build normalized report comparison framework | proposed | XL | REPORT-001, TEST-001 | web/report test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-010` | Add Data/API structural snapshot suite | proposed | L | PERL-001, TEST-001 | Perl API test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-011` | Add CLI black-box contract suite | proposed | L | BASE-005, TOOL-010 | CLI test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-012` | Add conversion round-trip suite | proposed | L | TOOL-004, TOOL-005, TEST-002, TEST-005 | format test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-013` | Add mixed merge differential suite | proposed | L | TOOL-009, TEST-005 | merge test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-014` | Add corruption/truncation recovery matrix | proposed | XL | TOOL-007, FMT-010, SEC-003 | reliability test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-015` | Add property-based event-stream round trips | proposed | L | RUST-011, TEST-002, and the writer selected by COL-009 | property test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-016` | Add parser/FFI fuzzing program | proposed | L | RUST-018, PERL-011, SEC-002 | security fuzz engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-017` | Add cross-platform v5 NV/endian corpus | proposed | XL | RUST-005, BUILD-006 | portability engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-018` | Add fork/process stress suite | proposed | XL | COL-015, TEST-003 | process/concurrency test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-019` | Add long-duration/overflow suite | proposed | L | COMPAT-003, COL-011, RUST-007, RUST-008, TEST-003 | numerical test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |
| `TEST-020` | Add release compatibility matrix runner | proposed | L | TEST-004 through TEST-019, BUILD-006 | release test engineer | [`10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md`](10_REGRESSION_DIFFERENTIAL_AND_FIXTURE_TASKS.md) |

### BENCH workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `BENCH-001` | Build reproducible benchmark harness and ratify gates | proposed | XL | BASE-001, BASE-007 | performance test architect | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) — light noise notes only: [`docs/BENCH_NOTES.md`](../BENCH_NOTES.md) (not harness/certification) |
| `BENCH-002` | Capture event-distribution and locality corpus | proposed | L | BASE-002, BASE-007, COL-016 or legacy instrumentation | workload/performance engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |
| `BENCH-003` | Build collector hot-path microbenchmark | proposed | L | BASE-003, COL-001 | low-level performance engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |
| `BENCH-004` | Measure legacy v5 writer components | proposed | M | BENCH-001, COL-016 | C performance engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |
| `BENCH-005` | Evaluate dictionaries/deltas/exact runs/source dedup | proposed | L | BENCH-002, FMT-005 through FMT-008, COL-010 through COL-013 | compression/performance engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |
| `BENCH-006` | Compare C and batched Rust v6 writers | proposed | L | COL-007, BENCH-001, BUILD-004; COL-008 only if re-opened | systems performance engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |
| `BENCH-007` | Select codec and chunk defaults | proposed | L | FMT-015, BENCH-001, BENCH-002 | compression engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |
| `BENCH-008` | Measure Rust v5/v6 decode and model memory | proposed | L | RUST-004 through RUST-009, RUST-016, BENCH-001 | Rust performance engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |
| `BENCH-009` | Benchmark native report pipeline and output size | proposed | L | REPORT-004 through REPORT-014, BENCH-001 | report performance engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |
| `BENCH-010` | Tune deterministic worker scheduling | proposed | M | REPORT-010, RUST-017, BENCH-009 | concurrency performance engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |
| `BENCH-011` | Benchmark exact index/summary acceleration | proposed | M | RUST-014, TOOL-008, REPORT-017 | data performance engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |
| `BENCH-012` | Benchmark conversion and mixed merge | proposed | M | TOOL-004, TOOL-005, TOOL-009 | tooling performance engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |
| `BENCH-013` | Run real-application canary benchmarks | proposed | XL | BENCH-001 through BENCH-012, TEST-020 prerequisite correctness rows | performance/release engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |
| `BENCH-014` | Add continuous performance regression monitoring | proposed | L | BENCH-001, BUILD-006 | CI/performance engineer | [`11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](11_BENCHMARKING_AND_PERFORMANCE_GATES.md) |

### BUILD workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `BUILD-001` | Ratify platform, Perl, compiler, and Rust support policy | proposed | L | COMPAT-000, BASE-001 | maintainer/release architect | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-002` | Add Rust workspace without changing legacy build | proposed | M | BUILD-001, RUST-001 | build engineer | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-003` | Integrate optional Cargo build with MakeMaker | proposed | XL | BUILD-001, BUILD-002 | Perl/Cargo build engineer | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-004` | Package and load stable native library/CLI | proposed | XL | RUST-010, BUILD-003 | systems packaging engineer | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-005` | Implement capability manifest and runtime self-test | proposed | M | ARCH-006, BUILD-004 | integration engineer | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-006` | Build CI compatibility matrix | proposed | XL | BUILD-001 through BUILD-005 | CI/release engineer | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-007` | Automate C header generation and ABI verification | proposed | M | RUST-010, BUILD-002 | FFI build engineer | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-008` | Integrate codec dependencies safely | proposed | L | FMT-015, BENCH-007, BUILD-001 | dependency/build engineer | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-009` | Add sanitizer and debug configurations | proposed | L | BUILD-003, RUST-010 | toolchain engineer | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-010` | Add reproducible source and artifact packaging | proposed | L | BUILD-003, BUILD-004 | release engineer | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-011` | Add dependency and license governance | proposed | M | BUILD-002, BUILD-008 | supply-chain engineer | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-012` | Test CPAN/client installation fallback | proposed | L | BUILD-003 through BUILD-005 | packaging test engineer | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-013` | Define versioning across Perl dist, crates, ABI, and format | proposed | M | BUILD-001, FMT-002, RUST-010 | release architect | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-014` | Implement staged feature flags and defaults | proposed | M | BUILD-005, PERL-002, REPORT-020, COL-014 | release integration engineer | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |
| `BUILD-015` | Create release evidence bundle | proposed | M | TEST-020, BENCH-013, BUILD-006, SEC-012 | release manager agent | [`12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md`](12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) |

### SEC workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `SEC-001` | Define threat model and global resource-limit API | proposed | L | ARCH-003, BUILD-001 | security architect | [`13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md) |
| `SEC-002` | Establish continuous fuzzing strategy | proposed | L | RUST-003, BUILD-009 | security fuzz engineer | [`13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md) |
| `SEC-003` | Specify checksum, framing, and salvage rules | proposed | L | FMT-002, SEC-001 | binary reliability architect | [`13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md) |
| `SEC-004` | Harden decompression and codec boundaries | proposed | L | SEC-001, FMT-009, BUILD-008 | compression security engineer | [`13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md) |
| `SEC-005` | Audit integer arithmetic and indexing | proposed | L | RUST-003, RUST-006, FMT-004 | secure numerical engineer | [`13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md) |
| `SEC-006` | Harden HTML/source rendering | proposed | L | REPORT-005, SEC-001 | application security engineer | [`13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md) |
| `SEC-007` | Harden filesystem paths and atomic output | proposed | L | REPORT-011, TOOL-001, SEC-001 | filesystem security engineer | [`13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md) |
| `SEC-008` | Audit C/Rust/Perl FFI and unsafe code | proposed | XL | RUST-010, PERL-011, BUILD-009 | senior memory-safety reviewer | [`13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md) |
| `SEC-009` | Validate optional summary/index trust | proposed | M | RUST-014, TOOL-008 | data integrity engineer | [`13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md) |
| `SEC-010` | Harden conversion and merge against hostile inputs | proposed | L | TOOL-004, TOOL-005, TOOL-009, SEC-001 | security tooling engineer | [`13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md) |
| `SEC-011` | Protect sensitive embedded source and metadata | proposed | M | SEC-007, BUILD-001 | security/privacy engineer | [`13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md) |
| `SEC-012` | Complete security/reliability release review | proposed | L | SEC-001 through SEC-011, TEST-014 through TEST-019 | independent security reviewer | [`13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md`](13_SECURITY_CORRUPTION_AND_RECOVERY_TASKS.md) |

### REL workstream

| ID | Task | Status | Size | Dependencies | Suggested owner | Source |
|---|---|---|---:|---|---|---|
| `REL-001` | Define release levels and compatibility windows | proposed | L | BUILD-001, COMPAT-009 through COMPAT-011, `15_PHASES_DEPENDENCIES_AND_CRITICAL_PATH.md` | release/product lead | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |
| `REL-002` | Build migration and interoperability documentation | proposed | L | COMPAT-005, COMPAT-006, COMPAT-008, COMPAT-009, TOOL-004 through TOOL-010, PERL-014 | technical documentation + compatibility agent | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |
| `REL-003` | Add release-visible capability and provenance reporting | proposed | M | TOOL-006, TOOL-013, TOOL-014, BUILD-005, BUILD-013 | diagnostics/release agent | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |
| `REL-004` | Build release-candidate artifact and evidence bundle | proposed | XL | BUILD-015, TEST-020, BENCH-013, SEC-012 | release engineering lead | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |
| `REL-005` | Ship native v5 reporting as opt-in | proposed | L | Phase 2 exit, REPORT-020, COMPAT-014, BUILD-015, SEC-012, TEST-020, BENCH-013 | release lead | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |
| `REL-006` | Evaluate and promote native reporting in `auto` | proposed | L | REL-005 field window, ADR-Q024, COMPAT-014, TEST-020, BENCH-013 | release review group | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |
| `REL-007` | Ship stable v6 collection as opt-in | proposed | XL | Phase 4 exit, COL-009 through COL-015, TOOL-004 through TOOL-009, TOOL-013, TOOL-016, COMPAT-014, BUILD-015, SEC-012, TEST-020, BENCH-013 | release lead with collector/format leads | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |
| `REL-008` | Evaluate and promote v6 output default | proposed | XL | REL-007 field window, ADR-Q025, COMPAT-014, TEST-020, BENCH-013, SEC-012 | independent release review group | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |
| `REL-009` | Add post-release field validation and incident process | proposed | L | REL-005 or REL-007 | maintenance/release lead | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |
| `REL-010` | Define telemetry/privacy policy | proposed | M | COL-016, REL-003 | privacy/release agent | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |
| `REL-011` | Maintain cross-release compatibility tests | proposed | L | COMPAT-012, BUILD-006, BUILD-012, TEST-020 | release QA agent | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |
| `REL-012` | Run legacy retirement/deprecation review | deferred | XL | sustained R4 field use, ADR-Q026 | ecosystem/release review group | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |
| `REL-013` | Final modernization program review | proposed | L | advertised target release level achieved, COMPAT-014, REL-004, REL-009, all relevant sign-offs | independent program review board | [`19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) |

## Validation status

- Unique task identifiers: **206**.
- Duplicate task identifiers: **0**.
- Unresolved exact task-ID references: **0**.
- Risk-register entries: **30**.
- Open ADR queue entries: **26**.
- Matrix rows (`M1`-`M10`), correctness/performance gates, phase names, work packages, risks, and ADRs use separate namespaces and are not treated as task dependencies unless explicitly mapped by a task.

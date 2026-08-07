# Benchmarking and Performance Gate Task Plan

## 1. Objective

Measure whether the modernization improves collection runtime, storage, report runtime, and report memory without changing precision, event coverage, or enabled features. Performance conclusions must compare equivalent configurations and must follow correctness gates.

## 2. Measurement principles

1. Compare the same application, Perl build, clock, NYTPROF options, input, CPU affinity, and environment.
2. Report unprofiled, legacy-v5, new-v5, and new-v6 results separately.
3. Keep calls, statements, blocks, source, slow-op, compression, and report options identical unless the experiment explicitly varies one factor.
4. Do not claim an improvement by disabling a feature or changing output semantics.
5. Separate warm-up, startup, collection, finalization, parsing, aggregation, rendering, and external-tool time.
6. Store raw samples and machine metadata, not only averages.
7. Use enough repetitions and robust statistics; publish confidence intervals and outliers.
8. Check canonical event/report equality before accepting each benchmark sample set.
9. Measure peak RSS and allocations as well as wall time.
10. Re-run default-switch gates on dedicated, quiet runners.

## 3. Metrics

### Collection

- wall and CPU time versus unprofiled execution;
- absolute and percentage profiler overhead;
- events captured per second by type;
- clock reads and sink calls per event;
- buffer flushes and write syscalls;
- raw bytes and compressed bytes per logical event;
- compression/checksum CPU;
- dictionary hit/miss and memory;
- peak RSS and high-water buffer size;
- startup/finalization cost;
- parent/child overhead for fork workloads.

### Reading and reporting

- decode, decompress, aggregate, model, IR, render, and external-tool time;
- peak RSS and retained model bytes;
- allocations/objects where measurable;
- throughput in events/sec and source lines/sec;
- output file count and total bytes;
- worker scaling and CPU utilization;
- cold/warm cache and optional index effects;
- small-profile startup overhead.

### Correctness paired metrics

Every performance row links to:

- canonical event comparison checksum/status;
- aggregate-model comparison status;
- report comparison status where applicable;
- exact options and feature flags.

## 4. Workload corpus

### Micro workloads

- empty/tiny program;
- one hot statement repeated N times;
- alternating lines and files;
- deep statement blocks;
- rapid calls to one sub;
- many unique sub names;
- recursion and mutual recursion;
- calls=0/1/2;
- source-heavy compile with little runtime;
- slow-op-heavy I/O/regex fixtures using controlled data;
- frequent start/stop and file switches;
- fork around full buffers.

### Synthetic scale workloads

- 1M, 10M, and larger statement events;
- millions of call events;
- thousands of files/subs;
- large embedded source;
- repeated identical source across processes;
- high-entropy names/locations that reduce compression benefit;
- chunk/dictionary boundary stress.

### Real applications

Select redistributable or scripted workloads representing:

- command-line data processing;
- test suite execution;
- template/web-style application startup and requests;
- object-heavy framework code;
- regex/text processing;
- forked workers;
- large report generation from archived production-like profiles.

Record exact versions and inputs. Avoid network dependence in benchmark runs.

## 5. Experimental matrices

### Encoding matrix

- v5 zlib levels currently supported;
- v6 none/zlib/zstd/LZ4 candidates;
- chunk sizes and reset frequency;
- dictionary on/off;
- deltas on/off;
- exact run encoding on/off;
- source dedup on/off;
- checksum choices;
- C writer versus batched Rust writer.

### Report matrix

- legacy versus native reader/report;
- v5 versus v6 input;
- 1..N workers;
- raw replay versus validated exact summary/index;
- compatibility versus optional compact rendering;
- HTML only versus all auxiliary outputs;
- cold and warm filesystem cache.

## 6. Provisional gates

The contract file contains provisional project thresholds. `BENCH-001` must ratify or replace them before a default changes. Correctness remains absolute.

Recommended promotion rules:

- no statistically significant geometric-mean regression for new v5 collection;
- no unexplained workload regression above the approved tail threshold;
- v6 must improve at least one primary dimension without materially regressing the others;
- native report must materially reduce wall time and peak RSS for large profiles;
- tiny-profile startup regressions require explicit threshold and justification;
- size claims use the same exact logical event stream and source/call options;
- benchmark noise greater than the claimed gain invalidates the conclusion.

## 7. Benchmark tasks

### BENCH-001 - Build reproducible benchmark harness and ratify gates

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BASE-001, BASE-007
- **Agent:** performance test architect
- **Work:** runner isolation, repetitions, affinity, metadata, raw samples, statistics, correctness hooks, JSON schema, visualization.
- **Deliverables:** harness, baseline database, approved thresholds ADR.
- **Acceptance:** reruns on reference runner remain within documented noise; all claims can be reproduced from raw data.

### BENCH-002 - Capture event-distribution and locality corpus

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-002, BASE-007, COL-016 or legacy instrumentation
- **Agent:** workload/performance engineer
- **Work:** counts by type, file/line/depth deltas, name/source repetition, run lengths, call depths, payload sizes, fork patterns.
- **Deliverables:** anonymized/distributable histograms and representative traces.
- **Acceptance:** format/dictionary designs are based on measured distributions rather than synthetic assumptions.

### BENCH-003 - Build collector hot-path microbenchmark

- **Status:** proposed
- **Size:** L
- **Dependencies:** BASE-003, COL-001
- **Agent:** low-level performance engineer
- **Work:** isolate hook/event append, clock, direct/indirect sink dispatch, buffer capacity, compiler settings; inspect assembly/perf counters.
- **Deliverables:** microbenchmark and baseline.
- **Acceptance:** detects sub-percent per-event changes with acceptable confidence on dedicated runner.

### BENCH-004 - Measure legacy v5 writer components

- **Status:** proposed
- **Size:** M
- **Dependencies:** BENCH-001, COL-016
- **Agent:** C performance engineer
- **Work:** serialization, zlib, memcpy, write calls, source/finalization, calls/NV conversion.
- **Deliverables:** cost attribution report.
- **Acceptance:** dominant collection costs are quantified for each workload class.

### BENCH-005 - Evaluate dictionaries/deltas/exact runs/source dedup

- **Status:** proposed
- **Size:** L
- **Dependencies:** BENCH-002, FMT-005 through FMT-008, COL-010 through COL-013
- **Agent:** compression/performance engineer
- **Work:** factorial A/B tests for size, CPU, memory, startup/finalization; include adversarial high-entropy inputs.
- **Deliverables:** feature recommendation and thresholds for auto-disable if any.
- **Acceptance:** each adopted encoding has measured end-to-end value and exact comparison pass.

### BENCH-006 - Compare C and batched Rust v6 writers

- **Status:** proposed
- **Size:** L
- **Dependencies:** COL-007, BENCH-001, BUILD-004; COL-008 only if re-opened (COL-008 is non-baseline deferred)
- **Agent:** systems performance engineer
- **Work:** batch sizes, FFI copies, compression, code size, RSS, platform/build availability, failure paths.
- **Deliverables:** raw results feeding COL-009 ADR.
- **Acceptance:** equivalent format features and correctness; no per-event FFI in either measurement.

### BENCH-007 - Select codec and chunk defaults

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-015, BENCH-001, BENCH-002
- **Agent:** compression engineer
- **Work:** none/zlib/zstd/LZ4 levels, chunk sizes, dictionary snapshots, checksum cost on all workload classes.
- **Deliverables:** default and fallback policy with small/large profile behavior.
- **Acceptance:** recommendation balances collector CPU, file size, report decode, recovery, and packaging.

### BENCH-008 - Measure Rust v5/v6 decode and model memory

- **Status:** proposed
- **Size:** L
- **Dependencies:** RUST-004 through RUST-009, RUST-016, BENCH-001
- **Agent:** Rust performance engineer
- **Work:** decode/decompress/aggregate allocation profiles, sparse/dense thresholds, interning, source/call retention modes.
- **Deliverables:** hotspot and RSS report.
- **Acceptance:** regressions are attributed; large-model target is evaluated against legacy object amplification.

### BENCH-009 - Benchmark native report pipeline and output size

- **Status:** proposed
- **Size:** L
- **Dependencies:** REPORT-004 through REPORT-014, BENCH-001
- **Agent:** report performance engineer
- **Work:** phase timings, RSS, workers, file I/O, asset duplication, external tools, compatibility/compact mode.
- **Deliverables:** scaling curves and output-size breakdown.
- **Acceptance:** report performance gates use equality-passing outputs and identify small-profile startup cost.

### BENCH-010 - Tune deterministic worker scheduling

- **Status:** proposed
- **Size:** M
- **Dependencies:** REPORT-010, RUST-017, BENCH-009
- **Agent:** concurrency performance engineer
- **Work:** worker counts, task granularity, I/O contention, memory high-water, CPU quotas.
- **Deliverables:** auto/default worker policy.
- **Acceptance:** deterministic output and no RSS explosion; scaling recommendation per profile size.

### BENCH-011 - Benchmark exact index/summary acceleration

- **Status:** proposed
- **Size:** M
- **Dependencies:** RUST-014, TOOL-008, REPORT-017
- **Agent:** data performance engineer
- **Work:** index build cost/size, raw versus indexed report, invalidation, repeated reports.
- **Deliverables:** enablement policy.
- **Acceptance:** cached and raw results identical; total lifecycle benefit documented.

### BENCH-012 - Benchmark conversion and mixed merge

- **Status:** proposed
- **Size:** M
- **Dependencies:** TOOL-004, TOOL-005, TOOL-009
- **Agent:** tooling performance engineer
- **Work:** throughput, memory, temp storage, streaming behavior, many-input merges, source dedup.
- **Deliverables:** scaling report and resource-limit recommendations.
- **Acceptance:** tools remain bounded as designed and meet operational targets set by maintainers.

### BENCH-013 - Run real-application canary benchmarks

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BENCH-001 through BENCH-012, TEST-020 prerequisite correctness rows
- **Agent:** performance/release engineer
- **Work:** representative applications, equivalent options, multiple Perl/platforms, correctness verification, regression triage.
- **Deliverables:** canary dashboard and signed promotion report.
- **Acceptance:** default-change thresholds pass with no unexplained outlier.

### BENCH-014 - Add continuous performance regression monitoring

- **Status:** proposed
- **Size:** L
- **Dependencies:** BENCH-001, BUILD-006
- **Agent:** CI/performance engineer
- **Work:** scheduled dedicated-runner jobs, baseline versioning, control charts, alert thresholds, artifact retention.
- **Deliverables:** dashboard and triage procedure.
- **Acceptance:** seeded regression triggers alert; noisy shared CI does not gate releases falsely.

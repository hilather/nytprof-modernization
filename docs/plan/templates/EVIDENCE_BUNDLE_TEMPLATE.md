# Evidence Bundle - TASK/WP/Release ID

For the charter **R3** `engine=auto` **field window** (opt-in evidence only; **no product default flip**), prefer the specialized pack and report:

- Guide: [docs/R3_FIELD_WINDOW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md)
- Report template: [docs/templates/R3_FIELD_WINDOW_REPORT.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R3_FIELD_WINDOW_REPORT.md)
- Collector: `./scripts/field/r3_field_window_collect.sh`
- Schema: [docs/schemas/r3-field-window-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r3-field-window-mvp-v0.md)

This generic template remains the release-candidate / multi-gate evidence bundle shape (REL-004 scale).

## Identity and provenance

- Source repository and commit:
- Oracle manifest/version:
- Candidate build manifest/version:
- Spec/schema/vector versions:
- Date and operator/agent:
- Artifact root and checksum manifest:

## Environment

Record OS/architecture/libc, Perl `-V`, compiler/linker, Rust/Cargo, zlib/codecs, CPU/memory/storage, locale/timezone, power/governor/container/VM, and relevant environment variables.

## Exact feature configuration

Record all `NYTPROF` options, engine/format/codec/chunk/report options, input workload/version/checksum, process/fork topology, and expected event counts. State how equality is checked before performance data is accepted.

## Commands

Provide copy-paste build, collection, validation, conversion/merge, report, test, fuzz, and benchmark commands. Include repetitions, warmups, timeouts, and random seeds.

## Correctness artifacts

- canonical event streams and hashes;
- normalized model/API/CLI/report results;
- immutable fixture/vector IDs;
- first-mismatch bundles;
- malformed/fault/recovery outputs;
- old/new producer-consumer matrix results.

## Performance/storage raw results

Store per-run wall/CPU/RSS/allocation/I/O/bytes/compression/event metrics rather than summaries alone. Include rejected outliers with policy and reason.

## Statistical summary

Report sample count, median/mean as approved, spread/confidence interval, effect size, noise floor, workload-level regressions, and aggregate method. Never combine unequal feature configurations.

## Security/reliability results

Include sanitizer/fuzz duration and corpus, resource-limit checks, decompression ratios/work limits, arithmetic/FFI reviews, fork/finalization/fault tests, filesystem/HTML checks, and dependency/SBOM outputs.

## Decision and sign-off

State which acceptance/performance/security gates pass or fail, waivers/ADRs, reviewers, and next action. Attach `SHA256SUMS` covering the complete bundle.

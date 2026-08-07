# Evidence Bundle - TASK/WP/Release ID

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

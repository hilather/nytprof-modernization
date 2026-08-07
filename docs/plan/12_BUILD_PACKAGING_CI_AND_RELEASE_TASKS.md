# Build, Packaging, CI, and Release Task Plan

## 1. Objective

Introduce Rust and optional new codecs without breaking installation or operation on existing supported Perl environments. Keep a complete legacy fallback during migration and make backend capabilities explicit.

## 2. Support-policy constraint

Devel::NYTProf 6.15 uses ExtUtils::MakeMaker and declares support for old Perl versions. Many such systems will not have a Rust toolchain or a supported prebuilt binary. Therefore the initial architecture must not make Rust mandatory for basic v5 collection/report compatibility.

A future support-policy change is possible only through a separate ADR and major/minor release decision with migration guidance.

## 3. Repository/build layout

Recommended layout:

```text
Makefile.PL
NYTProf.xs / FileHandle.xs / C collector sources
lib/Devel/NYTProf/*.pm
bin/* compatibility wrappers
rust/
  Cargo.toml
  crates/*
include/
  nytprof_event.h
  nytprof_ffi.h (generated/verified)
xt/
  compatibility, fuzz, performance, release tests
```

MakeMaker remains the top-level CPAN entry point. Cargo builds an optional native library/binary when enabled and available.

## 4. Packaging modes

| Mode | Contents | Use |
|---|---|---|
| Legacy source build | C/XS + Perl only | Old platforms, fallback, oracle |
| Native source build | C/XS + Perl + Rust | Development and supported toolchains |
| Optional native binary package | Verified platform-specific native engine plus Perl facade | Faster install where policy permits |
| Standalone tooling | Rust CLI reads profiles without Perl embedding | Report/conversion deployments |

No package may claim native support without a capability/version self-test.

## 5. Build selection

Suggested controls, subject to ADR:

```text
NYTPROF_NATIVE=auto|0|1
NYTPROF_CODEC_ZSTD=auto|0|1
NYTPROF_CODEC_LZ4=auto|0|1
```

- `auto`: attempt only on supported configurations; fall back cleanly.
- `0`: legacy-only build.
- `1`: require feature and fail configure/build if unavailable.

Build logs must state what was built and why a feature was skipped.

## 6. ABI/version policy

- Version the C event-sink ABI and Rust FFI ABI independently from file format.
- Embed ABI major/minor and build ID in the native library.
- Runtime checks occur before handle creation.
- Generated headers are checked into release tarballs or reproducibly generated.
- No Rust symbol/struct layout crosses the ABI directly.
- A panic/unwind never crosses into C/Perl.

## 7. CI dimensions

At minimum:

- representative supported Perl versions;
- 32/64-bit where feasible;
- Linux/macOS/Windows or the project-approved platform set;
- GCC/Clang/MSVC as applicable;
- legacy-only, native, and require-native builds;
- zlib-only and optional codec combinations;
- release/debug/sanitizer builds;
- legacy/native backends and v5/v6/dual tests;
- no-network packaging install test;
- minimum supported Rust version and current stable;
- cross-platform fixtures for unsupported layouts.

Large performance/fuzz jobs can be scheduled separately from pull-request smoke tests.

## 8. Release stages

1. **Development-only Rust v5 reader/tooling.** No default changes.
2. **Opt-in native v5 reports.** Legacy remains default and comparator.
3. **Experimental v6 writer/dual mode.** Explicit flag only.
4. **Supported v6 opt-in.** Converter and recovery tools complete.
5. **Native report auto preference.** Only after C4/C5 and packaging gates.
6. **v6 default consideration.** Only after real-world canaries and old-tool conversion support.
7. **Legacy retirement consideration.** Separate future policy; not part of this plan.

## 9. Build and release tasks

### BUILD-001 - Ratify platform, Perl, compiler, and Rust support policy

- **Status:** proposed
- **Size:** L
- **Dependencies:** COMPAT-000, BASE-001
- **Agent:** maintainer/release architect
- **Work:** current supported matrix, tier definitions, fallback, MSRV, prebuilt policy, end-of-life process.
- **Deliverables:** support ADR and CI matrix.
- **Acceptance:** every build mode has defined expected behavior on each tier.

### BUILD-002 - Add Rust workspace without changing legacy build

- **Status:** proposed
- **Size:** M
- **Dependencies:** BUILD-001, RUST-001
- **Agent:** build engineer
- **Work:** workspace, lockfile policy, source distribution inclusion, standalone build, license metadata.
- **Deliverables:** Cargo build and tests.
- **Acceptance:** legacy `perl Makefile.PL && make test` remains unchanged when native disabled/unavailable.

### BUILD-003 - Integrate optional Cargo build with MakeMaker

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BUILD-001, BUILD-002
- **Agent:** Perl/Cargo build engineer
- **Work:** configure detection, feature controls, build paths, install locations, clean/distclean, parallel builds, diagnostics.
- **Deliverables:** MakeMaker integration.
- **Acceptance:** auto/disabled/required modes behave as specified; source tarball installs offline with declared dependencies.

### BUILD-004 - Package and load stable native library/CLI

- **Status:** proposed
- **Size:** XL
- **Dependencies:** RUST-010, BUILD-003
- **Agent:** systems packaging engineer
- **Work:** shared/static choice, rpath/DLL lookup, symbol visibility, ABI self-test, executable discovery, relocatable install.
- **Deliverables:** native artifacts and loader tests.
- **Acceptance:** Perl facade finds the matching engine without unsafe search paths; mismatch fails/falls back clearly.

### BUILD-005 - Implement capability manifest and runtime self-test

- **Status:** proposed
- **Size:** M
- **Dependencies:** ARCH-006, BUILD-004
- **Agent:** integration engineer
- **Work:** file formats, ABI, codecs, index versions, target triple, Perl integration capabilities; quick self-test.
- **Deliverables:** machine-readable manifest/API.
- **Acceptance:** backend selection can decide support before parsing/reporting.

### BUILD-006 - Build CI compatibility matrix

- **Status:** proposed
- **Size:** XL
- **Dependencies:** BUILD-001 through BUILD-005
- **Agent:** CI/release engineer
- **Work:** platform/Perl/compiler/features/backends/formats, caching, artifact retention, oracle builds.
- **Deliverables:** CI workflows and coverage dashboard.
- **Acceptance:** required matrix rows are enforced and skips are explicit.

### BUILD-007 - Automate C header generation and ABI verification

- **Status:** proposed
- **Size:** M
- **Dependencies:** RUST-010, BUILD-002
- **Agent:** FFI build engineer
- **Work:** cbindgen or manual generated header policy, checked-in snapshot, C compile tests, size/alignment/static assertions.
- **Deliverables:** header generation/check command.
- **Acceptance:** CI fails on unreviewed ABI drift; release tarball contains usable headers.

### BUILD-008 - Integrate codec dependencies safely

- **Status:** proposed
- **Size:** L
- **Dependencies:** FMT-015, BENCH-007, BUILD-001
- **Agent:** dependency/build engineer
- **Work:** system versus bundled libraries, feature flags, licenses, CVE updates, static/dynamic linkage, zlib fallback.
- **Deliverables:** codec policy and build code.
- **Acceptance:** unsupported codec yields clear runtime error; legacy zlib-only build remains supported per policy.

### BUILD-009 - Add sanitizer and debug configurations

- **Status:** proposed
- **Size:** L
- **Dependencies:** BUILD-003, RUST-010
- **Agent:** toolchain engineer
- **Work:** ASan/UBSan/LSan, Perl debug builds, Rust sanitizers where available, valgrind jobs, assertions/fault injection.
- **Deliverables:** CI/debug recipes.
- **Acceptance:** collector, parsers, FFI, fork, and error suites run under applicable tools.

### BUILD-010 - Add reproducible source and artifact packaging

- **Status:** proposed
- **Size:** L
- **Dependencies:** BUILD-003, BUILD-004
- **Agent:** release engineer
- **Work:** deterministic file list, timestamps where possible, checksums/SBOM, generated files, no undeclared network fetch.
- **Deliverables:** release build script and verification.
- **Acceptance:** two clean builds produce equivalent source distributions and traceable native artifacts within documented toolchain limits.

### BUILD-011 - Add dependency and license governance

- **Status:** proposed
- **Size:** M
- **Dependencies:** BUILD-002, BUILD-008
- **Agent:** supply-chain engineer
- **Work:** dependency allowlist, lockfile updates, license scan, advisories, minimal features, vendoring policy.
- **Deliverables:** policy and CI checks.
- **Acceptance:** every shipped dependency has compatible license, owner, update path, and recorded rationale.

### BUILD-012 - Test CPAN/client installation fallback

- **Status:** proposed
- **Size:** L
- **Dependencies:** BUILD-003 through BUILD-005
- **Agent:** packaging test engineer
- **Work:** no Cargo, old compiler, read-only/home paths, staged installs, DESTDIR, local::lib, Windows paths, failed native compile.
- **Deliverables:** install matrix.
- **Acceptance:** auto mode either installs a working legacy distribution or fails only for a genuine base build error.

### BUILD-013 - Define versioning across Perl dist, crates, ABI, and format

- **Status:** proposed
- **Size:** M
- **Dependencies:** BUILD-001, FMT-002, RUST-010
- **Agent:** release architect
- **Work:** version fields, compatibility ranges, prerelease labels, user diagnostics, converter provenance.
- **Deliverables:** version policy ADR.
- **Acceptance:** tools can explain exactly which producer/format/ABI/backend generated or consumed an artifact.

### BUILD-014 - Implement staged feature flags and defaults

- **Status:** proposed
- **Size:** M
- **Dependencies:** BUILD-005, PERL-002, REPORT-020, COL-014
- **Agent:** release integration engineer
- **Work:** experimental warnings, opt-in/auto/default states, environment/CLI controls, rollback switches.
- **Deliverables:** release-stage configuration.
- **Acceptance:** defaults can change without removing the ability to force legacy and reproduce a regression.

### BUILD-015 - Create release evidence bundle

- **Status:** proposed
- **Size:** M
- **Dependencies:** TEST-020, BENCH-013, BUILD-006, SEC-012
- **Agent:** release manager agent
- **Work:** checksums, compatibility matrix, fuzz/sanitizer summary, benchmark raw/results, support manifest, known limits, rollback.
- **Deliverables:** versioned release dossier.
- **Acceptance:** all acceptance criteria are linked to evidence, not assertions.

# Primary Sources and Baseline References

## Purpose

Ground the architecture in Devel::NYTProf 6.15 source behavior. Version 6.15 was released March 31, 2026. The source baseline for the oracle is tag `v6.15`, commit `7578f4b`. These references are for design review and task execution; the pinned local oracle, immutable fixtures, and normative project specifications become the authoritative regression inputs.

Accessed for this plan on 2026-08-07.

## Release and repository

- Devel::NYTProf GitHub repository and v6.15 tag: <https://github.com/timbunce/devel-nytprof/tree/v6.15> (pin commit `7578f4bfb7e519908cc5431890f9121fdf60106c` in oracle manifests; short form `7578f4b`)
- Bootstrap note (2026-08-07): MetaCPAN/CPAN path `authors/id/T/TI/TIMB/Devel-NYTProf-6.15.tar.gz` returned HTTP 404 in this environment; the pin uses the GitHub tag archive `https://github.com/timbunce/devel-nytprof/archive/refs/tags/v6.15.tar.gz` (SHA-256 recorded under `baseline/6.15/`).
- MetaCPAN Devel::NYTProf distribution/release: <https://metacpan.org/dist/Devel-NYTProf>
- v6.15 Changes file: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/Changes>
- v6.15 MANIFEST: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/MANIFEST>

## Collector, format, and native data loading

- `NYTProf.xs`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/NYTProf.xs>
  - Current format constants identify profile format 5.0.
  - Contains statement/call hooks, timing/discount behavior, profile loading, and Perl data-structure construction.
- `FileHandle.xs`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/FileHandle.xs>
  - Contains buffering, zlib handling, variable-length integers, raw `NV` writing, statement/call/source record writers, and finalization behavior.
- `FileHandle.h`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/FileHandle.h>
  - Contains current record tags and native file-handle structures.

## Public streaming/data/report behavior

- `lib/Devel/NYTProf/ReadStream.pm`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/lib/Devel/NYTProf/ReadStream.pm>
  - Low-level ordered callback interface used as a compatibility oracle.
- `lib/Devel/NYTProf/Data.pm`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/lib/Devel/NYTProf/Data.pm>
  - Profile load/post-processing and public data-object behavior.
- `lib/Devel/NYTProf/Reader.pm`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/lib/Devel/NYTProf/Reader.pm>
  - Per-file/per-line report preparation and rendering behavior.
- `lib/Devel/NYTProf.pm`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/lib/Devel/NYTProf.pm>
  - User-facing options, semantics, configuration, and documentation.

## Command-line tools

- `bin/nytprofhtml`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/bin/nytprofhtml>
- `bin/nytprofcalls`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/bin/nytprofcalls>
- `bin/nytprofmerge`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/bin/nytprofmerge>
- `bin/nytprofcsv`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/bin/nytprofcsv>
- `bin/nytprofcg`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/bin/nytprofcg>

The source map task must verify every installed executable/helper in the pinned distribution rather than assuming this short list is complete.

## Build and support baseline

- `Makefile.PL`: <https://raw.githubusercontent.com/timbunce/devel-nytprof/v6.15/Makefile.PL>
  - ExtUtils::MakeMaker configuration, minimum Perl declaration, native build/dependency detection.

## Source-derived architectural observations to verify and freeze

1. The high-frequency collector is already C/XS, so language substitution alone is not the main collector optimization.
2. Statement profiling records an exact event stream; replacing it with aggregate totals would break low-level ordered-stream feature parity.
3. The current writer already uses variable-length integers, buffering, and zlib; v6 improvements must be measured against those features.
4. Repeated call/subroutine strings and native-width floating timing fields are candidates for lossless dictionary/integer-tick representation.
5. The normal load/report path creates substantial Perl object structures and serial report work, making an offline compact Rust model/report engine a high-value target.
6. Existing tools such as calls/merge consume ordered stream semantics, so raw logical events remain authoritative.
7. The distribution has broad tests and old-Perl/build constraints that require an explicit legacy-only fallback/support policy.

These are hypotheses until `BASE-*`, `COMPAT-*`, and fixture tasks freeze exact behavior and measurements.

## External technology references to evaluate during implementation

Use primary documentation for any selected dependency or language boundary, including:

- Rust Reference/Nomicon for C ABI, unwinding, ownership, and FFI.
- Official codec specifications/libraries for zlib, Zstandard, LZ4, or alternatives.
- Perl internals/XS documentation appropriate to each supported Perl version.
- ExtUtils::MakeMaker and CPAN tooling documentation.
- HTML parser/browser/security standards for report escaping/comparison.

Do not select a codec, checksum, hash, allocator, concurrency library, or packaging mechanism from generic benchmark claims. Run the NYTProf-specific benchmark and platform/security tasks first.

## Pinning procedure

`BASE-001` should create a local baseline manifest containing:

```text
upstream tag and commit
source archive URL and cryptographic checksum
release date/version
Perl -V
compiler/linker/build flags
zlib and other native library versions
OS/architecture/libc
built module and executable hashes
test logs
container/VM/image identity
```

The local pinned artifact paths, not live URLs, are used by automated regression suites.

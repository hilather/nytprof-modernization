# Feature-Parity and Regression Traceability Matrix

## Purpose and status

This is the seeded feature-parity checklist for the modernization. It prevents an optimization workstream from declaring success after validating only common statement profiles or the default HTML report.

The matrix is **provisional until `BASE-006` is complete**. `BASE-002` through `BASE-005` must add any record, option, API, CLI behavior, report artifact, platform case, or de facto public behavior found in v6.15. Removing a row requires an approved compatibility ADR; absence of documentation is not enough.

For every applicable row, parity means:

- exact logical event multiplicity and order;
- exact captured tick values under the deterministic/same-run oracle;
- identical timing attribution, discount, call, source, and process semantics;
- compatible v5 projection and unmodified v6.15 consumption where representable;
- compatible public API/CLI/report behavior;
- no unequal-feature benchmark comparison.

## Collection and timing

| Surface or mode | Required parity | Primary implementation tasks | Required evidence |
|---|---|---|---|
| Statement timing | Preserve every statement event, file/line association, ticks, counts, and attribution | `BASE-003`, `COMPAT-001`, `COL-001`, `COL-004`, `COL-005`, `COL-011`, `RUST-007` | `TEST-003`, `TEST-005`, `TEST-007`, `TEST-008`, `TEST-019`; M1-M6 |
| Block timing | Preserve block/sub-line fields, discount behavior, and report totals | `BASE-002`, `BASE-003`, `COL-001`, `COL-012`, `RUST-007`, `REPORT-006` | block fixtures under every writer/reader; M2-M6; normalized report values |
| Subroutine aggregate profiling | Preserve calls, inclusive/exclusive/recursive totals, caller/callee edges, and locations | `BASE-003`, `RUST-008`, `REPORT-003`, `REPORT-007` | recursion/mutual recursion/XSUB/exception fixtures; API and report comparisons |
| `calls=0` | Preserve aggregate subroutine behavior without individual call-event output | `COL-001`, `RUST-008`, `PERL-005` | feature-equivalent v5/v6 collection and report tests |
| `calls=1` | Preserve return events and all derived calls/flame behavior supported by v6.15 | `COL-001`, `COL-017`, `RUST-008`, `REPORT-007` | exact same-run event comparison plus calls/flame report parity |
| `calls=2` | Preserve entry and return events, stack depth, locations, and ordering | `COL-001`, `COL-017`, `RUST-008`, `REPORT-007` | deep recursion, exceptions, non-local exits, and boundary fixtures |
| Discount/profiler overhead | Preserve marker placement and exact count/time effects | `BASE-003`, `COMPAT-003`, `COL-002`, `COL-005` | scripted clock around flush/compression/I/O; forced capacities 1..production |
| `slowops` | Preserve operation selection, time accounting, and reports | `BASE-002`, `BASE-003`, `COL-017`, `RUST-007` | on/off matrix and representative regex/I/O/overload cases |
| `leave` correction | Preserve all legacy modes and exception/non-local-exit semantics | `BASE-003`, `COL-017`, `RUST-008` | leave-mode matrix with recursion, die/eval, `goto &sub`, XSUBs |
| `stmts`, `blocks`, and `subs` options | Preserve each legal combination, emitted records, aggregates, and warnings | `BASE-002`, `COMPAT-004`, `COL-001`, `PERL-001` | pairwise plus known interaction matrix; no benchmark may disable a feature only on one side |
| Clock selection | Preserve supported clocks, frequency/scale metadata, monotonic/anomaly behavior, and diagnostics | `BASE-003`, `FMT-004`, `COL-011`, `RUST-005` | platform clock matrix, deterministic clock, long-duration/overflow suite |
| Integer ticks in v6 | Store exact raw ticks; convert only at legacy API/report boundaries | `COMPAT-003`, `FMT-004`, `COL-011`, `RUST-007`, `RUST-008` | boundary vectors; same-run equality; strict v6-to-v5 representability |
| Compile/INIT/runtime/END start modes | Preserve when capture starts/stops and what metadata/source is emitted | `BASE-003`, `COL-002`, `COL-015` | lifecycle fixtures and exact callback order |
| Explicit start/stop/restart | Preserve legal transitions, discarded/included intervals, warnings, and finalization | `ARCH-002`, `COL-002`, `PERL-001` | scripted transition tests and partial-profile checks |
| `start` configuration | Preserve option parsing, default, environment precedence, and timing boundary | `BASE-005`, `PERL-001`, `TOOL-010` | CLI/environment black-box suite and deterministic clock |
| `optimize` configuration | Preserve documented/de facto collection optimization semantics and side effects | `BASE-002`, `BASE-005`, `COMPAT-004` | legacy option fixture and API/report comparison |
| `stash` configuration | Preserve stash filtering/selection behavior and diagnostics | `BASE-002`, `BASE-005`, `PERL-001` | included/excluded package fixtures and callback/report checks |
| `findcaller` configuration | Preserve caller-resolution behavior and costs without changing call edges | `BASE-003`, `RUST-008`, `PERL-001` | ambiguous/anonymous/caller edge fixtures |
| `use_db_sub` configuration | Preserve DB::sub integration, call semantics, and compatibility limits | `BASE-003`, `COL-017`, `PERL-001` | upstream matrix plus recursion/XSUB/exception fixtures |

## Names, source, evals, and files

| Surface or mode | Required parity | Primary implementation tasks | Required evidence |
|---|---|---|---|
| File ID definitions and metadata | Preserve byte names, flags, definition order, path identity, and references | `BASE-002`, `FMT-005`, `COL-010`, `RUST-009` | canonical callback and event comparison; dictionary collision tests |
| Source-line records | Preserve exact bytes, line mapping, flags, callback order, and selected `savesrc` behavior | `FMT-008`, `COL-013`, `RUST-009`, `REPORT-005` | Unicode, invalid/non-UTF-8, long lines, missing/changed source, partial source |
| `savesrc` modes | Embed or omit exactly as selected; never omit source to claim storage gains | `BASE-005`, `FMT-008`, `COL-013`, `PERL-001` | every supported mode across v5/v6 and report portability tests |
| Source deduplication | Share physical bytes only; retain distinct logical file/eval identities and event order | `FMT-008`, `COL-013`, `RUST-009` | hash-collision injection, identical bytes/different identity, fork/merge fixtures |
| String evals and nested evals | Preserve naming, source, parentage, collapse rules, sub definitions, and reports | `BASE-004`, `PERL-007`, `RUST-009`, `REPORT-005` | nested/repeated eval fixtures, API object identity, source-page parity |
| `nameevals` | Preserve configured eval naming behavior and collision handling | `BASE-005`, `PERL-007` | black-box option and API/report comparisons |
| Anonymous subs and closures | Preserve generated names, definition locations, identity, redefinition, and calls | `RUST-009`, `PERL-008`, `REPORT-006`, `REPORT-007` | closures, repeated definitions, recursion, calls=1/2 |
| `nameanonsubs` | Preserve configured anonymous-sub naming and all downstream identities | `BASE-005`, `PERL-008` | exact names/bytes/order in events, APIs, reports, merge |
| `.pm`/`.pmc`, AutoLoader/AutoSplit | Preserve source selection, logical files, definitions, and reports | `COL-013`, `RUST-009`, `PERL-007` | `.pmc`, AutoLoader, AutoSplit, missing source, changed source fixtures |
| Unicode and arbitrary bytes | Preserve byte values and UTF-8 semantic flags without lossy transcoding | `COMPAT-001`, `FMT-005`, `FMT-008`, `RUST-002` | cross-language vectors, malformed UTF-8, HTML escaping, round trips |
| Very long names/paths/source | Preserve values with bounded parser/model behavior | `FMT-005`, `FMT-008`, `RUST-003`, `SEC-001` | maximum/over-limit fixtures and deterministic diagnostics |

## Process, fork, lifecycle, and failure behavior

| Surface or mode | Required parity | Primary implementation tasks | Required evidence |
|---|---|---|---|
| PID/process markers | Preserve process identity, stream boundaries, and callback/report behavior | `COMPAT-001`, `COL-003`, `COL-015`, `RUST-012` | same-run process streams and mixed merge tests |
| `addpid` | Preserve filename behavior, PID transitions, parent/child output, and CLI/environment precedence | `BASE-005`, `COL-015`, `PERL-001` | fork matrix and exact filesystem effects |
| `addtimestamp` | Preserve filename timestamp behavior and normalization rules | `BASE-005`, `COMPAT-002`, `PERL-001` | controlled wall-clock and black-box filesystem tests |
| `forkdepth` | Preserve profiling-depth policy, inherited state, file ownership, and merge behavior | `COL-015`, `TEST-018`, `RUST-013` | nested-fork stress, near-full buffers, parent/child errors |
| File switching/rotation where supported | Preserve finalization, IDs, source metadata, and event boundaries | `ARCH-002`, `COL-002`, `COL-015` | scripted clock and sink-failure tests |
| Normal exit and global destruction | Preserve end records, final caller/source data, warnings, and files | `COL-002`, `COL-015`, `PERL-001` | END/destructor/global-destruction fixtures |
| `endatexit` | Preserve configured END/exit finalization behavior | `BASE-003`, `COL-002`, `PERL-001` | exit-path matrix and partial-profile validation |
| `sigexit` | Preserve supported signal finalization behavior and documented limitations | `BASE-003`, `COL-002`, `SEC-008` | signal matrix without unsafe extra work in handlers |
| `posix_exit` and `libcexit` | Preserve interception/finalization behavior and failure modes | `BASE-003`, `COL-002`, `PERL-001` | `_exit`, POSIX, libc, fork child, and partial-file fixtures |
| Die/eval/exception/non-local exit | Preserve stack, timing, source, and close behavior | `BASE-003`, `COL-017`, `RUST-008` | recursion, `goto &sub`, die/eval, destructor exceptions |
| I/O, compression, ENOSPC, short write | Never silently lose events; preserve legacy diagnostics where required | `COL-018`, `SEC-003`, `SEC-004`, `TOOL-016` | injected failures and atomic publication checks |
| Partial/truncated profiles | Match or improve readable-prefix recovery while marking incompleteness explicitly | `FMT-010`, `RUST-003`, `TOOL-007`, `SEC-003` | truncate/flip/reorder corpus; M10 |

## Format, storage, and interoperability

| Surface or mode | Required parity | Primary implementation tasks | Required evidence |
|---|---|---|---|
| v5 uncompressed output | Remain readable by unmodified v6.15 tools | `COL-006`, `COMPAT-005`, `TEST-007` | M4 across all required features |
| v5 zlib output and levels | Preserve option behavior and old-reader compatibility | `COL-006`, `BUILD-008`, `TEST-007` | compressed/uncompressed and level matrix |
| `compress` configuration | Preserve parsing, range/defaults, errors, and equivalent-feature output | `BASE-005`, `COL-006`, `COL-009`, `TOOL-010` | CLI/API matrix plus bytes/CPU measurement |
| v6 dictionaries | Losslessly intern exact bytes/flags; no identity conflation | `FMT-005`, `COL-010`, `RUST-011` | collision, reset, high-cardinality, fork, recovery tests |
| v6 deltas | Reconstruct absolute fields and every event exactly | `FMT-006`, `COL-012`, `RUST-011` | varint/delta boundaries, corruption, random round trips |
| Reversible run/pattern records | Expand to the exact original multiplicity, order, fields, and per-event ticks | `FMT-007`, `COL-012`, `TEST-015` | property tests and corpus benefit threshold; defer if not clearly beneficial |
| Source blob dedup | Reduce bytes without changing logical source identity | `FMT-008`, `COL-013`, `RUST-009` | semantic event/report equality and collision tests |
| Independent chunks | Preserve order; make state dependencies explicit; localize corruption | `FMT-002`, `FMT-009`, `FMT-010`, `COL-007`, `RUST-011` | arbitrary boundary, missing chunk, index, and salvage tests |
| Codec negotiation | Unsupported required codecs fail clearly; optional sections remain skippable | `ARCH-006`, `FMT-009`, `BUILD-008`, `SEC-004` | capability matrix, old-tier/no-codec installs, hostile compressed data |
| Optional index/summary | Never replace raw events or become authoritative without validation | `FMT-011`, `RUST-014`, `TOOL-008`, `SEC-009` | raw replay equality, stale/tampered cache tests |
| v5 to v6 conversion | Preserve canonical events and raw legacy numeric provenance | `FMT-013`, `TOOL-004`, `TEST-012` | M7 and canonical hashes |
| v6 to v5 conversion | Succeed only when exactly representable; fail before publish otherwise | `COMPAT-003`, `FMT-013`, `TOOL-005`, `TEST-012` | M8, overflow/native-NV/unknown-extension cases |
| Mixed v5/v6 merge | Preserve defined legacy merge semantics, run/process identity, source identity, and checked totals | `RUST-013`, `TOOL-009`, `COMPAT-014` | `TEST-013`, M9, deterministic input-order cases |

## Perl APIs and engine selection

| Surface or mode | Required parity | Primary implementation tasks | Required evidence |
|---|---|---|---|
| `Devel::NYTProf::ReadStream` | Preserve callback names, order, arguments, scalar flags/bytes, errors, and incomplete-file behavior | `PERL-004`, `PERL-011`, `COMPAT-007` | callback traces under legacy/native and v5/v6 |
| `Devel::NYTProf::Data` and related objects | Preserve classes, methods, deep shapes, units, identity, aliases, mutation, warnings, and errors | `PERL-005`, `PERL-006`, `PERL-009`, `PERL-010` | `TEST-010` structural and behavioral snapshots |
| Reader/FileInfo/SubInfo/line/block/call objects | Preserve all documented and de facto public behavior | `BASE-004`, `PERL-001`, `PERL-006` | downstream/bundled-tool probes and object snapshots |
| Legacy backend | Remain independently selectable and tested | `PERL-002`, `PERL-012`, `BUILD-014` | legacy-only build/install and engine-forcing matrix |
| Native backend | Never silently change semantics; expose clear capability/errors | `PERL-002`, `PERL-003`, `PERL-012` | forced native/auto/legacy, ABI mismatch, unsupported feature, corrupt input |
| `trace` and `log` diagnostics | Preserve option behavior or document/ADR any diagnostic-only evolution | `BASE-005`, `COMPAT-008`, `PERL-001` | stdout/stderr/path/exit black-box tests |

## Reports and command-line tools

| Surface or mode | Required parity | Primary implementation tasks | Required evidence |
|---|---|---|---|
| `nytprofhtml` summary/index | Preserve values, ranking, statistics, filenames, links, options, stdout/stderr, and exit code | `REPORT-001` through `REPORT-005`, `TOOL-010` | parsed value, DOM, artifact, visual, and CLI comparisons |
| Source pages | Preserve source bytes/lines, highlighting, IDs/anchors, counts/times, navigation, and escaping | `REPORT-005`, `SEC-006`, `SEC-007` | source/eval/Unicode/binary/path-injection matrix |
| Block and sub-level pages | Preserve page availability, calculations, links, and option behavior | `REPORT-006`, `REPORT-020` | normalized artifact tree and values |
| Calls/folded stacks | Preserve exact derivation, modes, ordering, and output syntax | `REPORT-007`, `TOOL-011` | calls=1/2, recursion, exceptions, CLI streaming cases |
| Flame graphs | Preserve integration, input data, options, assets/paths, and failure behavior | `REPORT-008` | folded-stack equality and rendered artifact checks |
| Graph/DOT/Graphviz | Preserve nodes/edges/weights, CLI behavior, artifacts, and external-tool errors | `REPORT-009` | parsed DOT graph comparison and missing-Graphviz paths |
| CSV | Preserve columns, values, quoting, ordering, newline/encoding behavior | `REPORT-012`, `TOOL-012` | parsed-cell and byte-policy tests |
| Callgrind | Preserve events, positions, calls, totals, ordering, and syntax | `REPORT-013`, `TOOL-012` | parser-based comparison and consumer smoke tests |
| `nytprofmerge` | Preserve accepted inputs/options, merged values/identity, diagnostics, and output compatibility | `RUST-013`, `TOOL-009`, `TOOL-010` | M9 and old-tool consumption for v5 output |
| Other bundled CLIs | Preserve names, options, aliases, environment, streams, exit codes, and file effects | `BASE-005`, `COMPAT-008`, `TEST-011` | black-box matrix under legacy/native and v5/v6 |
| Report callbacks/customization | Preserve supported Perl callbacks and fallback behavior | `REPORT-014`, `PERL-009` | custom callback fixtures, error propagation, object/API parity |
| Deterministic parallel rendering | Worker count must not change values, ordering, filenames, or published completeness | `REPORT-010`, `REPORT-011`, `RUST-017` | repeated 1..N worker hashes, failure injection, atomic publish |
| Compatibility report storage | Default compatibility mode preserves artifact names/links/offline use; compact mode is separately opt-in | `REPORT-015`, `REPORT-016`, `COMPAT-006` | artifact/DOM/browser/deep-link/accessibility tests |

## Build, platform, and release parity

| Surface or mode | Required parity | Primary implementation tasks | Required evidence |
|---|---|---|---|
| Legacy-only/no-Rust installation | All legacy v5 collection/report functionality remains installable on supported tiers | `COMPAT-011`, `BUILD-001` through `BUILD-005`, `BUILD-012` | clean CPAN/client installs with Cargo absent |
| Native-capable installation | ABI/library loading is versioned, safe, diagnosable, and reproducible | `RUST-010`, `BUILD-004`, `BUILD-005`, `BUILD-007`, `SEC-008` | clean install, upgrade/downgrade, ABI mismatch, loader-path tests |
| 32/64-bit and native `NV` layouts | V5 reader interprets supported layouts exactly or rejects/falls back deterministically | `RUST-005`, `TEST-017`, `COMPAT-003` | real/emulated cross-platform corpus |
| Windows/Unix path and filesystem behavior | Preserve supported filenames, separators, links, output publication, and errors | `BUILD-006`, `SEC-007`, `TEST-011` | platform runners and filesystem edge cases |
| Cross-release compatibility | Retain v5 read/write promises and stable v6 read compatibility window | `REL-001`, `REL-011`, `COMPAT-005`, `BUILD-013` | rolling producer/consumer matrix including pinned v6.15 |
| Default selection and rollback | Native report and v6 output defaults promote separately and remain reversible | `BUILD-014`, `REL-005` through `REL-009` | field window, certification evidence, forced legacy/v5, simulated rollback |

## Completion rule

A task or release cannot mark a row complete from a single aggregate comparison. Evidence must cover the row's producer events, canonical decode, compact model or API projection, every applicable report/tool, lifecycle/failure behavior, and supported platform tier. `BASE-006` owns the final machine-readable traceability version and must identify any rows still lacking tests or owners.

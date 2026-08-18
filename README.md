# Devel::NYTProf Modernization

Hybrid modernization of [Devel::NYTProf](https://metacpan.org/dist/Devel-NYTProf): keep C/XS on the collector hot path; use Rust for exact offline decode, compact models, reports, and tools; preserve full event fidelity and v5 interoperability.

## Start here

| Doc | Purpose |
|-----|---------|
| [`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/main/AGENTS.md) | **Agent hints** — regression tests, docs, release notes, perf/size, benchmarks vs Perl & prior versions |
| [`docs/PROGRAM_CHARTER.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md) | Mission, release levels, non-goals |
| [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md) | Ordered first-slice work board |
| [`docs/RELEASE_NOTES_R2_PREVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_PREVIEW.md) | **R2-preview** packaging notes (v6 **opt-in only**; not R3 / R4) |
| [`docs/RELEASE_NOTES_R2_STABLE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_STABLE.md) | **R2-stable** packaging notes (Phase C tools + residual honesty; not R3/R4; public perf waived) |
| [`docs/RELEASE_NOTES_v0.2.21.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_v0.2.21.md) | **v0.2.21** testdrive: statement TIME_LINE excludes hook/write cost, EL8 RPM 6.15-15 |
| [`docs/RELEASE_NOTES_v0.2.20.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_v0.2.20.md) | **v0.2.20** testdrive: default `slowops=2` is the 6.15 full table, EL8 RPM 6.15-14 |
| [`docs/RELEASE_NOTES_v0.2.19.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_v0.2.19.md) | **v0.2.19** testdrive: EL8 RPM 6.15-13 (`%check` sums `SUB_CALLERS.count`; v0.2.17 attach) |
| [`docs/RELEASE_NOTES_v0.2.18.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_v0.2.18.md) | **v0.2.18** testdrive: full `collector/xs/` in staged dist (no EL8 RPM — `%check` counted tags) |
| [`docs/RELEASE_NOTES_v0.2.17.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_v0.2.17.md) | **v0.2.17** testdrive: default C `OP_ENTERSUB` + `OP_GOTO`, `SUB_CALLERS` aggregated at finish (no EL8 RPM) |
| [`docs/RELEASE_NOTES_v0.2.16.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_v0.2.16.md) | **v0.2.16** testdrive: nested exclusive split, `stmts=0`, Profile of `$0` (not Config_heavy.pl), RPM 6.15-10 |
| [`docs/RELEASE_NOTES_v0.2.15.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_v0.2.15.md) | **v0.2.15** testdrive: logger `caller` is the app (not `NYTProfM.pm`), RPM 6.15-9 |
| [`docs/RELEASE_NOTES_v0.2.14.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_v0.2.14.md) | **v0.2.14** testdrive: Memoize `caller` under attach (no `DB::fn` croak), RPM 6.15-8 |
| [`docs/RELEASE_NOTES_v0.2.13.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_v0.2.13.md) | **v0.2.13** testdrive: fail-closed `nodebug_stash` (no GP-less GV SEGV), RPM 6.15-7 |
| [`docs/RELEASE_NOTES_v0.2.12.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_v0.2.12.md) | **v0.2.12** testdrive: DateTime/Rex attach survival, 20-app catalog, RPM 6.15-6 |
| [`docs/RELEASE_NOTES_v0.2.8.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_v0.2.8.md) | **v0.2.8** testdrive: zlib-6 default, opt-in durable seals, collection-only RPM 6.15-3 (`nytprofm-cli`) |
| [`docs/OPERATOR_PROFILE_SIZE_AND_DURABILITY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/OPERATOR_PROFILE_SIZE_AND_DURABILITY_v0.md) | Profile size + durability (zlib default, sealed publish, `aggregate=1` ADR-0013) |
| [`docs/R4_FIELD_WINDOW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_FIELD_WINDOW.md) | R4 `format=v6` field-window evidence pack (no runtime flip; PR-E01) |
| [`docs/adrs/0008-r4-v6-output-default-promotion.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) | **ADR-0008** R4 product format default promotion policy (gated; flip not executed; PR-E02) |
| [`docs/R4_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md) | R4 flip execution + rollback checklist |
| [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md) | Dual-equality E1–E5 readiness checklist |
| [`docs/PHASE0_EXIT_CRITERIA.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PHASE0_EXIT_CRITERIA.md) | Phase-0 “good enough” gates |
| [`docs/plan/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/README.md) | Full architecture + 206-task plan package |
| [`docs/governance/COMPAT-000_RATIFICATION.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/COMPAT-000_RATIFICATION.md) | Binding compatibility contract sign-off |
| [`docs/governance/ARCH-008_ADR_PROCESS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/governance/ARCH-008_ADR_PROCESS.md) | ADR process |
| [`baseline/6.15/`](https://github.com/hilather/nytprof-modernization/blob/main/baseline/6.15/) | Pinned v6.15 oracle |

## Layout

```text
docs/           charter, boards, governance, imported plan package
baseline/       immutable oracle pin, inventories, manifests
fixtures/       golden profiles and expected dumps
tools/oracle/   scripts to build oracle, dump fixtures, compare
tools/bench/    light offline timing harness (not certification)
scripts/        baseline build/test helpers + packaging smokes + `ci/offline_gate.sh` + `field/` R4 evidence pack + Rocky 8 Docker profile lab
Makefile.PL     candidate dual-path packaging entry (not full XS CPAN)
crates/         Rust workspace (v5 reader, provisional v6 preflight crate, compact model, report MVP) — not required for oracle
perl/           candidate Perl engine-dispatch facade (nytprof-engine) — not used by oracle builds
collector/      B0-A overlay COL-001 semantic sink scaffold (opt-in C build; never on oracle PERL5LIB)
AGENTS.md       binding agent quality bars (tests, docs, release notes, perf/size, benchmarks)
```

## Rust workspace (optional)

Requires a stable Rust toolchain (`cargo`). The oracle Perl path does **not**
depend on this workspace.

```sh
cargo test --workspace
# Event dump (default / dump subcommand):
cargo run -p nytprof-cli -- fixtures/v5/default-calls1/nytprof.out > /tmp/rust.jsonl
# Text summary report (REPORT-MVP):
cargo run -p nytprof-cli -- report fixtures/v5/default-calls1/nytprof.out
# Structured JSON aggregates (NATIVE-AGG-JSON; leaf/mid/edge 15/3/15):
cargo run -p nytprof-cli -- report --json fixtures/v5/default-calls1/nytprof.out
# CSV / tabular report (subs + call edges):
cargo run -p nytprof-cli -- csv fixtures/v5/default-calls1/nytprof.out
# Minimal HTML report (stdout or -o file; single document):
cargo run -p nytprof-cli -- html fixtures/v5/default-calls1/nytprof.out -o /tmp/nytprof.html
# Multi-file HTML site (index.html + file-<fid>.html + source.html alias):
cargo run -p nytprof-cli -- html fixtures/v5/default-calls1/nytprof.out --out-dir /tmp/site
# Verify / inspect (decode + model; short OK summary):
cargo run -p nytprof-cli -- verify fixtures/v5/default-calls1/nytprof.out
# Native offline capability self-test (decode/report/verify + optional golden probe):
cargo run -p nytprof-cli -- capability
# Folded stacks + Callgrind-style export (MVP contracted fields):
cargo run -p nytprof-cli -- folded fixtures/v5/default-calls1/nytprof.out
cargo run -p nytprof-cli -- callgrind fixtures/v5/default-calls1/nytprof.out
# Structural compare with oracle dump (optional):
cargo run -q -p nytprof-cli -- dump fixtures/v5/default-calls1/nytprof.out > /tmp/rust.jsonl
python3 tools/oracle/normalize_jsonl.py fixtures/v5/default-calls1/readstream.jsonl \
  > /tmp/oracle.norm.jsonl
python3 tools/oracle/normalize_jsonl.py /tmp/rust.jsonl > /tmp/rust.norm.jsonl
perl tools/oracle/compare_jsonl.pl /tmp/oracle.norm.jsonl /tmp/rust.norm.jsonl
# Native dump parity smoke (dump×2 stability + golden full match):
./tools/oracle/selftest_native_dump_parity.sh              # default-calls1
./tools/oracle/selftest_native_dump_parity_all.sh          # + calls2-default + blocks-calls1
# Light wall-time + size samples (not certification; no public claims):
bash tools/bench/light_bench.sh
# P1/P2-focused proxies only:
# STEPS=size,collector_micro bash tools/bench/light_bench.sh
```

Binary: `nytprof-dump` (package `nytprof-cli`; subcommands: `dump` / `report` / `summary` / `aggregates` / `csv` / `html` / `folded` / `callgrind` / `cg` / `verify` / `inspect` / `capability` / `selftest` / `capabilities`). Schemas:
[`docs/schemas/canonical-event-dump-v0.md`](docs/schemas/canonical-event-dump-v0.md),
[`docs/schemas/aggregate-comparison-v0.md`](docs/schemas/aggregate-comparison-v0.md),
[`docs/schemas/native-aggregates-json-mvp-v0.md`](docs/schemas/native-aggregates-json-mvp-v0.md),
[`docs/schemas/html-report-mvp-v0.md`](docs/schemas/html-report-mvp-v0.md),
[`docs/schemas/html-multifile-mvp-v0.md`](docs/schemas/html-multifile-mvp-v0.md),
[`docs/schemas/html-per-file-mvp-v0.md`](docs/schemas/html-per-file-mvp-v0.md)
(per-file `file-<fid>.html` + A4b **Block line totals**),
[`docs/schemas/export-formats-mvp-v0.md`](docs/schemas/export-formats-mvp-v0.md),
[`docs/schemas/export-semantic-parity-mvp-v0.md`](docs/schemas/export-semantic-parity-mvp-v0.md),
[`docs/schemas/verify-cli-mvp-v0.md`](docs/schemas/verify-cli-mvp-v0.md),
[`docs/schemas/capability-selftest-mvp-v0.md`](docs/schemas/capability-selftest-mvp-v0.md),
[`docs/schemas/native-dump-parity-mvp-v0.md`](docs/schemas/native-dump-parity-mvp-v0.md).
Board: [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md). Exploratory timing notes + P1/P2 methodology (not certification; no public claims until R2-stable gates green): [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md), [`tools/bench/light_bench.sh`](https://github.com/hilather/nytprof-modernization/blob/main/tools/bench/light_bench.sh).

## Oracle (BASE-001)

```sh
./scripts/baseline/run_all.sh
# equivalent: fetch → build → test → write_manifest
./scripts/baseline/fetch_oracle.sh
./scripts/baseline/build_oracle.sh
./scripts/baseline/test_oracle.sh
./scripts/baseline/write_manifest.sh
```

See `baseline/6.15/manifest.json` after a successful pin.

**Cargo is never required** for the oracle path. `PERL5LIB` for oracle tools is limited to the pin under `baseline/6.15/` (see `tools/oracle/env.sh`); it must not include `crates/` or candidate `perl/`.

## Offline R1 gate (CI-OFFLINE-GATE / CI-OFFLINE-GATE-EXPAND / CI-QUERY-JSON-GATE)

Single documented fail-fast gate for critical offline R1 checks (not multi-OS CI):

```sh
./scripts/ci/offline_gate.sh
# after perl Makefile.PL:
make offline-gate
```

| Step | Action | If cargo missing |
|------|--------|------------------|
| 1 | `cargo test -p nytprof-format-v5 -p nytprof-format-v6 -p nytprof-model -p nytprof-report -p nytprof-cli` | Honest skip |
| 2 | `./tools/oracle/selftest_harness.sh` | Still required |
| 3 | `./scripts/packaging/dual_path_smoke.sh` (primary packaging) | Still required (native half skips inside dual-path) |
| 4 | `./scripts/packaging/engine_auto_fallback_smoke.sh` (ENGINE-AUTO-FALLBACK) | Still required (needs native discoverable or cargo to build) |
| 5 | `./scripts/packaging/perl_jsonl_data_all_smoke.sh` (pure-Perl JsonlData roll-up) | Still required (golden JSONL; no cargo) |
| 6 | `./scripts/packaging/perl_query_json_smoke.sh` (QUERY-JSON-MVP / QUERY-JSON-EXPAND / **CI-QUERY-JSON-GATE**) | Still required (golden `--jsonl`; no cargo) |
| 6b | `./scripts/packaging/json_sub_entry_smoke.sh` (JSON-SUB-ENTRY-MVP) | Still required (`sub_entry_events` **0**/**27**; pure-Perl golden) |
| 6c | `./scripts/packaging/json_blocks_smoke.sh` (JSON-BLOCKS-MVP) | Still required (blocks-calls1 `line_calls_1_5` **780** / `block_line_calls_1_4` **810**) |
| 7 | `./scripts/packaging/native_agg_json_smoke.sh` (NATIVE-AGG-JSON) | Optional when native CLI available (**15/3/15**) |
| 8 | `./scripts/packaging/native_query_json_cross_smoke.sh` (NATIVE-QUERY-JSON-CROSS) | Optional when native: native `report --json` vs Perl `query --json` shared fields **15/3/15** + discount **818** |
| 9 | `./scripts/packaging/capability_selftest_smoke.sh` (CI-CAPABILITY-GATE) | Honest skip if no cargo **and** no `prefix`/`target` native CLI (same condition as `packaging_gate`; dual_path with cargo usually installs `prefix/bin` first) |

Never puts `crates/` on oracle `PERL5LIB`. Broader packaging suite remains `./scripts/packaging/packaging_gate.sh`. Policy: [`docs/BUILD_SUPPORT_POLICY.md`](docs/BUILD_SUPPORT_POLICY.md).

## Packaging smoke

Prove legacy-only isolation (no Cargo), optional native workspace tests, engine selection, the Perl engine-dispatch facade, and a stable native install under `prefix/bin`:

```sh
# Offline R1 gate (cargo? + harness + dual-path + auto-fallback + JsonlData + query-JSON + capability; recommended single entry)
./scripts/ci/offline_gate.sh

# Unified fail-fast packaging gate (broader packaging suite)
./scripts/packaging/packaging_gate.sh

# Dual-path support tiers (legacy always; native if cargo present)
./scripts/packaging/dual_path_smoke.sh

# Candidate MakeMaker packaging entry (BUILD-MAKEMAKER-OPT; not full XS CPAN)
perl Makefile.PL && make legacy-smoke          # no cargo required
perl Makefile.PL && make offline-gate          # CI-OFFLINE-GATE wrapper
./scripts/packaging/makemaker_dual_path_smoke.sh
# optional native via Make (requires cargo):
#   make native-install
#   NYTPROF_NATIVE=1 perl Makefile.PL && make
# I03 cargo-free product scripts (no cargo):
#   make install-product-scripts
#   make i03-dist-scripts-smoke

# Or run steps individually:
./scripts/packaging/legacy_only_smoke.sh
# G03a load + G04 attach-parity smokes (not wired into dual_path or offline_gate):
#   https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/product_attach_smoke.sh
#   https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/product_legacy_smoke.sh
./scripts/packaging/product_attach_smoke.sh    # OK: G03a load (no file=)
./scripts/packaging/g03b_stmt_emit_smoke.sh    # OK: G03b nytp_emit_*; NOT-YET: G06
./scripts/packaging/g03c_sub_emit_smoke.sh     # OK: G03c nytp_emit_sub_*; NOT-YET: G06
./scripts/packaging/g03d_meta_emit_smoke.sh    # OK: G03d nytp_emit_* meta; NOT-YET: G06
./scripts/packaging/g03e_compress_emit_smoke.sh # OK: G03e start-deflate; mid-deflate fork residual
./scripts/packaging/g04_v5_parity_smoke.sh     # OK: G04 live -d:NYTProf 15/3/15
./scripts/packaging/g05_options_format_smoke.sh # OK: G05 unknown/dual/D1-B v6 fail-closed; D1-A NYTPROF6
./scripts/packaging/g06_fork_addpid_smoke.sh    # OK: G06 live fork+addpid parent + <file>.<pid> NYTProf 5
./scripts/packaging/g07_getopt_compile_smoke.sh # OK: PR-7 Getopt/Exporter compile under -d:NYTProfM
./scripts/packaging/g10_datetime_hints_smoke.sh # OK: PR-10 no CORE::GLOBAL::require wrap; DateTime::Duration
./scripts/packaging/g11_nodebug_stash_nogp_smoke.sh # OK: PR-11 GP-less stash GV does not SEGV nodebug_stash
./scripts/packaging/g12_memoize_caller_smoke.sh # OK: PR-12 Memoize::memoize caller is not DB
./scripts/packaging/g13_logger_caller_smoke.sh # OK: PR-13 logger caller is the app, not NYTProfM.pm
./scripts/packaging/g14_nested_excl_smoke.sh # OK: PR-14 3-level excl = incl − child incl; stmts=0 size
./scripts/packaging/g15_dbstate_timeline_smoke.sh # OK: PR-15 C OP_DBSTATE TIME_LINE; $DB::single=0
./scripts/packaging/g16_wrap_enter_smoke.sh # OK: PR-16 wrap_push faster than WRAP_SLOW caller+fid
./scripts/packaging/g17_entersub_attach_smoke.sh # OK: DI-03 E1a entersub=1 opcode (default still wrap)
./scripts/packaging/g19_leave_discount_smoke.sh # OK: DI-03 E3 leave=1 DISCOUNT (default leave=0)
./scripts/field/complex_app_docker_profile.sh   # --app rex\|ppi\|json_xs\|… Rocky 8 attach
./scripts/field/complex_app_docker_profile_smoke.sh # --app rex --engine both (honest docker SKIP)
./scripts/packaging/product_legacy_smoke.sh    # I01: cargo-free prefix install + live attach 15/3/15
./scripts/packaging/install_product_xs.sh      # I01: install product Devel::NYTProf (no cargo)
./scripts/packaging/i02_makemaker_native_smoke.sh # I02: NYTPROF_NATIVE=1 fail-closed; auto/0 cargo-free; CLI 15/3/15
./scripts/packaging/install_product_scripts.sh # I03: cargo-free EngineDispatch + nytprofhtml/csv
./scripts/packaging/i03_dist_scripts_smoke.sh  # I03: installed query --json --jsonl 15/3/15
./scripts/packaging/g01_drop_in_docs_selftest.sh  # G01 regression: drives the real smokes + KD greps
# G02 v5-only product archive + load-only XS (not attach; not in dual_path / offline_gate):
./scripts/packaging/g02_v5_product_link_smoke.sh
./scripts/packaging/engine_select_smoke.sh     # requires cargo + crates/
./scripts/packaging/perl_engine_dispatch_smoke.sh  # Perl facade: native + invalid + legacy
./scripts/packaging/install_native.sh          # install nytprof-cli → prefix/bin
./scripts/packaging/native_install_smoke.sh    # smoke via prefix binary
./scripts/packaging/capability_selftest_smoke.sh  # CAPABILITY-SELFTEST + JSON (capability×2 + --json×2)
./scripts/packaging/native_agg_json_smoke.sh   # NATIVE-AGG-JSON: report --json ×2 → leaf/mid/edge 15/3/15
./scripts/packaging/native_optional_smoke.sh   # skips if cargo / crates/ absent
cargo run -p nytprof-cli -- --engine=native report fixtures/v5/default-calls1/nytprof.out
cargo run -p nytprof-cli -- report --json fixtures/v5/default-calls1/nytprof.out  # aggregates JSON
cargo test --workspace   # optional full native suite
```

### Native install (stable CLI for Perl dispatch)

```sh
./scripts/packaging/install_native.sh
# optional: PREFIX=... NATIVE_RELEASE=1 ./scripts/packaging/install_native.sh
./scripts/packaging/native_install_smoke.sh
```

Installs `$REPO/prefix/bin/nytprof-cli` (and `nytprof-dump` alias). Schema: [`docs/schemas/native-install-mvp-v0.md`](docs/schemas/native-install-mvp-v0.md).

### Perl engine-dispatch facade

Thin operator CLI under `perl/` (not on oracle `PERL5LIB`) that dispatches to native or legacy:

```sh
perl -Iperl/lib perl/bin/nytprof-engine --engine=native report fixtures/v5/default-calls1/nytprof.out
perl -Iperl/lib perl/bin/nytprof-engine --engine=legacy report fixtures/v5/default-calls1/nytprof.out
perl -Iperl/lib perl/bin/nytprof-engine --engine=native query fixtures/v5/default-calls1/nytprof.out
perl -Iperl/lib perl/bin/nytprof-engine query --jsonl fixtures/v5/default-calls1/readstream.jsonl
perl -Iperl/lib perl/bin/nytprof-engine query --json --jsonl fixtures/v5/default-calls1/readstream.jsonl
perl -Iperl/lib perl/bin/nytprof-engine --engine=native folded fixtures/v5/default-calls1/nytprof.out
perl -Iperl/lib perl/bin/nytprof-engine --engine=native callgrind fixtures/v5/default-calls1/nytprof.out
./scripts/packaging/perl_engine_dispatch_smoke.sh
./scripts/packaging/perl_engine_query_smoke.sh
./scripts/packaging/perl_engine_query_expand_smoke.sh
./scripts/packaging/perl_query_json_smoke.sh
./scripts/packaging/json_blocks_smoke.sh
./scripts/packaging/perl_engine_export_smoke.sh
```

- `scripts/ci/offline_gate.sh` is the **CI-OFFLINE-GATE** / **CI-OFFLINE-GATE-EXPAND** / **CI-QUERY-JSON-GATE** / **CI-CAPABILITY-GATE** / **COL-001-SINK-MVP** single entry: optional focused cargo tests → required oracle harness → primary packaging `dual_path_smoke.sh` → `engine_auto_fallback_smoke.sh` → `perl_jsonl_data_all_smoke.sh` → required `perl_query_json_smoke.sh` (QUERY-JSON-MVP / QUERY-JSON-EXPAND golden `--jsonl`) → required `json_sub_entry_smoke` + `json_blocks_smoke` (JSON-BLOCKS-MVP **780**/**810**) → optional `native_agg_json_smoke` + `native_query_json_cross_smoke` when native available → `capability_selftest_smoke.sh` when cargo or prefix/target CLI present (honest skip otherwise) → `collector_sink_smoke.sh` (COL-001 overlay; honest skip without CC); fails fast.
- `packaging_gate.sh` runs the packaging smokes in order (legacy → engine_select → perl dispatch → native install if present → optional cargo tests) and fails fast.
- Root `Makefile.PL` is a **candidate packaging facade** (`NYTPROF_NATIVE=0|1|auto`): `make legacy-smoke` / `dual-path-smoke` / `offline-gate` / `native-install` wrap existing scripts; not a full Devel::NYTProf XS CPAN dist (see [`docs/BUILD_SUPPORT_POLICY.md`](docs/BUILD_SUPPORT_POLICY.md)).
- `legacy_only_smoke.sh` sources oracle env isolation, refuses `/crates/` on `PERL5LIB`, and loads `Devel::NYTProf` from `baseline/6.15/install` only.
- `engine_select_smoke.sh` exercises Rust CLI `--engine=native` report/verify, rejects bogus engines, and fails closed on `--engine=legacy` (oracle path message; not a fake Rust legacy backend).
- `perl_engine_dispatch_smoke.sh` exercises the Perl facade (`perl/bin/nytprof-engine`): native report (`main::leaf` / `returns=15`), invalid engine non-zero, legacy stream-dump when oracle is present; also runs `legacy_only_smoke.sh` and `engine_select_smoke.sh` when present.
- `perl_engine_query_smoke.sh` exercises `query` / `data-query` via pure-Perl `JsonlData` (golden `--jsonl` + optional native dump): default-calls1 **leaf=15** / **mid=3** / **mid→leaf=15**.
- `perl_engine_query_expand_smoke.sh` expands default `query` output: **sub_def** leaf/mid ranges, **source_line 1:5** hot-loop, blocks-calls1 **line_calls 1:5=780** + **block_line_calls 1:4=810** (JsonlData APIs only).
- `perl_query_json_smoke.sh` (QUERY-JSON-MVP / QUERY-JSON-EXPAND): `query --json --jsonl` ×2 → parse JSON; **leaf_returns=15** / **mid_returns=3** / **mid_leaf_edge=15**; **discount_events=818** / **is_stream_complete=true**; human default unchanged.
- `json_blocks_smoke.sh` (JSON-BLOCKS-MVP): blocks-calls1 `query --json --jsonl` → **line_calls_1_5=780** / **block_line_calls_1_4=810**; default-calls1 block **0**; optional native `report --json`.
- `native_query_json_cross_smoke.sh` (NATIVE-QUERY-JSON-CROSS): native `report --json` vs Perl `query --json --jsonl` pair ×2; equal shared fields **15/3/15** + **discount_events=818**; optional `query --json <profile>` dump path.
- `perl_engine_export_smoke.sh` exercises `folded` / `callgrind` / `cg` via native CLI subprocess (not reimplemented in Perl): default-calls1 `main::mid;main::leaf 15`, `main::RUNTIME;main::mid 3`, callgrind `fn=main::leaf` + `calls`.
- `install_native.sh` / `native_install_smoke.sh` build and exercise a stable `prefix/bin` CLI used by Perl `find_native_cli`.
- Optional helper: `./scripts/packaging/native_optional_smoke.sh` (skips if `cargo` / `crates/` absent).
- Engine selection for the Rust CLI (`--engine` / `NYTPROF_ENGINE`): [`docs/schemas/engine-selection-mvp-v0.md`](docs/schemas/engine-selection-mvp-v0.md).
- Perl dispatch contract: [`docs/schemas/perl-engine-dispatch-mvp-v0.md`](docs/schemas/perl-engine-dispatch-mvp-v0.md).

## Rust tools (optional)

The Cargo workspace under `crates/` is **optional acceleration** for offline native tools (readers, models, reports, CLIs). It is not part of the BASE-001 oracle and must not break legacy-only installs.

| State | What to do |
|-------|------------|
| `crates/` not present yet | Normal during Phase-0; workspace is **landing** with BUILD-002 / RUST-001. Oracle rebuild still works. |
| `crates/` present | Build/test with a local toolchain: `cargo build --workspace` and `cargo test --workspace` |

MSRV is **not frozen** (open ADR-Q017). Use the installed compiler (`rustc --version`) until policy lands.

Packaging tiers, isolation rules, dual-path policy, and candidate MakeMaker entry: [`docs/BUILD_SUPPORT_POLICY.md`](docs/BUILD_SUPPORT_POLICY.md), [`docs/PACKAGING_SPIKE.md`](docs/PACKAGING_SPIKE.md).

## Integrity of the plan package

```sh
cd docs/plan && sha256sum -c SHA256SUMS
```

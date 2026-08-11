# Packaging Spike Priority (BUILD-001 draft)

**Status:** early Phase-0 priority (do not wait for compact model)  
**Related tasks:** BUILD-001, BUILD-002, BUILD-003, COMPAT-009, COMPAT-011, ADR-Q016, ADR-Q017

## Why early

CPAN installability without Rust (RSK-009) can kill the program even if the Rust report engine is excellent. A thin spike lands in the same window as BASE-001 so packaging constraints shape crate/FFI choices.

## Spike goals (this charter window)

1. Document **support tiers** draft: legacy-only vs native-accelerated.
2. Prove a **layout** where:
   - oracle rebuild and legacy v5 collection/report never require Cargo;
   - optional native components are gated (`NYTPROF_NATIVE=0` / feature flags / missing shared lib → clear fallback);
   - `perl Makefile.PL && make && make test` (when candidate packaging lands) works without Cargo for legacy-only.
3. Note MSRV / dependency policy as **open ADR-Q017** (no freeze yet).
4. Prefer **no Rust in the collector** (already architecture baseline).

## Non-goals for the spike

- Full MakeMaker ↔ Cargo integration (BUILD-003) — **not** implemented in this spike.
- Full CI matrix (BUILD-006 full). **MVP:** GHA Linux+macOS offline_gate rows (**BUILD-006-MVP**).
- Prebuilt binary distribution policy finalization.
- Shipping native reports as default.

---

## Layout: `crates/` is optional acceleration

The modernization repo is **oracle-first**. Rust is an **optional acceleration path** for offline tools (v5/v6 readers, compact model, reports, converters), not a dependency of the pinned Devel::NYTProf 6.15 oracle.

Intended tree (whether or not directories exist yet):

```text
baseline/6.15/     pinned oracle pin, install tree, inventories
scripts/baseline/  fetch → build → test → manifest (Perl/C only)
tools/oracle/      fixture dump/compare using oracle PERL5LIB only
fixtures/          golden profiles and expected dumps
docs/              charter, boards, packaging notes, plan package

crates/            OPTIONAL Cargo workspace (native tools / engines)
  Cargo.toml       workspace root (when landed)
  …                individual crates (reader, model, report, CLI, …)

perl/              OPTIONAL candidate Perl facade (not on oracle PERL5LIB)
```

| Path | Required for oracle rebuild? | Role |
|------|------------------------------|------|
| `baseline/`, `scripts/baseline/`, `tools/oracle/` | Yes | BASE-001 pin and isolation |
| `crates/` | **No** | Optional Rust workspace for native acceleration |
| `perl/` | **No** | Future candidate facade; never required by oracle |

If `crates/` is missing, that is normal until BUILD-002 / RUST-001 land. Documentation and scripts must assume absence is fine.

**Do not** implement full MakeMaker integration in this spike. When BUILD-003 is scheduled, MakeMaker remains the CPAN entry point and Cargo is invoked only when native build is enabled and a toolchain is available.

---

## Oracle rebuild: never require Cargo

The full BASE-001 pipeline is:

```sh
./scripts/baseline/run_all.sh
```

which runs, in order:

1. `fetch_oracle.sh`
2. `build_oracle.sh`
3. `test_oracle.sh`
4. `write_manifest.sh`

**Hard requirement:** this pipeline (and each step) must succeed on a machine **without** `cargo`, `rustc`, or a `crates/` tree.

- No script under `scripts/baseline/` may call `cargo` or assume `target/`.
- No script under `tools/oracle/` may require Rust binaries for oracle dumps/compares.
- Presence or absence of `crates/` must not change oracle success or failure.

Verification sketch (operator):

```sh
# Optional: confirm cargo is unused by the pipeline
command -v cargo >/dev/null && echo "cargo present (ok; must still not be required)" || echo "cargo absent (ok)"
./scripts/baseline/run_all.sh
```

### Packaging smoke scripts

| Script | Requires Cargo? | Purpose |
|--------|-----------------|---------|
| [`scripts/packaging/legacy_only_smoke.sh`](../scripts/packaging/legacy_only_smoke.sh) | **No** (never invokes `cargo` / `rustc`) | Proves oracle pin + `PERL5LIB` isolation + load from `baseline/6.15/install`; optional ReadStream dump on a golden fixture |
| [`scripts/packaging/native_optional_smoke.sh`](../scripts/packaging/native_optional_smoke.sh) | Yes (skips cleanly if absent) | Optional `cargo test` of native offline packages |
| [`scripts/packaging/install_native.sh`](../scripts/packaging/install_native.sh) | Yes | Stable install of `nytprof-cli` / `nytprof-dump` into `$REPO/prefix/bin` (see [`docs/schemas/native-install-mvp-v0.md`](schemas/native-install-mvp-v0.md)) |
| [`scripts/packaging/native_install_smoke.sh`](../scripts/packaging/native_install_smoke.sh) | Yes | Install + report via prefix path (`main::leaf` / `returns=15`) |
| [`scripts/packaging/capability_selftest_smoke.sh`](../scripts/packaging/capability_selftest_smoke.sh) | Yes (packaging-native; fails closed without CLI/cargo) | CAPABILITY-SELFTEST + CAPABILITY-JSON-MVP: `capability` ×2 + `capability --json` ×2 + markers/JSON fields |
| [`scripts/packaging/dual_path_smoke.sh`](../scripts/packaging/dual_path_smoke.sh) | Mixed (policy entry) | Dual-path: legacy always; install_native + native_install_smoke (or native_optional) when cargo present; honest skip if cargo absent |
| [`scripts/packaging/makemaker_dual_path_smoke.sh`](../scripts/packaging/makemaker_dual_path_smoke.sh) | Mixed (MakeMaker entry) | Candidate root `Makefile.PL` + `make legacy-smoke`; native via make when cargo present; not full XS CPAN |
| [`scripts/packaging/packaging_gate.sh`](../scripts/packaging/packaging_gate.sh) | Mixed (fail-fast gate) | Runs legacy → engine_select → perl dispatch → native install (if present) → optional cargo tests → capability_selftest when native CLI available |
| [`scripts/ci/offline_gate.sh`](../scripts/ci/offline_gate.sh) | Mixed (CI-OFFLINE-GATE-EXPAND + CI-QUERY-JSON-GATE + CI-CAPABILITY-GATE) | Fail-fast offline R1: optional focused cargo tests → required `selftest_harness` → primary packaging `dual_path_smoke` → `engine_auto_fallback_smoke` → `perl_jsonl_data_all_smoke` → required `perl_query_json_smoke` (QUERY-JSON-MVP) → `capability_selftest_smoke` when cargo/prefix/target present |
| [`scripts/packaging/engine_auto_fallback_smoke.sh`](../scripts/packaging/engine_auto_fallback_smoke.sh) | Mixed (offline gate step 4) | ENGINE-AUTO-FALLBACK: auto prefer-native / fall-back-legacy |
| [`scripts/packaging/perl_jsonl_data_all_smoke.sh`](../scripts/packaging/perl_jsonl_data_all_smoke.sh) | No (pure-Perl) | Thin fail-fast roll-up of JsonlData pure-Perl smokes (offline gate step 5) |
| [`scripts/packaging/perl_query_json_smoke.sh`](../scripts/packaging/perl_query_json_smoke.sh) | No (pure-Perl) | QUERY-JSON-MVP / QUERY-JSON-EXPAND: `query --json --jsonl` ×2 + parse; leaf/mid/edge **15/3/15**; `discount_events` **818**; `is_stream_complete` true; human default unchanged |

```sh
# Offline R1 gate (single operator entry; multi-OS MVP = matrix_gate / GHA)
./scripts/ci/offline_gate.sh
# make offline-gate   # after perl Makefile.PL

# Dual-path support-tier check (BUILD policy entry; packaging half of offline gate)
./scripts/packaging/dual_path_smoke.sh

# Candidate MakeMaker dual-path entry (BUILD-MAKEMAKER-OPT)
perl Makefile.PL && make legacy-smoke          # no cargo required
./scripts/packaging/makemaker_dual_path_smoke.sh
# optional native via Make:
#   make native-install    # or: NYTPROF_NATIVE=1 perl Makefile.PL && make

# Unified packaging gate (broader operator entry)
./scripts/packaging/packaging_gate.sh

# Legacy-only / oracle isolation (AC1, AC3) — must pass without Cargo
./scripts/packaging/legacy_only_smoke.sh

# Optional native acceleration — only when crates/ + cargo are present
./scripts/packaging/native_optional_smoke.sh
# equivalent focused command:
# cargo test -p nytprof-format-v5 -p nytprof-model -p nytprof-report -p nytprof-cli
# or full workspace:
# cargo test --workspace

# Stable on-disk native CLI for Perl dispatch (prefix/bin)
./scripts/packaging/install_native.sh
./scripts/packaging/native_install_smoke.sh
```

`legacy_only_smoke.sh`:

- Sources `tools/oracle/env.sh` for the same isolation as fixture tools.
- If `baseline/6.15/oracle-perl5lib.txt` or the install tree is missing, attempts `./scripts/baseline/build_oracle.sh` **only when** `baseline/6.15/src` is present; otherwise fails with a clear fetch/build message (still no Cargo).
- Asserts `perl -MDevel::NYTProf` resolves under `baseline/6.15/install` and that `PERL5LIB` contains no `/crates/` entries.
- Does **not** break or extend `scripts/baseline/*`; does not put `crates/` on oracle `PERL5LIB`.

**Oracle never needs `crates/` or Cargo.** Presence of the optional workspace must not change legacy-only smoke success.

Engine / backend selection for the pure-Rust CLI (`--engine` / `NYTPROF_ENGINE`: `native`, `auto`, `legacy`) is documented separately — see [`docs/schemas/engine-selection-mvp-v0.md`](schemas/engine-selection-mvp-v0.md). That contract is for optional native tools; the legacy smoke above is the Cargo-free packaging gate.

---

## Env isolation: `PERL5LIB` must not need `crates/`

Oracle and fixture tools isolate the module path to the **pinned install tree** under `baseline/6.15/`, optionally plus `baseline/6.15/test-deps/` for prove deps.

| Mechanism | Behavior |
|-----------|----------|
| `scripts/baseline/build_oracle.sh` | Clears/rebuilds `PERL5LIB` from install tree only; refuses load paths under `/crates/` or candidate `perl/` |
| `scripts/baseline/test_oracle.sh` | Sets `PERL5LIB` from `oracle-perl5lib.txt` (+ test-deps) |
| `tools/oracle/env.sh` | Loads the same oracle `PERL5LIB`; used by dump/compare helpers |
| `scripts/baseline/common.sh` | Documents that candidate `crates/` / `perl/` must never sit on oracle `PERL5LIB` |
| `scripts/packaging/legacy_only_smoke.sh` | Operator smoke: sources `env.sh`, refuses `/crates/` on `PERL5LIB`, proves install-tree load (no Cargo) |

Rules:

1. **Oracle `PERL5LIB` never includes `crates/`** (Rust produces native libs/binaries, not the oracle `.pm` path).
2. **Oracle `PERL5LIB` never includes candidate `perl/`** until that facade is explicitly under test outside BASE-001.
3. Fixture capture (`tools/oracle/capture_fixture.sh` and related) must source oracle env isolation, not a developer `PERL5LIB` that mixes candidates.

If a smoke load of `Devel::NYTProf` resolves from a path containing `/crates/` or a non-baseline candidate tree, the oracle build is considered contaminated and must fail.

---

## Draft BUILD-001 support-tier table

Draft only — final freeze is COMPAT-009 / BUILD-001 ADR. Tiers describe **what a user or CI job is expected to get**, not a claim that all modes are implemented yet.

| Tier | Collector | Report / tools | Requires Rust toolchain? | Requires `crates/` build? | Notes |
|------|-----------|----------------|--------------------------|---------------------------|--------|
| **Legacy-only** | v5 C/XS (oracle / MakeMaker) | legacy Perl/C (`nytprofhtml`, etc.) | **No** | **No** | Default CPAN path; RSK-009 / COMPAT-011. Oracle rebuild lives here. |
| **Hybrid-v5 (opt-in)** | v5 C/XS | native reader/report/CLI **opt-in** when built or prebuilt | Build-time **or** prebuilt optional | Optional | Missing native lib → clear fallback to legacy. Suggested control: `NYTPROF_NATIVE=auto\|0\|1` (ADR later). |
| **Hybrid-v6 (later)** | v6 C writer (+ dual/v5 as designed) | native tools for v6 + converters | Same as Hybrid-v5 for tools | Optional | Collector remains C; Rust stays offline/tools unless a separate ADR reopens that. |
| **Standalone tooling** | N/A (reads profiles only) | Rust CLI without Perl embedding | Yes for source build of tools | Yes (or prebuilt) | Not a substitute for CPAN legacy install. |

### Packaging modes (map to tiers)

| Mode | Maps to | Cargo required at install? |
|------|---------|----------------------------|
| Legacy source build | Legacy-only | No |
| Native source build | Hybrid-* | Yes (optional feature) |
| Optional native binary package | Hybrid-* without local rustc | No (prebuilt; policy open ADR-Q016) |
| Standalone tooling | Standalone | Yes for from-source tools |

### Controls (suggested; not implemented in spike)

```text
NYTPROF_NATIVE=auto|0|1
```

- `auto` — attempt native only when supported; fall back cleanly  
- `0` — legacy-only  
- `1` — require native; fail configure/build if unavailable  

Build logs must state what was built and why a feature was skipped. Full MakeMaker wiring is **BUILD-003**, out of scope here.

---

## How to build native tools when desired

When the Cargo workspace exists under `crates/` (BUILD-002 / RUST-001), developers with a Rust toolchain can build and test native acceleration **without** affecting the oracle path:

```sh
# From repo root, after crates/ Cargo workspace lands:
cargo build --workspace
cargo test --workspace

# Or the packaging helper (skips cleanly if cargo/crates absent):
./scripts/packaging/native_optional_smoke.sh
```

Until `crates/` lands, these commands are simply unavailable; that does not block:

```sh
./scripts/baseline/run_all.sh
./scripts/packaging/legacy_only_smoke.sh
```

Guidance:

- Prefer workspace-level build/test so lockfile and feature flags stay consistent.
- Do not add steps to `run_all.sh` that invoke Cargo.
- Do not put `target/` or crate-local paths on `PERL5LIB`.
- Release/CI jobs that exercise Hybrid tiers should install a toolchain or use prebuilts explicitly; Legacy-only / oracle jobs must not.
- Engine selection for the Rust CLI (`NYTPROF_ENGINE` / `--engine`): see [`docs/schemas/engine-selection-mvp-v0.md`](schemas/engine-selection-mvp-v0.md).

Example separation:

| Job / workflow | Command surface | Cargo? |
|----------------|-----------------|--------|
| Oracle pin / BASE-001 | `./scripts/baseline/run_all.sh` | Forbidden dependency |
| Legacy-only packaging smoke | `./scripts/packaging/legacy_only_smoke.sh` | Forbidden dependency |
| Native tool smoke | `./scripts/packaging/native_optional_smoke.sh` or `cargo test --workspace` | Required for that job only |
| Candidate MakeMaker entry | `perl Makefile.PL && make legacy-smoke` / `makemaker_dual_path_smoke.sh` | Default without Cargo; native optional |
| Future full XS CPAN dual-build | BUILD-003 (not this spike) | Must still allow Cargo-absent legacy |

### Candidate MakeMaker packaging entry (BUILD-MAKEMAKER-OPT)

A thin root [`Makefile.PL`](../Makefile.PL) now exists as a **candidate packaging facade** (not a complete Devel::NYTProf XS CPAN tarball):

| Control / target | Behavior |
|------------------|----------|
| `NYTPROF_NATIVE=0` (default) | Legacy-only configure; no Cargo required |
| `NYTPROF_NATIVE=1` | Configure requires cargo; `make all` → `native-install` |
| `NYTPROF_NATIVE=auto` | Enable native targets when cargo present |
| `make legacy-smoke` | `scripts/packaging/legacy_only_smoke.sh` |
| `make dual-path-smoke` | `scripts/packaging/dual_path_smoke.sh` |
| `make native-install` / `make native` | `scripts/packaging/install_native.sh` (needs cargo) |
| `make test` | `legacy-smoke` only (honest: not full XS suite) |

Policy detail: [`docs/BUILD_SUPPORT_POLICY.md`](BUILD_SUPPORT_POLICY.md) (MakeMaker dual-path section). Full MakeMaker↔Cargo XS dual-build remains **BUILD-003**.

---

## MSRV note (open ADR-Q017)

**Minimum Supported Rust Version (MSRV) and dependency pinning are not frozen.**

- Track as **open ADR-Q017** (see `docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md`).
- Until an ADR lands, developers use **whatever `rustc` is installed** on the machine for optional native work.
- Check the active compiler before reporting toolchain issues:

```sh
rustc --version
cargo --version   # if cargo is installed
```

- Do not hard-code an MSRV in packaging docs or CI gates until ADR-Q017 decides one (recommended direction in the queue: conservative explicitly tested MSRV + pinned lockfile + small audited dependency set).
- Oracle and Legacy-only paths ignore MSRV entirely: they never invoke Rust.

---

## Landed dual-path policy

Support tiers and dual-path verification are landed beyond this spike’s prose:

| Artifact | Role |
|----------|------|
| [`docs/BUILD_SUPPORT_POLICY.md`](BUILD_SUPPORT_POLICY.md) | BUILD-001-style dual-path policy: **legacy-only** (no Cargo) vs **optional-native**; isolation rules; non-goals |
| [`scripts/packaging/dual_path_smoke.sh`](../scripts/packaging/dual_path_smoke.sh) | Runnable dual-path entry: always legacy; native when `cargo` present; honest skip otherwise |
| Board **BUILD-DUAL-PATH** | [`docs/FIRST_SLICE_BOARD.md`](FIRST_SLICE_BOARD.md) |

```sh
# Support-tier dual-path check (policy entry)
./scripts/packaging/dual_path_smoke.sh

# Broader packaging fail-fast (engine select + Perl facade + native)
./scripts/packaging/packaging_gate.sh
```

Full MakeMaker↔Cargo XS CPAN dual-build (**BUILD-003** full), full multi-OS CI matrix (**BUILD-006** full: multi-Perl/rustc/Windows/dashboard), and prebuilt policy remain open. Multi-OS **MVP** is **BUILD-006-MVP** (GHA ubuntu+macos + `matrix_gate.sh`). The candidate MakeMaker facade (**BUILD-MAKEMAKER-OPT**) is intentionally thinner than BUILD-003.

---

## Immediate repo artifacts

- Oracle pin and isolation under `baseline/6.15/` (no candidate on `PERL5LIB` during oracle tests).
- `./scripts/baseline/run_all.sh` must remain Cargo-free forever for BASE-001.
- `./scripts/packaging/legacy_only_smoke.sh` proves that isolation without Cargo (operator packaging gate).
- `./scripts/packaging/dual_path_smoke.sh` is the dual-path policy entry (legacy + optional native).
- Root `Makefile.PL` + `./scripts/packaging/makemaker_dual_path_smoke.sh` — candidate dual-path packaging entry (BUILD-MAKEMAKER-OPT).
- `./scripts/packaging/native_optional_smoke.sh` exercises optional crates when a toolchain is present.
- Future candidate code under `crates/` / `perl/` must not be required for `scripts/baseline/build_oracle.sh` or any oracle tool.
- Engine env / CLI names for optional native tools: [`docs/schemas/engine-selection-mvp-v0.md`](schemas/engine-selection-mvp-v0.md).
- See `docs/FIRST_SLICE_BOARD.md` for ordering relative to inventories.

## Open decisions (do not decide in code)

- ADR-Q016 native distribution model (source-only vs prebuilt vs both)
- ADR-Q017 MSRV and dependency pinning
- COMPAT-009 support-tier freeze
- BUILD-003 full MakeMaker optional Cargo + XS dual-build (beyond the BUILD-MAKEMAKER-OPT candidate facade)

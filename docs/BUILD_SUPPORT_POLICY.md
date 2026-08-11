# Build Support Policy (BUILD-001 draft / dual-path)

**Board ID:** BUILD-DUAL-PATH  
**Status:** landed as dual-path policy + runnable checks (not a full BUILD-001 ADR freeze)  
**Related:** BUILD-001, BUILD-002, BUILD-003, BUILD-006, COMPAT-009, COMPAT-011, RSK-009  
**Spike background:** [`docs/PACKAGING_SPIKE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PACKAGING_SPIKE.md)  
**Collector source-tree (accepted):** [`docs/adrs/0003-collector-packaging-source-tree.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-collector-packaging-source-tree.md) — **B0-A overlay** (`collector/`); blocks COL-001 until observed; not COL-007 product

This document freezes the **support-tier dual-path contract** for the modernization repo: what must work without Cargo, what is optional when Cargo is present, and how operators verify each path.

---

## Support tiers

| Tier | Cargo required? | What works |
|------|-----------------|------------|
| **legacy-only** | **No** | Oracle rebuild (`scripts/baseline/*`), v5 collection via pinned Devel::NYTProf, oracle tools (`tools/oracle/*`) via `PERL5LIB` isolation under `baseline/6.15/` |
| **optional-native** | **Yes** (when building native) | Cargo workspace under `crates/`, prefix/bin install (`nytprof-cli` / `nytprof-dump`), pure-Rust CLI (`--engine=native`), Perl facade `nytprof-engine --engine=native` when facade is present |

### Mapping to packaging modes

| Mode | Tier | Cargo at install/build? |
|------|------|-------------------------|
| Oracle pin / BASE-001 | legacy-only | Never required |
| Legacy packaging smoke | legacy-only | Never required |
| Native source build + prefix install | optional-native | Yes |
| Optional workspace `cargo test` | optional-native | Yes (or honest skip if absent) |
| Full CPAN MakeMaker dual-build | **BUILD-003 full** — open | Must still allow Cargo-absent legacy |
| Candidate MakeMaker packaging entry | dual-path facade | Default legacy; native only when `NYTPROF_NATIVE=1` / `make native-install` |
| BUILD-003 depth (partial) | dual-path facade + prefix installs | `install-facade` / `dual-install` / depth smoke; **not** full XS CPAN |

---

## MakeMaker dual-path packaging entry (BUILD-MAKEMAKER-OPT)

**Status:** candidate facade landed — **not** a full CPAN tarball of Devel::NYTProf XS (that remains **BUILD-003 full**).

Root [`Makefile.PL`](https://github.com/hilather/nytprof-modernization/blob/main/Makefile.PL) is a thin packaging entry that generates Makefile targets wrapping the existing `scripts/packaging/*` smokes. It does **not** build oracle XS under `baseline/6.15/src`, and it never puts `crates/` on oracle `PERL5LIB`.

### Control: `NYTPROF_NATIVE`

| Value | Configure behavior | Default `make all` |
|-------|--------------------|--------------------|
| `0` (default) | Legacy-only; **Cargo not required** | Message + no cargo |
| `1` | **Requires** `cargo` on `PATH` or configure dies | Runs `dual-install` (native CLI + pure-Perl facade) |
| `auto` | Native targets enabled if cargo present; else legacy | Legacy message; `make dual-install` / `native-install` available when cargo present |

### Reserved codec envs (not wired)

| Env | Values | Behavior in this facade |
|-----|--------|-------------------------|
| `NYTPROF_CODEC_ZSTD` | `auto` / `0` / `1` | Recorded in stamp; `=1` prints configure NOTE only — **not** MakeMaker feature wiring (**BUILD-008** / BUILD-003 full) |
| `NYTPROF_CODEC_LZ4` | `auto` / `0` / `1` | Same |

### Operator recipes

```sh
# Legacy-only (no Cargo on critical path)
perl Makefile.PL                 # or: NYTPROF_NATIVE=0 perl Makefile.PL
make legacy-smoke                # → scripts/packaging/legacy_only_smoke.sh
make install-facade              # pure-Perl nytprof-engine → $NYTPROF_PREFIX (default $REPO/prefix)
make test                        # same path as legacy-smoke (candidate entry; not full XS suite)

# Dual-path policy via Make targets
make dual-path-smoke             # → scripts/packaging/dual_path_smoke.sh

# Offline R1 gate (cargo? + harness + dual-path + auto-fallback + JsonlData + query-JSON + capability)
make offline-gate                # → scripts/ci/offline_gate.sh

# Optional native (requires cargo)
make cargo-build                 # cargo build -p nytprof-cli
make native-install              # → scripts/packaging/install_native.sh
make dual-install                # native-install + install-facade
# or reconfigure require-native:
NYTPROF_NATIVE=1 perl Makefile.PL && make dual-install
# alias:
make native                      # → native-install only
```

### Verification smokes

```sh
./scripts/packaging/makemaker_dual_path_smoke.sh
./scripts/packaging/makemaker_build003_depth_smoke.sh
# or: make build003-depth-smoke
```

`makemaker_dual_path_smoke.sh` behavior:

1. `NYTPROF_NATIVE=0 perl Makefile.PL` (must succeed without cargo).
2. `make legacy-smoke` (required).
3. `make dual-path-smoke` (legacy + optional native per dual-path policy).
4. If cargo present: `make native-install` (+ `make native-smoke` when available); also checks `NYTPROF_NATIVE=1` configure.
5. If cargo absent: honest skip of native-install; asserts `NYTPROF_NATIVE=1` configure **fails**.
6. Exit non-zero on any failure. Cleans generated Makefile products after the run.

### Honesty boundary

| This entry **is** | This entry **is not** |
|-------------------|----------------------|
| Candidate dual-path packaging facade | Complete CPAN dist of Devel::NYTProf |
| Make targets → existing packaging scripts | Full MakeMaker ↔ Cargo XS dual-build (**BUILD-003 full**) |
| Default legacy without Cargo | Multi-OS CI matrix (**BUILD-006**) / CPAN upload |

---

## BUILD-003 depth (partial — toward full dual-build)

**Board ID:** BUILD-003-DEPTH  
**Status:** **partial depth landed** — closer dual-build for installable native CLI + pure-Perl facade under a shared `prefix/`; **does not** complete **BUILD-003 full** (no oracle XS in this Makefile, no CPAN tarball, no codec feature wiring).

Absolute policy parent: [BUILD-003 task](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/12_BUILD_PACKAGING_CI_AND_RELEASE_TASKS.md) (Integrate optional Cargo build with MakeMaker).

### What depth adds beyond BUILD-MAKEMAKER-OPT

| Artifact | Role | Cargo? |
|----------|------|--------|
| `make cargo-build` | Direct `cargo build -p nytprof-cli` from Make | Required |
| `make install-facade` / [`scripts/packaging/install_facade.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/install_facade.sh) | Install `$NYTPROF_PREFIX/bin/nytprof-engine` + `lib/Devel/NYTProf/*.pm` (default `$REPO/prefix`) | **Never** |
| Shared prefix resolve | [`scripts/packaging/resolve_packaging_prefix.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/resolve_packaging_prefix.sh) | Used by both install scripts — no dual-install root split |
| `make dual-install` | `native-install` + `install-facade` | Required |
| `make packaging-status` | Print `nytprof-packaging.mode` honesty stamps | No |
| `make build003-depth-smoke` / [`scripts/packaging/makemaker_build003_depth_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/makemaker_build003_depth_smoke.sh) | Regression: facade without cargo + dual-install when cargo present | Mixed |
| Stamp fields | `packaging_depth=BUILD-003-depth-v0`, **`full_build003=0`**, `not_full_xs_cpan=1` | — |
| Configure diagnostics | cargo/rustc versions when present; explicit legacy when absent | — |

### Legacy-only unbroken (hard)

| Must | Evidence |
|------|----------|
| `NYTPROF_NATIVE=0 perl Makefile.PL` without cargo | depth smoke step 1 |
| `make install-facade` without cargo | depth smoke step 2 |
| Pure-Perl `query --json --jsonl` via installed facade | depth smoke (leaf **15** / mid **3**) |
| Never `crates/` on oracle `PERL5LIB` | child smokes; depth smoke uses private `PREFIX` |

### Residual (still open for BUILD-003 full)

- No `Devel::NYTProf` XS / collector sources in this Makefile
- No CPAN-installable dual dist with optional Cargo subdir
- No MakeMaker feature flags that enable/disable ZSTD/LZ4 at build time
- No multi-OS CI (**BUILD-006**) or prebuilt native package policy (ADR-Q016)

### Definition of done for BUILD-003-DEPTH

- [x] `install-facade` + pure-Perl prefix layout (`bin` + `lib`)
- [x] `dual-install` / `cargo-build` / `packaging-status` Make targets
- [x] Configure stamps: `packaging_depth=BUILD-003-depth-v0`, **`full_build003=0`**
- [x] Depth smoke: legacy facade without cargo; dual path when cargo present
- [x] Docs: this section + residual matrix honesty + board row
- [x] **Does not** claim BUILD-003 full complete

---

## Critical path: legacy must never require Cargo

**Hard requirement:** the legacy-only tier (oracle rebuild, fixture tools, legacy packaging smoke) must succeed on a machine **without** `cargo`, `rustc`, or a built `target/` tree.

| Must never | Why |
|------------|-----|
| `scripts/baseline/*` calling `cargo` / `rustc` | Oracle pin is Perl/C only |
| `tools/oracle/*` requiring Rust binaries | Fixture dump/compare is oracle-side |
| Oracle / legacy smoke putting `crates/` on `PERL5LIB` | Contamination of the pin |
| Failing legacy smoke because Cargo is missing | RSK-009 / COMPAT-011 |

Presence of `crates/` or a Rust toolchain is **allowed** on developer machines; it must not become a **dependency** of the legacy path.

---

## Isolation: `crates/` never on oracle `PERL5LIB`

| Rule | Detail |
|------|--------|
| Oracle `PERL5LIB` | Built from `baseline/6.15/install` (+ optional `test-deps/`) only |
| Forbidden | Any `PERL5LIB` entry under `crates/` |
| Forbidden (oracle context) | Candidate `perl/` facade until explicitly under test outside BASE-001 |
| Native tools | Use binaries (`prefix/bin`, `target/…`) or `cargo run`; not module path |

Enforced by `tools/oracle/env.sh`, `scripts/baseline/build_oracle.sh`, and `scripts/packaging/legacy_only_smoke.sh`.

---

## Collector overlay source tree (ADR-0003 / B0-A)

**Accepted layout:** modernization collector C/XS sources live under repository-root **`collector/`** (B0-A overlay). The 6.15 oracle pin under `baseline/6.15/` remains archives + isolated install for differential tests. **Do not** implement COL-001..007 by patching `baseline/6.15/src` (B0-B rejected). Full decision: [`docs/adrs/0003-collector-packaging-source-tree.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-collector-packaging-source-tree.md).

| Path | Role | On oracle `PERL5LIB`? |
|------|------|------------------------|
| `baseline/6.15/archives/` | Immutable pin | N/A |
| `baseline/6.15/install/` | Oracle install only | **Yes** (only this + optional test-deps) |
| `collector/` | Overlay sink / writers (PR-B02+) | **Never** |
| `collector/install/` or `prefix/collector/` | Candidate collector install prefix | **Never** (oracle context) |
| `fixtures/v6/from-c/**` | C-produced v6 fixture output tree | N/A |
| `crates/`, `perl/` | Native tools / candidate facade | **Never** (oracle context) |

### Dual-path interaction

| Requirement | Detail |
|-------------|--------|
| Legacy-only | Must succeed **without** building `collector/` or a C v6 writer |
| Optional-native | Unchanged; Cargo optional for Rust tools only |
| Overlay collector | Opt-in when testing sink/writer; not a dependency of `make legacy-smoke` or the legacy half of `offline_gate` |
| Pin archives | Never rewritten by collector or fixture jobs |

### CI / offline_gate notes (neutrality proof)

| Phase | Gate expectation |
|-------|------------------|
| **Pre-sink (today)** | `./scripts/ci/offline_gate.sh` must stay green when `collector/` is absent; no new hard dependency on a C toolchain for R1-preview steps |
| **COL-001+ (sink)** | When overlay builds are available, add fail-fast **v5-via-sink neutrality** checks (oracle-aligned stream on the agreed corpus) plus isolation asserts; **honest skip** when C toolchain / overlay build is absent |
| **Isolation** | Parent offline gate still must not put `crates/` (or `collector/` install) on oracle `PERL5LIB`; child smokes own isolation |
| **Fixtures** | C writer / dual harness writes under `fixtures/v6/from-c/**` only; never mutates `baseline/6.15/archives/` |

**Merge rule:** PR-B02 (COL-001 semantic sink) **must not merge** without ADR-0003 accepted (this policy section + ADR). This ADR alone does **not** claim COL-007 product, wire freeze, or CLI v6 default.

---

## How to verify each path

### Offline R1 gate (CI-OFFLINE-GATE / CI-OFFLINE-GATE-EXPAND / CI-QUERY-JSON-GATE / CI-CAPABILITY-GATE — recommended single operator entry)

Single **fail-fast** gate for critical offline R1 checks. Not a multi-OS CI matrix (**BUILD-006**).

```sh
./scripts/ci/offline_gate.sh
# after perl Makefile.PL:
make offline-gate
```

| Step | What | Cargo |
|------|------|-------|
| 1 | `cargo test -p nytprof-format-v5 -p nytprof-format-v6 -p nytprof-model -p nytprof-report -p nytprof-cli` | **Honest skip** if `cargo` / `crates/` absent |
| 2 | `./tools/oracle/selftest_harness.sh` | **Required** (oracle normalize/compare + nested selftests) |
| 3 | `./scripts/packaging/dual_path_smoke.sh` | **Primary packaging path** (legacy always; native if cargo present — installs `prefix/bin` when cargo present) |
| 4 | `./scripts/packaging/engine_auto_fallback_smoke.sh` | **Required** (ENGINE-AUTO-FALLBACK; needs native discoverable or cargo to build) |
| 5 | `./scripts/packaging/perl_jsonl_data_all_smoke.sh` | **Required** (pure-Perl JsonlData roll-up: data / line_totals / subdefs / source / a4b / meta / pid / stream_complete / discount / sub_entry) |
| 6 | `./scripts/packaging/perl_query_json_smoke.sh` | **Required** (**CI-QUERY-JSON-GATE** / QUERY-JSON-MVP / QUERY-JSON-EXPAND: golden `query --json --jsonl`; no cargo) |
| 6b | `./scripts/packaging/json_sub_entry_smoke.sh` | **Required** (JSON-SUB-ENTRY-MVP: `sub_entry_events` **0** / **27**; pure-Perl golden; native when available) |
| 6c | `./scripts/packaging/json_blocks_smoke.sh` | **Required** (JSON-BLOCKS-MVP: blocks-calls1 `line_calls_1_5` **780** / `block_line_calls_1_4` **810**; pure-Perl golden; optional native) |
| 6h | `./scripts/packaging/json_event_counts_smoke.sh` | **Required** (JSON-EVENT-COUNTS-MVP: default-calls1 `sub_return`/**27** `new_fid`/**3** `sub_callers`/**13** `src_line`/**632** `sub_info`/**31**; pure-Perl golden; optional native) |
| 7 | `./scripts/packaging/native_agg_json_smoke.sh` | **Optional when native:** NATIVE-AGG-JSON structured aggregates JSON (**15/3/15**) |
| 8 | `./scripts/packaging/native_query_json_cross_smoke.sh` | **Optional when native:** NATIVE-QUERY-JSON-CROSS — native `report --json` vs Perl `query --json` shared fields (**15/3/15** + discount **818**); pure-Perl query alone is step 6 |
| 9 | `./scripts/packaging/capability_selftest_smoke.sh` | **CI-CAPABILITY-GATE:** run when cargo **or** `prefix`/`target` native CLI (or `$NYTPROF_NATIVE_CLI`) present; **honest skip** otherwise (same condition as `packaging_gate`) |

Rules:

- Exit non-zero on the first failing step; clear banners per step.
- When cargo is absent: skip step 1 with a clear message; still run harness + packaging + expand steps (step 4 needs a discoverable native CLI or cargo; steps 5–6 are pure-Perl golden JSONL; steps 7–9 skip unless prefix/target/`NYTPROF_NATIVE_CLI` already present).
- After step 3 with cargo, dual-path typically installs `prefix/bin`, so steps 7–9 usually run on developer hosts with a Rust toolchain.
- Never puts `crates/` on oracle `PERL5LIB` (parent does not source oracle env; children own isolation).
- **Primary packaging:** `dual_path_smoke.sh` (BUILD dual-path policy). Not re-run here: `packaging_gate.sh` (broader suite) or `makemaker_dual_path_smoke.sh` (MakeMaker facade).
- **Expand (CI-OFFLINE-GATE-EXPAND):** after dual-path, also run `engine_auto_fallback_smoke.sh` and the thin `perl_jsonl_data_all_smoke.sh` roll-up of pure-Perl JsonlData smokes.
- **Query JSON (CI-QUERY-JSON-GATE):** step 6 wires required `perl_query_json_smoke.sh` (QUERY-JSON-MVP / QUERY-JSON-EXPAND golden `--jsonl`; fail if script missing; no cargo).
- **JSON-SUB-ENTRY-MVP / JSON-BLOCKS-MVP:** steps 6b–6c required pure-Perl (golden `--jsonl`); native half optional when CLI present.
- **Native aggregates / cross (NATIVE-AGG-JSON / NATIVE-QUERY-JSON-CROSS):** steps 7–8 when native CLI available; cross asserts shared fields equal between `report --json` and `query --json`.
- **Capability (CI-CAPABILITY-GATE):** step 9 wires `capability_selftest_smoke.sh` into the offline gate with packaging_gate’s native-available condition (fail-fast when native can be exercised).

### Dual-path entry (BUILD policy packaging half)

```sh
./scripts/packaging/dual_path_smoke.sh
```

Behavior:

1. **Always** runs `legacy_only_smoke.sh` (required; fails if the legacy path needs Cargo or is broken).
2. **If `cargo` is on `PATH`:** runs `install_native.sh` (when present) + `native_install_smoke.sh`, or falls back to `native_optional_smoke.sh`; **fails** if those fail.
3. **If `cargo` is missing:** prints an honest skip for the native half and **exits 0** when legacy passed.
4. Never puts `crates/` on oracle `PERL5LIB` (child smokes own isolation).

### Per-tier scripts

| Script | Tier | Cargo |
|--------|------|-------|
| [`scripts/ci/offline_gate.sh`](../scripts/ci/offline_gate.sh) | offline R1 gate (CI-OFFLINE-GATE-EXPAND + CI-QUERY-JSON-GATE + CI-CAPABILITY-GATE) | Cargo tests skip if absent; harness + dual-path + engine_auto_fallback + JsonlData roll-up + query-JSON required; capability when cargo/prefix/target present |
| [`scripts/packaging/engine_auto_fallback_smoke.sh`](../scripts/packaging/engine_auto_fallback_smoke.sh) | offline gate step 4 | Prefer-native / fall-back-legacy; never `crates/` on oracle PERL5LIB |
| [`scripts/packaging/perl_jsonl_data_all_smoke.sh`](../scripts/packaging/perl_jsonl_data_all_smoke.sh) | offline gate step 5 | Thin fail-fast roll-up of pure-Perl JsonlData smokes |
| [`scripts/packaging/perl_query_json_smoke.sh`](../scripts/packaging/perl_query_json_smoke.sh) | offline gate step 6 (CI-QUERY-JSON-GATE) | QUERY-JSON-MVP / QUERY-JSON-EXPAND: `query --json --jsonl` golden; pure-Perl; no cargo |
| [`scripts/packaging/json_sub_entry_smoke.sh`](../scripts/packaging/json_sub_entry_smoke.sh) | offline gate step 6b | JSON-SUB-ENTRY-MVP: `sub_entry_events` **0**/**27** |
| [`scripts/packaging/json_blocks_smoke.sh`](../scripts/packaging/json_blocks_smoke.sh) | offline gate step 6c | JSON-BLOCKS-MVP: blocks-calls1 **780**/**810** greppable A4/A4b ints |
| [`scripts/packaging/native_agg_json_smoke.sh`](../scripts/packaging/native_agg_json_smoke.sh) | offline gate step 7 (optional when native) | NATIVE-AGG-JSON: `report --json` ×2 → **15/3/15** |
| [`scripts/packaging/native_query_json_cross_smoke.sh`](../scripts/packaging/native_query_json_cross_smoke.sh) | offline gate step 8 (optional when native) | NATIVE-QUERY-JSON-CROSS: native `report --json` vs Perl `query --json` shared fields **15/3/15** + discount **818** |
| [`scripts/packaging/capability_selftest_smoke.sh`](../scripts/packaging/capability_selftest_smoke.sh) | offline gate step 9 (CI-CAPABILITY-GATE); also packaging_gate | CAPABILITY-SELFTEST + CAPABILITY-JSON-MVP; fails closed without CLI/cargo when invoked directly; offline/packaging gates skip honestly when native unavailable |
| [`scripts/packaging/legacy_only_smoke.sh`](../scripts/packaging/legacy_only_smoke.sh) | legacy-only | Must not invoke |
| [`scripts/packaging/install_native.sh`](../scripts/packaging/install_native.sh) | optional-native | Required |
| [`scripts/packaging/native_install_smoke.sh`](../scripts/packaging/native_install_smoke.sh) | optional-native | Required (expects prefix install) |
| [`scripts/packaging/native_optional_smoke.sh`](../scripts/packaging/native_optional_smoke.sh) | optional-native | Skips cleanly if absent |
| [`scripts/packaging/dual_path_smoke.sh`](../scripts/packaging/dual_path_smoke.sh) | both (policy entry / offline gate packaging primary) | Legacy always; native if present |
| [`scripts/packaging/makemaker_dual_path_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/makemaker_dual_path_smoke.sh) | both (MakeMaker entry) | `Makefile.PL` + `make legacy-smoke`; native via make when cargo present |
| [`scripts/packaging/install_facade.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/install_facade.sh) | BUILD-003-DEPTH (facade half) | Pure-Perl engine + modules → `$NYTPROF_PREFIX` (default `$REPO/prefix`); **no cargo**; shared root resolve with native |
| [`scripts/packaging/resolve_packaging_prefix.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/resolve_packaging_prefix.sh) | BUILD-003-DEPTH (shared) | Identical `NYTPROF_PREFIX` / bare-`PREFIX` denylist (`$HOME/perl5`, `*/perl5`, trailing `/`) for dual-install |
| [`scripts/packaging/makemaker_build003_depth_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/makemaker_build003_depth_smoke.sh) | BUILD-003-DEPTH | Facade without cargo + dual-install when cargo; honesty stamps |
| Root [`Makefile.PL`](https://github.com/hilather/nytprof-modernization/blob/main/Makefile.PL) | packaging facade + depth | Default legacy; `NYTPROF_NATIVE=0\|1\|auto`; `install-facade` / `dual-install`; not full XS CPAN; `make offline-gate` |
| [`scripts/packaging/packaging_gate.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/packaging_gate.sh) | broader packaging gate | Mixed fail-fast (legacy + engine select + Perl dispatch + native when present) |

Broader operator gate (includes engine-selection and Perl facade smokes beyond dual-path tiers):

```sh
./scripts/packaging/packaging_gate.sh
```

Oracle rebuild (Cargo-free forever):

```sh
./scripts/baseline/run_all.sh
```

Native install MVP contract: [`docs/schemas/native-install-mvp-v0.md`](schemas/native-install-mvp-v0.md).

---

## Explicit non-goals (open / future)

| Item | Status | Notes |
|------|--------|-------|
| Full CI matrix | **BUILD-006** — open | Multi-OS / multi-Perl / multi-rustc jobs not required by dual-path policy or CI-OFFLINE-GATE |
| Multi-OS prebuilt binaries | open (ADR-Q016) | Distribution model undecided |
| Full MakeMaker ↔ Cargo CPAN dual-build | **BUILD-003** full — open | **BUILD-003-DEPTH** partial (facade + prefix dual-install) landed; still not a complete XS CPAN tarball |
| Multi-OS CI / CPAN upload | **BUILD-006** / release — open | Not required by the candidate packaging entry; offline gate is single-host only |
| COMPAT-009 final tier freeze / full BUILD-001 ADR | open | This doc is the runnable dual-path draft feeding that freeze |
| MSRV freeze | open (ADR-Q017) | Optional-native uses whatever `rustc` is installed until frozen |
| Default engine/format flips to native | out of first slice | Native remains opt-in |
| Collector overlay sources / COL-001 sink | open (blocked only on layout — **ADR-0003 accepted**) | Tree lands in sink PRs under `collector/`; not shipped by dual-path policy alone |
| COL-007 C v6 writer / wire freeze / CLI v6 default | deferred / open | ADR-0003 does **not** complete these |

---

## Relationship to packaging spike

- Spike charter and layout notes: [`docs/PACKAGING_SPIKE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PACKAGING_SPIKE.md)
- Collector overlay layout (B0-A): [`docs/adrs/0003-collector-packaging-source-tree.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-collector-packaging-source-tree.md)
- This policy **lands** the dual-path contract with a dedicated smoke; the spike remains historical/background detail.
- **CI-OFFLINE-GATE** / **CI-OFFLINE-GATE-EXPAND** / **CI-QUERY-JSON-GATE** / **CI-CAPABILITY-GATE** (`scripts/ci/offline_gate.sh`) is the single offline R1 fail-fast entry (cargo tests? + harness + dual-path packaging primary + engine_auto_fallback + pure-Perl JsonlData roll-up + required query-JSON smoke + capability_selftest when native available).
- Unified packaging gate remains the broader packaging fail-fast suite; dual-path smoke is the support-tier-focused packaging half.
- Candidate MakeMaker entry (`Makefile.PL` + `makemaker_dual_path_smoke.sh`) is the dual-path **packaging facade**; **BUILD-003-DEPTH** adds `install-facade` / `dual-install` / depth smoke toward full BUILD-003 without completing XS CPAN dual-build.

---

## Definition of done for BUILD-DUAL-PATH

- [x] Support tiers documented (legacy-only vs optional-native)
- [x] Critical path: legacy never requires Cargo
- [x] `crates/` never on oracle `PERL5LIB`
- [x] Runnable `dual_path_smoke.sh` (legacy required; native if cargo present; honest skip otherwise)
- [x] Non-goals called out (BUILD-003 full, BUILD-006, multi-OS prebuilts)
- [x] Board row + packaging spike pointer

## Definition of done for BUILD-MAKEMAKER-OPT

- [x] Root candidate `Makefile.PL` (facade; not full XS CPAN)
- [x] `NYTPROF_NATIVE=0` default legacy; `=1` requires cargo; `make native` / `native-install`
- [x] Targets: `legacy-smoke`, `dual-path-smoke`, `native-install` → existing packaging scripts
- [x] `perl Makefile.PL && make legacy-smoke` works without cargo
- [x] `scripts/packaging/makemaker_dual_path_smoke.sh` (exit non-zero on failure)
- [x] Docs: this section + PACKAGING_SPIKE / README notes; board row before COL-007

## Definition of done for CI-OFFLINE-GATE

- [x] Runnable `scripts/ci/offline_gate.sh` (fail-fast banners; exit non-zero on first failure)
- [x] Step 1: focused `cargo test` of offline packages with honest skip if cargo/crates absent
- [x] Step 2: required `tools/oracle/selftest_harness.sh`
- [x] Step 3: primary packaging = `dual_path_smoke.sh` (documented; packaging_gate / makemaker not required in-gate)
- [x] Never puts `crates/` on oracle `PERL5LIB`
- [x] Optional `make offline-gate` via root `Makefile.PL`
- [x] Docs: this section + README; board row **before COL-007**
- [x] Non-goal: full multi-OS CI (**BUILD-006**)

## Definition of done for CI-OFFLINE-GATE-EXPAND

- [x] Step 4: required `scripts/packaging/engine_auto_fallback_smoke.sh` (ENGINE-AUTO-FALLBACK)
- [x] Step 5: required `scripts/packaging/perl_jsonl_data_all_smoke.sh` (thin roll-up of pure-Perl JsonlData smokes)
- [x] Roll-up covers: `perl_jsonl_data` + `perl_line_totals` + `perl_subdefs` + `perl_source` + `perl_a4b` + `perl_meta` + `perl_pid` + `perl_stream_complete` + `perl_discount` + `perl_sub_entry`
- [x] Fail-fast banners for new steps; never puts `crates/` on oracle `PERL5LIB`
- [x] Docs: this section + README offline gate table; board row **before COL-007**
- [x] Non-goal: still not multi-OS CI (**BUILD-006**); still not full `packaging_gate` breadth

## Definition of done for CI-CAPABILITY-GATE

- [x] Step 9: `scripts/packaging/capability_selftest_smoke.sh` wired into `scripts/ci/offline_gate.sh` (after native-agg / cross optional steps)
- [x] Fail-fast when native CLI can be exercised (cargo **or** `prefix/bin/{nytprof-cli,nytprof-dump}` **or** `target/{debug,release}/nytprof-dump` **or** `$NYTPROF_NATIVE_CLI`)
- [x] Honest skip when none of the above (same pattern as `packaging_gate` capability step)
- [x] Document that dual_path with cargo typically installs `prefix/bin` before native steps, so capability usually runs on cargo hosts
- [x] Fail-fast banner; never puts `crates/` on oracle `PERL5LIB`
- [x] Docs: this section + README offline gate table + residual matrix offline gate row; board row **before COL-007**
- [x] Non-goal: still not multi-OS CI (**BUILD-006**); still not full `packaging_gate` breadth

## Definition of done for CI-QUERY-JSON-GATE

- [x] Step 6: required `scripts/packaging/perl_query_json_smoke.sh` wired into `scripts/ci/offline_gate.sh` (after `perl_jsonl_data_all_smoke`; before native-agg / capability)
- [x] Fail-fast if script missing; clear **OFFLINE GATE** banner for the step
- [x] Golden `--jsonl` path only; does not require cargo
- [x] Never puts `crates/` on oracle `PERL5LIB`
- [x] Docs: this section + README offline gate table + residual matrix offline gate row + R1_PREVIEW_OPERATOR_RUNBOOK; board row **before COL-007**
- [x] Non-goal: still not multi-OS CI (**BUILD-006**); still not full `packaging_gate` breadth

## Definition of done for NATIVE-QUERY-JSON-CROSS

- [x] Runnable `scripts/packaging/native_query_json_cross_smoke.sh` (fail-fast; fails closed without native)
- [x] Invokes real CLIs only: native `report --json` + Perl `query --json --jsonl` (no re-aggregation)
- [x] Asserts shared fields equal: `leaf_returns` **15**, `mid_returns` **3**, `mid_leaf_edge` **15**, `discount_events` **818**
- [x] Pair run twice for consistency; optional `query --json <profile>` dump path
- [x] Never puts `crates/` on oracle `PERL5LIB`
- [x] Wired into `offline_gate.sh` when native available (step 8); pure-Perl query remains step 6
- [x] Schema notes in `native-aggregates-json-mvp-v0.md` + `perl-engine-dispatch-mvp-v0.md`; board row **before COL-007**

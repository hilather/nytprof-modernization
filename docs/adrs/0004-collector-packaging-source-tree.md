# ADR-0004 - Collector packaging / source-tree layout (B0-A overlay)

- **Status:** accepted
- **Date:** 2026-08-11
- **Owners/approvers:** build/release lead; collector (C/XS) lead
- **Related ADR-Q:** design-program **OQ-8** (collector source-tree layout — **not** plan [`ADR-Q008`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md) chunk-size policy); in-repo SoT: resolved entry **BUILD-LAYOUT** in that same queue file; packaging dual-path in [`docs/BUILD_SUPPORT_POLICY.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md); BASE-001 pin under [`baseline/6.15/`](https://github.com/hilather/nytprof-modernization/blob/main/baseline/6.15/README.md)
- **Related tasks/risks/gates:** COL-001..006 (sink), COL-007 (C v6 writer — still deferred product), BUILD-001/003 dual-path, BUILD-006 (not required here), RSK-009 / COMPAT-011 (legacy without Rust), oracle contamination (CR-05), `./scripts/ci/offline_gate.sh`
- **Decision scope/version:** repository layout and isolation rules for modernization collector C/XS work through R2 runway; **not** a v6 wire freeze, COL-007 product claim, or full CPAN XS dual-build (BUILD-003)

## Context

R2 critical path requires a **v5-neutral semantic sink** (COL-001..) and later a **C v6 writer** (COL-007). Collector sources must live somewhere the build and CI can compile and test them without:

1. contaminating the **pinned 6.15 oracle** under `baseline/6.15/` (archives + install used for differential fixtures);
2. putting `crates/`, candidate `perl/`, or overlay collector install prefixes on oracle `PERL5LIB`;
3. making legacy-only / offline packaging require a C writer or Cargo;
4. rewriting immutable oracle archives when regenerating install trees or producing v6 fixtures.

Two layouts were considered for the R2 collector runway (B0 options restated normatively in Decision below; external design notes are non-normative background only):

| Option | Description |
|--------|-------------|
| **B0-A Overlay** | Modernization collector sources live in a **parallel tree** (e.g. `collector/`); oracle pin remains archives + isolated install for differential tests |
| **B0-B Patch-in-pin** | Edit sources under `baseline/6.15/src` with “no archive rewrite” discipline; regenerate install only via scripts |

**PR-B02 (COL-001 sink) must not merge without this ADR accepted.** Sink and writer PRs implement against the layout frozen here.

## Evidence

| Source | Role |
|--------|------|
| [`baseline/6.15/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/baseline/6.15/README.md) | Pin identity; `src/` / `install/` gitignored build products; isolation rule |
| [`scripts/baseline/*`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/baseline/common.sh) | Oracle fetch/build/test never requires Cargo; `PERL5LIB` from install only |
| [`tools/oracle/env.sh`](https://github.com/hilather/nytprof-modernization/blob/main/tools/oracle/env.sh) | Fixture tools refuse non-install NYTProf load paths |
| [`docs/BUILD_SUPPORT_POLICY.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) | Dual-path tiers: legacy-only (no Cargo) vs optional-native |
| [`docs/PACKAGING_SPIKE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PACKAGING_SPIKE.md) | Layout: `crates/` optional; oracle-first |
| [`scripts/ci/offline_gate.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/offline_gate.sh) | Fail-fast offline R1 gate; never puts `crates/` on oracle `PERL5LIB` |
| [`scripts/packaging/legacy_only_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/legacy_only_smoke.sh) | Cargo-free legacy packaging proof |
| This ADR Decision §§1–6 | Normative B0-A vs B0-B choice, isolation, fixtures, offline_gate neutrality, dual-path (external design Phase B.0 / design-program OQ-8 is background only) |

No collector overlay sources are required to exist **in this ADR commit** — the decision freezes layout and isolation so COL-001 can land sources under the agreed tree.

## Decision

### 1. Choose **B0-A Overlay** (accepted)

Modernization collector C/XS sources **MUST** live under a repository-root overlay tree:

```text
collector/                    # B0-A overlay root (canonical)
  include/                    # public/internal C headers (sink API, provisional v6 IDs when added)
  src/                        # C/XS sources for sink + writers (PR-B02+)
  xs/                         # optional XS glue not part of the oracle pin
  t/                          # C/XS unit / mapping tests owned by modernization
  README.md                   # build/test entry when tree lands
```

**Alias rejected as primary:** `src/collector/` is acceptable only as a **symlink or documented secondary** if a later packaging ADR renames; new work **MUST** use `collector/` unless this ADR is superseded.

**Oracle pin remains immutable for differential use:**

```text
baseline/6.15/
  archives/          # committed pin (tarball + checksums) — never rewrite
  oracle-*.txt       # committed pin metadata
  manifest.json
  src/               # gitignored extract/build tree from archives only
  install/           # gitignored install tree from scripts/baseline/build_oracle.sh
```

### 2. Reject **B0-B Patch-in-pin** as the default

Editing `baseline/6.15/src` for modernization sink/writer work is **rejected** as the product layout because:

- `src/` is a **local extract** of the pin, not a modernization source of truth;
- risk of accidental archive/install contamination and non-reproducible oracle rebuilds;
- dual-path packaging and offline gate isolation become harder to prove.

**Narrow exception (not a layout flip):** throwaway local experiments under a **non-committed** extract are allowed for investigation only; they must not be committed, must not alter `baseline/6.15/archives/`, and must not be cited as the COL-001..007 source tree.

### 3. Oracle pin isolation (normative)

| Rule | Detail |
|------|--------|
| Oracle `PERL5LIB` | Built only from `baseline/6.15/install` (+ optional `baseline/6.15/test-deps/`) |
| Forbidden on oracle `PERL5LIB` | Any path under `crates/`, candidate `perl/`, or `collector/` (including `collector/install/` / `prefix/collector/` candidate installs) |
| Oracle rebuild | `./scripts/baseline/run_all.sh` (or step scripts) — **Perl/C only**; never requires Cargo or `collector/` build |
| Pin archives | Committed under `baseline/6.15/archives/`; **never rewritten** by collector or fixture jobs |
| Install regeneration | Only via `scripts/baseline/build_oracle.sh` from the **unmodified** pin extract |
| Contamination fail-closed | Load of `Devel::NYTProf` from outside install tree fails oracle/env smokes |

Collector overlay builds that produce a **candidate** install for sink tests **MUST** use a separate prefix (e.g. `collector/install/` or `prefix/collector/`), never `baseline/6.15/install`.

Path-component asserts for `collector/` on oracle `PERL5LIB` land with COL-001 packaging smokes; today `legacy_only_smoke` / oracle env primarily scan for `crates/` and non-install load paths.

### 4. Fixture production path (C writer / dual later)

When C-produced v6 (or dual) profiles are checked in or regenerated:

| Path | Role |
|------|------|
| `fixtures/v6/from-c/**` | Canonical location for profiles / dumps produced by the **overlay collector** C path |
| `fixtures/v5/**` | Existing golden v5 fixtures from **oracle** capture tools — remain oracle-driven |
| Production harness | Scripts under `tools/` or `scripts/` that build/run the **overlay** collector binary/module; write under `fixtures/v6/from-c/` (or a temp dir then promote) |
| Forbidden | Mutating `baseline/6.15/archives/*`; writing C fixtures into the oracle install tree; putting `crates/` or overlay install on oracle `PERL5LIB` during capture |

Stand-in / Rust preflight encoders may continue to produce **non-product** vectors under crate tests or provisional schema docs; product dual-equality **C** evidence uses `fixtures/v6/from-c/**` once COL-007 bytes exist.

### 5. Offline gate / CI neutrality proof

**Pre-sink:** `./scripts/ci/offline_gate.sh` remained the offline R1 fail-fast gate (cargo tests optional; oracle harness + dual-path packaging + expand steps required). It stayed **neutral** to a missing `collector/` tree: absence of overlay sources must not fail the gate.

**COL-001 scaffold (PR-B02) — merge bar (landed):**

| Requirement | Status |
|-------------|--------|
| Isolation asserts — no oracle `PERL5LIB` entry under `crates/`, `perl/`, or overlay `collector/` / install paths | **Required** — `scripts/packaging/collector_sink_smoke.sh` (offline_gate step 10) |
| Honest skip when no C toolchain / overlay not built | **Required** — same smoke; legacy dual-path half independent |
| Compile + unit-test sink scaffold when CC present | **Required for scaffold** — `make -C collector test` |
| **v5-via-sink stream neutrality** (oracle-aligned profiles/dumps on corpus; stream order/multiplicity) | **Deferred** to **COL-006** (real v5 writer behind the sink API) with lifecycle/seq support from **COL-002** / **COL-003**; fake-clock corpus gates land with **TEST-003** / PR-B03 |

**Rationale:** COL-001 introduces the semantic sink boundary and B0-A tree; the stub v5 adapter counts events only and does **not** encode wire bytes. Requiring oracle stream equality before COL-006 would force a false product claim. Full “proof of neutrality” for production collection remains merge-blocking for **COL-006** (and dual when applicable), not the interface-only scaffold.

Wiring: `offline_gate.sh` step 10 + `collector_sink_smoke.sh` (implementation detail may evolve).

### 6. Dual-path packaging (legacy without C writer)

| Mode | Cargo | Overlay C writer / sink build | Expected |
|------|-------|-------------------------------|----------|
| **Legacy-only** | Not required | Not required | Oracle rebuild, `legacy_only_smoke`, pure-Perl JsonlData / query paths succeed |
| **Optional-native** | Yes for Rust tools | Not required for native report | Existing R1-preview native CLI path unchanged |
| **Overlay collector (R2 runway)** | Not required for v5 sink | Required only when testing/building collector | Opt-in configure/make target or dedicated smoke; **must not** become default dependency of `make legacy-smoke` / `offline_gate` legacy half |

Root `Makefile.PL` dual-path facade remains **default legacy**. Full MakeMaker ↔ XS CPAN dual-build that ships the overlay as product is **BUILD-003** / later packaging work — out of scope for this ADR.

### 7. Numbering note (ADR-0001 .. ADR-0004)

| Number | Topic | Track |
|--------|-------|-------|
| **ADR-0001** | Format v6 event-body packing candidate | R2 format (PR-B01) |
| **ADR-0002** | Format v6 FOOTER string-pool / dictionary candidate | R2 format (PR-B01) |
| **ADR-0003** | Full R1 residual policy (CLOSE / WAIVE / OUT-OF-R1) | Track A residual (PR-A04) |
| **ADR-0004** | Collector packaging / source-tree (this ADR) | Track B packaging (PR-B00) |

This packaging decision is **ADR-0004** so it does not collide with packing ADRs (0001/0002) or the residual-policy ADR (0003).

**Merge handoff:** when rebasing or merging with PR-B01 / PR-A04, treat [`docs/adrs/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/README.md) as a **multi-row index** — preserve all accepted/proposed rows (0001–0004) rather than replacing the whole table.

## Exactness and compatibility consequences

- **v5 oracle behavior** is unchanged; differential fixtures continue to use pin install only.
- **No wire-format change** from this ADR.
- **No product COL-007/COL-008 claim**; no CLI v6 default; no R3/R4 flips.
- Sink/writer implementation PRs must not edit pin archives or replace oracle install with candidate modules on oracle `PERL5LIB`.
- CPAN legacy install path continues to require neither Cargo nor the overlay C writer until an explicit later packaging ADR changes support tiers.

## Alternatives considered

| Alternative | Correctness/compatibility | Performance/storage | Security/reliability | Build/portability | Reason accepted/rejected |
|---|---|---|---|---|---|
| **B0-A Overlay** (`collector/`) | Clear isolation from pin; dual-path honest | Extra build wiring once | Lower contamination risk | Parallel tree + separate prefix | **Accepted** |
| **B0-B Patch-in-pin** (`baseline/6.15/src`) | Easy short-term edit of upstream tree | None | High contamination / non-repro pin risk | Couples modernization to local extract | **Rejected** as default |
| Vendor full 6.15 tree at repo root as sole sources | Diverges from BASE-001 pin model | Large tree | Pin checksum story weakens | Confuses oracle vs candidate | Rejected for R2 runway |
| Collector only inside `crates/` via FFI | Violates “no Rust on statement hot path” | — | Wrong architecture | Fights dual-path | Rejected (architecture baseline) |

## Implementation and testing requirements

| Requirement | Owner slice |
|-------------|-------------|
| Land `collector/` tree + sink headers/sources | PR-B02 (COL-001) and follow-ons — layout unblocked by this **accepted** ADR |
| Separate candidate install prefix; never `baseline/6.15/install` | All collector build scripts |
| Preserve `scripts/baseline/*` Cargo-free and archive-immutable | BUILD / BASE-001 |
| Fixture harness → `fixtures/v6/from-c/**` without pin mutation | COL-007 / dual-equality when C bytes exist |
| offline_gate: remain green without hard CC dep; COL-001 scaffold = isolation + honest skip + unit tests; oracle stream equality with COL-006 | CI + COL-001 scaffold / COL-006 |
| Update BUILD policy cross-links (this change set) | PR-B00 |
| Regression: legacy_only_smoke + offline_gate must pass on a host without overlay build | merge gate |

## Migration, rollout, and rollback

- **Rollout:** documentation-only until PR-B02 creates `collector/`. No operator-facing default change.
- **Feature flags:** none required for the layout decision; collector enablement uses existing dual-path / future Make targets.
- **Rollback:** supersede this ADR only with a new accepted ADR; do not silently patch the pin. If overlay proves unworkable for CPAN packaging, a superseding ADR may adopt a controlled hybrid — still forbidding archive rewrite.
- **Already-produced files:** none depend on overlay layout yet.

## Revisit triggers

- BUILD-003 full CPAN XS packaging cannot ship overlay without an accepted layout change.
- Evidence that oracle isolation is broken by overlay install discovery.
- Decision to retire the separate pin tree (would require BASE-001 supersession + major program ADR).
- Security finding on fixture production writing into pin paths.

## Non-claims

This ADR does **not**:

- mark COL-007, COL-008, wire freeze, or CLI v6 default done;
- implement the semantic sink or C v6 writer;
- freeze MSRV, multi-OS CI (BUILD-006), or prebuilt native distribution (ADR-Q016);
- change dual-path defaults or require Cargo for legacy-only installs.

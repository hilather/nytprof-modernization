# ADR-0007 - Production v6 writer backend: reaffirm C baseline (COL-009)

- **Status:** **accepted**
- **Date:** 2026-08-12
- **Accepted:** 2026-08-12 (R2-preview packaging cut — PR-B13)
- **Owners/approvers:** architecture review group; maintainers
- **Related ADR-Q:** plan COL-009; charter “C on collector hot path”; design Key Decision 2 / CR-06
- **Related tasks/risks/gates:** COL-007 (product E3-EVENT **done** PR-B09); COL-008 (batched Rust writer — **deferred** non-baseline); BUILD-004 packaging; dual-equality E3/E4; **not** BENCH-006 (no COL-008 measurement package opened)
- **Decision scope/version:** production **writer backend** selection for format major=6; format wire IDs remain implementation-independent (ADR-0006)

## Context

Plan task **COL-009** requires an explicit decision of the production v6 writer backend after COL-007 lands. Two historical candidates:

| Candidate | Plan task | Role |
|-----------|-----------|------|
| **C v6 writer** | COL-007 | Collector hot-path emitter via semantic sink (`nytp_sink_v6` / `nytp_v6_sink_*`) |
| **Batched Rust writer via FFI** | COL-008 | Optional measured alternative only; never per-event FFI |

The program charter and completion design already treat **C as baseline** and COL-008 as **non-baseline**. COL-007 product E3-EVENT is **done** (C fixtures + `e3_c_*` + offline_gate step 11). COL-008 was **never re-opened** with dual-equality-green evidence that a batched Rust encoder is worth packaging/ABI cost. COL-009 therefore **reaffirms** the existing baseline rather than starting a competitive bake-off.

## Evidence

| Item | Path / note |
|------|-------------|
| C product E3-EVENT | [`fixtures/v6/from-c/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v6/from-c/); `crates/nytprof-format-v6/tests/e3_c.rs`; `tools/oracle/e3_c_writer_parity.sh` |
| C sink + packing / dict / codecs | `collector/` (`nytp_sink_v6*`); schemas `collector-v6-*-mvp-v0.md` |
| Wire freeze (IDs independent of backend) | [ADR-0006](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0006-v6-wire-freeze.md) |
| COL-008 plan non-baseline | [`docs/plan/05_COLLECTOR_AND_C_XS_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/05_COLLECTOR_AND_C_XS_TASKS.md) COL-008 |
| Dual-equality readiness | [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md) |
| No COL-008 / BENCH-006 package | No re-open ADR; no certified COL-008 vs COL-007 measurement set in this cut |

**Measurement placeholders (only if COL-008 is re-opened later):** wall time, peak RSS, profile size, binary size, portability matrix, sanitizer cleanliness, packaging dual-path impact — recorded under plan BENCH-006 / BUILD-004 with dual-equality still green on C.

## Decision

1. **Production v6 writer backend = C (COL-007).** Collector emission of major=6 profiles uses the C writer on the semantic sink. No per-event Rust/FFI on the hot path.
2. **COL-008 remains deferred / non-baseline.** Do **not** start a batched Rust writer as product baseline. Re-open only via a superseding ADR after dual-equality with the C writer is green **and** measured evidence shows packaging/ABI cost is justified.
3. **Format remains backend-independent.** Wire numeric IDs and layouts are frozen by ADR-0006; consumers (Rust always-inflate, ProfileModel, CLI) must not depend on C vs hypothetical Rust encoder internals.
4. **Fallback policy:** product dual-path continues to support **v5** collection/default (`collection_default: v5`) and legacy 6.15 paths without requiring the C v6 writer. R2-preview v6 is **opt-in** (CLI read/report on v6 files; collection default flip is R4).
5. **No COL-008 measurement claim** under this ADR — absence of a bake-off is intentional given charter + COL-007 green + COL-008 never re-opened.

## Exactness and compatibility consequences

| Surface | Effect |
|---------|--------|
| Wire bytes | Unchanged by backend choice; ADR-0006 + golden vectors remain SoT for IDs |
| Dual-equality | Product E3 evidence remains **C bytes only** (`e3_c_*`); stand-in harness is not product evidence |
| CLI / capability | `v6_decode` / `v6_report` true; `convert`/`merge` residual; `collection_default: v5` |
| Packaging | C writer builds under collector overlay (ADR-0004); legacy-only without Cargo/CC still valid |
| COL-008 | Stays deferred; no product capability bit for “rust_writer” |

## Alternatives considered

| Alternative | Correctness/compatibility | Performance/storage | Security/reliability | Build/portability | Reason |
|---|---|---|---|---|---|
| **C writer baseline (this ADR)** | Matches charter; E3-EVENT green with C | Engineering light benches only; no public perf claim | No per-event FFI; fail-closed C limits | Overlay `collector/`; dual-path without C still works | **Accepted** |
| Open COL-008 as competing baseline now | Would need dual-equality + packaging ADR re-open | Unknown without BENCH-006 | Per-event FFI forbidden; batch ABI cost | Extra rustc+FFI packaging surface | **Rejected** — COL-008 not re-opened; no superior evidence |
| Defer COL-009 until COL-008 measured | Leaves production backend ambiguous at R2-preview | N/A | Ambiguity risks agents treating COL-008 as baseline | N/A | **Rejected** — process completeness requires reaffirm |

## Implementation and testing requirements

- Board row **COL-009** / **COL-008** honesty: C baseline done; COL-008 deferred.
- Residual matrix + dual-equality readiness + R2-preview release notes cite this ADR.
- Offline gate continues to prove E3-C + E4 product paths; no new COL-008 gate.
- Any future COL-008 re-open must: (1) supersede or amend this ADR, (2) keep format IDs frozen, (3) ban per-event FFI, (4) ship BENCH-006 evidence.

## Migration, rollout, and rollback

| Item | Policy |
|------|--------|
| R2-preview | Opt-in v6 **read/report** via magic detect; collection default **v5** |
| Rollback | Operators stay on v5 profiles / legacy 6.15; no format default flip |
| Files already produced | C-emitted v6 streams remain valid under ADR-0006 |

## Revisit triggers

- Maintainer-approved COL-008 re-open with dual-equality + BENCH-006 evidence package.
- Packaging (BUILD-004) or security finding that blocks C writer on an advertised platform tier.
- Superseding major format change (new major beyond 6) — does not by itself change backend preference.

## Non-claims

- **Not** a performance certification or “C is faster than Rust” claim.
- **Not** COL-008 done or measured.
- **Not** R3/R4 default flips; **not** convert/merge tooling; **not** E3-mixed complete; **not** R2-stable.
- **Not** a wire-ID change (see ADR-0006).

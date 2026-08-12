# Release notes — R2-stable packaging cut (PR-C05)

**Date:** 2026-08-12  
**PLAN_ID:** `8c9b1a63`  
**Board ID:** `R2-STABLE-READINESS-CUT`  
**Horizon:** charter **R2-stable** (Phase C certification depth on the integrated stack — **not** R3/R4 default flips)  
**Prior cut:** [RELEASE_NOTES_R2_PREVIEW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_PREVIEW.md) (PR-B13; opt-in only)  
**Dual-equality readiness:** [DUAL_EQUALITY_READINESS_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md)  
**Residual matrix:** [R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) (§ R2-stable)  
**Operator runbook:** [R1_PREVIEW_OPERATOR_RUNBOOK.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) (§ R2-stable honesty)

These notes freeze the **advertised R2-stable product scope** after Track C integration (PR-C01 convert, PR-C02 merge/repack/salvage, PR-C02b COL-015 fork MVP, PR-C03 security/fuzz package, PR-C04 P1/P2 methodology, on top of R2-preview Track B). They are **not** a CPAN upload statement, public performance certification, or permission to flip collection/engine defaults (R3/R4).

---

## Summary

| Theme | What ships under R2-stable | Honesty |
|-------|----------------------------|---------|
| **Collection default** | **v5** remains product default (`collection_default: v5`) | **No R4** format default flip |
| **v6 offline tools (E5)** | `dump` / `verify` / `report` / `html` / `csv` / `folded` / `callgrind` on v6 via magic auto-detect | **Opt-in read path** — not “v6 is default” |
| **Convert (PR-C01)** | `nytprof-cli convert --to=v5\|v6` strict path; capability `convert: yes` | No lossy mode; absolute v6 EVENT out only; refuse unrepresentable |
| **Merge / repack / salvage (PR-C02)** | Stream-concat merge, full re-encode repack, longest-prefix salvage; capability true | Not full `nytprofmerge` aggregate-sum parity |
| **Capability** | `v6_decode` / `v6_report` / `convert` / `merge` / `repack` / `salvage` **true**; `collection_default: v5` | Fail-closed dual-sink probes when fixtures present |
| **COL-015 fork/PID (PR-C02b)** | Protocol + batch preflush/discard + child reinit + stress suite | Not full TEST-018 oracle forkdepth/addpid; mid-deflate continue-in-child residual |
| **Security/fuzz (PR-C03)** | Offline SEC-FUZZ package; 0 critical/high on covered surfaces; offline_gate step | Not full SEC-002 continuous fuzz; not SEC-012 release review |
| **P1/P2 (PR-C04)** | Methodology + light harness (`size` / `collector_micro`) | **Public perf claims waived** until BENCH-001+ gates green |
| **COL-007 / COL-009 / wire freeze** | Unchanged from R2-preview (done) | E3-mixed residual; COL-008 deferred |
| **E4** | E4-v0 model + E4 product CLI (dual-sink scaled pairs) | Full oracle dual residual (TEST-008) |
| **Dual-path legacy** | Unchanged | Never put `crates/` on oracle `PERL5LIB` |
| **R3 / R4** | — | **Not claimed** |

---

## Capability honesty (stable markers)

Human:

```text
OK: native capability self-test
decode: yes
report: yes
verify: yes
v6_decode: yes
v6_report: yes
convert: yes
merge: yes
repack: yes
salvage: yes
collection_default: v5
```

JSON (selected fields):

```json
{"ok":true,"decode":true,"report":true,"verify":true,
 "v6_decode":true,"v6_report":true,"convert":true,"merge":true,
 "repack":true,"salvage":true,"collection_default":"v5"}
```

Schemas:

- [`docs/schemas/capability-selftest-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/capability-selftest-mvp-v0.md)
- [`docs/schemas/convert-strict-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/convert-strict-mvp-v0.md)
- [`docs/schemas/merge-repack-salvage-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/merge-repack-salvage-mvp-v0.md)

---

## Tooling (Phase C)

| Tool | CLI | Semantics |
|------|-----|-----------|
| **convert** | `nytprof-cli convert --to=v5\|v6 IN -o OUT` | Strict fail-closed; successful v5 outputs old-tool shape |
| **merge** | `nytprof-cli merge --to=v5\|v6 -o OUT IN…` | Every input fully decodes; stream-concat + fid remap |
| **repack** | `nytprof-cli repack [--to=v5\|v6] IN -o OUT` | Full decode required; clean re-encode |
| **salvage** | `nytprof-cli salvage [--to=v5\|v6] IN -o OUT` | Longest complete verified prefix; always labels incomplete |

Automatic salvage as default verify/report remains **forbidden** (COMPAT-010).

---

## Collector / security / perf

| Item | R2-stable status |
|------|------------------|
| COL-007 C v6 EVENT writer | **done** (PR-B09) |
| Wire freeze ADR-0006 | **done** (PR-B11) |
| COL-009 / ADR-0007 C baseline | **done** (PR-B13) |
| COL-014 dual-sink test/dev | **done** (PR-B10a) |
| COL-015 fork/PID MVP | **done** (PR-C02b scaffold/stress) |
| SEC-FUZZ-HARDENING-MVP | **done** (PR-C03 offline package) |
| P1/P2 methodology | **done** (PR-C04); **public claims waived** |
| COL-008 batched Rust writer | **deferred** |
| E3-mixed multi-kind C matrix | **residual** |
| Full oracle E4 dual | **residual** (TEST-008) |
| Full TEST-018 oracle fork corpus | **residual** |
| Full SEC-002 continuous fuzz | **residual** |
| Public P1–P4 SLOs | **waived** until certified gates green |
| Multi-OS CI full platform matrix | residual unless closed on Track A with evidence |
| Live Perl/XS collection default v6 | **not** claimed |

---

## Dual-equality (E1–E5) under R2-stable

| Class | R2-stable status |
|-------|------------------|
| E1 v5 surfaces | ready (R0/R1-preview stack) |
| E2 encode↔decode | ready + golden vectors |
| E3 C writer ↔ Rust decode | **ready (EVENT)**; E3-mixed residual |
| E4 v5↔v6 semantic | E4-v0 + E4 product CLI **ready** on dual-sink scaled pairs; full oracle residual |
| E5 CLI product path | **ready** opt-in + convert/merge/repack/salvage; collection default remains v5 |

Authoritative checklist: [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md).

---

## Explicit non-claims (binding)

Do **not** advertise under this cut:

1. **R3** `engine=auto` product default **runtime** flip (Track D; policy ADR when present still gated).
2. **R4** collection / format default **runtime** flip to v6 (Track E) — **PR-E02** lands [ADR-0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) **policy** + [`docs/R4_DEFAULT_FLIP.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md); **flip not executed**; incomplete field evidence → do not flip.
3. **CLI v6 collection default** — capability must keep `collection_default: v5` until a future flip PR completes the R4 checklist.
4. **Lossy convert** (`--allow-lossy`) or packing/string-dict v6 convert targets.
5. **Full `nytprofmerge`** aggregate-sum / option parity (stream-concat MVP only).
6. **COL-008** batched Rust writer as baseline.
7. **E3-mixed** multi-kind product C fixture matrix complete.
8. **Full oracle E4** dual pairs (TEST-003/TEST-008).
9. **Full TEST-018** oracle forkdepth/addpid/merge corpus (COL-015 MVP only).
10. **Full SEC-002** continuous fuzz / **SEC-012** independent security release review.
11. **Public performance SLOs** / certified P1–P4 numbers (methodology + light harness only; see [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md)).
12. **CPAN upload** readiness.
13. Mid-deflate compressor **continue-in-child** matching 6.15 oracle (child reinit = clean stream).
14. That **this packaging PR** implemented convert/COL-015/security — those are PR-C01..C04; this cut **integrates + honesty-promotes**.

---

## Upgrade / operator notes

| Audience | Guidance |
|----------|----------|
| Operators upgrading from R2-preview | Offline tools now claim convert/merge/repack/salvage when green. Collection default remains v5. Keep `./scripts/ci/offline_gate.sh` as primary gate. |
| Convert users | Prefer integer-tick dual-sink / representable streams. Fractional wall-clock PID times and non-zero `TIME_BLOCK.sub_line` refuse v5→v6 (strict). |
| Recovery | Use `salvage` for truncated profiles; never expect verify/report to auto-salvage incomplete streams. |
| Embedders / collectors | Production v6 writer backend remains **C** (ADR-0007). COL-015 protocol available for fork; product option wiring residual. |
| Release engineers | This is an **R2-stable honesty cut** on the integrated Phase C stack. R3/R4 **runtime** default flips remain separate; R4 **policy** is ADR-0008 (flip gated on accepted field **Promote**). |
| Perf claimants | Do **not** publish “% faster” / SLOs from light harness alone. |

### Quick commands

```bash
./scripts/ci/offline_gate.sh
cargo run -q -p nytprof-cli -- capability
cargo run -q -p nytprof-cli -- convert --to=v6 fixtures/e4/dual-sink/m4_v5.nytprof -o /tmp/m4.v6
cargo run -q -p nytprof-cli -- merge --to=v6 -o /tmp/m.v6 \
  fixtures/e4/dual-sink/m4_v5.nytprof fixtures/e4/dual-sink/m4_v6.nytprof
make -C collector test   # includes test_fork_pid when CC present
bash tools/oracle/selftest_security_fuzz.sh
STEPS=size,collector_micro bash tools/bench/light_bench.sh   # claim: none
```

---

## Evidence map

| Item | Path |
|------|------|
| This cut board row | `R2-STABLE-READINESS-CUT` in [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md) |
| Prior R2-preview notes | [`docs/RELEASE_NOTES_R2_PREVIEW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_PREVIEW.md) |
| Dual-equality | [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md) |
| Residual matrix | [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) |
| Security package | [`docs/contracts/SECURITY_FUZZ_HARDENING_PACKAGE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/SECURITY_FUZZ_HARDENING_PACKAGE_v0.md) |
| Bench methodology | [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md) |
| Runbook | [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) |
| Offline gate | [`scripts/ci/offline_gate.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/offline_gate.sh) |

---

## Track C PR index (path to this cut)

| PR | Role | Status in this cut |
|----|------|--------------------|
| PR-B13 | R2-preview packaging + COL-009 | done (base) |
| PR-C01 | Strict v5↔v6 convert | done |
| PR-C02 | Merge / repack / salvage | done |
| PR-C02b | COL-015 fork/PID MVP | done |
| PR-C03 | Security/fuzz hardening package | done |
| PR-C04 | P1/P2 methodology (no public claims) | done |
| **PR-C05** | **This R2-stable packaging + honesty promotion** | **done** |

R2s DoD alignment (design):

| # | Criterion | This cut |
|---|-----------|----------|
| R2s-1 | Wire freeze ADR + vectors | yes (B11) |
| R2s-2 | Convert/verify/inspect; merge/salvage as scoped | yes (C01/C02) |
| R2s-3 | COL-015 fork/PID suite green | yes (MVP stress; oracle residual) |
| R2s-4 | Security/fuzz + P1/P2 + platform as advertised | security offline package + P1/P2 methodology; public perf waived; platform residual honesty |
| R2s-5 | E3-mixed closed **or** residual documented | residual documented |

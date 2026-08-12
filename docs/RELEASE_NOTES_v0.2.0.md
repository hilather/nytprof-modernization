# Release notes — v0.2.0 (R2-stable packaging cut)

**Tag:** `v0.2.0`  
**Date:** 2026-08-12  
**PLAN_ID:** `8c9b1a63`  
**Horizon:** charter **R2-stable** integrated stack (not R3/R4 runtime default flips)

Normative longer form: [`docs/RELEASE_NOTES_R2_STABLE.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.0/docs/RELEASE_NOTES_R2_STABLE.md).

## Highlights

| Area | What ships |
|------|------------|
| **R1 residuals** | Shared HTML CSS + exclusive-sub index + optional flame; residual-policy ADR; multi-OS CI MVP; packaging depth; FFI cdylib MVP; product Data/ReadStream over binary via native CLI |
| **Collector / COL-007** | C overlay tree; semantic sink; lifecycle/seq/fake-clock; batch; real v5 wire; absolute v6 + codecs/multi-chunk/CRC + packing/FOOTER dict; **E3-EVENT with C fixtures** (board COL-007 done); COL-009 C baseline ADR |
| **Dual equality** | Dual-sink test/dev-only (OQ-4); v6→`ProfileModel` dual-dispatch; E4-v0 model + E4 product CLI smoke; wire freeze ADR + golden vectors |
| **CLI E5** | report/html/csv/folded/callgrind/dump/verify on v6 via magic detect; **`collection_default: v5`** (no R4 flip) |
| **Tooling** | Strict convert; merge/repack/salvage; COL-015 fork MVP; security/fuzz offline package; P1/P2 methodology (public claims waived) |
| **Governance** | R2-preview + R2-stable release notes; R3/R4 field-window packs + default ADRs (policy only, flip gated); R5 retirement governance (never automatic) |

## Capability honesty (this tag)

```text
collection_default: v5
v6_decode / v6_report: yes
convert / merge / repack / salvage: yes
```

## Explicit non-claims

- No R3 engine default flip and no R4 format default flip (ADRs + field packs only)
- No public P1–P4 performance certification
- No full oracle dual-equality (TEST-008 residual); E3-mixed residual
- COL-008 batched Rust writer remains deferred
- R5 does not retire any component automatically

## CI

GitHub Actions: **CI matrix (BUILD-006 MVP)** — Linux rust-smoke + offline_gate on `linux-x86_64` and `macos-arm64`. Agents must watch CI green after this tag (see `AGENTS.md` § Releases and CI watch).

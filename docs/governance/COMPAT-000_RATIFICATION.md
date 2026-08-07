# COMPAT-000 — Compatibility Contract Ratification

**Status:** accepted / binding  
**Task:** COMPAT-000  
**Date:** 2026-08-07  
**Owner sign-off:** project maintainer / architecture (repository bootstrap)

## Decision

The document [`docs/plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](../plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md) is the **binding compatibility contract** for this modernization program.

An implementation that violates that contract must not merge without:

1. an **accepted ADR** describing the exception, migration, and user-visible impact; and  
2. explicit project-owner sign-off recorded in the ADR.

The default disposition for contract violations is **rejection**.

## What is ratified

- Precision and event fidelity (full ordered stream; no sampling replacement).
- File-format compatibility (v5 read/write; v6 read/write when introduced; conversion rules).
- Same-run dual output as a regression oracle (not a production performance mode).
- Perl API, CLI, and report compatibility obligations.
- Platform/build: Rust optional; legacy path installable without Cargo during initial releases.
- Regression matrix M1–M10 as release-candidate requirements for the scopes they cover.

## What is not frozen by this ratification

- Specific v6 wire constants, codecs, or magic bytes (open ADR-Q* items).
- Default engine or default format flips (R3/R4; separate ADRs).
- Provisional inventory open items (use legacy fallback until contracted).

## Related

- Program charter: [`docs/PROGRAM_CHARTER.md`](../PROGRAM_CHARTER.md)  
- ADR process: [`docs/governance/ARCH-008_ADR_PROCESS.md`](ARCH-008_ADR_PROCESS.md)  
- Phase-0 exit: [`docs/PHASE0_EXIT_CRITERIA.md`](../PHASE0_EXIT_CRITERIA.md)  

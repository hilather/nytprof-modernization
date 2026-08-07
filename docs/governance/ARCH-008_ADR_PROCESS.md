# ARCH-008 — ADR Governance Process

**Status:** done (process established)  
**Task:** ARCH-008  
**Date:** 2026-08-07

## Decision

Architecture Decision Records are mandatory for changes that affect stable semantics, wire bytes, platform support, packaging, compatibility, security, or defaults. Agents must not settle these inside implementation patches alone.

## Lifecycle

1. **Open** — question recorded in `docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md` or as `docs/adrs/NNNN-title.md` with status `open`.
2. **Proposed** — recommendation + evidence ready for review.
3. **Accepted** — binding; normative specs/tests updated.
4. **Superseded** — replaced by a later ADR.
5. **Deferred** — not required for the current release level.

## Location and numbering

| Path | Role |
|------|------|
| `docs/adrs/` | Numbered ADRs (`0001-...md`, …) |
| `docs/adrs/README.md` | Index of ADR statuses |
| `docs/plan/templates/ADR_TEMPLATE.md` | Template source |
| `docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md` | Queue of blocking questions |

## Required content

Each ADR must include: context/evidence, options, decision, consequences, compatibility/versioning, required spec/test updates, revisit triggers.

## Merge-blocking policy

A change is **blocked** if it:

- freezes or renumbers stable wire/format IDs without an accepted ADR;
- changes timing attribution or discount semantics without oracle evidence + ADR when conflicting with the freeze notes;
- alters support tiers, MSRV, or default engine/format without ADR;
- “fixes” golden fixtures to match candidate behavior without semantic ADR.

## Reviewers

- Correctness/compatibility: compatibility lead  
- Format/wire: format architect  
- Packaging: build/release lead  
- Performance claims: performance lead (after correctness P0)

## Acceptance for ARCH-008

- [x] Template available  
- [x] Log directory and index exist  
- [x] Merge-blocking policy written  
- [x] Linked from program charter  

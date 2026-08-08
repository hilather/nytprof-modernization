# Agent notes (light institutional memory)

**Status:** binding scratch for agents — **not** an ADR, charter, or readiness claim  
**Audience:** coding agents and human implementers  
**Parent duty:** [`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/main/AGENTS.md) §6 “Failed attempts and language semantics (save automatically)”

## Purpose

**Automatically** capture **negative knowledge** so later sessions do not re-learn the same failure or misread the same language rule:

| Ledger | File | Use when |
|--------|------|----------|
| Failed attempts | [`failed-attempts.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/failed-attempts.md) | Perf tries that lost, API rewrites, format experiments, packaging approaches that were **tried and abandoned** (or failed gates) |
| Language semantics | [`language-semantics.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/language-semantics.md) | **Perl** or **Rust** behavior an agent **misunderstood** and then corrected (or still open) |

### Light vs drill-down

| Layer | Where | Context cost | When |
|-------|-------|--------------|------|
| **Light** (default) | table row in a ledger above | cheap — load always | every abandoned attempt / corrected misunderstanding |
| **Detail** (optional) | [`details/<slug>.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/details/) | load only when following a link | benches, stack traces, multi-step postmortems the light row cannot carry |

## Rules of use

1. **Write automatically on failure / correction.** Do not wait for a separate task. If you spent meaningful effort and reverse the approach (including a perf try that did not win), or you correct a Perl/Rust misunderstanding, add a **light** entry in the same change set (or immediately after aborting).
2. **Do not invent architecture.** Notes record *what was tried* and *why it failed or was wrong* — they do not override fixtures, ADRs, or the charter.
3. **Prefer facts over narrative.** Symptoms, commands, commit/branch, and the one-line “do not retry without …” are enough for the light ledger.
4. **Light first; drill down only when needed.** Default = one table row. Expand `details/<slug>.md` only when a later agent would need more than that one line.
5. **No certification claims.** Failed perf experiments are engineering memory, not BENCH-* certification.
6. **Skim before big rewrites.** Check both light ledgers before re-proposing a known-failed approach without new evidence.

## Light entry template (failed attempts)

```markdown
| YYYY-MM-DD | short-slug | area | one-line what was tried | why it failed / residual | details link or — |
```

## Light entry template (language semantics)

```markdown
| YYYY-MM-DD | perl|rust | short topic | wrong assumption | correct rule / pointer | open? | details or — |
```

## Detail file template

When the light row is not enough, create `docs/agent-notes/details/<slug>.md`:

```markdown
# <slug>

**Date:** YYYY-MM-DD  
**Area:** perf | format | perl | rust | packaging | report | …  
**Related:** board IDs, PRs, commits, files  

## Attempt / misunderstanding
…

## Evidence (commands, numbers, failing tests)
…

## Outcome
abandoned | deferred | corrected understanding | partially reused

## Do not retry without
- …
```

## Explicit non-goals

- Not a full lab notebook of every agent turn  
- Not a substitute for residual matrix / board status  
- Not permission to re-open deferred COL-007 / wire freeze without a real goal

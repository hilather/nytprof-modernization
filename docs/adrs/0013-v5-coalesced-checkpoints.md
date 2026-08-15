# ADR-0013 — In-memory v5 coalesced checkpoints (fid,line + call edges)

- **Status:** **proposed** — A13 only. `accepted` only after project-owner sign-off, **before** C2, **not** inside C1
- **Date:** 2026-08-15
- **Owners/approvers:** project owner (plan 01 sign-off; named in ADR-Q027).
  Compatibility lead + collector owner **review**; they cannot self-accept
  a charter exception.
- **Related ADR-Q:** [ADR-Q027](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md)
  (not R4 / ADR-Q025)
- **Related tasks/risks/gates:** charter #2–#4; plan 01 A2/A4; COMPAT-001
  multiplicity; COL-005/006 wrap; TEST attach smokes; **not** COL-007;
  **not** ADR-0008 flip
- **Decision scope/version:** product NYTPROF `aggregate=1` writer
  representation inside **NYTProf 5**. Does **not** change
  `collection_default`. Does **not** replace v5 `z` with zstd.

## Context

Default NYTProfM collection writes one TIME_LINE/TIME_BLOCK per last-site
interval. Operator files stay megabytes (same order as 6.15). Reports
already sum ticks and SUB_CALLERS. Operators want a size win **without**
flipping R4 to v6.

This is **not** a routine COL “new representation.” It **violates**:

- Charter #2 no dropped statement/call events
- Charter #3 no pre-aggregation that replaces the ordered event stream
- Charter #4 preserve counts
- Plan 01 A2 exact multiplicity on decode
- Plan 01 A4 “replacing ordered events with only line/subroutine
  aggregates” is **out of scope**
- Plan 05 §1–§2 (unconditional exactness; permitted reductions must
  **not** remove information)

Plan 01 header: must not merge without an approved ADR **and explicit
project-owner sign-off**. Default disposition is **rejection**.
ARCH-008: agents must not settle this inside implementation patches.

## Evidence

- Charter non-negotiables #2–#4:
  https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md
- Plan 01 A2/A4 + owner-sign-off header:
  https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md
- Plan 05 §1–§2 (no “unless ADR” escape):
  https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/05_COLLECTOR_AND_C_XS_TASKS.md
- A4 `LineTotal.calls += 1` per TIME_LINE/TIME_BLOCK:
  `crates/nytprof-model/src/lib.rs` ~255–268
- di01 bar is **TIME_BLOCK event occupancy 780**, not ticks:
  `scripts/packaging/di01_blocks_780_smoke.sh`
- `%check` 15/3/15 is **per-tag** (`scan_profile` increments edge once
  per `c`, skipping the count field): `t/installed_attach.t`
- Product last-site still emits one `+`/`*` per closed interval:
  `collector/xs/NYTProf.xs` `product_emit_last_site_elapsed`
- Field (not-in-repo) 25s scanner files remain megabytes of per-interval
  tags; not a BENCH gate

## Decision

1. NYTPROF `aggregate=0` (default): per-interval v5 events (today).
   Charter still holds on the default path.
2. NYTPROF `aggregate=1` (**exception**, owner-accepted): in-memory maps
   `(fid,line[,block]) → {ticks,hits}` and
   `(fid,line,caller,called) → {count,incl,excl,reci,max_rec_depth}`
   with fail-closed caps of 250_000 each. Checkpoints and process end
   emit **coalesced** v5 TIME_LINE / TIME_BLOCK / SUB_CALLERS **dirty
   deltas**. After emit, **zero** those accumulators; keep the slot.
   SUB_ENTRY / SUB_RETURN stay live.
3. Checkpoint container is the **same** `nytprof.out` (complete v5 tags;
   zlib only as item-2 sealed `z` + Z_FINISH copy). Not a sidecar. Not v6.
4. `collection_default` remains `v5`. format=v6 on D1-B remains fail-closed.
5. Opt-in is **not** a substitute for this ADR + owner sign-off.

## Exactness and compatibility consequences

- TIME_LINE / TIME_BLOCK **event counts** drop; **tick sums** per location
  remain. A4 `LineTotal.calls` becomes window occupancy, not statement hits.
- SUB_CALLERS **count/incl/excl** remain; per-return `c` multiplicity drops.
- dual_path compare_jsonl vs 6.15 is **not** required under `aggregate=1`.
- Unmodified 6.15 tools **read** the file; they will show lower statement
  call counts and correct seconds (within COMPAT-003).
- Golden per-hit fixtures (`t/installed_attach.t`, di01 780) stay on
  `aggregate=0` forever.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Treat as ordinary ADR under plan 05 “unless ADR” | That clause **does not exist**; plan 05 is unconditional |
| N TIME_LINE copies to keep hits | No size win |
| v6 TIME_LINE_RUN as product default | Conflicts with collection_default=v5 / R4 |
| Sidecar + zstd | Two files; old tools blind |
| Silent default-on coalescing | Breaks 15/3/15-style hit tests and dual_path |
| Accept ADR in the first implementation PR | Violates ARCH-008 and plan 01 |

## Implementation and testing requirements

- PR-A13: this file + ADR-Q027 + README index; **no code**; status
  `proposed`.
- Separate sign-off: status `accepted` + named project owner.
- Live `perl -d:NYTProfM` tests: returns 15/3; **parse `c` count** for
  edge 15 (do not reuse `scan_profile`); tick sums match aggregate=0;
  TIME_LINE multiplicity strictly smaller; add 10 / emit / add 5 / emit
  → 10 then 5; cap overflow fails closed; kill-after-seal is not a torn
  zlib OK.
- Docs: runbook, ROCKY remaining, FIRST_SLICE_BOARD, BENCH_NOTES (no cert).

## Migration, rollout, and rollback

Opt-in `aggregate=1` after acceptance. Rollback: omit the option
(default 0). Files already produced remain valid v5.

## Revisit triggers

Need for accurate statement hit counts on the v5 wire; R4 v6 default;
any reader that double-counts dirty-window emits (should not, if
zero-after-emit is implemented); owner withdraws the exception.

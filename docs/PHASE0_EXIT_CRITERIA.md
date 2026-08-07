# Phase 0 — Exit Criteria (“Good Enough”)

**Related:** [`docs/plan/15_PHASES_DEPENDENCIES_AND_CRITICAL_PATH.md`](plan/15_PHASES_DEPENDENCIES_AND_CRITICAL_PATH.md)  
**Purpose:** Prevent infinite inventory completeness from blocking WP-02/WP-03. Phase 0 exits when the oracle and contracts are **trustworthy enough** for an independent v5 reader and comparator—not when every obscure option combination is fully prose-specified.

## Hard exit criteria (required)

| # | Criterion | Evidence artifact |
|---|-----------|-------------------|
| P0-1 | v6.15 oracle pinned (tag, commit, archive checksum) and rebuildable | `baseline/6.15/manifest.json` + `scripts/baseline/*` |
| P0-2 | Clean build of oracle does not load candidate modules | Manifest `module_path` / isolation check |
| P0-3 | Compatibility contract ratified | `docs/governance/COMPAT-000_RATIFICATION.md` |
| P0-4 | ADR process live; log location known | `docs/governance/ARCH-008_ADR_PROCESS.md`, `docs/adrs/` |
| P0-5 | Provisional v5 record inventory covers **default HTML path + calls + merge + incomplete + top options** | `baseline/inventories/v5-record-inventory.md` (+ json) |
| P0-6 | Timing/lifecycle freeze notes for statement attribution, discount, calls=0/1/2, fork/finalization | `baseline/inventories/timing-lifecycle-notes.md` |
| P0-7 | Open items for incomplete inventory explicitly listed (not silently assumed) | Open-item sections in inventory docs |
| P0-8 | Differential/fixture capture path exists (scripts + fixture dir layout) | `tools/oracle/`, `fixtures/` |
| P0-9 | Packaging spike priority recorded (legacy-only + optional Cargo notes) | `docs/PACKAGING_SPIKE.md` |
| P0-10 | First-slice effort board ordered and current | `docs/FIRST_SLICE_BOARD.md` |

## “Good enough” inventory cutoff (BASE-002/003)

Inventories may exit Phase 0 as **provisional freeze** when:

1. **Every tag/constant** in `FileHandle.h` / writer paths used by the oracle has a disposition: `mapped` | `opaque-extension` | `unreachable-with-evidence` | `open`.
2. **Covered end-to-end first:**
   - default statement + block profiling;
   - `calls=0`, `calls=1`, `calls=2` (basic);
   - `savesrc` default and off;
   - fork/`addpid` basic;
   - incomplete/truncated profile recovery as legacy does;
   - `nytprofhtml` default report path inputs;
   - `nytprofmerge` / `nytprofcalls` stream consumers at callback level.
3. **Rare combinations** may remain `open` with “fallback to legacy engine until contracted” — not guessed into wire IDs.
4. A time-box review (see [`docs/INVENTORY_TIMEBOXES.md`](INVENTORY_TIMEBOXES.md)) has closed or deferred each open item.

## Forbidden until Phase 0 exit

- Freezing stable v6 numeric IDs / production magic (experimental drafts OK with experimental version bits).
- Major collector hook refactor beyond inventory instrumentation.
- Updating golden fixtures to match candidate (not oracle) behavior.
- Declaring R1 complete.

## Phase 0 does **not** require

- Complete BASE-004–008 (API/CLI full freeze, full corpus, RSS study) — track on the first-slice board.
- Full COMPAT-001–004 freeze beyond provisional event taxonomy.
- Full TEST-001–004 production harness — skeleton + golden capture path is enough to exit Phase 0 foundations for this goal.
- Any COL-008 work.

## Sign-off

Phase 0 exit is recorded by updating `docs/FIRST_SLICE_BOARD.md` status and linking evidence paths. No agent may mark COMPAT event taxonomy `accepted` for wire freeze without an ADR.

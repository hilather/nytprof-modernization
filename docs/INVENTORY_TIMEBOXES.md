# Inventory Time-Boxes

**Purpose:** BASE-002 through BASE-005 are XL tasks that block most of the graph. Time-boxes force provisional freezes so reader and comparator work can start.

## Rules

1. Each inventory has a **hard review gate** after the listed calendar/work budget.
2. At the gate: freeze what is evidence-backed; list residual **open items** with owner and disposition (`legacy-fallback`, `needs-fixture`, `needs-ADR`).
3. Do not invent semantics for open items. Prefer “unsupported in native path → legacy engine” over guessing.
4. Re-open a freeze only with new oracle evidence or an ADR.

## Budgets (from BASE-001 pin complete)

| Task | Scope | Time-box | Gate artifact |
|------|--------|----------|---------------|
| BASE-002 | v5 event protocol / tags | **5 working days** | `baseline/inventories/v5-record-inventory.md` provisional freeze section |
| BASE-003 | Timing, call, numeric, lifecycle | **5 working days** (overlaps BASE-002) | `baseline/inventories/timing-lifecycle-notes.md` |
| BASE-004 | Perl API / object model | **5 working days** after BASE-001 | Inventory + open items (later board) |
| BASE-005 | CLI / report contracts | **5 working days** after BASE-001 | Inventory + open items (later board) |
| COMPAT-001 | Canonical logical events | **3 working days** after BASE-002/003 provisional | Schema draft + open ADR-Q001 notes |

## Minimum coverage before provisional freeze (BASE-002/003)

See Phase-0 exit criteria “good enough” list. At minimum, inventories must not block:

- Rust v5 streaming decoder for common profiles (default options);
- Canonical event dump + comparator seeded mutations;
- One native report path on v5 input.

## Escalation

If a time-box expires with >20% of tags still `open` without disposition, stop expanding scope: run a review, assign dispositions, and proceed with native work limited to `mapped` events.

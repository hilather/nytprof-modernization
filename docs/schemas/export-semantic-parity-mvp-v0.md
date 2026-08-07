# Export semantic parity MVP (v0) — Callgrind + folded stacks

**Status:** first-slice semantic checklist for shipped **folded** and **callgrind** exports  
**Board ID:** `EXPORT-SEMANTIC-PARITY`  
**Related:** [export-formats-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/export-formats-mvp-v0.md) (format shape), [report-semantic-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/report-semantic-parity-mvp-v0.md) (default-calls1 leaf/mid/edge HTML), [aggregate-comparison-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/aggregate-comparison-v0.md) (A5/A7)  
**Not:** full legacy `nytprofcg` / `nytprofcalls` dialect parity / Valgrind tool acceptance / multi-file merge

## Profile under test

| Field | Value |
|-------|-------|
| Fixture | `fixtures/v5/default-calls1/nytprof.out` |
| Workload | `fixtures/v5/default-calls1/workload.pl` (`mid` ×3 → `leaf` ×5) |
| Oracle aggregates | `fixtures/v5/default-calls1/aggregates.oracle.json` |
| Aggregate contract | [aggregate-comparison-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/aggregate-comparison-v0.md) |

Golden counts (oracle A5 / A7):

| Check | Source | Expected |
|-------|--------|----------|
| `main::leaf` returns | A5 `sub_return_totals` | **15** |
| `main::mid` returns | A5 `sub_return_totals` | **3** |
| `main::mid` → `main::leaf` call count | A7 `call_edges` | **15** |
| `main::RUNTIME` → `main::mid` call count | A7 `call_edges` | **3** (when edge present) |

These numbers are fixed by the committed fixture and `aggregates.oracle.json`. Tests **must** load the real profile via `ProfileModel::from_path` (or the shipped CLI path) and assert against real model APIs / rendered exports — not invent unrelated constants.

## Semantic checklist (required)

1. **Model API** — after `ProfileModel::from_path` on `fixtures/v5/default-calls1/nytprof.out`:
   - `sub_total("main::leaf").returns == 15`
   - `sub_total("main::mid").returns == 3`
   - `call_edge("main::mid", "main::leaf").count == 15`
   - when present: `call_edge("main::RUNTIME", "main::mid").count == 3`
2. **Folded stacks** — `render_folded_stacks` / `nytprof-cli folded` contains:
   - `main::mid;main::leaf 15`
   - `main::RUNTIME;main::mid 3` (when the RUNTIME→mid edge exists on the model)
   - leaf / mid names present via those edges (or as endpoints)
3. **Callgrind-style** — `render_callgrind` / `nytprof-cli callgrind` (or `cg`) contains:
   - header markers (`# callgrind format`, `positions: line`, events line)
   - presence of `main::leaf` and `main::mid` (`fn=` and/or `cfn=`)
   - under mid (or anywhere as contracted evidence): `cfn=main::leaf` with `calls=15` (or `calls=15 0`)
   - mid returns relationship **3** where format allows: `cfn=main::mid` with `calls=3` (typically under `fn=main::RUNTIME`)
4. **CLI smoke** — `nytprof-cli folded` and `callgrind`/`cg` on the fixture (cargo or prefix), each run **twice** for stability; both runs must match the semantic patterns (byte-identical stdout preferred).

## Native side

```sh
cargo run -q -p nytprof-cli -- folded fixtures/v5/default-calls1/nytprof.out
cargo run -q -p nytprof-cli -- callgrind fixtures/v5/default-calls1/nytprof.out
cargo run -q -p nytprof-cli -- cg fixtures/v5/default-calls1/nytprof.out   # alias

# Prefix install (if present)
prefix/bin/nytprof-cli folded fixtures/v5/default-calls1/nytprof.out
prefix/bin/nytprof-cli callgrind fixtures/v5/default-calls1/nytprof.out
```

Library entry points (shipped report path):

- `nytprof_report::render_folded_stacks`
- `nytprof_report::render_callgrind`
- `ProfileModel::from_path` / `sub_total` / `call_edge`

Contracted folded evidence:

```text
main::mid;main::leaf 15
main::RUNTIME;main::mid 3
```

Contracted callgrind evidence (layout may simplify; counts are exact):

```text
fn=main::mid
...
cfn=main::leaf
calls=15 0
...
fn=main::RUNTIME
...
cfn=main::mid
calls=3 0
```

Tick / cost lines may use exclusive ticks or fall back to counts; **tick equality is not required** for this checklist (see COMPAT-003).

## Format shape (non-semantic)

Wire format / CLI surface for exports is specified in  
[export-formats-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/export-formats-mvp-v0.md).  
This document is the **semantic** gate: real-model counts and format-appropriate string evidence.

## How ticks / time are treated

| Field class | Parity rule for this MVP |
|-------------|---------------------------|
| **Counts** (returns, call-edge count, `calls=N`, folded trailing count) | **Exact** — must match oracle aggregates / model |
| **Time ticks** (Callgrind self/edge cost lines) | **Not required** for this checklist. Compare only under **COMPAT-003** when that contract is frozen |

Do not fail semantic parity solely because cost/tick strings differ in formatting or floating conversion.

## Explicit non-requirements

- Full legacy `nytprofcg` / `nytprofcalls` dialect parity
- Valgrind Callgrind tool acceptance / KCachegrind UI fidelity
- Byte-identical export to oracle tools
- Tick/time equality (see COMPAT-003)
- Multi-file merge / flamegraph.pl integration
- COL-008 (batched Rust writer) — out of scope

## Verification

| Gate | Command |
|------|---------|
| Schema + checklist (this file) | Read / review |
| Rust model + export render | `cargo test -p nytprof-report export_semantic_parity_default_calls1` |
| Operator smoke (native CLI ×2 per format) | `bash tools/oracle/export_semantic_parity.sh` |

Evidence paths land on the first-slice board as `EXPORT-SEMANTIC-PARITY`.

## Relation to existing tests

Existing coverage that this board consolidates under a clearly named gate:

| Test | Crate | Overlap |
|------|-------|---------|
| `folded_stacks_default_calls1_real_render` | nytprof-report | mid→leaf 15, RUNTIME→mid 3, sort |
| `callgrind_default_calls1_real_render` | nytprof-report | leaf/mid presence, `calls=15` |

The dedicated test `export_semantic_parity_default_calls1` is the named gate for this board: real `from_path` + exact model leaf **15** / mid **3** / mid→leaf **15** + both export render paths with format-appropriate evidence.

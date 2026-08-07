# Aggregate comparison contract (v0)

**Status:** provisional — required totals for first-slice model/report MVP  
**Fixtures in scope:** `fixtures/v5/default-calls1`, `fixtures/v5/default-calls2`, `fixtures/v5/blocks-calls1`  
**Not:** full Data.pm eval/recursion parity matrix

## Required comparable totals

Built by replaying the ordered logical event stream (v5 decoder or oracle JSONL dump).

| ID | Total | Definition |
|----|-------|------------|
| A1 | `time_line_events` | Count of `TIME_LINE` events in the stream |
| A2 | `time_block_events` | Count of `TIME_BLOCK` events |
| A3 | `discount_events` | Count of `DISCOUNT` events |
| A4 | `line_totals` | Map `(fid, line) → { calls: N, ticks: sum }` from **`TIME_LINE` and `TIME_BLOCK`** (both contribute to the statement `line` field; ticks/calls summed). Profiles with only `blocks=1` therefore have non-empty A4. |
| A4b | `block_line_totals` | Map `(fid, block_line) → { calls: N, ticks: sum }` from **`TIME_BLOCK` only** using the event's `block_line` argument (4th field after tag in ReadStream: ticks, fid, line, block_line, sub_line). Empty when no TIME_BLOCK. |
| A5 | `sub_return_totals` | Map `subname → { returns: N, incl_ticks: sum, excl_ticks: sum }` from `SUB_RETURN` (`incl`/`excl` args as f64 sums) |
| A6 | `workload_subs` | Subset of A5 for names ending with `::leaf`, `::mid`, and containing `main::leaf` / `main::mid` / exact `main::leaf` etc. as present in fixtures |
| A7 | `call_edges` | Map `(caller, called) → { count: N, incl: sum, excl: sum, reci: sum, max_rec_depth: max, sites: optional }` from **`SUB_CALLERS`** args order `[fid, line, count, incl, excl, reci, rec_depth, called, caller]` — key is `(caller, called)` strings; **sum** count/incl/excl/reci across sites; max of rec_depth |
| A8 | `source_lines` | Map `(fid, line) → source text` from **`SRC_LINE`** (last write wins if duplicate); at minimum retain whether text is present and the text for workload fid lines |
| A9 | `sub_defs` | Map `subname → { fid, first_line, last_line }` from **`SUB_INFO`** args order `[fid, first_line, last_line, name]` (ReadStream callback order). Last write wins if duplicate names. Workload fixtures include `main::leaf` / `main::mid`. |

## Oracle baseline source

Baselines are **generated** from committed oracle `readstream.jsonl` dumps (already differentially equal to Rust dumps after normalize):

```sh
python3 tools/oracle/aggregate_from_jsonl.py fixtures/v5/default-calls1/readstream.jsonl \
  > fixtures/v5/default-calls1/aggregates.oracle.json
```

Rust model tests must:
1. `decode_path("fixtures/v5/.../nytprof.out")`
2. Aggregate with the same definitions
3. Compare to `aggregates.oracle.json` (or re-run the Python aggregator on the fly against the dump)

**Forbidden:** inventing expected tick constants not derived from oracle dump or live oracle tools.

## Report MVP content

The native report entry must print at least:

- profile path
- `time_line_events` (A1)
- for each of `main::leaf`, `main::mid` (if present): return count and exclusive ticks (or inclusive)
- optional: top N lines by calls for the primary workload fid

Values must match the model (and thus the oracle baseline) for those fields.

## CSV / tabular report (v0)

Native CSV entry must emit at least one of:

1. **Subroutines CSV** — header including `name,returns,incl,excl` with rows for `main::leaf` / `main::mid` matching A5.
2. **Call edges CSV** — header including `caller,called,count` with workload edges (e.g. `main::mid`→`main::leaf` count 15, `main::RUNTIME`→`main::mid` count 3) matching A7.

Command sketch: `cargo run -p nytprof-cli -- csv <profile.out>` (or `report --format=csv`).

## Explicit non-requirements (this version)

- Statement exclusive vs inclusive beyond raw TIME_LINE tick sums
- Block-level attribution when only TIME_LINE is present
- Full HTML/CSS/DOM / flame / Callgrind parity
- Byte-identical match to legacy `nytprofcsv` file layout (field values for contracted rows must match oracle-sourced aggregates)

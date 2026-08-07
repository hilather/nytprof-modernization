# Aggregate comparison contract (v0)

**Status:** provisional — required totals for first-slice model/report MVP  
**Fixtures in scope:** `fixtures/v5/default-calls1`, `fixtures/v5/default-calls2`  
**Not:** full Data.pm eval/recursion parity matrix

## Required comparable totals

Built by replaying the ordered logical event stream (v5 decoder or oracle JSONL dump).

| ID | Total | Definition |
|----|-------|------------|
| A1 | `time_line_events` | Count of `TIME_LINE` events in the stream |
| A2 | `time_block_events` | Count of `TIME_BLOCK` events |
| A3 | `discount_events` | Count of `DISCOUNT` events |
| A4 | `line_totals` | Map `(fid, line) → { calls: N, ticks: sum }` from `TIME_LINE` (+ `TIME_BLOCK` line field only for optional extension; MVP uses TIME_LINE only) |
| A5 | `sub_return_totals` | Map `subname → { returns: N, incl_ticks: sum, excl_ticks: sum }` from `SUB_RETURN` (`incl`/`excl` args as f64 sums) |
| A6 | `workload_subs` | Subset of A5 for names ending with `::leaf`, `::mid`, and containing `main::leaf` / `main::mid` / exact `main::leaf` etc. as present in fixtures |

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

## Explicit non-requirements (this version)

- Statement exclusive vs inclusive beyond raw TIME_LINE tick sums
- Block-level attribution when only TIME_LINE is present
- Full sub caller graph report
- HTML/CSS/DOM parity

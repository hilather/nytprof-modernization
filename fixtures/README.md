# Golden fixtures

Profiles and ReadStream dumps captured with the **pinned 6.15 oracle** (`tools/oracle/`).

## Layout

```text
fixtures/v5/<name>/
  nytprof.out              # binary profile
  readstream.jsonl         # oracle ReadStream dump (quiet)
  aggregates.oracle.json   # oracle-sourced aggregate totals (see below)
  workload.pl              # script that was profiled
  fixture.json             # metadata + checksums
  SHA256SUMS
  oracle-module-path.txt
```

## Capture

Requires oracle isolation (`tools/oracle/env.sh` via capture script):

```sh
./tools/oracle/capture_fixture.sh default-calls1 "trace=0:start=begin:calls=1"
```

## Normalize + compare (structural golden path)

Canonical schema: [`docs/schemas/canonical-event-dump-v0.md`](../docs/schemas/canonical-event-dump-v0.md).

`compare_jsonl.pl` is a **pure** tag+args comparator (seq ignored). It does **not** strip volatile fields. For golden / cross-run compare, **normalize first**:

```sh
# Pure Python — no oracle env required
python3 tools/oracle/normalize_jsonl.py fixtures/v5/default-calls1/readstream.jsonl \
  > /tmp/a.norm.jsonl
python3 tools/oracle/normalize_jsonl.py path/to/other.jsonl \
  > /tmp/b.norm.jsonl

# Perl comparator — needs a JSON::PP-capable perl (system perl is fine)
perl tools/oracle/compare_jsonl.pl /tmp/a.norm.jsonl /tmp/b.norm.jsonl
```

### Structural normalize rules (`--mode structural`, default)

| Field | Normalization |
|-------|----------------|
| `COMMENT` | args → `["<COMMENT>"]` |
| `ATTRIBUTE` `basetime` | value → `"<BASETIME>"` |
| `ATTRIBUTE` `application` | basename of path, or `"<APP>"` if empty |
| `NEW_FID` name (last arg) | basename when path-like |
| floating NVs | stable re-box for deterministic JSON |
| `seq` | renumbered from 0 (use `--preserve-seq` to keep) |
| `_END` | kept by default (`--drop-end` to strip) |

## Aggregate baselines (model / report MVP)

Contract: [`docs/schemas/aggregate-comparison-v0.md`](../docs/schemas/aggregate-comparison-v0.md).

`aggregates.oracle.json` is **generated** from the committed `readstream.jsonl` (not hand-typed). Totals include `time_line_events`, `time_block_events`, `discount_events`, `line_totals` (`"fid:line"` → calls/ticks from `TIME_LINE` only), `sub_return_totals` (from `SUB_RETURN`), and `workload_subs` (`main::leaf` / `main::mid`).

```sh
# Regenerate a baseline after recapturing a fixture
python3 tools/oracle/aggregate_from_jsonl.py fixtures/v5/default-calls1/readstream.jsonl \
  > fixtures/v5/default-calls1/aggregates.oracle.json

# Self-test: re-aggregate and json-equal against committed baselines (no Rust)
./tools/oracle/selftest_aggregates.sh
```

Rust model tests should decode `nytprof.out`, apply the same aggregate definitions, and compare to these files (or re-run the Python aggregator on a dump).

## Self-test harness

One command from the repo root (no Rust; no oracle env for the pure path):

```sh
./tools/oracle/selftest_harness.sh
# alias:
./tools/oracle/selftest_compare.sh
```

The harness checks, for `default-calls1` and `default-calls2` when present:

1. **Identity** — normalize fixture twice → compare PASS  
2. **Tag mutation** — `TIME_LINE` → `TIME_BLOCK` → compare FAIL  
3. **Ticks mutation** — change a `TIME_LINE` tick → compare FAIL  
4. **Volatiles** — change basetime / application path / COMMENT (and NEW_FID paths) → raw compare FAIL; after normalize → PASS  
5. **Aggregates** — runs `selftest_aggregates.sh` when that script is present  

Standalone aggregates only:

```sh
./tools/oracle/selftest_aggregates.sh
```

## Note on absolute paths and timestamps

`ATTRIBUTE application`, profile header `COMMENT`s, and some `NEW_FID` names embed absolute paths and run-specific timestamps. Always run `normalize_jsonl.py` before golden compare. Timing ticks and PID clocks remain **non-volatile for same-dump identity** but differ across recaptures (use controlled clocks / exact fixtures when comparing independent runs).

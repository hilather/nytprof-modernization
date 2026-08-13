# Merge `--aggregate-sum` MVP (v0)

**Board ID:** `TOOL-MERGE-AGGREGATE-SUM-MVP` / **L02**  
**Status:** **done (MVP)** — opt-in only; **stream-concat remains default**  
**Depends on:** [`merge-repack-salvage-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/merge-repack-salvage-mvp-v0.md)  
**Not:** full `nytprofmerge` option / eval-fold / overflow parity; silent default change; TEST-008 corpus

## CLI

```text
nytprof-cli merge --to=v5|v6 --aggregate-sum -o <output> <input> [<input>...]
```

Without `--aggregate-sum`, merge stays on the **stream-concat** path (later fids offset; `OK: merge` without the flag).

With `--aggregate-sum`, later-stream `NEW_FID` filenames remap onto the first stream so A4 line totals and A5/A7 sub/edge totals combine. Distinct filenames still get a fresh fid. Every input must fully decode (fail closed). `OK: merge` line names `--aggregate-sum`.

Oracle `fixtures/v5/default-calls1/nytprof.out` pair is encoded as `--to=v5` (strict convert of that fixture to v6 still needs `--allow-lossy`; E4/oracle equality must **not** pass `--allow-lossy`). Dual-sink integer-tick profiles may use `--to=v6`.

## Library API

```text
nytprof_model::{MergeMode, merge_bytes_with, merge_paths_with}
MergeMode::StreamConcat   // default
MergeMode::AggregateSum
```

`merge_bytes` / `merge_paths` remain stream-concat wrappers.

## Evidence

- `cargo test -p nytprof-model merge_tools::`
- `cargo test -p nytprof-cli --test merge_repack_salvage_cli`
- `./scripts/packaging/l02_aggregate_sum_merge_smoke.sh`

When `baseline/6.15/install/bin/nytprofmerge` is present, the smoke loads oracle merge output through shipped `report --json` (isolated oracle `PERL5LIB`; never `crates/`) and compares advertised leaf/mid/edge.

## Residuals

| Residual | Notes |
|----------|--------|
| Full `nytprofmerge` options | eval-fold, overflow, attribute/option fold, output naming |
| Packing / string-dict v6 out | Same as convert/merge MVP |
| Default merge = aggregate-sum | Not this slice; concat stays default |

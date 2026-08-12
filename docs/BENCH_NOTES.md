# Benchmark notes (noise study — not certification)

**Status:** light first-slice notes + runnable local harness  
**Related board rows:** `BENCH-001`, `LIGHT-BENCH` on [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md)  
**Full plan task:** [`docs/plan/11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/11_BENCHMARKING_AND_PERFORMANCE_GATES.md) (`BENCH-001` harness + gate ratification)  
**Agent duty:** keep engineering baselines **current vs 6.15 oracle and vs prior native versions** when decode/report/export paths change — see [`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/main/AGENTS.md) (benchmarks section). No public perf claims until certified.

## Explicit non-claims

- These notes are **not** a performance certification.
- **No public performance claims** until a reproducible plan-grade harness, noise bounds, and correctness gates exist (plan `BENCH-001`+).
- Numbers from the light harness or the sketches below are **local exploratory samples** only. Do not cite them in release notes, README badges, or marketing.

## Light harness (runnable today)

One-shot wall timings for offline CLI paths on committed fixtures:

```sh
bash tools/bench/light_bench.sh
# optional: also write the same report to a file
OUT=/tmp/nytprof-light-bench.txt bash tools/bench/light_bench.sh
```

What it does:

1. `cargo build -q -p nytprof-cli` once (untimed build cost separate from steps)
2. Per fixture (`fixtures/v5/default-calls1`, and `default-calls2` if present):
   - timed `cargo run -q -p nytprof-cli -- dump <nytprof.out> >/dev/null`
   - timed `cargo run -q -p nytprof-cli -- report <nytprof.out> >/dev/null`
   - timed `csv` **only if** the CLI exposes a `csv` subcommand; otherwise skipped
3. Prints wall seconds per step; exits 0 on success

Timing backend: GNU `/usr/bin/time -f '%e'` when available, else bash `TIMEFORMAT`, else `python3` `perf_counter` around a subprocess.

This harness is **smoke + noise study**, not BENCH gate certification. Prefer it over ad-hoc commands when collecting first samples.

## Collector statement fast path (engineering only — PR-B04)

`make -C collector test` runs a **light** `nytp_fast_bench_time_line` loop inside `test_batch_fast` (5000 TIME_LINE appends through a capacity-16 batch into a counting child). Sample host output is printed as a NOTE (e.g. `sizeof(event)=88`, wall ns).

| Claim | Status |
|-------|--------|
| No-alloc stmt append after batch create | Unit-tested (`heap_allocs` stable; `arena_bytes_copied==0` on pure TIME_LINE/BLOCK) |
| Exact order under batch stress | Unit-tested (cap 1..64 vs direct counting) |
| Public performance / median regression gates | **Not claimed** — needs BENCH-003 + certified harness |

Do **not** promote the light loop into release notes as a certified win.

## What to measure later

| Area | Metric sketch | Why |
|------|---------------|-----|
| Collection overhead | Wall time / CPU of a fixed workload with vs without `Devel::NYTProf` (oracle pin) | Separates collector cost from offline tools |
| Decode time | Time to stream-decode `nytprof.out` to events (dump path today) | Reader cost independent of model/report |
| Model build | Time + peak RSS to aggregate events into compact model | RUST-006/007/008 memory story |
| Report path | Time + peak RSS of `nytprof-cli report` (when present) | R1 opt-in report cost |
| Oracle compare baseline | Same metrics for `nytprofhtml` / Perl load path on same fixture | Differential only after semantic equality |

Preferred fixtures for early noise work: `fixtures/v5/default-calls1`, `fixtures/v5/default-calls2` (small, committed).

## Command sketches (manual / deeper)

Use `/usr/bin/time -v` (GNU time) for elapsed + max RSS. Repeat ≥5 times on a quiet machine; record host, `rustc --version`, commit, and fixture path. Do **not** average away outliers without writing them down. For a quick combined dump/report pass, prefer `tools/bench/light_bench.sh` first.

### Decode / dump (available today)

```sh
/usr/bin/time -v cargo run -q -p nytprof-cli --release -- dump \
  fixtures/v5/default-calls1/nytprof.out > /tmp/nytprof.dump.jsonl
```

### Report MVP (`report` / `summary` subcommand)

```sh
/usr/bin/time -v cargo run -q -p nytprof-cli --release -- report \
  fixtures/v5/default-calls1/nytprof.out
```

Same path without `--release` is fine for functional smoke; keep mode consistent when comparing runs. Evidence crate: `crates/nytprof-report` via `nytprof-cli`.

### CSV (when present)

```sh
/usr/bin/time -v cargo run -q -p nytprof-cli --release -- csv \
  fixtures/v5/default-calls1/nytprof.out
```

Skip if the CLI has no `csv` subcommand (light harness skips automatically).

### Oracle HTML path (collection/report baseline, not Rust)

```sh
# Requires baseline/6.15 pin; see tools/oracle/env.sh
# Example shape only — exact flags live in oracle scripts.
/usr/bin/time -v perl -I baseline/6.15/install/lib/perl5 \
  baseline/6.15/install/bin/nytprofhtml \
  --file fixtures/v5/default-calls1/nytprof.out \
  --out /tmp/nytprofhtml-out
```

### Collection overhead (later)

Run a fixed workload with and without `-d:NYTProf` under the pinned oracle `PERL5LIB`. Compare wall time only after confirming the profile is complete and comparable (`calls=`, `stmts=`, etc. fixed).

## Recording template (local only)

```text
date:
host:
commit:
rustc:
fixture:
command:
n_runs:
elapsed_s: [ ... ]
max_rss_kb: [ ... ]
notes: (load, thermal, background jobs)
```

Keep raw samples with the command line. Do not promote means or “% faster” without the plan harness and an ADR on thresholds.

## Out of scope for this note

- CI performance gates
- Codec / chunk size selection (`BENCH-007`)
- Real-application canaries (`BENCH-013`)
- Ratified noise bounds or public claim language

# Benchmark notes (noise study — not certification)

**Status:** R1-scoped **methodology + residual honesty** (light harness only)  
**Board rows:** `LIGHT-BENCH`, `BENCH-001`, `R1-P3P4-METHODOLOGY` on [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md)  
**Full plan:** [`docs/plan/11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/11_BENCHMARKING_AND_PERFORMANCE_GATES.md) (WP-13 / `BENCH-001`+)  
**Acceptance map:** [`docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md) (R1 P3/P4; project P1–P4)  
**Residual matrix:** [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md)  
**Agent duty:** keep engineering baselines **current vs 6.15 oracle and vs prior native versions** when decode/report/export paths change — see [`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/main/AGENTS.md). **No public perf claims until certified.**

---

## Explicit non-claims (binding)

| Claim | Status for R0 / R1-preview |
|-------|----------------------------|
| Public performance SLOs / “% faster” marketing | **Not claimed** |
| Plan-grade **P3** (native read/model) certification | **Not claimed** — methodology documented only |
| Plan-grade **P4** (native report) certification | **Not claimed** — methodology documented only |
| WP-13 / `BENCH-001` noise bounds + signed certificate | **Open** (full harness residual) |
| Collector **P1** / storage **P2** | **Out of R1-preview scope** (collector remains 6.15 oracle / v5) |

**R1-preview posture (PR-A09 default):** **waive public performance certification**. Ship a **light engineering harness** and this methodology so operators can take local samples without inventing heavy CI or false claims. If a future cut **publishes** numbers, it must close plan `BENCH-001`+ on advertised surfaces with correctness-paired evidence — not this light path alone.

These notes are **not** a performance certification. Numbers from the light harness or sample tables below are **local exploratory samples** only. Do **not** cite them in release notes, README badges, marketing, or as R1 DoD “P3/P4 results reported” under plan §16 without a certified package.

---

## R1-scoped P3 / P4 methodology (what would be certified)

Charter / acceptance language maps **R1** to controlled-host **P3/P4** for the **native v5 read/report** path (not collector P1/P2). This section freezes **what to measure** and **what evidence is required** before any claim. It does **not** record a certified result.

### Gate definitions (plan-facing)

| Gate | Plan meaning (summary) | R1-preview surface under test | Light harness proxy step |
|------|------------------------|-------------------------------|--------------------------|
| **P3** | Native **read/model** meets ratified wall/RSS targets without precision/feature loss | `nytprof-cli dump` (decode → JSONL) + `verify`/`inspect` (decode + compact model) | `dump`, `verify` |
| **P4** | Native **report** pipeline meets ratified wall/RSS (+ deterministic parallelism when applicable) | `report` / `summary`, `csv`, `html` (single + multi-file), optional exports | `report`, `csv`, `html` |
| **P1** | Collector overhead | Oracle 6.15 collection only today | *out of light harness* |
| **P2** | Profile storage size | v5 fixtures / future v6 | *out of light harness* |

Sources: [`16_ACCEPTANCE…`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md) § Runtime and memory; [`15_PHASES…`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/15_PHASES_DEPENDENCIES_AND_CRITICAL_PATH.md) controlled-host P3/P4 for native report milestone.

### Measurement principles (must hold for any future claim)

1. **Correctness first** — every timed sample set pairs with equality/parity that already gates the surface (default-calls1 leaf **15** / mid **3** / mid→leaf **15**; dump structural parity; report/csv/export semantic parity smokes). Reject samples that fail correctness.
2. **Same configuration** — same fixture, same CLI flags, same build profile (`debug` vs `release`), same host class. Do not disable features on one side of a compare.
3. **Separate phases** — build time is **not** step time; report build cost separately from dump/verify/report.
4. **Raw samples** — store all runs, not only means; keep outliers; record host + `rustc` + commit.
5. **Noise bound** — if measurement noise ≥ claimed gain, the claim is invalid (plan provisional rule).
6. **No public mean/% without BENCH-001** — light harness has no ratified noise model, affinity control, or CI dedicated runner.

### Fixture corpus (R1 offline)

| Fixture | Role |
|---------|------|
| `fixtures/v5/default-calls1` | Primary semantic + timing smoke (small) |
| `fixtures/v5/default-calls2` | Second default-path smoke if present |
| `fixtures/v5/blocks-calls1` | Optional blocks path (`FIXTURES=…`) |
| `fixtures/v5/calls2-default` | Optional `calls=2` path |

Preferred early noise work stays on committed small fixtures. Large synthetic / real-app canaries remain plan `BENCH-013` (not R1-preview certification).

### Metrics sketch (certified package later)

| Area | Metric | Notes |
|------|--------|-------|
| Decode (P3) | Wall + peak RSS of dump/verify | Reader cost vs model cost |
| Model (P3) | Peak RSS of compact model path | via verify / report --json |
| Report (P4) | Wall + peak RSS of text/html/csv | Equality-passing outputs only |
| Output size (P4) | HTML site / CSV bytes | No silent bloat |
| Oracle differential | Same metrics for `nytprofhtml` on same fixture | **After** semantic equality only |

### Promotion checklist (when claiming P3/P4)

- [ ] Plan-grade harness or equivalent (`BENCH-001`): repetitions, metadata, raw samples, statistics
- [ ] Thresholds ratified (ADR or board-approved contract) — light notes do **not** set SLOs
- [ ] Correctness hashes/parity green on every accepted sample
- [ ] Dedicated quiet runner notes (CPU affinity optional; no shared noisy CI as sole evidence)
- [ ] Residual matrix / release notes updated to **claim** only what was measured
- [ ] Public language limited to certified configurations

Until that checklist is green, residual matrix remains: **no performance certification claims**.

---

## Light harness (runnable today)

One-shot wall timings for offline CLI paths on committed fixtures:

```sh
bash tools/bench/light_bench.sh
# optional: also write the same report to a file
OUT=/tmp/nytprof-light-bench.txt bash tools/bench/light_bench.sh

# comparable samples (still not certification)
RELEASE=1 RUNS=3 bash tools/bench/light_bench.sh

# expand fixtures / steps
FIXTURES="fixtures/v5/default-calls1 fixtures/v5/blocks-calls1" \
  STEPS="dump,verify,report,csv,html" bash tools/bench/light_bench.sh
```

### What it does

1. `cargo build -q -p nytprof-cli` once (untimed relative to steps; optional `--release` when `RELEASE=1`)
2. Probes CLI for optional subcommands (`csv`, `html`, `verify`)
3. Per fixture (default: `fixtures/v5/default-calls1`, and `default-calls2` if present):
   - timed `dump` (P3 proxy)
   - timed `verify` when present (P3 model proxy)
   - timed `report` (P4 proxy)
   - timed `csv` when present
   - timed `html -o <temp>` when present (P4 HTML proxy)
4. Optional `RUNS=N` (default 1) repeats each step and prints each sample
5. Prints wall seconds; exits 0 on success

### Environment

| Variable | Default | Meaning |
|----------|---------|---------|
| `OUT` | unset | Also write report to this path |
| `FIXTURES` | `default-calls1` + `default-calls2` if present | Space-separated fixture dirs |
| `STEPS` | `dump,verify,report,csv,html` | Comma-separated steps; missing optional steps skip |
| `RUNS` | `1` | Timed repetitions per step |
| `RELEASE` | unset/`0` | `1` → `cargo build/run --release` |

Timing backend: GNU `/usr/bin/time -f '%e'` when available, else bash `TIMEFORMAT`, else `python3` `perf_counter` around a subprocess.

This harness is **smoke + noise study**, not BENCH gate certification. Prefer it over ad-hoc commands when collecting first samples. It is **not** wired into `offline_gate.sh` (no CI perf gate by design).

---

## Engineering sample log (local only — not a claim)

Illustrative single-host smoke after harness expansion. **Do not treat as baseline or SLO.**

```text
date: 2026-08-11
host: local Linux x86_64 (engineering laptop)
commit: (see harness banner at run time)
rustc: rustc 1.97.x
profile: debug (default light_bench; cargo run after build)
fixtures: fixtures/v5/default-calls1, fixtures/v5/default-calls2
command: bash tools/bench/light_bench.sh
n_runs: 1
note: wall seconds dominated by process startup on tiny fixtures; noise >> any meaningful delta
```

Re-run after decode/report changes and append a new raw block (or `OUT=` file). Compare directionally only; never publish “% faster” from this log.

---

## Command sketches (manual / deeper)

Use `/usr/bin/time -v` (GNU time) for elapsed + max RSS. Repeat ≥5 times on a quiet machine; record host, `rustc --version`, commit, and fixture path. Do **not** average away outliers without writing them down. For a quick combined pass, prefer `tools/bench/light_bench.sh` first.

### Decode / dump (P3 proxy — available today)

```sh
/usr/bin/time -v cargo run -q -p nytprof-cli --release -- dump \
  fixtures/v5/default-calls1/nytprof.out > /tmp/nytprof.dump.jsonl
```

### Verify / model (P3 proxy)

```sh
/usr/bin/time -v cargo run -q -p nytprof-cli --release -- verify \
  fixtures/v5/default-calls1/nytprof.out
```

### Report MVP (P4 proxy)

```sh
/usr/bin/time -v cargo run -q -p nytprof-cli --release -- report \
  fixtures/v5/default-calls1/nytprof.out
```

Same path without `--release` is fine for functional smoke; keep mode consistent when comparing runs. Evidence crate: `crates/nytprof-report` via `nytprof-cli`.

### HTML (P4 proxy when present)

```sh
/usr/bin/time -v cargo run -q -p nytprof-cli --release -- html \
  fixtures/v5/default-calls1/nytprof.out -o /tmp/nytprof-summary.html
```

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

### Collection overhead (later — P1, not R1-preview)

Run a fixed workload with and without `-d:NYTProf` under the pinned oracle `PERL5LIB`. Compare wall time only after confirming the profile is complete and comparable (`calls=`, `stmts=`, etc. fixed). **Never** put `crates/` on oracle `PERL5LIB`.

---

## Recording template (local only)

```text
date:
host:
commit:
rustc:
profile: debug|release
fixture:
command:
n_runs:
elapsed_s: [ ... ]
max_rss_kb: [ ... ]
correctness: (parity smoke / leaf15 mid3 edge15 / verify OK)
notes: (load, thermal, background jobs)
claim: none  # keep "none" until BENCH-001 certification package
```

Keep raw samples with the command line. Do not promote means or “% faster” without the plan harness and an ADR on thresholds.

---

## Out of scope for this note

- CI performance gates (`BENCH-014` continuous monitoring)
- Codec / chunk size selection (`BENCH-007`)
- Real-application canaries (`BENCH-013`)
- Ratified noise bounds or public claim language
- Collector P1 / storage P2 certification
- Inventing numeric SLOs for R1-preview

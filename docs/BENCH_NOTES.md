# Benchmark notes (noise study — not certification)

**Status:** R2-scoped **P1/P2 methodology + residual honesty** (light harness only; public claims waived until R2-stable gates green)  
**Board rows:** `LIGHT-BENCH`, `BENCH-001`, `R2-P1P2-METHODOLOGY` on [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md)  
**Full plan:** [`docs/plan/11_BENCHMARKING_AND_PERFORMANCE_GATES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/11_BENCHMARKING_AND_PERFORMANCE_GATES.md) (WP-13 / `BENCH-001`+)  
**Acceptance map:** [`docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md) (R2 requires **P1/P2 results**; project P1–P4)  
**Residual matrix:** [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md)  
**Agent duty:** keep engineering baselines **current vs 6.15 oracle and vs prior native versions** when decode/report/export/collector paths change — see [`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/main/AGENTS.md). **No public perf claims until certified.**

---

## Explicit non-claims (binding)

| Claim | Status for R0 / R1-preview / R2-preview |
|-------|----------------------------------------|
| Public performance SLOs / “% faster” marketing | **Not claimed** |
| Plan-grade **P1** (collector overhead) certification | **Not claimed** — methodology documented only |
| Plan-grade **P2** (profile storage size) certification | **Not claimed** — methodology documented only |
| Plan-grade **P3** (native read/model) certification | **Not claimed** — offline light proxies only |
| Plan-grade **P4** (native report) certification | **Not claimed** — offline light proxies only |
| WP-13 / `BENCH-001` noise bounds + signed certificate | **Open** (full harness residual) |
| R2-stable performance gate package | **Open** until checklist green |

**R2-preview posture (PR-C04 default):** **waive public performance certification**. Ship **P1/P2 methodology** + light harness proxies so engineering can take local samples without inventing heavy CI or false claims. Design gate text: *“BENCH P1/P2 certified — no public claims until green.”* If a future **R2-stable** cut **publishes** numbers, it must close plan `BENCH-001`+ on advertised surfaces with correctness-paired evidence — not this light path alone.

These notes are **not** a performance certification. Numbers from the light harness or sample tables below are **local exploratory samples** only. Do **not** cite them in release notes, README badges, marketing, or as R2 DoD “P1/P2 results reported” under plan §16 without a certified package.

---

## R2-scoped P1 / P2 methodology (what would be certified)

Charter / acceptance language maps **R2** (v6 collection opt-in / R2-stable depth) to controlled-host **P1/P2** for **collection overhead** and **profile storage**, alongside offline **P3/P4** for native read/report. This section freezes **what to measure** and **what evidence is required** before any claim. It does **not** record a certified result.

### Gate definitions (plan-facing)

| Gate | Plan meaning (summary) | Surface under test | Light harness proxy step |
|------|------------------------|--------------------|--------------------------|
| **P1** | Collector overhead meets ratified wall/CPU targets without precision/feature loss | Overlay `collector/` fast path + (later) live Perl/XS collection vs unprofiled / oracle 6.15 | `collector_micro` (engineering microbench only); oracle collection sketch below |
| **P2** | Profile **storage size** meets ratified targets via reversible encoding (dict/delta/chunk/codec), not lost events | Committed `fixtures/v5/*` + `fixtures/v6/from-c/*` (+ future dual-sink / scale corpus) | `size` (byte sizes of profile files) |
| **P3** | Native **read/model** | `nytprof-cli dump` / `verify` | `dump`, `verify` (when present) |
| **P4** | Native **report** | `report` / `csv` / `html` | `report`, `csv`, `html` (when present) |

Sources: [`16_ACCEPTANCE…`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/16_ACCEPTANCE_CRITERIA_AND_DEFINITION_OF_DONE.md) § Storage + Runtime and memory (P1/P2/P3/P4); plan [`11_BENCHMARKING…`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/11_BENCHMARKING_AND_PERFORMANCE_GATES.md) measurement principles; design residual: *no public claims until green*.

### Measurement principles (must hold for any future claim)

1. **Correctness first** — every timed sample set pairs with equality/parity that already gates the surface (E3-EVENT C fixtures; dual-sink E4 when applicable; dump/report parity; collector unit tests). Reject samples that fail correctness.
2. **Same configuration** — same fixture/workload, same `NYTPROF` options (`calls=`, `stmts=`, codecs), same build profile, same host class. Do **not** disable features on only one side of a compare.
3. **Separate phases** — unprofiled vs profiled wall; collector emit/flush/compress vs offline dump/report; build time is **not** step time.
4. **Raw samples** — store all runs, not only means; keep outliers; record host + toolchain + commit.
5. **Noise bound** — if measurement noise ≥ claimed gain, the claim is invalid (plan provisional rule).
6. **No public mean/% without BENCH-001** — light harness has no ratified noise model, affinity control, or CI dedicated runner.
7. **Never** put `crates/` on oracle `PERL5LIB` for any collection or differential run.

### Fixture / workload corpus (engineering → certified later)

| Corpus | Role |
|--------|------|
| `fixtures/v5/default-calls1`, `default-calls2` | Small committed v5 profiles (P2 size + offline P3/P4) |
| `fixtures/v5/blocks-calls1`, `calls2-default` | Optional blocks / `calls=2` paths (`FIXTURES=…`) |
| `fixtures/v6/from-c/*` | Product E3-EVENT C writer outputs (P2 size by codec/packing mode) |
| Overlay `collector/` unit microbench | P1 **engineering** proxy (`nytp_fast_bench_time_line`) — **not** end-to-end collection |
| Oracle 6.15 fixed workload | True P1 collection overhead (with vs without `-d:NYTProf`) — plan BENCH-003/013 class |
| Dual-sink / scale / real-app canaries | Plan `BENCH-013` + certified package — residual |

### Metrics sketch (certified package later)

| Area | Metric | Notes |
|------|--------|-------|
| Collection (P1) | Wall + CPU of fixed workload unprofiled vs profiled | Absolute and % overhead; same options both sides |
| Collector micro (P1 eng.) | `nytp_fast_bench_time_line` elapsed_ns, sizeof(event) | Engineering only; detects gross regressions |
| Storage (P2) | Raw / compressed profile bytes per logical event | Same event stream; codec/chunk/dict variants labeled |
| Decode (P3) | Wall + peak RSS of dump/verify | Reader vs model |
| Report (P4) | Wall + peak RSS + output bytes | Equality-passing outputs only |

### Promotion checklist (when claiming P1/P2)

- [ ] Plan-grade harness or equivalent (`BENCH-001`): repetitions, metadata, raw samples, statistics
- [ ] Thresholds ratified (ADR or board-approved contract) — light notes do **not** set SLOs
- [ ] Correctness hashes/parity green on every accepted sample (E3/E4/collector as applicable)
- [ ] Dedicated quiet runner notes (CPU affinity optional; no shared noisy CI as sole evidence)
- [ ] Same-options matrix documented (calls/stmts/codecs/features identical unless factor under test)
- [ ] Residual matrix / release notes updated to **claim** only what was measured
- [ ] Public language limited to certified configurations

Until that checklist is green, residual matrix remains: **no performance certification claims** / **no public P1/P2 until R2-stable gates green**.

---

## Light harness (runnable today)

One-shot wall timings + size inventory for offline CLI and collector engineering paths:

```sh
bash tools/bench/light_bench.sh
# optional: also write the same report to a file
OUT=/tmp/nytprof-light-bench.txt bash tools/bench/light_bench.sh

# comparable samples (still not certification)
RELEASE=1 RUNS=3 bash tools/bench/light_bench.sh

# P1/P2-focused steps
STEPS="size,collector_micro,dump,report" bash tools/bench/light_bench.sh

# expand fixtures
FIXTURES="fixtures/v5/default-calls1 fixtures/v5/blocks-calls1" \
  STEPS="size,dump,report" bash tools/bench/light_bench.sh
```

### What it does

1. `cargo build -q -p nytprof-cli` once when any offline CLI step is requested (untimed relative to steps; optional `--release` when `RELEASE=1`)
2. Probes CLI for optional subcommands (`csv`, `html`, `verify`)
3. Optional **P2** `size`: print byte sizes for each fixture profile + committed `fixtures/v6/from-c/*.nytprof` when present
4. Optional **P1** `collector_micro`: when `cc` and `collector/Makefile` are usable, run `make -C collector test` and surface the light microbench NOTE (engineering only)
5. Per fixture (default: `fixtures/v5/default-calls1`, and `default-calls2` if present):
   - timed `dump` (P3 proxy)
   - timed `verify` when present (P3 model proxy)
   - timed `report` (P4 proxy)
   - timed `csv` / `html` when present
6. Optional `RUNS=N` (default 1) repeats each timed step and prints each sample
7. Prints wall seconds / sizes; exits 0 on success; ends with `claim: none`

### Environment

| Variable | Default | Meaning |
|----------|---------|---------|
| `OUT` | unset | Also write report to this path |
| `FIXTURES` | `default-calls1` + `default-calls2` if present | Space-separated fixture dirs |
| `STEPS` | `size,dump,verify,report,csv,html,collector_micro` | Comma-separated steps; missing optional steps skip |
| `RUNS` | `1` | Timed repetitions per timed step |
| `RELEASE` | unset/`0` | `1` → `cargo build/run --release` |
| `SKIP_COLLECTOR` | unset/`0` | `1` → skip `collector_micro` even if listed |

Timing backend: GNU `/usr/bin/time -f '%e'` when available, else bash `TIMEFORMAT`, else `python3` `perf_counter` around a subprocess.

This harness is **smoke + noise study**, not BENCH gate certification. Prefer it over ad-hoc commands when collecting first samples. It is **not** wired into `offline_gate.sh` (no CI perf gate by design).

---

## Collector statement fast path (engineering only — PR-B04)

`make -C collector test` runs a **light** `nytp_fast_bench_time_line` loop inside `test_batch_fast` (5000 TIME_LINE appends through a capacity-16 batch into a counting child). Sample host output is printed as a NOTE (e.g. `sizeof(event)=88`, wall ns). The light harness `collector_micro` step surfaces that path.

| Claim | Status |
|-------|--------|
| No-alloc stmt append after batch create | Unit-tested (`heap_allocs` stable; `arena_bytes_copied==0` on pure TIME_LINE/BLOCK) |
| Exact order under batch stress | Unit-tested (cap 1..64 vs direct counting) |
| Public performance / median regression gates | **Not claimed** — needs BENCH-003 + certified harness |

Do **not** promote the light loop into release notes as a certified win.

---

## Engineering sample log (local only — not a claim)

Illustrative single-host smoke after harness expansion. **Do not treat as baseline or SLO.**

```text
date: 2026-08-12
host: local Linux x86_64 (engineering laptop)
commit: (see harness banner at run time)
rustc: rustc 1.97.x
profile: debug (default light_bench; cargo run after build)
fixtures: fixtures/v5/default-calls1, fixtures/v5/default-calls2
command: bash tools/bench/light_bench.sh
n_runs: 1
note: wall seconds dominated by process startup on tiny fixtures; size rows are inventory only
claim: none
```

Re-run after collector/decode/report changes and append a new raw block (or `OUT=` file). Compare directionally only; never publish “% faster” from this log.

**2026-08-15 output-size note (flame default-on):** CLI `html` now emits the call-tree flame unless `--no-flame` (oracle `nytprofhtml` parity). On `fixtures/v5/default-calls1`: site total **251,059 → 262,419 B** (+11,360 B ≈ +4.5%; `all_stacks_by_time.svg` 4,854 + `.folded` 354 + index inline section); `index.html` **18,520 → 24,672 B** (+6,152 B ≈ +33% on this small fixture). Bounded by design: sub-pixel frames omitted, labels ≥48 px, depth ≤16; folded is linear in `call_edges`. `--no-flame` reproduces the old footprint exactly.

**2026-08-15 output-size note (HTML report CSS refresh):** `SHARED_STYLE_CSS` was restyled (modern tables/header, `prefers-color-scheme: dark` block). Published `style.css` on `fixtures/v5/default-calls1`: **2,989 → 6,990 bytes** (+4,001 B, one shared asset per multi-file site; single-file summaries inline the same text once). Same-day flame polish (rounded frames, separator stroke, `pointer-events` labels): `all_stacks_by_time.svg` **4,481 → 4,854 bytes** (+373 B ≈ 31 B/frame on this 12-frame fixture; index.html grows the same +373 B from the inlined copy). No fixture or timing change; direction: intentional one-time growth for theming, bounded (constant per site / linear in painted frames, which are themselves capped).

---

## Command sketches (manual / deeper)

Use `/usr/bin/time -v` (GNU time) for elapsed + max RSS. Repeat ≥5 times on a quiet machine; record host, `rustc --version` / `cc --version`, commit, and fixture path. Do **not** average away outliers without writing them down. For a quick combined pass, prefer `tools/bench/light_bench.sh` first.

### Storage inventory (P2 proxy — available today)

```sh
# harness
STEPS=size bash tools/bench/light_bench.sh

# manual
stat -c '%n %s' fixtures/v5/*/nytprof.out fixtures/v6/from-c/*.nytprof 2>/dev/null
```

### Collector microbench (P1 engineering proxy)

```sh
STEPS=collector_micro bash tools/bench/light_bench.sh
# or:
make -C collector test
# look for: NOTE: light microbench TIME_LINE ... (engineering only; not BENCH certification)
```

### Decode / dump (P3 proxy)

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

### HTML / CSV (P4 proxies when present)

```sh
/usr/bin/time -v cargo run -q -p nytprof-cli --release -- html \
  fixtures/v5/default-calls1/nytprof.out -o /tmp/nytprof-summary.html
/usr/bin/time -v cargo run -q -p nytprof-cli --release -- csv \
  fixtures/v5/default-calls1/nytprof.out
```

Skip if the CLI has no matching subcommand (light harness skips automatically).

### Oracle HTML path (collection/report baseline, not Rust)

```sh
# Requires baseline/6.15 pin; see tools/oracle/env.sh
# Example shape only — exact flags live in oracle scripts.
/usr/bin/time -v perl -I baseline/6.15/install/lib/perl5 \
  baseline/6.15/install/bin/nytprofhtml \
  --file fixtures/v5/default-calls1/nytprof.out \
  --out /tmp/nytprofhtml-out
```

### Collection overhead (true P1 — later / R2-stable cert package)

Run a fixed workload **with and without** `-d:NYTProf` under the pinned oracle `PERL5LIB`. Compare wall time only after confirming the profile is complete and comparable (`calls=`, `stmts=`, etc. fixed). **Never** put `crates/` on oracle `PERL5LIB`. Overlay collector live hooks remain residual until product XS path lands.

---

## Recording template (local only)

```text
date:
host:
commit:
rustc:
cc:
profile: debug|release
fixture:
command:
n_runs:
elapsed_s: [ ... ]
max_rss_kb: [ ... ]
profile_bytes: [ ... ]   # P2
correctness: (parity smoke / E3 / E4 / collector test OK)
notes: (load, thermal, background jobs)
claim: none  # keep "none" until BENCH-001 certification package
```

Keep raw samples with the command line. Do not promote means or “% faster” without the plan harness and an ADR on thresholds.

---

## D3 durable-seal cost (engineering only — not certification)

**Date:** 2026-08-15  
**Host:** local Linux x86_64  
**Commit:** `10e9137` + uncommitted D2/D3 tree  
**Command:** dest `collector/build/xs-nytprof` + `t/workload-calls1.pl`  
`NYTPROF=file=…:compress=6:durable=0` vs `:durable=1`  
**Samples (one run each):** both `elapsed_s=0.02`; RSS 7088 vs 7320 KiB; profile 10131 vs 10099 bytes.  
**Direction:** close-only cost on this tiny stream is noise (file ≪ 256 KiB dirty, so no periodic seal).  
**Gates:** `g09_tokenize_excl_smoke.sh` and `di01_blocks_780_smoke.sh` green with dest `.so`.  
**Default:** **`durable` stays 0.** KD-D5 requires a 25s-scanner seal-cost sample plus those gates before flipping; this note is not that sample.  
**claim:** none

Copy of the local snippet: `/tmp/grok-goal-0641539d8045/implementer/bench-note.txt` (not in-tree).

---

## Field 25s scanner — zlib default vs compress=0 (engineering only)

**Date:** 2026-08-15  
**Host:** local Linux x86_64  
**Command:** `./scripts/field/compare_oracle_native_reports.sh --seconds 25 --out ~/Downloads/nytprof-compare-apples`  
plus same corpus `NYTPROF=file=…:compress=0` native control.  
**Samples (one run each):**

| Side | `nytprof.out` | zlib | passes |
|------|---------------|------|--------|
| oracle 6.15 default | 6,063,503 | yes | 821 |
| native omitted (`compress=6`) | 472,181 | yes | 2328 |
| native `compress=0` | 2,895,334 | no | 2500 |

**Direction:** omitted-compress native is **−83.7%** vs the same-run `compress=0` file (~6.1×). Previous inspectable native pair was 4.2 MiB uncompressed (different pass count). Oracle was already zlib; oracle vs native size is not a codec-only compare.  
**claim:** none  
**Artifacts:** `~/Downloads/nytprof-compare-apples/SIZE_COMPARE.txt`

---

## PR-16 wrap_push / wrap_pop (engineering only)

**Date:** 2026-08-17  
**Host:** local Linux x86_64  
**Command:** `./scripts/packaging/g16_wrap_enter_smoke.sh` (dest `collector/build/xs-nytprof`, `stmts=0`, N=120000 leaf wraps, inner-loop `Time::HiRes`; control `NYTPROF_WRAP_SLOW=1` still does Perl `caller(0)` + fid/clock/emit XSUBs).  
**Samples (g16 smoke captured in-session):** default loops **0.94s / 1.09s** (mean **1.01s**) vs WRAP_SLOW **1.72s**; both default runs `leaf SUB_RETURN=120000`; leaf `SUB_CALLERS` file is `wrap.pl` (not `NYTProfM.pm`).  
**Direction:** fewer C crossings on the instrumented wrap (COP pin + emit in `wrap_push`/`wrap_pop`) is cheaper than the Perl caller+XSUB control. **Does not** beat 6.15 `entersub`. Not DI-03 / not stock 6.15 XS.  
**claim:** none

---

## PR-15 C `OP_DBSTATE` TIME_LINE (engineering only)

**Date:** 2026-08-17  
**What changed:** default `stmts=1` no longer enters Perl `DB::DB` (`caller` + `fid_for_filename` XSUB) on every statement. `TIME_LINE` comes from `pp_product_dbstate_line` after `INIT`; `$DB::single=0`. Last-COP fid pointer cache is CopFILE-only.  
**Direction:** cheaper than the previous native Perl `DB::DB` path on statement-heavy work. **Does not** automatically beat 6.15 C `entersub` / opcode attach — Perl `DB::sub` wrap remains.  
**Gates:** `g15_dbstate_timeline_smoke.sh`, `g04_v5_parity_smoke.sh`, `g09_tokenize_excl_smoke.sh`, `di01_blocks_780_smoke.sh` green.  
**claim:** none

---

## Out of scope for this note

- CI performance gates (`BENCH-014` continuous monitoring)
- Codec / chunk size **selection** ADRs (`BENCH-007`) — size inventory only here
- Real-application canaries (`BENCH-013`) as certified package
- Ratified noise bounds or public claim language
- Inventing numeric SLOs for R2-preview
- Treating `collector_micro` elapsed_ns as public P1 certification

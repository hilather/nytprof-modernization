# Agent hints — Devel::NYTProf modernization

**Status:** binding for every agent session (main and subagents) in this repository  
**Audience:** coding agents and human implementers acting as agents  
**Does not replace:** [`docs/PROGRAM_CHARTER.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md), [`docs/plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md), or accepted ADRs

Read this file before implementing, reviewing, or shipping changes. Agents own **tasks**, not architectural truth — specs, ADRs, immutable fixtures, and the charter override local preferences.

---

## Mandatory quality bars (always)

### 1. Regression tests for every fix

| Rule | Detail |
|------|--------|
| **No fix without a test** | Every bug fix, fail-closed path, regression, or behavioral repair **must** land with a **regression test** that fails before the fix and passes after. |
| **Drive shipped code** | Tests must call the **real entry point** (CLI, library API, script) — no reimplementation of the code under test, no hardcoded “always pass” stubs, no starting past the broken path. |
| **Keep the gate green** | Prefer `./scripts/ci/offline_gate.sh` (or the focused package suite the gate runs) after non-trivial changes. Do not merge or claim done with a known red offline gate. |
| **Fixture honesty** | Do not edit golden fixtures solely to make a broken implementation pass. Fixture changes require an approved semantic reason and dual-path evidence. |

### 2. Performance and output size must stay optimal

| Rule | Detail |
|------|--------|
| **No silent bloat** | Prefer the smaller, faster design when correctness is equal. Avoid unnecessary copies, full-buffer re-aggregations, and oversized intermediate files. |
| **Bound allocations** | Length-prefixed and frame parsers must fail closed on oversize lengths **before** large allocations (see provisional v6 schemas under `docs/schemas/`). |
| **Output size** | Encoded profiles, dumps, reports, and packaged artifacts should not grow without cause. When changing codecs, headers, or report emitters, measure and document size impact. |
| **No unequal benchmarks** | Do not disable features on only one side of a comparison (native vs oracle / new vs old). See feature-parity and bench plan docs. |
| **Claims** | Public performance claims require certified gates (plan `BENCH-*`). Light harnesses (`tools/bench/light_bench.sh`) inform engineering; they are **not** release certification by themselves. |

### 3. Documentation must stay current

| Rule | Detail |
|------|--------|
| **Same change set** | When behavior, CLI flags, schemas, packaging, or operator guidance changes, **update the matching docs in the same change** (board, residual matrix, runbook, schemas, README, BUILD policy as applicable). |
| **No silent capability claims** | Do not mark board/parity rows done without implementation; do not ship behavior without updating status docs that claim readiness. |
| **Residual honesty** | Keep residual limitations explicit (FFI, XS Data, full DOM, COL-007 full writer, multi-OS CI, perf claims). |
| **Absolute links** | In README / docs / release notes, use **absolute HTTPS URLs** for cross-file links (relative links break outside the tree). |

Project global rule also applies: structured code review after non-trivial changes; docs with behavior (see maintainer/agent global rules).

### 4. Release notes must list all relevant changes

| Rule | Detail |
|------|--------|
| **Complete delta** | Every release (tag, R0/R1-preview cut, or versioned package) **must** have release notes covering **all relevant changes** since the previous release: features, fixes, schema/CLI changes, packaging, known residuals, and upgrade notes. |
| **Where to write** | Prefer a dated entry under project release notes / `Changes` / GitHub Release body for the tag. Pin absolute links to the **version tag** for cross-file refs. |
| **No “misc fixes” dump** | Group by theme (CLI, format, Perl facade, packaging, docs). Call out breaking or fail-closed behavior changes explicitly. |
| **Omit noise** | Skip pure formatting/typo-only churn unless it affects operators; never omit behavioral or security-relevant fixes. |

### 5. Benchmarks must stay up to date (Perl oracle and prior versions)

| Rule | Detail |
|------|--------|
| **Vs Perl / 6.15 oracle** | When changing decode, model, report, export, or collector-adjacent paths that affect wall time, CPU, peak memory, or output size, refresh or extend comparisons against the **pinned oracle** path (`baseline/6.15/`, isolated `PERL5LIB` — **never** put `crates/` on oracle `PERL5LIB`). |
| **Vs previous versions** | Keep engineering baselines current against **prior native builds/tags** (or the last recorded harness snapshot in `docs/BENCH_NOTES.md` / plan BENCH tasks) so regressions are visible over time. |
| **Same workload** | Use the same fixtures and feature options on both sides of a comparison. |
| **Apples-to-apples reports** | Inspectable oracle vs native **HTML/profile** comparisons (Downloads, dual-docker lab, design evidence) must use the **same** scanner script, **same** `--seconds`, **same** corpus tree, and the **same** host class. Do **not** compare a 25s oracle site to a 60s native site, or 2-file lab seed to a 12-copy Gutenberg demo, and then discuss exclusive ranking as if it were the same run. Use [`scripts/field/compare_oracle_native_reports.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/compare_oracle_native_reports.sh). Dual-docker `--engine both` already shares `TARGET_SECS` and corpus — keep it that way. Residual exclusive-math differences may remain; duration/corpus must not. |
| **How to read speed / time gaps** | See [Interpreting oracle vs native times](#interpreting-oracle-vs-native-times) below. Same wall budget does **not** mean the same amount of application work or the same exclusive seconds. |
| **Record results** | Update `docs/BENCH_NOTES.md` (or the certified bench package when present) with command, host notes, and direction of change — without inventing certification claims. |
| **Harness** | Local light harness: `./tools/bench/light_bench.sh`. Full certification remains plan WP-13 / `docs/plan/11_BENCHMARKING_AND_PERFORMANCE_GATES.md`. |

#### Interpreting oracle vs native times

Even a correctly paired run (same script, seconds, corpus, host) will **not** show the same exclusive seconds. That is expected. Do not “fix” HTML units or rescale native ticks to match 6.15.

| What you see | What it means |
|--------------|----------------|
| Same `--seconds`, native did **more scanner passes** (e.g. 3788 vs 1151 in 25s) | Devel::NYTProf **6.15 is heavier** (C `entersub` / stmt / slowops). NYTProfM’s `DB::sub` + MATCH/PRINT wrap lets the app finish more work in the same wall budget. |
| Native `CORE:match` exclusive is several times oracle’s (e.g. 15.5s vs 4.47s) on that pair | Scale by **work**, not wall: match calls / scanner `passes=`. 15.5/4.47 ≈ 3788/1151. Per-match cost is similar; native is not 3× “too large.” |
| Oracle “Profile of … for **9.08s (of 24.2s)**” vs native “**24.2s**” | 6.15 `profiler_active` vs process wall. Native summary is **Σ statement ticks** / `ticks_per_sec` (often ≈ requested wall). Different numerator — do not treat 9s vs 24s as a clock bug. |
| Oracle `tokenize` excl **137ms** / incl **4.61s**; native **470ms** / **16.0s** | **Shape** must match: parent exclusive is a **small remainder** after MATCH/PRINT (`excl ≈ incl − children`). Inclusive tracks the extra passes. |
| Native `tokenize` excl ≈ `CORE:match` excl (old Rocky 60s demo: 39.6s ≈ 38.6s) | **Broken exclusive split** on that profile (child slowop not subtracted), or a pre-fix `nytprof.out`. Re-profile with current attach; do not re-render HTML and claim a timing fix. |
| Mixed 25s oracle vs 60s native, or 2-file vs 12-copy corpus | **Not a comparison.** Re-run [`compare_oracle_native_reports.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/compare_oracle_native_reports.sh). |

Native finishing more work in N seconds is **engineering observation**, not a certified public perf claim (still plan `BENCH-*`). `collection_default` stays v5.

### 6. Failed attempts and language semantics (save automatically)

Negative knowledge is part of the repo. Agents **must** automatically record failed implementation attempts and corrected language misunderstandings so later sessions do not re-pay the same cost.

**Shape:** keep the **default notes light** (one short table row — cheap to load into context). **Drill down** into `docs/agent-notes/details/<slug>.md` only when a later agent would need more than one line (benches, stack traces, multi-step postmortems).

| Rule | Detail |
|------|--------|
| **When to write (automatic)** | Do **not** wait for a dedicated “write notes” task. Append a light row as soon as you **abandon** an approach, **fail a gate** after real effort, or **correct** a Perl/Rust misunderstanding — ideally in the **same change set**, otherwise immediately after aborting. |
| **Failed attempts** | Abandoned or gate-failed approaches — especially **perf** optimizations that regressed or did not win, but also format/API experiments that broke parity, packaging/CI dead-ends, rewrites that lost simplicity without benefit. **Append one short row** to [`docs/agent-notes/failed-attempts.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/failed-attempts.md). |
| **Language semantics (Perl & Rust)** | When **Perl** or **Rust** (or oracle / XS / dual-engine) behavior was **misunderstood** and then corrected — or remains open — append a short row to [`docs/agent-notes/language-semantics.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/language-semantics.md). Prefer pointers to fixtures/contracts over restating whole manuals. |
| **Light by default** | One table row only: date, slug/topic, what was tried or wrong assumption, why it failed / correct rule, optional detail link. **No** session transcripts, full logs, or essay postmortems in the light ledgers. |
| **Drill-down when needed** | If the light row is insufficient, add `docs/agent-notes/details/<slug>.md` and link it from the row. Index + templates: [`docs/agent-notes/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/README.md). |
| **Same honesty bar** | Notes record *what failed or was wrong* — they do **not** override ADRs, fixtures, residual matrix, or the charter. Failed perf rows are **not** certification claims. |
| **Before proposing big rewrites** | Skim the two light ledgers (and any linked details) so you do not re-propose a known-failed approach without new evidence. |

**Examples that always get a light row**

| Situation | Ledger |
|-----------|--------|
| Perf tweak tried; wall time / peak RSS / size worse or no win | `failed-attempts.md` |
| Alternate codec / API / packaging path abandoned after red gate or wrong parity | `failed-attempts.md` |
| Misread Perl oracle option, call multiplicity, or `PERL5LIB` isolation | `language-semantics.md` (`perl`) |
| Misread Rust ownership, fail-closed parse, or crate API contract | `language-semantics.md` (`rust`) |

---

## Working rules (agents)

1. Prefer small, mergeable slices that keep the offline gate green.  
2. Never put `crates/` on oracle `PERL5LIB`.  
3. Derive counts and aggregates from dump/model/JsonlData — do not invent fixture constants detached from real loads.  
4. Oracle vs native **report** comparisons are apples-to-apples only: same script, same seconds, same corpus (quality bar §5). Never present mixed-duration sites as the same profile.  
5. Provisional v6 preflight (`nytprof-format-v6`, `docs/schemas/v6-*-provisional-v0.md`) is **not** a wire freeze and does **not** complete COL-007 (C v6 writer).  
6. Stop and escalate when observed 6.15 behavior contradicts frozen specs; do not guess wire or timing semantics.  
7. Handoffs include commits, commands, artifacts, open questions, and known limitations.  
8. **Automatically** append light rows under `docs/agent-notes/` for abandoned attempts (incl. failed perf) and corrected Perl/Rust misunderstandings; open a `details/<slug>.md` only when the light row is not enough.  
9. **After any push to `main`, tag, or GitHub Release: watch CI until green** (see [Releases and CI watch](#releases-and-ci-watch) below). Do not declare a release done while Actions is red or unwatched.

## Releases and CI watch

Agents that cut a release, merge a stack to `main`, push a version tag, or publish a GitHub Release **must** monitor continuous integration until it passes, and must prefer CI hardening so the same class of failure does not recur.

### Mandatory post-release / post-merge CI watch

| Step | Action |
|------|--------|
| **1. Identify the head** | After `git push origin main` and/or `git push origin <tag>`, note the commit SHA and any tag (`v*`). |
| **2. List runs** | `gh run list --branch main --limit 10` (and `gh run list --limit 10` filtered by the tag push if present). Expect workflow **CI matrix (BUILD-006 MVP)** (`.github/workflows/ci-matrix.yml`). On `v*` also expect **Release EL8 RPM (test-drive)** (`.github/workflows/release-el8-rpm.yml`) and wait until `perl-NYTProfM-6.15-*.el8.x86_64.rpm` is attached to the GitHub Release. |
| **3. Wait for completion** | `gh run watch <run-id> --exit-status` (or poll until `completed`). Prefer waiting on **all** jobs: `rust-smoke` and both matrix rows (`linux-x86_64`, `macos-arm64`). |
| **4. On failure** | `gh run view <run-id> --log-failed`. Fix on a branch, open/merge to `main`, retag only if the release notes/tag commit must move (prefer a patch tag `vX.Y.Z` over force-moving published tags). Re-run watch until green. |
| **5. Record** | In the release body or session handoff: run URL(s), conclusion, and any residual honest skips (oracle pin, cargo-absent packaging paths). |
| **6. Do not ship red** | Do not announce the release complete, close the task, or mark board “released” while required CI is failed or still in progress without an explicit maintainer waiver. |

### CI hardening expectations (agents)

When fixing a CI failure, also **harden** so the next release does not hit the same class of bug:

| Hardening | Detail |
|-----------|--------|
| **Fail closed on capability honesty** | `nytprof-cli capability --json` must emit the keys product tests assert (`collection_default`, `v6_decode`, `v6_report`, convert/merge/repack/salvage when claimed). Stack/merge conflict resolution must not drop those fields. |
| **Keep smoke job first** | `.github/workflows/ci-matrix.yml` runs a Linux `rust-smoke` (`cargo test` on gate packages + targeted `clippy -D warnings --no-deps`) **before** the longer multi-OS offline_gate matrix. Preserve that ordering when editing CI. |
| **Matrix deps** | Linux needs `zlib1g-dev` / `libzstd-dev` / `liblz4-dev` for collector codec links; macOS needs brew `zstd`/`lz4` when collector builds, plus **zlib** (SDK and/or brew) so the host-local oracle pin builds with `HAS_ZLIB`. Without zlib, dual_path fails on compressed fixtures. Do not remove them without an honest skip path. |
| **No soft-fail gates** | Matrix and offline_gate scripts must use `set -euo pipefail` (or equivalent) so a red suite fails the job. Do not wrap gates in `|| true`. |
| **Local before push** | For release merges: at least `cargo test -p nytprof-format-v6 --lib`, `cargo test -p nytprof-cli --test cli_e5_v6 --test capability_selftest`, and `cargo clippy -p nytprof-cli -p nytprof-model -p nytprof-report --no-deps -- -D warnings`. Prefer `./scripts/ci/offline_gate.sh` when oracle pin time allows. |
| **Clippy scope** | Prefer `-D warnings` on product CLI/model/report surfaces; preflight-heavy crates (e.g. format-v6) and C-ABI FFI raw pointers may stay outside `-D` until cleaned — document exceptions in the workflow comments. |
| **Never trust bare `cargo fix`** | `cargo fix` / auto-import cleanup drops imports that are **only** used under `#[cfg(test)]`, which breaks `cargo test -p … --lib` (and thus `rust-smoke`) while `cargo check` still passes. Put test-only imports **inside** the `#[cfg(test)] mod tests` block; after any fix pass, re-run the local suite above before push. |
| **Stack assemble must keep PR bodies** | Plain-git stack reassembly (`--theirs` / conflict mass-resolve) can keep **tests + schemas** while dropping **implementation** (e.g. HTML `--flame`, `index-subs-excl.html`, stderr style listing). Before tagging: run the full rust-smoke package list locally (`cargo test -p nytprof-cli …` includes `html_optional_flame`, `html_subs_excl`, `html_shared_css`). Do not ship a release when stack tip only “documents done.” |

### Suggested watch commands

```bash
# After push to main / tag:
gh run list --branch main --limit 5
gh run watch "$(gh run list --branch main --workflow 'CI matrix (BUILD-006 MVP)' --limit 1 --json databaseId -q '.[0].databaseId')" --exit-status
# After a v* tag:
gh run watch "$(gh run list --workflow 'Release EL8 RPM (test-drive)' --limit 1 --json databaseId -q '.[0].databaseId')" --exit-status

# On failure:
gh run view <run-id> --log-failed | tail -200
```

## Primary gates and docs

| Item | Path |
|------|------|
| Offline R1 gate | `./scripts/ci/offline_gate.sh` |
| Multi-OS CI matrix (GHA) | [`.github/workflows/ci-matrix.yml`](https://github.com/hilather/nytprof-modernization/blob/main/.github/workflows/ci-matrix.yml) |
| Operator runbook | [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) |
| First-slice board | [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md) |
| Residual matrix | [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) |
| Agent work packages | [`docs/plan/14_AGENT_WORK_PACKAGES_AND_HANDOFFS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/14_AGENT_WORK_PACKAGES_AND_HANDOFFS.md) |
| Bench notes | [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md) |
| Agent notes (failed attempts + language) | [`docs/agent-notes/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/README.md) |
| Oracle vs native paired reports | [`scripts/field/compare_oracle_native_reports.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/compare_oracle_native_reports.sh) |

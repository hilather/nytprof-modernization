# Release notes — Full R1 MVP product cut (PR-A10)

**Date:** 2026-08-11  
**PLAN_ID:** `8c9b1a63`  
**Board ID:** `R1-FULL-READINESS-CUT`  
**Policy:** [ADR-0003 — Full R1 residual close-or-waive policy](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md)  
**Residual matrix:** [R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) (§ Residual for full R1 + § Full R1 ready)  
**Operator runbook:** [R1_PREVIEW_OPERATOR_RUNBOOK.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md)

These notes freeze the **advertised full R1 MVP product scope** after Phase A. They are **not** a CPAN upload statement, performance certification, or complete residual-table close.

---

## Summary

| Theme | What ships under this cut | Honesty |
|-------|---------------------------|---------|
| **FFI (OQ-2 / PR-A05)** | `nytprof-ffi` cdylib open/query/close over `ProfileModel` | **MVP only** — not full RUST-010 |
| **Perl Data/ReadStream (OQ-2 / PR-A06)** | Product `Data` / `ReadStream` over **binary** profiles (thin native-cli-jsonl) | **MVP only** — not COMPAT-007 / pure-XS wire decode |
| **HTML (PR-A01 / A02)** | Shared CSS + structure; exclusive sub index page | **MVP only** — not full oracle DOM |
| **HTML Shared JS** | — | **WAIVE** for GA-candidate (**PR-M01** / Q4) — A01 CSS-only; tablesorter/jquery **not** shipped; not a remaining CLOSE requirement |
| **HTML flame (PR-A03)** | — | **Residual open** — not claimed ready |
| **Multi-OS CI (PR-A07)** | GHA Linux + macOS offline_gate matrix | **MVP only** — not full BUILD-006 |
| **Packaging (PR-A08)** | MakeMaker facade + install-facade / dual-install depth | **Depth MVP** — not full BUILD-003 XS CPAN |
| **Performance (PR-A09)** | Light bench + methodology notes | **Public claims WAIVED** |
| **Collector / format** | v5 read/report product path; v6 preflight crate only | **No COL-007**, no wire freeze, no CLI v6 default |

---

## CLI / report

- Native CLI surfaces remain as offline R0 / R1-preview: `dump`, `verify`/`inspect`, `report`/`summary` (+ `--json` / aggregates), `html` (± multi-file `--out-dir`), `csv`, `folded`, `callgrind`/`cg`, `capability`/`selftest`.
- **HTML A01:** multi-file `style.css` + single-file inline CSS policy; stable structure classes (`table.subs`, call-edges, …). Schema: [`html-shared-css-structure-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md).
- **HTML A02:** multi-file `index-subs-excl.html` exclusive ranking; semantic leaf **15** / mid **3** on default-calls1. Schema: [`html-subs-excl-index-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-subs-excl-index-mvp-v0.md).
- **HTML Shared JS:** **not** shipped — **PR-M01** / Q4 user-final **WAIVE** for GA-candidate (documentation residual, not CLOSE). Do not advertise tablesorter/jquery as native-ready.
- **HTML A03 flame:** **not** shipped on this cut — do not advertise native flame SVG / site flame inputs as ready. Related export: native `folded` for external flame tools.
- Frozen semantic counts unchanged (default-calls1 leaf **15** / mid **3** / mid→leaf **15**; blocks-calls1 line5 **780**; JSON blocks **780**/**810**; calls2 `sub_entry` **27**; etc.).

---

## Perl facade / product Data

- Pure-Perl `JsonlData` / `JsonlReadStream` dump-JSONL path remains the bridge.
- **PR-A06 product path:** `Devel::NYTProf::Data` and `::ReadStream` open **binary** profiles via native `nytprof-cli dump` → Jsonl* ([`perl-xs-data-readstream-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-xs-data-readstream-mvp-v0.md)).
- `claims_compat007_shapes=0` — **no COMPAT-007** claim.
- Offline gate step **5b:** `scripts/packaging/perl_xs_data_readstream_smoke.sh`.

---

## FFI / packaging / CI

- **PR-A05:** `crates/nytprof-ffi` (`cdylib`+`rlib`) + `include/nytprof_ffi.h`; schema [`ffi-cdylib-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/ffi-cdylib-mvp-v0.md). Dual-path must work **without** loading the dylib.
- **PR-A07:** [`.github/workflows/ci-matrix.yml`](https://github.com/hilather/nytprof-modernization/blob/main/.github/workflows/ci-matrix.yml) — `ubuntu-latest` (`linux-x86_64`) + `macos-latest` (`macos-arm64`); entry `scripts/ci/matrix_gate.sh`.
- **PR-A08:** `make install-facade` / `dual-install` / packaging-status stamps `full_build003=0`; smoke `scripts/packaging/makemaker_build003_depth_smoke.sh`.
- Primary operator gate remains `./scripts/ci/offline_gate.sh` (includes `-p nytprof-ffi` when cargo present).

---

## Performance

- **Default full-R1 posture:** **WAIVE** public performance certification (ADR-0003 / PR-A09 default).
- Engineering only: [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md), `tools/bench/light_bench.sh`.
- Do **not** cite light harness numbers as R1 DoD P3/P4 results or marketing “% faster.”

---

## Explicit non-claims (binding)

Do **not** advertise under this cut:

1. **COL-007** C v6 writer (or COL-008 batched Rust writer).
2. **v6 wire freeze** or stable v6 numeric/wire IDs.
3. **CLI v6 default** / default-parse always-inflate product path.
4. **Full oracle `nytprofhtml` DOM** (tablesorter/JS chrome, Graphviz, treemap, flame site, block/sub page modes, oracle naming). Shared JS/tablesorter is **WAIVE** for GA-candidate (**PR-M01** / Q4; not shipped). Flame A03 remains **OPEN**.
5. **Full BUILD-003** XS CPAN dual-build with collector/XS in root Makefile.
6. **Full BUILD-006** multi-Perl / multi-rustc / Windows / coverage dashboard certification.
7. **Full RUST-010** beyond open/query/close MVP (batch APIs, production dylib install, ABI freeze tooling).
8. **COMPAT-007** bless-array fidelity or pure-XS binary decode without CLI.
9. **Public performance SLOs** / certified P3–P4 package.
10. **R3/R4 product default flips** (`engine=auto` as product default; format defaults). R3 **policy** is [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) (PR-D02); **runtime flip not executed** — requires accepted field report + [R3_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md).
11. **CPAN upload** readiness.

---

## Upgrade / operator notes

| Audience | Guidance |
|----------|----------|
| Preview operators | Unchanged offline gate; native remains **opt-in**. Never put `crates/` on oracle `PERL5LIB`. |
| Embedders | Optional `nytprof-ffi` open/query/close MVP; dual-path without dylib still required. |
| Perl Data consumers | Prefer product `Data`/`ReadStream` for binary profiles when native CLI is available; Jsonl* dump path remains. |
| HTML consumers | Expect multi-file site with `style.css` + `index-subs-excl.html`; do not expect oracle DOM or flame SVG yet. |
| Release engineers | Use multi-OS matrix MVP + offline_gate; do not treat depth packaging as full BUILD-003. |

---

## Evidence map

| Item | Path |
|------|------|
| Residual cut | [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) |
| Policy ADR | [`docs/adrs/0003-r1-full-residual-policy.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) |
| HTML inventory | [`docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md) |
| Runbook | [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) |
| Board | [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md) (`R1-FULL-READINESS-CUT`) |
| Offline gate | [`scripts/ci/offline_gate.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/offline_gate.sh) |

---

## Phase A PR index

| PR | Role | Status in this cut |
|----|------|--------------------|
| PR-A01 | Shared CSS + structure | closed (MVP); Shared JS **WAIVE** (PR-M01) |
| PR-A02 | Exclusive sub index | closed (MVP) |
| PR-A03 | Optional flame | residual open |
| PR-A04 | ADR-0003 policy | done |
| PR-A05 | FFI cdylib MVP | closed (MVP) |
| PR-A06 | XS Data/ReadStream MVP | closed (MVP) |
| PR-A07 | Multi-OS CI MVP | closed (MVP) |
| PR-A08 | Packaging depth | closed (depth MVP) |
| PR-A09 | Perf certification | public claims **WAIVED** |
| PR-A10 | This readiness cut | done |

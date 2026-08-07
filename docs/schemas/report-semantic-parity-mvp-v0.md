# Report semantic parity MVP (v0)

**Status:** first-slice semantic checklist (not full DOM parity)  
**Board ID:** `REPORT-SEMANTIC-PARITY`  
**Not:** full `nytprofhtml` multi-file DOM / CSS / tablesorter parity (REPORT-001..020)

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

These numbers are fixed by the committed fixture and `aggregates.oracle.json`. Tests **must** load the real profile via `ProfileModel::from_path` (or the shipped CLI path) and assert against real model APIs / rendered output — not invent unrelated constants.

## Semantic checklist (required)

1. **Oracle HTML site** — under isolated oracle `PERL5LIB` (never `crates/`), `nytprofhtml` produces a non-empty HTML site for the profile (at least `index.html` or equivalent HTML files under the output directory).
2. **Native single-file HTML** — `nytprof-cli html <profile> -o <path.html>` (or stdout) contains:
   - names `main::leaf` and `main::mid`
   - returns **15** and **3** in the subroutine table context (table cells or clearly associated text)
   - call-edge mid→leaf count **15** (call-edges table row cells)
3. **Native multi-file HTML (optional but preferred)** — `nytprof-cli html <profile> --out-dir DIR` writes `index.html` (and related pages) with the same leaf/mid/edge numbers on the index.
4. **Model API** — after `ProfileModel::from_path` on the fixture:
   - `sub_total("main::leaf").returns == 15`
   - `sub_total("main::mid").returns == 3`
   - `call_edge("main::mid", "main::leaf").count == 15`

## Oracle side

Isolation (mandatory):

```sh
# Prefer shared helper — sets PERL5LIB from baseline/6.15 only
source tools/oracle/env.sh
# Assert: PERL5LIB has no crates/ path component
nytprofhtml -o <tmpdir>/oracle-html -f fixtures/v5/default-calls1/nytprof.out
# or: nytprofhtml -o <tmpdir>/oracle-html fixtures/v5/default-calls1/nytprof.out
```

Rules:

- `PERL5LIB` must come only from `baseline/6.15` (install + optional test-deps) — see `tools/oracle/env.sh` and `baseline/6.15/oracle-perl5lib.txt`.
- **Never** put `crates/` (or candidate `perl/` facade) on oracle `PERL5LIB`.
- **Runtime dep:** `nytprofhtml` requires `File::Which` (oracle `Makefile.PL`). Install into the local gitignored tree if missing:
  `cpanm -L baseline/6.15/test-deps File::Which`
  The smoke script `tools/oracle/report_semantic_parity.sh` bootstraps this automatically when `cpanm`/`cpan` is available.
- Success criterion for this MVP: output directory contains **non-empty** HTML (e.g. `index.html`). Full DOM / visual parity with native is **out of scope**.

## Native side

```sh
# Single-file
cargo run -q -p nytprof-cli -- html fixtures/v5/default-calls1/nytprof.out -o <tmpdir>/native.html

# Multi-file site
cargo run -q -p nytprof-cli -- html fixtures/v5/default-calls1/nytprof.out --out-dir <tmpdir>/native-site
```

Library entry points (shipped report path):

- `nytprof_report::render_html_summary`
- `nytprof_report::render_html_site` / `write_html_site`

Schemas for HTML shape (not full oracle DOM):  
[html-report-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-report-mvp-v0.md),  
[html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md).

## How ticks / time are treated

| Field class | Parity rule for this MVP |
|-------------|---------------------------|
| **Counts** (returns, call-edge count, event counts used in smoke) | **Exact** — must match oracle aggregates / model |
| **Time ticks** (incl/excl sub totals, edge incl/excl, line ticks) | **Not required** for this checklist. Compare only under **COMPAT-003** (precision / numeric-conversion policy) when that contract is frozen |

Do not fail semantic parity smoke solely because displayed tick strings differ in formatting or floating conversion.

## Explicit non-requirements

- Full `nytprofhtml` DOM / CSS / tablesorter / flame / JS visualization parity
- REPORT-001..020 complete report matrix
- Byte-identical HTML to oracle
- Tick/time equality (see COMPAT-003)
- COL-008 (batched Rust writer) — out of scope

**Artifact residual inventory** (which oracle site classes exist vs native paths):  
[REPORT_HTML_RESIDUAL_INVENTORY_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md).  
Lister: `bash tools/oracle/list_html_artifacts.sh`.

## Verification

| Gate | Command |
|------|---------|
| Schema + checklist (this file) | Read / review |
| Rust model + HTML render | `cargo test -p nytprof-report report_semantic_parity_default_calls1` |
| Operator smoke (oracle + native) | `bash tools/oracle/report_semantic_parity.sh` (or `chmod +x` then `./tools/oracle/report_semantic_parity.sh`) |

Evidence paths land on the first-slice board as `REPORT-SEMANTIC-PARITY`.

## Related board

For the **blocks** fixture path (`TIME_BLOCK` / A4 line calls **780** on blocks-calls1), see  
[blocks-semantic-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/blocks-semantic-parity-mvp-v0.md) (`BLOCKS-SEMANTIC-PARITY`).

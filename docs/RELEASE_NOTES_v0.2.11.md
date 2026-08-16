# NYTProfM v0.2.11 — flame graph on by default (oracle parity)

**Tag:** `v0.2.11`  
**Date:** 2026-08-15  
**Since:** [`v0.2.10`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.10)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-5** (upgrades `6.15-4`). **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA.

## The change

`nytprofm-cli html` now emits the **flame graph by default**, matching oracle `nytprofhtml`, whose `flame!` option defaults to 1 ([`baseline/6.15/src/bin/nytprofhtml`](https://github.com/hilather/nytprof-modernization/blob/v0.2.11/baseline/6.15/src/bin/nytprofhtml) line 107). Opt out with **`--no-flame`** — also the oracle spelling:

```bash
nytprofm-cli html nytprof.out --out-dir site          # flame included (new default)
nytprofm-cli html nytprof.out --out-dir site --no-flame
```

This supersedes the MVP's opt-in `--flame` design (KD-FLAME's "do not match 6.15 default-on" in [`OPERATOR_HTML_V2_DESIGN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.11/docs/OPERATOR_HTML_V2_DESIGN_v0.md) and the "Default off" rule in [`html-optional-flame-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.11/docs/schemas/html-optional-flame-mvp-v0.md) — both amended 2026-08-15). The **library** default is unchanged: `HtmlRenderOptions::default()` keeps `flame: false` for embedders; only the CLI flips.

Same-change hardening, now that flame is default-on:

- **No-calls profiles get no flame artifacts** — profiles with zero non-zero `call_edges` skip the flame files and index section entirely (oracle also skips flame when the profile has no calls data). No empty `all_stacks_by_time.svg`.
- Output-size impact is measured and bounded: on `fixtures/v5/default-calls1` the site grows 251,059 → 262,419 B (+4.5%); `index.html` +6,152 B from the inlined SVG (sub-pixel frames omitted, depth ≤ 16). Details in [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.11/docs/BENCH_NOTES.md).

## Changes since v0.2.10 (grouped)

**CLI behavior**

- `html`: flame default on; `--no-flame` opt-out; `--flame` still accepted (explicit on). Conflict error unchanged when both are passed.
- Help/usage text updated (`nytprof-cli --help`).

**Report engine**

- `render_html_site_with_options` / `render_html_summary_with_options`: flame artifacts skipped when `collect_nonzero_call_edges` is empty.

**Contracts / docs**

- `html-optional-flame-mvp-v0.md`: default-on amendment, `--no-flame` CLI block, no-calls skip rule, updated test table.
- `html-shared-css-structure-mvp-v0.md` scope note: the shared-CSS "no inline `<style>`" rule contracts the **page** stylesheet; the flame SVG's SVG-scoped `<style>` is separate (CLI shared-CSS suite now runs `--no-flame`).
- `OPERATOR_HTML_V2_DESIGN_v0.md` KD-FLAME superseded; ADR-0012 WAIVE list amended; residual inventory + R1 readiness matrix updated (A03 flame closed MVP, default-on CLI; `flamegraph.pl`/`nytprofcalls` multi-frame remains residual).

**Tests**

- `html_out_dir_default_writes_flame_files` (default-on), `html_out_dir_no_flame_writes_no_flame_files` (opt-out), `flame_skipped_when_no_call_edges` (report lib). Explicit `--flame` and single-file embed tests unchanged.
- Full `cargo test -p nytprof-cli` + `-p nytprof-report --lib` green; clippy `-D warnings`-clean on cli/report.

## Known residuals (unchanged)

- Oracle `flamegraph.pl` / `nytprofcalls` multi-frame stacks and `flamegraph_subattr.txt` remain residual; native flame is a call-tree SVG from `call_edges` (two-frame folded basis), not visual/byte parity with oracle.
- FFI surface, XS Data, full oracle DOM/tablesorter (WAIVE), COL-007 full C v6 writer, multi-OS product certification: see the [residual matrix](https://github.com/hilather/nytprof-modernization/blob/v0.2.11/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md).

## Upgrade notes

- `dnf upgrade` from 6.15-4 picks up the refreshed CLI. Existing scripts passing `--flame` keep working (explicit on == default); scripts that must not emit flame artifacts should pass `--no-flame`.
- Site output gains `all_stacks_by_time.svg` + `.folded` and an inlined flame section in `index.html` by default; automation asserting exact site file inventories should be updated or run with `--no-flame`.

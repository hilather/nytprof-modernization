# NYTProfM v0.2.7 — operator HTML v2 + live attach times

**Tag:** `v0.2.7`  
**Date:** 2026-08-15  
**Since:** [`v0.2.6`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.6)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-2** (upgrades `6.15-1`). **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.7/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.7/scripts/packaging/build_el8_module_rpm.sh).

Download `perl-NYTProfM-6.15-2.el8.x86_64.rpm` from this release. Install:

```text
sudo rpm -Uvh --replacefiles perl-NYTProfM-6.15-2.el8.x86_64.rpm
# --replacefiles overwrites stock /usr/bin/nytprofhtml if perl-Devel-NYTProf is present
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM your_script.pl
nytprofhtml /tmp/nytprof.out --out-dir /tmp/nytprof-html
# optional call-tree flame (off by default):
nytprof-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html --flame
```

Rocky 8 Docker lab (native, oracle 6.15, or both): [`scripts/field/rocky8_docker_profile_demo.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.7/scripts/field/rocky8_docker_profile_demo.sh).

## Changes since v0.2.6 (grouped)

**Native HTML (operator v2 — ADR-0012)**

- 6.15-like chrome / index IA / six-column source / compact `fmt_time` units
- Vanilla `nytprof-sort.js` (no jquery / tablesorter — M01 still **WAIVE**)
- Native `packages-callgraph.dot` / `subs-callgraph.dot` (source only; no Graphviz PNG)
- Source call-in/out from usable `call_sites` (omit stub `(1,1)`)

**Optional `--flame`**

- Call-tree SVG from `call_edges` (inclusive-time widths; not per-edge barcode columns)
- Inlined on `index.html` so `file://` works
- Hover tooltip (calls + incl/excl); click a frame to `file-{fid}.html#L{line}`
- Still **opt-in**; not oracle `flamegraph.pl` / `nytprofcalls`

**Collection (`perl -d:NYTProfM`)**

- `nytp_clock_now` (`CLOCK_MONOTONIC`, 10M ticks/s) + last-site `TIME_LINE` / `TIME_BLOCK`
- Real `SUB_CALLERS` fid/line (not hardcoded `1,1`)
- Parent exclusive = incl − pending child excl (PRINT/MATCH fold into tokenize)
- `INIT` `$DB::single` + `goto &$raw` for Getopt / Exporter / `vars` / `constant` / `overload`
- Smokes: `g07` Getopt compile, `g08` slowops times, `g09` tokenize exclusive split

**Field / compare**

- Dual-container Rocky lab (`--engine native|oracle|both`); migrate-then-link HTML dirs
- `compare_oracle_native_reports.sh` — same scanner, seconds, and corpus
- Agent hints: apples-to-apples compare, then remaining wall-time gap is 6.15 overhead (fewer passes)

## Residuals (do not claim)

Full 6.15 opcode / DOM/jquery/tablesorter (**WAIVE**) / COMPAT-007 Data / COL-007 C v6 writer / `_exit` flush / default-on flame / Graphviz image render / multi-frame `nytprofcalls` / PAUSE / signed COPR / default Rocky `format=v6` / public perf certification.

## Docs

- [OPERATOR_HTML_V2_DESIGN_v0.md](https://github.com/hilather/nytprof-modernization/blob/v0.2.7/docs/OPERATOR_HTML_V2_DESIGN_v0.md)
- [html-operator-v2-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/v0.2.7/docs/schemas/html-operator-v2-mvp-v0.md)
- [html-optional-flame-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/v0.2.7/docs/schemas/html-optional-flame-mvp-v0.md)
- [rocky8-docker-profile-lab-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/v0.2.7/docs/schemas/rocky8-docker-profile-lab-mvp-v0.md)
- [R1_PREVIEW_OPERATOR_RUNBOOK.md](https://github.com/hilather/nytprof-modernization/blob/v0.2.7/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md)
- [perl-NYTProfM.spec](https://github.com/hilather/nytprof-modernization/blob/v0.2.7/packaging/rpm/perl-NYTProfM.spec)

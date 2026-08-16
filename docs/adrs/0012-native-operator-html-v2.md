# ADR-0012 - Native operator HTML v2 (oracle look/feel/nav; modern JS/CSS)

- **Status:** **accepted (policy)** — implementation lands as HTML-OP-V2 (chrome / IA / sort / time / source columns / dual-docker lab)
- **Date:** 2026-08-15
- **Accepted:** 2026-08-15 (user request after a live 6.15 vs native Rocky comparison)
- **Owners/approvers:** report / field-lab leads; architecture review group
- **Related ADR-Q:** does **not** un-waive [ADR-0003](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) Amendment 2026-08-12 (M01/Q4 jquery / tablesorter / floatThead)
- **Related tasks/risks/gates:** design [`docs/OPERATOR_HTML_V2_DESIGN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/OPERATOR_HTML_V2_DESIGN_v0.md); inventory [`docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md); prior class [ADR-0011](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0011-native-operator-html-v1.md)
- **Decision scope/version:** advertise a **new** native HTML class. Not oracle DOM parity. Not a `collection_default` / `engine=auto` flip. Not COL-007.

## Context

ADR-0011 closed a usefulness gap (zero times, empty source, no heat, no sort). A live 6.15 `nytprofhtml` site of `scripts/field/workloads/minute_text_scanner.pl` (25s, isolated pin, **no `crates/` on PERL5LIB**) is a **navigable product**: blue header, `← Index`, top-15 exclusive sub table (Calls / P / F / Exclusive / Inclusive / Subroutine), “See all N” → `index-subs-excl.html`, files table with `line` links, six-column source. Native v1 is useful but a different IA (`source.html` pointed at `warnings.pm`, name-sorted 4-col tables, file `<ul>`, first-click sort ascending). Operators who know 6.15 reports do not feel at home.

jquery / tablesorter stay **WAIVE** (M01). Live 6.15 even references `jquery.floatThead.min.js` without shipping it (404).

## Evidence

| Item | Path / note |
|------|-------------|
| Oracle HTML (this pass) | `/home/mbrewer/Downloads/nytprof-oracle-6.15-scanner/` — `source tools/oracle/env.sh`; `perl -d:NYTProf`; **oracle** `nytprofhtml` |
| Native HTML v1 | `/home/mbrewer/Downloads/nytprof-rocky8-demo/html/` (60s; look/feel only — durations differ) |
| Isolation | PERL5LIB = `baseline/6.15/{test-deps,install}` only |
| Design | [`docs/OPERATOR_HTML_V2_DESIGN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/OPERATOR_HTML_V2_DESIGN_v0.md) |

## Decision

1. **CLOSE** a new class **Native operator HTML v2**: semantic 6.15 **information architecture and chrome** implemented with CSS custom properties and extended vanilla `nytprof-sort.js`.
2. **Keep native filenames** (`file-<fid>.html`, `source.html`, `index-subs-excl.html`). Do not emit oracle `{safe}-{fid}-line.html`.
3. **Leave WAIVE:** jquery, tablesorter, floatThead, Graphviz, treemap, JIT, block/sub page modes, default-on flame, pixel-identical stacked-div header. **Amendment 2026-08-15:** default-on flame is un-WAIVEd by user direction for oracle parity — CLI `html` now emits flame unless `--no-flame` (see `docs/schemas/html-optional-flame-mvp-v0.md` amendment; `nytprofcalls` multi-frame stacks remain residual).
4. **Capability JSON** must not emit `tablesorter: true`.
5. **`source.html` is the application script** (KD-PRIMARY), not the minimum fid with `source_lines`.
6. **Call-in/out** is optional (PR-7) and must treat product `(fid,line)==(1,1)` as unusable unless `sub_def` starts there. Calls / Time-in-subs stay `—`, never `0`.
7. **Dual-container Rocky lab** (`--engine native|oracle|both`) with KD-LAYOUT migrate-then-link. Oracle container builds 6.15 from the committed archive; never mount repo root onto oracle `PERL5LIB`; never `source tools/oracle/env.sh` in docker.

## Exactness and compatibility consequences

Text, CSV, and `report --json` stay **integer ticks**. HTML display copies 6.15 `fmt_time` compact units (`ns` / `µs` / `ms` / `s`). Heat class **names** stay `heat-*` (no `.c0`–`.c3` in markup). v1 greps for `time_line_events`, `href="source.html"`, 15/3 on default-calls1 index, and `nytprof-sort.js` remain.

## Alternatives considered

| Alternative | Reason accepted/rejected |
|-------------|--------------------------|
| Un-waive M01 and vendor jquery/tablesorter | Rejected — ADR-0003; floatThead is not even shipped by 6.15 |
| Pixel-identical / XHTML / 51-div gradient | Rejected — charter + ADR-0011; one CSS gradient is enough |
| Oracle `{safe}-{fid}-line.html` filenames | Rejected — inventory WAIVE; keep `file-<fid>.html` |
| Single container for oracle + native | Rejected — isolation leak (`crates/` on oracle PERL5LIB) |
| Copy host `baseline/6.15/install` into Rocky | Rejected — glibc-incompatible `.so` |

## Implementation and testing requirements

See [`docs/OPERATOR_HTML_V2_DESIGN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/OPERATOR_HTML_V2_DESIGN_v0.md) Key Decisions and PR Plan. Tests drive real `nytprof-cli html` / `nytprof-dump html`. Rocky smoke is honest-SKIP, not in `offline_gate`.

## Migration, rollout, and rollback

No product default flip. Testdrive `nytprofhtml` already dispatches native. Revert the implementing change to roll back HTML/JS. Dual-lab `--engine native` stays the operator default.

## Revisit triggers

User request for jquery/tablesorter; product attach stops stubbing `emit_sub_callers(1,1,…)`; COMPAT-003 freeze of HTML display units.

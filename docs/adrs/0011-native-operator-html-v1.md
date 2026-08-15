# ADR-0011 - Native operator HTML v1 (heat, links, vanilla sort)

- **Status:** **accepted (policy)** — implementation lands in follow-up PRs (PR-2..PR-6 of the live-metrics / HTML program)
- **Date:** 2026-08-14
- **Accepted:** 2026-08-14 (user override after Rocky 8 testdrive report inspection)
- **Owners/approvers:** report / collector leads; architecture review group
- **Related ADR-Q:** does **not** un-waive [ADR-0003](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) Amendment 2026-08-12 (M01/Q4 jquery / tablesorter / floatThead)
- **Related tasks/risks/gates:** design [`docs/OPERATOR_HTML_AND_LIVE_METRICS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/OPERATOR_HTML_AND_LIVE_METRICS_v0.md); inventory [`docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md)
- **Decision scope/version:** advertise a **new** native HTML class. Not oracle DOM parity. Not a `collection_default` / `engine=auto` flip.

## Context

A Rocky 8 testdrive `nytprofhtml` site showed all `incl`/`excl` as 0, visit-count statement ticks, empty `file-1.html`, and MVP tables without heat or sub→source links. The user asked to complete report metrics and DOM/JS.

ADR-0003 Amendment 2026-08-12 **WAIVE**s jquery / tablesorter / floatThead for GA-candidate. Superseding that WAIVE requires a new ADR. This ADR **does not** rewrite that waiver.

## Decision

1. **CLOSE** a new class **Native operator HTML v1**: heat CSS (`heat-hot|high|mid|low`), vanilla `nytprof-sort.js`, sub→source `#Ln` links, HTML-only seconds display (`title=` raw ticks).
2. **Leave WAIVE:** jquery, tablesorter, floatThead, Graphviz, treemap, oracle `get_css()` / block-sub pages.
3. **Capability JSON** must not emit `tablesorter: true`.
4. Live **metrics** (incl/excl, elapsed `TIME_LINE`) are a collector completeness fix under existing D1/D3 and **do not** require this ADR. They land independently (PR-1).
5. A03 optional `--flame` stays opt-in MVP; do not re-implement.

## Exactness and compatibility consequences

Text, CSV, and `report --json` stay **integer ticks**. Only HTML converts ticks to seconds for display. Oracle golden fixtures are unchanged.

## Alternatives considered

| Alternative | Reason accepted/rejected |
|-------------|--------------------------|
| Un-waive M01 and vendor jquery/tablesorter | Rejected — larger XSS/supply surface; ADR-0003 forbids silent rewrite |
| Derive excl from line ticks only | Rejected — wrong exclusive math vs 6.15 `incr_sub_inclusive_time` |
| Pixel-identical oracle DOM | Rejected — user asked for useful HTML, not clone |

## Implementation and testing requirements

See [`docs/OPERATOR_HTML_AND_LIVE_METRICS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/OPERATOR_HTML_AND_LIVE_METRICS_v0.md) PR Plan (PR-0 this ADR; PR-1 metrics; PR-2 HTML seconds + source union; PR-4 heat/links; PR-5 sort JS). Tests drive `nytprof-cli html` and Rocky `--lab`, not invented fixture constants.

## Migration, rollout, and rollback

No product default flip. Testdrive `nytprofhtml` already dispatches native. Revert the implementing PR to roll back HTML/JS. Live times roll back by reverting PR-1.

## Revisit triggers

User request for jquery/tablesorter; COMPAT-003 freeze of HTML display units; Rocky 8 lab showing remaining empty source after PR-3.

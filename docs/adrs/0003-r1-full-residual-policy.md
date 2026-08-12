# ADR-0003 — Full R1 residual close-or-waive policy (HTML map + OQ-2)

- **Status:** accepted
- **Date:** 2026-08-11
- **Owners/approvers:** program completion plan (PLAN_ID `8c9b1a63`); user-resolved **OQ-2**
- **Related ADR-Q:** ADR-Q018 (FFI vs subprocess — product direction fixed for full R1); ADR-Q020 (report compatibility threshold — HTML classes mapped here)
- **Related tasks/risks/gates:** RUST-010, PERL-004, PERL-005, REPORT-001..020, REPORT-HTML-RESIDUAL-INV, BUILD-003, BUILD-006, WP-13 / BENCH-*; Phase A PRs **PR-A01..PR-A10**
- **Decision scope/version:** full product **R1** residual disposition only (not R2 COL-007 / wire freeze; not R3/R4 default flips)

---

## Context

Offline **R0 / R1-preview** freezes what is advertised ready versus residual in:

- [`docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md)
- [`docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md)
- [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md)

Full product **R1** requires every residual row to be either **closed with implementation evidence** or **explicitly waived** with user-facing residual honesty. Without a binding map:

- HTML work can expand indefinitely without a “full R1 HTML posture” boundary;
- FFI / XS rows risk being waived by default when the product decision is to **implement**;
- **PR-A10** (full R1 readiness cut) cannot advertise full-R1 HTML posture honestly.

User **OQ-2** (resolved): **close both** production FFI (`nytprof-ffi`) and XS Data / ReadStream product paths. Do **not** waive those rows for full R1.

Numbering coordination (PLAN `8c9b1a63`):

| Number | Owner | Topic |
|--------|-------|--------|
| **0001** / **0002** | **PR-B01** | v6 packing / FOOTER string-pool candidates (OQ-1) |
| **0003** | **PR-A04** (this ADR) | Full R1 residual close-or-waive policy |
| **0004** | **PR-B00** | Collector packaging / source-tree (file lands as `0004-collector-packaging-source-tree.md` when B00 renumbers) |

Do **not** reuse 0003 for packaging or packing ADRs.

---

## Evidence

| Source | Role |
|--------|------|
| Program completion design (Phase A, OQ-2) | Close-or-waive strategy; HTML slice order; FFI/XS must close via PR-A05/A06 |
| HTML residual inventory v0 | Artifact classes on `fixtures/v5/default-calls1` (oracle `nytprofhtml` vs native) |
| Residual readiness matrix v0 | Full-R1 residual table (FFI, XS, HTML, BUILD-003/006, perf, R3/R4 out-of-scope rows) |
| REPORT_SURFACE_CONTRACT | Advertised MVP HTML; not-advertised oracle DOM list |
| User OQ-2 | Close FFI + XS (implement); not waive |

This ADR does **not** close residuals by itself — it freezes **disposition**. Closing still requires the named PR + tests + matrix/inventory honesty updates.

---

## Decision

### Disposition vocabulary

| Disposition | Meaning |
|-------------|---------|
| **CLOSE** | Must be implemented before PR-A10 may claim the residual/class closed. Named close PR(s) are binding for Phase A. |
| **WAIVE** | Explicitly **not** required for full R1 product claim. Legacy oracle tools may retain the capability. Residual matrix / inventory remain honest; PR-A10 must not advertise the waived class as native-ready. |
| **done** | Already advertised ready under offline R0 / R1-preview (no further Phase A close work for this class/path). Prefer this token over “CLOSE (done)” so “CLOSE” always means remaining implement work. |
| **N/A** | Not an oracle gap (e.g. native-only convenience). Not something to close or waive against `nytprofhtml`. |
| **OUT-OF-R1** | Tracked elsewhere (R2+ collector/format, R3/R4 defaults). Not a Phase A close/waive choice. |

### OQ-2 — FFI and XS (binding)

| Residual | Plan refs | Disposition | Close PR | Waiver forbidden? |
|----------|-----------|-------------|----------|-------------------|
| No production C ABI / FFI / cdylib | **RUST-010**, `nytprof-ffi` | **CLOSE** | **PR-A05** | **Yes** — do not waive for full R1 |
| No XS ReadStream over binary profiles | **PERL-004** | **CLOSE** | **PR-A06** | **Yes** — do not waive for full R1 |
| No XS / bless-array Data materializer | **PERL-005** (+ COMPAT-007) | **CLOSE** | **PR-A06** | **Yes** — do not waive; COMPAT-007 shapes only with tests |

Product sketch (not implementation detail of this ADR):

```text
PR-A05: nytprof-ffi cdylib → decode/query C ABI over ProfileModel (panic-safe)
PR-A06: XS Data / ReadStream over binary profiles (may use FFI or pure-XS);
        pure-Perl JsonlData remains available bridge
```

### Non-HTML full-R1 residuals (close or waive)

| Residual | Plan refs | Disposition | Close PR / note |
|----------|-----------|-------------|-----------------|
| No full nytprofhtml DOM / REPORT-001..020 | REPORT-001..020, **REPORT-HTML-RESIDUAL-INV** | **Per HTML map below** | See § HTML residual class map; not a single blanket close |
| No full MakeMaker XS dual-build CPAN | **BUILD-003** | **CLOSE** (preferred) | **PR-A08** — packaging depth toward BUILD-003; legacy-only unbroken. Acceptable future waiver only via superseding ADR + public residual note. |
| No multi-OS CI matrix | **BUILD-006** | **CLOSE** (preferred) | **PR-A07** — ≥1 additional OS/arch MVP; honest skips preserved. Exact matrix still open (**OQ-3**). Acceptable future waiver only via superseding ADR. |
| No performance certification claims | WP-13 / BENCH-* | **CLOSE or WAIVE** | **PR-A09** if publishing R1-scoped P3/P4 claims; otherwise **WAIVE** (light_bench / `docs/BENCH_NOTES.md` only — no public SLOs). Default full-R1 posture: **WAIVE** public perf claims unless A09 lands green. |
| No v6 wire freeze | format plan / Phase-0 | **OUT-OF-R1** | R2 wire freeze ADR after E3/E4 |
| COL-007 / COL-008 deferred | COL-007, COL-008 | **OUT-OF-R1** | R2 critical path; preflight is not product writer |
| `engine=auto` product default flip | charter **R3** | **OUT-OF-R1** | Phase D |
| Default engine/format flips | charter R3/R4 | **OUT-OF-R1** | Phase D/E |

### HTML residual class map (binding for PR-A10 HTML posture)

Every artifact class from [`REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md) maps to **CLOSE**, **WAIVE**, **done**, or **N/A**. First-column labels match the inventory class names exactly. Semantic counts (leaf **15** / mid **3** / mid→leaf **15** on default-calls1) are **already advertised** and are not residuals.

| Artifact class | Inventory residual? | Disposition | Close PR or waive note |
|----------------|---------------------|-------------|------------------------|
| Index / home page | partial | **CLOSE** | **PR-A01** — structure/CSS depth; counts already ready — **closed (MVP)** at PR-A10 |
| Full sub index (excl sort) | partial | **CLOSE** | **PR-A02** (`index-subs-excl.html`) — **closed (MVP)** at PR-A10 (not oracle DOM) |
| Exclusive-time ranking | partial | **CLOSE** | **PR-A02** (full excl page) — **closed (MVP)** at PR-A10; index “Top exclusive” section already MVP |
| Per-file / line source pages | partial | **WAIVE** | Naming alias: keep permanent native `file-<fid>.html` + `source.html` (not oracle `{safe}-{fid}-line.html`) |
| Block-level report pages | yes / partial | **WAIVE** | No oracle `*-block.html` page mode; native A4b block_line table remains MVP when present |
| Sub-level report pages | yes | **WAIVE** | No `*-sub.html`; legacy `nytprofhtml` retains sub page mode |
| Shared CSS | yes | **CLOSE** | **PR-A01** (`style.css` or documented equivalent) — **closed (MVP)** at PR-A10 |
| Shared JS (jquery / tablesorter / floatThead) | yes | **CLOSE** | **PR-A01** mapped both CSS and JS; **A01 shipped CSS/structure only**. Shared JS/tablesorter remains **OPEN residual (CLOSE path)** at PR-A10 — **not** silently waived. Follow-on close PR or superseding ADR required before claiming closed |
| JIT / treemap assets | yes | **WAIVE** | `js/jit/*` |
| Treemap HTML page | yes | **WAIVE** | `subs-treemap-excl.html` |
| Flame graph SVG | yes | **CLOSE** | **PR-A03** — optional `--flame`; no default site bloat (`all_stacks_by_time.svg`) |
| Call-stack flame inputs | yes | **CLOSE** | **PR-A03** (site `.calls` / `flamegraph_subattr.txt`); native `folded` remains related export outside HTML |
| Packages call graph (Graphviz) | yes | **WAIVE** | Waive-by-default unless later demand + new ADR/PR |
| Subs call graph (Graphviz) | yes | **WAIVE** | same |
| Per-file call graph `.dot` | yes | **WAIVE** | same |
| Call-edges table (caller/called/count) | partial | **WAIVE** | Oracle presentation residual; semantic counts already advertised |
| Subroutine returns table | partial | **WAIVE** | Oracle tablesorter chrome residual; counts advertised |
| Source line table (A4) | partial | **WAIVE** | Oracle DOM residual; MVP tables advertised |
| A4b block_line totals | partial | **WAIVE** | Presentation beyond MVP table residual |
| Multi-file site directory publish | no | **done** | Already advertised (`html --out-dir`) |
| Single self-contained HTML | no (native-only) | **N/A** | Native convenience; not an oracle gap to close |
| Browser open helper | yes | **WAIVE** | Oracle `--open` |
| Delete-out-dir flag | partial | **WAIVE** | Exact `-d` / `--delete`; atomic out-dir overwrite accepted |
| Eval merge UI / `--mergeevals` | yes | **WAIVE** | |
| Footer / version branding | partial | **WAIVE** | Oracle Devel::NYTProf footer; native titles sufficient for full R1 |

**PR-A10 rule:** full R1 HTML posture may be claimed only when every class above is either **closed with evidence** (CLOSE rows + tests + inventory flip) or still listed as **WAIVE** with residual honesty. PR-A10 must **not** claim full oracle `nytprofhtml` DOM.

### Close-PR roll-up (Phase A)

| PR | Role |
|----|------|
| **PR-A01** | Shared CSS + structure (**closed MVP**); Shared JS/tablesorter remaining OPEN CLOSE residual |
| **PR-A02** | `index-subs-excl.html` + exclusive ranking page depth |
| **PR-A03** | Optional flame path (`--flame`) |
| **PR-A04** | **This ADR** + matrix / inventory / runbook map (policy only) |
| **PR-A05** | FFI cdylib product path (**OQ-2 CLOSE**) |
| **PR-A06** | XS Data / ReadStream product path (**OQ-2 CLOSE**) |
| **PR-A07** | Multi-OS CI matrix (BUILD-006 MVP) |
| **PR-A08** | Packaging depth toward BUILD-003 |
| **PR-A09** | Optional R1-scoped perf certification (else waive public claims) |
| **PR-A10** | Full R1 readiness cut (matrix + release notes). Depends on: **A04 map** (this ADR); **required CLOSE** **A05/A06** (OQ-2 FFI/XS); **preferred CLOSE** **A07/A08** (BUILD-006 / BUILD-003) **or** a superseding waiver ADR; **A01–A03** CLOSE evidence for HTML classes marked CLOSE (plus inventory flip); **A09** optional (else **WAIVE** public perf claims). Must not claim full oracle DOM, COL-007, wire freeze, or R3/R4 defaults. |

---

## Exactness and compatibility consequences

| Area | Effect |
|------|--------|
| Offline R0 / R1-preview | **Unchanged** — still no production FFI/XS, no full DOM, no multi-OS CI, no perf claims until close PRs land and honesty docs flip |
| Full R1 product claim | Requires CLOSE rows implemented (incl. **OQ-2** FFI+XS) and WAIVE rows explicitly residual |
| Semantic counts | Still exact on advertised surfaces; COMPAT-003 for ticks/times |
| COMPAT-007 bless-array shapes | Only claimable when PR-A06 lands dual-engine tests; JsonlData remains bridge |
| Legacy-only installs | Must keep working without Cargo / without loading `nytprof-ffi` |
| Oracle isolation | Never put `crates/` on oracle `PERL5LIB` |
| R2+ | COL-007, wire freeze, CLI v6 default remain **OUT-OF-R1** |

---

## Alternatives considered

| Alternative | Correctness/compatibility | Performance/storage | Security/reliability | Build/portability | Reason accepted/rejected |
|-------------|---------------------------|---------------------|----------------------|-------------------|--------------------------|
| Waive FFI + XS for full R1 (CLI + JsonlData only) | Preview-compatible; weaker embed story | Smaller packaging surface | Isolation via subprocess | Simpler dual-path | **Rejected** — user **OQ-2** requires close |
| Close full oracle HTML DOM (all classes) | Highest oracle visual parity | Large CSS/JS/Graphviz/treemap surface | Asset/path risk | Heavier report crate | **Rejected** for full R1 — high-value CLOSE only; Graphviz/treemap/block-sub waived |
| Waive multi-OS CI and BUILD-003 packaging depth | Leaves single-host / facade-only | n/a | Weaker release confidence | Weakest portability claim | **Rejected as default** — preferred CLOSE via A07/A08; waiver only via later superseding ADR |
| Require public perf certification for full R1 | Stronger marketing claims | Bench cost | Misclaim risk if light_bench only | Host variance | **Rejected as default** — CLOSE only if A09 certifies; else WAIVE public claims |

---

## Implementation and testing requirements

1. **This PR (A04):** land this ADR; sync residual matrix, HTML inventory, and operator runbook disposition columns/sections; update ADR index.
2. **Close PRs (A01–A03, A05–A09):** each CLOSE row needs schema/contract update where applicable, regression tests driving real entry points, and inventory/matrix honesty flip when the class becomes advertised.
3. **PR-A10:** may claim full R1 only when residual table rows are closed or waived per this ADR; must not claim COL-007 / wire freeze / R3–R4 defaults; release notes list deltas + residual honesty.
4. **OQ-2 tests:** PR-A05 panic-safe ABI + dual-path without dylib; PR-A06 binary ReadStream/Data path + JsonlData still available; no COMPAT-007 claim without tests.
5. Gates: keep `./scripts/ci/offline_gate.sh` green for report/HTML slices; multi-OS expands gate via A07.

---

## Migration, rollout, and rollback

| Topic | Policy |
|-------|--------|
| Preview operators | No behavior change until close PRs ship |
| Capability / docs | Do not advertise FFI/XS/full DOM until close evidence + honesty sync |
| Rollback | Revert a close PR → restore residual **yes** / preview honesty; this ADR remains the map |
| Supersede | New ADR required to flip a **WAIVE** to **CLOSE** (or to waive a preferred CLOSE row such as BUILD-006) |

---

## Revisit triggers

- Maintainer demand for Graphviz / treemap / block-sub page modes → new close PR + inventory flip (supersede waive rows).
- OQ-3 matrix decision changes BUILD-006 scope.
- Evidence that COMPAT-007 full bless-array fidelity is infeasible on schedule → escalate; do **not** silently waive OQ-2 without a superseding ADR and explicit product note.
- Field evidence that CLI-only embed is sufficient long-term → superseding ADR only (cannot silently reverse OQ-2).
- Perf certification methodology ready → land PR-A09 and flip perf residual from waive to closed claims.

---

## Normative doc pointers

| Doc | Update expectation |
|-----|--------------------|
| [`R1_RESIDUAL_READINESS_MATRIX_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) | Disposition column / policy section referencing this ADR |
| [`REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md) | Per-class close PR / waive map |
| [`R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) | Operator-facing residual disposition + OQ-2 note |
| [`docs/adrs/README.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/README.md) | Index entry |

---

## Board / plan placement

| ID | Status | Evidence |
|----|--------|----------|
| `R1-RESIDUAL-POLICY-ADR` (PR-A04) | **done** (policy) | this ADR |
| `REPORT-HTML-RESIDUAL-INV` | done (inventory); map extended by A04 | inventory + this ADR |
| `R1-RESIDUAL-MATRIX` | done (preview freeze); disposition extended by A04 | matrix + this ADR |
| COL-007 | deferred | OUT-OF-R1 |

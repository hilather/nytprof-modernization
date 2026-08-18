# catalog-method-times

**Date:** 2026-08-18  
**Area:** report / perl / field  
**Related:** KD-35, E4 `slowops=full`, g09 / g14 exclusive split, catalog top-10 labs

## Attempt / misunderstanding

Paired oracle 6.15 vs native NYTProfM on catalog labs looked like “some methods have the wrong exclusive time.” The Aug 15 scanner `scan_file` grandchild leak is **gone**. What remains is attribution policy, not a unit/clock bug.

## Evidence (commands, numbers, failing tests)

Same host, same driver, same `NYTPROF_DEMO_SECONDS`, isolated oracle `PERL5LIB` (never `crates/`). Native default opcode + **then-default** `slowops=2` (PRINT/MATCH only — pre-flip). Times from native `nytprof-cli csv --subs` (ticks → seconds via `ticks_per_sec=10000000`). Scale exclusive by **passes**, not wall. **2026-08-18:** product default now installs the full 6.15 table so `CORE:open`/`readline`/`subst` appear without `slowops=full`.

| App | secs | oracle passes | native passes | work n/o | Hottest “off” row |
|-----|------|---------------|---------------|----------|-------------------|
| scanner | 8 | 195 | 653 | **3.35×** | `scan_file` excl 18ms vs 178ms (3.0×/pass) — native missing `CORE:open`/`readline`/`close` |
| html_tree | 5 | 764 | 1126 | 1.47× | workload methods present; `CORE:subst` 18336× only on oracle (16ms) |
| ppi | 4 | 263 | 270 | 1.03× | `PPI::Lexer::CORE:match` 11ms/66ms vs 93ms/93ms (**8.6×/pass**) |
| mojolicious | 4 | 601 | 722 | 1.20× | 1745 vs 121 named subs (BEGIN/import + extra CORE:*) |
| template_toolkit | 4 | 952 | 1325 | 1.39× | `Parser::_parse` 0.36s vs 1.47s (2.9×/pass) — many small ANON calls |
| json_xs | 4 | 216943 | 306295 | 1.41× | encode/decode **1.22–1.25×/pass** (aligned) |
| dbi_sqlite | 4 | 8182 | 9515 | 1.16× | `DBI::st::execute` **1.07×/pass** (aligned) |
| csv_xs | 4 | 59122 | 69694 | 1.18× | Perl wrappers 1.6–3.5×/pass; XSUB print closer |

PPI edge (the match child 6.15 records and native does not):

```
oracle: PPI::Lexer::CORE:match → PPI::Node::content   n=1315  incl=0.056s excl=0.022s
oracle: PPI::Lexer::_continues → PPI::Lexer::CORE:match n=1315  incl=0.065s excl=0.009s
native: PPI::Lexer::_continues → PPI::Lexer::CORE:match n=1350  incl=excl=0.092s
        (no match → content edge)
```

Control: native scanner `NYTPROF=…:slowops=full` (3s) emits `CORE:open` / `readline` / `close` / `sort` / `readdir` and `scan_file` exclusive shrinks back to a remainder (31ms incl 1.86s).

## Outcome

corrected understanding | **default flipped** to the 6.15 full table (2026-08-18) | exclusive still thin; not a public perf claim

## Do not retry without

- Do **not** treat mixed-duration HTML or unscaled exclusive seconds as a native clock bug (`AGENTS.md` §5).
- Do **not** revert default to PRINT/MATCH-only without a new operator reason — that is what made method lists look “off.”
- Do **not** claim 6.15 exclusive on thin MATCH/full-table when the opcode re-enters Perl.
- Exclusive shape check: `tokenize` / `scan_file` remainder (g09 / g14) — still the right smoke; it does not cover PPI match-as-parent.

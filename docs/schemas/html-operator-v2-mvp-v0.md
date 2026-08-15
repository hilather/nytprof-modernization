# HTML operator v2 MVP (chrome / IA / sort / compact time)

**Status:** MVP contract for **Native operator HTML v2** (ADR-0012)  
**Does not close:** M01 jquery / tablesorter (still **WAIVE**)  
**Design:** [OPERATOR_HTML_V2_DESIGN_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/OPERATOR_HTML_V2_DESIGN_v0.md)  
**Prior class:** [html-shared-css-structure-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md), [html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md)

## Must (multi-file `--out-dir`)

| Marker | Rule |
|--------|------|
| Chrome | `div.header` / `.siteTitle` / `.siteSubtitle` / `.header_back` on inner pages (`← Index` or `&larr; Index`) |
| Index title copy | `Performance Profile Index` (as `.siteTitle` or greppable) |
| Index first screen | `div.index_summary` → (optional `--flame`) → `table#subs_table` top **15** exclusive-desc → “See all N” → `table#filestable` → `href="source.html"` |
| Sub table columns | Calls, P, F, Exclusive Time, Inclusive Time, Subroutine |
| P (v2a) | Distinct caller **names** from `call_edges` (not oracle Places) |
| F (v2a) | Distinct caller `sub_defs.fid` (1 if unknown) |
| Event counts | `time_line_events` **must** remain on `index.html` **below** `#filestable` |
| Files | `#filestable` with Reports `line` → `file-<fid>.html`; keep `href="source.html"` |
| Sort | `nytprof-sort.js`; first-click **num desc**; `data-sort-default="desc"` on excl-index / file `#subs_table` / `#filestable` only — **not** index `#subs_table` |
| Time | 6.15 `fmt_time` compact units; `title="{ticks} ticks"` |
| Heat | `heat-hot\|high\|mid\|low` class names only; **no** `.c0`–`.c3` in markup; unused/zero source rows uncolored |
| jquery | **must not** appear |
| Graphviz | `packages-callgraph.dot` + `subs-callgraph.dot` from `call_edges`; index links both; real `digraph` with `->` (no `dot` PNG) |
| `source.html` | Application fid (attributes `application` basename, else hottest non-`@INC` `.pl`) |

## Source pages

| Column | Content |
|--------|---------|
| Line | `id="L{n}"` |
| Statements | `line_totals.calls` or `—` |
| Time on line | compact time or `—` |
| Calls | usable outgoing sites only; else **`—`** (never `0`) |
| Time in subs | usable outgoing incl; else **`—`** |
| Code | escaped source, class `.s` |

Opcode stub rows: `id="{pkg}__{sub}"` e.g. `main__CORE_match` for `CORE:` names without `sub_def`.

## Dual lab

`--engine native|oracle|both` (default `native`). `--engine both`: migrate-then-link so `$OUT/html` is a **symlink** to `native/html`. Oracle container: archive + scanner mounts only; fail closed if `PERL5LIB` contains `crates` or `baseline/6.15/install`.

## Honest residuals

jquery/tablesorter (M01 **WAIVE**), Graphviz **PNG/SVG** (`.dot` source shipped), treemap, block/sub pages, default-on flame, MAD heat, COL-007. Product `SUB_CALLERS` `(1,1)` stubs stay omitted in HTML. Parent exclusive subtracts slowop children (`g09_tokenize_excl_smoke.sh`).

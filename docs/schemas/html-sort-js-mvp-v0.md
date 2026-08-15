# HTML vanilla sort JS (operator v1 / MVP v0)

**Status:** implemented (operator HTML v1 PR-5 + v2 first-click desc / `data-sort-default`)  
**Does not close:** oracle Shared JS (jquery / tablesorter / floatThead) — those stay **WAIVE**  
**Complements:** [html-shared-css-structure-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md), [html-multifile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-multifile-mvp-v0.md)

## Policy

Native reports ship a small **in-crate** script (`nytprof_report::SHARED_SORT_JS`, ~2–4 KB):

| Surface | How JS is loaded |
|---------|------------------|
| Multi-file (`html --out-dir`) | Sibling **`nytprof-sort.js`**; every HTML page includes `<script src="nytprof-sort.js" defer></script>` |
| Single-file (`html` stdout / `-o`) | The **same** source inlined in one `<script>` (self-contained, parallel to CSS) |

This is **not** jquery, tablesorter, floatThead, or a CDN. Do not add a `tablesorter` capability key.

## Behavior

- On `DOMContentLoaded` (`nytprofSortInit`), bind every `table` that has `th[data-sort]` (advertised tables also have `class="sortable"`).
- **First click** on `data-sort="num"` is **descending**; first click on `data-sort="text"` is ascending. Toggle thereafter.
- On init, if a `th` has `data-sort-default="desc"` or `"asc"`, apply that sort once after bind. Used on exclusive-index / file `#subs_table` and `#filestable`. **Do not** put `data-sort-default` on the **index** `#subs_table` (already emitted exclusive-desc + truncated).
- Set `aria-sort="ascending|descending|none"` on headers.
- Click a header: reorder existing `tbody tr` nodes only (stable-ish via original index). Toggle `sort-asc` / `sort-desc` on the `th`.
- Numeric sort reads `data-sort-value` (raw integer ticks or counts — **not** the displayed `ms`/`µs` string). Text columns fall back to cell text.
- **XSS:** never assign `innerHTML` from profile strings; never rewrite cell text.

## Markup contract

- `th data-sort="text"` for names / source; `th data-sort="num"` for counts and times.
- Numeric `td`: `data-sort-value="{raw_ticks_or_count}"`.
- Time cells may display compact units (`4.72s`, `129ms`, `49µs`) with `title="{ticks} ticks"`; sort still uses the raw tick integer.
- Exclusive Time headers: `<th data-sort="num" data-sort-default="desc">Exclusive<br>Time</th>` (excl-index / file / `#filestable` only).

## Library / CLI

| API | Behavior |
|-----|----------|
| `SHARED_SORT_JS` / `SORT_JS_FILENAME` | Public constant + `"nytprof-sort.js"` |
| `HtmlSite.sort_js` / `sort_js_filename` | Body + name |
| `write_html_site` | Publishes the file atomically with the rest of the site |
| `nytprof-cli html … --out-dir DIR` | Writes `DIR/nytprof-sort.js`; stderr lists it |

## Tests

| Test | Asserts |
|------|---------|
| `shared_style_and_sort_js_contract` | Constant contains `nytprofSortInit` / `data-sort`; no jquery / tablesorter / `innerHTML` |
| CLI `html_operator_v1_cli_default_calls1` | File exists, is referenced with `defer`, no jquery / tablesorter; **15/3** still greppable |

## Residual honesty

jquery / tablesorter / floatThead remain **WAIVE**. This class is native vanilla sort only.

# HTML report MVP (v0)

**Status:** first-slice minimal native HTML  
**Not:** full nytprofhtml multi-file site  
**CSS / structure:** [html-shared-css-structure-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/html-shared-css-structure-mvp-v0.md)

## CLI

```text
nytprof-cli html <profile.out>                 # write HTML to stdout
nytprof-cli html <profile.out> -o report.html  # write to file
```

Library: `nytprof_report::render_html_summary(model: &ProfileModel, profile_path: &str) -> String`

## Required HTML content (self-contained single document)

1. `<!DOCTYPE html>` and a `<title>` mentioning NYTProf or the profile basename.
2. Profile path (escaped).
3. Event counts: at least `time_line_events`.
4. Subroutine table with **oracle-sourced** rows for `main::leaf` and `main::mid` showing **returns** (15 and 3 on default-calls1).
5. Source section for the primary workload fid (lowest fid with `workload` in the name, else fid 1): list lines with **source text** and **calls/ticks** from `line_totals` where available; include the hot loop line body (e.g. `$x++ for 1 .. 50`).
6. Escape `<`, `>`, `&`, `"` in all text/source.
7. **Call edges table** from `call_edges` (at least caller, called, count) sorted deterministically; must include `main::mid` → `main::leaf` with count **15** on default-calls1.
8. **Exclusive-time ranking** section: top subroutines by `excl` from `sub_return_totals` (name + excl + returns), including workload subs.
9. **CSS policy (single-file):** embed `SHARED_STYLE_CSS` in an inline `<style>` block so the document is self-contained (no external `style.css` dependency). Multi-file sites use a shared `style.css` instead — see the shared CSS structure schema.
10. **Sort JS (single-file):** embed `SHARED_SORT_JS` in an inline `<script>` (same source as multi-file `nytprof-sort.js`). Not jquery/tablesorter.
11. **HTML-only seconds:** incl/excl/source ticks use `format_time_cell` when `ticks_per_sec` is present; `title=` holds raw ticks. Text/CSV/`report --json` stay integer ticks.

## Explicit non-requirements

- jquery / tablesorter / oracle CSS / flame / Graphviz / CSS polish parity with legacy nytprofhtml
- Exact DOM match to legacy nytprofhtml

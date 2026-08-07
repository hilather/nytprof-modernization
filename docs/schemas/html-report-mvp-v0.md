# HTML report MVP (v0)

**Status:** first-slice minimal native HTML  
**Not:** full nytprofhtml multi-file site

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

## Explicit non-requirements

- tablesorter / multi-file index / flame / Callgrind / CSS polish parity
- Exact DOM match to legacy nytprofhtml

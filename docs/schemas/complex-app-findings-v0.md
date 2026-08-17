# Complex-app attach findings (v0)

**Catalog:** [`complex-app-catalog-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/complex-app-catalog-v0.md)  
**Gate:** survive/fail/SKIP under in-tree `perl -d:NYTProfM` and optional 6.15. **Not** exclusive-time match.

| id | family | native | oracle 6.15 | symptom / notes |
|----|--------|--------|-------------|-----------------|
| rex | Rex/Moo-automation | survive | survive | `rex_lab_ok` both sides; `--engine both` 3s. No exclusive-time compare. |
| mojolicious | Mojolicious/Mojo | survive | SKIP | `mojo_lab_ok`; oracle not run this slice |
| dbi_sqlite | DBI/DBD-SQLite | survive | SKIP | `dbi_sqlite_lab_ok`. Residual: `disconnect` warns about active sth (DB::sub wrap of `DESTROY`/`disconnect`) — not an attach-kill |
| template_toolkit | Template-Toolkit | survive | SKIP | `tt_lab_ok` |
| ppi | PPI-perl-AST | survive | SKIP | `ppi_lab_ok` |
| xml_libxml | XML-LibXML-XS | survive | SKIP | `xml_libxml_lab_ok` |
| json_xs | Cpanel-JSON-XS | survive | SKIP | `json_xs_lab_ok` |
| csv_xs | Text-CSV_XS | survive | SKIP | `csv_xs_lab_ok` |
| cryptx | CryptX | survive | SKIP | `cryptx_lab_ok` |
| html_tree | HTML-Parser | survive | SKIP | `html_tree_lab_ok` |

Status values: `survive` (token + `NYTProf 5`), `fail` (attach-kill or nonzero), `SKIP` (honest: no docker / no module / not run).

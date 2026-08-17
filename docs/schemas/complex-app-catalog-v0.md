# Complex-app attach catalog (v0)

**Machine catalog:** [`scripts/field/workloads/complex_apps/catalog.tsv`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/workloads/complex_apps/catalog.tsv)  
**Findings:** [`complex-app-findings-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/complex-app-findings-v0.md)  
**Lab:** [`scripts/field/complex_app_docker_profile.sh --app ID`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/complex_app_docker_profile.sh)

Twenty real Perl applications chosen for attach-edge density (XS, importer/`caller`, large OO graphs). The **top 10** each declare a **distinct primary dependency family**. Shared `strict` / `Getopt::Long` / `Exporter` does **not** count as overlap. Two Moose+DateTime stacks in the top 10 **would**.

## Top 10 (drivers required)

| id | primary family | token | Why this edge |
|----|----------------|-------|----------------|
| `rex` | Rex/Moo-automation | `rex_lab_ok` | Inherited `import`, Shared::Var, Getopt, DateTime |
| `mojolicious` | Mojolicious/Mojo | `mojo_lab_ok` | Mojo DOM/URL — evented toolkit, no daemon |
| `dbi_sqlite` | DBI/DBD-SQLite | `dbi_sqlite_lab_ok` | DBI method resolution + SQLite XS |
| `template_toolkit` | Template-Toolkit | `tt_lab_ok` | TT compile/process stash |
| `ppi` | PPI-perl-AST | `ppi_lab_ok` | Many small PPI classes + `find` |
| `xml_libxml` | XML-LibXML-XS | `xml_libxml_lab_ok` | libxml2 XSUB DOM/XPath |
| `json_xs` | Cpanel-JSON-XS | `json_xs_lab_ok` | JSON XSUB encode/decode |
| `csv_xs` | Text-CSV_XS | `csv_xs_lab_ok` | CSV XSUB parse/print |
| `cryptx` | CryptX | `cryptx_lab_ok` | CryptX digest/PRNG XSUBs |
| `html_tree` | HTML-Parser | `html_tree_lab_ok` | HTML::Parser XS + TreeBuilder |

## Catalog 11–20 (named only; not top-10 drivers)

| id | primary family | Why listed / why not top 10 |
|----|----------------|------------------------------|
| `dancer2` | Dancer2/Plack | Plack web — overlaps Mojo family |
| `dbix_class` | DBIx-Class-ORM | ORM — overlaps DBI family |
| `app_ack` | App-Ack | ack / Getopt compile residual |
| `sqitch` | App-Sqitch | DB change-management CLI |
| `git_repository` | Git-Repository | git porcelain wrapper |
| `yaml_xs` | YAML-LibYAML | libyaml XS — overlaps Rex’s YAML userland |
| `net_dns` | Net-DNS | DNS packet codec |
| `dist_zilla` | Dist-Zilla | plugin-heavy release toolkit |
| `io_async` | IO-Async | event-loop futures, no listen |
| `minion` | Minion/Mojo-jobs | job queue — overlaps Mojo family |

## Diversity rule

Top-10 `primary_family` values must be unique. Tests: [`t/complex_app_catalog.t`](https://github.com/hilather/nytprof-modernization/blob/main/t/complex_app_catalog.t).

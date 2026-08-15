# NYTProfM v0.2.10 — EL8 RPM actually ships the HTML v2 visual refresh

**Tag:** `v0.2.10` (supersedes the **burned** `v0.2.9` tag — its own new freshness gate fired on first run: the marker was computed in a UTF-8 locale, CI sorts in POSIX `C`; fixed by pinning `LC_ALL=C` in `cli_source_sha256.sh`. No v0.2.9 release was ever published.)  
**Date:** 2026-08-15  
**Since:** [`v0.2.8`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.8)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-4** (upgrades `6.15-3`). **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.10/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.10/scripts/packaging/build_el8_module_rpm.sh).

```text
sudo rpm -Uvh perl-NYTProfM-6.15-4.el8.x86_64.rpm
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
# optional call-tree flame (off by default):
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html --flame
```

## Changes since v0.2.8 (grouped)

**Packaging fix (the point of this tag)**

- v0.2.8 **documented** the operator HTML v2 visual refresh (carded tables, sticky header, `prefers-color-scheme: dark`, rounded flame frames), but the 6.15-3 RPM still carried the **pre-refresh** `nytprofm-cli`: the EL8 package installs the committed prebuilt ELF ([ADR-0010](https://github.com/hilather/nytprof-modernization/blob/v0.2.10/docs/adrs/0010-signed-ci-prebuilt-native-cli.md) test-drive path), and that binary was last built 2026-08-13 — two days before the styling commit. The release workflow packaged it unchanged.
- 6.15-4 refreshes `packaging/prebuilt/el8-x86_64/nytprof-cli` (rebuilt in `rockylinux:8` from the v0.2.8 source, rustc 1.97.1), so `nytprofm-cli html` output now matches the v0.2.8 notes.

**CI hardening (fail-closed)**

- New freshness contract: [`scripts/packaging/cli_source_sha256.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.10/scripts/packaging/cli_source_sha256.sh) hashes the `crates/` + manifest inputs; `build_el8_nytprof_cli.sh` records it in `nytprof-cli.source-sha256`; the release workflow **fails closed** before rpmbuild when the committed marker does not match the tagged source. A stale prebuilt can no longer ship silently.

**No collector, wire-format, or default-behavior changes** vs v0.2.8. `collection_default` stays **v5**; zlib-6 default and opt-in `durable=1` are unchanged.

## Residuals (do not claim)

Same as v0.2.8: full 6.15 opcode / DOM/jquery/tablesorter (**WAIVE**) / COMPAT-007 Data / COL-007 C v6 writer / `_exit` flush / default-on `durable` / `aggregate=1` checkpoints / default-on flame / Graphviz image render / PAUSE / signed COPR (ADR-0010 signed pipeline still residual — the freshness gate is test-drive grade, not signing) / default Rocky `format=v6` / public perf or size certification.

## Docs

- HTML v2 design: [`docs/OPERATOR_HTML_V2_DESIGN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.10/docs/OPERATOR_HTML_V2_DESIGN_v0.md)
- Shared CSS contract: [`docs/schemas/html-shared-css-structure-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.10/docs/schemas/html-shared-css-structure-mvp-v0.md)
- Operator runbook: [`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.10/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md)

# BASE-001 — Pinned Devel::NYTProf 6.15 oracle

## Pin identity

| Field | Value |
|-------|-------|
| Distribution | Devel-NYTProf 6.15 |
| Tag | `v6.15` |
| Commit (tag object) | see `oracle-commit.txt` / `manifest.json` |
| Archive SHA-256 | see `oracle-archive.sha256` / `archives/SHA256SUMS` |

## Rebuild (clean machine)

From the repository root:

```sh
./scripts/baseline/run_all.sh
```

Or step by step:

```sh
./scripts/baseline/fetch_oracle.sh
./scripts/baseline/build_oracle.sh
./scripts/baseline/test_oracle.sh
./scripts/baseline/write_manifest.sh
```

After rebuild, confirm `manifest.json` → `isolation.loads_from_install_tree` is true and `candidate_contamination` is false.

## What is committed vs local

| Path | Committed? |
|------|------------|
| `archives/` + checksums | yes (source pin) |
| `manifest.json`, `oracle-*.txt` | yes (last proven pin metadata) |
| `src/`, `install/`, `logs/`, `test-deps/` | **no** — gitignored build products |

## Test note

Upstream `make test` requires optional modules `Capture::Tiny` and `Test::Differences` for `t/12-data.t` and `t/42-global.t`. Install them into `baseline/6.15/test-deps` (see cpanm in project notes) or accept the documented core suite: all other `t/*.t` files that ship with 6.15 and do not need author-only modules.

On the bootstrap host, core suite ran **4378** subtests with only those two files failing for missing deps; after installing deps into a local lib, both additional files passed (**368** more tests).

## Isolation rule

Never put candidate `crates/` or `perl/` on `PERL5LIB` when running oracle tools. `tools/oracle/env.sh` enforces install-tree module path.

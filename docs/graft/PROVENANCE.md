# Product XS graft provenance

**Status:** E0 pin stamp only — **no pin files copied yet**  
**Annex:** [product-xs-graft-annex-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-graft-annex-v0.md) A.1  
**DI-03 design:** [DI03_OPCODE_ENTERSUB_ATTACH_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/DI03_OPCODE_ENTERSUB_ATTACH_v0.md)  
**ADR:** [0004-collector-packaging-source-tree.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md)  
**Date:** 2026-08-17

This file is the annex A.1 provenance stamp. Graft **copy** into `collector/xs/` starts in DI-03 **E1a**. E0 records the pin identity so later PRs append file and delta rows instead of inventing a pin.

**Do not edit** `baseline/6.15/src` as SoT. Never ship or `dlopen` pin `NYTProf.so` as the product debugger. Never put `crates/` on oracle `PERL5LIB`.

## Pin identity

| Field | Value |
|-------|-------|
| Distribution | Devel-NYTProf **6.15** |
| Tag | `v6.15` |
| Commit | `7578f4bfb7e519908cc5431890f9121fdf60106c` ([`baseline/6.15/oracle-commit.txt`](https://github.com/hilather/nytprof-modernization/blob/main/baseline/6.15/oracle-commit.txt)) |
| Archive SHA-256 | see [`baseline/6.15/oracle-archive.sha256`](https://github.com/hilather/nytprof-modernization/blob/main/baseline/6.15/oracle-archive.sha256) / [`baseline/6.15/manifest.json`](https://github.com/hilather/nytprof-modernization/blob/main/baseline/6.15/manifest.json) |
| Pin tree | `baseline/6.15/` (archives + isolated install; **P-ORACLE only**) |
| License | Artistic-1.0-Perl OR GPL-1.0-or-later |

## Files copied from the pin (E0)

**None.** E0 does not graft `pp_entersub_profiler` / `subr_entry_*` / `slowops.h` / leave ops.

| Pin source | Product destination | When |
|------------|---------------------|------|
| — | — | E1a+ will append rows |

## Deltas vs pin (E0)

**None copied.** Product attach remains wrap + C `OP_DBSTATE` on `nytp_emit_*` → `nytp_sink_v5`.

| Delta | Status |
|-------|--------|
| `NYTPROF` `wrap` / `entersub` known keys + 0/1 stamps | product-only E0 (not a pin copy) |
| Product `use_db_sub=1` = wrap escape (**not** 6.15 stmt `DB::DB`) | intentional fork (KD-E11) |
| Replace `NYTP_write_*` with `nytp_emit_*` | later E1a write-site substitution |

## Security backports

Track upstream 6.15.x / security fixes; cherry-pick into `collector/xs/` after E1a copies exist. Never rewrite pin archives.

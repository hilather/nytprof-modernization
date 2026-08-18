# Product XS graft provenance

**Status:** E2 — default opcode hooks `OP_ENTERSUB` + `OP_GOTO` (`goto &sub` keeps original caller + goto-site fid:line); wrap list stays `wrap=1` only. DI-03 not fully done (E3 leave default 0, E4 full table opt-in, di02 27 remain).  
**Annex:** [product-xs-graft-annex-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-graft-annex-v0.md) A.1  
**DI-03 design:** [DI03_OPCODE_ENTERSUB_ATTACH_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/DI03_OPCODE_ENTERSUB_ATTACH_v0.md)  
**ADR:** [0004-collector-packaging-source-tree.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md)  
**Date:** 2026-08-17

This file is the annex A.1 provenance stamp. Graft **copy** into `collector/xs/` starts in DI-03 **E1a**.

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

## Files copied from the pin (E1a)

Pin `baseline/6.15/src/NYTProf.xs` is **not** present in this tree. Functions were read from the committed archive `baseline/6.15/archives/Devel-NYTProf-6.15.tar.gz` (`devel-nytprof-6.15/NYTProf.xs`, extracted to a temp dir only — never written into `baseline/`).

| Pin source | Product destination | When |
|------------|---------------------|------|
| `NYTProf.xs` `subr_entry_t` ~1959–1986 | `collector/xs/nytprof_pp.h` + `pp_entersub.c` (FileHandle/overhead fields dropped) | E1a |
| `incr_sub_inclusive_time` / `_ix` ~2086–2274 | `collector/xs/pp_entersub.c` (`product_incr_sub_inclusive_time`) | E1a |
| `subr_entry_destroy` / savestack `SSNEWa` / `save_destructor_x` | `collector/xs/pp_entersub.c` | E1a |
| `subr_entry_setup` ~2390–2628 | `collector/xs/pp_entersub.c` (fid/clock/skip/emit gate) | E1a |
| `resolve_sub_to_cv` ~2277–2341 | `collector/xs/pp_entersub.c` (no `tryAMAGICunDEREF`) | E1a |
| `current_cv` ~2345–2386 | `collector/xs/pp_entersub.c` | E1a |
| `pp_entersub_profiler` / `pp_subcall_profiler` ~2631–2928 | `collector/xs/pp_entersub.c` (`pp_product_entersub`; E1a **no** `OP_GOTO`) | E1a |
| `pp_subcall_profiler` `OP_GOTO` branch ~2667–2766 + setup clone ~2509–2513 | `collector/xs/pp_entersub.c` (template `subr_entry_t *tmpl`; separate `product_orig_pp_goto`; pin REFCNT_inc/mortalize) | E2 |
| `PL_ppaddr[OP_GOTO] = pp_entersub_profiler` ~3257 | `product_install_entersub` / uninstall | E2 |
| `NYTP_MAX_SUB_NAME_LEN` ~106–108 | `collector/xs/nytprof_pp.h` | E1a |

## Deltas vs pin (E1a)

| Delta | Status |
|-------|--------|
| `NYTPROF` `wrap` / `entersub` known keys + 0/1 stamps | product-only E0 (not a pin copy) |
| Product `use_db_sub=1` = wrap escape (**not** 6.15 stmt `DB::DB`) | intentional fork (KD-E11) |
| Replace `NYTP_write_call_entry` / `NYTP_write_call_return` with `nytp_emit_sub_*` | E1a write-site substitution |
| `nytp_emit_sub_return` **and** `nytp_emit_sub_callers` in **ticks at return** | KD-E03; no `sub_callers_hv`; no finalize `/ ticks_per_sec` |
| Clock = `nytp_clock_now`; fid = `product_fid_for_file_ptr` | KD-E07 |
| `cumulative_subr_ticks` + `initial_subr_ticks` **copied** | required for g14 remainder |
| `cumulative_overhead_ticks` **omitted** (overhead = 0) | KD-E13 |
| Recursion: wrap semantics (`reci=0`, `rec_depth=0`, full incl/excl) | not pin `called_cv_depth <= 1` |
| Skip `DB::*` and `Devel::NYTProfM` internals | product identity |
| Install `OP_ENTERSUB` at `file=`; emit only after INIT | KD-E17 / di02 **27** |
| Opcode when `PRODUCT_ENTERSUB && !PRODUCT_WRAP`; E1b omit-default is opcode | KD-E01 / KD-E16 |
| Keep pending-excl mailbox; `product_credit_child_excl` branches | KD-E12 |
| E1b omits `OP_GOTO` / leave / full `slowops.h` | E2–E4 (E2 landed) |
| E2 `goto &CV` only; other gotos run orig | pin pre-condition ~2667–2670 |
| E2 `subr_entry_ix == -1` → orig only | pin ~2670 |
| E2 caller_* from copied frame; fid:line from goto COP | pin ~2731–2735 / ~2509–2513 |
| E2 separate `product_orig_pp_goto` (never call orig ENTERSUB for `OP_GOTO`) | product adaptation |
| E2 port pin REFCNT_inc/mortalize leak comment (no new leak “fix”) | pin ~2745 |
| E2 wrap-list `goto &$raw` stays wrap=1 / `use_db_sub=1` only | not a substitute for opcode GOTO |
| E2 omits leave / full `slowops.h` | E3–E4 |

## Security backports

Track upstream 6.15.x / security fixes; cherry-pick into `collector/xs/` after E1a copies exist. Never rewrite pin archives.

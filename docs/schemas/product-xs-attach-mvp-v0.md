# Product XS attach MVP (v0) — G02 scaffold + G03a load + G03b/G03c/G03d/G03e emit-MVP

**Board IDs:** `G02-V5-PRODUCT-LINK` (scaffold), `G03A-LOAD-ONLY` (debugger load), `G03B-STMT-EMIT` (stmt emit-MVP), `G03C-SUB-EMIT` (sub emit-MVP), `G03D-META-EMIT` (meta/finalize emit-MVP), `G03E-COMPRESS-EMIT` (compress emit-MVP), `PRODUCT-XS-ATTACH-MVP` (G04 attach-MVP landed)  
**Status:** **G04 attach-MVP landed; live times measured (PR-1); live finish emits SRC_LINE/SUB_INFO (PR-3); C stmt-ops / PRINT-MATCH slowops measured (PR-8); PR-9 real `SUB_CALLERS` fid/line + parent excl = incl − slowop children (`g09_tokenize_excl_smoke.sh`); PR-15 default TIME_LINE from C `OP_DBSTATE` (`g15_dbstate_timeline_smoke.sh`); PR-16 wrap escape `wrap_push`/`wrap_pop` (`g16_wrap_enter_smoke.sh`); E1b default call attach is opcode `OP_ENTERSUB` (`g17_entersub_attach_smoke.sh`; wrap is `wrap=1`).** `NYTPROF file=` + default `OP_ENTERSUB` + C `OP_DBSTATE` emit `SUB_RETURN` / `SUB_CALLERS` / `TIME_LINE` through shipped `nytp_emit_*`. Incl/excl and statement ticks come from `nytp_clock_now` (CLOCK_MONOTONIC, 10M ticks/s), not hardcoded `0.0` / visit-`1`. `finish_profiler` flushes the last site, `begin_finalize`s, walks `product_fid_map` for `SRC_LINE` (no `HAS_SRC`) and `%DB::sub` for lookup-only `SUB_INFO` (no `NEW_FID` after finalize), then `PID_END`. `savesrc` default **on**; `PL_perldb |= PERLDBf_SAVESRC | PERLDBf_SAVESRC_NOSUBS` at `file=` (not `$^P |= 0x400`). Live `perl -d:NYTProfM` on default-calls1-shaped work; shipped dump/report of those bytes: leaf **15** / mid **3** / mid→leaf **15**, `main::leaf` incl **> 0**, `src_line_events > 0`, `sub_def` for `main::leaf` / `main::mid`, no `HASH(` callers, `verify` OK (PID pair). G03a trivial `-e` (no `file=`) still writes no `nytprof.out`. **PR-8:** `blocks=1` stmt-ops attribute `now-last` on `TIME_BLOCK` (same last-site clock as `TIME_LINE`; first hit seeds; `now < last` → `NYTP_ERR_OVERFLOW`, no wrap). Thin `OP_PRINT`/`OP_MATCH` emit NV incl/excl (`excl = incl`) from `nytp_clock_now` around the original ppaddr. **Still not** DI-03 done (E2 `goto` / E3 leave / E4 full `slowops.h` / live di02 **21** vs oracle **27**).  
**Annex:** [product-xs-graft-annex-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-graft-annex-v0.md) A.3 / A.4 / A.5 / A.6  
**Smokes:** [g02_v5_product_link_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g02_v5_product_link_smoke.sh) · [product_attach_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/product_attach_smoke.sh) · [g03b_stmt_emit_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03b_stmt_emit_smoke.sh) · [g03c_sub_emit_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03c_sub_emit_smoke.sh) · [g03d_meta_emit_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03d_meta_emit_smoke.sh) · [g03e_compress_emit_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03e_compress_emit_smoke.sh) · [g04_v5_parity_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g04_v5_parity_smoke.sh) · [g08_slowops_times_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g08_slowops_times_smoke.sh) · [g09_tokenize_excl_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g09_tokenize_excl_smoke.sh) · [g15_dbstate_timeline_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g15_dbstate_timeline_smoke.sh) · [g16_wrap_enter_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g16_wrap_enter_smoke.sh) · [g17_entersub_attach_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g17_entersub_attach_smoke.sh) · [di01_blocks_780_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/di01_blocks_780_smoke.sh)  
**Attach smoke schema:** [product-attach-smoke-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-attach-smoke-mvp-v0.md)

## What G02 shipped

| Piece | Honesty |
|-------|---------|
| `make -C collector libnytp_sink_v5.a` | D1-B product archive: v5 + counting + batch + clock + fork; **no** v6/dual; **`-lz` only** |
| `make -C collector probe-v5` | C probe links that archive with `-lz` only; calls `nytp_v5_sink_create(NULL)` and checks `NYTProf 5` header |
| `make -C collector xs-bootstrap` | `Devel::NYTProf::CollectorBootstrap` `.so` + thin `.pm`; BOOT probes the real v5 sink |
| Exports | `loaded()`, `product_link_flavor` → `v5-only`, `product_xs_attach` → **0**, `probe_v5_header()` |
| Build tree | Sources in `collector/xs/`; objects under `collector/build/` (gitignored) |

## What G03a shipped

| Piece | Honesty |
|-------|---------|
| `collector/xs/Devel/NYTProf.pm` | Debugger entry for `perl -d:NYTProf` (`$^P` = 0x010\|0x100\|0x200, no 0x01) |
| `collector/xs/Devel/NYTProf/Core.pm` | `XSLoader::load('Devel::NYTProf', $VERSION)` — `$VERSION` **7.00** (KD-16) |
| `collector/xs/NYTProf.xs` | `MODULE Devel::NYTProf`; `PACKAGE = DB`; `init_profiler` holds in-memory `nytp_v5_sink_create(NULL)` |
| `make -C collector xs-nytprof` | Builds `collector/build/xs-nytprof/auto/Devel/NYTProf/NYTProf.so` + copies `.pm` files; links `libnytp_sink_v5.a` + **`-lz` only** |
| Load stamp | `$Devel::NYTProf::PRODUCT_XS_LOAD = 1` after Core load |
| Attach stamp | `$Devel::NYTProf::PRODUCT_XS_ATTACH` stays **0** |
| Profile file | Trivial `-e` **must not** write `nytprof.out` |

## What G03b shipped

| Piece | Honesty |
|-------|---------|
| Held product sink | Static `nytp_sink *`; default `init_profiler` is in-memory (no file) |
| `DB::enable_sink($path)` | Replace held sink with `nytp_v5_sink_create($path)` + `nytp_sink_activate` |
| `DB::emit_time_line` / `emit_time_block` / `emit_discount` | Call **only** shipped `nytp_emit_*`; return `nytp_status` |
| `DB::finish_profiler` | `nytp_sink_close` + `nytp_sink_destroy`; clear static |
| `DB::run_m4_mini_sample` | Shipped `nytp_m4_mini_sample_run` on the held sink (fake-clock gate) |
| `DB::overflow_probe` | `nytp_emit_time_line` with ticks `> INT32_MAX` → `NYTP_ERR_OVERFLOW` (4) |
| Emit stamp | `$Devel::NYTProf::PRODUCT_STMT_EMIT = 1` (not attach-green) |
| Mini stream | Real `NYTProf 5` bytes; dump JSONL contains TIME_LINE / TIME_BLOCK / DISCOUNT from those bytes |
| Clock / discount | C `test_fake_clock` + product m4 dump TIME_LINE + DISCOUNT order |

## What G03c shipped

| Piece | Honesty |
|-------|---------|
| `DB::emit_sub_entry($caller_fid, $caller_line)` | Call **only** shipped `nytp_emit_sub_entry`; return `nytp_status` |
| `DB::emit_sub_return($depth, $incl, $excl, $subname)` | Call **only** shipped `nytp_emit_sub_return` with `nytp_sv_cstr` on an **owned** C copy of the Perl SV bytes |
| NULL sink | Both wrappers return `NYTP_ERR_NULL` (1) when the held sink is unset |
| Emit stamp | `$Devel::NYTProf::PRODUCT_SUB_EMIT = 1` (not attach-green) |
| Mini stream | Real `NYTProf 5` bytes; dump JSONL contains `SUB_ENTRY` / `SUB_RETURN` from those bytes |
| Opcode / entersub | **Not** grafted; still G04 |

## What G03d shipped

| Piece | Honesty |
|-------|---------|
| `DB::emit_attribute($key, $value)` | Call **only** shipped `nytp_emit_attribute` with `nytp_sv_cstr` on **owned** C copies of the Perl SV bytes |
| `DB::emit_option($key, $value)` | Call **only** shipped `nytp_emit_option` with owned `nytp_sv_cstr` copies |
| `DB::emit_new_fid($fid, $eval_fid, $eval_line, $flags, $size, $mtime, $name)` | Call **only** shipped `nytp_emit_new_fid` with owned `nytp_sv_cstr` name |
| `DB::emit_src_line($fid, $line, $text)` | Call **only** shipped `nytp_emit_src_line` with owned `nytp_sv_cstr` text |
| `DB::emit_sub_info($fid, $first, $last, $name)` | Call **only** shipped `nytp_emit_sub_info` with owned `nytp_sv_cstr` name |
| `DB::emit_pid_start($pid, $ppid, $start_time)` | Call **only** shipped `nytp_emit_pid_start` |
| `DB::emit_pid_end($pid, $end_time)` | Call **only** shipped `nytp_emit_pid_end` |
| NULL sink | All seven wrappers return `NYTP_ERR_NULL` (1) when the held sink is unset |
| Emit stamp | `$Devel::NYTProf::PRODUCT_META_EMIT = 1` (not attach-green) |
| Mini stream | Real `NYTProf 5` bytes; dump JSONL contains ATTRIBUTE / OPTION / NEW_FID / SRC_LINE / SUB_INFO / PID_START / PID_END from those bytes |
| `SUB_CALLERS` / `emit_comment` | **Not** required (optional wrappers) |
| Opcode / entersub | **Not** grafted; still G04 |

## What G03e shipped

| Piece | Honesty |
|-------|---------|
| `DB::emit_start_deflate()` | Call **only** shipped `nytp_emit_start_deflate` on the held v5 sink; return `nytp_status` |
| `DB::is_deflating()` | Call **only** shipped `nytp_v5_sink_is_deflating`; 0 when sink is unset or before START_DEFLATE |
| NULL sink | `emit_start_deflate` returns `NYTP_ERR_NULL` (1) when the held sink is unset |
| Duplicate START_DEFLATE | Second `emit_start_deflate` on the same sink returns `NYTP_ERR_STATE` (2) |
| Emit stamp | `$Devel::NYTProf::PRODUCT_COMPRESS_EMIT = 1` (not attach-green) |
| Mini stream | Real `NYTProf 5` bytes; tag `z` then zlib body (`-lz` only); dump/verify inflate recovers a post-deflate event |
| Mid-deflate fork | **Residual** — child does not inherit the compressor |
| Opcode / entersub | **Not** grafted; still G04 |

## What G04 shipped

| Piece | Honesty |
|-------|---------|
| `NYTPROF file=` | Parsed like 6.15 (colon-separated, backslash-escapes). **No default `nytprof.out`** — absent `file=` keeps G03a in-memory |
| `DB::enable_sink` on `file=` | Replaces the held sink; activate; `$PRODUCT_XS_ATTACH=1` for that session only |
| `$^P \|= 0x01` | **Wrap escape only** (`wrap=1` / `use_db_sub=1`). E1b default opcode clears this bit (`DB::sub` stub). |
| `$^P \|= 0x02` + `$DB::single` in `INIT` | **PR-7:** do **not** set `$DB::single` at `file=` enable (`pp_dbstate` would run `DB::DB` during `use`/`BEGIN`). **PR-15:** default `stmts=1` installs C `OP_DBSTATE` TIME_LINE and leaves `$DB::single=0` so Perl `DB::DB` is not entered. `$DB::single=1` is only the fallback when the C hook is not installed. Smoke [`g07_getopt_compile_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g07_getopt_compile_smoke.sh) + [`g15_dbstate_timeline_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g15_dbstate_timeline_smoke.sh). |
| No `CORE::GLOBAL::require`; hint-magic `CvNODEBUG` | **PR-10:** do **not** wrap `CORE::require`. Preload BHES / Variable::Magic / namespace::* and `DB::nodebug_stash` **before** `$^P \|= 0x01`. `DB::sub` during `on_scope_end` breaks `%^H` / `DateTime::Duration`. Do **not** defer 0x01 to `INIT`. Smoke [`g10_datetime_hints_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g10_datetime_hints_smoke.sh). |
| `nodebug_stash` GP-safe | **PR-11:** `GvCV` requires a GP. Walk stash slots with `isGV_with_GP` / `product_stash_val_cv`; do not treat `CvROOT` of an XSUB as an `OP*`. Smoke [`g11_nodebug_stash_nogp_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g11_nodebug_stash_nogp_smoke.sh). |
| Memoize `caller` | **PR-12:** goto `Memoize::`. `memoize('fn')` uses `caller` as the package; wrap looks up `DB::fn` (`Cannot operate on nonexistent function`). Smoke [`g12_memoize_caller_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g12_memoize_caller_smoke.sh). |
| Logger `caller` | **PR-13:** do not `eval` around `&$raw` (`CXt_EVAL` is visible; package-DB subs are not). DESTROY emits `SUB_RETURN` on die. Smoke [`g13_logger_caller_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g13_logger_caller_smoke.sh). |
| Nested exclusive | **PR-14:** parent exclusive = incl − Σ child **inclusive**. `stmts=0` skips TIME_LINE. Smoke [`g14_nested_excl_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g14_nested_excl_smoke.sh). |
| C `OP_DBSTATE` TIME_LINE | **PR-15:** default `stmts=1` emits TIME_LINE from `pp_product_dbstate_line` + last-COP fid pointer cache. Do **not** hook NEXTSTATE/UNSTACK (that is `blocks=1`). `$DB::single=0`. Smoke [`g15_dbstate_timeline_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g15_dbstate_timeline_smoke.sh). **Not** NEXTSTATE / `TIME_BLOCK` (that is `blocks=1`). Call attach is E1b opcode. |
| C wrap push/pop | **PR-16 + E1b:** wrap assertions only under `wrap=1`. `DB::wrap_push`/`wrap_pop` (COP pin + fid + clock + pending-excl + `SUB_RETURN`/`SUB_CALLERS`). `NYTPROF_WRAP_SLOW=1` is nested under that escape. Smoke [`g16_wrap_enter_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g16_wrap_enter_smoke.sh). Default attach is opcode (g17). **Not** E2 GOTO / stock 6.15 XS. |
| `ATTRIBUTE application` | **$0** at `file=` enable (6.15 `output_header`). HTML primary file / “Profile of …” must not pick `Config_heavy.pl`. |
| `DB::sub` | E1b default: **stub** (opcode `OP_ENTERSUB` emits). Wrap body only if `wrap=1` / `use_db_sub=1` / `entersub=0`. Goto list remains wrap-escape only. |
| `DB::emit_sub_callers` | Call **only** shipped `nytp_emit_sub_callers` |
| `DB::DB` | Fallback only. Default path is C `OP_DBSTATE` (`PRODUCT_DBSTATE_LINE`). |
| `END { finish_profiler }` | **PR-1:** `flush_last_site` → `PID_END` → drop. **PR-3:** after flush, `begin_finalize` + `SRC_LINE` (fid-map walk) + lookup-only `SUB_INFO` **before** `PID_END`. Second call is a no-op. **Still not** full opcode finalize. |
| `savesrc` / `PL_perldb` | Default **1**. `file=` calls `DB::set_savesrc` → `PL_perldb \|= PERLDBf_SAVESRC \| PERLDBf_SAVESRC_NOSUBS`. `savesrc=0` skips file `SRC_LINE`. |
| Smoke | [`g04_v5_parity_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g04_v5_parity_smoke.sh) — real `perl -d:NYTProfM` on `fixtures/v5/default-calls1/workload.pl`; dump/report of **those** bytes: leaf **15** / mid **3** / mid→leaf **15**; live `SRC_LINE` / `SUB_INFO` + `sub_def` leaf/mid |
| Attach stamp | `$PRODUCT_XS_ATTACH` is **1** only when `file=` is set; G03a trivial `-e` stays **0** |

## Explicit non-claims

| Residual | Rule |
|----------|------|
| E1b `OP_ENTERSUB` (product sink) | **Landed** — default omit installs `OP_ENTERSUB`; wrap is `wrap=1`. Smoke [`g17_entersub_attach_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g17_entersub_attach_smoke.sh) |
| E2 `OP_GOTO` | **Landed** — smoke [`g18_goto_sub_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g18_goto_sub_smoke.sh) |
| E3 leave / E4 full `slowops.h` | **E4 default flip landed** — `leave=1` (default 0); `slowops=2` / `full` / `=3` install the full 6.15 table. Exclusive remains thin. |
| XSUB / leave default 1 / live di02 **27** | **Residual** — DI-03 **not** done. |
| G05 `format=v6` D1-A / D1-B fail-closed | **Landed** — [`g05_options_format_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g05_options_format_smoke.sh) |
| G06 fork / `addpid` | **Landed** — mid-deflate continue-in-child / TEST-018 remain residual |
| blocks-calls1 line5 **780** | **Not** claimed |
| FileHandle dual writer | **Not** added |
| `collection_default` | Remains **v5** |
| Dual-path / G01 legacy smoke | Unchanged; still oracle-primary; `product_legacy_smoke.sh` stays S0/S1 honest skip |
| CollectorBootstrap | Remains distinct (`xs-bootstrap`); not the debugger |

## Load proof (G03a)

```text
make -C collector xs-nytprof
PERL5LIB=collector/build/xs-nytprof perl -Icollector/build/xs-nytprof \
  -d:NYTProf -e 'print "ok\n"'
# → ok  (exit 0; no nytprof.out)
```

## Emit proof (G03b)

```text
./scripts/packaging/g03b_stmt_emit_smoke.sh
# enable_sink + nytp_emit_* → NYTProf 5; dump has TIME_LINE/TIME_BLOCK/DISCOUNT
# run_m4_mini_sample dump has TIME_LINE + DISCOUNT in mini order
```

## Emit proof (G03c)

```text
./scripts/packaging/g03c_sub_emit_smoke.sh
# enable_sink + nytp_emit_sub_entry / sub_return → NYTProf 5
# dump has SUB_ENTRY + SUB_RETURN from those bytes
```

## Emit proof (G03d)

```text
./scripts/packaging/g03d_meta_emit_smoke.sh
# enable_sink + nytp_emit_attribute / option / new_fid / src_line /
# sub_info / pid_start / pid_end → NYTProf 5
# dump has ATTRIBUTE OPTION NEW_FID SRC_LINE SUB_INFO PID_START PID_END
```

## Emit proof (G03e)

```text
./scripts/packaging/g03e_compress_emit_smoke.sh
# enable_sink + pre-deflate emit + nytp_emit_start_deflate + post-deflate emit
# → NYTProf 5; after z the body is zlib; dump inflate recovers g03e_after
```

## Attach proof (G04)

```text
./scripts/packaging/g04_v5_parity_smoke.sh
# NYTPROF=file=… perl -d:NYTProfM fixtures/v5/default-calls1/workload.pl
# → NYTProf 5; report --json leaf_returns=15 mid_returns=3 mid_leaf_edge=15
# → dump SRC_LINE + SUB_INFO; sub_def_leaf / sub_def_mid present; verify OK
```

## PR-8 proof (stmt-ops TIME_BLOCK + slowops times)

```text
./scripts/packaging/di01_blocks_780_smoke.sh
# NYTPROF=file=…:blocks=1 perl -d:NYTProfM fixtures/v5/blocks-calls1/workload.pl
# → TIME_BLOCK present; line5=780 block4=810; 15/3/15
# → TIME_BLOCK args[0] ticks not identically 1 (last-site clock)

./scripts/packaging/g08_slowops_times_smoke.sh
# NYTPROF=file=… perl -d:NYTProfM -e 'print; "foo" =~ /foo/'
# → dump/report CORE:print and/or CORE:match incl/excl not both 0
```

G05 `format=v6` tests landed. G06 fork/`addpid` landed (mid-deflate-in-child / TEST-018 residual). DI-03 residual is E2–E4 + live emit-after-INIT di02 **21** vs oracle start=begin **27** — not “entersub not grafted.”

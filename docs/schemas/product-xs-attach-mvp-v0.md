# Product XS attach MVP (v0) — G02 scaffold + G03a load + G03b/G03c/G03d/G03e emit-MVP

**Board IDs:** `G02-V5-PRODUCT-LINK` (scaffold), `G03A-LOAD-ONLY` (debugger load), `G03B-STMT-EMIT` (stmt emit-MVP), `G03C-SUB-EMIT` (sub emit-MVP), `G03D-META-EMIT` (meta/finalize emit-MVP), `G03E-COMPRESS-EMIT` (compress emit-MVP), `PRODUCT-XS-ATTACH-MVP` (G04 attach-MVP landed)  
**Status:** **G04 attach-MVP landed.** `NYTPROF file=` + Perl `DB::sub`/`DB::DB` emit `SUB_RETURN` / `SUB_CALLERS` / `TIME_LINE` through shipped `nytp_emit_*`. Live `perl -d:NYTProf` on default-calls1-shaped work; shipped dump/report of those bytes: leaf **15** / mid **3** / mid→leaf **15**. G03a trivial `-e` (no `file=`) still writes no `nytprof.out`. G03b–G03e emit-MVP remain. G05 options/`format=v6` tests landed separately. **Not** G06 fork or full 6.15 opcode/`entersub`.  
**Annex:** [product-xs-graft-annex-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-graft-annex-v0.md) A.3 / A.4 / A.5 / A.6  
**Smokes:** [g02_v5_product_link_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g02_v5_product_link_smoke.sh) · [product_attach_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/product_attach_smoke.sh) · [g03b_stmt_emit_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03b_stmt_emit_smoke.sh) · [g03c_sub_emit_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03c_sub_emit_smoke.sh) · [g03d_meta_emit_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03d_meta_emit_smoke.sh) · [g03e_compress_emit_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03e_compress_emit_smoke.sh) · [g04_v5_parity_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g04_v5_parity_smoke.sh)  
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
| `$^P \|= 0x01` | Sub enter/exit → `DB::sub` |
| `$^P \|= 0x02` + `$DB::single=1` | Line-by-line → `DB::DB` (`pp_dbstate` only calls `DB::DB` when `$DB::single` is true) |
| `DB::sub` | Emits `SUB_RETURN` + `SUB_CALLERS` via shipped `DB::emit_*` → `nytp_emit_*` |
| `DB::emit_sub_callers` | Call **only** shipped `nytp_emit_sub_callers` |
| `DB::DB` | Emits `TIME_LINE` via shipped `nytp_emit_time_line` |
| `END { finish_profiler }` | Closes the held sink (second call is a no-op) |
| Smoke | [`g04_v5_parity_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g04_v5_parity_smoke.sh) — real `perl -d:NYTProf` on `fixtures/v5/default-calls1/workload.pl`; dump/report of **those** bytes: leaf **15** / mid **3** / mid→leaf **15** |
| Attach stamp | `$PRODUCT_XS_ATTACH` is **1** only when `file=` is set; G03a trivial `-e` stays **0** |

## Explicit non-claims

| Residual | Rule |
|----------|------|
| Full 6.15 opcode / `entersub` / XSUB / goto | **Not** grafted |
| G05 `format=v6` D1-A / D1-B fail-closed | **Landed** — [`g05_options_format_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g05_options_format_smoke.sh) |
| G06 fork / `addpid` / mid-deflate continue-in-child | **Residual** |
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
# NYTPROF=file=… perl -d:NYTProf fixtures/v5/default-calls1/workload.pl
# → NYTProf 5; report --json leaf_returns=15 mid_returns=3 mid_leaf_edge=15
```

G05 `format=v6` tests landed. G06 fork/`addpid` and full 6.15 opcode/`entersub` remain residual.

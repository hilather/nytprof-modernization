# Language semantics gotchas (Perl & Rust) — light ledger

**Status:** living light index — expand under [`details/`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/details/) when a row needs more than one line  
**Duty:** [`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/main/AGENTS.md) §6 — save corrected **Perl** and **Rust** misunderstandings **automatically**  
**Not:** a language tutorial; not a substitute for oracle fixtures or COMPAT contracts

Record **language- or runtime-specific rules** that an agent got wrong (or nearly wrong) while working this repo — especially NYTProf / XS / dual-engine edges. Prefer pointers into contracts, fixtures, or oracle code over restating whole manuals. Keep rows **light**; open a detail file only when the correction needs more than one line.

| Date | Lang | Topic | Wrong assumption | Correct rule / pointer | Open? | Detail |
|------|------|-------|------------------|------------------------|-------|--------|
| 2026-08-13 | perl | for-modifier + dbstate | `$x++ for 1..50` with `$^P` / `$DB::single` fires `DB::DB` 52× (or `nextstate` per iter if `PERLDBf_NOOPT`) | Optree is one `dbstate` then `enteriter`/`iter`/`preinc`/`unstack`/`leaveloop`. 780 = 15×(1 dbstate + 50 unstack + 1 post-unstack replay). 6.15 uses UNSTACK leave + previous-statement write. DI-01 slice emits `TIME_BLOCK` on those ops only. | no | [`di01-spike`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v5/product-attach/di01-spike/) |
| 2026-08-07 | both | oracle `PERL5LIB` | Putting `crates/` or native tree on oracle `PERL5LIB` is fine for “convenience” | **Never** put `crates/` on oracle `PERL5LIB` — isolates 6.15 oracle; see `AGENTS.md` + residual matrix isolation rule | no | — |
| 2026-08-07 | rust | `total_events` on JSON | ProfileModel event counter equals advertised `total_events` (e.g. 2473) | Advertised default-calls1 **`total_events` is 2474** (`model.total_events + 1` / stream account as frozen by JSON-TOTAL-EVENTS-MVP + residual matrix) | no | — |
| 2026-08-07 | rust | v6 chunk payload | Default `parse_chunk_frame` should inflate ZLIB and verify CRC | Default parse is **non-inflating / non-CRC-verify**; inflate and CRC are **explicit** helpers (`FMT-V6-PAYLOAD-ZLIB/ZSTD/LZ4-*`, `FMT-V6-CRC-*`) | no | — |
| 2026-08-07 | rust | v6 LZ4 wire | LZ4 frame with embedded size is the MVP wire | MVP is **raw LZ4 block**; size is chunk `uncompressed_len` only (`FMT-V6-PAYLOAD-LZ4-*`) | no | — |
| 2026-08-07 | perl | SUB_ENTRY multiplicity | Every profile has SUB_ENTRY events | `calls=1` default-calls1 → **0**; `calls=2` calls2-default → **27** (multiplicity only; not full call-stack freeze) | no | — |
| 2026-08-11 | both | v5 NV vs v6 u64 SUB_RETURN times | Dual-sink fractional wall NV (0.01/0.005) stays E4-equal across formats | v5 stores **NV doubles** on wire; v6 `nv_to_u64` **truncates** toward 0 → fractional times become 0. E4-v0 dual fixtures use **integer tick** doubles (e.g. 100.0/40.0) so aggregates match (`e4_v0_*`, E4 policy) | no | — |
| 2026-08-12 | rust | `nytp_status` enum | Duplicate start-deflate / illegal lifecycle is `NYTP_ERR_IO` (3) | `NYTP_ERR_STATE` is **2**; `NYTP_ERR_IO` is **3**; overflow is **4**. See `collector/include/nytp_types.h`. G03e duplicate `nytp_emit_start_deflate` returns 2. | no | — |
| 2026-08-12 | perl | XS link vs Embed `ldopts` | `perl -MExtUtils::Embed -e ldopts` is the right way to link a loadable XS `.so` | Embed `ldopts` is for **embedding** perl (`-lperl`). Debian often has `libperl.so.5.38` but **no** `libperl.so` symlink, so `-lperl` fails. XS `.so` files are loaded into the running interpreter — use `$Config{lddlflags}` (`-shared`) + product libs (`-lz` for D1-B). `ccopts` is still correct for compile. G02 `xs-bootstrap` in `collector/Makefile`. | no | — |
| 2026-08-12 | perl | `$^P` 0x01 vs single-step | `$^P` 0x01 is single-step / statement profiler (`DB::DB`) | **0x01** is **sub enter/exit** (`DB::sub`). **0x02** is line-by-line (`DB::DB`). **0x20** is “start with single-step on.” See `perlvar` / `perldebguts`. G03a omitted 0x01 so Perl would not require `DB::sub`; G04 sets 0x01 only when `NYTPROF file=` is present. | no | — |
| 2026-08-12 | perl | `DB::DB` / `$DB::single` | Setting `$^P \|= 0x02` is enough for `DB::DB` to run each statement | `pp_dbstate` calls `DB::DB` only when **`$DB::single` is true**. `-d` already compiles `dbstate` once `$^P != 0`. G04 must set `$DB::single = 1` (and/or `$^P \|= 0x20`) after `file=` enable, or shipped `report` fail-closes on zero `TIME_LINE`. | no | — |
| 2026-08-12 | perl | D1-A vs D1-B `format=v6` | Product `format=v6` can write v6 from the default `xs-nytprof` D1-B `.so` | Default `xs-nytprof` is **D1-B** (`libnytp_sink_v5.a`, `-lz` only, `NYTPROF_V6_COLLECT` undefined) — `format=v6` **croaks** with the `v6_collect` rebuild string and must not write `NYTPROF6`. D1-A is a **separate** `xs-nytprof-v6` dest (`-DNYTPROF_V6_COLLECT`, `-lz -lzstd -llz4`). `format=dual` is always rejected. | no | — |
| 2026-08-12 | perl | product `fork` + addpid | 6.15 opcode `pp_fork` is required to get a child addpid file | Smallest G06 hook is **`CORE::GLOBAL::fork`** (installed when `addpid=1` + `file=`) calling shipped `nytp_fork_prepare` / `resume_parent` / `resume_child` + `nytp_v5_sink_fork_child_reinit` to `<file>.<pid>`. Skip `CORE::GLOBAL::fork` in `DB::sub` so the hook is not wrapped. Child `exit` runs `END` → `finish_profiler`. ParseXS rejects column-1 `#ifdef` inside XSUB CODE — keep v6/v5 reinit in a C helper. | no | — |
| 2026-08-12 | perl | prefix `nytprof-engine` | Installed `$PREFIX/bin/nytprof-engine` can still `find_repo_root` via Cargo.toml | `EngineDispatch::find_repo_root` requires `Cargo.toml` + `crates/nytprof-cli`. A product prefix has neither. I03: add `@INC` `../lib/perl5` + `../lib`; on failed walk treat prefix as repo for sibling `bin/nytprof-cli`. `query --json --jsonl` must stay cargo-free. | no | — |
| 2026-08-12 | perl | J01 MakeMaker NAME | Facade `NAME => NYTProf::Modernization::PackagingEntry` / `0.001` can stay as advertised CPAN identity | KD-16/17: advertised dist is **`Devel::NYTProf` ≥ 7.00**. `VERSION_FROM` `collector/xs/Devel/NYTProf.pm` (already `$VERSION = '7.00'`). `make manifest` + `MANIFEST.SKIP` must drop `baseline/` `target/` `prefix/`. `CPAN-TRIAL-READY` is J02, not J01. | no | — |
| 2026-08-13 | perl | Option B identity | Product debugger stays `-d:NYTProf` / `Devel::NYTProf` ≥ 7.00 | **Option B:** `perl -d:NYTProfM` / `Devel::NYTProfM` **6.15**, CPAN `DISTNAME` **NYTProfM**. Operators switch. RPM `perl-NYTProfM`. Do **not** `Provides: perl(Devel::NYTProf)`. | no | — |
| 2026-08-13 | perl | `CORE:print` / `CORE:match` | The 3 extra `calls=2` events (of 27) are XSUB `OP_ENTERSUB` | They are **slowops**: `OP_PRINT` / `OP_MATCH` via `pp_slowop_profiler`; names `${CopSTASHPV}::CORE:` + `PL_op_name` when `slowops=2` (`subr_entry_setup` ~2457–2485; `slowops.h`). Thin XSUB-only ENTERSUB does **not** close 27. See [`docs/DROP_IN_RPM_COMPLETION_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/DROP_IN_RPM_COMPLETION_v0.md) KD-26/35. | no | — |

## Scope examples (what belongs here)

| Belongs | Does not belong |
|---------|-----------------|
| Perl `calls=1` vs `calls=2` SUB_ENTRY multiplicity | Generic “remember to use strict” |
| Rust fail-closed oversize length before alloc | Style-only clippy nits |
| Oracle `PERL5LIB` isolation (never `crates/`) | Unrelated crate ecosystem trivia |
| Tick/display vs exact integer counts (COMPAT-003) | Restating the whole COMPAT-001 field list |

## How to append (agents)

1. One row per distinct misconception (merge duplicates by editing the existing row).  
2. **Open?** = `yes` if still unresolved / needs ADR; `no` if corrected and settled.  
3. Prefer absolute links to contracts, schemas, or oracle paths.  
4. If the light row cannot carry the nuance (e.g. dual-equality timing), add `details/<slug>.md` and link it.

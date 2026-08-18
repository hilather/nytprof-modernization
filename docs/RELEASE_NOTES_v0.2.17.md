# NYTProfM v0.2.17 — opcode `entersub` default + aggregated `SUB_CALLERS`

**Tag:** `v0.2.17`  
**Date:** 2026-08-18  
**Since:** [`v0.2.16`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.16)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-11** (upgrades `6.15-10`). **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA. **Not** a certified perf claim.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.17/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.17/scripts/packaging/build_el8_module_rpm.sh).

```text
sudo rpm -Uvh perl-NYTProfM-6.15-11.el8.x86_64.rpm
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM script.pl
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
# rollback to Perl wrap if needed:
NYTPROF=file=/tmp/nytprof.out:wrap=1 perl -d:NYTProfM script.pl
```

## The change

Default `perl -d:NYTProfM` with `NYTPROF file=` now profiles **calls** through grafted C `OP_ENTERSUB` (and `OP_GOTO`) on the product sink (`nytp_emit_*`). `$^P` bit `0x01` stays off; `DB::sub` is a stub. The old Perl wrap is an escape: `wrap=1` (or `use_db_sub=1` / `entersub=0`). Product `use_db_sub=1` is a **wrap synonym**, not 6.15 stmt `DB::DB`.

`SUB_RETURN` is still one wire record per return. **`SUB_CALLERS` is aggregated in C** and flushed once per distinct `(fid, line, called, caller)` at finish, so default zlib is not compressing a 1:1 copy of every return.

Also in this cut (stacked on v0.2.16, not released separately):

- **PR-15:** default `stmts=1` `TIME_LINE` from C `OP_DBSTATE` (not Perl `DB::DB`); `INIT` leaves `$DB::single=0`.
- **PR-16:** C `wrap_push` / `wrap_pop` — now the **`wrap=1` escape** only.
- **DI-03 E2:** `OP_GOTO` so `goto &sub` keeps the original caller and the goto-site fid:line.
- **DI-03 E3:** opt-in `leave=1` → `nytp_emit_discount`. Product default `leave` stays **0**. `UNSTACK` stays with `blocks=1`.
- **DI-03 E4:** opt-in `slowops=full` / `=3`. Product `slowops=2` stays PRINT/MATCH.

## Changes since v0.2.16 (grouped)

**CLI / attach**

- Default call attach: C `OP_ENTERSUB` + `OP_GOTO`. Smoke: [`g17_entersub_attach_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.17/scripts/packaging/g17_entersub_attach_smoke.sh), [`g18_goto_sub_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.17/scripts/packaging/g18_goto_sub_smoke.sh).
- Escape: `NYTPROF=wrap=1` (or `use_db_sub=1` / `entersub=0`). Smoke: [`g16_wrap_enter_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.17/scripts/packaging/g16_wrap_enter_smoke.sh).
- C `OP_DBSTATE` `TIME_LINE`. Smoke: [`g15_dbstate_timeline_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.17/scripts/packaging/g15_dbstate_timeline_smoke.sh).
- `SUB_CALLERS` C table (`collector/xs/product_callers.c`); flush in `finish_profiler` before finalize. Smoke: [`g20_callers_agg_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.17/scripts/packaging/g20_callers_agg_smoke.sh).
- Opt-in `leave=1` / `slowops=full`. Defaults unchanged.

**Format / exclusive**

- No v5/v6 wire ID changes. Exclusive remains **incl − Σ child inclusive**. `SUB_RETURN` still carries ticks at return; aggregated `SUB_CALLERS` sums count/incl/excl (model already sums).

**Packaging**

- Module RPM **6.15-11**. Bundled EL8 `nytprofm-cli` unchanged (collector-only cut; source-sha256 matches).

## Engineering benches (claim: none)

Not BENCH certification. Same-host inner `Time::HiRes`, `stmts=0`, 120k leaf/mid loop: wrap **0.88s** / default opcode **0.12s** / isolated 6.15 **0.16s**. 25s paired scanner ([`compare_oracle_native_reports.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.17/scripts/field/compare_oracle_native_reports.sh) `--seconds 25`, same 2-file corpus): native **3468** passes / **433 KB** `nytprof.out`; oracle **1162** passes / **5.45 MB**. Native did more app work in the same wall (6.15 is heavier). See [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.17/docs/BENCH_NOTES.md).

## Known residuals

- Live `calls=2` `sub_entry_events` is **21** (emit after INIT); oracle golden **27** is `start=begin` (BEGIN/import).
- Product `leave` default stays **0** (not 6.15 `leave=1`).
- Product `slowops=2` stays PRINT/MATCH; full table is `slowops=full` / `=3` only (thin exclusive on `=full`, not 6.15 savestack).
- `collection_default` stays **v5**.
- Exclusive seconds vs 6.15 HTML are **not** a gate ([`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.17/AGENTS.md) §5).
- FFI, XS Data, COL-007 C v6 writer, multi-OS product certification: [residual matrix](https://github.com/hilather/nytprof-modernization/blob/v0.2.17/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md).

## Upgrade notes

- `dnf upgrade` / `rpm -Uvh` to **6.15-11**.
- Re-profile. Default attach is opcode; `caller()` is the real caller (wrap goto list is `wrap=1` only).
- Rollback: `NYTPROF=file=…:wrap=1`.
- Smaller call-heavy files come from aggregated `SUB_CALLERS` + existing `savesrc=0` / `stmts=0`.
- Field lab default HTML remains `--no-flame`; operator `nytprofm-cli html` still defaults flame **on** (v0.2.11).

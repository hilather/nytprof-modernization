# Product XS graft annex (v0)

**Board IDs:** `G03A-LOAD-ONLY` (load landed), `G03B-STMT-EMIT` (stmt emit-MVP landed), `G03C-SUB-EMIT` (sub emit-MVP landed), `G03D-META-EMIT` (meta/finalize emit-MVP landed), `G03E-COMPRESS-EMIT` (compress emit-MVP landed), `PRODUCT-XS-ATTACH-MVP` (G04 attach-MVP landed), `DROP-IN-DOD-V0` (this annex docs-landed), `G02-V5-PRODUCT-LINK` (scaffold landed)  
**Status:** **A.6 G02 landed** (`libnytp_sink_v5.a` + load-only `Devel::NYTProf::CollectorBootstrap`). **A.4 G03a load landed** (product `perl -d:NYTProf` loads; no `nytprof.out` on trivial `-e`). **A.3/A.4/A.5 G03b–G03e emit-MVP landed**. **G04 attach-MVP landed** (`NYTPROF file=`; E1b default `OP_ENTERSUB` on the product sink; wrap is `wrap=1`; default-calls1 leaf **15** / mid **3** / mid→leaf **15**). **G05 options/`format=v6` landed** (D1-B fail-closed; D1-A `NYTPROF6`). **G06 fork/`addpid` landed** (`CORE::GLOBAL::fork` → `nytp_fork_*` + addpid child file). **Not** full TEST-018 / mid-deflate-in-child / DI-03 done (E2 GOTO / E3 leave / E4 full slowops / live di02 **21** vs oracle **27**).  

**Binding for:** later G03a–G03e / G04 / G06 slices (implement from this annex, not overview prose alone)  
**Does not supersede:** [PROGRAM_CHARTER.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md), ADRs 0001–0009, [DROP_IN_DOD_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md)  
**Design:** [docs/PRODUCT_COMPLETION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PRODUCT_COMPLETION_DROP_IN_v0.md) (approved rev 4)

Extracted from the approved design Annex A–C. Do **not** invent write-site mappings or clock rules beyond this annex.

---

## A.1 Provenance, license, security backports

| Item | Rule |
|------|------|
| Pin identity | Graft source = Devel::NYTProf **6.15** at `baseline/6.15/archives/` + oracle-commit metadata |
| License | Artistic-1.0-Perl OR GPL-1.0-or-later (match workspace `Cargo.toml` / upstream) |
| Import method | **Copy** into `collector/xs/` and product `lib/` with modernization delta commits — **do not** edit `baseline/6.15/src` as SoT (ADR-0004 rejects B0-B) |
| Provenance stamp | `docs/graft/PROVENANCE.md`: pin SHA, date, list of files copied, list of deltas |
| Security backports | Track upstream 6.15.x / security fixes; cherry-pick into graft tree; never rewrite pin archives |
| `make dist` | **Exclude** `baseline/`, `target/`, `prefix/`, large `collector/build/` — J01 enforces MANIFEST |

**ADR-0004:** the 6.15 pin remains archives + isolated install for **P-ORACLE** differential tests. Product graft lives under `collector/xs/` + product `lib/`.

---

## A.2 File inventory (copy vs rewrite vs defer)

| Path (oracle) | Product destination | Disposition |
|---------------|---------------------|-------------|
| `NYTProf.xs` (~5442) | `collector/xs/NYTProf.xs` | **Copy + rewrite write sites** → sink (phased G03a–e) |
| `FileHandle.xs` (~1565) | `collector/xs/FileHandle.xs` initially **or** omit | **Transition:** keep for Perl FileHandle API / read path; **production v5 write path becomes sink-only** after G03e (see A.4) |
| `lib/Devel/NYTProf.pm` | product `lib/Devel/NYTProf.pm` | Copy (debugger entry) |
| `lib/Devel/NYTProf/{Core,Run,Util,Constants,FileHandle,FileInfo,SubInfo,SubCallInfo,Reader,Apache,Test}.pm` | product `lib/…` | Copy; Apache = residual load (Open Q / ownership table) |
| `lib/Devel/NYTProf/Data.pm` / `ReadStream.pm` | See **Annex B** ownership | Do **not** blindly overwrite facade strategy (hybrid; one Data.pm = legacy default) |
| `bin/nytprofhtml` etc. | product `bin/` | Copy wrappers; may dispatch to native via EngineDispatch later |
| `collector/src/nytp_*.c` | link into XS | Already modernization SoT |
| `collector/t/*` | stay unit tests | Not product attach |

---

## A.3 Write-site → `nytp_emit_*` map

Oracle call sites in `NYTProf.xs` (via `NYTP_write_*` in `FileHandle.xs` / `FileHandle.c`) map to sink API in `collector/include/nytp_sink.h`:

| Oracle write site (approx.) | Logical event | Sink API | Phase |
|-----------------------------|---------------|----------|-------|
| `NYTP_write_header` + attributes | header / ATTRIBUTE | `nytp_emit_attribute` (+ v5 sink create writes `NYTProf 5 0\n`) | **G03d** |
| `NYTP_write_option_*` | OPTION | `nytp_emit_option` | **G03d** |
| `NYTP_write_comment` | comment | `nytp_emit_comment` | G03d optional (wrapper not required) |
| `NYTP_start_deflate_write_tag_comment` | START_DEFLATE | `nytp_emit_start_deflate` | **G03e** |
| `NYTP_write_process_start` | PID_START | `nytp_emit_pid_start` | **G03d** |
| `NYTP_write_process_end` | PID_END | `nytp_emit_pid_end` | **G03d** |
| `NYTP_write_new_fid` | NEW_FID | `nytp_emit_new_fid` | **G03d** |
| `NYTP_write_time_line` (~1588) | TIME_LINE | `nytp_emit_time_line` / `nytp_fast_emit_time_line` | **G03b** |
| `NYTP_write_time_block` (~1584) | TIME_BLOCK | `nytp_emit_time_block` / fast | **G03b** |
| `NYTP_write_discount` (~1710) | DISCOUNT | `nytp_emit_discount` | **G03b** (with clock gate) |
| `NYTP_write_call_entry` (~2621) | SUB_ENTRY | `nytp_emit_sub_entry` | **G03c** |
| `NYTP_write_call_return` (~2259) | SUB_RETURN | `nytp_emit_sub_return` | **G03c** |
| `NYTP_write_sub_info` (~3591) | SUB_INFO | `nytp_emit_sub_info` | **G03d** finalize |
| `NYTP_write_sub_callers` (~3667) | SUB_CALLERS | `nytp_emit_sub_callers` | **G04** (`DB::emit_sub_callers` + live `DB::sub`) |
| `NYTP_write_src_line` (~3756) | SRC_LINE | `nytp_emit_src_line` | **G03d** finalize |
| `NYTP_write_sawampersand` | attribute | `nytp_emit_attribute` | residual / G03d |
| Fork re-init paths (~1905+) | lifecycle | `nytp_fork_prepare` / `resume_*` (COL-015) | G06 |

**Batching:** Prefer `nytp_batch` facade already used in `collector/t/test_batch_fast.c` so statement path stays no-malloc after create (COL-004/005). Hooks call `nytp_emit_*` / `nytp_fast_emit_*` only — **no** per-event Rust/FFI (charter).

---

## A.4 FileHandle cutover: sink-only production v5 writes by G03b

| Stage | v5 production write path | FileHandle.xs role |
|-------|--------------------------|--------------------|
| **G03a** | **landed (load)** — in-memory `nytp_v5_sink_create(NULL)` held; `perl -d:NYTProf` loads product `Devel::NYTProf`; **no** `nytprof.out` on trivial `-e` | Optional link for symbols only (`libnytp_sink_v5.a` + `-lz`) |
| **G03b** | **landed (stmt emit-MVP)** — single path `nytp_sink_v5` via `nytp_emit_time_line` / `time_block` / `discount`; `DB::enable_sink($path)` is the product write path; fake-clock `nytp_m4_mini_sample_run` green | **No** FileHandle dual writer. Opcode hooks still absent (G04) |
| **G03c** | **landed (sub emit-MVP)** — single path `nytp_sink_v5` via `nytp_emit_sub_entry` / `sub_return`; `DB::emit_sub_*` writes a real `NYTProf 5` mini; dump has `SUB_ENTRY` / `SUB_RETURN` | **No** FileHandle dual writer. **No** opcode/entersub hooks. **Not** G04 fixture parity |
| **G03d** | **landed (meta/finalize emit-MVP)** — single path `nytp_sink_v5` via `nytp_emit_attribute` / `option` / `new_fid` / `src_line` / `sub_info` / `pid_start` / `pid_end`; dump has the seven tags | **No** FileHandle dual writer. **No** opcode hooks. **Not** G04 fixture parity |
| **G03e** | **landed (compress emit-MVP)** — v5 sink + `nytp_emit_start_deflate` (COL-006; tag `z` then zlib; `-lz` only); dump/verify inflate recovers a post-deflate event | **No** FileHandle dual writer. **Residual:** mid-deflate fork. **No** opcode hooks |
| **G04** | **landed (attach-MVP)** — `NYTPROF file=` + E1b default `OP_ENTERSUB` emit `SUB_RETURN` / `SUB_CALLERS` / `TIME_LINE` via shipped `nytp_emit_*`; wrap is `wrap=1`; default-calls1 leaf **15** / mid **3** / mid→leaf **15** | **No** FileHandle dual writer. **Not** DI-03 done (E2 GOTO / E3 leave / E4 full slowops). **Residuals:** G05/G06/blocks-780 landed separately |
| **Rejected long-term** | Dual write FileHandle **and** sink for same event | Dual maintenance + RSK-001; COL-001 acceptance is adapt every write site |

**Preferred end state:** production v5 I/O = `collector/src/nytp_sink_v5.c` only. FileHandle.xs may remain for **read**/legacy Perl API wrappers if required by pure-Perl modules; product collection must not depend on `NYTP_write_time_*` for the hot path after G03b.

**A7 contingency** (not preferred): G03a + legacy FileHandle v5 writes without sink cutover only if G03b slips. Default plan remains **sink cutover by G03b**.

---

## A.5 Clock / discount non-negotiables

From [baseline/inventories/timing-lifecycle-notes.md](https://github.com/hilather/nytprof-modernization/blob/main/baseline/inventories/timing-lifecycle-notes.md) (BASE-003):

1. **Do not move clock reads** around flushes without timing ADR + oracle test.
2. **DISCOUNT** placement must match 6.15 relative to internal work / flush (RSK-001).
3. **Fake-clock first:** `collector` fake-clock harness + TEST-003 mini green **before** claiming G03b complete. **G03b landed:** `make -C collector test` `test_fake_clock` + product `DB::run_m4_mini_sample` dump has TIME_LINE + DISCOUNT in mini order. **Not** G04 / default-calls1 fixture parity.
4. Gate order: **fake-clock mini stream equality (landed) → primary aggregate fixtures → full TEST-003 residual**.
5. I32 tick overflow: sink already fails closed on overflow (`NYTP_ERR_OVERFLOW`); G03b `DB::emit_time_line` / `overflow_probe` return status 4 (no silent wrap). Preserve 6.15 projection semantics (OI-003-01 residual honesty until frozen).

---

## A.6 `libnytp_sink_v5.a` mandatory for D1-B

**PR-G02 status (landed, scaffold only):** `make -C collector libnytp_sink_v5.a` produces [`collector/build/libnytp_sink_v5.a`](https://github.com/hilather/nytprof-modernization/blob/main/collector/Makefile) with the v5/batch/clock/fork/counting objects and **no** `nytp_sink_v6.o` / `nytp_sink_dual.o`. Link-proof: `make -C collector probe-v5` (`-lz` only). Load-only XS: `make -C collector xs-bootstrap` → `Devel::NYTProf::CollectorBootstrap` (not `-d:NYTProf`). Smoke: [`scripts/packaging/g02_v5_product_link_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g02_v5_product_link_smoke.sh). **This is not product attach.**

**Repo (`collector/Makefile`):** default `libnytp_sink.a` still archives **all** of `nytp_sink_v5.o`, `nytp_sink_v6.o`, `nytp_sink_dual.o`, batch/clock/fork, and `LDLIBS = -lz -lzstd -llz4`. That archive remains **dev/test-only** for the full collector unit suite (`make test`) — **not** a valid product v5-only link for KD-21 / D1-B.

| Artifact | Contents | Use |
|----------|----------|-----|
| `libnytp_sink.a` (current default) | v5 + v6 + dual + all objects; needs zstd/lz4 | **Dev/test only** (`make -C collector test`) — **forbidden** as sole MYEXTLIB for D1-B / default EL8 module |
| **`libnytp_sink_v5.a`** (**G02 landed** product target) | `nytp_sink.o`, `nytp_sink_v5.o`, `nytp_sink_counting.o`, `nytp_batch.o`, `nytp_clock.o`, `nytp_fork.o` — **no** `nytp_sink_v6.o` / `nytp_sink_dual.o`; link **`-lz` only** | **Mandatory** for D1-B and default EL8 module RPM |
| Selective `OBJECT=` in MakeMaker | Same object set as `libnytp_sink_v5.a` | Equivalent to static lib; acceptable alternative |
| Full product (D1-A) | v5 objects + `nytp_sink_v6.o` (+ dual only if test build); `-lz -lzstd -llz4` | CPAN default advertised-options / EL8 `--with v6_collect` |

```text
# Product MakeMaker — D1-B / EL8 default (KD-21) — REQUIRED shape
INC = -Icollector/include
# G02 landed:
#   make -C collector libnytp_sink_v5.a
MYEXTLIB = collector/build/libnytp_sink_v5.a
LIBS = -lz
# DO NOT: MYEXTLIB = collector/build/libnytp_sink.a   # pulls v6/dual + zstd/lz4
```

| Build flavor | Codecs linked | Module RPM on EL8 |
|--------------|---------------|-------------------|
| **v5-default product (D1-B)** | **zlib only** (`-lz`) via **`libnytp_sink_v5.a` / selective OBJECT** | **No** zstd/lz4 BuildRequires |
| **v6 collection enabled (D1-A)** | zlib + zstd + lz4 | Add `libzstd-devel`, `lz4-devel` (EPEL if needed) or `--with v6_collect` |

**G02 ≠ attach:** the v5-only archive + CollectorBootstrap load prove the D1-B link path. Live attach is G04 (`g04_v5_parity_smoke.sh`).

**G03a ≠ attach:** `make -C collector xs-nytprof` + product `perl -d:NYTProf` **load** (in-memory sink). No `nytprof.out` on trivial `-e`. `$PRODUCT_XS_ATTACH` stays false.

**G03b ≠ attach:** statement emit-MVP via `nytp_emit_*` (`$PRODUCT_STMT_EMIT=1`). Smoke [`g03b_stmt_emit_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03b_stmt_emit_smoke.sh). No opcode hooks; no G04 fixture parity.

**G03c ≠ attach:** sub emit-MVP via `nytp_emit_sub_*` (`$PRODUCT_SUB_EMIT=1`). Smoke [`g03c_sub_emit_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03c_sub_emit_smoke.sh). No opcode/entersub hooks; no G04 fixture parity.

**G03d ≠ attach:** meta/finalize emit-MVP via `nytp_emit_*` (`$PRODUCT_META_EMIT=1`). Smoke [`g03d_meta_emit_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03d_meta_emit_smoke.sh). No opcode hooks; no G04 fixture parity.

**G03e ≠ attach:** compress emit-MVP via `nytp_emit_start_deflate` (`$PRODUCT_COMPRESS_EMIT=1`). Smoke [`g03e_compress_emit_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03e_compress_emit_smoke.sh). Dump/verify inflate recovers a post-deflate event. Mid-deflate fork residual.

**G04 attach-MVP:** `NYTPROF file=` enables the file sink. **E1b:** default omit `entersub` installs `OP_ENTERSUB` (emit after INIT; `$^P` 0x01 off; `DB::sub` stub). Wrap is `wrap=1` (`use_db_sub=1` synonym) — that escape still sets `$^P` 0x01. **PR-7:** do not set `$DB::single` at enable. **PR-15:** default `stmts=1` installs C `OP_DBSTATE` TIME_LINE and leaves `$DB::single=0`. Live `perl -d:NYTProf` on `fixtures/v5/default-calls1/workload.pl`. Smoke [`g04_v5_parity_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g04_v5_parity_smoke.sh). Getopt/Exporter compile: [`g07_getopt_compile_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g07_getopt_compile_smoke.sh). C TIME_LINE: [`g15_dbstate_timeline_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g15_dbstate_timeline_smoke.sh). Wrap escape: [`g16_wrap_enter_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g16_wrap_enter_smoke.sh). Default opcode: [`g17_entersub_attach_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g17_entersub_attach_smoke.sh). `$PRODUCT_XS_ATTACH=1` only when `file=` is set. **Not** E2 GOTO / E3 leave / E4 full slowops / stock 6.15 XS.

---

## Annex B — Pure-Perl package ownership

Graft vs modernization packages. **One CPAN dist — one `Data.pm`.** Default = **6.15 legacy materializer** (API drop-in for `nytprofhtml` / scripts). Do **not** replace legacy Data with thin Jsonl-only Data and call it API drop-in.

| Package | Lines (approx.) | Source of truth for product | Strategy |
|---------|-----------------|----------------------------|----------|
| `Devel::NYTProf` | — | **6.15 graft** | Product debugger entry |
| `Devel::NYTProf::Core` | 166 | 6.15 graft | Bootstrap XS load |
| `Devel::NYTProf::Run` | 107 | 6.15 graft | |
| `Devel::NYTProf::Util` | 284 | 6.15 graft | |
| `Devel::NYTProf::Constants` | 44 | 6.15 graft | |
| `Devel::NYTProf::FileHandle` | 19 | 6.15 graft | |
| `Devel::NYTProf::FileInfo` | 615 | 6.15 graft | |
| `Devel::NYTProf::SubInfo` | 413 | 6.15 graft | |
| `Devel::NYTProf::SubCallInfo` | 26 | 6.15 graft | |
| `Devel::NYTProf::Reader` | 596 | 6.15 graft | Legacy HTML/report path |
| `Devel::NYTProf::Apache` | 255 | 6.15 graft **residual** | Not first GA tier; ship file but document untested **or** omit from MANIFEST until tested |
| `Devel::NYTProf::Test` | 15 | 6.15 graft / optional | Dev only |
| `Devel::NYTProf::Data` | oracle 798 / facade 347 | **Hybrid** | **Default = 6.15 legacy materializer**; optional native backend via `engine=native` / thin bridge **without** claiming COMPAT-007 until PERL-005 |
| `Devel::NYTProf::ReadStream` | oracle 227 / facade 241 | **Hybrid** | Legacy stream callbacks default; thin native-cli-jsonl remains available for product tests |
| `Devel::NYTProf::JsonlData` | 963 | **modernization** | Keep; not a 6.15 package |
| `Devel::NYTProf::JsonlReadStream` | 332 | **modernization** | Keep |
| `Devel::NYTProf::EngineDispatch` | 1203 | **modernization** | Keep; install as product report dispatcher |
| `Devel::NYTProf::LegacyBridge` | 500 | **modernization** | Keep; P-ORACLE / force-legacy bridge |

**Install layout (target, not shipped at G01):**

```text
$PREFIX/lib/perl5/Devel/NYTProf.pm
$PREFIX/lib/perl5/Devel/NYTProf/{Core,Run,Util,Reader,Data,ReadStream,...}.pm
$PREFIX/lib/perl5/Devel/NYTProf/{EngineDispatch,JsonlData,JsonlReadStream,LegacyBridge}.pm
$PREFIX/lib/perl5/auto/Devel/NYTProf/NYTProf.so
$PREFIX/bin/{nytprofhtml,nytprofcsv,...,nytprof-engine}
$PREFIX/bin/nytprof-cli   # only if dual-path native installed
```

**Tier limit:** Collection drop-in ≠ full API drop-in. `API-DATA-COMPAT007` remains residual.

---

## Annex C — EL8 / Rocky names (Option B)

| RPM | Contents | Drop-in? |
|-----|----------|----------|
| `perl-NYTProfM` | XS + pure-Perl + legacy scripts | **Yes** (collection + legacy tools) — when attach is actually green. **Parallel** to stock `perl-Devel-NYTProf` |
| `nytprof-cli` | Native CLI binaries | **No** — tools companion (`Recommends: perl-NYTProfM`) |
| Optional | `perl-NYTProfM-tools` meta Requires both | Convenience |

**Identity / Obsoletes:**

- Product `$VERSION` / RPM Version **6.15** (same number as stock; different **Name** — no EVR fight).
- Do **not** Provides stock `perl(Devel::NYTProf)`.
- **No self-Obsoletes** (`perl-NYTProfM < %{version}` confuses solvers).
- Obsoletes only for *other* names / aliases being retired.
- Default module RPM = **D1-B** (`libnytp_sink_v5.a`, `-lz` only) unless `--with v6_collect` → D1-A.

```text
Name:      perl-NYTProfM
Provides:  perl(Devel::NYTProfM) = %{version}
# Parallel to stock perl-Devel-NYTProf. Do NOT Provides perl(Devel::NYTProf).
# Do NOT self-Obsoletes: perl-NYTProfM < %{version}
```

**EL8 tools (KD-13):** signed CI prebuilt `nytprof-cli` artifacts — **not** rustup-in-mock. K03 ADR hard-gates K02.

**G01/A4 status:** Annex C is **docs-landed** (Option B). `EL8-RPM-MODULE` spec MVP is [`perl-NYTProfM.spec`](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/perl-NYTProfM.spec). Mock-certified / public COPR remain residual.

---

## Related (absolute)

| Doc | Role |
|-----|------|
| [DROP_IN_DOD_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md) | Binding D1–D6 + options matrix |
| [product-attach-smoke-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-attach-smoke-mvp-v0.md) | G03a load smoke; attach residual |
| [product-xs-attach-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-attach-mvp-v0.md) | G02 scaffold + G03a load + G03b stmt emit + G03c sub emit + G03d meta/finalize emit-MVP (attach residual) |
| [BUILD_SUPPORT_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) | P-ORACLE / P-PRODUCT-LEGACY / P-PRODUCT-DUAL + S0–S3 |
| [0004-collector-packaging-source-tree.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0004-collector-packaging-source-tree.md) | Do not edit `baseline/6.15/src` as SoT |

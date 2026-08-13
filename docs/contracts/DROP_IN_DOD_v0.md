# Drop-in definition of done (v0)

**Status:** docs-landed (PR-G01) — G03a load + G03b–G03e emit-MVP + G04 attach-MVP + G05 options/`format=v6` + G06 fork/`addpid` MVP landed; **not** full TEST-018 / mid-deflate-in-child / full 6.15 opcode  
**Board ID:** `DROP-IN-DOD-V0`  
**Date:** 2026-08-12  
**Approved design:** [docs/PRODUCT_COMPLETION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PRODUCT_COMPLETION_DROP_IN_v0.md) (rev 4, product answers frozen; identity superseded by Option B)  
**Rocky remaining-work SoT:** [docs/ROCKY8_DEPLOYMENT_REMAINING_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/ROCKY8_DEPLOYMENT_REMAINING_v0.md) (claim language after A4b)  
**Graft annex:** [docs/schemas/product-xs-graft-annex-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-graft-annex-v0.md)  
**Attach smoke schema:** [docs/schemas/product-attach-smoke-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-attach-smoke-mvp-v0.md)  
**Dual-path policy:** [docs/BUILD_SUPPORT_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md)  
**Residual matrix:** [docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md)

This contract extracts the binding drop-in DoD from the approved rev-4 design. It does **not** supersede the [program charter](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md), accepted ADRs 0001–0009, or the residual matrix.

**Explicit honesty:** G03a **load** is landed (`perl -d:NYTProfM` loads product `Devel::NYTProfM`; no `nytprof.out` on trivial `-e`). G03b–G03e **emit-MVP** remain. G04 **attach-MVP** is landed: live `perl -d:NYTProfM` with `NYTPROF file=` on a default-calls1-shaped program writes `NYTProf 5`; shipped dump/report shows leaf **15** / mid **3** / mid→leaf **15**. This is Perl `DB::sub`/`DB::DB`, **not** full 6.15 opcode/`entersub`. G05 **options + format=v6** tests are landed: unknown keys and `format=dual` fail-closed; D1-B `format=v6` fail-closed (`v6_collect` rebuild text, no `NYTPROF6` file); D1-A `xs-nytprof-v6` writes `NYTPROF6`; default/`format=v5` live attach still leaf **15** / mid **3** / mid→leaf **15**. G06 **fork/`addpid` MVP** is landed: live `fork` + `addpid=1` writes parent `NYTProf 5` and `<file>.<childpid>` `NYTProf 5` via shipped `nytp_fork_*`. **Residuals:** mid-deflate continue-in-child, full TEST-018, `POSIX::_exit` flush. DI-01 live **780/810** and DI-02 live **27** + CORE: names are landed (not full opcode).

**Claim language (A4b):** Rocky **collection attach** on default D1-B may be claimed only after A3 maintainer-mock + A5a unsigned-bootstrap honesty ([`docs/ROCKY8_DEPLOYMENT_REMAINING_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/ROCKY8_DEPLOYMENT_REMAINING_v0.md)). Until then: spec MVP, not mock-certified, not public COPR. Do **not** claim CPAN-TRIAL **upload**, full BUILD-003, tools-alone drop-in, or full 6.15 opcode. The module RPM ships I03 `nytprofhtml` wrappers (overwrite stock `/usr/bin` names); native html still needs `nytprof-cli`.

---

## Drop-in is D1–D6 — not a CLI-only RPM

Drop-in is **not** “CLI-only RPM.” A CLI-only package is a **tools companion**.

D1–D6 are **independent hard gates** for the GA claim (KD-1):

| ID | Dimension | Meaning |
|----|-----------|---------|
| **D1** | Attach | Product `perl -d:NYTProfM` loads from product install (not the 6.15 `Devel::NYTProf` pin) |
| **D2** | Fidelity | Mini stream equality + primary aggregate equality on advertised workloads |
| **D3** | Tools / report | Dump/verify/report/html/csv/export thresholds with residual honesty |
| **D4** | Install | CPAN source + EL8 RPM installability without repo `baseline/` or `crates/` on product `PERL5LIB` |
| **D5** | Dual-path | Three isolation profiles (P-ORACLE / P-PRODUCT-LEGACY / P-PRODUCT-DUAL) |
| **D6** | Honesty stamps | `product_xs_attach`, capability JSON, forbidden over-claims |

TRIAL may ship a **subset**. Packaging (D4) and dual-path (D5) **must gate TRIAL**, not only GA.

---

## Frozen key decisions (rev 4)

| ID | Decision |
|----|----------|
| **KD-1** | Drop-in = D1–D6; **not** CLI-only |
| **KD-2** | **CPAN primary** + **Rocky/EL8 RPM companion** (same sources) |
| **KD-13** | EL8 tools RPM from **signed CI prebuilts** (not rustup-in-mock; not system EL8 rustc) |
| **KD-16** | Product `$VERSION` / RPM Version **6.15** (match `baseline/6.15` pin). Parallel `NYTProfM` dist — no EVR fight with stock `Devel::NYTProf` |
| **KD-17** | CPAN/RPM name **`NYTProfM`**; debugger **`Devel::NYTProfM`** / `perl -d:NYTProfM` (Option B; operators switch) |
| **KD-16/17** | **`NYTProfM` / `Devel::NYTProfM` 6.15** (supersedes prior `Devel::NYTProf` ≥ 7.00) |
| **KD-21** | EL8 default module = **D1-B v5-only** (`libnytp_sink_v5.a` / selective `OBJECT`, **`-lz` only**). `format=v6` **fail-closed** on D1-B |
| **KD-24** | Product must **not** product-link full `libnytp_sink.a` for v5-only / D1-B (that archive is test-only: v6/dual + zstd/lz4) |
| **KD-5** | `collection_default: v5` until R4 flip |
| **KD-25** | `dual_path_smoke` stays oracle-primary until **S2**; do not rewrite the primary half before product is installable |
| **M01/Q4** | tablesorter / shared JS **WAIVE** for GA-candidate (doc residual, not CLOSE) — **PR-M01 landed (docs)**; jquery **not** shipped |

---

## D1 packaging flavors (D1-A vs D1-B)

| Flavor ID | Typical artifact | Linked sinks | D1 bar | `format=v6` behavior |
|-----------|------------------|--------------|--------|----------------------|
| **D1-A — full product** | CPAN dual-flavor / source build with `NYTPROF_V6_COLLECT=1` (default for **advertised-options GA** on CPAN); EL8 **`--with v6_collect`** rebuild | v5 + v6 (`-lz -lzstd -llz4`) | Full advertised-options matrix including v6 opt-in → `NYTPROF6` | **work** (G05) |
| **D1-B — v5-only module** | **Default** Rocky/EL8 `perl-NYTProfM` RPM (KD-21); optional CPAN `NYTPROF_V6_COLLECT=0` | **v5 only** (`-lz`; selective OBJECT / `libnytp_sink_v5.a`) | D1 **minus** v6 collection; all other advertised-options rows | **fail-closed**: croak/clear error *“format=v6 requires v6-enabled build (install v6_collect package or rebuild with --with v6_collect)”* — never silent ignore or partial write |

| Claim language | Requires |
|----------------|----------|
| “Drop-in collection (advertised options) on CPAN” | **D1-A** green |
| “Drop-in collection on Rocky 8 default RPM” | **D1-B** green (K01 `%check`); **not** automatic full D1-A |
| “Drop-in including Rocky 8 **with** `format=v6`” | K01 **v6_collect** subpackage/rebuild green (**D1-A** on EL8) **or** residual honesty that default RPM is D1-B only |

**PR-K01 `%check`:** exercise **D1-B** only (attach + `format=v5` + fail-closed `format=v6`). Do **not** require v6 file production for default K01 green.

Default EL8/module RPM **must not** product-link full `libnytp_sink.a` for D1-B (KD-24). G02 **landed** `libnytp_sink_v5.a` (scaffold / D1-B link path). That is **not** product attach.

---

## D1 options residual matrix (vs 6.15 `options[]`)

Sources: `baseline/6.15/src/NYTProf.xs` options table (~lines 249–283) + string options (`file`, `start`, `addpid`, `end`, `sigexit`, …).

| Option | 6.15 role | Attach-MVP (G03a–G03e) | Advertised drop-in GA | Residual / note |
|--------|-----------|------------------------|----------------------|-----------------|
| `file` | output path | **work** | work | |
| `start` | when to start | **work** (begin/no) | work | full start modes residual if not ported |
| `end` | end/finish modes | fail-closed or work subset | work subset documented | OI-003-04 lifecycle residual |
| `compress` / level | zlib after header | **live attach** `compress=1` → START_DEFLATE | work | mid-deflate fork residual |
| `stmts` | statement profiling | **work** (G03b) | work | |
| `blocks` | TIME_BLOCK | **live attach work** (DI-01 780/810; not full opcode) | work | G03b emit + PR-B1 live `blocks=1`; not DI-03 |
| `subs` | sub profiling | **work** (G03c) | work | |
| `calls` | 0/1/2 entry-return | **live attach work** (`calls=2` SUB_ENTRY **27** + CORE:print/match) | work | XSUB/goto/exception residual OI-003-03; not full opcode |
| `leave` | leave correction | residual or work | work if green else residual | |
| `slowops` | slow op profiling | residual | residual or work | fail-closed if unsupported |
| `usecputime` | removed in 6.15 | fail-closed / warn like 6.15 | same | 6.15 warns removed |
| `clock` | clock_id | work default clock | work | platform matrix residual |
| `trace` | debug trace | work or residual | residual OK | |
| `findcaller` | caller resolution | residual | residual | |
| `forkdepth` | generations | **work** subset (G06) | work subset | full TEST-018 residual |
| `addpid` | pid in filename | **work** (G06) | work | |
| `nameevals` / `nameanonsubs` | naming | residual | residual or work | |
| `evals` | eval profiling | residual | residual | |
| `sigexit` / posix exit | signal end | **live** `sigexit=1` → INT/TERM/HUP/PIPE flush | work subset | `_exit` / BUS/SEGV residual |
| `perldb` / embed options | debugger interaction | residual | residual | |
| **`format`** | **new product** | `v5` default; `v6` per **D1-A/D1-B**; `dual` **reject** | **D1-A:** v6 work; **D1-B:** v6 fail-closed | not a 6.15 option; see packaging flavors |
| unknown option | — | **fail-closed** (croak/warn+abort configure) | fail-closed | prefer over silent ignore |

**Attach tiers:**

| Tier | Required options working | Claim language |
|------|--------------------------|----------------|
| **Minimal attach** | `file`, `stmts`, default calls/subs enough for smoke script | “attach preview” |
| **Advertised-options attach** | All GA “work” rows green; residual rows listed | “drop-in collection on advertised options” |
| **Full 6.15 options** | Every 6.15 option work | Not required for first GA |

---

## D2 — Collection fidelity (summary)

| Check | Pass criteria |
|-------|---------------|
| **Stream equality (mini)** | Product-collected v5 on TEST-003 mini / M4-mini shaped stream: canonical dump structural equality after normalize (COL-006 bar) |
| **Aggregate equality (primary)** | On default-calls1 / calls2-default / blocks-calls1 **shaped** product workloads: leaf **15** / mid **3** / mid→leaf **15**; blocks line5 **780**; discount **818** where applicable — until **complete TEST-003** residual closes |
| E4 scaled | Dual-sink or sequential v5+v6 product: E4 product CLI green |
| Fork MVP | COL-015 product hooks; full TEST-018 residual honesty |
| Fail-closed | COMPAT-010 incomplete never OK |

Aggregates alone can hide ordering/discount/deflate bugs — mini stream equality is **mandatory** before advertised-options attach. Full fixture stream equality remains residual until TEST-003 complete.

---

## D3 — Tools / report thresholds (summary)

| Surface | Drop-in threshold | Residual honesty |
|---------|-------------------|------------------|
| dump / verify / report | Semantic counts exact | COMPAT-003 ticks |
| html multi-file | CSS + excl + optional flame MVP | Not oracle DOM/JS/tablesorter/Graphviz/treemap. **M01/Q4:** tablesorter/shared JS **WAIVE** for GA-candidate — **PR-M01 landed (docs)**; jquery **not** shipped |
| csv / folded / callgrind | Semantic leaf/mid/edge | Not full `nytprofcg` bytes |
| convert / merge / salvage | Capability true; strict convert | L01 lossy + L02 aggregate-sum MVP; full nytprofmerge option parity residual |
| Perl Data/ReadStream | Thin product path for advertised queries | **API drop-in residual:** COMPAT-007 bless-array; pure-XS decode (`API-DATA-COMPAT007`) |

---

## D4 — Packaging installability (summary)

| Form | Pass |
|------|------|
| CPAN source | `perl Makefile.PL && make && make test && make install` — **no Cargo** for collection + legacy report |
| Optional native | `NYTPROF_NATIVE=1|auto` installs CLI when cargo/prebuilt present |
| RPM EL8 | Module package attach smoke in mock; tools package separate |
| Clean env | No repo `baseline/` or `crates/` required on product `PERL5LIB` |

**KD-2:** CPAN is primary; Rocky/EL8 RPM is a companion of the **same sources**.

---

## D5 — Dual-path: three isolation profiles (summary)

| Profile ID | Name | `PERL5LIB` / load path | Cargo? | Purpose |
|------------|------|------------------------|--------|---------|
| **P-ORACLE** | Oracle differential | `baseline/6.15/install` (+ optional `test-deps`) **only** | Never | Fixtures, dump compare, oracle `t` subset |
| **P-PRODUCT-LEGACY** | Product legacy-only | Product install prefix (`site`/`prefix/lib`) — **XS + pure-Perl** | **Never** | **RSK-009 product collection proof** |
| **P-PRODUCT-DUAL** | Product dual-path | Product prefix + discoverable `nytprof-cli` | Optional | Accelerated report/convert |

**Rules:**

1. Never put `crates/`, product install, or `collector/` on **P-ORACLE** `PERL5LIB`.
2. Never use P-ORACLE as the ship/install path (ADR-0004).
3. `legacy_only_smoke.sh` **remains P-ORACLE forever**; new `product_legacy_smoke.sh` proves **P-PRODUCT-LEGACY** attach without cargo.
4. After BUILD-003, operator “legacy-only” means **P-PRODUCT-LEGACY**, not “install the oracle pin.”
5. `dual_path_smoke.sh` stays oracle-primary until **S2** (KD-25). Smoke migration: **S0** (today / pre-installable) → **S1** (documented profiles; G01 skeletons) → **S2** (product installable) → **S3** (offline_gate expand).

Full profile + S0–S3 table: [BUILD_SUPPORT_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md#three-isolation-profiles--smoke-migration-s0s3).

---

## D6 — Capability / stamp honesty (summary)

| Mechanism | When cargo absent | When native present |
|-----------|-------------------|---------------------|
| Install stamp `product_xs_attach=1` | **Required** when D1 green | Required |
| `$Devel::NYTProfM::PRODUCT_XS_ATTACH` or `->VERSION` metadata | **Required** | Required |
| `nytprof-cli capability --json` | N/A / skip | Must not claim `collection_default: v6` pre-R4; optional key `product_xs_attach: true` when probes product tree |
| Forbidden claims | full SEC-002 (cargo-fuzz/AFL/deep corpus), independent SEC-012 sign-off, public P1–P4, full BUILD-006 multi-Perl/Windows, full oracle E4, COL-008 baseline, R5 retirement | same |

G01 attach smoke (no `file=`) must **not** print `OK: attach works`. I01 `product_legacy_smoke` proves prefix install+attach; it must **not** rewrite `dual_path_smoke.sh` (S2 not claimed).

---

## Marketing tiers (honest names)

| Tier | What operator gets | Claim language |
|------|-------------------|----------------|
| **Tools-only** | CLI RPM/prefix | “Native NYTProf tools” — **not** drop-in |
| **Attach-preview / TRIAL** | D1 + D4 + D5 + D6a; API/tools residuals listed | “Drop-in collection **preview** on advertised options” |
| **Collection drop-in (GA-candidate)** | D1–D2 + D4–D6 scoped by flavor (**D1-A** CPAN / **D1-B** Rocky default); D3 tools MVP; **API residual** explicit | “Drop-in **collection** on advertised tiers/options/**flavor**” — **not** “full API DOM parity”; Rocky default without v6 called out |
| **Full dual-path drop-in (GA)** | Collection drop-in + D3 thresholds + native optional | Same + accelerated tools; still list HTML JS / COMPAT-007 / merge / EL8 v6 residual |
| **Post-R4** | + `collection_default: v6` on eligible tiers | Only after ADR-0008 flip checklist |

Do **not** market “Full dual-path drop-in” without listing day-one residuals operators hit (API Data shapes, HTML JS, full nytprofmerge options).

**M01/Q4:** tablesorter / shared JS = **WAIVE** for GA-candidate. **PR-M01 landed (docs)** — documentation residual, not CLOSE implementation; jquery/tablesorter **not** shipped.

---

## Residual matrix row IDs

| Board / residual ID | Meaning | Flips when | G01 honesty |
|---------------------|---------|------------|-------------|
| `DROP-IN-DOD-V0` | This contract | G01 lands | **done (docs-landed)** |
| `G02-V5-PRODUCT-LINK` | v5-only product archive + load-only XS | G02 lands | **done (scaffold)** — not attach |
| `G03A-LOAD-ONLY` | Product `perl -d:NYTProf` loads; no `nytprof.out` on trivial `-e` | G03a lands | **done** — not collection attach |
| `G03B-STMT-EMIT` | `nytp_emit_time_*` / `discount` via product XS + fake-clock mini | G03b lands | **done** — not opcode attach / G04 |
| `G03C-SUB-EMIT` | `nytp_emit_sub_entry` / `sub_return` via product XS | G03c lands | **done** — not opcode attach / G04 |
| `G03D-META-EMIT` | `nytp_emit_attribute` / `option` / `new_fid` / `src_line` / `sub_info` / `pid_*` via product XS | G03d lands | **done** — not opcode attach / G04 |
| `G03E-COMPRESS-EMIT` | `nytp_emit_start_deflate` via product XS (zlib after `z`; `-lz` only) | G03e lands | **done** — mid-deflate fork residual; not opcode attach / G04 |
| `PRODUCT-XS-ATTACH-MVP` | Live collection attach (`-d:NYTProf` + default-calls1 15/3/15) | G04 green | **done (MVP)** — not full opcode |
| `PRODUCT-FORK-ADDPID-MVP` | Live `fork` + `addpid=1` parent + `<file>.<pid>` | G06 green | **done (MVP)** — not TEST-018 / mid-deflate-in-child |
| `PRODUCT-LEGACY-SMOKE` | P-PRODUCT-LEGACY without cargo | `product_legacy_smoke` green | **done (MVP)** — not BUILD-003-FULL / not S2 dual_path |
| `I02-MAKEMAKER-NATIVE` | MakeMaker `NYTPROF_NATIVE` CLI install | I02 smoke green | **done (MVP)** — not BUILD-003-FULL |
| `I03-DIST-SCRIPTS` | Familiar scripts + EngineDispatch prefix | I03 smoke green (15/3/15) | **done (MVP)** — not 6.15 nytprofhtml DOM / not BUILD-003-FULL / not S2 |
| `MIG01-MIGRATION-GUIDE` | Operator migration guide | `docs/MIGRATION_DROP_IN_v0.md` | **done (docs)** — not CPAN-TRIAL / not EL8 RPM |
| `K03-PREBUILT-CLI-ADR` | Signed CI prebuilt `nytprof-cli` policy | ADR-0010 | **done (docs)** — not K02 spec / not EL8 tools RPM |
| `J01-CPAN-HYGIENE` | Dist identity `NYTProfM` / `Devel::NYTProfM` **6.15** | J01 smoke green | **done (MVP)** — not CPAN-TRIAL / not PAUSE |
| `PRODUCT-OPTIONS-MATRIX` | Options residual matrix | G01 doc; G05 tests | **done (docs + tests)** |
| `PRODUCT-V6-COLLECT-EL8` | EL8 default RPM v6 collection | D1-A on EL8 via `--with v6_collect` **or** claim documents D1-B-only Rocky default | **residual** |
| `BUILD-003-FULL` | `full_build003=1` | I01–I02 | **residual** |
| `CPAN-TRIAL-READY` | CPAN TRIAL attach-preview notes | J02 notes + smoke | **done (notes-ready / MVP)** — **not** PAUSE uploaded |
| `EL8-RPM-MODULE` | EL8 D1-B module spec | K01 spec + smoke | **done (MVP)** — not mock-certified / not D1-A default / not tools RPM |
| `EL8-RPM-TOOLS` | EL8 nytprof-cli companion spec | K02 spec + smoke | **done (MVP)** — not signed-pipeline complete / not tools-alone drop-in |
| `M01-HTML-JS-WAIVE` | Shared JS / tablesorter WAIVE for GA-candidate | PR-M01 docs land | **done (docs)** — not jquery shipped |
| `P01-GA-CANDIDATE` | GA-candidate collection drop-in honesty | P01 notes + smoke | **done (MVP)** — not SEC-012 complete / not final GA / Rocky default D1-B only |
| `P02-SEC-CUT` | SEC-012 checklist + SEC-002 job MVP | P02 smoke green | **done (MVP / checklist / job)** — not independent sign-off / not full continuous fuzz / not GA marketing / not S2 |
| `TOOL-MERGE-AGGREGATE-SUM-MVP` | Opt-in `--aggregate-sum` merge | L02 smoke green | **done (MVP)** — not full nytprofmerge option parity / not S2 |
| `E4-02-ORACLE-PAIR-MVP` | Second oracle dual pair (blocks-calls1) | E4 smoke + pair files | **done (MVP)** — not full TEST-008 / not A4 780 attach / not S2 |
| `E4-03-ORACLE-PAIR-MVP` | Third oracle dual pair (calls2-default) | E4 smoke + pair files | **done (MVP)** — not full TEST-008 / not SUB_ENTRY 27 attach / not S2 |
| `SEC-012-CHECKLIST-MVP` | Release security review checklist | checklist file | **done (MVP / checklist)** — not independent sign-off |
| `SEC-002-CONTINUOUS-FUZZ-MVP` | Continuous-fuzz job wrapping existing batteries | wrapper + workflow | **done (MVP / job)** — not cargo-fuzz / AFL / deep corpus |
| `API-DATA-COMPAT007` | Bless-array residual | explicit residual until PERL-005 | **residual** |
| `NS-NYTPROFM-IDENTITY` | CPAN/RPM name **NYTProfM**, `$VERSION` **6.15**, `-d:NYTProfM` | identity slice | **done (MVP / Option B)** — operators switch; not PAUSE; report facades stay `Devel::NYTProf::*` |
| `DROP-IN-REMAINING` | Remaining advertised-options / GA drop-in | opcode + D2 + S2 + publish | **residual** — attach-preview MVP only. Completion design: [DROP_IN_RPM_COMPLETION_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/DROP_IN_RPM_COMPLETION_v0.md) |

**Not ready (do not advertise as shipped):** full 6.15 opcode/`entersub` attach, full TEST-018, mid-deflate-in-child, signed CI prebuilt **pipeline**, PAUSE upload, `BUILD-003-FULL`, independent SEC-012 sign-off, full SEC-002 cargo-fuzz/AFL. `EL8-RPM-MODULE` / `EL8-RPM-TOOLS` are **spec MVP** only (not mock-certified / not pipeline-complete). `CPAN-TRIAL-READY` is **notes-ready MVP** only (not uploaded). P02 is **checklist / job MVP** only (not GA marketing). G03a debugger **load**, G03b–G03e **emit-MVP**, G04 **attach-MVP**, G05 **options/`format=v6` tests**, and G06 **fork/`addpid` MVP** are shipped.

---

## Related (absolute)

| Doc | Role |
|-----|------|
| [docs/PRODUCT_COMPLETION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PRODUCT_COMPLETION_DROP_IN_v0.md) | Approved rev-4 design (body SoT) |
| [docs/schemas/product-xs-graft-annex-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-graft-annex-v0.md) | Binding graft annex A–C |
| [docs/schemas/product-attach-smoke-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-attach-smoke-mvp-v0.md) | G03a load smoke + attach residual honesty |
| [docs/BUILD_SUPPORT_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) | Three profiles + S0–S3 |
| [docs/MIGRATION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/MIGRATION_DROP_IN_v0.md) | Operator migration (MIG01) |
| [docs/contracts/SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md) | P02 SEC-012 checklist (not independent sign-off) |
| [docs/adrs/0010-signed-ci-prebuilt-native-cli.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0010-signed-ci-prebuilt-native-cli.md) | K03 signed prebuilt policy |
| [docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) | Residual honesty |
| [docs/PROGRAM_CHARTER.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md) | R0–R5 |

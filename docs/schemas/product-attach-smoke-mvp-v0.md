# Product attach / legacy smoke MVP (v0)

**Board IDs:** `G03A-LOAD-ONLY` (attach smoke load path), `G03B-STMT-EMIT` (separate stmt emit smoke), `G03C-SUB-EMIT` (separate sub emit smoke), `G03D-META-EMIT` (separate meta emit smoke), `G03E-COMPRESS-EMIT` (separate compress emit smoke), `PRODUCT-XS-ATTACH-MVP` (G04 attach-parity smoke), `PRODUCT-LEGACY-SMOKE`  
**Status:** **G03a load landed** via real `perl -d:NYTProfM` (this script; no `file=`). G03b–G03e emit-MVP are **separate** smokes. G04 live attach-parity is [`g04_v5_parity_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g04_v5_parity_smoke.sh). I01 **P-PRODUCT-LEGACY** is [`product_legacy_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/product_legacy_smoke.sh) (cargo-free prefix install + attach 15/3/15). Dual-path primary half stays **P-ORACLE** (S2 not claimed).  
**Scripts:**  
[`scripts/packaging/product_attach_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/product_attach_smoke.sh)  
[`scripts/packaging/product_legacy_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/product_legacy_smoke.sh)  
**DoD / annex:** [DROP_IN_DOD_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md) · [product-xs-graft-annex-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-graft-annex-v0.md)  
**XS schema:** [product-xs-attach-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-attach-mvp-v0.md)  
**Policy:** [BUILD_SUPPORT_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) (three profiles + S0–S3)

## Goal

Document the attach/legacy smoke contract. G03a flips the attach smoke from “load unimplemented skip” to **real debugger load**. Dual-path primary half stays **P-ORACLE** (`legacy_only_smoke.sh`).

This schema does **not** claim D1 collection attach green.

## Phase honesty

| Phase | Behavior | Exit |
|-------|----------|------|
| **G03a (now)** | When CC + Perl XS headers exist: `make -C collector xs-nytprof`, then isolated `perl -d:NYTProfM -e '…'` (product dest on `@INC` only). Assert exit 0, product `$INC`, `$PRODUCT_XS_LOAD`, **no** `nytprof.out`. | 0 |
| **No CC / no XS headers** | Honest `SKIP:` after asserting debugger sources exist | 0 |
| Wrapper misuse | Unknown flag | **2** (fail closed) |
| **Legacy smoke (I01)** | Cargo-free `install_product_xs.sh` + live `-d:NYTProf` 15/3/15 when CC/XS exist; `SKIP:` without toolchain | 0 |
| **S2** | Product prefix installable; `product_legacy_smoke` proves P-PRODUCT-LEGACY | 0 on product path |
| **G03b (separate smoke)** | `g03b_stmt_emit_smoke.sh` — explicit `nytp_emit_*` + dump; not this attach smoke | 0 on emit-MVP |
| **G03c (separate smoke)** | `g03c_sub_emit_smoke.sh` — explicit `nytp_emit_sub_*` + dump; not this attach smoke | 0 on emit-MVP |
| **G03d (separate smoke)** | `g03d_meta_emit_smoke.sh` — explicit `nytp_emit_*` meta/finalize + dump; not this attach smoke | 0 on emit-MVP |
| **G03e (separate smoke)** | `g03e_compress_emit_smoke.sh` — explicit `nytp_emit_start_deflate` + dump inflate; not this attach smoke | 0 on emit-MVP |
| **G04** | Separate [`g04_v5_parity_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g04_v5_parity_smoke.sh): live `-d:NYTProfM` + `file=` + dump/report **15/3/15**; **PR-3** also `SRC_LINE` / `SUB_INFO` + `sub_def` leaf/mid; `verify` still **OK:**. **Still not** full opcode. | 0 on attach-parity |

**Hard rules:**

- Do **not** wire these scripts into `dual_path_smoke.sh` or `offline_gate.sh`.
- Do **not** rewrite `dual_path_smoke.sh` primary half (`legacy_only_smoke.sh` remains required first half / **P-ORACLE forever**).
- Never invoke cargo as required; never put `crates/` on `PERL5LIB`.
- Isolated product `@INC` / `PERL5LIB` = `collector/build/xs-nytprof` **only** (never `baseline/6.15/install`, never `crates/`).
- Do **not** print `OK: attach works` or claim `product_xs_attach=1` while collection attach is residual.
- Do **not** skip-success when the product module is present and CC/XS headers exist.

## Flavor stub

| Input | Meaning |
|-------|---------|
| `PRODUCT_D1_FLAVOR=A` or `--flavor=d1-a` / `--flavor=A` | D1-A stub (full product / v6-capable claim later) |
| `PRODUCT_D1_FLAVOR=B` or `--flavor=d1-b` / `--flavor=B` | D1-B stub (**default** — EL8 v5-only) |

G03a still **prints** `flavor_stub: …`. It does not fork the D1-A vs D1-B link yet (product debugger is D1-B / `-lz` only).

## Greppable markers (required)

| Marker | Attach smoke (G03a) | Legacy smoke |
|--------|---------------------|--------------|
| Skip / not-yet | `SKIP:` when no CC/XS; else `G03a load-only` + `NOT-YET: G05` | `SKIP:` / `NOT-YET:` |
| Phase | `phase: S0/S1` | same |
| Flavor | `flavor_stub: d1-a` or `d1-b` | same |
| Attach honesty | `product_xs_attach: no` (or `not-ready`) | same |
| Load success (CC+XS) | `OK: G03a load` + real `perl -d:NYTProfM` | n/a |
| Residual ID | `NOT-YET: mid-deflate / TEST-018 / full opcode` | I01: `OK: P-PRODUCT-LEGACY install+attach` (or `SKIP:` without CC/XS) |
| Forbidden | `OK: attach works`, `product_xs_attach=1` | same |

## G03a load checks (when CC + XS headers exist)

| Check | Pass |
|-------|------|
| Debugger load | Real `perl -d:NYTProfM -e '…'` exits **0** |
| Module identity | `$INC{'Devel/NYTProfM.pm'}` contains `collector/build/xs-nytprof`; **must not** contain `baseline/6.15/install` |
| Load stamp | `$Devel::NYTProf::PRODUCT_XS_LOAD` true / printed marker |
| Attach stamp | `$Devel::NYTProf::PRODUCT_XS_ATTACH` **false** / 0 |
| Profile file | `nytprof.out` **absent** (temp cwd) |

## When attach is green (G04)

Live attach lives in [`g04_v5_parity_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g04_v5_parity_smoke.sh), **not** this G03a load smoke.

| Check | Pass |
|-------|------|
| Collection attach | `NYTPROF file=` + live `-d:NYTProfM` writes `NYTProf 5`; dump/report leaf **15** / mid **3** / mid→leaf **15** |
| Stamp | `$PRODUCT_XS_ATTACH=1` only in the `file=` session; G03a no-`file=` stays 0 |
| Flavor | D1-B: `format=v5` (this MVP). `format=v6` fail-closed remains G05 |
| Live finish (PR-3) | Dump `SRC_LINE` count **> 0** and `SUB_INFO` for `main::leaf` / `main::mid`; report `sub_def_leaf` / `sub_def_mid` present; `verify` still **OK:**. `savesrc=0` skips file source. **Still not** full 6.15 opcode / eval source / XSUB package-filename cache. |

**Residuals:** G06 fork, full 6.15 opcode/`entersub` / full `slowops.h`, `goto &$raw`. PR-8 measured stmt-ops `TIME_BLOCK` + PRINT/MATCH times ([`g08_slowops_times_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g08_slowops_times_smoke.sh), [`di01_blocks_780_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/di01_blocks_780_smoke.sh) tick diversity). G05 options/`format=v6` tests: [`g05_options_format_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g05_options_format_smoke.sh).

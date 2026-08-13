<!--
Status: docs-landed (PR-MIG01 + PR-A4 Option B identity)
Board: MIG01
Does not supersede: charter / ADRs 0001–0010 / DROP_IN_DOD
-->

**Status:** docs-landed (PR-MIG01 + **PR-A4 Option B identity**) — **not GA**  
**Board:** `MIG01`  
**Date:** 2026-08-13  
**Plan alignment:** REL-002-style operator migration ([`docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) C.6 in [`docs/PRODUCT_COMPLETION_DROP_IN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PRODUCT_COMPLETION_DROP_IN_v0.md); identity superseded by [`docs/DROP_IN_RPM_COMPLETION_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/DROP_IN_RPM_COMPLETION_v0.md) Option B)  
**Does not supersede:** [`docs/PROGRAM_CHARTER.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md), accepted ADRs 0001–0010, or [`docs/contracts/DROP_IN_DOD_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md)

# Operator migration — NYTProfM (Option B)

How to move between stock Devel::NYTProf **6.14/6.15** (`perl -d:NYTProf`), this product (**`NYTProfM` / `Devel::NYTProfM` `$VERSION` 6.15** / `perl -d:NYTProfM`), and native tools — without treating any flavor as “full drop-in.”

Operators **switch** at the name. The product is **parallel** to stock. It does **not** take the stock CPAN or RPM name.

This guide is **docs-landed**. It is **not** a CPAN TRIAL upload, **not** a public COPR ship notice, and **not** a GA claim.

---

## 1. Install paths

| Channel | Command / form | Status today |
|---------|----------------|--------------|
| **CPAN (primary)** | Dist **`NYTProfM`**, module **`Devel::NYTProfM`**, product `$VERSION` **6.15** | Identity frozen (Option B / KD-16/17 superseded). **`CPAN-TRIAL-READY` is notes-ready** — no TRIAL upload, no `cpanm NYTProfM` of this tree from PAUSE yet |
| **Rocky / EL8 RPM (companion)** | `dnf install perl-NYTProfM` | Spec MVP (`EL8-RPM-MODULE`). **Not** mock-certified / public COPR unless those rows land. Same sources as CPAN |

When those channels exist:

```text
# CPAN (after J01/J02 TRIAL — not uploaded now)
cpanm NYTProfM                # or: cpanm Devel::NYTProfM
                              # expect 6.15; refuse any 0.3.x / PackagingEntry $VERSION

# Rocky / EL8 — unsigned internal bootstrap (NOT a production policy)
# gpgcheck=0 is temporary until A5b live key (holder: hilather).
# /etc/yum.repos.d/nytprofm-internal.repo:
#   [nytprofm-internal]
#   name=NYTProfM internal (unsigned bootstrap)
#   baseurl=https://example.invalid/nytprofm/el8/
#   enabled=1
#   gpgcheck=0
dnf install perl-NYTProfM
```

The EL8 **module RPM** ships collection **and** I03 report wrappers: `nytprof-engine`, `nytprofhtml`, `nytprofcsv`, `nytprofcg`. Those names overwrite stock `/usr/bin/nytprofhtml` (etc.) if `perl-Devel-NYTProf` is also installed — test-drive with `rpm -Uvh --replacefiles` if `dnf` reports a file conflict. The RPM does **not** Obsoletes stock.

`nytprofhtml` is the **product** wrapper (exec sibling `nytprof-engine html`), **not** a new 6.15 DOM `nytprofhtml`. The same RPM ships an **unsigned Rocky 8** `nytprof-cli` next to the wrappers so native `html`/`csv` work without a second package. Cargo-free proof is still `nytprof-engine query --json --jsonl`. RPM signing / public COPR is **not** required for this test-drive.

Optional native (when BUILD-003 / I02 path is used): `NYTPROF_NATIVE=0` (default, cargo-free collection + legacy report), `=1` (require cargo/prebuilt), `=auto` (install CLI if present).

---

## 2. Coexistence with the stock package

Stock Rocky/EPEL may already ship **`perl-Devel-NYTProf` 6.14/6.15**. The product is a **different RPM name** (`perl-NYTProfM`) and does **not** replace stock via EVR.

| Rule | Operator meaning |
|------|------------------|
| **Parallel NEVRA** | Product Version **6.15** matches the stock number on purpose. Different **Name** avoids an EVR fight with distro 6.15 |
| **No Provides stock** | Do **not** `Provides: perl(Devel::NYTProf)` — solvers must not treat this as stock |
| **No self-Obsoletes** | Do **not** `Obsoletes: perl-NYTProfM < %{version}` on the same name |
| **Obsoletes only for other names** | Only when retiring an alias (`perl-NYTProfM-modern`, `nytprof-modernization-perl`, …) |
| **Provides** | `perl(Devel::NYTProfM) = %{version}` |

You **can** leave stock `perl-Devel-NYTProf` installed. The operator switch is the debugger name (`-d:NYTProfM` vs stock `-d:NYTProf`).

---

## 3. Rocky / EL8 flavors (D1-B default vs D1-A)

Default Rocky/EL8 `perl-NYTProfM` is **D1-B**: v5-only collection, linked **`-lz` only** (`libnytp_sink_v5.a` / selective `OBJECT`). It does **not** pull zstd/lz4.

| Flavor | Typical artifact | `format=v6` |
|--------|------------------|-------------|
| **D1-B** (default EL8 RPM) | `dnf install perl-NYTProfM` | **Fail-closed** (no file, no silent ignore) |
| **D1-A** | CPAN advertised-options build, or EL8 rebuild **`--with v6_collect`** | **Works** → `NYTPROF6` |

On a D1-B install, `NYTPROF=format=v6:…` must croak with **exactly**:

```text
format=v6 requires v6-enabled build (install v6_collect package or rebuild with --with v6_collect)
```

**How to get D1-A on EL8** (when the spec exists — K01 residual for mock `--with`):

```text
# rebuild / mock flavor (packager)
rpmbuild --with v6_collect …
# or install a v6_collect subpackage if the spec ships one
# (perl-NYTProfM+v6_collect / equivalent)

# BuildRequires then add libzstd-devel + lz4-devel (EPEL if needed)
# Runtime: libzstd + lz4 in addition to zlib
```

“Drop-in on Rocky 8 **default** RPM” means **D1-B**, not D1-A. “Drop-in including Rocky 8 **with `format=v6`**” needs the `--with v6_collect` path green.

---

## 4. `NYTPROF` options

`collection_default` remains **`v5`** until a gated R4 flip ([ADR-0008](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md)). `format` is a **product** option (not in 6.15).

| Option | Behavior |
|--------|----------|
| *(omit)* / `format=v5` | Default. Writes classic `NYTProf 5` |
| `format=v6` | **Flavor-gated:** D1-A writes `NYTPROF6`; D1-B fail-closed with the rebuild string above |
| `format=dual` | **Rejected** (fail-closed). Not a product operator mode |
| unknown keys | Fail-closed (no silent ignore) |
| `file=` | Output path (attach MVP) |
| `addpid=1` + `file=` | Product fork **MVP**: parent `NYTProf 5` plus `<file>.<childpid>` `NYTProf 5`. Not full TEST-018 / mid-deflate-in-child |

Examples:

```sh
# Default v5 (any flavor) — product debugger name
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM your_script.pl

# Explicit v5
NYTPROF=file=/tmp/nytprof.out:format=v5 perl -d:NYTProfM your_script.pl

# v6 — only on D1-A / --with v6_collect
NYTPROF=file=/tmp/nytprof.out:format=v6 perl -d:NYTProfM your_script.pl

# Fork MVP: parent file + per-child file.<pid>
NYTPROF=file=/tmp/nytprof.out:addpid=1 perl -d:NYTProfM your_script.pl
```

Stock/oracle collection remains `perl -d:NYTProf` under isolated `P-ORACLE` (`baseline/6.15/install` only). That is **not** this product.

Advertised-options vs residual rows live in [`docs/contracts/DROP_IN_DOD_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md). Full 6.15 opcode/`entersub` attach is **not** claimed.

---

## 5. Tools

| Entry | Role |
|-------|------|
| `nytprofhtml` / `nytprofcsv` | Familiar 6.15 names. After **I03**, product wrappers `exec` sibling `nytprof-engine html` / `csv`. Default engine is **native** — those actions need a discoverable `nytprof-cli`. Cargo-free I03 proof is `nytprof-engine query --json --jsonl`. Not 6.15 `nytprofhtml` DOM |
| `nytprof-engine` | Perl facade: `--engine=native\|legacy\|auto` for `report` / `verify` / `html` / `csv` / `query` / exports |
| `nytprof-cli html` (also `report`, `csv`, `verify`, `dump`, …) | Native CLI (tools companion / optional `NYTPROF_NATIVE`) |

```sh
# Cargo-free query (I03 prefix; no nytprof-cli required)
nytprof-engine query --json --jsonl readstream.jsonl

# Familiar names — default engine=native; need nytprof-cli on PATH / prefix/bin
nytprofhtml nytprof.out
nytprofcsv  nytprof.out

# Explicit engines
nytprof-engine --engine=legacy report nytprof.out
nytprof-engine --engine=native  html nytprof.out

# Native CLI directly
nytprof-cli html nytprof.out --out-dir ./report
```

`engine=legacy` is the one-step report escape (oracle / product-legacy path). Native HTML is MVP (CSS + excl + optional flame) — **not** oracle DOM. Tablesorter / shared JS are **WAIVE** for GA-candidate (M01), not CLOSE.

A CLI-only RPM is **“native NYTProf tools”**, not drop-in collection. Tools `Recommends: perl-NYTProfM`.

---

## 6. Rollback

| What you changed | How to go back |
|------------------|----------------|
| RPM fleet on product `perl-NYTProfM` | `dnf remove perl-NYTProfM` — stock `perl-Devel-NYTProf` is untouched |
| CPAN / local prefix | Uninstall `NYTProfM` / `Devel::NYTProfM`; stock `-d:NYTProf` remains if installed |
| Tried `format=v6` | `format=v5` or omit `format` (`collection_default` is v5) |
| Native report / HTML surprise | `engine=legacy` or `NYTPROF_ENGINE=legacy`; omit the tools RPM |
| EL8 v6 rebuild | Return to default D1-B package (no `--with v6_collect`) |

Do **not** `dnf downgrade` stock `perl-Devel-NYTProf` to undo this product — that was the pre-Option-B story.

v5 profile files remain valid across rollback. v6 files are **opt-in**; old 6.15 tools do **not** read them — convert or keep v5.

R3 (`engine=auto` as product default) and R4 (`collection_default: v6`) are **not** executed. Do not treat this guide as permission to flip them.

---

## 7. Profile compatibility

| Producer | Old 6.15 `nytprofhtml` / dump | New `nytprof-cli` / `nytprof-engine` |
|----------|-------------------------------|--------------------------------------|
| Stock or product **v5** (`NYTProf 5`) | **Readable** | **Readable** (magic auto-detect) |
| Product **v6** (`NYTPROF6`, D1-A only) | **Not** readable | Readable when `v6_decode` / `v6_report` are on |

- Default write path stays **v5** so mixed fleets (6.15 + product) keep working.
- v6 is **opt-in** and flavor-gated. Do not set `format=v6` as a site default.
- Convert/merge/salvage exist on the native CLI with residual honesty (lossy convert; full nytprofmerge aggregate-sum residual). v6→v5 may **refuse** unrepresentable data.

---

## 8. Dual-path isolation (operators and CI)

Never put repo **`crates/`** on oracle `PERL5LIB`. Also keep product install and `collector/` off the oracle pin.

| Profile | `PERL5LIB` | Use |
|---------|------------|-----|
| **P-ORACLE** | `baseline/6.15/install` only | Differential / 6.15 dump compare; stock `perl -d:NYTProf` |
| **P-PRODUCT-LEGACY** | Product prefix (XS + pure-Perl) | Cargo-free collection (`perl -d:NYTProfM`) + legacy report |
| **P-PRODUCT-DUAL** | Product prefix + discoverable `nytprof-cli` | Accelerated report/convert |

`dual_path_smoke.sh` is still **oracle-primary**. **S2** (primary half switches to P-PRODUCT-LEGACY) is **not executed**. After full BUILD-003, operator “legacy-only” means **P-PRODUCT-LEGACY**, not “install the 6.15 pin.”

Policy: [`docs/BUILD_SUPPORT_POLICY.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md).

---

## 9. Honesty (do not over-claim)

This document does **not** make these true:

| Residual / stamp | Meaning |
|------------------|---------|
| **Not `BUILD-003-FULL`** | Root `Makefile.PL` is not a complete XS CPAN dual-build (`full_build003=1` residual) |
| **Not `CPAN-TRIAL-READY` upload** | No `NYTProfM` **6.15** TRIAL on PAUSE |
| **Not EL8 RPM shipped to COPR** | `EL8-RPM-MODULE` spec MVP; `dnf install perl-NYTProfM` is the **intended** path when a repo exists |
| **S2 `dual_path` not executed** | Packaging smoke still P-ORACLE primary (KD-25) |
| **`collection_default: v5`** | Capability / stamps must not claim v6 default (pre-R4) |
| **M01 tablesorter / shared JS** | **WAIVE** for GA-candidate (doc residual, not CLOSE). Native HTML is not oracle DOM/JS |

Also not claimed: full 6.15 opcode attach, COMPAT-007 bless-array Data, COL-008, public P1–P4 SLOs, R3/R4 runtime flips, R5 retirement, `PRODUCT-V6-COLLECT-EL8` (default EL8 stays D1-B until `--with v6_collect` ships).

Do **not** market “full drop-in” without naming the **flavor** (D1-A vs D1-B) and the residuals operators actually hit.

---

## Related (absolute)

| Doc | Role |
|-----|------|
| [docs/contracts/DROP_IN_DOD_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md) | Binding D1–D6 / flavors / options |
| [docs/DROP_IN_RPM_COMPLETION_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/DROP_IN_RPM_COMPLETION_v0.md) | Option B identity + remaining completion plan |
| [docs/PRODUCT_COMPLETION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PRODUCT_COMPLETION_DROP_IN_v0.md) | Approved rev-4 design (identity superseded by Option B) |
| [docs/BUILD_SUPPORT_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) | Dual-path + S0–S3 |
| [docs/R1_PREVIEW_OPERATOR_RUNBOOK.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) | Offline operator runbook |
| [docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) | Residual honesty |
| [docs/PROGRAM_CHARTER.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md) | R0–R5 |
| [docs/R3_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) / [docs/R4_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md) | Gated flips (not executed) |
| [docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) | REL-002 and siblings |

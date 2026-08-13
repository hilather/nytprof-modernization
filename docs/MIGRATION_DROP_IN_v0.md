<!--
Status: docs-landed (PR-MIG01) — not GA
Board: MIG01
Does not supersede: charter / ADRs 0001–0009 / DROP_IN_DOD
-->

**Status:** docs-landed (PR-MIG01) — **not GA**  
**Board:** `MIG01`  
**Date:** 2026-08-12  
**Plan alignment:** REL-002-style operator migration ([`docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) C.6 in [`docs/PRODUCT_COMPLETION_DROP_IN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PRODUCT_COMPLETION_DROP_IN_v0.md))  
**Does not supersede:** [`docs/PROGRAM_CHARTER.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md), accepted ADRs 0001–0009, or [`docs/contracts/DROP_IN_DOD_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md)

# Operator migration — drop-in Devel::NYTProf (v0)

How to move between stock Devel::NYTProf **6.14/6.15**, this product (**`Devel::NYTProf` ≥ 7.00** identity), and native tools — without treating any flavor as “full drop-in.”

This guide is **docs-landed**. It is **not** a CPAN TRIAL, **not** an EL8 RPM ship notice, and **not** a GA claim.

---

## 1. Install paths

| Channel | Command / form | Status today |
|---------|----------------|--------------|
| **CPAN (primary)** | Coordinated name **`Devel::NYTProf`**, product `$VERSION` **≥ 7.00** | Identity frozen (KD-16/17). **`CPAN-TRIAL-READY` is residual** — no TRIAL upload, no `cpanm Devel::NYTProf` of this tree yet |
| **Rocky / EL8 RPM (companion)** | `dnf install perl-Devel-NYTProf` | **Residual** — `EL8-RPM-MODULE` not shipped. Same sources as CPAN when K01 lands |

When those channels exist:

```text
# CPAN (after J01/J02 TRIAL — not ready now)
cpanm Devel::NYTProf          # expect ≥ 7.00; refuse any 0.3.x product $VERSION

# Rocky / EL8 (after K01 — not shipped now)
dnf install perl-Devel-NYTProf
```

Collection + legacy scripts live in the **module** package (`perl-Devel-NYTProf` / CPAN dist). Native `nytprof-cli` is a **tools companion**, not drop-in by itself.

Optional native (when BUILD-003 / I02 path is used): `NYTPROF_NATIVE=0` (default, cargo-free collection + legacy report), `=1` (require cargo/prebuilt), `=auto` (install CLI if present).

---

## 2. Coexistence with the stock package

Stock Rocky/EPEL may already ship **`perl-Devel-NYTProf` 6.14/6.15**. The product keeps the **same RPM name**.

| Rule | Operator meaning |
|------|------------------|
| **EVR / Epoch upgrade** | Product Version **≥ 7.00** (optional Epoch if the packager needs it) so `dnf upgrade` replaces 6.15 |
| **No self-Obsoletes** | Do **not** `Obsoletes: perl-Devel-NYTProf < %{version}` on the same name — that confuses solvers |
| **Obsoletes only for other names** | Only when retiring an alias (`perl-Devel-NYTProf-modern`, `nytprof-modernization-perl`, …) |
| **Provides** | `perl(Devel::NYTProf) = %{version}` |

You do **not** install the product next to stock 6.15 under the same name. Upgrade or downgrade the one package.

---

## 3. Rocky / EL8 flavors (D1-B default vs D1-A)

Default Rocky/EL8 `perl-Devel-NYTProf` is **D1-B**: v5-only collection, linked **`-lz` only** (`libnytp_sink_v5.a` / selective `OBJECT`). It does **not** pull zstd/lz4.

| Flavor | Typical artifact | `format=v6` |
|--------|------------------|-------------|
| **D1-B** (default EL8 RPM) | `dnf install perl-Devel-NYTProf` | **Fail-closed** (no file, no silent ignore) |
| **D1-A** | CPAN advertised-options build, or EL8 rebuild **`--with v6_collect`** | **Works** → `NYTPROF6` |

On a D1-B install, `NYTPROF=format=v6:…` must croak with **exactly**:

```text
format=v6 requires v6-enabled build (install v6_collect package or rebuild with --with v6_collect)
```

**How to get D1-A on EL8** (when the spec exists — K01 residual):

```text
# rebuild / mock flavor (packager)
rpmbuild --with v6_collect …
# or install a v6_collect subpackage if the spec ships one
# (perl-Devel-NYTProf+v6_collect / equivalent)

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
# Default v5 (any flavor)
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProf your_script.pl

# Explicit v5
NYTPROF=file=/tmp/nytprof.out:format=v5 perl -d:NYTProf your_script.pl

# v6 — only on D1-A / --with v6_collect
NYTPROF=file=/tmp/nytprof.out:format=v6 perl -d:NYTProf your_script.pl

# Fork MVP: parent file + per-child file.<pid>
NYTPROF=file=/tmp/nytprof.out:addpid=1 perl -d:NYTProf your_script.pl
```

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

A CLI-only RPM is **“native NYTProf tools”**, not drop-in collection.

---

## 6. Rollback

| What you changed | How to go back |
|------------------|----------------|
| RPM fleet on product ≥ 7.00 | `dnf downgrade perl-Devel-NYTProf` → distro 6.15 |
| CPAN / local prefix | Reinstall prior 6.15 (or prior TRIAL when one exists) |
| Tried `format=v6` | `format=v5` or omit `format` (`collection_default` is v5) |
| Native report / HTML surprise | `engine=legacy` or `NYTPROF_ENGINE=legacy`; omit the tools RPM |
| EL8 v6 rebuild | Return to default D1-B package (no `--with v6_collect`) |

v5 profile files remain valid across downgrade. v6 files are **opt-in**; old 6.15 tools do **not** read them — convert or keep v5.

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
| **P-ORACLE** | `baseline/6.15/install` only | Differential / 6.15 dump compare |
| **P-PRODUCT-LEGACY** | Product prefix (XS + pure-Perl) | Cargo-free collection + legacy report |
| **P-PRODUCT-DUAL** | Product prefix + discoverable `nytprof-cli` | Accelerated report/convert |

`dual_path_smoke.sh` is still **oracle-primary**. **S2** (primary half switches to P-PRODUCT-LEGACY) is **not executed**. After full BUILD-003, operator “legacy-only” means **P-PRODUCT-LEGACY**, not “install the 6.15 pin.”

Policy: [`docs/BUILD_SUPPORT_POLICY.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md).

---

## 9. Honesty (do not over-claim)

This document does **not** make these true:

| Residual / stamp | Meaning |
|------------------|---------|
| **Not `BUILD-003-FULL`** | Root `Makefile.PL` is not a complete XS CPAN dual-build (`full_build003=1` residual) |
| **Not `CPAN-TRIAL-READY`** | No coordinated `Devel::NYTProf` ≥ 7.00 TRIAL upload |
| **Not EL8 RPM shipped** | `EL8-RPM-MODULE` / `EL8-RPM-TOOLS` residual; `dnf install` above is the **intended** path |
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
| [docs/PRODUCT_COMPLETION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PRODUCT_COMPLETION_DROP_IN_v0.md) | Approved design (C.6 source) |
| [docs/BUILD_SUPPORT_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) | Dual-path + S0–S3 |
| [docs/R1_PREVIEW_OPERATOR_RUNBOOK.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md) | Offline operator runbook |
| [docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) | Residual honesty |
| [docs/PROGRAM_CHARTER.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md) | R0–R5 |
| [docs/R3_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) / [docs/R4_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md) | Gated flips (not executed) |
| [docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/19_ROLLOUT_RELEASE_AND_MIGRATION_TASKS.md) | REL-002 and siblings |

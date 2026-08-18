# NYTProfM v0.2.21 — statement TIME_LINE no longer includes hook/write cost

**Tag:** `v0.2.21`  
**Date:** 2026-08-18  
**Since:** [`v0.2.20`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.20)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-15**. **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA. **Not** a certified perf claim.

A tight statement such as `$abc = 1 if ($about == $is_about)` run millions of times no longer shows ~profiled wall (e.g. **3s**) while 6.15 shows the work (~**0.3–0.9s**). Product now matches 6.15 `DB_stmt` clock order: write the previous interval, **then restart the clock** after fid/emit so hook cost is not charged to the next line.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.21/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.21/scripts/packaging/build_el8_module_rpm.sh).

```text
sudo rpm -Uvh perl-NYTProfM-6.15-15.el8.x86_64.rpm
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM script.pl
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
```

## Changes since v0.2.20

**CLI / attach**

- Last-site TIME_LINE / TIME_BLOCK: `product_close_last_site` then `product_seed_last_site` after emit (6.15 `DB_stmt` order). Same-host 4e6 if-modifier: unprofiled **0.32s**, native line **2.94s → 0.50s** (profiled wall still ~2s — that is remaining hook overhead, not charged to the line).
- Smoke: [`g15_dbstate_timeline_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.21/scripts/packaging/g15_dbstate_timeline_smoke.sh) if-modifier TIME_LINE sum must stay under 55% of profiled wall; [`t/dbstate_timeline_attach.t`](https://github.com/hilather/nytprof-modernization/blob/v0.2.21/t/dbstate_timeline_attach.t).

**Packaging**

- Module RPM **6.15-15**. Bundled EL8 `nytprofm-cli` unchanged (collector-only cut; source-sha256 matches).

## Known residuals

- Profiled **process wall** is still larger than unprofiled (hook cost). 6.15 is often heavier on wall but reports smaller statement seconds because it excludes that cost from TIME_LINE.
- Exclusive on slowops remains **thin** (not 6.15 savestack).
- Live `calls=2` `sub_entry_events` is **21** (emit after INIT); oracle golden **27** is `start=begin`.
- Product `leave` default stays **0**. `collection_default` stays **v5**.
- Exclusive seconds vs 6.15 HTML are **not** a gate ([`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.21/AGENTS.md) §5).

## Upgrade notes

- `dnf upgrade` / `rpm -Uvh` to **6.15-16** ([`v0.2.22`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.22); sub incl/excl subtract last-site hook cost).
- Re-profile. Hot statement lines should track application work, not the collector write path.
- Rollback attach: `NYTPROF=file=…:wrap=1`.

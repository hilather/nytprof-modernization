# NYTProfM v0.2.22 — subroutine incl/excl subtract last-site hook cost

**Tag:** `v0.2.22`  
**Date:** 2026-08-18  
**Since:** [`v0.2.21`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.21)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-16**. **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA. **Not** a certified perf claim.

v0.2.21 stopped charging collector emit cost to **statement** TIME_LINE. Subroutine inclusive/exclusive still used raw wall (`now − t0`), so HTML sub times stayed near profiled wall. Product now matches 6.15 `incr_sub_inclusive_time`: accumulate the close-to-seed gap as `product_overhead_ticks` and subtract it from opcode `SUB_RETURN` / wrap `wrap_pop`.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.22/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.22/scripts/packaging/build_el8_module_rpm.sh).

```text
sudo rpm -Uvh perl-NYTProfM-6.15-16.el8.x86_64.rpm
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM script.pl
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
```

## Changes since v0.2.21

**CLI / attach**

- Opcode `product_incr_sub_inclusive_time` and wrap `wrap_pop` subtract `product_overhead_ticks − initial_overhead_ticks` (6.15 `cumulative_overhead_ticks`). KD-E13 superseded.
- Smoke [`g15_dbstate_timeline_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.22/scripts/packaging/g15_dbstate_timeline_smoke.sh): named-sub `do_ifmod` TIME_LINE **and** incl/excl must stay under 55% of profiled wall (opcode and `wrap=1`); floor so a zeroed SUB_RETURN cannot pass. [`t/dbstate_timeline_attach.t`](https://github.com/hilather/nytprof-modernization/blob/v0.2.22/t/dbstate_timeline_attach.t). g17 now requires the overhead symbols.

**Docs**

- Nested-hash / `for`-body lines that look ~2× on stock 6.15 vs NYTProfM are default `leave=1` writing a second TIME_LINE on the same line (`UNSTACK`; DISCOUNT affects **count** only). `leave=0` on both engines matches. Not a missed HELEM. [`leave-unstack-double-timeline`](https://github.com/hilather/nytprof-modernization/blob/v0.2.22/docs/agent-notes/details/leave-unstack-double-timeline.md).

**Packaging**

- Module RPM **6.15-16**. Bundled EL8 `nytprofm-cli` unchanged (collector-only cut; source-sha256 matches).

## Known residuals

- Profiled **process wall** is still larger than unprofiled (hook cost). Statement and sub seconds now exclude the last-site close-to-seed gap.
- Product `leave` default stays **0** (6.15 default is `1`). Do not flip default `leave=1` to chase 6.15 HTML on `for` bodies.
- Exclusive on slowops remains **thin** (not 6.15 savestack).
- Live `calls=2` `sub_entry_events` is **21** (emit after INIT); oracle golden **27** is `start=begin`.
- `collection_default` stays **v5**.
- Exclusive seconds vs 6.15 HTML are **not** a gate ([`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/v0.2.22/AGENTS.md) §5).

## Upgrade notes

- `dnf upgrade` / `rpm -Uvh` to **6.15-16**.
- Re-profile. Hot **sub** incl/excl should track statement work, not collector write time.
- Rollback attach: `NYTPROF=file=…:wrap=1`.

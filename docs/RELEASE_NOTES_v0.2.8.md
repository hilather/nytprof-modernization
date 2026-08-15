# NYTProfM v0.2.8 — smaller profiles, opt-in durable seals, collection-only RPM

**Tag:** `v0.2.8`  
**Date:** 2026-08-15  
**Since:** [`v0.2.7`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.7)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-3** (upgrades `6.15-2`). **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.8/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.8/scripts/packaging/build_el8_module_rpm.sh).

Download `perl-NYTProfM-6.15-3.el8.x86_64.rpm` from this release. Install **beside** stock `perl-Devel-NYTProf` (no `--replacefiles`):

```text
sudo rpm -Uvh perl-NYTProfM-6.15-3.el8.x86_64.rpm
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM your_script.pl
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
# optional call-tree flame (off by default):
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html --flame
# opt-in crash-safe mid-run snapshots:
NYTPROF=file=/tmp/nytprof.out:durable=1 perl -d:NYTProfM your_script.pl
```

`collection_default` stays **v5**. Omitted `compress` is zlib **level 6** (6.15 `HAS_ZLIB`). `compress=0` remains the opt-out.

Rocky 8 Docker lab (native, oracle 6.15, or both): [`scripts/field/rocky8_docker_profile_demo.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.8/scripts/field/rocky8_docker_profile_demo.sh).

## Changes since v0.2.7 (grouped)

**Profile size (6.15-compatible zlib)**

- Omitted `NYTPROF compress` ⇒ zlib level **6** (`z` + `windowBits=15`). `compress=1..9` is that zlib level.
- `%check` / `t/installed_attach.t` inflate `z` (64 MiB cap; nested `z` fail-closed) and assert omitted-compress CMF `0x78`.
- Field 25s scanner (engineering only, **claim: none**): same-run native `compress=0` **2.8 MiB** → omitted zlib-6 **461 KiB** (−84%). Oracle was already zlib.

**Durability (opt-in `durable=1`)**

- Live RAM stays uncompressed complete records. Seal is `tmp`+`fsync`+`rename` (`z`+`Z_FINISH` copy when compress≠0).
- `v5_flush` / `v5_close` are that idempotent publish. Periodic seal: 1s ∧ ≥256 KiB dirty; last-site is not flushed; I/O discounted.
- `DB::durable_seal_now` + [`di_durable_kill_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.8/scripts/packaging/di_durable_kill_smoke.sh): `kill -9` after a seal leaves dumpable `TIME_LINE`; torn live `z` never prints verify `OK:`.
- Default **`durable=0`**. Snapshot I/O cannot sticky-fail the live sink.

**Packaging (sit beside stock NYTProf)**

- Module RPM is **collection-only**: `Devel::NYTProfM` + XS + `nytprofm-cli` / `nytprofm-dump`.
- Does **not** install `nytprofhtml` / `nytprofcsv` / `nytprofcg` / `nytprof-engine` or `Devel::NYTProf::*` report facades.
- I03 Perl wrappers remain a prefix/dev path (`install_product_scripts.sh`), not this RPM.

**Perl facade**

- `NYTPROF=aggregate=1` fail-closes (ADR-0013 is **proposed**; not implemented).

**Native HTML**

- Shared CSS polish: surfaces, row hover, `#L` target highlight, `prefers-color-scheme: dark`. Still no jquery (M01 **WAIVE**).

## Residuals (do not claim)

Full 6.15 opcode / DOM/jquery/tablesorter (**WAIVE**) / COMPAT-007 Data / COL-007 C v6 writer / `_exit` flush / default-on `durable` / `aggregate=1` checkpoints / default-on flame / Graphviz image render / PAUSE / signed COPR / default Rocky `format=v6` / public perf or size certification.

## Docs

- [OPERATOR_PROFILE_SIZE_AND_DURABILITY_v0.md](https://github.com/hilather/nytprof-modernization/blob/v0.2.8/docs/OPERATOR_PROFILE_SIZE_AND_DURABILITY_v0.md)
- [0013-v5-coalesced-checkpoints.md](https://github.com/hilather/nytprof-modernization/blob/v0.2.8/docs/adrs/0013-v5-coalesced-checkpoints.md) (proposed)
- [collector-v5-wire-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/v0.2.8/docs/schemas/collector-v5-wire-mvp-v0.md)
- [MIGRATION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/v0.2.8/docs/MIGRATION_DROP_IN_v0.md)
- [R1_PREVIEW_OPERATOR_RUNBOOK.md](https://github.com/hilather/nytprof-modernization/blob/v0.2.8/docs/R1_PREVIEW_OPERATOR_RUNBOOK.md)
- [perl-NYTProfM.spec](https://github.com/hilather/nytprof-modernization/blob/v0.2.8/packaging/rpm/perl-NYTProfM.spec)

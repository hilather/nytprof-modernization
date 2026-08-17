# NYTProfM v0.2.14 — Memoize `caller` under attach

**Tag:** `v0.2.14`  
**Date:** 2026-08-17  
**Since:** [`v0.2.13`](https://github.com/hilather/nytprof-modernization/releases/tag/v0.2.13)

Unsigned **Rocky 8 / EL8** `perl-NYTProfM` testdrive RPM **6.15-8** (upgrades `6.15-7`). **Not** PAUSE, **not** COPR, **not** GPG-signed, **not** CI-mock certified, **not** GA.

GitHub Actions workflow [`Release EL8 RPM (test-drive)`](https://github.com/hilather/nytprof-modernization/blob/v0.2.14/.github/workflows/release-el8-rpm.yml) rebuilds the RPM in `rockylinux:8` and attaches it here. Local rebuild: [`scripts/packaging/build_el8_module_rpm.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.14/scripts/packaging/build_el8_module_rpm.sh).

```text
sudo rpm -Uvh perl-NYTProfM-6.15-8.el8.x86_64.rpm
NYTPROF=file=/tmp/nytprof.out perl -d:NYTProfM script.pl
nytprofm-cli html /tmp/nytprof.out --out-dir /tmp/nytprof-html
```

## The change

`Memoize::memoize('fn')` (and `unmemoize` / `flush_cache`) does `my $uppack = caller` then looks up `$uppack::fn`. v0.2.13 wrapped those CVs with `&$raw`, so `caller` was `DB` and Memoize croaked **`Cannot operate on nonexistent function \`fn'`** even though `main::fn` exists (works without `-d:NYTProfM`). This cut `goto`s `Memoize::` so `caller` is the real package.

## Changes since v0.2.13 (grouped)

**Perl attach (collector)**

- `_product_needs_goto` includes `Memoize::` (`memoize` / `unmemoize` / `flush_cache` / internals).
- Smoke: [`g12_memoize_caller_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/v0.2.14/scripts/packaging/g12_memoize_caller_smoke.sh). Test: [`t/memoize_attach.t`](https://github.com/hilather/nytprof-modernization/blob/v0.2.14/t/memoize_attach.t).

**Packaging / CI**

- Module RPM **6.15-8**. Prebuilt `nytprof-cli` ELF unchanged (source hash still matches; the fix is `.pm`).
- Release attach uses `gh` with backoff after GitHub 503 killed `softprops/action-gh-release` on v0.2.13 (already on `main`; this tag includes it).

## Known residuals (unchanged)

- Exclusive seconds vs 6.15 are **not** a gate (`AGENTS.md` §5). Native HTML is MVP (no tablesorter / full DOM).
- FFI, XS Data, COL-007 C v6 writer, multi-OS product certification: [residual matrix](https://github.com/hilather/nytprof-modernization/blob/v0.2.14/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md).
- Full opcode / `entersub` / XSUB attach is still residual. Other caller-sensitive CPAN may still need a `goto` (same class as Memoize / Exporter / XSLoader).

## Upgrade notes

- `dnf upgrade` / `rpm -Uvh` to **6.15-8** to pick up the attach `.pm`. Scripts that already set `NYTPROF=file=…` need no flag change.
- If v0.2.13 / 6.15-7 died with `Cannot operate on nonexistent function` on a `memoize('…')` that works without the debugger, re-profile with this RPM (or in-tree `xs-nytprof`).
- Field lab default HTML remains `--no-flame`; operator `nytprofm-cli html` still defaults flame **on** (v0.2.11).

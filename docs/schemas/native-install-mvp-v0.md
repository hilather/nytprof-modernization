# Native install path MVP (v0)

**Status:** implemented (R1 packaging — stable on-disk CLI for Perl dispatch)  
**Not:** system-wide cargo install / CPAN dual release

## Install location (frozen)

```text
$REPO_ROOT/prefix/bin/nytprof-cli
```

Also acceptable aliases if both exist:
- `prefix/bin/nytprof-dump` (current Cargo binary name from package `nytprof-cli`)

## Install script

```sh
./scripts/packaging/install_native.sh
# optional: NYTPROF_PREFIX=... ./scripts/packaging/install_native.sh
# bare PREFIX=... also accepted when not the MakeMaker/local::lib default
```

Behavior:
1. `cargo build -q -p nytprof-cli` (release preferred if `NATIVE_RELEASE=1`, else debug)
2. Copy the built binary to `$PREFIX/bin/nytprof-cli` (and optionally `nytprof-dump` same file)
3. `chmod +x`
4. Print installed path

Default install root `$REPO_ROOT/prefix`. Prefer **`NYTPROF_PREFIX`** over bare `PREFIX` when calling from MakeMaker recipes (MakeMaker defines `PREFIX` and rewrites an exported `PREFIX` in child environments).

Shared resolution (also used by `install_facade.sh` so **dual-install cannot split roots**): [`scripts/packaging/resolve_packaging_prefix.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/resolve_packaging_prefix.sh).

1. `NYTPROF_PREFIX` always wins (trailing `/` stripped).  
2. Bare `PREFIX` only if, after stripping one trailing `/`, it is **not** `$HOME/perl5` and **not** any path ending in `/perl5` (MakeMaker/local::lib denylist).  
3. Else `$REPO_ROOT/prefix`.

## Discovery order (`find_native_cli`)

0. If `$ENV{NYTPROF_FORCE_NO_NATIVE}` is truthy → fail immediately (**test hook only**; ENGINE-AUTO-FALLBACK)  
1. `$ENV{NYTPROF_NATIVE_CLI}` if executable  
2. `$REPO/prefix/bin/nytprof-cli` or `nytprof-dump`  
3. `$REPO/target/release/nytprof-dump` then `target/debug/nytprof-dump`  
4. `cargo run -q -p nytprof-cli --` fallback  

## Smoke

```sh
./scripts/packaging/install_native.sh
./scripts/packaging/native_install_smoke.sh
# uses prefix binary via NYTPROF_NATIVE_CLI or find order for report leaf/mid 15/3
```

## Related: pure-Perl facade install (BUILD-003-DEPTH)

```sh
./scripts/packaging/install_facade.sh
# NYTPROF_PREFIX=... ./scripts/packaging/install_facade.sh
# or: perl Makefile.PL && make install-facade
# dual (needs cargo): make dual-install
```

Installs (default `$REPO_ROOT/prefix`; override with **`NYTPROF_PREFIX`**):

```text
$PREFIX/bin/nytprof-engine
$PREFIX/lib/Devel/NYTProf/*.pm
```

**No cargo required.** Combined native + facade regression: [`scripts/packaging/makemaker_build003_depth_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/makemaker_build003_depth_smoke.sh). Policy: [`docs/BUILD_SUPPORT_POLICY.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) (BUILD-003-DEPTH). **Not** full BUILD-003 XS CPAN dual-build.

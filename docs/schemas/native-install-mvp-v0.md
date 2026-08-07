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
# optional: PREFIX=... ./scripts/packaging/install_native.sh
```

Behavior:
1. `cargo build -q -p nytprof-cli` (release preferred if `NATIVE_RELEASE=1`, else debug)
2. Copy the built binary to `$PREFIX/bin/nytprof-cli` (and optionally `nytprof-dump` same file)
3. `chmod +x`
4. Print installed path

Default `PREFIX=$REPO_ROOT/prefix`.

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

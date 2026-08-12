# Engine / backend selection MVP (v0)

**Status:** R1 packaging wave — operator control for native vs legacy  
**Perl facade:** true `auto` prefer-native / fall-back-legacy (ENGINE-AUTO-FALLBACK).  
**Product default (omit flag/env):** still **`native`** on the facade until an executed R3 flip.  
**R3 policy:** [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) accepted; runtime flip **not executed** — procedure [R3_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md).  
**Not:** claiming charter R3 complete, or pure-Rust `nytprof-cli` dual-path legacy.

## Names (frozen for this wave)

| Mechanism | Values | Notes |
|-----------|--------|-------|
| CLI flag | `--engine=<name>` | Preferred for `nytprof-cli` |
| Environment | `NYTPROF_ENGINE=<name>` | Used when flag omitted |
| Precedence | CLI flag **overrides** env | Unset flag + unset env → **`native`** (facade product default today; pure-Rust CLI same). Post-R3 flip (gated): facade product default becomes **`auto`** per ADR-0005 — **not** live until flip checklist completes |

### Engine names

| Name | Behavior |
|------|----------|
| `native` | Run the real Rust decode/model/report/verify path (default when flag/env omitted). On the **Perl facade**, missing native CLI → **fail** (clear error). On pure-Rust `nytprof-cli`, always the native path. |
| `auto` | **Perl facade (`nytprof-engine`):** prefer native when the CLI is discoverable; if not, **fall back to legacy** (oracle stream-dump; STDERR note `auto: native CLI not found; using legacy`). See ENGINE-AUTO-FALLBACK. **Rust `nytprof-cli` residual:** still maps `auto` → `native` (no legacy half in-process); full dual-path auto is the shipped Perl surface. |
| `legacy` | **Perl facade:** oracle path under `baseline/6.15` (no Cargo). **Rust `nytprof-cli`:** do **not** run Rust report as a fake legacy engine — print a clear oracle message and exit **2** (or non-zero), or document/exec the oracle tool path without requiring Cargo. |
| anything else | Fail closed: error to stderr, non-zero exit. |

### Resolve vs runtime (Perl facade)

| Function | Role |
|----------|------|
| `resolve_engine($cli, $env)` | Returns **requested** name: `native` \| `legacy` \| `auto` (does **not** collapse `auto`). |
| `select_runtime_engine($repo, $requested)` | Concrete path: `legacy`→legacy; `native`→native; `auto`→ try `find_native_cli`, else legacy + STDERR note. |

Test hook (packaging only): `NYTPROF_FORCE_NO_NATIVE=1` makes `find_native_cli` fail immediately so auto→legacy can be exercised without renaming binaries.

### `auto` success contract (ENGINE-AUTO-SMOKE + ENGINE-AUTO-FALLBACK)

When native CLI is discoverable, both of the following must produce a real native report (or query) on default-calls1 with leaf **15** / mid **3**:

```sh
perl -Iperl/lib perl/bin/nytprof-engine --engine=auto report fixtures/v5/default-calls1/nytprof.out
NYTPROF_ENGINE=auto perl -Iperl/lib perl/bin/nytprof-engine report fixtures/v5/default-calls1/nytprof.out
```

When native is hidden (`NYTPROF_FORCE_NO_NATIVE=1`), auto report/verify must still exit **0** via legacy stream-dump (oracle PERL5LIB isolation; never `crates/`), with a STDERR fallback note — not a false native success.

Packaging smokes:

```sh
./scripts/packaging/engine_auto_smoke.sh
./scripts/packaging/engine_auto_fallback_smoke.sh
```

## Commands that honor `--engine` / `NYTPROF_ENGINE`

At minimum: `report`, `summary`, `html`, `csv`, `verify`, `inspect`.  
`dump` / `folded` / `callgrind` may also honor the flag (native-only; `legacy` still fails closed with message).

## Native success contract

```sh
cargo run -p nytprof-cli -- --engine=native report fixtures/v5/default-calls1/nytprof.out
# or: NYTPROF_ENGINE=native ...
```

Must show `main::leaf` / `main::mid` with returns **15** / **3**.

```sh
cargo run -p nytprof-cli -- --engine=native verify fixtures/v5/default-calls1/nytprof.out
# → OK: ...
```

## Invalid engine

```sh
cargo run -p nytprof-cli -- --engine=bogus verify fixtures/v5/default-calls1/nytprof.out
# → non-zero exit, clear error naming allowed values
```

## Legacy path (no Cargo)

Documented smoke: `./scripts/packaging/legacy_only_smoke.sh`  
Proves oracle isolation without calling `cargo`.

## Integration smoke (native CLI)

```sh
./scripts/packaging/engine_select_smoke.sh
```

Covers: `--engine=native` report (`main::leaf`, `returns=15`) + verify (`OK:`), `--engine=not-a-thing` non-zero, `--engine=legacy` non-zero with oracle/baseline message.
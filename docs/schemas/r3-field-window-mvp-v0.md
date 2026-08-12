# R3 field-window evidence pack MVP (v0)

**Board ID:** `R3-FIELD-WINDOW-PACK`  
**Status:** implemented (instrumentation + template only — **PR-D01**)  
**Not:** charter R3 product default flip, PR-D02 ADR, R4 format field window, external telemetry, public perf certification

## Goal

Define a **local, inspectable** directory layout and machine-readable summary for field evidence that `engine=auto` (Perl facade prefer-native / fall-back-legacy) and opt-in native reporting behave correctly — **without changing product defaults**.

Collector: [`scripts/field/r3_field_window_collect.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/r3_field_window_collect.sh)  
Smoke: [`scripts/field/r3_field_window_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/r3_field_window_smoke.sh)  
Human report: [`docs/templates/R3_FIELD_WINDOW_REPORT.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R3_FIELD_WINDOW_REPORT.md)  
Guide: [`docs/R3_FIELD_WINDOW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md)

## Non-goals

| Non-goal | Notes |
|----------|-------|
| Flip default engine/format | Product defaults unchanged; `summary.json` must record `no_default_flip: true` |
| Network telemetry | Packs stay on local disk; no upload by the collector |
| Replace offline_gate | Lab smokes remain separate; collector is field/lab evidence |
| Claim COL-007 / wire freeze / R4 | Explicit residual honesty |
| Store unredacted secrets | Operators must redact paths/source/PII before sharing packs |

## Pack layout

Root: `$OUT` (from `--out PATH`).

```text
$OUT/
  MANIFEST.md                 # human index + binding non-claims
  summary.json                # machine-readable roll-up (required fields below)
  SHA256SUMS                  # optional; written when sha256sum/shasum available
  env/
    provenance.txt            # uname, date, git, perl -v, tool discovery
  capability/
    capability.json           # nytprof-cli capability --json (when native present)
    capability.stdout.txt
    capability.stderr.txt
    capability.rc             # integer exit code
  runs/
    <run_id>.meta.json        # per-run metadata
    <run_id>.stdout.txt
    <run_id>.stderr.txt
    <run_id>.rc
  profiles/
    README.md                 # what was exercised (fixture paths; no blob copies by default)
```

### `run_id` naming (collector)

| Pattern | Meaning |
|---------|---------|
| `engine_auto_report_<label>` | Perl `nytprof-engine --engine=auto report` |
| `engine_auto_query_<label>` | Perl `nytprof-engine --engine=auto query` (when profile is golden JSONL-friendly or binary dump path works) |
| `engine_native_report_<label>` | Explicit `--engine=native report` |
| `engine_legacy_report_<label>` | Explicit `--engine=legacy report` (stream-dump smoke path) |
| `engine_auto_force_no_native_report_<label>` | `NYTPROF_FORCE_NO_NATIVE=1` + **auto** report (fallback exercise). **STDERR fallback note** required; **`rc==0` only when** `baseline/6.15/install` present — honest non-zero if pin install absent |
| `engine_native_force_no_native_report_<label>` | `NYTPROF_FORCE_NO_NATIVE=1` + **native** report (when native was discoverable). Must **fail closed** (non-zero `rc`); no silent legacy success |

`<label>` is a safe basename of the profile path (fixtures use e.g. `default-calls1`).

## `summary.json` (required fields)

Single JSON object, UTF-8, pretty-printed allowed.

| Field | Type | Meaning |
|-------|------|---------|
| `schema` | string | always `r3-field-window-mvp-v0` |
| `generated_at_utc` | string | ISO-8601 UTC |
| `git_commit` | string or null | `git rev-parse HEAD` when available |
| `site` | string or null | `--site` label |
| `note` | string or null | free-text operator note |
| `no_default_flip` | boolean | **must be `true`** — collector never changes defaults |
| `native_discoverable` | boolean | whether a native CLI was found before runs |
| `native_cli_spec` | string or null | discovery summary (`path:…` / `cargo` / null) |
| `profiles` | array of strings | profile paths exercised (repo-relative when under tree) |
| `runs` | array of objects | one entry per `runs/<run_id>.*` |
| `fixture_default_calls1` | object or null | when default-calls1 was run with native present: leaf/mid samples |
| `residuals` | object | fixed honesty flags (see below) |

### `runs[]` object

| Field | Type | Meaning |
|-------|------|---------|
| `id` | string | `run_id` |
| `engine_requested` | string | `auto` / `native` / `legacy` |
| `force_no_native` | boolean | whether test hook was set |
| `action` | string | `report` / `query` / `capability` / … |
| `profile` | string or null | path |
| `rc` | integer | process exit code |
| `stderr_fallback_note` | boolean | true if STDERR matched auto fallback note |
| `leaf_returns` | integer or null | grepped `main::leaf` returns when present |
| `mid_returns` | integer or null | grepped `main::mid` returns when present |

### `residuals` object (required keys)

```json
{
  "r3_product_default_flip": false,
  "r4_format_default_flip": false,
  "col007_product_writer": false,
  "v6_wire_freeze": false,
  "public_perf_certification": false
}
```

All values **must** be JSON `false` for packs produced by this MVP collector.

### Example (abridged)

```json
{
  "schema": "r3-field-window-mvp-v0",
  "generated_at_utc": "2026-08-11T12:00:00Z",
  "git_commit": "6590892133e77c80474a93decf12a02e4f963836",
  "site": "lab",
  "note": null,
  "no_default_flip": true,
  "native_discoverable": true,
  "native_cli_spec": "path:/repo/prefix/bin/nytprof-cli",
  "profiles": ["fixtures/v5/default-calls1/nytprof.out"],
  "runs": [
    {
      "id": "engine_auto_report_default-calls1",
      "engine_requested": "auto",
      "force_no_native": false,
      "action": "report",
      "profile": "fixtures/v5/default-calls1/nytprof.out",
      "rc": 0,
      "stderr_fallback_note": false,
      "leaf_returns": 15,
      "mid_returns": 3
    }
  ],
  "fixture_default_calls1": {
    "leaf_returns": 15,
    "mid_returns": 3,
    "auto_rc": 0,
    "native_rc": 0
  },
  "residuals": {
    "r3_product_default_flip": false,
    "r4_format_default_flip": false,
    "col007_product_writer": false,
    "v6_wire_freeze": false,
    "public_perf_certification": false
  }
}
```

## Semantic samples (default-calls1)

When native is discoverable and default-calls1 is included:

| Sample | Value |
|--------|-------|
| `main::leaf` returns | **15** |
| `main::mid` returns | **3** |

These match engine-auto and native report smokes ([`engine-selection-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/engine-selection-mvp-v0.md)).

## Isolation rules

1. Never put `crates/` on oracle `PERL5LIB`.
2. Legacy path uses oracle isolation already defined by `EngineDispatch` / packaging smokes.
3. Collector does not install, rewrite defaults, or mutate `baseline/6.15` archives.
4. `NYTPROF_FORCE_NO_NATIVE=1` is a **documented test/field-exercise hook** only (same as ENGINE-AUTO-FALLBACK).

## Redaction

Before sharing packs outside the trust boundary:

- Strip absolute paths that embed usernames or proprietary trees (or rewrite to tokens).
- Do not include full application source or raw production profiles with secrets.
- Prefer fixture-only lab packs for public CI artifacts.

## Smoke contract

```sh
./scripts/field/r3_field_window_smoke.sh
```

Must:

1. Run the collector on at least `fixtures/v5/default-calls1/nytprof.out` into a temp directory.
2. Assert `summary.json` parses and `no_default_flip === true`.
3. Assert all `residuals.* === false`.
4. When native is discoverable: assert default-calls1 auto report `rc==0` and leaf **15** / mid **3**.
5. When `engine_auto_force_no_native_report_*` is present: require STDERR auto-fallback note; require `rc==0` **only if** `baseline/6.15/install` exists (honest non-zero when oracle pin install is absent).
6. When native is discoverable and `engine_native_force_no_native_report_*` is present: require **non-zero** `rc` (fail closed; no silent legacy / no leaf **15** as success).
7. Exit non-zero on layout or honesty failures.

Not wired into `offline_gate.sh` (field package; packaging gate remains separate).

## Related schemas

| Schema | Role |
|--------|------|
| [engine-selection-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/engine-selection-mvp-v0.md) | `--engine` / `NYTPROF_ENGINE` names |
| [perl-engine-dispatch-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-engine-dispatch-mvp-v0.md) | Perl facade auto fallback |
| [capability-selftest-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/capability-selftest-mvp-v0.md) | capability JSON fields |

# R4 field-window evidence pack MVP (v0)

**Board ID:** `R4-FIELD-WINDOW-PACK`  
**Status:** implemented (instrumentation + template only — **PR-E01**)  
**Not:** charter R4 product format default flip, ADR-Q025 / REL-008 default-change ADR, R3 engine field window completion, external telemetry, public perf certification, lossy convert

## Goal

Define a **local, inspectable** directory layout and machine-readable summary for field evidence that **opt-in `format=v6`** (and R2-stable convert / report / verify tooling) behaves correctly — **without changing product defaults**. Collection default remains **v5**.

Collector: [`scripts/field/r4_field_window_collect.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/r4_field_window_collect.sh)  
Smoke: [`scripts/field/r4_field_window_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/r4_field_window_smoke.sh)  
Human report: [`docs/templates/R4_FIELD_WINDOW_REPORT.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R4_FIELD_WINDOW_REPORT.md)  
Guide: [`docs/R4_FIELD_WINDOW.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_FIELD_WINDOW.md)

## Non-goals

| Non-goal | Notes |
|----------|-------|
| Flip default engine/format | Product defaults unchanged; `summary.json` must record `no_default_flip: true` and `collection_default: "v5"` |
| Network telemetry | Packs stay on local disk; no upload by the collector |
| Replace offline_gate | Lab smokes remain separate; collector is field/lab evidence |
| Claim R4 complete / wire redesign | Explicit residual honesty |
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
  artifacts/
    <name>                    # convert outputs (local only; may be large)
  profiles/
    README.md                 # what was exercised (fixture paths; no blob copies by default)
```

### `run_id` naming (collector)

| Pattern | Meaning |
|---------|---------|
| `v5_report_<label>` | Native `report` on a **v5** profile (escape hatch / baseline) |
| `v6_report_<label>` | Native `report` on a **v6** profile (opt-in read) |
| `v6_verify_<label>` | Native `verify` on a **v6** profile |
| `convert_to_v6_<label>` | `convert --to=v6` from a v5 input |
| `convert_to_v5_<label>` | `convert --to=v5` from a v6 input |
| `report_after_convert_to_v6_<label>` | `report` on convert v5→v6 output |
| `report_after_convert_to_v5_<label>` | `report` on convert v6→v5 output |

`<label>` is a safe basename of the profile path (fixtures use e.g. `default_calls1_v5`, `default_calls1_v6`).

## Default lab fixtures

When `--no-default-fixture` is **not** set, the collector includes:

| Path | Role |
|------|------|
| `fixtures/e4/dual-sink/default_calls1_v5.nytprof` | Strict-convertible v5 dual-sink pair (E4) |
| `fixtures/e4/dual-sink/default_calls1_v6.nytprof` | Matching v6 dual-sink pair |

These are preferred over golden `fixtures/v5/default-calls1/nytprof.out` for convert exercises because some golden v5 timestamps may refuse strict convert (honest non-zero `rc` is correct; dual-sink pairs are the lab convert contract).

## `summary.json` (required fields)

Single JSON object, UTF-8, pretty-printed allowed.

| Field | Type | Meaning |
|-------|------|---------|
| `schema` | string | always `r4-field-window-mvp-v0` |
| `generated_at_utc` | string | ISO-8601 UTC |
| `git_commit` | string or null | `git rev-parse HEAD` when available |
| `site` | string or null | `--site` label |
| `note` | string or null | free-text operator note |
| `no_default_flip` | boolean | **must be `true`** — collector never changes defaults |
| `collection_default` | string | **must be `"v5"`** when capability ran successfully |
| `native_discoverable` | boolean | whether a native CLI was found before runs |
| `native_cli_spec` | string or null | discovery summary (`path:…` / `cargo` / null) |
| `profiles` | array of strings | profile paths exercised (repo-relative when under tree) |
| `runs` | array of objects | one entry per `runs/<run_id>.*` |
| `sizes` | object or null | byte sizes for dual-sink pair and convert outputs when present |
| `fixture_default_calls1` | object or null | leaf/mid samples when dual-sink default_calls1 exercised |
| `residuals` | object | fixed honesty flags (see below) |

### `runs[]` object

| Field | Type | Meaning |
|-------|------|---------|
| `id` | string | `run_id` |
| `action` | string | `report` / `verify` / `convert` / `capability` / … |
| `format_family` | string or null | `v5` / `v6` / `convert` / null |
| `profile` | string or null | input path |
| `output` | string or null | convert output path (repo- or pack-relative) |
| `rc` | integer | process exit code |
| `leaf_returns` | integer or null | grepped `main::leaf` returns when present |
| `mid_returns` | integer or null | grepped `main::mid` returns when present |
| `bytes_in` | integer or null | input size when measured |
| `bytes_out` | integer or null | output size when measured |

### `residuals` object (required keys)

```json
{
  "r4_format_default_flip": false,
  "r3_product_default_flip": false,
  "col008_batched_rust_writer": false,
  "lossy_convert": false,
  "public_perf_certification": false
}
```

All values **must** be JSON `false` for packs produced by this MVP collector.

### Example (abridged)

```json
{
  "schema": "r4-field-window-mvp-v0",
  "generated_at_utc": "2026-08-12T12:00:00Z",
  "git_commit": "ebb40cadc0ffee",
  "site": "lab",
  "note": null,
  "no_default_flip": true,
  "collection_default": "v5",
  "native_discoverable": true,
  "native_cli_spec": "path:/repo/target/debug/nytprof-dump",
  "profiles": [
    "fixtures/e4/dual-sink/default_calls1_v5.nytprof",
    "fixtures/e4/dual-sink/default_calls1_v6.nytprof"
  ],
  "runs": [
    {
      "id": "v6_report_default_calls1_v6",
      "action": "report",
      "format_family": "v6",
      "profile": "fixtures/e4/dual-sink/default_calls1_v6.nytprof",
      "output": null,
      "rc": 0,
      "leaf_returns": 15,
      "mid_returns": 3,
      "bytes_in": 1181,
      "bytes_out": null
    }
  ],
  "sizes": {
    "default_calls1_v5_bytes": 1548,
    "default_calls1_v6_bytes": 1181,
    "convert_to_v6_bytes": 1185,
    "convert_to_v5_bytes": 244
  },
  "fixture_default_calls1": {
    "leaf_returns": 15,
    "mid_returns": 3,
    "v5_report_rc": 0,
    "v6_report_rc": 0,
    "convert_to_v6_rc": 0,
    "convert_to_v5_rc": 0
  },
  "residuals": {
    "r4_format_default_flip": false,
    "r3_product_default_flip": false,
    "col008_batched_rust_writer": false,
    "lossy_convert": false,
    "public_perf_certification": false
  }
}
```

## Semantic samples (default_calls1 dual-sink)

When native is discoverable and dual-sink default_calls1 is included:

| Sample | Value |
|--------|-------|
| `main::leaf` returns | **15** |
| `main::mid` returns | **3** |

These match E4 dual-sink / CLI E5 report samples on the pair.

## Isolation rules

1. Never put `crates/` on oracle `PERL5LIB`.
2. Collector does not install, rewrite defaults, or mutate `baseline/6.15` archives.
3. Collector does not set product env vars that would change collection default.
4. Convert outputs land under `$OUT/artifacts/` only (pack-local).

## Redaction

Before sharing packs outside the trust boundary:

- Strip absolute paths that embed usernames or proprietary trees (or rewrite to tokens).
- Do not include full application source or raw production profiles with secrets.
- Prefer fixture-only lab packs for public CI artifacts.
- Convert artifacts under `artifacts/` may be omitted from shared archives when large.

## Smoke contract

```sh
./scripts/field/r4_field_window_smoke.sh
```

Must:

1. Run the collector on default dual-sink fixtures into a temp directory.
2. Assert `summary.json` parses and `no_default_flip === true`.
3. Assert `collection_default === "v5"`.
4. Assert all `residuals.* === false`.
5. When native is discoverable: assert dual-sink v5 and v6 report `rc==0` and leaf **15** / mid **3**; convert `--to=v6` and `--to=v5` `rc==0`; capability `convert`/`v6_decode`/`v6_report` true.
6. Exit non-zero on layout or honesty failures.

Not wired into `offline_gate.sh` (field package; packaging gate remains separate).

## Related schemas

| Schema | Role |
|--------|------|
| [cli-e5-v6-opt-in-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cli-e5-v6-opt-in-mvp-v0.md) | v6 opt-in CLI surfaces |
| [convert-strict-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/convert-strict-mvp-v0.md) | strict v5↔v6 convert |
| [capability-selftest-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/capability-selftest-mvp-v0.md) | capability JSON fields |
| [r3-field-window-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r3-field-window-mvp-v0.md) | sibling engine field pack (when present) |

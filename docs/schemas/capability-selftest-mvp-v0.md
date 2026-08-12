# Capability / self-test CLI MVP (v0)

**Board ID:** CAPABILITY-SELFTEST (+ CAPABILITY-JSON-MVP)  
**Status:** implemented (BUILD-005 first-slice MVP — offline native tools only; JSON structured form)  
**Not:** full capability manifest (codecs / ABI / target triple matrix), Perl integration negotiation, or release provenance (REL-003)

## Goal

Operators and packaging gates must be able to ask the **shipped native offline CLI** whether decode / report / verify paths are present and, when a golden fixture is available, that they actually load a real profile. Fail closed or report honestly — never put `crates/` on oracle `PERL5LIB`.

Machine-readable consumers (packaging gates, installers, agent harnesses) get a **structured JSON form** under `--json` / `--format=json` without losing the greppable human default.

## CLI

```text
nytprof-cli capability
nytprof-cli selftest          # alias
nytprof-cli capabilities      # alias
nytprof-cli capability [--json | --format=json | --format json]
nytprof-cli capability [--profile PATH]
nytprof-cli capability PATH   # bare path = forced probe
```

Also accepted as the binary name `nytprof-dump` (same package).

### Engine independence

`capability` / `selftest` / `capabilities` report **this binary's** native offline tools. They run even when `--engine=legacy` is set (legacy still applies to profile report/dump/verify subcommands only).

## Success output — human (exit 0, default)

Stable markers (order fixed; greppable). **Default must remain this form** when neither `--json` nor `--format=json` is given.

```text
OK: native capability self-test
decode: yes
report: yes
verify: yes
convert: yes
profile_ok: fixtures/v5/default-calls1/nytprof.out
```

or, when no probe path is available:

```text
OK: native capability self-test
decode: yes
report: yes
verify: yes
convert: yes
profile_ok: skip
```

| Line | Meaning |
|------|---------|
| `OK: native capability self-test` | Self-test entry succeeded |
| `decode: yes` | Native v5 decode is linked in this binary |
| `report: yes` | Native report path is linked |
| `verify: yes` | Native verify/inspect path is linked |
| `convert: yes` | Strict v5↔v6 convert is linked (PR-C01); dual-sink probe exercised when fixtures present |
| `profile_ok: <path>` | Optional probe: `verify` succeeded on that profile |
| `profile_ok: skip` | No probe path resolved (still exit 0) |

## Success output — JSON (exit 0)

Flags (any one selects JSON mode):

- `--json`
- `--format=json` (also `--format=JSON`)
- `--format json` (also `--format JSON`)

Stdout is a single JSON object (one line, no leading greppable `OK:` block). Required fields when exit 0:

| Field | Type | Meaning |
|-------|------|---------|
| `ok` | boolean `true` | Self-test succeeded |
| `decode` | boolean `true` | Native v5 decode is linked |
| `report` | boolean `true` | Native report path is linked |
| `verify` | boolean `true` | Native verify/inspect path is linked |
| `convert` | boolean `true` | Strict v5↔v6 convert is linked (PR-C01) |
| `profile_ok` | string path **or** `null` | Probe path that verified; `null` when no probe (human `skip`) |

Example (fixture present):

```json
{"ok":true,"decode":true,"report":true,"verify":true,"convert":true,"profile_ok":"fixtures/v5/default-calls1/nytprof.out"}
```

Example (no probe):

```json
{"ok":true,"decode":true,"report":true,"verify":true,"convert":true,"profile_ok":null}
```

JSON mode does **not** print the human `OK: native capability self-test` / `decode: yes` lines. Human default remains greppable for operators and existing smokes.

### Probe resolution order

1. Explicit `--profile PATH` / `-p PATH` / bare `PATH` argument  
2. Env `NYTPROF_CAPABILITY_FIXTURE`  
3. CWD-relative `fixtures/v5/default-calls1/nytprof.out`  
4. Repo-relative via compile-time `CARGO_MANIFEST_DIR` → workspace root + same fixture path  

If a probe path is resolved (including forced), **verify must succeed** or the self-test exits non-zero. Missing optional probe → human `profile_ok: skip` / JSON `profile_ok: null`, exit 0.

## Failure (non-zero exit)

- Forced or resolved probe path fails decode/model/verify  
- Probe produces a summary without an `OK:` line  
- Unknown option / unknown `--format` value  

Print error to stderr; exit status ≠ 0. Do not print a leading `OK: native capability self-test` success block (human) or an `ok: true` JSON object on failure (failure goes through the CLI error path).

Corrupt / incomplete fixtures follow the same fail-closed rules as `verify` (see [`verify-cli-mvp-v0.md`](verify-cli-mvp-v0.md), [`COMPAT-010_ERROR_FAIL_CLOSED.md`](../contracts/COMPAT-010_ERROR_FAIL_CLOSED.md)).

## What this is not

| Out of scope (this MVP) | Notes |
|-------------------------|--------|
| Codec / ABI / triple manifest | Full BUILD-005 / ARCH-006 later |
| Claiming native when CLI is absent | Packaging smoke must fail (or dual-path document skip) |
| Oracle / Perl self-test | Never mutates oracle `PERL5LIB` with `crates/` |
| Silent success on broken tools | If tools are claimed `yes`/`true`, a found fixture must verify |
| Pretty-printed multi-line JSON | Compact single-line object is the contract; pretty is optional/non-normative |

## Smoke

```sh
./scripts/packaging/capability_selftest_smoke.sh
```

Behavior:

1. Resolve native CLI (`NYTPROF_NATIVE_CLI` → `cargo run -p nytprof-cli` when cargo present → `prefix/bin` → `target/{debug,release}`).  
2. Run `capability` **twice**; both must exit 0.  
3. Assert both outputs contain the stable markers (`OK: native capability self-test`, `decode: yes`, `report: yes`, `verify: yes`, `convert: yes`) and that the two runs are **consistent** on those markers.  
4. In a repo checkout with the default golden fixture, assert `profile_ok:` is not `skip` (verify probe ran).  
5. Run `capability --json` **twice**; both exit 0; parse JSON (`python3 -m json.tool` or `JSON::PP`); assert `ok`/`decode`/`report`/`verify`/`convert` are true; `profile_ok` consistent across runs and non-null when the golden fixture is present.  
6. **No native binary and no cargo:** **fail** with a clear message (this is a packaging-native smoke). Dual-path / legacy-only gates do not require this smoke when cargo is absent — see packaging gate wiring.  
7. Never puts `crates/` on oracle `PERL5LIB`.

## Packaging gate / offline gate (optional wire)

`scripts/packaging/packaging_gate.sh` runs this smoke when cargo is present or a prefix/target binary already exists (native half). Legacy-only environments skip it with a clear note.

`scripts/ci/offline_gate.sh` step 9 (**CI-CAPABILITY-GATE**) uses the **same condition**: run when cargo **or** `prefix/bin/{nytprof-cli,nytprof-dump}` **or** `target/{debug,release}/nytprof-dump` **or** `$NYTPROF_NATIVE_CLI` is available; honest skip otherwise. After dual_path (step 3) with cargo, `prefix/bin` is typically installed so the step usually runs on developer hosts (after optional NATIVE-AGG-JSON / NATIVE-QUERY-JSON-CROSS steps 7–8).

## Operator examples

```sh
# From repo root (probes default golden fixture):
cargo run -q -p nytprof-cli -- capability
# or after install:
./prefix/bin/nytprof-cli capability
./prefix/bin/nytprof-cli selftest

# Structured JSON (CAPABILITY-JSON-MVP):
cargo run -q -p nytprof-cli -- capability --json
nytprof-cli capability --format=json

# Forced probe:
nytprof-cli capability --profile fixtures/v5/blocks-calls1/nytprof.out
nytprof-cli capability --json --profile fixtures/v5/blocks-calls1/nytprof.out
```

## Related

- Board: [`docs/FIRST_SLICE_BOARD.md`](../FIRST_SLICE_BOARD.md) (`CAPABILITY-SELFTEST`, `CAPABILITY-JSON-MVP`)  
- Plan: BUILD-005 (full manifest still proposed)  
- Native install: [`native-install-mvp-v0.md`](native-install-mvp-v0.md)  
- Verify: [`verify-cli-mvp-v0.md`](verify-cli-mvp-v0.md)  
- Dual-path policy: [`docs/BUILD_SUPPORT_POLICY.md`](../BUILD_SUPPORT_POLICY.md)  

# Verify / inspect CLI MVP (v0)

## CLI

```text
nytprof-cli verify <profile.out>
nytprof-cli inspect <profile.out>   # alias for verify
```

## Success (exit 0)

Decode profile, build model (or at least decode events), print a short human-readable summary, e.g.:

```text
OK: fixtures/v5/default-calls1/nytprof.out
  events: 2473
  TIME_LINE: 916
  TIME_BLOCK: 0
  files: 3
  subs: 10
```

Exact wording flexible; must contain `OK` (or `ok:`) and the path or basename.

## Failure (non-zero exit)

- Empty file
- Truncated golden profile bytes
- Bad header / unsupported tag during decode
- **Incomplete stream** (record-aligned short prefix): missing `PID_END` after `PID_START`, and/or no `TIME_LINE`/`TIME_BLOCK` statement timing — fail closed by default even if decode/model succeed

Print error to stderr; exit status ≠ 0.

Fail-closed policy (provisional COMPAT-010-ERR): see
[`docs/contracts/COMPAT-010_ERROR_FAIL_CLOSED.md`](../contracts/COMPAT-010_ERROR_FAIL_CLOSED.md).
Incomplete streams: [`docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md`](../contracts/COMPAT-010_INCOMPLETE_STREAM.md).
Corrupt / incomplete input must never panic or return silent success on default `verify` / `report`.

### Salvage (opt-in)

`NYTPROF_ALLOW_INCOMPLETE=1` allows verify to succeed on incomplete streams with an
`INCOMPLETE:` summary (not bare `OK:`). Dump remains lenient without the env.

## Library (optional)

```rust
pub fn verify_profile(path: &Path) -> Result<VerifyReport, Error>
```

Or implement entirely in CLI via `decode_path` + `ProfileModel::from_path`.

## Tests

- Success: real default-calls1 path → Ok
- Failure: temp truncated half of default-calls1 → Err / non-zero
- DECODE-FUZZ-MVP (no panic on corrupt battery): see
  [decode-fuzz-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/decode-fuzz-mvp-v0.md)
  (`decode_fuzz_no_panic_*` / `fuzz_truncated_mutations*` in format-v5 + report;
  empty / bad magic / half / stepped prefixes / single-byte XOR)

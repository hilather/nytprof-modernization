# Decode / verify fuzz MVP (v0)

**Board ID:** `DECODE-FUZZ-MVP`  
**Status:** done (MVP)  
**Depends on:** DECODE-ROBUST, COMPAT-010-ERR, VERIFY-CLI, INCOMPLETE-STREAM  
**Gate:** evidence **before COL-007**

## Goal

Exercise the **shipped** v5 decoder and verify path on corrupt / truncated /
single-byte-mutated inputs so that:

1. Call sites return **`Ok` or `Err` only** — **never panic**.
2. Fail-closed classes still **must `Err`** on `verify_profile` / CLI `verify`
   (empty, mid-file half of default-calls1, bad magic).
3. Tests **call** `nytprof_format_v5::decode_all` / `decode_path` and
   `nytprof_report::verify_profile` — they do **not** reimplement the decoder.

This is a **deterministic mutation battery**, not a full SEC-002 fuzz program
(cargo-fuzz / AFL / continuous corpus).

## Covered battery

| Case | Input | Decoder (`decode_all` / `decode_path`) | `verify_profile` |
|------|-------|----------------------------------------|------------------|
| (a) empty | `b""` | `Err` | `Err` |
| (b) bad magic | `NOTPROF 5 0\n` (and similar garbage) | `Err` | `Err` |
| (c) mid-file half | first `len/2` bytes of `fixtures/v5/default-calls1/nytprof.out` | `Err` | `Err` |
| (d) prefix steps | lengths `0, step, 2*step, …, n` (+ fixed cuts) of default-calls1 | `Ok` or `Err` only | `Ok` or `Err` only; empty/half **must** `Err` |
| XOR mutations | flip one byte (`^= 0xFF`) at ~32–64 offsets on a copy of the golden | `Ok` or `Err` only | `Ok` or `Err` only; magic-byte flip **must** `Err` |

Full golden default-calls1 remains **`Ok`** (sanity).

## Test names

Integration tests (shipped public APIs only):

- [`crates/nytprof-format-v5/tests/decode_fuzz.rs`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-format-v5/tests/decode_fuzz.rs)
- [`crates/nytprof-report/tests/decode_fuzz.rs`](https://github.com/hilather/nytprof-modernization/blob/main/crates/nytprof-report/tests/decode_fuzz.rs)

### `nytprof-format-v5`

| Test | Role |
|------|------|
| `decode_fuzz_no_panic_empty` | (a) empty → `Err` |
| `decode_fuzz_no_panic_bad_magic` | (b) bad magic → `Err` |
| `decode_fuzz_no_panic_mid_file_half` | (c) half → `Err` (`decode_all` + `decode_path`) |
| `fuzz_truncated_mutations` | (d) stepped prefixes → no panic; half/empty `Err`; full `Ok` |
| `decode_fuzz_no_panic_byte_xor_mutations` | XOR offsets → no panic; magic flip `Err` |

### `nytprof-report`

| Test | Role |
|------|------|
| `decode_fuzz_no_panic_verify_empty_magic_half` | (a)(b)(c) `verify_profile` → `Err` |
| `fuzz_truncated_mutations_verify` | (d) prefixes via `verify_profile`; empty/half `Err`; full `Ok` |
| `decode_fuzz_no_panic_verify_byte_xor_mutations` | XOR via `verify_profile`; magic flip `Err` |

Related (pre-existing, must stay green):

- format-v5: `decode_empty_input_errors`, `decode_bad_header_errors`, `decode_truncated_*`
- report: `fail_closed_*`, `verify_profile_truncated_default_calls1_err`
- CLI: `crates/nytprof-cli/tests/fail_closed.rs`
- Contracts: [COMPAT-010_ERROR_FAIL_CLOSED.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-010_ERROR_FAIL_CLOSED.md),
  [COMPAT-010_INCOMPLETE_STREAM.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md)
- Verify CLI schema: [verify-cli-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/verify-cli-mvp-v0.md)

## How to run

```bash
# Decoder fuzz battery
cargo test -p nytprof-format-v5 decode_fuzz_no_panic -- --nocapture
cargo test -p nytprof-format-v5 fuzz_truncated_mutations -- --nocapture

# Verify-path fuzz battery
cargo test -p nytprof-report decode_fuzz_no_panic -- --nocapture
cargo test -p nytprof-report fuzz_truncated_mutations_verify -- --nocapture

# Optional shell smoke (cargo or already-built binary)
bash tools/oracle/selftest_decode_fuzz.sh
```

## Explicit non-goals

| Topic | Status |
|-------|--------|
| Full SEC-002 continuous fuzz program | deferred |
| Identical error messages across dump/verify/report | not frozen |
| Opt-in salvage on truncated mid-record bytes | not allowed as silent success |
| Re-decoding logic inside tests | **forbidden** — call shipped APIs only |

## Panic policy

Prefer **Result-only** APIs. Tests do not require `catch_unwind` when the public
surface returns `Result`; if a panic occurs inside `decode_*` / `verify_profile`,
the Rust test harness fails the case (that is the intended gate).

# Security / fuzz hardening package — MVP schema (v0)

**Board ID:** `SEC-FUZZ-HARDENING-MVP`  
**Status:** done (MVP package)  
**Contract:** [`docs/contracts/SECURITY_FUZZ_HARDENING_PACKAGE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/SECURITY_FUZZ_HARDENING_PACKAGE_v0.md)  
**PR:** PR-C03  
**Depends on:** DECODE-FUZZ-MVP, COL-007 E3-EVENT C fixtures, COMPAT-010  
**Gate:** offline — `tools/oracle/selftest_security_fuzz.sh`; offline_gate step 12 when cargo present

## Goal

1. Keep **v5** decode/verify deterministic fuzz batteries green.  
2. Add **v6** deterministic mutation battery on **C-produced** EVENT sinks (`fixtures/v6/from-c/**`) so always-inflate consumers return **`Ok` or `Err` only** — **never panic**.  
3. Catalogue **batching** and **fork** threat mitigations with existing collector unit evidence.  
4. State **no open critical/high** for covered surfaces; residual honesty for continuous fuzz and COL-015.

## V6 decode-fuzz battery

Tests call shipped APIs only:

- `nytprof_format_v6::e3_decode_writer_bytes` (primary product sink path)
- `nytprof_format_v6::decode_decoded_event_profile` (always-inflate EVENT)

They **do not** reimplement the decoder or re-encode via `e3_standin_*`.

| Case | Input | Expectation |
|------|-------|-------------|
| (a) empty | `b""` | `Err` |
| (b) bad magic | `NOTPROF` / garbage | `Err` |
| (c) mid-file half | first `len/2` of `fixtures/v6/from-c/absolute.nytprof` | `Err` |
| (d) prefix steps | stepped prefixes of `absolute.nytprof` (+ packing / dict smoke) | `Ok` or `Err` only; empty/half **must** `Err` |
| XOR mutations | flip one byte (`^= 0xFF`) at ~24–48 offsets on absolute + packing fixtures | `Ok` or `Err` only; magic-byte flip **must** `Err` |
| Full golden | complete C fixtures used | **must** `Ok` (sanity; CRC verify `true`) |

Fixtures required present (committed):

- `fixtures/v6/from-c/absolute.nytprof`
- `fixtures/v6/from-c/packing.nytprof`
- `fixtures/v6/from-c/dict.nytprof` (expect_string_dict path)

## Test names

### `nytprof-format-v6` (`tests/decode_fuzz.rs`)

| Test | Role |
|------|------|
| `v6_decode_fuzz_no_panic_empty` | (a) |
| `v6_decode_fuzz_no_panic_bad_magic` | (b) |
| `v6_decode_fuzz_no_panic_mid_file_half` | (c) |
| `fuzz_truncated_mutations_v6` | (d) prefixes on absolute |
| `v6_decode_fuzz_no_panic_byte_xor_mutations` | XOR on absolute |
| `v6_decode_fuzz_no_panic_packing_and_dict_fixtures` | packing + dict full Ok; half Err; modest XOR no-panic |

### Pre-existing (must stay green)

| Package | Tests |
|---------|-------|
| `nytprof-format-v5` | `decode_fuzz_no_panic_*`, `fuzz_truncated_mutations` |
| `nytprof-report` | `decode_fuzz_no_panic_verify_*`, `fuzz_truncated_mutations_verify` |
| `collector` | `test_sv_lifetime` (batch UAF), `test_fork_split_seq_reset` (fork state) |

## Smoke harness

```bash
bash tools/oracle/selftest_security_fuzz.sh
```

Behavior:

1. Requires cargo for Rust batteries (fail if cargo missing — same honesty as decode-fuzz MVP smoke).  
2. Runs v5 format + report decode-fuzz filters.  
3. Runs v6 `v6_decode_fuzz` / `fuzz_truncated_mutations_v6` filters.  
4. When `cc`/`make` available: `make -C collector test` (batch + lifecycle threat evidence).  
5. Honest skip of collector when C toolchain absent (NOTE + continue).

## Explicit non-goals

| Topic | Status |
|-------|--------|
| cargo-fuzz / AFL continuous program | deferred (SEC-002 full; P02 is job MVP only) |
| Re-decoding logic inside tests | **forbidden** |
| Claiming COL-015 complete | residual |
| Claiming SEC-012 independent sign-off | residual (P02 checklist MVP only) |
| E3-mixed multi-kind fuzz corpus | residual |
| Panic via `catch_unwind` requirement | Result-only; panic fails the suite |

## Panic policy

Prefer **Result-only** APIs. Any panic inside `decode_*` / `e3_decode_writer_bytes` fails the Rust test harness (intended gate).

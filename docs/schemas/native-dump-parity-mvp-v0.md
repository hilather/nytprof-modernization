# Native dump parity MVP (v0)

**Status:** first-slice structural equality gate (multi-fixture)  
**Board IDs:** `NATIVE-DUMP-PARITY`, `DUMP-PARITY-EXPAND`  
**Not:** a second dump implementation; this document freezes **how** to prove parity of the **shipped** CLI dump against the oracle golden JSONL.

**Related:**

- Record shape / tags: [canonical-event-dump-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/canonical-event-dump-v0.md)
- Volatile normalization: [COMPAT-002_VOLATILE_NORMALIZATION.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-002_VOLATILE_NORMALIZATION.md)
- Numeric policy: [COMPAT-003_PRECISION_NUMERIC_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-003_PRECISION_NUMERIC_POLICY.md)

## Profiles under test

Full structural equality (dump×2 stability + normalize + `compare_jsonl.pl` + tag multiplicity) is gated on:

| Fixture | Profile | Golden dump | Timing shape |
|---------|---------|-------------|--------------|
| `default-calls1` | `fixtures/v5/default-calls1/nytprof.out` | `fixtures/v5/default-calls1/readstream.jsonl` | `TIME_LINE` present; `TIME_BLOCK == 0` |
| `calls2-default` | `fixtures/v5/calls2-default/nytprof.out` | `fixtures/v5/calls2-default/readstream.jsonl` | `TIME_LINE` present; `calls=2` (`SUB_ENTRY`); `TIME_BLOCK == 0` |
| `blocks-calls1` | `fixtures/v5/blocks-calls1/nytprof.out` | `fixtures/v5/blocks-calls1/readstream.jsonl` | **`TIME_BLOCK` present; `TIME_LINE == 0`** (statement timing as blocks) |

Workload for each is `fixtures/v5/<name>/workload.pl` (mid×3 → leaf×5 when returns are present).

**Important:** tag multiplicities (`TIME_LINE`, `TIME_BLOCK`, `SUB_RETURN`) are **loaded per fixture golden** — do **not** hard-code default-calls1 counts for blocks/calls2. On `blocks-calls1`, statement timing is `TIME_BLOCK`, not `TIME_LINE`.

## How to dump (native)

Use the **shipped** CLI path only — do not reimplement dump inside tests:

```sh
# Preferred (works without a prebuilt binary)
cargo run -q -p nytprof-cli -- dump fixtures/v5/default-calls1/nytprof.out \
  > /tmp/native.jsonl

# Or prefix / target binary if present
prefix/bin/nytprof-cli dump fixtures/v5/default-calls1/nytprof.out
# bare path is also dump (back-compat):
prefix/bin/nytprof-cli fixtures/v5/default-calls1/nytprof.out
```

Same dump path for `calls2-default` and `blocks-calls1` (swap the fixture directory).

Schema for the JSONL record shape: [canonical-event-dump-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/canonical-event-dump-v0.md).

## Normalize both sides

`compare_jsonl.pl` is a pure tag+args comparator and does **not** strip volatiles. Always normalize first (structural mode is the default):

```sh
python3 tools/oracle/normalize_jsonl.py fixtures/v5/default-calls1/readstream.jsonl \
  > /tmp/golden.norm.jsonl
python3 tools/oracle/normalize_jsonl.py /tmp/native.jsonl \
  > /tmp/native.norm.jsonl
```

## Compare

```sh
perl tools/oracle/compare_jsonl.pl /tmp/golden.norm.jsonl /tmp/native.norm.jsonl
# Expected: OK: N records match (tag+args)
```

## Result: full structural equality (multi-fixture)

On each covered fixture, after structural normalize of both the golden `readstream.jsonl` and a live native `dump` of `nytprof.out`, `compare_jsonl.pl` reports a **full match** (no residual tag/arg mismatches).

There is **no residual inequality list** for these fixtures under the current contract: every record’s `tag` + `args` matches after normalization (including trailing `_END` when present on both sides).

Multiplicity of critical timing / sub tags must match between golden and native (counts derived from the files for **that** fixture, not hard-coded alone):

| Tag | Role |
|-----|------|
| `TIME_LINE` | Statement timing events (default `blocks=0` fixtures; **0** on `blocks-calls1`) |
| `TIME_BLOCK` | Block timing events (`blocks=1` fixtures; **0** on default-calls1 / calls2-default) |
| `SUB_RETURN` | Subroutine return events |

Sanity (also per golden, not hard-coded numbers): `TIME_LINE + TIME_BLOCK > 0` and `SUB_RETURN > 0`.

## Stability (same binary, two dumps)

Dumping the same profile **twice** with the shipped CLI, normalizing both outputs, and comparing them must also match (determinism / stability of the dump path).

## Residual policy (future fixtures)

If a later fixture or format edge case fails full structural equality:

1. **Do not** weaken `compare_jsonl.pl` or silently drop tags.
2. Document an explicit residual list in this schema (or a fixture-local note) with:
   - tag name(s) and why they differ
   - whether the residual is volatile (should be fixed in normalize), numeric (COMPAT-003), or a true decode/encode gap
3. Keep full equality as the default gate for fixtures that already match (including the multi-fixture set above).
4. Additional fixtures (e.g. `default-calls2`) may be added to the smoke under the same residual policy.

## Oracle / PERL5LIB

This parity path uses only:

- native CLI dump (`cargo run` / prefix / target binary)
- `tools/oracle/normalize_jsonl.py`
- `tools/oracle/compare_jsonl.pl`

It does **not** require the oracle Perl env and must **never** put `crates/` on oracle `PERL5LIB`.

## Verification

| Gate | Command |
|------|---------|
| Schema (this file) | Read / review |
| Operator smoke (default-calls1) | `./tools/oracle/selftest_native_dump_parity.sh` |
| Operator smoke (named fixture) | `./tools/oracle/selftest_native_dump_parity.sh calls2-default` |
| Operator smoke (blocks) | `./tools/oracle/selftest_native_dump_parity.sh blocks-calls1` |
| Operator smoke (all three) | `./tools/oracle/selftest_native_dump_parity_all.sh` |
| Harness (includes all-fixture smoke) | `./tools/oracle/selftest_harness.sh` |
| Optional Rust tag multiplicity | `cargo test -p nytprof-format-v5 native_dump_tag_counts_match_golden` |

Evidence paths land on the first-slice board as `NATIVE-DUMP-PARITY` and `DUMP-PARITY-EXPAND`.

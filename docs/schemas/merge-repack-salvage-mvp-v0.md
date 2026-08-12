# Merge / repack / salvage CLI MVP (v0)

**Board ID:** `TOOL-MERGE-REPACK-SALVAGE-MVP`  
**Status:** implemented (PR-C02)  
**Plan tasks:** TOOL-007 (salvage), TOOL-009 (merge scoped), RUST-013 (stream-concat MVP)  
**Depends on:** strict convert encoders (PR-C01), product dual decode, absolute v6 EVENT encode  
**Not:** full `nytprofmerge` aggregate-sum parity, SEC-003 multi-chunk mid-corruption matrix, packing/string-dict v6 output, lossy convert

## Goal

Operators get **unambiguous recovery semantics** for v5/v6 profiles:

| Command | When to use | Completeness claim |
|---------|-------------|--------------------|
| **merge** | Combine multiple fully-decoded profiles | Output is a clean encode of concatenated remapped streams |
| **repack** | Normalize / re-encode a good profile | Full decode required; clean absolute v6 or v5 |
| **salvage** | Recover from truncated/corrupt input | **Always** labeled salvage/incomplete; longest complete verified prefix only |

## CLI

```text
nytprof-cli merge --to=v5|v6 -o <output> <input> [<input>...]
nytprof-cli repack [--to=v5|v6] <input> -o <output>
nytprof-cli salvage [--to=v5|v6] <input> -o <output>
```

| Flag | merge | repack | salvage |
|------|-------|--------|---------|
| `--to=v5\|v6` | **required** | optional (default: input family) | optional (default: input family) |
| `-o` / `--output` | **required** | **required** | **required** |
| inputs | ≥1 path | 1 path | 1 path |

### Success lines (greppable)

```text
OK: merge --to=v6 inputs=2 -> /tmp/out.v6
OK: repack --to=v6 in.nytprof -> /tmp/out.v6
OK: salvage incomplete=yes wire=v5 events=3 bytes=45/68 discarded_tail=23 -> /tmp/out.v5
SALVAGE: stream_incomplete=missing_pid_end,no_statement_timing   # when applicable
```

Exit **≠ 0** on decode/encode failure (merge/repack) or zero recoverable events (salvage). No `OK:` line on failure.

## Recovery semantics (normative for this MVP)

### merge

1. **Every** input must **fully** decode (v5 or v6 dual dispatch). Corrupt members → fail closed; no silent skip.
2. Streams are concatenated in **CLI input order** (deterministic).
3. `VERSION` / `START_DEFLATE` from streams after the first are dropped (target encoder supplies one).
4. File IDs from later streams are **offset** so they cannot collide with earlier `NEW_FID` values; `eval_fid` remapped when non-zero; fid-bearing tags (`TIME_LINE`, `TIME_BLOCK`, `SUB_ENTRY`, `SUB_INFO`, `SUB_CALLERS`, `SRC_LINE`) follow.
5. Process (`PID_*`) boundaries are preserved as independent sequences.
6. `seq` renumbered 0..n-1.
7. Encode via strict convert path (`encode_events` → absolute v6 EVENT or v5).

**Not** legacy `nytprofmerge` same-run line-total sum (residual).

### repack

1. Input must **fully** decode (same as convert).
2. Re-encode to `--to` (default: same wire family as input).
3. Truncated / mid-record / mid-chunk inputs → **error** (use `salvage`).

### salvage

1. Recover the **longest complete verified event prefix**:
   - **v5:** progressive tag salvage (`nytprof_format_v5::decode_salvage_prefix`). Incomplete `START_DEFLATE` zlib units are discarded entirely; pre-`z` complete tags kept. `bytes_consumed` stops at the incomplete unit.
   - **v6:** product always-inflate decode; trailing garbage after a complete profile is discarded (`bytes_consumed`); hard failure → longest successful prefix search.
2. Mid-record / mid-chunk tails are **never** emitted as events.
3. Output is **always** labeled:
   - `ATTRIBUTE nytprof.salvage=1`
   - `ATTRIBUTE nytprof.salvage.incomplete=1`
   - `ATTRIBUTE nytprof.salvage.wire` / `bytes_consumed` / `input_len` / `discarded_tail` / `stream_incomplete`
   - COMMENT banner
4. Salvage **never** pretends the result is an unlabeled clean complete profile.
5. Zero recoverable events → error.

## Library API

```text
nytprof_model::{
  merge_bytes, merge_paths,
  repack_bytes, repack_path,
  salvage_bytes, salvage_path, SalvageReport,
  detect_convert_target,
  MergeToolsError,
}
```

## Capability claim

When tools are green, `nytprof-cli capability` reports:

```text
merge: yes
repack: yes
salvage: yes
```

JSON: `"merge": true, "repack": true, "salvage": true`.

Live probe (when dual-sink m4 fixtures present): repack v5→v6 + verify; merge m4_v5+m4_v6→v6 + verify; salvage mid-zlib cut → labeled output. Fail closed if present but broken.

## Evidence

| Check | Command |
|-------|---------|
| Library | `cargo test -p nytprof-model merge_tools::` |
| v5 salvage primitive | `cargo test -p nytprof-format-v5 --lib decode_salvage` |
| CLI | `cargo test -p nytprof-cli --test merge_repack_salvage_cli` |
| Capability | `cargo test -p nytprof-cli --test capability_selftest` |

## Residuals

| Residual | Notes |
|----------|--------|
| Full `nytprofmerge` aggregate-sum / option parity | Not this MVP (stream-concat + fid remap only) |
| SEC-003 multi-chunk mid-corruption resume matrix | Progressive v5 + v6 trailing strip; full fuzz corpus later (PR-C03) |
| Packing / string-dict v6 output | Absolute EVENT / v5 only (via convert encoders) |
| Lossy convert | Not shipped |
| Automatic salvage as default verify/report | **Forbidden** (COMPAT-010); salvage is explicit CLI |
| Oracle wall-NV PID refuse on v5→v6 encode paths | Same as convert residual when targeting v6 from fractional wall times |

## Related

- [`docs/schemas/convert-strict-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/convert-strict-mvp-v0.md)
- [`docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md)
- [`docs/plan/09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md)

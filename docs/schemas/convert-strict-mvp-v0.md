# Strict v5↔v6 convert CLI MVP (v0)

**Board ID:** `TOOL-CONVERT-STRICT-MVP`  
**Status:** implemented (PR-C01)  
**Plan tasks:** TOOL-004, TOOL-005, FMT-013 (strict path)  
**Depends on:** product dual decode (`ProfileModel` / PR-B11a), wire freeze (PR-B11), v5 encoder (`nytprof-format-v5`), absolute v6 EVENT encode  
**Not:** default lossy; packing/string-dict v6 output; full oracle wall-NV PID projection on the **strict** path. Opt-in `--allow-lossy`: [`convert-lossy-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/convert-lossy-mvp-v0.md) (**L01**).

## Goal

Operators and packaging gates convert product profiles between **v5** (`NYTProf 5 0`) and **v6** (`NYTPROF6` absolute EVENT) under a **strict** path: unrepresentable values fail closed with diagnostics. Successful v5 outputs are readable by the independent v5 decoder (old-tool / 6.15 shape).

## CLI

```text
nytprof-cli convert --to=v5|v6 <input> -o <output>
nytprof-cli convert --to v5|v6 <input> --output <output>
```

| Flag | Required | Meaning |
|------|----------|---------|
| `--to=v5` / `--to=v6` | yes | Target wire family |
| `-o` / `--output` | yes | Output path (parents created) |
| `<input>` | yes | Source profile path |

Exit **0** and print a greppable line:

```text
OK: convert --to=v6 <input> -> <output>
```

Exit **≠ 0** on decode/encode/strict representability failure (stderr diagnostics). No `OK:` line on failure.

## Pipeline

1. Dual-dispatch decode (`decode_events_from_bytes`) — v5 text header or `NYTPROF6`
2. Strict projection for target ranges / fields
3. Encode:
   - **v5:** `nytprof_format_v5::encode_all_as_v5` (header always `NYTProf 5 0`; auto `START_DEFLATE` when binary tags appear without one)
   - **v6:** absolute EVENT / codec NONE via shipped encode helper (`e3_standin_write_absolute` surface; product always-inflate decode)

Library API: `nytprof_model::{convert_bytes, convert_path, encode_events, ConvertTarget}`.

## Strict rules (fail closed)

| Check | Target | Behavior |
|-------|--------|----------|
| I32 ticks | v5 | `TIME_LINE` / `TIME_BLOCK` ticks outside `i32` → error |
| U32 fields | v5 | fields above `u32::MAX` → error |
| Finite + exact NV | v5 | non-finite → error; integer values must be **exactly** representable as f64 (mantissa; e.g. `2^53+1` refuses — no silent round) |
| Exact integer ticks/times | v6 | fractional NV (e.g. oracle wall-clock `PID_START`) → error (no silent truncate) |
| Unknown tags | both | error |
| Non-zero extended `NEW_FID` | v6 | `eval_fid` / `eval_line` / `flags` / `size` / `mtime` ≠ 0 → **error** (no silent zero) |
| Non-zero `TIME_BLOCK.sub_line` | v6 | **error** (absolute body has no field; no silent zeroing). Zero `sub_line` is representable. |
| VERSION major | v5 projection | only majors **5** or **6** accepted; other majors refuse |

**`--allow-lossy` is opt-in only** (L01). Strict remains the default. Lossy projections are documented in [`convert-lossy-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/convert-lossy-mvp-v0.md).

## Old-tool acceptance (v5 outputs)

Successful `--to=v5` outputs:

- Start with `NYTProf 5 0\n`
- Decode with `nytprof_format_v5::decode_all` / CLI `dump` / `verify`
- Shape matches oracle 6.15 FileHandle protocol (packed ints, LE NV, optional zlib after `z`)

## Capability claim

When convert is green, `nytprof-cli capability` reports:

```text
convert: yes
```

JSON:

```json
"convert": true
```

Capability **exercises** dual-sink `m4_v5` → v6 → verify and `m4_v6` → v5 → verify when fixtures are present (fail closed if present but broken).

## Evidence

| Check | Command / path |
|-------|----------------|
| Library | `cargo test -p nytprof-model convert::` |
| v5 encoder | `cargo test -p nytprof-format-v5 --lib writer::` |
| CLI | `cargo test -p nytprof-cli --test convert_cli` |
| Capability | `cargo test -p nytprof-cli --test capability_selftest` |
| Dual-sink fixtures | `fixtures/e4/dual-sink/*` |

## Residuals

| Residual | Notes |
|----------|--------|
| Lossy / `--allow-lossy` | Not shipped; non-zero `sub_line` / extended `NEW_FID` / fractional wall PID / non-mantissa-exact NV all **refuse** (no silent path) |
| Dual-sink `blocks_calls1` v5→v6 | Refuses until lossy mode or v6 body carries `sub_line` (v6→v5 still green) |
| Packing / string-dict / multi-kind v6 output | Absolute EVENT only |
| Merge / repack / salvage | **done** in PR-C02 — see [`merge-repack-salvage-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/merge-repack-salvage-mvp-v0.md) |
| Full oracle dual convert matrix | Integer-tick dual-sink + v5 identity; oracle v5→v6 residual on wall NV |

## Related

- [`docs/plan/09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/09_CLI_CONVERTERS_MERGE_AND_TOOLING_TASKS.md)
- [`docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/E4_V5_V6_SEMANTIC_EQUALITY_POLICY_v0.md)
- [`docs/schemas/capability-selftest-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/capability-selftest-mvp-v0.md)

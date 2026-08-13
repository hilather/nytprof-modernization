# Optional `--allow-lossy` convert MVP (v0)

**Board ID:** `TOOL-CONVERT-LOSSY-MVP` / **L01**  
**Status:** **done (MVP)** — opt-in only; **strict remains default**  
**Depends on:** [`convert-strict-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/convert-strict-mvp-v0.md)  
**Not:** default lossy; packing/string-dict v6 convert; full TEST-008; product `format=dual`

## CLI

```text
nytprof-cli convert --to=v6 --allow-lossy <input> -o <output>
```

Without `--allow-lossy`, convert stays on the **strict** path (oracle `fixtures/v5/default-calls1/nytprof.out` still refuses fractional `PID_START.start_time`; no `OK: convert`).

With `--allow-lossy`, the shipped convert pipeline (`nytprof_model::convert_path_with`) writes target-family bytes. `--to=v6` starts with `NYTPROF6`. Stderr prints a greppable `NOTE: --allow-lossy`.

## Projection limits (explicit)

| Input | Lossy projection |
|-------|------------------|
| Fractional NV ticks/times | Truncate toward 0 to `u64` |
| Non-zero `NEW_FID` eval/flags/size/mtime | Drop (absolute v6 body is fid+name) |
| Non-zero `TIME_BLOCK.sub_line` | Drop (absolute body has no field) |
| Negative / non-finite / unknown tags | Still **refuse** |

## Evidence

- `cargo test -p nytprof-model oracle_default_calls1`
- `cargo test -p nytprof-cli --test convert_cli`
- `./scripts/packaging/l01_lossy_convert_smoke.sh`

E4/oracle equality oracles must **not** pass `--allow-lossy`.

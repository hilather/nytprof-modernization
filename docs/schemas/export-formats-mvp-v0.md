# Export formats MVP (v0) — Callgrind + folded stacks

**Status:** first-slice machine-oriented exports  
**Not:** byte-identical legacy `nytprofcg` / `nytprofcalls` dialects

**Semantic gate (counts on default-calls1):**  
[export-semantic-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/export-semantic-parity-mvp-v0.md)  
(`EXPORT-SEMANTIC-PARITY` — real model leaf **15** / mid **3** / mid→leaf **15**; folded + callgrind evidence).

## CLI

```text
nytprof-cli callgrind <profile.out>     # Callgrind-style text to stdout
nytprof-cli folded <profile.out>        # folded stack lines to stdout
# optional aliases:
nytprof-cli cg <profile.out>
```

## Callgrind-style (contracted content)

Minimum viable text suitable for tools that expect Callgrind-ish structure:

```text
# callgrind format
positions: line
events: Ticks
...
fn=main::leaf
...
cfn=main::leaf
calls=15 ...
```

**Required:**
- Mentions of `main::leaf` and `main::mid`
- Contracted call count **15** for mid→leaf relationship (from `call_edges` or sub returns)
- Non-empty file

Exact layout may be simplified; tests check string presence of workload names and counts, not full Valgrind tool acceptance.

## Folded stacks (contracted content)

One or more lines of the form:

```text
main::mid;main::leaf 15
```

and/or

```text
main::RUNTIME;main::mid 3
```

Built from `call_edges` (caller;called count). Sorted deterministically.

## Library API (suggested)

```rust
nytprof_report::render_callgrind(model: &ProfileModel) -> String
nytprof_report::render_folded_stacks(model: &ProfileModel) -> String
```

# Failed / abandoned implementation attempts (light ledger)

**Status:** living light index — expand under [`details/`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/details/) when a row needs more than one line  
**Duty:** [`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/main/AGENTS.md) §6 — save failed attempts **automatically**  
**Not:** ADRs, certification, or board “done” claims

Add a **new row near the top** when an implementation, optimization (including **perf** tries that did not win), or approach is abandoned or fails gates after real effort. Keep cells short; link a detail file only when the light row is not enough.

| Date | Slug | Area | Tried | Why failed / residual | Detail |
|------|------|------|-------|----------------------|--------|
| 2026-08-12 | stack-merge-drop-html-a02-a03 | ci / html | Linear stack reassembly of PR-A01/A02/A03 | Kept CLI tests + schemas; dropped `html --flame`, `index-subs-excl.html`, and stderr `style.css` listing from product code. rust-smoke red after format-v6 fix. Re-port A02/A03 into report+cli; never trust --theirs stack without full package tests. | — |
| 2026-08-12 | cargo-fix-test-only-imports | ci / format-v6 | `cargo fix` / import cleanup on format-v6 | Dropped imports used only in `#[cfg(test)]` modules; `cargo check` green but `cargo test -p nytprof-format-v6 --lib` (and GHA rust-smoke) red E0425. Do not retry bare `cargo fix` without re-running lib tests; put test-only imports inside the test module. | — |
| 2026-08-12 | capability-json-stack-merge | ci / cli | Linear stack merge of parallel execute-plan PRs dropped `collection_default` / `v6_decode` from capability JSON | CI red on cli_e5_v6: assert collection_default==v5 got Null; restore keys + smoke/clippy gate | — |

## How to append (agents)

1. Prefer one row per distinct attempt (not per micro-edit).  
2. **Area** examples: `perf`, `v6-format`, `v5-decode`, `report`, `html`, `perl-jsonl`, `packaging`, `ci`, `ffi`.  
3. **Why failed** should say *what evidence closed it* (red gate, worse wall time, wrong semantics, unmaintainable, contradicted fixture).  
4. If someone might naively retry the same idea, put a “do not retry without …” clause in **Why** or in the detail file.  
5. Successful pivots still keep the failed row — the win lives in code/docs; the failure stays here so it is not re-proposed blindly.  
6. Keep cells **short** (context budget). Put benches, stack traces, or multi-step postmortems under `details/<slug>.md` and link from **Detail**.

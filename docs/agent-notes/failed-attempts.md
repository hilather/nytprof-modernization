# Failed / abandoned implementation attempts (light ledger)

**Status:** living light index — expand under [`details/`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/details/) when a row needs more than one line  
**Duty:** [`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/main/AGENTS.md) §6 — save failed attempts **automatically**  
**Not:** ADRs, certification, or board “done” claims

Add a **new row near the top** when an implementation, optimization (including **perf** tries that did not win), or approach is abandoned or fails gates after real effort. Keep cells short; link a detail file only when the light row is not enough.

| Date | Slug | Area | Tried | Why failed / residual | Detail |
|------|------|------|-------|----------------------|--------|
| 2026-08-15 | ci-rocky8-lab-before-tag-rpm | ci / field | Required CI `rocky8-docker-lab` `--engine both` on GHA | (1) Same-tag RPM 404 before attach. (2) After RPM exists, oracle half `fail`s on missing gitignored `File::Which`. `--engine both` must SKIP oracle when Which is absent; native half still required. | — |
| 2026-08-15 | flame-per-edge-equal-width-columns | html / flame | Paint one 2-level icicle column per `call_edges` pair; width = call count | Scanner report: every hot edge count=7576 → barcode of identical ~239px columns; `scan_file`/`RUNTIME` repeated as siblings. Index `<object type="image/svg+xml">` blank under `file://`. Do not retry per-edge columns; use inclusive-time call tree + `<img src>`. | — |
| 2026-08-14 | pr7-goto-list-missed-constant | collector / attach | PR-7 `goto &$raw` only for Exporter / Getopt / `vars` | Host Getopt::Long 2.54 then failed `use constant CTL_*` (`caller` is `DB` under `&$raw` wrap). Need compile-time goto-all (`$product_after_init`) plus `constant::` / `overload::` on the runtime goto list. | — |
| 2026-08-14 | rocky8-profile-ack-d-nytprofm | field / attach | `perl -d:NYTProfM` on downloaded ack v3 (Getopt::Long) and a Time::HiRes scanner in rockylinux:8 | Both exit 255 at compile: Getopt::Long `$VERSION` strict; Exporter::Heavy `goto` → `heavy_(eval)`. Product `DB::sub` did `&$raw` (not `goto &$raw`) and `$DB::single=1` from `file=`. **PR-7 landed** (`INIT` `$DB::single` + `goto` for Exporter/Getopt/`vars`; `g07_getopt_compile_smoke.sh`). Rocky demo still uses the core-only scanner until ack is retried as a field change. | — |
| 2026-08-13 | di01-dbdb-visit-contexts | collector / attach | `DB::DB` + `visit_contexts` for `blocks=1` 780/810 | From `DB::DB`, `PL_curcop` is the hook; `block_line==line`. For-modifier `$x++ for 1..50` is **one** `dbstate` + `preinc`+`unstack` (not 52 `DB::DB`). `$^P` 0x04 / NEXTSTATE-only still 15. Landed: targeted DBSTATE/NEXTSTATE/UNSTACK `nytp_emit_time_*` slice (not DI-03). Do not retry hook-only 780 without new optree evidence. | [`fixtures/v5/product-attach/di01-spike/`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v5/product-attach/di01-spike/) |
| 2026-08-12 | macos-collector-codec-link | ci / collector | collector_sink_smoke on macos-arm64 | Bare `-lz -lzstd -llz4` failed after HAS_ZLIB fix: brew codec libs not on default link path. Fix: smoke + CI export brew `-I/-L` for zlib/zstd/lz4 into make. | — |
| 2026-08-12 | macos-oracle-no-has-zlib | ci / oracle | Host-local oracle rebuild on GHA macos-arm64 | Makefile.PL never saw zlib.h (not under /usr/include); built without HAS_ZLIB → dual_path “compression is not supported”. Fix: export SDK/brew INCLUDE in build_oracle + CI; fail closed if deflateInit2 not found. | — |
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

# Language semantics gotchas (Perl & Rust) — light ledger

**Status:** living light index — expand under [`details/`](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/details/) when a row needs more than one line  
**Duty:** [`AGENTS.md`](https://github.com/hilather/nytprof-modernization/blob/main/AGENTS.md) §6 — save corrected **Perl** and **Rust** misunderstandings **automatically**  
**Not:** a language tutorial; not a substitute for oracle fixtures or COMPAT contracts

Record **language- or runtime-specific rules** that an agent got wrong (or nearly wrong) while working this repo — especially NYTProf / XS / dual-engine edges. Prefer pointers into contracts, fixtures, or oracle code over restating whole manuals. Keep rows **light**; open a detail file only when the correction needs more than one line.

| Date | Lang | Topic | Wrong assumption | Correct rule / pointer | Open? | Detail |
|------|------|-------|------------------|------------------------|-------|--------|
| 2026-08-07 | both | oracle `PERL5LIB` | Putting `crates/` or native tree on oracle `PERL5LIB` is fine for “convenience” | **Never** put `crates/` on oracle `PERL5LIB` — isolates 6.15 oracle; see `AGENTS.md` + residual matrix isolation rule | no | — |
| 2026-08-07 | rust | `total_events` on JSON | ProfileModel event counter equals advertised `total_events` (e.g. 2473) | Advertised default-calls1 **`total_events` is 2474** (`model.total_events + 1` / stream account as frozen by JSON-TOTAL-EVENTS-MVP + residual matrix) | no | — |
| 2026-08-07 | rust | v6 chunk payload | Default `parse_chunk_frame` should inflate ZLIB and verify CRC | Default parse is **non-inflating / non-CRC-verify**; inflate and CRC are **explicit** helpers (`FMT-V6-PAYLOAD-ZLIB/ZSTD/LZ4-*`, `FMT-V6-CRC-*`) | no | — |
| 2026-08-07 | rust | v6 LZ4 wire | LZ4 frame with embedded size is the MVP wire | MVP is **raw LZ4 block**; size is chunk `uncompressed_len` only (`FMT-V6-PAYLOAD-LZ4-*`) | no | — |
| 2026-08-07 | perl | SUB_ENTRY multiplicity | Every profile has SUB_ENTRY events | `calls=1` default-calls1 → **0**; `calls=2` calls2-default → **27** (multiplicity only; not full call-stack freeze) | no | — |
| 2026-08-11 | perl | MakeMaker `PREFIX` env | Exporting `PREFIX=…` before `make` lets install scripts honor that path | MakeMaker **redefines** `PREFIX` and re-exports it into recipe envs (often `~/perl5` via `PERL_MM_OPT`/`INSTALL_BASE`). Prefer **`NYTPROF_PREFIX`**; bare `PREFIX` denylist is shared (`resolve_packaging_prefix.sh`: strip trailing `/`, reject `$HOME/perl5` and `*/perl5`) so dual-install cannot split roots | no | — |

## Scope examples (what belongs here)

| Belongs | Does not belong |
|---------|-----------------|
| Perl `calls=1` vs `calls=2` SUB_ENTRY multiplicity | Generic “remember to use strict” |
| Rust fail-closed oversize length before alloc | Style-only clippy nits |
| Oracle `PERL5LIB` isolation (never `crates/`) | Unrelated crate ecosystem trivia |
| Tick/display vs exact integer counts (COMPAT-003) | Restating the whole COMPAT-001 field list |

## How to append (agents)

1. One row per distinct misconception (merge duplicates by editing the existing row).  
2. **Open?** = `yes` if still unresolved / needs ADR; `no` if corrected and settled.  
3. Prefer absolute links to contracts, schemas, or oracle paths.  
4. If the light row cannot carry the nuance (e.g. dual-equality timing), add `details/<slug>.md` and link it.

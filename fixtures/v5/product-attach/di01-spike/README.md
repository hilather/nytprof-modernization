# DI-01 / PR-B1 spike — TIME_BLOCK multiplicities (one `leaf()`)

**Not** an oracle golden. **Not** a 780 redefinition. Regenerated on the implementer host while landing PR-B1.

## Host

| Field | Value |
|-------|--------|
| Perl | 5.38.2 `x86_64-linux-gnu-thread-multi` (ithreads) |
| EL8 5.26 | not available on this host; same for-modifier optree (`preinc` + `unstack`) is expected |
| Command | `NYTPROF=file=…:blocks=1 perl -Icollector/build/xs-nytprof -d:NYTProfM one_leaf.pl` |
| Counts | [`one-leaf-time-block-counts.txt`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v5/product-attach/di01-spike/one-leaf-time-block-counts.txt) |

`one_leaf.pl` is the leaf body from `fixtures/v5/blocks-calls1/workload.pl` (lines 4–6: `my $x = 0;` / `$x++ for 1 .. 50;` / `return $x`).

## Raw `TIME_BLOCK (fid,line,block,sub)` after the landed slice

Workload fid (basename `one_leaf.pl`, first-seen):

| line | block | sub | count | Note |
|------|-------|-----|-------|------|
| 4 | 4 | 4 | 1 | `my $x = 0` |
| 5 | 4 | 4 | **52** | 1 `dbstate` + 50 `unstack` + 1 post-unstack replay |
| 6 | 4 | 4 | 1 | `return` |

`visit_contexts` on the **opcode** COP yields `block_line=4` / `sub_line=4` for the leaf body.

## Approaches that did not hit 780/810

| Try | line5 / block4 (full 15×leaf workload) | Why |
|-----|----------------------------------------|-----|
| `DB::DB` + `visit_contexts` only | 15 / 15 (`block_line==line`) | `PL_curcop` is the hook; for-modifier is not 52 `DB::DB` visits |
| `DBSTATE`/`NEXTSTATE` + `$^P` 0x04 | 15 / 45 (`block=4` on 4/5/6 only) | Modifier compiles to `preinc`+`unstack`; **no** per-iter `nextstate` |
| `UNSTACK` emit `last_stmt` COP | 765 / 822 | Mid’s `unstack` re-attributed leaf line 6 |
| Landed: `UNSTACK` emit `PL_curcop` + one replay when last stmt is still the loop line | **780 / 810** | 15×52 line5 + 15 line4 + 15 line6 |

This slice only calls `nytp_emit_time_*`. It is **not** DI-03 (no `entersub` / full leave / DISCOUNT / full `slowops.h`).

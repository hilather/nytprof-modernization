# Collector overlay tree (stub)

Preferred layout for modernization collector sources (overlay vs oracle pin under `baseline/6.15/`). Packaging ADR (PR-B00) may refine this tree; **do not** put `crates/` on oracle `PERL5LIB`.

## Provisional v6 ID lockfile (C)

| Path | Role |
|------|------|
| [`include/nytprof_v6_ids.h`](https://github.com/hilather/nytprof-modernization/blob/main/collector/include/nytprof_v6_ids.h) | Mirrored provisional MAGIC / kind / codec / opcode / flag constants for COL-007 |

Normative note: [`docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md).

**Not** a wire freeze; **not** COL-007 product complete.

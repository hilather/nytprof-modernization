# Product attach mini kinds (DI-04 / PR-B3)

**Status:** shipped comparator + dual-collect smoke  
**Board:** `DROP-IN-REMAINING` (DI-04 kinds MVP)  
**Not:** full TEST-003 `compare_jsonl` tag+args (DI-05 / milestone E); not the DI-01 **780** or DI-02 **27** bars.

## Comparator

[`tools/oracle/compare_event_kinds.py`](https://github.com/hilather/nytprof-modernization/blob/main/tools/oracle/compare_event_kinds.py) projects each dump JSONL onto `MUST_KIND_SET`, then applies presence/absent rules. It does **not** invoke [`compare_jsonl.pl`](https://github.com/hilather/nytprof-modernization/blob/main/tools/oracle/compare_jsonl.pl).

```text
MUST_KIND_SET (default mini, blocks=0):
  NEW_FID, TIME_LINE, SUB_RETURN, SUB_CALLERS, SUB_ENTRY

DROP_SET (ignored; anything not in MUST_KIND_SET is dropped):
  DISCOUNT, SRC_LINE, SUB_INFO, ATTRIBUTE, OPTION, START_DEFLATE,
  PID_START, PID_END, COMMENT, TIME_BLOCK, VERSION
```

| Projected tag | `calls=1` | `calls=2` |
|---------------|-----------|-----------|
| `NEW_FID` | present (`≥1`) | present |
| `TIME_LINE` | present | present |
| `TIME_BLOCK` | absent | absent |
| `SUB_RETURN` | present | present |
| `SUB_CALLERS` | present | present |
| `SUB_ENTRY` | absent | present (`≥1`, **not** 27) |

## Dual collect

Same [`fixtures/v5/product-attach/m4-mini/workload.pl`](https://github.com/hilather/nytprof-modernization/blob/main/fixtures/v5/product-attach/m4-mini/workload.pl):

- Product: `perl -d:NYTProfM` from `collector/build/xs-nytprof`
- Oracle: `perl -d:NYTProf` via [`tools/oracle/env.sh`](https://github.com/hilather/nytprof-modernization/blob/main/tools/oracle/env.sh) — never `crates/` on that `PERL5LIB`

Goldens store the **projected** presence vector only. Regen: dual dump + project + review.

Smoke: [`scripts/packaging/di04_mini_kinds_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/di04_mini_kinds_smoke.sh).

Design: [`docs/DROP_IN_RPM_COMPLETION_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/DROP_IN_RPM_COMPLETION_v0.md) KD-32.

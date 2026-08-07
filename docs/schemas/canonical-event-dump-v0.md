# Canonical event dump schema (v0) — shared freeze

**Status:** provisional shared schema for differential testing  
**Not:** a stable v6 wire format  
**Consumers:** oracle `dump_readstream.pl`, Rust `nytprof-dump`, `compare_jsonl.pl` / normalize tools  
**Logical events:** tag → named event mapping in [`docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md`](../contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md) and [`docs/contracts/logical-events.schema.json`](../contracts/logical-events.schema.json)

## Record shape (JSONL)

One JSON object per line:

```json
{"seq": 0, "tag": "VERSION", "args": [5, 0]}
```

| Field | Type | Notes |
|-------|------|-------|
| `seq` | non-negative integer | Monotonic per dump; comparator may ignore |
| `tag` | string | ReadStream tag name (see inventory) |
| `args` | JSON array | Tag-specific; see below |

Trailing synthetic record allowed:

```json
{"seq": N, "tag": "_END", "args": []}
```

## Tag argument shapes (ReadStream / loader callback order)

Aligned with `NYTProf.xs` `callback_info` and `load_profile_data_from_stream`:

| Tag | Args | Types |
|-----|------|-------|
| `VERSION` | major, minor | u, u |
| `COMMENT` | text | string (may include trailing `\n` as oracle emits) |
| `ATTRIBUTE` | key, value | string, string |
| `OPTION` | key, value | string, string |
| `START_DEFLATE` | _(empty)_ | |
| `PID_START` | pid, ppid, start_time | u, u, n (NV as JSON number) |
| `PID_END` | pid, end_time | u, n |
| `NEW_FID` | fid, eval_fid, eval_line, flags, size, mtime, name | u×6, string |
| `TIME_LINE` | ticks, fid, line | i (as non-neg JSON number if ≥0), u, u |
| `TIME_BLOCK` | ticks, fid, line, block_line, sub_line | i, u×4 |
| `DISCOUNT` | _(empty)_ | |
| `SUB_ENTRY` | caller_fid, caller_line | u, u |
| `SUB_RETURN` | depth, incl_time, excl_time, subname | u, n, n, string |
| `SUB_INFO` | fid, first_line, last_line, name | u, u, u, string |
| `SUB_CALLERS` | fid, line, count, incl, excl, reci, rec_depth, called, caller | u,u,u,n,n,n,u,s,s |
| `SRC_LINE` | fid, line, text | u, u, string |

## Numeric JSON conventions

- Integers that fit exact JSON numbers without fraction are emitted as JSON integers when the oracle does so.
- Floating NVs may appear as JSON numbers; **normalization** re-encodes via `normalize_number` / `%.17g` for compare (see `tools/oracle/normalize_jsonl.py` and [`docs/contracts/COMPAT-003_PRECISION_NUMERIC_POLICY.md`](../contracts/COMPAT-003_PRECISION_NUMERIC_POLICY.md)).
- Signed ticks use two’s-complement bit pattern from v5 varint; values ≥0 display as unsigned magnitude in practice. Statement/block ticks remain integer counts (COMPAT-003).

## Volatile fields (normalize before golden compare)

Normative provisional rules: [`docs/contracts/COMPAT-002_VOLATILE_NORMALIZATION.md`](../contracts/COMPAT-002_VOLATILE_NORMALIZATION.md).

| Location | Field | Normalization |
|----------|-------|----------------|
| `COMMENT` | any | replace with `<COMMENT>` or drop for structural compare mode |
| `ATTRIBUTE` `basetime` | value | `<BASETIME>` |
| `ATTRIBUTE` `application` | value | basename only or `<APP>` |
| `NEW_FID` name | path | basename only |
| `SRC_LINE` text | — | keep exact (not volatile) |
| Absolute paths in strings | — | map to basename for path-like args |

Exact rules implemented in `tools/oracle/normalize_jsonl.py`.

## Unsupported tags

Decoders must **not** silently drop unknown tag bytes after the binary stream starts. Emit a hard error: `unsupported tag 0xNN at offset O` (or equivalent). Text-phase unknown lines may error similarly.

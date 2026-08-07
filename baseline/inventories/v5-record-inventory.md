# BASE-002 — Provisional v5 record inventory (Phase-0 freeze)

**Status:** provisional freeze (time-boxed; not complete for every option matrix)  
**Oracle:** Devel::NYTProf 6.15 (`baseline/6.15/`, tag `v6.15`)  
**Primary sources:** `FileHandle.h`, `FileHandle.xs`, `NYTProf.xs`, `lib/Devel/NYTProf/ReadStream.pm`  
**Date:** 2026-08-07

## Format identity

| Field | Value | Source |
|-------|-------|--------|
| Major | 5 | `NYTP_FILE_MAJOR_VERSION` in `NYTProf.xs` |
| Minor | 0 | `NYTP_FILE_MINOR_VERSION` in `NYTProf.xs` |
| Compression | optional zlib via `NYTP_TAG_START_DEFLATE` / `z` | `FileHandle.xs` |

## Tag disposition table

Disposition values: `mapped` (canonical event required) | `opaque-extension` | `control` (stream/control not a logical profile event) | `open` | `unreachable-with-evidence`.

| Tag constant | Byte | Writer | Loader case | ReadStream callback | Disposition | Notes |
|--------------|------|--------|-------------|---------------------|-------------|-------|
| `NYTP_TAG_NO_TAG` | `\0` | flag only | n/a | n/a | control | Not a stream record |
| `NYTP_TAG_ATTRIBUTE` | `:` | `NYTP_write_attribute_*` | yes | `ATTRIBUTE` | mapped | `:name=value\n` text form |
| `NYTP_TAG_OPTION` | `!` | `NYTP_write_option_*` | yes | `OPTION` | mapped | option snapshot |
| `NYTP_TAG_COMMENT` | `#` | `NYTP_write_comment` | yes | `COMMENT` | mapped | includes version/header comments |
| `NYTP_TAG_TIME_BLOCK` | `*` | `NYTP_write_time_block` | yes | `TIME_BLOCK` | mapped | elapsed I32 + overflow, fid, line, block_line, sub_line |
| `NYTP_TAG_TIME_LINE` | `+` | `NYTP_write_time_line` | yes | `TIME_LINE` | mapped | statement timing |
| `NYTP_TAG_DISCOUNT` | `-` | `NYTP_write_discount` | yes | `DISCOUNT` | mapped | profiler overhead discount marker |
| `NYTP_TAG_NEW_FID` | `@` | `NYTP_write_new_fid` | yes | `NEW_FID` | mapped | file id + eval parentage + name |
| `NYTP_TAG_SRC_LINE` | `S` | `NYTP_write_src_line` | yes | `SRC_LINE` | mapped | source line text bytes |
| `NYTP_TAG_SUB_INFO` | `s` | `NYTP_write_sub_info` | yes | `SUB_INFO` | mapped | sub definition range/name |
| `NYTP_TAG_SUB_CALLERS` | `c` | `NYTP_write_sub_callers` | yes | `SUB_CALLERS` | mapped | aggregate caller edge (shutdown) |
| `NYTP_TAG_PID_START` | `P` | `NYTP_write_process_start` | yes | `PID_START` | mapped | pid, ppid, time |
| `NYTP_TAG_PID_END` | `p` | `NYTP_write_process_end` | yes | `PID_END` | mapped | pid, end time |
| `NYTP_TAG_STRING` | `'` | string helper | via others | (embedded) | control | length-prefixed string payload helper |
| `NYTP_TAG_STRING_UTF8` | `"` | string helper | via others | (embedded) | control | UTF-8 flagged string helper |
| `NYTP_TAG_START_DEFLATE` | `z` | `NYTP_start_deflate_write_tag_comment` | yes | `START_DEFLATE` | control | compression switch |
| `NYTP_TAG_SUB_ENTRY` | `>` | `NYTP_write_call_entry` | yes | *(via stream; see open items)* | mapped | calls=2 entry; `caller_fid`, `caller_line` |
| `NYTP_TAG_SUB_RETURN` | `<` | `NYTP_write_call_return` | yes | *(via stream; see open items)* | mapped | calls=1/2 return; depth + name + times |

## Writer API inventory (FileHandle.h)

| Function | Related tag / role |
|----------|-------------------|
| `NYTP_write_header` | file magic/version prelude |
| `NYTP_write_comment` | `COMMENT` |
| `NYTP_write_attribute_string/signed/unsigned/nv` | `ATTRIBUTE` |
| `NYTP_write_option_pv/iv` | `OPTION` |
| `NYTP_start_deflate_write_tag_comment` | `START_DEFLATE` |
| `NYTP_write_process_start/end` | `PID_START` / `PID_END` |
| `NYTP_write_sawampersand` | **open** — present in header; no dedicated `NYTP_TAG_*` in same block |
| `NYTP_write_new_fid` | `NEW_FID` |
| `NYTP_write_time_block/line` | `TIME_BLOCK` / `TIME_LINE` |
| `NYTP_write_sub_info` | `SUB_INFO` |
| `NYTP_write_sub_callers` | `SUB_CALLERS` |
| `NYTP_write_src_line` | `SRC_LINE` |
| `NYTP_write_discount` | `DISCOUNT` |
| `NYTP_write_call_entry/return` | `SUB_ENTRY` / `SUB_RETURN` |

## ReadStream surface (observable order)

From `ReadStream.pm` POD (callback argument shapes):

- `VERSION`, attribute-like keys (`basetime`, `xs_version`, `perl_version`, `clock_id`, `ticks_per_sec`, `nv_size`, `application`)
- `COMMENT`, `ATTRIBUTE`, `OPTION`, `START_DEFLATE`
- `PID_START`, `NEW_FID`, `TIME_BLOCK`, `TIME_LINE`, `DISCOUNT`
- `SUB_INFO`, `SUB_CALLERS`, `SRC_LINE`, `PID_END`

**Open:** exact callback names/args for `SUB_ENTRY` / `SUB_RETURN` must be confirmed against `for_chunks` implementation and `t/22-readstream.t` (loader has cases; POD list above may be incomplete for call events). Treat as `mapped` with fixture-driven field freeze (see open items).

## Phase-0 “good enough” coverage checklist

| Surface | Covered in this provisional freeze? |
|---------|-------------------------------------|
| Default statement + block tags | yes |
| Discount | yes |
| FID + source | yes |
| Sub info + callers aggregates | yes |
| PID start/end | yes |
| Compression control | yes |
| Call entry/return tags | yes (fields open) |
| sawampersand | open |
| Full option matrix predicates | open (deferred to BASE-005 / COMPAT-004) |

## Open items

| ID | Item | Disposition until closed |
|----|------|--------------------------|
| OI-002-01 | Exact ReadStream callback names/args for SUB_ENTRY/SUB_RETURN | Capture via `tools/oracle/dump_readstream.pl` fixtures; legacy ReadStream is oracle |
| OI-002-02 | `NYTP_write_sawampersand` wire encoding | Trace writer call sites; map or mark opaque |
| OI-002-03 | Attribute key vocabulary complete list | Grep writers in NYTProf.xs; expand inventory |
| OI-002-04 | Option key vocabulary complete list | Same |
| OI-002-05 | Overflow field semantics on TIME_* (I32 elapsed + U32 overflow) | BASE-003 numeric notes |
| OI-002-06 | Incomplete file / missing PID_END behavior | Fixture truncation matrix (TEST-014 later) |

## Provisional freeze statement

All 18 `NYTP_TAG_*` constants from `FileHandle.h` have a disposition. No agent may invent additional stable v5 tags. Unknown future tags from older producers → `opaque-extension` or explicit error per COMPAT contract. **v6 wire IDs remain unfrozen.**

Machine-readable companion: `v5-record-inventory.json`.

**Logical-event mapping (COMPAT-001 provisional):** every **mapped** tag is bound to a named canonical event in [`docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md`](../../docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md) and [`docs/contracts/logical-events.schema.json`](../../docs/contracts/logical-events.schema.json). That contract is **not** a v6 wire freeze.

# COMPAT-001 — Provisional canonical logical-event contract

**Status:** provisional (Phase-0 taxonomy + field shapes)  
**Task:** COMPAT-001  
**Date:** 2026-08-07  
**Depends on:** BASE-001 (oracle pin), BASE-002 (v5 tag inventory), BASE-003 (timing/lifecycle notes)  
**Oracle:** Devel::NYTProf 6.15 (`baseline/6.15/`, tag `v6.15`)

---

## Not a v6 wire freeze

**This document is NOT a stable v6 wire format freeze.**

| Frozen here (provisional) | Explicitly unfrozen |
|---------------------------|---------------------|
| Named logical events for every Phase-0 **mapped** v5 tag | v6 opcodes, magic, prelude, feature flags |
| Field names, order, and domains as observed via ReadStream / loader | v6 varint / delta codecs |
| Dump tag strings aligned with `docs/schemas/canonical-event-dump-v0.md` | Default engine / default format flips |
| Unknown-tag hard-error policy for binary streams | Full option-matrix predicates (BASE-005 / COMPAT-004) |
| Open-item IDs that block full freeze | Merge-across-clock-domains (ADR-Q023), exotic NV widths (ADR-Q013) |

Wire IDs for v6 remain open under ADR-Q001–Q004 and later FMT tasks.  
Full COMPAT-001 **normative freeze** (acceptance: every 6.15 record representable without loss) is **not** claimed by this provisional landing; status remains `in-progress` until open items close.

Machine-readable companion: [`logical-events.schema.json`](logical-events.schema.json)  
Dump JSONL shape: [`docs/schemas/canonical-event-dump-v0.md`](../schemas/canonical-event-dump-v0.md)  
Tag inventory: [`baseline/inventories/v5-record-inventory.md`](../../baseline/inventories/v5-record-inventory.md)  
Timing notes: [`baseline/inventories/timing-lifecycle-notes.md`](../../baseline/inventories/timing-lifecycle-notes.md)  
Volatile normalization (provisional): [`COMPAT-002_VOLATILE_NORMALIZATION.md`](COMPAT-002_VOLATILE_NORMALIZATION.md)  
Precision / numeric policy (provisional): [`COMPAT-003_PRECISION_NUMERIC_POLICY.md`](COMPAT-003_PRECISION_NUMERIC_POLICY.md)

### Remaining full-freeze blockers (status stays provisional)

This document remains **provisional**. The following block **full** COMPAT-001 normative freeze and acceptance (they do **not** constitute a v6 wire freeze even when closed):

| Blocker | Where tracked |
|---------|----------------|
| Open field/domain items OI-001-*, OI-002-*, OI-003-* | Open items registry below |
| Volatile-field rules beyond shipped dump structural mode | [COMPAT-002](COMPAT-002_VOLATILE_NORMALIZATION.md) (OI-N-*) |
| Tick composition, NV units, display/NV portability | [COMPAT-003](COMPAT-003_PRECISION_NUMERIC_POLICY.md), BASE-003 |
| Option / surface matrices | BASE-005, [COMPAT-004 provisional](COMPAT-004_SURFACE_CLASSIFICATION.md) |
| ADR-Q001 taxonomy decision and later wire IDs | `docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md` |

**Do not** treat Phase-0 board “COMPAT-001 provisional done” as full task acceptance or as a v6 format freeze.

---

## Principles

1. **One logical event per observable mapped v5 record meaning** (ADR-Q001 recommended direction).
2. **Order is semantic:** event multiplicity and stream order must match oracle ReadStream dumps for the same profile and options (subject to volatile-field normalization under [COMPAT-002](COMPAT-002_VOLATILE_NORMALIZATION.md)).
3. **Do not invent closed semantics** for open items; mark `open` and fall back to oracle behavior.
4. **Control tags are not logical profile events** (string helpers, NO_TAG, compression switch). They may still appear in dumps for stream fidelity.
5. **v5 and future v6 decoders must emit the same ordered logical events** for dual comparison; wire representation may differ.
6. **Numeric domains:** statement/block ticks are exact integers; floating conversion at API/display/NV boundaries follows [COMPAT-003](COMPAT-003_PRECISION_NUMERIC_POLICY.md) (provisional).

---

## Disposition summary (all BASE-002 tags)

| Tag constant | Byte | ReadStream / dump tag | Disposition | Canonical logical event |
|--------------|------|----------------------|-------------|-------------------------|
| `NYTP_TAG_NO_TAG` | `\0` | — | control | *(none)* |
| `NYTP_TAG_ATTRIBUTE` | `:` | `ATTRIBUTE` | mapped | `attribute` |
| `NYTP_TAG_OPTION` | `!` | `OPTION` | mapped | `option` |
| `NYTP_TAG_COMMENT` | `#` | `COMMENT` | mapped | `comment` |
| `NYTP_TAG_TIME_BLOCK` | `*` | `TIME_BLOCK` | mapped | `time_block` |
| `NYTP_TAG_TIME_LINE` | `+` | `TIME_LINE` | mapped | `time_line` |
| `NYTP_TAG_DISCOUNT` | `-` | `DISCOUNT` | mapped | `discount` |
| `NYTP_TAG_NEW_FID` | `@` | `NEW_FID` | mapped | `new_fid` |
| `NYTP_TAG_SRC_LINE` | `S` | `SRC_LINE` | mapped | `src_line` |
| `NYTP_TAG_SUB_INFO` | `s` | `SUB_INFO` | mapped | `sub_info` |
| `NYTP_TAG_SUB_CALLERS` | `c` | `SUB_CALLERS` | mapped | `sub_callers` |
| `NYTP_TAG_PID_START` | `P` | `PID_START` | mapped | `pid_start` |
| `NYTP_TAG_PID_END` | `p` | `PID_END` | mapped | `pid_end` |
| `NYTP_TAG_STRING` | `'` | *(embedded)* | control | *(none)* |
| `NYTP_TAG_STRING_UTF8` | `"` | *(embedded)* | control | *(none)* |
| `NYTP_TAG_START_DEFLATE` | `z` | `START_DEFLATE` | control | *(none — stream control; dump-visible)* |
| `NYTP_TAG_SUB_ENTRY` | `>` | `SUB_ENTRY` | mapped | `sub_entry` |
| `NYTP_TAG_SUB_RETURN` | `<` | `SUB_RETURN` | mapped | `sub_return` |

### Dump-only prelude (not a `NYTP_TAG_*`)

| Dump tag | Role | Notes |
|----------|------|-------|
| `VERSION` | Header major/minor as dump event | From file magic/version prelude (`NYTP_write_header`); not a tag byte in the record stream. Included in dump schema for comparator alignment. |
| `_END` | Optional synthetic trailer | Dump convenience only; not part of profile semantics. |

---

## Type domains (provisional)

Used in field tables below. JSON dump conventions: see [`canonical-event-dump-v0.md`](../schemas/canonical-event-dump-v0.md).

| Code | Domain | Dump / JSON notes |
|------|--------|-------------------|
| `u` | non-negative integer (file-id, line, pid, flags, counts, depth, …) | JSON integer when exact |
| `i` | signed integer (statement/block **ticks** as composed from wire) | non-negative in practice for normal profiles; bit pattern from v5 varint |
| `n` | native NV image (Perl floating `NV`) | JSON number; units **open** where noted |
| `s` | string / byte payload | opaque bytes as oracle emits; UTF-8 flag path is control (`STRING_UTF8`) — see OI-001-02 |
| `empty` | no payload fields | empty `args` array in dump |

---

## Mapped logical events

Field **order** below is the canonical dump/`args` order (ReadStream / `callback_info` order). Order is meaningful for equality testing.

---

### `attribute`

| | |
|--|--|
| **Name** | `attribute` |
| **Source tag(s)** | `NYTP_TAG_ATTRIBUTE` (`:`), dump tag `ATTRIBUTE` |
| **Writer** | `NYTP_write_attribute_string` / `_signed` / `_unsigned` / `_nv` |
| **Meaning** | Profile metadata key/value from header or stream. Values are projected as strings in the dump. |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `key` | `s` | Attribute name (e.g. `basetime`, `application`, `xs_version`, `clock_id`, `ticks_per_sec`, `nv_size`, …) |
| `value` | `s` | String form of value as oracle emits |

**Order meaning:** Multiple `attribute` events appear in writer order (typically early header). Keys are not sorted.

**Open items:** OI-002-03 (complete key vocabulary). Structural volatiles for `basetime` / `application` are defined in [COMPAT-002](COMPAT-002_VOLATILE_NORMALIZATION.md) (provisional shipped rules).

---

### `option`

| | |
|--|--|
| **Name** | `option` |
| **Source tag(s)** | `NYTP_TAG_OPTION` (`!`), dump tag `OPTION` |
| **Writer** | `NYTP_write_option_pv` / `_iv` |
| **Meaning** | Profiler option snapshot as recorded in the file. |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `key` | `s` | Option name (e.g. `stmts`, `blocks`, `calls`, `compress`, `leave`, …) |
| `value` | `s` | String form of value as oracle emits |

**Order meaning:** Writer order of option snapshot (typically after attributes, before compression / body).

**Open items:** OI-002-04 (complete key vocabulary); full matrix deferred to BASE-005 / COMPAT-004.

---

### `comment`

| | |
|--|--|
| **Name** | `comment` |
| **Source tag(s)** | `NYTP_TAG_COMMENT` (`#`), dump tag `COMMENT` |
| **Writer** | `NYTP_write_comment` |
| **Meaning** | Free-text comment line(s), including version/header and compression notices. |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `text` | `s` | Comment body; may include trailing `\n` as oracle emits |

**Order meaning:** As written; not semantically ordered relative to profile points beyond stream position.

**Open items:** Structural compare replaces comment text with `<COMMENT>` per [COMPAT-002](COMPAT-002_VOLATILE_NORMALIZATION.md).

---

### `time_line`

| | |
|--|--|
| **Name** | `time_line` |
| **Source tag(s)** | `NYTP_TAG_TIME_LINE` (`+`), dump tag `TIME_LINE` |
| **Writer** | `NYTP_write_time_line` |
| **Meaning** | Statement-level timing sample: ticks attributed to a file/line. |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `ticks` | `i` | Logical ticks for this sample (composed from wire `I32 elapsed` + `U32 overflow` — composition **open** OI-003-01 / OI-002-05) |
| `fid` | `u` | File id |
| `line` | `u` | Source line |

**Order meaning:** Stream order of statement samples. Multiplicity must match oracle; do not coalesce adjacent same-line samples unless a future encoding proves exact re-expansion (FMT run encodings).

**Open items:** OI-002-05, OI-003-01; attribution state machine remains BASE-003 provisional.

---

### `time_block`

| | |
|--|--|
| **Name** | `time_block` |
| **Source tag(s)** | `NYTP_TAG_TIME_BLOCK` (`*`), dump tag `TIME_BLOCK` |
| **Writer** | `NYTP_write_time_block` |
| **Meaning** | Block-level timing sample when blocks profiling is enabled. |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `ticks` | `i` | Same tick domain as `time_line` (OI-003-01 / OI-002-05) |
| `fid` | `u` | File id |
| `line` | `u` | Line |
| `block_line` | `u` | Block start line (as writer/loader define) |
| `sub_line` | `u` | Sub start line association |

**Order meaning:** Interleaved with `time_line` / call events as the collector emits them.

**Open items:** OI-002-05, OI-003-01; when `blocks=0`, event absent (option matrix open).

---

### `discount`

| | |
|--|--|
| **Name** | `discount` |
| **Source tag(s)** | `NYTP_TAG_DISCOUNT` (`-`), dump tag `DISCOUNT` |
| **Writer** | `NYTP_write_discount` |
| **Meaning** | Marker that profiler overhead discount was applied at this stream position (exact legacy placement). |

| Field | Type | Domain / notes |
|-------|------|----------------|
| _(none)_ | `empty` | No payload |

**Order meaning:** Position relative to surrounding timing events is significant; do not drop or move.

**Open items:** Exact discount accounting relative to counts remains BASE-003 / fake-clock suite (TEST-003).

---

### `new_fid`

| | |
|--|--|
| **Name** | `new_fid` |
| **Source tag(s)** | `NYTP_TAG_NEW_FID` (`@`), dump tag `NEW_FID` |
| **Writer** | `NYTP_write_new_fid` |
| **Meaning** | Introduce a file id with eval parentage, flags, size, mtime, and name. |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `fid` | `u` | New file id |
| `eval_fid` | `u` | Parent eval fid (0 if none) |
| `eval_line` | `u` | Parent eval line |
| `flags` | `u` | File flags bitfield (semantic bit names **open** beyond observed dumps) |
| `size` | `u` | Size field as writer stores |
| `mtime` | `u` | mtime field as writer stores |
| `name` | `s` | Path or eval name string |

**Order meaning:** A fid must be introduced before events that reference it in a well-formed stream (incomplete streams: OI-002-06).

**Open items:** Path-like `name` → basename under [COMPAT-002](COMPAT-002_VOLATILE_NORMALIZATION.md); eval naming option matrix open.

---

### `src_line`

| | |
|--|--|
| **Name** | `src_line` |
| **Source tag(s)** | `NYTP_TAG_SRC_LINE` (`S`), dump tag `SRC_LINE` |
| **Writer** | `NYTP_write_src_line` |
| **Meaning** | Source text for a file/line (typically finalization phase). |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `fid` | `u` | File id |
| `line` | `u` | Line number |
| `text` | `s` | Source line bytes as stored; **not** treated as volatile |

**Order meaning:** Usually late in stream with other finalization records; order must match oracle for equality.

**Open items:** OI-003-04 (finalization order modes).

---

### `sub_info`

| | |
|--|--|
| **Name** | `sub_info` |
| **Source tag(s)** | `NYTP_TAG_SUB_INFO` (`s`), dump tag `SUB_INFO` |
| **Writer** | `NYTP_write_sub_info` |
| **Meaning** | Subroutine definition range and name. |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `fid` | `u` | Defining file id |
| `first_line` | `u` | First line of sub |
| `last_line` | `u` | Last line of sub |
| `name` | `s` | Sub name as stored |

**Order meaning:** Typically shutdown/finalization aggregate; preserve oracle order among peers.

**Open items:** Interaction with `nameanonsubs` / `nameevals` options (matrix open).

---

### `sub_callers`

| | |
|--|--|
| **Name** | `sub_callers` |
| **Source tag(s)** | `NYTP_TAG_SUB_CALLERS` (`c`), dump tag `SUB_CALLERS` |
| **Writer** | `NYTP_write_sub_callers` |
| **Meaning** | Aggregate caller-edge statistics (shutdown). |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `fid` | `u` | Caller file id |
| `line` | `u` | Caller line |
| `count` | `u` | Call count |
| `incl` | `n` | Inclusive time (NV image) |
| `excl` | `n` | Exclusive time (NV image) |
| `reci` | `n` | Recursive inclusive component (NV image) |
| `rec_depth` | `u` | Recursion depth field |
| `called` | `s` | Called sub name |
| `caller` | `s` | Caller identity string |

**Order meaning:** Aggregate finalization order; significant for dump equality.

**Open items:** Exact NV units / scaling vs ticks (ties to [COMPAT-003](COMPAT-003_PRECISION_NUMERIC_POLICY.md), OI-003-02 for related call paths); OI-003-04.

---

### `pid_start`

| | |
|--|--|
| **Name** | `pid_start` |
| **Source tag(s)** | `NYTP_TAG_PID_START` (`P`), dump tag `PID_START` |
| **Writer** | `NYTP_write_process_start` |
| **Meaning** | Process profile segment start. |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `pid` | `u` | Process id |
| `ppid` | `u` | Parent process id |
| `start_time` | `n` | Start time NV (wall-ish process time as writer stores) |

**Order meaning:** Marks process/run association for subsequent events until `pid_end` (fork: COL-015 later).

**Open items:** Fork re-init semantics deferred; OI-003-04 lifecycle modes.

---

### `pid_end`

| | |
|--|--|
| **Name** | `pid_end` |
| **Source tag(s)** | `NYTP_TAG_PID_END` (`p`), dump tag `PID_END` |
| **Writer** | `NYTP_write_process_end` |
| **Meaning** | Process profile segment end. |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `pid` | `u` | Process id |
| `end_time` | `n` | End time NV |

**Order meaning:** Closing boundary; missing end is incomplete-stream (OI-002-06).

**Open items:** OI-002-06, OI-003-04.

---

### `sub_entry`

| | |
|--|--|
| **Name** | `sub_entry` |
| **Source tag(s)** | `NYTP_TAG_SUB_ENTRY` (`>`), dump tag `SUB_ENTRY` |
| **Writer** | `NYTP_write_call_entry` |
| **Meaning** | Call-entry event when `calls=2` (entry+return mode). |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `caller_fid` | `u` | Caller file id |
| `caller_line` | `u` | Caller line |

**Order meaning:** Immediately associates with subsequent timing/return events in stream order; pairing rules for leave/XSUB/goto/exception **open** (OI-003-03).

**Open items:** OI-002-01 (callback/POD confirmation — fixtures already emit `SUB_ENTRY` with `uu` args), OI-003-03; absent when `calls=0` or `calls=1`.

**Fixture evidence:** `fixtures/v5/default-calls2/readstream.jsonl` shows `SUB_ENTRY` with `[caller_fid, caller_line]`.

---

### `sub_return`

| | |
|--|--|
| **Name** | `sub_return` |
| **Source tag(s)** | `NYTP_TAG_SUB_RETURN` (`<`), dump tag `SUB_RETURN` |
| **Writer** | `NYTP_write_call_return` |
| **Meaning** | Call-return event when `calls=1` or `calls=2`. |

| Field | Type | Domain / notes |
|-------|------|----------------|
| `depth` | `u` | Profile depth at return |
| `incl_time` | `n` | Inclusive time (units **open** — OI-003-02) |
| `excl_time` | `n` | Exclusive time (units **open** — OI-003-02) |
| `subname` | `s` | Returned sub name |

**Order meaning:** Stream order of returns; depth nesting must match oracle for well-formed runs.

**Open items:** OI-002-01, OI-003-02, OI-003-03.

**Fixture evidence:** `fixtures/v5/default-calls1/` and `default-calls2/` emit `SUB_RETURN` with `[depth, incl, excl, subname]`.

---

## Control / non-logical (inventory completeness)

These are **not** canonical logical profile events. Decoders still must handle them correctly on the wire.

| Tag | Role | Decoder policy (provisional) |
|-----|------|------------------------------|
| `NYTP_TAG_NO_TAG` | Flag / sentinel only | Not a stream record |
| `NYTP_TAG_STRING` / `STRING_UTF8` | Length-prefixed string payload helpers | Embedded inside other records; not standalone logical events |
| `NYTP_TAG_START_DEFLATE` | Compression switch | Stream control; dump may emit `START_DEFLATE` with empty args for ReadStream fidelity |

**Writer open:** `NYTP_write_sawampersand` — no dedicated tag in inventory; OI-002-02.

---

## Order and process association (provisional)

Typical well-formed default stream shape (illustrative, not a hard state machine):

```text
VERSION (dump prelude)
COMMENT / ATTRIBUTE* / OPTION* / COMMENT?
START_DEFLATE?          # control, if compress
PID_START
  ( NEW_FID | TIME_* | DISCOUNT | SUB_ENTRY? | SUB_RETURN? )*
  SUB_INFO*  SUB_CALLERS*  SRC_LINE*   # finalization cluster — order open OI-003-04
PID_END
```

| Rule | Status |
|------|--------|
| Preserve exact multiplicity and order of mapped events vs oracle dump | **required** |
| Sequence numbers (`seq` in dump) monotonic per dump | dump convention; COL-003 for collector dual-output |
| Process association via surrounding `pid_start`/`pid_end` | provisional |
| Incomplete / truncated files | **open** OI-002-06 |
| enable/disable/restart / file switch | **open** OI-003-05 |

---

## Unknown and unsupported tags

Aligned with dump schema:

- After the binary stream begins, unknown tag bytes → **hard error** (`unsupported tag 0xNN at offset O` or equivalent).
- Do **not** silently drop unknown tags.
- Older producers with unknown future tags → `opaque-extension` or explicit error per later COMPAT freeze; no new stable v5 tags may be invented (BASE-002 freeze statement).

---

## Open items registry

### Carried from BASE-002 (OI-002-*)

| ID | Item | Blocks / notes |
|----|------|----------------|
| OI-002-01 | Exact ReadStream callback names/args for SUB_ENTRY/SUB_RETURN | Fixtures already show `SUB_ENTRY`/`SUB_RETURN`; close when POD/`for_chunks` fully audited |
| OI-002-02 | `NYTP_write_sawampersand` wire encoding | Map or mark opaque |
| OI-002-03 | Attribute key vocabulary complete list | Expand inventory from writers |
| OI-002-04 | Option key vocabulary complete list | Expand inventory from writers |
| OI-002-05 | Overflow field semantics on TIME_* | Shared with OI-003-01 |
| OI-002-06 | Incomplete file / missing PID_END behavior | Provisional: [`COMPAT-010_INCOMPLETE_STREAM.md`](COMPAT-010_INCOMPLETE_STREAM.md) (verify/report fail closed; dump lenient) — full TEST-014 matrix later |

### Carried from BASE-003 (OI-003-*)

| ID | Item | Blocks / notes |
|----|------|----------------|
| OI-003-01 | elapsed+overflow → logical ticks | `time_line` / `time_block` ticks field |
| OI-003-02 | call-return numeric units | `sub_return` incl/excl |
| OI-003-03 | leave / XSUB / goto / exception matrix | call event pairing |
| OI-003-04 | Finalization order modes | `sub_info` / `sub_callers` / `src_line` / `pid_end` |
| OI-003-05 | enable/disable/restart | lifecycle |
| OI-003-06 | clock anomaly matrix | platform later |

### New under this provisional contract (OI-001-*)

| ID | Item | Notes |
|----|------|-------|
| OI-001-01 | Promote this provisional taxonomy to full COMPAT-001 freeze | Requires closing OI-002/003 blockers that affect field domains; ADR-Q001 decision |
| OI-001-02 | UTF-8 vs byte string identity on named string fields | When `STRING_UTF8` helper is used, dump/API must preserve flag vs lossy decode — exact surface open until vectors |
| OI-001-03 | Whether dump-visible control (`START_DEFLATE`) and prelude (`VERSION`) participate in dual-output sequence numbers | COL-003 must specify; provisional dumps include them for ReadStream equality |
| OI-001-04 | `new_fid.flags` bit vocabulary | Document named bits from XS when audited |
| OI-001-05 | Sawampersand as logical vs opaque | Depends on OI-002-02 |

Do **not** invent closed answers for the above; implement to oracle fixtures until closed.

---

## Cross-links

| Artifact | Path |
|----------|------|
| This contract | `docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md` |
| Machine schema | `docs/contracts/logical-events.schema.json` |
| Dump schema v0 | `docs/schemas/canonical-event-dump-v0.md` |
| Volatile normalization (COMPAT-002) | `docs/contracts/COMPAT-002_VOLATILE_NORMALIZATION.md` |
| Precision / numeric policy (COMPAT-003) | `docs/contracts/COMPAT-003_PRECISION_NUMERIC_POLICY.md` |
| Surface classification (COMPAT-004 provisional) | `docs/contracts/COMPAT-004_SURFACE_CLASSIFICATION.md` |
| Normalize tool | `tools/oracle/normalize_jsonl.py` |
| COMPAT normalize selftest | `tools/oracle/selftest_normalize_compat.sh` |
| v5 record inventory | `baseline/inventories/v5-record-inventory.md` |
| v5 inventory JSON | `baseline/inventories/v5-record-inventory.json` |
| Timing / lifecycle | `baseline/inventories/timing-lifecycle-notes.md` |
| COMPAT-000 ratification | `docs/governance/COMPAT-000_RATIFICATION.md` |
| COMPAT-001 task definition | `docs/plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md` |
| ADR-Q001 taxonomy | `docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md` |
| Example dumps | `fixtures/v5/default-calls1/readstream.jsonl`, `fixtures/v5/default-calls2/readstream.jsonl` |

---

## Provisional acceptance (Phase-0)

This provisional COMPAT-001 landing is **done for Phase-0 board purposes** when:

1. Every BASE-002 **mapped** tag has a named logical event above.  
2. Field order matches dump schema / oracle fixtures for known tags.  
3. Open items are listed, not silently closed.  
4. Explicit “not a v6 wire freeze” statement is present.  

Full task acceptance (plan XL deliverable: every 6.15 record without ambiguity) remains **in-progress** until OI-001-01 and blocking OI-002/003 items close.

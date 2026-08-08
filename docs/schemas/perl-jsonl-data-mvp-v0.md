# Pure-Perl JSONL Data query MVP (v0)

**Status:** first-slice pure-Perl dump aggregator (not full XS `Devel::NYTProf::Data`)  
**Board IDs:** `PERL-DATA-JSONL`, `PERL-LINE-TOTALS`, `PERL-A4B-JSONL`, `PERL-SUBDEFS-JSONL`, `PERL-SOURCE-JSONL`, `PERL-META-JSONL`, `PERL-PID-JSONL`, `PERL-STREAM-COMPLETE`, `PERL-DISCOUNT-JSONL`, `PERL-SUB-ENTRY-JSONL`  
**Not:** full `PERL-*` Data materializer / XS FileHandle / eval-depth parity

**Related:**

- Stream bridge: [perl-jsonl-readstream-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-jsonl-readstream-mvp-v0.md)
- Aggregate definitions: [aggregate-comparison-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/aggregate-comparison-v0.md)
- Record shape / tags: [canonical-event-dump-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/canonical-event-dump-v0.md)
- Report semantic parity (leaf/mid/edge): [report-semantic-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/report-semantic-parity-mvp-v0.md)
- Blocks semantics (line 5 = 780): [blocks-semantic-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/blocks-semantic-parity-mvp-v0.md)
- Incomplete stream contract: [COMPAT-010_INCOMPLETE_STREAM.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md)

## Goal

A **pure-Perl** module under `perl/` that:

1. Consumes native dump JSONL (committed golden **or** subprocess to shipped `nytprof-cli dump`)
2. Builds queryable **subroutine return totals** (from `SUB_RETURN`) and **call-edge counts** (from `SUB_CALLERS`)
3. Builds **A4 line totals** from **`TIME_LINE` and `TIME_BLOCK`** (statement `fid`/`line`)
4. Builds **A4b block_line totals** from **`TIME_BLOCK` only** (`fid` / `block_line` at args\[3\])
5. Builds **A9 sub definitions** from **`SUB_INFO`** and **file identity** from **`NEW_FID`**
6. Builds **A8 source lines** from **`SRC_LINE`** (`"fid:line" => text`, last write wins)
7. Builds **profile metadata** from **`ATTRIBUTE`** and **`OPTION`** (`key => value`, last write wins; dump values as-is)
8. Builds **process lifecycle** from **`PID_START`** / **`PID_END`** (event lists + counts; dump-derived PIDs only)
9. Exposes **stream completeness** aligned with [`COMPAT-010_INCOMPLETE_STREAM`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md): `is_stream_complete` / `stream_incompleteness_reasons` using `pid_*_events` + `time_line_events` / `time_block_events`
10. Counts **A3 `discount_events`** from **`DISCOUNT`** tags (empty args; **event multiplicity only** — not exclusive-time policy freeze)
11. Counts **`sub_entry_events`** from **`SUB_ENTRY`** tags (`caller_fid`, `caller_line`; **event multiplicity only** — not full call-stack / arg freeze)
12. Counts stream tag multiplicities (**JSON-EVENT-COUNTS-MVP**): `sub_return_events` / `new_fid_events` / `sub_callers_events` / `src_line_events` / `sub_info_events` — one per matching tag successfully ingested
13. On `fixtures/v5/default-calls1`, observes from **real dump events**:
   - `main::leaf` returns **15**
   - `main::mid` returns **3**
   - mid→leaf edge count **15**
   - `sub_def('main::leaf')` → `fid=1`, `first_line=3`, `last_line=7`
   - `sub_def('main::mid')` → `fid=1`, `first_line=8`, `last_line=12`
   - `file(1)` path contains **`workload.pl`**; `file_basename(1) eq 'workload.pl'`
   - `source_line(1, 5)` exact dump text **`    $x++ for 1 .. 50;\n`** (contains `$x++` and `1 .. 50`)
   - `attribute('ticks_per_sec')` and/or `attribute('basetime')` defined; `option('calls')` defined
   - `pid_start_count >= 1`, `pid_end_count >= 1`; start pid matches end pid (**2975381** on committed golden)
   - `is_stream_complete` **true**; `stream_incompleteness_reasons` **empty**; `time_line_events > 0`
   - `discount_events` / `discount_count` **818** (A3; independent stream re-count of `DISCOUNT` tags)
   - `sub_entry_events` / `sub_entry_count` **0** (`calls=1`; independent stream re-count of `SUB_ENTRY` tags)
   - `sub_return_events` **27**, `new_fid_events` **3**, `sub_callers_events` **13**, `src_line_events` **632**, `sub_info_events` **31** (stream re-count of tags)
   - `block_line_totals` **empty** (no `TIME_BLOCK`)
14. On `fixtures/v5/calls2-default` (`calls=2` → `SUB_ENTRY` present), observes:
   - `sub_entry_events` / `sub_entry_count` **27** (independent stream re-count of `SUB_ENTRY` tags)
15. On `fixtures/v5/blocks-calls1` (`blocks=1` → timing as `TIME_BLOCK`), observes:
   - `line_calls(1, 5) == 780` (A4 hot loop statement line)
   - `block_line_totals` non-empty; sample **`"1:4".calls == 810`** (A4b from `block_line`)
   - `is_stream_complete` **true** (`time_block_events > 0`)

No XS, no FFI, no oracle `PERL5LIB`. Core `JSON::PP` only (via `JsonlReadStream`).

## Module

| Path | Role |
|------|------|
| `perl/lib/Devel/NYTProf/JsonlData.pm` | JSONL → queryable sub / edge / line / block_line / source / sub_defs / files / attributes / options / PID lifecycle / discount (A3) / SUB_ENTRY multiplicity / stream completeness |
| `perl/lib/Devel/NYTProf/JsonlReadStream.pm` | Underlying line/stream parse (reused) |
| `perl/t/jsonl_data_default_calls1.t` | Fixture aggregation assertions (subs/edges) |
| `perl/t/jsonl_data_blocks_calls1_line_totals.t` | blocks-calls1 A4 `line_calls(1,5)==780` |
| `perl/t/jsonl_data_a4b_blocks_calls1.t` | blocks-calls1 A4b `block_line_totals` + `"1:4".calls==810` + A4 780 |
| `perl/t/jsonl_data_subdefs_default_calls1.t` | default-calls1 A9 leaf/mid ranges + file 1 basename |
| `perl/t/jsonl_data_source_default_calls1.t` | default-calls1 A8 `source_line(1,5)` dump text |
| `perl/t/jsonl_data_meta_default_calls1.t` | default-calls1 ATTRIBUTE / OPTION metadata |
| `perl/t/jsonl_data_pid_default_calls1.t` | default-calls1 PID_START / PID_END counts + matching pid |
| `perl/t/jsonl_data_stream_complete_default_calls1.t` | default-calls1 complete + crafted incomplete (COMPAT-010) |
| `perl/t/jsonl_data_discount_default_calls1.t` | default-calls1 A3 `discount_events` / stream re-count (**818**) |
| `perl/t/jsonl_data_sub_entry.t` | default-calls1 **0** + calls2-default **27** `sub_entry_events` / stream re-count |
| `scripts/packaging/perl_jsonl_data_smoke.sh` | Packaging smoke (default-calls1 golden + optional native dump) |
| `scripts/packaging/perl_line_totals_smoke.sh` | Line totals smoke (blocks 780 + default leaf/mid) |
| `scripts/packaging/perl_a4b_smoke.sh` | A4b smoke (blocks `1:4=810` + A4 780 + default empty A4b) |
| `scripts/packaging/perl_subdefs_smoke.sh` | Sub defs + file identity smoke (leaf/mid ranges + workload.pl) |
| `scripts/packaging/perl_source_smoke.sh` | Source lines smoke (`source_line(1,5)` + optional native dump) |
| `scripts/packaging/perl_meta_smoke.sh` | Metadata smoke (`attribute` / `option` + optional native dump) |
| `scripts/packaging/perl_pid_smoke.sh` | PID lifecycle smoke (`pid_start_count` / `pid_end_count` + optional native dump) |
| `scripts/packaging/perl_stream_complete_smoke.sh` | Stream completeness smoke (complete golden + incomplete craft) |
| `scripts/packaging/perl_discount_smoke.sh` | DISCOUNT A3 multiplicity smoke (`discount_events` **818** + optional native dump) |
| `scripts/packaging/perl_sub_entry_smoke.sh` | SUB_ENTRY multiplicity smoke (default **0** / calls2 **27** + optional native dump) |
| `scripts/packaging/perl_jsonl_data_all_smoke.sh` | Thin fail-fast roll-up of the pure-Perl JsonlData smokes above (offline gate step 5) |

## API

```perl
use Devel::NYTProf::JsonlData;

my $data = Devel::NYTProf::JsonlData->from_jsonl($jsonl_path);
# or:
# my $data = Devel::NYTProf::JsonlData->from_cli([ $cli, 'dump', $profile ]);
# my $data = Devel::NYTProf::JsonlData->from_fh($fh);

$data->sub_returns('main::leaf');                    # count (0 if missing)
$data->sub_return_totals;                            # { name => count, ... }
$data->call_edge_count('main::mid', 'main::leaf');   # 15
$data->call_edge_totals;                             # { "caller\tcallee" => count }
$data->line_totals;                                  # A4: "fid:line" => { calls, ticks }
$data->line_calls($fid, $line);                      # A4 call count (0 if missing)
$data->block_line_totals;                            # A4b: "fid:block_line" => { calls, ticks }
$data->block_line_calls($fid, $block_line);          # A4b call count (0 if missing)
$data->sub_def('main::leaf');                        # A9: { fid, first_line, last_line } or undef
$data->sub_defs;                                     # { name => { fid, first_line, last_line }, ... }
$data->file($fid);                                   # full path from NEW_FID (or undef)
$data->files;                                        # { fid => path, ... }
$data->file_basename($fid);                          # basename of file($fid) (or undef)
$data->source_line($fid, $line);                     # A8 exact SRC_LINE text (or undef)
$data->source_lines;                                 # { "fid:line" => text, ... }
$data->attribute($key);                              # ATTRIBUTE value (or undef)
$data->attributes;                                   # { key => value, ... }
$data->option($key);                                 # OPTION value (or undef)
$data->options;                                      # { key => value, ... }
$data->pid_start_count;                              # PID_START event count
$data->pid_end_count;                                # PID_END event count
$data->pid_starts;                                   # [ { pid, ppid?, start_time? }, ... ]
$data->pid_ends;                                     # [ { pid, end_time? }, ... ]
$data->pids;                                         # sorted unique pids from starts/ends
$data->time_line_events;                             # TIME_LINE event count
$data->time_block_events;                            # TIME_BLOCK event count
$data->discount_events;                              # A3 DISCOUNT event count (multiplicity only)
$data->discount_count;                               # alias for discount_events
$data->sub_entry_events;                             # SUB_ENTRY event count (multiplicity only)
$data->sub_entry_count;                              # alias for sub_entry_events
$data->sub_return_events;                            # SUB_RETURN tag multiplicity (JSON-EVENT-COUNTS-MVP)
$data->new_fid_events;                               # NEW_FID tag multiplicity
$data->sub_callers_events;                           # SUB_CALLERS tag multiplicity (not edge sum)
$data->src_line_events;                              # SRC_LINE tag multiplicity
$data->sub_info_events;                              # SUB_INFO tag multiplicity
$data->is_stream_complete;                           # 1 iff incompleteness reasons empty
$data->stream_incompleteness_reasons;                # arrayref of reason strings (empty if complete)
$data->records_seen;                                 # JSONL records processed
```

### Constructors

| Method | Input |
|--------|--------|
| `from_jsonl($path)` | UTF-8 JSONL file path |
| `from_cli(\@argv)` | Subprocess stdout as JSONL (`open '-|'`) |
| `from_fh($fh)` | Open readable handle |

### Line queries (A4)

| Method | Returns |
|--------|---------|
| `line_totals` | Hashref `"fid:line" => { calls => N, ticks => sum }` from **`TIME_LINE` + `TIME_BLOCK`** statement line |
| `line_calls($fid, $line)` | Integer call count for that location (missing → **0**) |

Empty `line_totals` only when the dump has neither timing tag.

### Block-line queries (A4b)

| Method | Returns |
|--------|---------|
| `block_line_totals` | Hashref `"fid:block_line" => { calls => N, ticks => sum }` from **`TIME_BLOCK` only** using `block_line` at **args\[3\]** |
| `block_line_calls($fid, $block_line)` | Integer call count for that block start line (missing → **0**) |

Empty `block_line_totals` when the dump has no `TIME_BLOCK` (e.g. default-calls1).

### Sub definitions (A9) and file identity

| Method | Returns |
|--------|---------|
| `sub_def($name)` | Hashref `{ fid => N, first_line => N, last_line => N }` from **`SUB_INFO`** (last write wins), or **undef** if missing |
| `sub_defs` | Hashref copy `name => { fid, first_line, last_line }` for all seen names |
| `file($fid)` | Full path string from **`NEW_FID`** (last write wins per fid), or **undef** |
| `files` | Hashref copy `fid => path` |
| `file_basename($fid)` | Basename of `file($fid)` (`/` or `\` separators); bare names unchanged; missing → **undef** |

`SUB_INFO` args order: `[fid, first_line, last_line, name]`. `NEW_FID` stores the **last arg** (path) at `fid` (args\[0\]).

### Source lines (A8)

| Method | Returns |
|--------|---------|
| `source_line($fid, $line)` | Exact source text string from **`SRC_LINE`** (last write wins), or **undef** if missing |
| `source_lines` | Hashref copy `"fid:line" => text` for all seen `(fid, line)` keys |

`SRC_LINE` args order: `[fid, line, text]`. Text is stored **exactly** as the dump emits it (including trailing newline); do not invent or normalize source.

### Profile metadata (ATTRIBUTE / OPTION)

| Method | Returns |
|--------|---------|
| `attribute($key)` | Value string from **`ATTRIBUTE`** (last write wins), or **undef** if missing |
| `attributes` | Hashref copy `key => value` for all seen ATTRIBUTE keys |
| `option($key)` | Value string from **`OPTION`** (last write wins), or **undef** if missing |
| `options` | Hashref copy `key => value` for all seen OPTION keys |

`ATTRIBUTE` / `OPTION` args order: `[key, value]`. Values are stored **exactly** as the dump emits them (do not invent, coerce, or normalize). Typical default-calls1 keys include attributes `basetime`, `application`, `ticks_per_sec`, `xs_version` and options `calls`, `blocks`, `stmts`, `compress`.

### Process lifecycle (PID_START / PID_END)

| Method | Returns |
|--------|---------|
| `pid_start_count` / `pid_start_events` | Integer count of **`PID_START`** events |
| `pid_end_count` / `pid_end_events` | Integer count of **`PID_END`** events |
| `pid_starts` | Arrayref of hashrefs `{ pid => N, ppid => ?, start_time => ? }` (optional fields only when dump provides them); element copies |
| `pid_ends` | Arrayref of hashrefs `{ pid => N, end_time => ? }`; element copies |
| `pids` | Sorted arrayref of unique integer PIDs seen in starts and/or ends |

`PID_START` args: `[pid, ppid?, start_time?]`. `PID_END` args: `[pid, end_time?]`. **Do not invent PIDs** — only store dump-derived values. On default-calls1 committed golden: one start and one end for pid **2975381**.

### Discount events (A3 DISCOUNT)

| Method | Returns |
|--------|---------|
| `discount_events` / `discount_count` | Integer count of **`DISCOUNT`** tags (empty args); **event multiplicity only** |

`DISCOUNT` is a zero-arg marker that profiler overhead discount was applied at this stream position. JsonlData counts tags only — it does **not** implement exclusive-time policy freeze / fake-clock discount accounting (BASE-003 / TEST-003). On default-calls1 committed golden, independent stream re-count observes **818**.

### SUB_ENTRY events (calls≥2 call-site entries)

| Method | Returns |
|--------|---------|
| `sub_entry_events` / `sub_entry_count` | Integer count of **`SUB_ENTRY`** tags; **event multiplicity only** |

`SUB_ENTRY` is emitted when NYTPROF `calls>=2` (richer call-site detail). Args order: `[caller_fid, caller_line]`. JsonlData counts tags only — it does **not** freeze full call-stack / arg semantics. On default-calls1 (`calls=1`) independent stream re-count observes **0**. On calls2-default (`calls=2`) independent stream re-count observes **27**. Sample: `{"args":[1,1],"seq":32,"tag":"SUB_ENTRY"}`.

### Stream completeness (COMPAT-010_INCOMPLETE_STREAM)

Aligned with Rust `ProfileModel::is_stream_complete` / `stream_incompleteness_reasons` and [`docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md).

| Method | Returns |
|--------|---------|
| `time_line_events` | Integer count of successfully ingested **`TIME_LINE`** events |
| `time_block_events` | Integer count of successfully ingested **`TIME_BLOCK`** events |
| `is_stream_complete` | Boolean (Perl true/false): **true** iff `stream_incompleteness_reasons` is empty |
| `stream_incompleteness_reasons` | Arrayref of reason strings (fresh copy each call); empty when complete |

Completeness rules (all must hold):

1. **PID balance:** if `pid_start_events > 0`, then `pid_end_events >= pid_start_events`
2. **Statement timing:** `time_line_events + time_block_events > 0` (equivalently non-empty `line_totals` when counters are used)

Canonical reason strings (stable for grepping; match Rust model wording):

| Condition | Reason string |
|-----------|---------------|
| PID start without matching end | `missing PID_END after PID_START` |
| No `TIME_LINE` / `TIME_BLOCK` | `no statement timing events (TIME_LINE/TIME_BLOCK)` |

Notes:

- `from_jsonl` / load **may succeed** on incomplete streams (same as Rust model load); completeness is a query, not a constructor failure.
- Incomplete test evidence prefers filtering **real golden dump lines** (e.g. keep only `VERSION` / `ATTRIBUTE` / `OPTION` / `COMMENT` / `START_DEFLATE`, optionally `PID_START`) — do not invent fake event semantics.

## Tag argument shapes (MVP)

Aligned with [canonical-event-dump-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/canonical-event-dump-v0.md) and aggregate-comparison A4/A5/A7/A8/A9:

| Tag | Args order | Aggregation |
|-----|------------|-------------|
| `SUB_RETURN` | `depth, incl_time, excl_time, subname` | **subname at index 3**; each event increments that name’s return count by 1 |
| `SUB_CALLERS` | `fid, line, count, incl, excl, reci, rec_depth, called, caller` | **count @ 2**, **callee @ 7 (`args[-2]`)**, **caller @ 8 (`args[-1]`)**; sum `count` per `(caller, callee)` |
| `TIME_LINE` | `ticks, fid, line` | A4 `line_totals`: one call + sum ticks per `(fid, line)` |
| `TIME_BLOCK` | `ticks, fid, line, block_line, sub_line` | A4 `line_totals` uses **statement** `(fid, line)` at the **same indices as `TIME_LINE`** (ticks@0, fid@1, line@2). **Also** A4b `block_line_totals` via `(fid, block_line)` at fid@1, block_line@3. Profiles with only `blocks=1` therefore have non-empty A4 and A4b. |
| `SRC_LINE` | `fid, line, text` | A8 `source_lines`: **`"fid:line" → text`**; **last write wins** if the same key appears more than once |
| `SUB_INFO` | `fid, first_line, last_line, name` | A9 `sub_defs`: **name → { fid, first_line, last_line }**; **last write wins** if the same name appears more than once |
| `NEW_FID` | `fid, eval_fid, eval_line, flags, size, mtime, name` | `files`: **fid → full path** (last arg); `file_basename($fid)` derives basename only |
| `ATTRIBUTE` | `key, value` | `attributes`: **key → value**; **last write wins** if the same key appears more than once |
| `OPTION` | `key, value` | `options`: **key → value**; **last write wins** if the same key appears more than once |
| `PID_START` | `pid, ppid?, start_time?` | Append to `pid_starts`; increment `pid_start_count` / `pid_start_events` |
| `PID_END` | `pid, end_time?` | Append to `pid_ends`; increment `pid_end_count` / `pid_end_events` |
| `DISCOUNT` | _(empty)_ | A3: increment `discount_events` / `discount_count` by 1 per tag (multiplicity only) |
| `SUB_ENTRY` | `caller_fid, caller_line` | Increment `sub_entry_events` / `sub_entry_count` by 1 per tag (multiplicity only; not arg freeze) |

### SUB_CALLERS sample (default-calls1 mid→leaf)

```json
{"tag":"SUB_CALLERS","args":[1,10,15,5.24e-05,5.24e-05,0,0,"main::leaf","main::mid"]}
```

- `caller` = `args[-1]` = `main::mid`
- `callee` = `args[-2]` = `main::leaf`
- `count`  = `args[2]`  = **15**

### TIME_BLOCK sample (blocks-calls1 statement + block line)

```json
{"tag":"TIME_BLOCK","args":[14,1,1,1,1],"seq":32}
```

- `ticks` = `args[0]`, `fid` = `args[1]`, `line` = `args[2]` → A4 key `"fid:line"`
- `block_line` = `args[3]`, `sub_line` = `args[4]` → A4b key `"fid:block_line"`

### SUB_INFO sample (default-calls1 leaf / mid)

```json
{"tag":"SUB_INFO","args":[1,3,7,"main::leaf"]}
{"tag":"SUB_INFO","args":[1,8,12,"main::mid"]}
```

- leaf: `fid=1`, `first_line=3`, `last_line=7`
- mid: `fid=1`, `first_line=8`, `last_line=12`

### NEW_FID sample (default-calls1 workload)

```json
{"tag":"NEW_FID","args":[1,0,0,52,0,0,"/tmp/.../workload.pl"]}
```

- `fid` = `args[0]` = **1**
- path = last arg → full path stored; basename **`workload.pl`**

### SRC_LINE sample (default-calls1 line 5)

```json
{"tag":"SRC_LINE","args":[1,5,"    $x++ for 1 .. 50;\n"]}
```

- `fid` = `args[0]` = **1**, `line` = `args[1]` = **5**
- `text` = `args[2]` = **`    $x++ for 1 .. 50;\n`** (exact; trailing newline kept)

### ATTRIBUTE / OPTION sample (default-calls1 header)

```json
{"tag":"ATTRIBUTE","args":["basetime","1786111723"]}
{"tag":"ATTRIBUTE","args":["ticks_per_sec","10000000"]}
{"tag":"ATTRIBUTE","args":["xs_version","6.15"]}
{"tag":"OPTION","args":["calls","1"]}
{"tag":"OPTION","args":["blocks","0"]}
{"tag":"OPTION","args":["stmts","1"]}
{"tag":"OPTION","args":["compress","6"]}
```

- `key` = `args[0]`, `value` = `args[1]` (strings as dump emits)
- Values above are **observed** on the committed golden; do not invent alternatives. `basetime` / `application` are volatile across recaptures (normalize for structural compare).

### PID_START / PID_END sample (default-calls1)

```json
{"tag":"PID_START","args":[2975381,2975366,1786111723.96777],"seq":30}
{"tag":"PID_END","args":[2975381,1786111723.97052],"seq":2472}
```

- `PID_START`: `pid` = `args[0]` = **2975381**, `ppid` = `args[1]`, `start_time` = `args[2]` (NV as JSON number)
- `PID_END`: `pid` = `args[0]` = **2975381**, `end_time` = `args[1]`
- Counts: `pid_start_count == 1`, `pid_end_count == 1` on this golden; start pid matches end pid
- PIDs are **fixture-capture values** (volatile across recaptures of a new profile); the committed golden is fixed at **2975381**

### DISCOUNT sample (default-calls1)

```json
{"tag":"DISCOUNT","args":[],"seq":36}
```

- Empty `args` per dump schema / COMPAT-001 `discount` logical event
- Each tag increments `discount_events` by **1** (A3 multiplicity)
- Independent stream re-count on committed golden: **818** (also `aggregates.oracle.json` `discount_events`)
- **Not** exclusive-time policy freeze — count only

### SUB_ENTRY sample (calls2-default)

```json
{"args":[1,1],"seq":32,"tag":"SUB_ENTRY"}
```

- `caller_fid` = `args[0]` = **1**, `caller_line` = `args[1]` = **1**
- Each tag increments `sub_entry_events` by **1** (multiplicity only)
- Independent stream re-count: default-calls1 (**calls=1**) **0**; calls2-default (**calls=2**) **27**
- **Not** full call-stack / arg freeze — count only

## Fixture contract (default-calls1)

| Field | Value |
|-------|-------|
| Golden dump | `fixtures/v5/default-calls1/readstream.jsonl` |
| Profile | `fixtures/v5/default-calls1/nytprof.out` |
| `main::leaf` `SUB_RETURN` count | **15** |
| `main::mid` `SUB_RETURN` count | **3** |
| `main::mid` → `main::leaf` edge count | **15** |
| `sub_def('main::leaf')` | **fid=1, first_line=3, last_line=7** |
| `sub_def('main::mid')` | **fid=1, first_line=8, last_line=12** |
| `file(1)` / `file_basename(1)` | path contains **`workload.pl`** / basename **`workload.pl`** |
| `source_line(1, 5)` | **`    $x++ for 1 .. 50;\n`** (from real `SRC_LINE`) |
| `attribute('ticks_per_sec')` / `attribute('basetime')` | defined from real `ATTRIBUTE` (values dump-derived) |
| `option('calls')` | defined from real `OPTION` (value dump-derived; golden observes `"1"`) |
| `pid_start_count` / `pid_end_count` | **≥ 1** each (golden observes **1** / **1**) |
| start / end `pid` | match; golden observes **2975381** (dump-derived; do not invent) |
| `is_stream_complete` | **true** |
| `stream_incompleteness_reasons` | **empty** arrayref |
| `time_line_events` | **> 0** (default-calls1 uses `TIME_LINE`; `time_block_events == 0`) |
| `discount_events` / `discount_count` | **818** (A3; independent stream re-count of `DISCOUNT` tags) |
| `sub_entry_events` / `sub_entry_count` | **0** (`calls=1`; independent stream re-count of `SUB_ENTRY` tags) |

## Fixture contract (calls2-default) — PERL-SUB-ENTRY-JSONL

| Field | Value |
|-------|-------|
| Golden dump | `fixtures/v5/calls2-default/readstream.jsonl` |
| Profile | `fixtures/v5/calls2-default/nytprof.out` |
| NYTPROF | `trace=0:start=begin:calls=2` |
| `sub_entry_events` / `sub_entry_count` | **27** (independent stream re-count of `SUB_ENTRY` tags) |
| Sample | `{"args":[1,1],"seq":32,"tag":"SUB_ENTRY"}` |

## Fixture contract (blocks-calls1) — PERL-LINE-TOTALS / PERL-A4B-JSONL

| Field | Value |
|-------|-------|
| Golden dump | `fixtures/v5/blocks-calls1/readstream.jsonl` |
| Profile | `fixtures/v5/blocks-calls1/nytprof.out` |
| NYTPROF | `trace=0:start=begin:calls=1:blocks=1` |
| Timing tags | `TIME_BLOCK` present; `TIME_LINE == 0` |
| `line_calls(1, 5)` / A4 `"1:5".calls` | **780** (statement line from real `TIME_BLOCK` events) |
| `block_line_calls(1, 4)` / A4b `"1:4".calls` | **810** (block start line from real `TIME_BLOCK` `block_line`) |
| `block_line_totals` | non-empty map; at least one entry with calls \> 0 |
| `main::leaf` / `main::mid` returns | **15** / **3** (same workload) |
| `is_stream_complete` | **true** (`time_block_events > 0`) |

These numbers must be **derived by iterating dump events**, not asserted from a constant alone without reading the file.

## Smoke

```sh
# Golden path only (no cargo / no oracle PERL5LIB)
prove -Iperl/lib perl/t/jsonl_data_default_calls1.t
prove -Iperl/lib perl/t/jsonl_data_blocks_calls1_line_totals.t
prove -Iperl/lib perl/t/jsonl_data_a4b_blocks_calls1.t
prove -Iperl/lib perl/t/jsonl_data_subdefs_default_calls1.t
prove -Iperl/lib perl/t/jsonl_data_source_default_calls1.t
prove -Iperl/lib perl/t/jsonl_data_meta_default_calls1.t
prove -Iperl/lib perl/t/jsonl_data_pid_default_calls1.t
prove -Iperl/lib perl/t/jsonl_data_stream_complete_default_calls1.t
prove -Iperl/lib perl/t/jsonl_data_discount_default_calls1.t
prove -Iperl/lib perl/t/jsonl_data_sub_entry.t

# Packaging smoke: default-calls1 leaf/mid + optional native dump
./scripts/packaging/perl_jsonl_data_smoke.sh

# Line totals smoke: blocks-calls1 780 + default leaf/mid (+ optional native dump)
./scripts/packaging/perl_line_totals_smoke.sh

# A4b block_line_totals: blocks-calls1 1:4=810 + A4 780 + default empty A4b
./scripts/packaging/perl_a4b_smoke.sh

# Sub defs + file identity: leaf 1/3–7, mid 1/8–12, basename workload.pl
./scripts/packaging/perl_subdefs_smoke.sh

# Source lines: source_line(1,5) exact dump text (+ optional native dump)
./scripts/packaging/perl_source_smoke.sh

# Metadata: attribute ticks_per_sec|basetime + option calls (+ optional native dump)
./scripts/packaging/perl_meta_smoke.sh

# PID lifecycle: pid_start_count / pid_end_count + matching pid 2975381 (+ optional native dump)
./scripts/packaging/perl_pid_smoke.sh

# Stream completeness: complete golden + incomplete craft from real dump lines
./scripts/packaging/perl_stream_complete_smoke.sh

# DISCOUNT A3 multiplicity: discount_events == stream recount (818 on default-calls1)
./scripts/packaging/perl_discount_smoke.sh

# SUB_ENTRY multiplicity: default-calls1 0 + calls2-default 27 == stream recount
./scripts/packaging/perl_sub_entry_smoke.sh

# Fail-fast roll-up (offline gate step 5)
./scripts/packaging/perl_jsonl_data_all_smoke.sh
```

Native dump generation (optional second path):

```sh
cargo run -q -p nytprof-cli -- dump fixtures/v5/default-calls1/nytprof.out > /tmp/native.jsonl
# then JsonlData->from_jsonl → leaf 15 / mid 3 / mid→leaf 15
#      sub_def leaf 1/3–7, mid 1/8–12; file_basename(1) → workload.pl
#      source_line(1,5) → "    $x++ for 1 .. 50;\n"
#      attribute(ticks_per_sec|basetime) and option(calls) defined
#      pid_start_count >= 1, pid_end_count >= 1, start pid == end pid (2975381)
#      is_stream_complete → true; time_line_events > 0
#      discount_events → 818 (A3; matches stream recount)
#      sub_entry_events → 0 (calls=1)
#      block_line_totals empty

cargo run -q -p nytprof-cli -- dump fixtures/v5/calls2-default/nytprof.out > /tmp/calls2.jsonl
# then JsonlData->sub_entry_events → 27 (matches stream recount)

cargo run -q -p nytprof-cli -- dump fixtures/v5/blocks-calls1/nytprof.out > /tmp/blocks.jsonl
# then JsonlData->line_calls(1,5) → 780
#      JsonlData->block_line_calls(1,4) → 810
#      is_stream_complete → true; time_block_events > 0
```

## Shipped engine entry (PERL-ENGINE-QUERY / PERL-ENGINE-QUERY-EXPAND / PERL-QUERY-PID-META / QUERY-JSON-MVP / QUERY-JSON-EXPAND)

Operator CLI wraps this module. Default `query` output is always-full MVP via `print_query_results` (returns/edges + `sub_def` + `source_line` + `line_calls` / `block_line_calls` samples when present + PID lifecycle + ATTRIBUTE/OPTION). With `--json` / `--format=json`, stdout is a single JSON object (`ok` / `subs` / `edges` / `leaf_returns` / `mid_returns` / `mid_leaf_edge` / `discount_events` / `sub_entry_events` / `is_stream_complete` / `incompleteness_reasons` / `time_line_events` / `pid_start_events` / `pid_end_events`):

```sh
perl -Iperl/lib perl/bin/nytprof-engine --engine=native query fixtures/v5/default-calls1/nytprof.out
perl -Iperl/lib perl/bin/nytprof-engine query --jsonl fixtures/v5/default-calls1/readstream.jsonl
perl -Iperl/lib perl/bin/nytprof-engine query --json --jsonl fixtures/v5/default-calls1/readstream.jsonl
perl -Iperl/lib perl/bin/nytprof-engine query --jsonl fixtures/v5/blocks-calls1/readstream.jsonl
./scripts/packaging/perl_engine_query_smoke.sh
./scripts/packaging/perl_engine_query_expand_smoke.sh
./scripts/packaging/perl_engine_query_pid_meta_smoke.sh
./scripts/packaging/perl_query_json_smoke.sh
./scripts/packaging/json_sub_entry_smoke.sh
```

See [`perl-engine-dispatch-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-engine-dispatch-mvp-v0.md) (`run_query` / `print_query_results` / action `query` / QUERY-JSON-MVP / QUERY-JSON-EXPAND / JSON-SUB-ENTRY-MVP / JSON-BLOCKS-MVP / JSON-SUBDEF-SOURCE-MVP / JSON-META-FILES-MVP). `query --json` emits **`sub_entry_events`** from `$data->sub_entry_events` (same field name as native `report --json` / `ProfileModel.sub_entry_events`; JsonlData still has method alias `sub_entry_count`), greppable A4/A4b **`line_calls_1_5`** / **`block_line_calls_1_4`**, greppable A9/A8 samples **`sub_def_leaf`** / **`sub_def_mid`** / **`source_line_1_5`** from `$data->sub_def` / `$data->source_line` (**null** when absent), and greppable ATTRIBUTE/OPTION/NEW_FID samples **`attribute_ticks_per_sec`** / **`option_calls`** / **`file_1`** from `$data->attribute` / `$data->option` / `$data->file` (**null** when absent).

## Non-goals

- Full binary-profile `Devel::NYTProf::Data` XS materializer
- Inclusive/exclusive tick query surface beyond raw event pass-through (returns are counts only in MVP)
- Putting `crates/` or candidate `perl/` on oracle `PERL5LIB`
- Replacing native report / HTML / CSV paths
- Full A5/A7 field sets (incl/excl/reci/max_rec_depth) — counts only for MVP queries

## Acceptance

Done for board **`PERL-DATA-JSONL`** when:

1. Module exists under `perl/lib/Devel/NYTProf/JsonlData.pm` and uses only core Perl + `JSON::PP` (via `JsonlReadStream`)
2. Smoke/test on default-calls1 shows **leaf=15**, **mid=3**, **mid→leaf=15** from counted dump events
3. At least one path uses committed golden JSONL; optional path uses live native `dump` / `from_cli`
4. Schema linked from first-slice board evidence

Done for board **`PERL-LINE-TOTALS`** when:

1. `_ingest` accumulates **`TIME_BLOCK`** into the same A4 `line_totals` keys as `TIME_LINE` (statement `fid`/`line`)
2. `line_calls($fid, $line)` (or equivalent) returns the call count
3. On `fixtures/v5/blocks-calls1`, **`line_calls(1, 5) == 780`** from real events (not hard-coded theater)
4. Test `perl/t/jsonl_data_blocks_calls1_line_totals.t` + smoke `./scripts/packaging/perl_line_totals_smoke.sh` (also keeps default-calls1 leaf/mid 15/3)
5. Board marked done **before COL-007**

Done for board **`PERL-A4B-JSONL`** when:

1. `_ingest` accumulates **`TIME_BLOCK`** into A4b `block_line_totals` keyed by **`(fid, block_line)`** at args\[1\] / args\[3\] (in addition to existing A4 statement `(fid, line)`)
2. API: `block_line_totals()` hash `"fid:block_line" => { calls, ticks }`; `block_line_calls($fid, $block_line)` convenience (missing → 0)
3. On `fixtures/v5/blocks-calls1`, from real events: **`block_line_totals` non-empty**, sample **`"1:4".calls == 810`**, and **`line_calls(1, 5) == 780`** still
4. Test `perl/t/jsonl_data_a4b_blocks_calls1.t` + smoke `./scripts/packaging/perl_a4b_smoke.sh` (golden + optional native dump; independent stream re-count preferred)
5. Board marked done **before COL-007**

Done for board **`PERL-SUBDEFS-JSONL`** when:

1. `_ingest` accumulates **`SUB_INFO`** into `sub_defs` (`name => { fid, first_line, last_line }`, last write wins)
2. `_ingest` accumulates **`NEW_FID`** into `files` (`fid => full path`); `file_basename($fid)` available
3. API: `sub_def($name)`, `sub_defs()`, `file($fid)`, `files()`, `file_basename($fid)`
4. On `fixtures/v5/default-calls1`, from real events: leaf **fid=1 first=3 last=7**, mid **1/8–12**, `file(1)` contains **`workload.pl`**
5. Test `perl/t/jsonl_data_subdefs_default_calls1.t` + smoke `./scripts/packaging/perl_subdefs_smoke.sh` (golden + optional native dump)
6. Board marked done **before COL-007**

Done for board **`PERL-SOURCE-JSONL`** when:

1. `_ingest` accumulates **`SRC_LINE`** into `source_lines` (`"fid:line" => text`, last write wins)
2. API: `source_line($fid, $line)`, `source_lines()` (hash copy)
3. On `fixtures/v5/default-calls1`, from real events: `source_line(1, 5)` is **`    $x++ for 1 .. 50;\n`** (contains `$x++` and `1 .. 50`)
4. Test `perl/t/jsonl_data_source_default_calls1.t` + smoke `./scripts/packaging/perl_source_smoke.sh` (golden + optional native dump); independent stream re-count preferred
5. Board marked done **before COL-007**

Done for board **`PERL-META-JSONL`** when:

1. `_ingest` accumulates **`ATTRIBUTE`** into `attributes` (`key => value`, last write wins) and **`OPTION`** into `options` (`key => value`, last write wins)
2. API: `attribute($key)`, `attributes()`, `option($key)`, `options()` (hash copies; missing key → undef)
3. On `fixtures/v5/default-calls1`, from real events: `attribute('ticks_per_sec')` and/or `attribute('basetime')` defined; `option('calls')` defined (values dump-derived only)
4. Test `perl/t/jsonl_data_meta_default_calls1.t` + smoke `./scripts/packaging/perl_meta_smoke.sh` (golden + optional native dump); independent stream re-count preferred
5. Board marked done **before COL-007**

Done for board **`PERL-PID-JSONL`** when:

1. `_ingest` accumulates **`PID_START`** into `pid_starts` / `pid_start_count` (`pid_start_events`) and **`PID_END`** into `pid_ends` / `pid_end_count` (`pid_end_events`)
2. API: `pid_starts()`, `pid_ends()` (arrayrefs of hashes with at least `pid`; optional `ppid` / `start_time` / `end_time` when present); `pid_start_count()` / `pid_end_count()`; optional `pids()` unique list
3. On `fixtures/v5/default-calls1`, from real events: **start_count ≥ 1**, **end_count ≥ 1**, start pid matches end pid (golden **2975381**); do not invent PIDs
4. Test `perl/t/jsonl_data_pid_default_calls1.t` + smoke `./scripts/packaging/perl_pid_smoke.sh` (golden + optional native dump); independent stream re-count preferred; wire into `perl_jsonl_data_all_smoke.sh`
5. Residual matrix marks PID lifecycle **ready**; board marked done **before COL-007**

Done for board **`PERL-STREAM-COMPLETE`** when:

1. `_ingest` tracks **`time_line_events`** / **`time_block_events`** (and uses existing `pid_start_events` / `pid_end_events`)
2. API: `is_stream_complete()` → bool; `stream_incompleteness_reasons()` → arrayref of reason strings; accessors `time_line_events` / `time_block_events`
3. Rules match [`COMPAT-010_INCOMPLETE_STREAM`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md): if `pid_start_events > 0` require `pid_end_events >= pid_start_events`; require `time_line_events + time_block_events > 0`
4. On `fixtures/v5/default-calls1` golden: **complete**, reasons **empty**; incomplete craft from **real** golden lines (filter to header tags only, no invented events): **incomplete**, reasons **non-empty**
5. Test `perl/t/jsonl_data_stream_complete_default_calls1.t` + smoke `./scripts/packaging/perl_stream_complete_smoke.sh`; wire into `perl_jsonl_data_all_smoke.sh`
6. Residual matrix marks stream completeness **ready**; board marked done **before COL-007**

Done for board **`PERL-DISCOUNT-JSONL`** when:

1. `_ingest` increments **`discount_events`** on each **`DISCOUNT`** tag (empty args; A3 multiplicity)
2. API: `discount_events()` / `discount_count()` (aliases; integer count; missing stream → **0**)
3. On `fixtures/v5/default-calls1`, from real events: count matches **independent stream re-count** of `DISCOUNT` tags; golden re-count is **818** (also `aggregates.oracle.json`)
4. Test `perl/t/jsonl_data_discount_default_calls1.t` + smoke `./scripts/packaging/perl_discount_smoke.sh` (golden + optional native dump); wire into `perl_jsonl_data_all_smoke.sh`
5. Residual matrix marks discount event accounting **ready** (multiplicity only — not exclusive-time policy freeze); board marked done **before COL-007**

Done for board **`PERL-SUB-ENTRY-JSONL`** when:

1. `_ingest` increments **`sub_entry_events`** on each **`SUB_ENTRY`** tag (`caller_fid`, `caller_line`; multiplicity only)
2. API: `sub_entry_events()` / `sub_entry_count()` (aliases; integer count; missing stream → **0**)
3. On `fixtures/v5/default-calls1` (`calls=1`): count **0** matches independent stream re-count of `SUB_ENTRY` tags
4. On `fixtures/v5/calls2-default` (`calls=2`): count **27** matches independent stream re-count of `SUB_ENTRY` tags
5. Test `perl/t/jsonl_data_sub_entry.t` + smoke `./scripts/packaging/perl_sub_entry_smoke.sh` (golden + optional native dump); wire into `perl_jsonl_data_all_smoke.sh`
6. Residual matrix marks SUB_ENTRY event accounting **ready** (multiplicity only — not full call-stack / arg freeze); board marked done **before COL-007**

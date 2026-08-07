# BASE-004 — Public Perl API surface inventory (Phase-0)

**Status:** inventory only (no XS/Data backend implementation)  
**Board ID:** `BASE-004-INV`  
**Oracle:** Devel::NYTProf 6.15 (`baseline/6.15/`, tag `v6.15`)  
**Primary sources:** `baseline/6.15/src/lib/Devel/NYTProf/*.pm` (prefer `src/` over `install/` / `blib/`)  
**Related:** `baseline/6.15/src/lib/Devel/NYTProf.pm`, `NYTProf.xs`, `FileHandle.xs`  
**Date:** 2026-08-07

## Scope and method

- Grep of `package` / `sub` in oracle `src/lib/Devel/NYTProf/*.pm` plus POD and XS bootstrap packages.
- **Disposition** per method or method group:
  - `mapped` — needed for native facade / known contract (decode, model, dump, thin Perl bridge)
  - `legacy-only` — keep on oracle/legacy path for now
  - `open` — needs more investigation before mapping
  - `internal` — underscore / clearly private
- This is **not** a promise of full Perl API materializer parity (explicitly out of first slice).
- Do **not** put `crates/` on oracle `PERL5LIB` when exercising these packages.

## Package index

| Package | Source | Role | Package disposition |
|---------|--------|------|---------------------|
| `Devel::NYTProf` | `lib/Devel/NYTProf.pm` | Collector entry (`-d:NYTProf`); thin wrapper | `legacy-only` (collector) / `open` for facade later |
| `Devel::NYTProf::Core` | `Core.pm` + XS | Loads XS; `NYTPROF` env parse; not for direct use | `legacy-only` (runtime) / `mapped` concept for options |
| `Devel::NYTProf::Data` | `Data.pm` + XS `load_profile_data_from_file` | Profile load + aggregate object model | **`mapped`** (core contract) |
| `Devel::NYTProf::ReadStream` | `ReadStream.pm` | Stream callback API over Data loader | **`mapped`** (dump/oracle equality) |
| `Devel::NYTProf::Reader` | `Reader.pm` | CSV/HTML-oriented report generator | `legacy-only` (native has own report paths) |
| `Devel::NYTProf::FileInfo` | `FileInfo.pm` | Per-fid object (array-backed) | **`mapped`** (accessors subset) |
| `Devel::NYTProf::SubInfo` | `SubInfo.pm` | Per-sub object (array-backed) | **`mapped`** (accessors subset) |
| `Devel::NYTProf::SubCallInfo` | `SubCallInfo.pm` | Per-call-edge timing fields | **`mapped`** |
| `Devel::NYTProf::FileHandle` | `FileHandle.pm` + `FileHandle.xs` | Profile file read/write handle | `open` (merge/writer) / `legacy-only` for full API |
| `Devel::NYTProf::Util` | `Util.pm` + XS `trace_level` | Formatting, path strip, MAD | `mapped` (fmt helpers) / `legacy-only` (HTML/path report glue) |
| `Devel::NYTProf::Constants` | `Constants.pm` + XS consts | `NYTP_*` constants + `const_bits2names` | `mapped` (index constants) / `open` full set |
| `Devel::NYTProf::Run` | `Run.pm` | Experimental `profile_this` harness | `legacy-only` |
| `Devel::NYTProf::Apache` | `Apache.pm` | mod_perl child init/exit hooks | `legacy-only` |
| `Devel::NYTProf::Test` | `Test.pm` | Hidden test helper (`example_sub`) | `internal` / test-only |

Static assets under `lib/Devel/NYTProf/js/` are report resources for `nytprofhtml`, not Perl API.

---

## Devel::NYTProf::Data

**Source:** `baseline/6.15/src/lib/Devel/NYTProf/Data.pm`  
**XS:** `Devel::NYTProf::Data::load_profile_data_from_file($file, $cb=undef)` in `NYTProf.xs`  
**Public intent:** load `nytprof.out`, aggregate, expose file/sub maps without breaking encapsulation.

| Method | Notes | Disposition |
|--------|-------|-------------|
| `new(\%opts)` | `filename` (default `nytprof.out`), `quiet`, `callback` (stream mode → returns undef), `skip_collapse_evals` | `mapped` |
| `attributes` | returns `{attribute}` hashref | `mapped` |
| `options` | returns `{option}` hashref | `mapped` |
| `subname_subinfo_map` | shallow copy of `{sub_subinfo}` | `mapped` |
| `package_subinfo_map($merge_subs, $nested_pkgs)` | package grouping / optional merge | `open` |
| `all_fileinfos` | non-nullified fids (drops fid 0) | `mapped` |
| `eval_fileinfos` | eval fids only | `mapped` |
| `noneval_fileinfos` | non-eval fids | `mapped` |
| `fileinfo_of($arg)` | fid, path, or FileInfo; carps on fail | `mapped` |
| `subinfo_of($subname)` | by fully-qualified name | `mapped` |
| `inc` | currently `@INC` (POD notes profile `inc` missing) | `open` |
| `get_profile_levels` | `{profile_modes}` | `mapped` |
| `get_fid_line_data($level)` | `fid_${level}_time` for `line`/`block`/`sub` | `mapped` |
| `subs_defined_in_file($fid)` | hash subname → SubInfo | `mapped` |
| `subs_defined_in_file_by_line(...)` | first-line → [SubInfo…] | `mapped` |
| `file_line_range_of_sub($sub)` | `($file,$fid,$first,$last,$fi)` | `mapped` |
| `resolve_fid($file)` | path → fid; suffix match; int passthrough | `mapped` |
| `collapse_evals_in($parent_fi)` | sibling-eval rollup | `legacy-only` (complex; optional skip via ctor) |
| `dump_profile_data(\%opts)` | human dump / test grepping | `legacy-only` |
| `normalize_variables($normalize_options?)` | zero timings for test compare | `legacy-only` (test) |
| `load_profile_data_from_file` (XS) | HV aggregate or callback path | `mapped` (core loader) |
| `_caches` / `_clear_caches` | internal cache slot | `internal` |
| `_disconnect_subinfo` | remove sub from map | `internal` |
| `_dump_elements` | dump helper | `internal` |
| `_zero_array_elem` | normalize helper | `internal` |
| `_filename_to_fid` | cache builder | `internal` |

**Method count (Data.pm `sub`):** 22 (incl. internals) + 1 XS loader.

---

## Devel::NYTProf::ReadStream

**Source:** `baseline/6.15/src/lib/Devel/NYTProf/ReadStream.pm`  
**Export:** `for_chunks` (`@EXPORT_OK`)

| Method / export | Notes | Disposition |
|-----------------|-------|-------------|
| `for_chunks(\&cb, %opts)` | thin wrapper: `Data->new({ %opts, callback => $cb })`; `$.` = chunk seq | `mapped` |

**POD chunk tags (document callback shapes):**  
`VERSION`, `COMMENT`, `ATTRIBUTE`, `OPTION`, `START_DEFLATE`, `PID_START`, `NEW_FID`, `TIME_BLOCK`, `TIME_LINE`, `DISCOUNT`, `SUB_INFO`, `SUB_CALLERS`, `SRC_LINE`, `PID_END`.

**Note:** POD is labeled for format “4.0” and may omit call-stream tags. Runtime/loader also emits `SUB_ENTRY` / `SUB_RETURN` (used by `nytprofcalls`; see BASE-002 / COMPAT-001 open items). Treat call tags as **mapped** for dump contracts; freeze args from fixtures.

**Method count:** 1 public export.

---

## Devel::NYTProf::Reader

**Source:** `baseline/6.15/src/lib/Devel/NYTProf/Reader.pm`  
**Consumers:** `nytprofcsv` (primary); historical HTML scaffolding (largely superseded by `nytprofhtml` script).

| Method | Notes | Disposition |
|--------|-------|-------------|
| `new($file, \%opts?)` | loads `Data->new`; default CSV templates | `legacy-only` |
| `set_param($param, $value)` | configure header/line templates / callbacks | `legacy-only` |
| `get_param($param, $code_args?)` | get; CODE params invoked | `legacy-only` |
| `file_has_been_modified($file)` | mtime vs `basetime` | `legacy-only` |
| `output_dir([$dir])` | get/set report directory | `legacy-only` |
| `report(\%opts?)` | generate per level (`sub`/`block`/`line`) | `legacy-only` |
| `current_level([$level])` | default `line` | `legacy-only` |
| `fname_for_fileinfo($fi, $level?)` | safe unique report basename | `legacy-only` |
| `url_for_file` / `href_for_file` | HTML link helpers | `legacy-only` |
| `url_for_sub` / `href_for_sub` | HTML sub anchors | `legacy-only` |
| `_output_additional` | write extra file | `internal` |
| `_generate_report` | bulk report writer | `internal` |

**Method count:** 14 (2 internal). Native `nytprof-cli csv|html` is the mapped report surface, not this class.

---

## Devel::NYTProf::FileInfo

**Source:** `baseline/6.15/src/lib/Devel/NYTProf/FileInfo.pm`  
**Backing:** array indices `NYTP_FIDi_*` (Constants/XS).

| Method | Notes | Disposition |
|--------|-------|-------------|
| `filename` | path string | `mapped` |
| `eval_fid` / `eval_line` / `eval_fi` | eval parentage | `mapped` |
| `fid` | file id | `mapped` |
| `size` / `mtime` | file meta | `mapped` |
| `profile` | backref to Data | `mapped` |
| `flags` | `NYTP_FIDf_*` bitfield | `mapped` |
| `is_eval` / `is_fake` / `is_file` / `is_pmc` | classification | `mapped` |
| `meta` / `cache` | lazy hash slots | `open` |
| `has_evals` / `sibling_evals` | eval children | `mapped` |
| `subs_defined` / `subs_defined_sorted` | SubInfo list | `mapped` |
| `sub_call_lines` | calls-from-lines structure | `mapped` |
| `evals_by_line` | eval map by line | `open` |
| `line_time_data` | statement timing arrays | `mapped` |
| `excl_time` | total exclusive for fid | `mapped` |
| `sum_of_stmts_count` / `sum_of_stmts_time` | aggregates | `open` |
| `outer` | chase eval outer fileinfo | `mapped` |
| `collapse_sibling_evals` | merge eval fids | `legacy-only` |
| `filename_without_inc` / `abs_filename` | path helpers | `mapped` |
| `has_savesrc` / `srclines_array` / `src_digest` | saved source | `mapped` |
| `normalize_for_test` | test normalize | `legacy-only` |
| `summary` / `dump` | debug | `legacy-only` |
| `_nullify` / `_remove_sub_defined` / `_add_new_sub_defined` / `_sum_of_line_time_data` | private | `internal` |

**Method count:** ~36 (4 internal).

---

## Devel::NYTProf::SubInfo

**Source:** `baseline/6.15/src/lib/Devel/NYTProf/SubInfo.pm`  
**Backing:** array indices `NYTP_SIi_*`.

| Method | Notes | Disposition |
|--------|-------|-------------|
| `fid` / `first_line` / `last_line` | definition range | `mapped` |
| `calls` | call count | `mapped` |
| `incl_time` / `excl_time` | inclusive/exclusive runtime | `mapped` |
| `subname` / `subname_without_package` / `package` | naming | `mapped` |
| `profile` | backref | `mapped` |
| `recur_max_depth` / `recur_incl_time` | recursion stats | `mapped` |
| `meta` / `cache` | lazy slots | `open` |
| `caller_fid_line_places` | caller places map | `mapped` |
| `called_by_subnames` | reverse name set | `mapped` |
| `is_xsub` / `is_opcode` / `is_anon` / `kind` | classification | `mapped` |
| `fileinfo` | resolving FileInfo | `mapped` |
| `clone` | shallow clone | `open` |
| `merge_in` | merge another SubInfo | `legacy-only` (eval collapse / package merge) |
| `caller_fids` | list of caller fids | `mapped` |
| `caller_count` | **deprecated** alias of place count | `legacy-only` |
| `caller_places` | place list | `mapped` |
| `normalize_for_test` | test normalize | `legacy-only` |
| `dump` | debug | `legacy-only` |
| `_min` / `_max` / `_alter_fileinfo` / `_alter_called_by_fileinfo` / `_merge_in_caller_info` / `_fmt_sc` | private | `internal` |

**Method count:** ~32 (6 internal). Aligns with compact model A9 `sub_defs` + call edges for a **subset** of fields.

---

## Devel::NYTProf::SubCallInfo

**Source:** `baseline/6.15/src/lib/Devel/NYTProf/SubCallInfo.pm`  
**Backing:** `NYTP_SCi_*`.

| Method | Notes | Disposition |
|--------|-------|-------------|
| `calls` | call count | `mapped` |
| `incl_time` / `excl_time` | times | `mapped` |
| `recur_max_depth` / `recur_incl_time` | recursion | `mapped` |

**Method count:** 5. No POD; small pure-Perl accessors.

---

## Devel::NYTProf::FileHandle

**Source:** `baseline/6.15/src/lib/Devel/NYTProf/FileHandle.pm` + `FileHandle.xs`  
**Bootstrap:** loads `boot_Devel__NYTProf__FileHandle` from shared object after Core.

| Method (Perl/XS) | Notes | Disposition |
|------------------|-------|-------------|
| `open($pathname, $mode)` | open profile file | `open` |
| `close` | close handle | `open` |
| `write_header` | magic/version | `open` |
| `write_comment` / `write_attribute` / `write_option` | header tags | `open` |
| `start_deflate_write_tag_comment` | compression switch | `open` |
| `write_process_start` / `write_process_end` | PID lifecycle | `open` |
| `write_new_fid` | NEW_FID | `open` |
| `write_time_block` / `write_time_line` | timing | `open` |
| `write_call_entry` / `write_call_return` | call stream | `open` |
| `write_sub_info` / `write_sub_callers` | end aggregates | `open` |
| `write_src_line` / `write_discount` | source / discount | `open` |

**Primary consumer:** `nytprofmerge`. Native v5 decoder exists; **writer/merge facade not mapped** in first slice. Disposition package-level: `open` (merge/writer path) with use remaining `legacy-only` until COL/writer work.

**Method count (XS MODULE list):** 16 public methods.

---

## Devel::NYTProf::Util

**Source:** `Util.pm`; XS `Devel::NYTProf::Util::trace_level` in `NYTProf.xs`.

| Function | Notes | Disposition |
|----------|-------|-------------|
| `fmt_float` / `fmt_time` / `fmt_incl_excl_time` | display formatting | `mapped` (report display) |
| `html_safe_filename` | report filenames | `legacy-only` (native HTML has own escaping) |
| `strip_prefix_from_paths` / `make_path_strip_editor` | path normalize | `mapped` for test/normalize paths |
| `get_alternation_regex` / `get_abs_paths_alternation_regex` | path regex builders | `legacy-only` / shared helper |
| `calculate_median_absolute_deviation` | Reader severity coloring | `legacy-only` |
| `trace_level` (XS) | profiler option IV | `internal` / debug |
| `_dumper` | Data::Dumper helper | `internal` |

**Export count:** 12 symbols in `@EXPORT_OK`.

---

## Devel::NYTProf::Constants

| Surface | Notes | Disposition |
|---------|-------|-------------|
| Exported `NYTP_*` constants (from XS symbol table) | FIDi/SIi/SCi indices, FIDf flags, tags, etc. | `mapped` for indices used by FileInfo/SubInfo/SubCallInfo |
| `const_bits2names($group, $bits)` | decode flag bits to names | `open` |

---

## Devel::NYTProf::Core

| Surface | Notes | Disposition |
|---------|-------|-------------|
| `XSLoader::load('Devel::NYTProf', $VERSION)` | boots collector + Data/Util/FileHandle XS | `legacy-only` (runtime) |
| `NYTPROF` env parsing → `DB::set_option` | options + `sigexit` / `posix_exit` | `legacy-only` collector; option names are **mapped concepts** for contracts |
| POD: subroutine profiler internals | documentation only | n/a |

No pure-Perl methods beyond load-time side effects.

---

## Devel::NYTProf::Run

| Method | Notes | Disposition |
|--------|-------|-------------|
| `perl_command_words(%opt)` | build `perl` argv | `legacy-only` |
| `profile_this(%opt)` | run code under `-d:NYTProf`, return Data | `legacy-only` |

Experimental; POD says subject to change. Not a native facade target.

---

## Devel::NYTProf::Apache

| Method | Notes | Disposition |
|--------|-------|-------------|
| `trace` | debug warn | `legacy-only` |
| `child_init` | `DB::enable_profile` unless start=no/end | `legacy-only` |
| `child_exit` | `DB::finish_profile` | `legacy-only` |
| `current_perl_id` | multiplicity helper (glob alias) | `legacy-only` |

mod_perl only; out of first-slice native path.

---

## Devel::NYTProf / DB (collector surface, for completeness)

Not under `NYTProf/*.pm` but part of public runtime:

| Surface | Notes | Disposition |
|---------|-------|-------------|
| `-d:NYTProf` / `use Devel::NYTProf` | enables collector | `legacy-only` (COL path later) |
| `DB::set_option` / `enable_profile` / `disable_profile` / `finish_profile` | XS control API | `legacy-only` / `open` for dual-engine control later |
| `DB::DB` / `_INIT` / `_END` | internal hooks | `internal` |

---

## Disposition summary (counts)

Approximate public/non-internal methods inventoried above:

| Package | Public-ish methods | Dominant disposition |
|---------|-------------------:|----------------------|
| Data | ~18 | mapped (core) |
| ReadStream | 1 | mapped |
| Reader | ~12 | legacy-only |
| FileInfo | ~32 | mapped subset |
| SubInfo | ~26 | mapped subset |
| SubCallInfo | 5 | mapped |
| FileHandle | 16 | open |
| Util | ~10 | mixed |
| Run | 2 | legacy-only |
| Apache | 4 | legacy-only |
| Constants | 1 + many consts | mapped / open |

---

## Open items

| ID | Item | Notes |
|----|------|-------|
| OI-BASE004-01 | Exact public contract for `Data->new` object graph | Which keys (`fid_fileinfo`, `sub_subinfo`, `fid_*_time`, …) are stable vs private; POD admits API may change. |
| OI-BASE004-02 | ReadStream callback complete tag set + arg shapes | Confirm `SUB_ENTRY`/`SUB_RETURN`/`VERSION`/`all_loaded` against XS loader + `t/22-readstream.t` / `nytprofcalls` (POD incomplete). |
| OI-BASE004-03 | Native Perl facade shape for Data/FileInfo/SubInfo | Whether future XS-less facade reimplements bless-array layout or new OO; first slice uses Rust model + engine dispatch only. |
| OI-BASE004-04 | FileHandle writer parity / merge path | `nytprofmerge` depends on FileHandle; no native merge yet. |
| OI-BASE004-05 | `package_subinfo_map` merge semantics | Nested packages + `merge_in` interactions under-tested in oracle suite comments. |
| OI-BASE004-06 | Constants export surface freeze | Full `NYTP_*` list from 6.15 XS vs minimal set for facades. |
| OI-BASE004-07 | Reader vs nytprofhtml ownership | Reader still used by nytprofcsv; HTML primarily script-driven — avoid double-mapping. |
| OI-BASE004-08 | Collector `DB::*` control API for dual engine | How (if ever) native engine exposes enable/disable/finish without loading oracle Core. |

## Explicit non-goals (this inventory)

- Implementing XS Data/ReadStream backends  
- Full materializer parity with blessed FileInfo/SubInfo  
- Putting `crates/` on oracle `PERL5LIB`

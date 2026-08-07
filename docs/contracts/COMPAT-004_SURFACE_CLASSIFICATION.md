# COMPAT-004 — Public / de-facto surface classification (provisional)

**Status:** provisional — **not** a full COMPAT-004 freeze  
**Task:** COMPAT-004  
**Board ID:** `COMPAT-004-CLASS`  
**Date:** 2026-08-07  
**Depends on:** BASE-004 (Perl API inventory), BASE-005 (CLI/report inventory)  
**Oracle:** Devel::NYTProf 6.15 (`baseline/6.15/`, tag `v6.15`)

---

## Scope and non-claims

This document classifies **BASE-004 Perl API surfaces** and **BASE-005 CLI tools** so modernization work does not accidentally drop under-documented but used behavior.

It does **not**:

- freeze full plan COMPAT-004 acceptance (every fixture/consumer mapped with owners + required tests);
- freeze Perl object/callback fidelity (COMPAT-007), CLI black-box contracts (COMPAT-008), or report parity (COMPAT-006);
- claim native materializer parity for blessed FileInfo/SubInfo or full Reader/html dialect;
- reclassify individual underscore methods beyond package-level notes (see inventories).

Full plan COMPAT-004 remains **proposed / in-progress** until open items close and downstream-consumer mapping (COMPAT-013) lands.

### Inventories (source of truth for methods/flags)

| Inventory | Path |
|-----------|------|
| Perl API (BASE-004) | [`baseline/inventories/perl-api-surface.md`](../../baseline/inventories/perl-api-surface.md) |
| CLI / report (BASE-005) | [`baseline/inventories/cli-report-surface.md`](../../baseline/inventories/cli-report-surface.md) |

Disposition values **mapped** / **open** / **legacy-only** / **internal** in those inventories are **toward the native modernization path**. This document adds a **support class** for compatibility policy.

---

## Classification vocabulary

### Support class (this document)

| Class | Meaning |
|-------|---------|
| **public** | Documented (POD / man / usage) and intended as stable for external users and scripts. Changes require compatibility review; native targets must preserve intent or provide an explicit dual path. |
| **de-facto-public** | Widely used or scripted (tools, modules, downstream loaders) but weakly contracted (thin/no POD, “may change”, incomplete arg docs). Treat as compatibility-supported until reclassified by ADR or later freeze. |
| **internal** | Underscore / clearly private / test-only. Not a compatibility surface; may change without user-facing freeze. |
| **legacy-only** | Keep on the **oracle / legacy engine path** for now; **not** a native implementation target in the first slice. May still be public to users of the legacy install. |

### Native disposition (from BASE-004/005; restated)

| Disposition | Meaning |
|-------------|---------|
| **mapped** | Native path exists or is the known first-slice / near-term contract (possibly partial MVP). |
| **open** | Needs investigation before committing native shape or support level. |
| **legacy-only** | Remain on oracle tools/packages; no native reimplementation yet. |
| **internal** | Private implementation detail (not a facade target). |

A surface can be **public** as a legacy CLI while **legacy-only** for native disposition (e.g. `nytprofmerge`). Conversely, a surface can be **public** + **mapped** when native already exposes a related path (e.g. `nytprofhtml` → `nytprof-cli html` MVP).

---

## Package / tool index (summary)

### Perl (BASE-004)

| Surface | Support class | Native disposition | Notes |
|---------|---------------|--------------------|-------|
| `Devel::NYTProf::Data` | **public** | **mapped** | Core load + aggregate object; POD; dump/model contract |
| `Devel::NYTProf::ReadStream` | **public** | **mapped** | `for_chunks`; oracle dump equality |
| `Devel::NYTProf::Reader` | **de-facto-public** | **legacy-only** | Documented report generator; primary consumer `nytprofcsv`; native uses own report paths |
| `Devel::NYTProf::FileInfo` | **public** | **mapped** (accessor subset) | Array-backed; public accessors; private `_nullify` etc. **internal** |
| `Devel::NYTProf::SubInfo` | **public** | **mapped** (accessor subset) | Aligns with model A9 subset; merge/normalize helpers legacy/internal |
| `Devel::NYTProf::SubCallInfo` | **de-facto-public** | **mapped** | No POD; five accessors used via call-edge structures |
| `Devel::NYTProf::FileHandle` | **de-facto-public** | **open** / residual **legacy-only** | XS writer; primary consumer `nytprofmerge`; not first-slice native |
| `Devel::NYTProf::Util` | **de-facto-public** (fmt helpers) / mixed | **mapped** (fmt/path) / **legacy-only** (HTML glue) | See inventory |
| `Devel::NYTProf::Constants` | **public** (exported consts) | **mapped** (indices used by facades) / **open** (full set) | |
| `Devel::NYTProf::Core` | **internal** (direct use) / **legacy-only** (runtime) | **legacy-only** | Loads XS; “not for direct use” |
| `Devel::NYTProf` / `DB::*` control | **public** (collector entry) / **internal** hooks | **legacy-only** / **open** dual-engine | Collector remains COL path |
| `Devel::NYTProf::Run` | **legacy-only** | **legacy-only** | Experimental harness |
| `Devel::NYTProf::Apache` | **legacy-only** | **legacy-only** | mod_perl only |
| `Devel::NYTProf::Test` | **internal** | **internal** | Test helper |

### CLI (BASE-005)

| Tool | Support class | Native disposition | Notes |
|------|---------------|--------------------|-------|
| `nytprofhtml` | **public** | **mapped** (partial HTML MVP) / residual **legacy-only** | Primary HTML site; flame/graphviz remain legacy |
| `nytprofcsv` | **public** (POD: deprecated) | **mapped** (native subs/edges CSV) / **legacy-only** (Reader line CSV) | Different native column contract |
| `nytprofcg` | **public** | **mapped** (partial callgrind) | Content contract, not byte-identical |
| `nytprofcalls` | **public** | **mapped** (partial `folded`) / **open** | Multi-file + `--calls` open |
| `nytprofmerge` | **public** | **legacy-only** / future **open** | Blocked on FileHandle writer |
| `flamegraph.pl` | **legacy-only** | **legacy-only** | Bundled helper for html flame path |

---

## Perl surfaces (detail)

### `Devel::NYTProf::Data` — **public** · disposition **mapped**

| | |
|--|--|
| **Why public** | Documented package; primary profile load + aggregate API; scripts and report tools depend on `new` / fileinfo / subinfo maps. |
| **Native** | Core contract for dump, model, and future thin facade; XS `load_profile_data_from_file` is the load heart. |
| **Keep public (mapped subset)** | `new`, `attributes`, `options`, `subname_subinfo_map`, `all_fileinfos` / `eval_fileinfos` / `noneval_fileinfos`, `fileinfo_of`, `subinfo_of`, `get_profile_levels`, `get_fid_line_data`, `subs_defined_in_file*`, `file_line_range_of_sub`, `resolve_fid`. |
| **Legacy-only methods** | `collapse_evals_in`, `dump_profile_data`, `normalize_variables` (test). |
| **Internal** | `_caches`, `_clear_caches`, `_disconnect_subinfo`, `_dump_elements`, `_zero_array_elem`, `_filename_to_fid`. |
| **Open methods** | `package_subinfo_map`, `inc` (see OI-BASE004-01 / OI-C004-02). |

**Open items:** OI-C004-01, OI-C004-02 (and inventory OI-BASE004-01, OI-BASE004-05).

---

### `Devel::NYTProf::ReadStream` — **public** · disposition **mapped**

| | |
|--|--|
| **Why public** | Documented stream callback API; oracle dump / dual-equality spine (`for_chunks`). |
| **Native** | Mapped for dump contracts and COMPAT-001 logical events; callback tag set must stay complete for fixtures. |
| **Public export** | `for_chunks(\&cb, %opts)`. |
| **Caveat** | POD incomplete on call-stream tags; runtime emits `SUB_ENTRY` / `SUB_RETURN` (COMPAT-001 / OI-BASE004-02). |

**Open items:** OI-C004-03 (callback arg freeze for call tags).

---

### `Devel::NYTProf::Reader` — **de-facto-public** · disposition **legacy-only**

| | |
|--|--|
| **Why de-facto-public** | Documented class with template/param API; primary consumer is `nytprofcsv`. Not the primary long-term report facade (native `nytprof-cli` owns report). POD-level stability is weaker than Data/ReadStream for modernization. |
| **Native** | **legacy-only** — do not reimplement Reader as native; map report value via `nytprof-cli csv|html`. |
| **Public-ish methods** | `new`, `set_param` / `get_param`, `report`, `output_dir`, `current_level`, URL helpers, etc. |
| **Internal** | `_output_additional`, `_generate_report`. |

**Open items:** OI-C004-04 (Reader vs native CSV ownership — with OI-BASE004-07 / OI-BASE005-02).

---

### `Devel::NYTProf::FileInfo` — **public** · disposition **mapped** (subset)

| | |
|--|--|
| **Why public** | Documented per-fid object; report/tools navigate files via these accessors. |
| **Native** | Accessor subset mapped toward compact model (files, line totals, source, eval parentage). Full bless-array layout fidelity is COMPAT-007 / PERL work — **not** first-slice. |
| **Mapped examples** | `filename`, `fid`, `eval_*`, `flags` / `is_*`, `line_time_data`, `excl_time`, `subs_defined*`, `sub_call_lines`, `has_savesrc` / `srclines_array`, path helpers. |
| **Open** | `meta` / `cache`, `evals_by_line`, `sum_of_stmts_*`. |
| **Legacy-only** | `collapse_sibling_evals`, `normalize_for_test`, `summary` / `dump`. |
| **Internal** | `_nullify`, `_remove_sub_defined`, `_add_new_sub_defined`, `_sum_of_line_time_data`. |

**Open items:** OI-C004-05 (which FileInfo methods are hard public vs de-facto for facade).

---

### `Devel::NYTProf::SubInfo` — **public** · disposition **mapped** (subset)

| | |
|--|--|
| **Why public** | Documented per-sub object; name/time/caller places used by reports and exports. |
| **Native** | Subset aligns with A9 `sub_defs` + call-edge aggregates; not full AV identity. |
| **Mapped examples** | `fid` / lines, `calls`, `incl_time` / `excl_time`, naming, recursion stats, `caller_*`, `is_xsub` / `is_opcode` / `is_anon`, `fileinfo`. |
| **Open** | `meta` / `cache`, `clone`. |
| **Legacy-only** | `merge_in`, deprecated `caller_count`, normalize/dump helpers. |
| **Internal** | `_min` / `_max` / `_alter_*` / `_merge_in_caller_info` / `_fmt_sc`. |

**Open items:** OI-C004-05 (shared with FileInfo facade scope).

---

### `Devel::NYTProf::SubCallInfo` — **de-facto-public** · disposition **mapped**

| | |
|--|--|
| **Why de-facto-public** | No POD; pure-Perl accessors on call-edge structures used by tools that walk caller places. Not advertised as a standalone user API. |
| **Native** | Field set (`calls`, incl/excl, recursion) is mapped for edge aggregates; class shape freeze deferred to COMPAT-007. |
| **Methods** | `calls`, `incl_time`, `excl_time`, `recur_max_depth`, `recur_incl_time`. |

**Open items:** OI-C004-06 (whether to promote to **public** when facade ships).

---

### `Devel::NYTProf::FileHandle` — **de-facto-public** · disposition **open** (use **legacy-only** until writer work)

| | |
|--|--|
| **Why de-facto-public** | XS writer/read handle; primary consumer `nytprofmerge`; inventory question whether users instantiate directly. |
| **Native** | Writer/merge **not** first-slice; remain on oracle. Future native merge reopens classification. |
| **Methods** | `open`/`close`, header/tag writers (`write_*`), deflate switch — all **open** for native mapping. |

**Open items:** OI-C004-07 (direct-use survey + support class freeze; ties OI-BASE004-04 / OI-BASE005-05).

---

### Related packages (brief)

| Package | Class | Native disposition | Note |
|---------|-------|--------------------|------|
| `Util` | de-facto-public fmt helpers | mapped / legacy-only mix | HTML path strip helpers may stay legacy |
| `Constants` | public exports | mapped subset | Full `NYTP_*` freeze open (OI-BASE004-06) |
| `Core` | internal for direct use | legacy-only runtime | Option **names** are mapped concepts |
| `Run` / `Apache` | legacy-only | legacy-only | Out of first-slice native |
| `Test` | internal | internal | Test-only |
| `Devel::NYTProf` / `DB::*` | public entry; internal hooks | legacy-only / open control API | Collector dual-engine later (OI-BASE004-08) |

---

## CLI surfaces (detail)

### `nytprofhtml` — **public** · disposition **mapped** (partial) + residual **legacy-only**

| Aspect | Class | Native disposition |
|--------|-------|--------------------|
| Tool existence + core flags (`-f`/`-o`/`-d`/…) | public | mapped intent via `nytprof-cli html` |
| Multi-file HTML site / summary | public | **mapped** (MVP schemas) |
| Flame graph / `flamegraph.pl` integration | public usage of helper | **legacy-only** |
| Graphviz / block-sub pages | public in non-minimal | **legacy-only** |
| `--mergeevals` / eval UI | public flag | **open** |

**Evidence:** `docs/schemas/html-report-mvp-v0.md`, `html-multifile-mvp-v0.md`, `html-per-file-mvp-v0.md`.

**Open items:** OI-C004-08 (which html flags/artifacts are hard public vs acceptable MVP gap).

---

### `nytprofcsv` — **public** (deprecated in POD) · disposition **mapped** (partial) + **legacy-only** (Reader dialect)

| Aspect | Class | Native disposition |
|--------|-------|--------------------|
| Tool binary + `-f`/`-o`/`-d` | public | dual-path: native `csv` or legacy |
| Subs/edges CSV (native columns) | public operator path | **mapped** (`render_subs_csv` / `render_edges_csv`) |
| Reader line-level CSV dialect | de-facto-public for existing scripts | **legacy-only** |
| `--delim` / `--annotated` | de-facto-public (undocumented in thin usage) | **open** |

**Open items:** OI-C004-04, OI-C004-09 (undocumented flag freeze with OI-BASE005-06).

---

### `nytprofcg` — **public** · disposition **mapped** (partial)

| Aspect | Class | Native disposition |
|--------|-------|--------------------|
| Callgrind-style export | public | **mapped** (`nytprof-cli callgrind` / `cg`) |
| Full KCacheGrind byte/tool acceptance | de-facto expectation | **open** |

**Evidence:** `docs/schemas/export-formats-mvp-v0.md`, `docs/schemas/export-semantic-parity-mvp-v0.md` (board `EXPORT-SEMANTIC-PARITY`).

**Open items:** OI-C004-10 (callgrind structural equality vs content contract).

---

### `nytprofcalls` — **public** · disposition **mapped** (partial `folded`) / **open**

| Aspect | Class | Native disposition |
|--------|-------|--------------------|
| Folded / call-path text from stream | public | **mapped** partial (`nytprof-cli folded`) |
| Multi-file inputs | public | **open** |
| `--calls` / `--stable` / `--debug` | public or de-facto (usage incomplete) | **open** |

**Open items:** OI-C004-11 (promote residual modes legacy-only vs map).

---

### `nytprofmerge` — **public** · disposition **legacy-only** (future **open**)

| Aspect | Class | Native disposition |
|--------|-------|--------------------|
| Entire tool | public | **legacy-only** until FileHandle/writer plan |
| Future native merge | — | **open** (blocked OI-BASE004-04) |

**Open items:** OI-C004-07.

---

### `flamegraph.pl` — **legacy-only** · disposition **legacy-only**

Bundled third-party helper; not a native target. Invoked only via legacy html flame path.

---

## Cross-walk: class × native disposition (must-cover set)

| Surface | Support class | Native disposition |
|---------|---------------|--------------------|
| Data | public | mapped |
| ReadStream | public | mapped |
| Reader | de-facto-public | legacy-only |
| FileInfo | public | mapped (subset) |
| SubInfo | public | mapped (subset) |
| SubCallInfo | de-facto-public | mapped |
| FileHandle | de-facto-public | open / legacy-only use |
| nytprofhtml | public | mapped (partial) / residual legacy-only |
| nytprofcsv | public | mapped (partial) / Reader dialect legacy-only |
| nytprofcg | public | mapped (partial) |
| nytprofcalls | public | mapped (partial) / open residual |
| nytprofmerge | public | legacy-only |

---

## Open items (OI-C004-*)

| ID | Item | Notes / blockers |
|----|------|------------------|
| OI-C004-01 | Data object graph: which HV keys are public vs private | Feeds COMPAT-007; OI-BASE004-01 |
| OI-C004-02 | `package_subinfo_map` / `inc` support class | open methods; merge semantics OI-BASE004-05 |
| OI-C004-03 | ReadStream complete callback tag + arg freeze | `SUB_ENTRY`/`SUB_RETURN`; OI-BASE004-02; COMPAT-001 |
| OI-C004-04 | Reader CSV dialect: keep legacy forever vs native line-CSV | OI-BASE004-07, OI-BASE005-02 |
| OI-C004-05 | FileInfo/SubInfo public method subset for facade | Which accessors are hard public vs de-facto; COMPAT-007 |
| OI-C004-06 | SubCallInfo promote to **public** or keep de-facto | Depends on edge materializer |
| OI-C004-07 | FileHandle direct-use survey + merge native plan | OI-BASE004-04, OI-BASE005-05 |
| OI-C004-08 | nytprofhtml artifact/flag public matrix vs HTML MVP gaps | OI-BASE005-01; COMPAT-006 |
| OI-C004-09 | Undocumented CLI flags freeze (`csv`/`calls`/`html`) | OI-BASE005-06; COMPAT-008 |
| OI-C004-10 | nytprofcg vs native callgrind equality level | OI-BASE005-03 |
| OI-C004-11 | nytprofcalls residual modes (multi-file, `--calls`) | OI-BASE005-04 |
| OI-C004-12 | Downstream-consumer corpus mapping to classes | Full COMPAT-004 acceptance + COMPAT-013 |
| OI-C004-13 | Engine-dispatch coverage of classified tools | OI-BASE005-07; today not full html/cg/calls/merge |

---

## Explicit non-goals (this provisional landing)

- Full support matrix with per-surface owners and required test IDs (plan acceptance).  
- COMPAT-007 object shape / COMPAT-008 exit-code freeze / COMPAT-006 report DOM parity.  
- Implementing FileHandle writer, merge, or full nytprofhtml.  
- Changing runtime behavior of oracle packages.

---

## Cross-links

| Artifact | Path |
|----------|------|
| This classification | `docs/contracts/COMPAT-004_SURFACE_CLASSIFICATION.md` |
| COMPAT-001 logical events | `docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md` |
| COMPAT-002 volatiles | `docs/contracts/COMPAT-002_VOLATILE_NORMALIZATION.md` |
| COMPAT-003 precision | `docs/contracts/COMPAT-003_PRECISION_NUMERIC_POLICY.md` |
| Perl inventory BASE-004 | `baseline/inventories/perl-api-surface.md` |
| CLI inventory BASE-005 | `baseline/inventories/cli-report-surface.md` |
| Task definition | `docs/plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md` § COMPAT-004 |
| Board | `docs/FIRST_SLICE_BOARD.md` |

---

## Provisional acceptance (Phase-0 board)

This provisional COMPAT-004 landing is **done for board `COMPAT-004-CLASS`** when:

1. Must-cover Perl surfaces (Data, ReadStream, Reader, FileInfo, SubInfo; notes for SubCallInfo/FileHandle) each have a support class + native disposition.  
2. Must-cover CLI tools (`nytprofhtml`, `nytprofcsv`, and at least one of `nytprofcg` / `nytprofmerge` / `nytprofcalls`) are classified (this document covers all listed).  
3. Open items use `OI-C004-*` IDs and do not silently close inventory gaps.  
4. Status remains **provisional** — not a full plan COMPAT-004 freeze.

Full task acceptance (every compatibility fixture and downstream consumer maps to a classified surface with owners/tests) remains **open** (OI-C004-12 and plan COMPAT-004).

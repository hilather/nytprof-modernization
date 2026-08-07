# BASE-005 — CLI / report surface inventory (Phase-0)

**Status:** inventory only (no full report parity implementation)  
**Board ID:** `BASE-005-INV`  
**Oracle:** Devel::NYTProf 6.15 (`baseline/6.15/`, tag `v6.15`)  
**Primary sources:** `baseline/6.15/src/bin/*` (also installed at `baseline/6.15/install/bin/`)  
**Date:** 2026-08-07

## Scope and method

- Inventory operator tools shipped with 6.15: purpose, key flags (from script `GetOptions` / usage / POD), primary outputs, disposition.
- **Disposition:**
  - `mapped` — native path exists or is the known first-slice contract (possibly partial)
  - `legacy-only` — remain on oracle tools for now
  - `open` — needs investigation / no native path yet
- Native CLI today: `nytprof-cli` / `nytprof-dump` (see `docs/schemas/*-mvp-v0.md`). This inventory maps **legacy** surfaces to that work; it does not claim byte-identical parity.
- Do **not** put `crates/` on oracle `PERL5LIB` when running these tools.

## Tool index

| Tool | Path (src) | Install bin | Disposition |
|------|------------|-------------|-------------|
| `nytprofhtml` | `src/bin/nytprofhtml` | yes | `mapped` (partial native HTML) / residual `legacy-only` |
| `nytprofcsv` | `src/bin/nytprofcsv` | yes | `mapped` (partial native CSV) / residual `legacy-only` |
| `nytprofcg` | `src/bin/nytprofcg` | yes | `mapped` (partial callgrind export) |
| `nytprofcalls` | `src/bin/nytprofcalls` | yes | `open` / partial native `folded` |
| `nytprofmerge` | `src/bin/nytprofmerge` | yes | `legacy-only` / `open` |
| `flamegraph.pl` | `src/bin/flamegraph.pl` | yes | `legacy-only` (bundled helper) |

Also present in man pages under `install/man/man1/` for the five `nytprof*` tools (not `flamegraph.pl`).

---

## nytprofhtml

**Purpose:** Primary human-facing HTML report site for a profile (index, per-file pages, optional flame graph, optional graphviz dots).

**Depends on:** `Devel::NYTProf::Data`, `Util`, `Core`; optionally runs bundled `flamegraph.pl`.

### Key flags

| Flag | Default | Role |
|------|---------|------|
| `-f`, `--file` | `nytprof.out` | input profile |
| `-o`, `--out` | `nytprof` | output directory |
| `-d`, `--delete` | off | delete old report files in out dir |
| `--open` | off | open report in browser |
| `-l`, `--lib` | — | unshift `@INC` |
| `--no-flame` / `--flame` | flame on | enable/disable flame graph + call stacks |
| `--flamewidth` | `1200` | flame SVG width |
| `-m`, `--minimal` | off | skip graphviz `.dot` and block/sub-level reports; forces blocks off |
| `--no-mergeevals` / `--mergeevals` | merge on | string-eval collapse |
| `--profself` | — | profile the reporter itself (BEGIN handling) |
| `--debug` | off | debug |
| `-h`, `--help` | — | usage |

### Primary outputs

- Directory of HTML (index + per-source/file reports), CSS/JS assets from module share, optional SVG flame graph, optional `.dot` files (non-minimal).

### Disposition

| Aspect | Disposition | Notes |
|--------|-------------|-------|
| Multi-file HTML site | `mapped` | native `nytprof-cli html --out-dir` (MVP; not full nytprofhtml) |
| Single-file summary HTML | `mapped` | native `html -o` |
| Flame graph / call stacks | `legacy-only` | uses `flamegraph.pl`; no native flame yet |
| Graphviz / block-sub pages | `legacy-only` | beyond HTML MVP |
| Eval merge UI flag | `open` | native model may collapse differently |

**Native evidence:** `docs/schemas/html-report-mvp-v0.md`, `html-multifile-mvp-v0.md`, `html-per-file-mvp-v0.md`; board rows HTML-*.

**Artifact residual inventory (oracle site classes vs native):**  
[`docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md) (`REPORT-HTML-RESIDUAL-INV`); lister `tools/oracle/list_html_artifacts.sh`.

---

## nytprofcsv

**Purpose:** (Deprecated in POD) CSV reports via `Devel::NYTProf::Reader` — one CSV-ish file per source with time/calls/time-per-call/code.

**Depends on:** `Devel::NYTProf::Reader`.

### Key flags

| Flag | Default | Role |
|------|---------|------|
| `-f`, `--file` | `nytprof.out` | input profile |
| `-o`, `--out` | `nytprof` | output **directory** |
| `-d`, `--delete` | off | wipe out dir first |
| `--delim` | `comma` (`tab` accepted) | field delimiter (GetOptions; thin usage text omits) |
| `-a`, `--annotated` | off | prefix `srcline` column (GetOptions; usage text omits) |
| `-h`, `--help` | — | usage |

### Primary outputs

- Directory of per-file CSV reports (format controlled by Reader templates). Sample POD format: `time,calls,time/call,code`.

### Disposition

| Aspect | Disposition | Notes |
|--------|-------------|-------|
| Subs / edges CSV export | `mapped` | native `nytprof-cli csv` (`render_subs_csv` / `render_edges_csv`) — **not** legacy per-line Reader layout |
| Full Reader line-level CSV parity | `legacy-only` | keep oracle `nytprofcsv` for exact dialect |
| `--delim` / `--annotated` | `open` | not on native CLI |

**Native evidence:** board CSV-REPORT; packaging legacy smoke may optionally invoke `nytprofcsv`.

---

## nytprofcg

**Purpose:** Convert profile to Callgrind / KCacheGrind-style text for external viewers.

**Depends on:** `Devel::NYTProf::Data` (sub map + caller places).

### Key flags

| Flag | Default | Role |
|------|---------|------|
| `-f`, `--file` | `nytprof.out` | input |
| `-o`, `--out` | `nytprof.callgrind` | output **file** |
| `-h`, `--help` | — | usage |

### Primary outputs

- Single Callgrind-ish text file: `events: Ticks`, `fl=` / `fn=` / `cfl=` / `cfn=` / `calls=` lines; times scaled to microseconds (`* 1_000_000`).

### Disposition

| Aspect | Disposition | Notes |
|--------|-------------|-------|
| Callgrind-style export | `mapped` | native `nytprof-cli callgrind` / `cg` — content contract, not byte-identical |
| Full Valgrind tool acceptance | `open` | native MVP checks names/counts only |

**Native evidence:** `docs/schemas/export-formats-mvp-v0.md`, `docs/schemas/export-semantic-parity-mvp-v0.md`; boards EXPORT-CALLGRIND-FOLDED, EXPORT-SEMANTIC-PARITY.

---

## nytprofcalls

**Purpose:** Build call-path / stack aggregates from **stream** load (`SUB_RETURN` events), emit folded-style path timings (or call counts).

**Depends on:** `Devel::NYTProf::Core`, `Data` callback loader (not full aggregate object when streaming).

### Key flags

| Flag | Default | Role |
|------|---------|------|
| positional args | required | one or more `nytprof.out` files |
| `-v`, `--verbose` | off | verbosity |
| `--calls` | off | sum calls instead of time |
| `-d`, `--debug` | off | debug (+ verbose) |
| `--stable` | off | stable ordering for tests |
| `-h`, `--help` | — | usage |

Usage text documents only help/verbose; `--calls` / `--debug` / `--stable` exist in `GetOptions`.

### Primary outputs

- Text call-path lines (tree extracted from return stream) suitable for flamegraph-style tools.

### Disposition

| Aspect | Disposition | Notes |
|--------|-------------|-------|
| Folded stacks export | `mapped` (partial) | native `nytprof-cli folded` — not full nytprofcalls dialect |
| Multi-file merge of streams | `open` | tool accepts multiple files |
| `--calls` mode | `open` | native folded uses time-oriented stacks |

---

## nytprofmerge

**Purpose:** Merge multiple profile files into one `nytprof-merged.out` via `FileHandle` writers and fid remapping.

**Depends on:** `Core`, `FileHandle`, `Data`.

### Key flags

| Flag | Default | Role |
|------|---------|------|
| positional args | required | input profiles |
| `-o`, `--out` | `nytprof-merged.out` | merged output file |
| `-v`, `--verbose` | off | verbosity |
| `-h`, `--help` | — | usage |

### Primary outputs

- Single binary profile (`nytprof.out` format) with remapped fids/subs.

### Disposition

| Aspect | Disposition | Notes |
|--------|-------------|-------|
| Entire tool | `legacy-only` | no native merge |
| Future native merge | `open` | blocked on FileHandle writer parity (OI-BASE004-04) |

---

## flamegraph.pl

**Purpose:** Bundled Brendan Gregg–style flame graph SVG generator (generic; not NYTProf-specific). Invoked by `nytprofhtml` when flame is enabled.

### Key flags (usage)

| Flag | Role |
|------|------|
| `--title`, `--width`, `--height`, `--minwidth` | layout |
| `--fonttype`, `--fontsize` | fonts |
| `--countname`, `--nametype` | labels |
| `--colors`, `--hash`, `--cp` | palette |
| `--reverse`, `--inverted`, `--negate` | graph mode |
| `--help` | usage |
| (also in GetOptions) `--encoding`, `--fontwidth`, `--nameattr`, `--total`, `--factor`, … | advanced |

### Primary outputs

- SVG on stdout from folded stack input lines.

### Disposition

| Aspect | Disposition | Notes |
|--------|-------------|-------|
| Tool | `legacy-only` | third-party helper; keep with oracle HTML path |
| Native flame | `open` | not first-slice |

---

## Cross-walk: legacy tool → native `nytprof-cli`

| Legacy | Native (first-slice) | Parity level |
|--------|----------------------|--------------|
| `nytprofhtml` | `html` [`-o` \| `--out-dir`] | MVP site/summary; not full UI |
| `nytprofcsv` | `csv` | different columns (subs/edges) |
| `nytprofcg` | `callgrind` / `cg` | simplified Callgrind-ish |
| `nytprofcalls` | `folded` | folded stacks MVP |
| `nytprofmerge` | — | none |
| `flamegraph.pl` | — | none |
| (oracle dump) | `dump` / verify path | ReadStream JSONL via tools/oracle + native dump |
| — | `report` / `summary` / `verify` / `inspect` | native-only operator paths |

Perl dispatcher: `perl/bin/nytprof-engine` (`docs/schemas/perl-engine-dispatch-mvp-v0.md`) routes `report`/`csv`/`html`/… to native or legacy dump smoke — **not** a full reimplementation of each binary.

---

## Open items

| ID | Item | Notes |
|----|------|-------|
| OI-BASE005-01 | nytprofhtml full page inventory | Catalog every generated HTML/asset type vs native multi-file schema gaps (dots, flame, severity coloring). |
| OI-BASE005-02 | nytprofcsv dialect parity decision | Keep legacy forever for Reader CSV, or define native line-level CSV contract. |
| OI-BASE005-03 | nytprofcg vs native callgrind structural equality | Whether KCacheGrind must accept native output; freeze ticks unit and `fl`/`fn` rules. |
| OI-BASE005-04 | nytprofcalls multi-input + `--calls` | Map to native folded or keep legacy-only. |
| OI-BASE005-05 | nytprofmerge native plan | Fid remapping + FileHandle writer; depends OI-BASE004-04. |
| OI-BASE005-06 | Undocumented flags freeze | csv `--delim`/`--annotated`; calls `--stable`/`--debug`; html `--profself` — document or drop in any wrapper. |
| OI-BASE005-07 | Engine-dispatch coverage of tools | Today legacy engine mostly ReadStream dump + optional csv; not full html/cg/calls/merge. |

## Explicit non-goals (this inventory)

- Implementing full nytprofhtml / merge / flame parity  
- Byte-identical Callgrind or CSV  
- Putting `crates/` on oracle `PERL5LIB`

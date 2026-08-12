# R1-preview operator runbook (offline R0 / R1-preview)

**Status:** operator-facing consolidation of the offline R0 / R1-preview stack  
**Board ID:** `R1-PREVIEW-RUNBOOK` (honesty sync: `R1-HONESTY-SYNC`)  
**Date:** 2026-08-07  
**Gate:** done **before COL-007** (C v6 writer)

---

## 1. What this is

This runbook is the single operator entry for **offline developer preview** of the first-slice modernization stack:

| Level | Meaning here |
|-------|----------------|
| **R0** | Developer preview (experimental / opt-in tools). No product default change. See [PROGRAM_CHARTER.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md). |
| **R1-preview** | Opt-in **native v5 read/report** path + pure-Perl dump-JSONL query facade. Not a full charter **R1** product claim. |

**This is not:**

- a full **R1** product release or CPAN readiness statement;
- full production **FFI** (beyond open/query/close MVP) or full **COMPAT-007 / pure-XS** Data materialization (thin product Data/ReadStream MVP may be present under **PERL-XS-DATA-READSTREAM-MVP**);
- full oracle **`nytprofhtml` DOM** / CSS / tablesorter / flame / Graphviz parity;
- **v6** wire freeze or **COL-007** C v6 writer;
- performance certification or public perf claims (R1-scoped P3/P4 methodology only — [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md); residual waived for public numbers);
- permission to flip product defaults (`engine=auto` as R3 product default, format defaults — charter R3/R4).

**Ready vs residual freeze:**  
[R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md)

**Isolation rule (always):** never put `crates/`, candidate `perl/`, or `collector/` (incl. `collector/install/`) on oracle `PERL5LIB`. Oracle tools use `baseline/6.15/install` only.

---

## 2. One-command health check

From repo root:

```sh
./scripts/ci/offline_gate.sh
# after perl Makefile.PL:
# make offline-gate
```

Script: [scripts/ci/offline_gate.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/offline_gate.sh)  
Policy: [BUILD_SUPPORT_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md)

### Steps (fail-fast; exit non-zero on first failure)

| Step | What | Cargo / native |
|------|------|----------------|
| 1 | `cargo test -p nytprof-format-v5 -p nytprof-format-v6 -p nytprof-model -p nytprof-report -p nytprof-cli -p nytprof-ffi` | **Honest skip** if `cargo` / `crates/` absent |
| 2 | `./tools/oracle/selftest_harness.sh` | **Required** (dump parity, fail-closed, incomplete-stream, decode-fuzz, normalize, …) |
| 3 | `./scripts/packaging/dual_path_smoke.sh` | **Primary packaging** — legacy always; native install when cargo present |
| 4 | `./scripts/packaging/engine_auto_fallback_smoke.sh` | **Required** (Perl `engine=auto` prefer-native / fall-back-legacy) |
| 5 | `./scripts/packaging/perl_jsonl_data_all_smoke.sh` | **Required** (pure-Perl JsonlData roll-up incl. DISCOUNT A3 + **SUB_ENTRY** multiplicity; golden JSONL; no cargo) |
| 5b | `./scripts/packaging/perl_xs_data_readstream_smoke.sh` | **Required** (**PERL-XS-DATA-READSTREAM-MVP** / PR-A06: product `Data` / `ReadStream` over golden JSONL; binary via native dump when CLI present; thin materializer only — not COMPAT-007 / pure-XS wire decode) |
| 6 | `./scripts/packaging/perl_query_json_smoke.sh` (+ JSON surface smokes 6b–6g) | **Required** (**CI-QUERY-JSON-GATE** / QUERY-JSON-MVP / QUERY-JSON-EXPAND; golden `--jsonl`; no cargo). Also **json_sub_entry** / **json_blocks** / **json_subdef_source** / **json_meta_files** / **json_time_block** / **json_file_basename** / **json_event_counts** / **json_total_basetime** (**JSON-FILE-BASENAME-MVP** / **JSON-EVENT-COUNTS-MVP** / **JSON-TOTAL-EVENTS-MVP** / **JSON-ATTR-BASETIME-MVP**: basename **workload.pl**, `total_events` **2474**, `attribute_basetime`) |
| 7 | `./scripts/packaging/native_agg_json_smoke.sh` (+ stream + incomplete) | **Optional when native** (**NATIVE-AGG-JSON** **15/3/15**; **JSON-NATIVE-STREAM-MVP**; **JSON-REPORT-INCOMPLETE-FAILCLOSED** via `json_report_incomplete_smoke.sh`) |
| 8 | `./scripts/packaging/native_query_json_cross_smoke.sh` | **Optional when native** (**NATIVE-QUERY-JSON-CROSS** / **CROSS-EXPAND** / **CROSS-BLOCKS** / **CROSS-META** / **CROSS-TIMEBLOCK** / **CROSS-COUNTS** / **CROSS-TOTAL**: native `report --json` ↔ Perl `query --json` **15/3/15** + discount **818** + `sub_entry_events` **0** when both expose; calls2 **27**; blocks-calls1 **780**/**810**; `time_block_events` **0**/**916** when both expose; event counts **27/3/13/632/31** + `file_1_basename` when both expose; default-calls1 stream/PID + A9/A8 + greppable meta when both expose) |
| 9 | `./scripts/packaging/capability_selftest_smoke.sh` | Run when cargo **or** `prefix`/`target` native CLI (or `$NYTPROF_NATIVE_CLI`); **honest skip** otherwise (**CI-CAPABILITY-GATE**) |
| 10 | `./scripts/packaging/collector_sink_smoke.sh` | **COL-001-SINK-MVP** — isolation asserts always; `make -C collector test` when CC present; **honest skip** without C toolchain. Stub v5 is **not** wire encode / COL-007 |

Not part of this gate (document only): broader `./scripts/packaging/packaging_gate.sh`, `./scripts/packaging/makemaker_dual_path_smoke.sh`. Not multi-OS CI (**BUILD-006**).

---

## 3. Native install + capability

### Install stable CLI under `prefix/bin`

Requires `cargo` on `PATH`.

```sh
./scripts/packaging/install_native.sh
# optional:
# PREFIX=/some/prefix NATIVE_RELEASE=1 ./scripts/packaging/install_native.sh
./scripts/packaging/native_install_smoke.sh
```

Installs:

```text
$REPO/prefix/bin/nytprof-cli
$REPO/prefix/bin/nytprof-dump   # same binary, dump-oriented name
```

Schema: [native-install-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-install-mvp-v0.md)

### Capability self-test (human + JSON)

```sh
# After install:
./prefix/bin/nytprof-cli capability
./prefix/bin/nytprof-cli capability --json

# Or via cargo:
cargo run -q -p nytprof-cli -- capability
cargo run -q -p nytprof-cli -- capability --json

# Packaging smoke (capability×2 + --json×2 + markers/fields):
./scripts/packaging/capability_selftest_smoke.sh
```

Expect (human): `OK: native capability self-test`, `decode: yes`, `report: yes`, `verify: yes`.  
Expect (JSON): `ok` / `decode` / `report` / `verify` true; `profile_ok` non-null when the default golden fixture is found.

Schema: [capability-selftest-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/capability-selftest-mvp-v0.md)

### Native aggregates JSON (NATIVE-AGG-JSON)

```sh
# Preferred:
./prefix/bin/nytprof-cli report --json fixtures/v5/default-calls1/nytprof.out
# Aliases:
./prefix/bin/nytprof-cli aggregates fixtures/v5/default-calls1/nytprof.out
cargo run -q -p nytprof-cli -- report --json fixtures/v5/default-calls1/nytprof.out

# Smoke (×2 + 15/3/15 field asserts):
./scripts/packaging/native_agg_json_smoke.sh
```

Expect JSON fields: `ok`, `profile`, `leaf_returns` **15**, `mid_returns` **3**, `mid_leaf_edge` **15**, `discount_events`, `sub_entry_events` (**JSON-SUB-ENTRY-MVP**; default **0**), greppable A4/A4b `line_calls_1_5` / `block_line_calls_1_4` (**JSON-BLOCKS-MVP**; 0 when absent; blocks-calls1 **780** / **810**), stream/PID (**JSON-NATIVE-STREAM-MVP**: `is_stream_complete` **true**, `incompleteness_reasons` **[]**, `time_line_events` / `pid_start_events` / `pid_end_events`), A2 `time_block_events` (**JSON-TIME-BLOCK-MVP**: default **0** / blocks **916**), A9/A8 samples (**JSON-SUBDEF-SOURCE-MVP**: `sub_def_leaf` / `sub_def_mid` / `source_line_1_5`), ATTRIBUTE/OPTION/file samples (**JSON-META-FILES-MVP**: `attribute_ticks_per_sec` / `option_calls` / `file_1`; null when absent), `subs`, `edges` (TAB edge keys). Fail-closed on incomplete/corrupt streams same as text report (**JSON-REPORT-INCOMPLETE-FAILCLOSED**: incomplete prefix must not emit complete `ok:true` + `is_stream_complete:true`).

```sh
# Incomplete stream fail-closed (COMPAT-010 / JSON-REPORT-INCOMPLETE-FAILCLOSED):
./scripts/packaging/json_report_incomplete_smoke.sh
# TIME_BLOCK multiplicity (JSON-TIME-BLOCK-MVP):
./scripts/packaging/json_time_block_smoke.sh
```

Schema: [native-aggregates-json-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-aggregates-json-mvp-v0.md)

### ATTRIBUTE / OPTION / file JSON samples (JSON-META-FILES-MVP)

Greppable samples only (not full `attributes` / `options` / `files` maps). Same keys on native `report --json` and Perl `query --json`.

| Field | Source | default-calls1 expect |
|-------|--------|------------------------|
| `attribute_ticks_per_sec` | ATTRIBUTE `ticks_per_sec` | string **`10000000`** (null if absent) |
| `option_calls` | OPTION `calls` | string **`1`** (null if absent) |
| `file_1` | NEW_FID fid **1** full path | path contains **`workload.pl`** (null if absent; **volatile** under `/tmp`) |
| `file_1_basename` | basename of fid **1** (**JSON-FILE-BASENAME-MVP**) | equals/contains **`workload.pl`** (typically exact **`"workload.pl"`**; stable contract) |

```sh
# Native (cargo tree preferred for latest fields):
cargo run -q -p nytprof-cli -- report --json fixtures/v5/default-calls1/nytprof.out \
  | grep -E 'attribute_ticks_per_sec|option_calls|file_1'

# Perl golden JSONL (no cargo):
perl -Iperl/lib perl/bin/nytprof-engine query --json \
  --jsonl fixtures/v5/default-calls1/readstream.jsonl \
  | grep -E 'attribute_ticks_per_sec|option_calls|file_1'

# Cargo asserts (model-matched):
cargo test -p nytprof-cli --test native_agg_json

# Focused packaging smokes (Perl golden + optional native):
./scripts/packaging/json_meta_files_smoke.sh
./scripts/packaging/json_file_basename_smoke.sh
```

### Cross-check native JSON vs Perl `query --json` (NATIVE-QUERY-JSON-CROSS / CROSS-EXPAND / CROSS-BLOCKS / CROSS-META / CROSS-TIMEBLOCK / CROSS-COUNTS / CROSS-TOTAL)

```sh
# Pair: native report --json  vs  Perl query --json --jsonl
# (×2 + equal fields + event counts 27/3/13/632/31 + basename + calls2 expand + blocks 780/810 + time_block 0/916 + stream/PID + A9/A8 + greppable meta)
./scripts/packaging/native_query_json_cross_smoke.sh
```

Shared fields must match on default-calls1: `leaf_returns` **15**, `mid_returns` **3**, `mid_leaf_edge` **15**, `discount_events` **818**, and `sub_entry_events` **0** when **both** sides expose SUB_ENTRY. Optional path also runs `query --json` on the live profile (native dump → JsonlData). **Expand (fixture-scoped):** on `fixtures/v5/calls2-default`, when both sides expose `sub_entry_events`, require **27**. **Blocks (NATIVE-QUERY-JSON-CROSS-BLOCKS):** on `fixtures/v5/blocks-calls1`, pair ×2 → `line_calls_1_5` **780** / `block_line_calls_1_4` **810** equal native↔perl. **Timeblock (NATIVE-QUERY-JSON-CROSS-TIMEBLOCK):** when both sides expose `time_block_events`, default-calls1 equal **0**, blocks-calls1 equal **916**. **Counts (NATIVE-QUERY-JSON-CROSS-COUNTS):** when both sides expose event counters (**JSON-EVENT-COUNTS-MVP**), equal `sub_return_events` **27**, `new_fid_events` **3**, `sub_callers_events` **13**, `src_line_events` **632**, `sub_info_events` **31**; when both expose `file_1_basename` (**JSON-FILE-BASENAME-MVP**), exact equal **or** both contain **`workload.pl`** (absolute `file_1` remains volatile; basename is the greppable stable sample). **Meta (NATIVE-QUERY-JSON-CROSS-META):** on default-calls1, when both sides expose stream/PID + A9/A8 samples, require equal `is_stream_complete` **true**, `incompleteness_reasons` **[]**, `time_line_events` / `pid_start_events` / `pid_end_events`, `sub_def_leaf` / `sub_def_mid` / `source_line_1_5`; greppable meta (`attribute_ticks_per_sec`, …) **required** equal when both expose. Fails closed without native CLI; pure-Perl query alone is `perl_query_json_smoke.sh`.

---

## 4. Facade: `nytprof-engine` (report / query / auto / fallback)

Thin Perl operator CLI under `perl/` (not on oracle `PERL5LIB`). Dispatches to native CLI or legacy oracle stream-dump.

```sh
# Report (native — needs installed or discoverable CLI / cargo)
perl -Iperl/lib perl/bin/nytprof-engine --engine=native report \
  fixtures/v5/default-calls1/nytprof.out

# Report (legacy oracle path — no cargo)
perl -Iperl/lib perl/bin/nytprof-engine --engine=legacy report \
  fixtures/v5/default-calls1/nytprof.out

# Prefer native; fall back to legacy if native CLI missing
perl -Iperl/lib perl/bin/nytprof-engine --engine=auto report \
  fixtures/v5/default-calls1/nytprof.out
# Env equivalent: NYTPROF_ENGINE=auto

# Query via native dump → JsonlData (or golden JSONL, no cargo)
perl -Iperl/lib perl/bin/nytprof-engine --engine=native query \
  fixtures/v5/default-calls1/nytprof.out
perl -Iperl/lib perl/bin/nytprof-engine query \
  --jsonl fixtures/v5/default-calls1/readstream.jsonl
# Structured JSON (QUERY-JSON-MVP / QUERY-JSON-EXPAND / JSON-SUB-ENTRY-MVP /
#   JSON-META-FILES-MVP / JSON-SUBDEF-SOURCE-MVP):
#   leaf_returns=15 / mid_returns=3 / mid_leaf_edge=15
#   discount_events=818 / sub_entry_events=0 / is_stream_complete=true
#   attribute_ticks_per_sec=10000000 / option_calls=1 / file_1=…workload.pl
#   (+ line_calls_1_5 / block_line_calls_1_4 when JSON-BLOCKS-MVP; 0 on this fixture)
perl -Iperl/lib perl/bin/nytprof-engine query --json \
  --jsonl fixtures/v5/default-calls1/readstream.jsonl

# Other native passthrough actions
perl -Iperl/lib perl/bin/nytprof-engine --engine=native verify \
  fixtures/v5/default-calls1/nytprof.out
perl -Iperl/lib perl/bin/nytprof-engine --engine=native html \
  fixtures/v5/default-calls1/nytprof.out -o /tmp/nytprof.html
perl -Iperl/lib perl/bin/nytprof-engine --engine=native csv \
  fixtures/v5/default-calls1/nytprof.out
perl -Iperl/lib perl/bin/nytprof-engine --engine=native folded \
  fixtures/v5/default-calls1/nytprof.out
perl -Iperl/lib perl/bin/nytprof-engine --engine=native callgrind \
  fixtures/v5/default-calls1/nytprof.out
```

| Engine | Behavior |
|--------|----------|
| `native` | Subprocess to `nytprof-cli`; missing CLI → **fail** (no silent legacy) |
| `auto` | **Perl facade:** prefer native, fall back to legacy + STDERR note; **not** charter R3 product default flip. Pure-Rust `nytprof-cli` still maps `auto` → `native`. |
| `legacy` | Oracle install-only `PERL5LIB` + stream dump smoke |

Smokes:

```sh
./scripts/packaging/perl_engine_dispatch_smoke.sh
./scripts/packaging/perl_engine_query_smoke.sh
./scripts/packaging/perl_engine_query_expand_smoke.sh
./scripts/packaging/perl_engine_query_pid_meta_smoke.sh
./scripts/packaging/perl_query_json_smoke.sh
./scripts/packaging/native_query_json_cross_smoke.sh   # needs native CLI
./scripts/packaging/perl_engine_export_smoke.sh
./scripts/packaging/engine_auto_smoke.sh
./scripts/packaging/engine_auto_fallback_smoke.sh
```

Schema: [perl-engine-dispatch-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-engine-dispatch-mvp-v0.md)

### Native CLI (direct) examples

```sh
cargo run -q -p nytprof-cli -- report fixtures/v5/default-calls1/nytprof.out
cargo run -q -p nytprof-cli -- verify fixtures/v5/default-calls1/nytprof.out
cargo run -q -p nytprof-cli -- dump fixtures/v5/default-calls1/nytprof.out
cargo run -q -p nytprof-cli -- html fixtures/v5/default-calls1/nytprof.out -o /tmp/nytprof.html
cargo run -q -p nytprof-cli -- html fixtures/v5/default-calls1/nytprof.out --out-dir /tmp/site
# or after install:
./prefix/bin/nytprof-cli report fixtures/v5/default-calls1/nytprof.out
```

---

## 5. Pure-Perl `JsonlData`

Dump-JSONL → queryable aggregates. **No XS**, no oracle `PERL5LIB`, core Perl + `JSON::PP` (via `JsonlReadStream`).

### Roll-up smoke (offline gate step 5)

```sh
./scripts/packaging/perl_jsonl_data_all_smoke.sh
```

Runs, in order: returns/edges → line_totals (A4) → sub_defs (A9) → source (A8) → A4b → ATTRIBUTE/OPTION → PID lifecycle → stream completeness (COMPAT-010) → DISCOUNT A3 multiplicity (`perl_discount_smoke`) → SUB_ENTRY multiplicity (`perl_sub_entry_smoke`: default-calls1 **0** / calls2-default **27**).

### Key APIs

```perl
use Devel::NYTProf::JsonlData;

my $data = Devel::NYTProf::JsonlData->from_jsonl($jsonl_path);
# or: from_cli([ $cli, 'dump', $profile ]); from_fh($fh);

$data->sub_returns('main::leaf');                  # returns count
$data->sub_return_totals;                          # { name => count }
$data->call_edge_count('main::mid', 'main::leaf');
$data->line_calls($fid, $line);                    # A4
$data->block_line_calls($fid, $block_line);        # A4b
$data->sub_def($name);                             # A9 { fid, first_line, last_line }
$data->file($fid);  $data->files;
$data->source_line($fid, $line);                   # A8
$data->attribute($key);  $data->option($key);
$data->pid_start_count;  $data->pid_end_count;
$data->pid_starts;  $data->pid_ends;
$data->discount_events;  $data->discount_count;    # A3 multiplicity only (818 on default-calls1)
$data->sub_entry_events; $data->sub_entry_count;   # SUB_ENTRY multiplicity (0 / 27)
$data->is_stream_complete;
$data->stream_incompleteness_reasons;
```

### SUB_ENTRY multiplicity (PERL-SUB-ENTRY-JSONL)

Event-count only (not call-stack / arg freeze). Independent stream re-count of `SUB_ENTRY` tags:

| Fixture | Expected `sub_entry_events` / `sub_entry_count` |
|---------|--------------------------------------------------|
| `fixtures/v5/default-calls1` (`calls=1`) | **0** |
| `fixtures/v5/calls2-default` (`calls=2`) | **27** |

```sh
./scripts/packaging/perl_sub_entry_smoke.sh
prove -Iperl/lib perl/t/jsonl_data_sub_entry.t
```

Module: `perl/lib/Devel/NYTProf/JsonlData.pm`  
Schema: [perl-jsonl-data-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-jsonl-data-mvp-v0.md)  
Stream bridge: [perl-jsonl-readstream-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-jsonl-readstream-mvp-v0.md)

---

## 6. Contracts (source of truth)

| Doc | Role |
|-----|------|
| [REPORT_SURFACE_CONTRACT_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_SURFACE_CONTRACT_v0.md) | Advertised native report/export/verify surfaces + frozen semantic counts |
| [R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) | Offline R0 / R1-preview **ready** vs residual full R1 |
| [REPORT_HTML_RESIDUAL_INVENTORY_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md) | Oracle `nytprofhtml` vs native HTML artifact residual honesty + full R1 CLOSE/WAIVE map |
| [0003-r1-full-residual-policy.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) | **ADR-0003** full R1 residual policy (HTML class map + **OQ-2** FFI/XS → CLOSE) |
| [BUILD_SUPPORT_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) | Dual-path packaging + offline gate policy |
| [FIRST_SLICE_BOARD.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md) | Ordered board (this runbook = `R1-PREVIEW-RUNBOOK`) |
| [PROGRAM_CHARTER.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md) | R0–R5 levels and non-goals |

---

## 7. Explicit residual honesty

Do **not** claim these under offline R0 / R1-preview (full-R1 residuals; `R1-HONESTY-SYNC`).

**Full R1 disposition (policy only — not yet shipped):**  
[ADR-0003](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) maps each residual to **CLOSE** (named Phase A PR) or **WAIVE** (or OUT-OF-R1). User **OQ-2 (resolved):** production **FFI** and **XS Data/ReadStream** must **CLOSE** via **PR-A05** / **PR-A06** — they are **not** eligible for waive at full R1. HTML classes close incrementally (PR-A01 CSS/JS, PR-A02 excl index, PR-A03 optional flame) or waive (Graphviz, treemap, block/sub page modes, oracle naming). Preview claims below remain honest until those PRs land and honesty docs flip.

| Residual | Notes |
|----------|--------|
| **FFI / XS Data residual** | **FFI MVP (PR-A05 / `FFI-CDYLIB-MVP`):** `crates/nytprof-ffi` cdylib open/query/close C ABI over `ProfileModel` is shipped ([`docs/schemas/ffi-cdylib-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/ffi-cdylib-mvp-v0.md)); panic-safe; dual-path still works **without** loading the dylib. **XS Data/ReadStream MVP (PR-A06 / `PERL-XS-DATA-READSTREAM-MVP`):** product `Devel::NYTProf::Data` + `::ReadStream` open **binary** profiles via native dump → JsonlData/JsonlReadStream ([`docs/schemas/perl-xs-data-readstream-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-xs-data-readstream-mvp-v0.md)); smoke `./scripts/packaging/perl_xs_data_readstream_smoke.sh`; `claims_compat007_shapes=0`. **Still residual:** full RUST-010 (batch/event-walk, BUILD-007, production dylib install, sanitizer package); full PERL-004 pure-XS wire decode; full PERL-005 COMPAT-007 bless-array. Preview primary = CLI subprocess + pure-Perl Jsonl* + product thin binary facades. **Full R1:** finish RUST-010 / PERL-004/005 completeness per ADR-0003 / **OQ-2** — not waive. |
| **No full nytprofhtml DOM** | Native HTML is MVP summary / multi-file site only — not oracle DOM, CSS/JS, tablesorter, flame SVG, Graphviz. See HTML residual inventory + **ADR-0003** per-class CLOSE/WAIVE map. |
| **No v6 / COL-007** | No v6 wire freeze; C v6 writer (**COL-007**) deferred; COL-008 batched Rust writer non-baseline. Collector remains 6.15 oracle / v5. **Preflight only (not full COL-007):** provisional fixed-header + chunk-frame + ULEB128 + ZigZag signed + length-prefixed string + header TLV + multi-TLV region + file-prefix + prefix+chunk stream + event-body (incl. TIME_BLOCK/SUB_ENTRY/SUB_RETURN/SUB_INFO/SRC_LINE/NEW_FID/PID_START/PID_END/SUB_CALLERS/DISCOUNT/ATTRIBUTE/OPTION/COMMENT/START_DEFLATE/VERSION/dual-output-sequence/mid-stream-codec-switch/auto-emit-VERSION/known-key-attr-option/unknown-optional-skip) + mini-profile + multi-chunk EVENT + SOURCE/INDEX/SUMMARY/FOOTER bodies + CRC32 optional verify + ZLIB/ZSTD/LZ4 payload codecs + compressed multi-codec mini-profile + multi-chunk compressed EVENT + compressed multi-kind mixed + per-kind codecs + multi-chunk EVENT under mixed + multi-chunk SOURCE + multi-chunk INDEX + multi-chunk SUMMARY + mid-record EVENT/SOURCE/INDEX/SUMMARY span + decoded-chunk + decoded-stream + decoded-EVENT/SOURCE/INDEX/SUMMARY + decoded-mixed + multi-chunk decoded-mixed + mid-record decoded-mixed (EVENT+SOURCE+INDEX+SUMMARY + concurrent multi-kind) always-inflate (`FMT-V6-HEADER-*` / `FMT-V6-CHUNK-*` / `FMT-V6-VARINT-*` / `FMT-V6-SVARINT-*` / `FMT-V6-STRING-*` / `FMT-V6-TLV-*` / `FMT-V6-TLV-REGION-*` / `FMT-V6-FILE-PREFIX-*` / `FMT-V6-PREFIX-CHUNK-STREAM-*` / `FMT-V6-EVENT-BODY-*` / `FMT-V6-MINI-PROFILE-*` / `FMT-V6-MULTI-CHUNK-EVENT-*` / `FMT-V6-SOURCE-BODY-*` / `FMT-V6-INDEX-BODY-*` / `FMT-V6-SUMMARY-BODY-*` / `FMT-V6-FOOTER-BODY-*` / `FMT-V6-CRC-*` / `FMT-V6-PAYLOAD-ZLIB-*` / `FMT-V6-PAYLOAD-ZSTD-*` / `FMT-V6-PAYLOAD-LZ4-*` / `FMT-V6-COMPRESSED-PROFILE-*` / `FMT-V6-MULTI-CHUNK-COMPRESSED-*` / `FMT-V6-COMPRESSED-MIXED-*` / `FMT-V6-PER-KIND-CODEC-*` / `FMT-V6-MULTI-CHUNK-KIND-*` / `FMT-V6-MULTI-CHUNK-SOURCE-*` / `FMT-V6-MULTI-CHUNK-INDEX-*` / `FMT-V6-MULTI-CHUNK-SUMMARY-*` / `FMT-V6-MID-RECORD-SPAN-*` / `FMT-V6-MID-RECORD-SOURCE-*` / `FMT-V6-MID-RECORD-INDEX-*` / `FMT-V6-MID-RECORD-SUMMARY-*` / `FMT-V6-DECODED-CHUNK-*` / `FMT-V6-DECODED-STREAM-*` / `FMT-V6-DECODED-EVENT-*` / `FMT-V6-DECODED-SOURCE-*` / `FMT-V6-DECODED-INDEX-*` / `FMT-V6-DECODED-SUMMARY-*` / `FMT-V6-DECODED-MIXED-*` / `FMT-V6-DECODED-MIXED-MULTI-CHUNK-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-INDEX-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-SUMMARY-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-CONCURRENT-*` / `FMT-V6-EVENT-BODY-TIME-BLOCK-SUB-ENTRY-*` / `FMT-V6-EVENT-BODY-SUB-RETURN-SUB-INFO-*` / `FMT-V6-EVENT-BODY-SRC-LINE-NEW-FID-*` / `FMT-V6-EVENT-BODY-PID-START-END-*` / `FMT-V6-EVENT-BODY-SUB-CALLERS-DISCOUNT-*` / `FMT-V6-EVENT-BODY-ATTRIBUTE-OPTION-*` / `FMT-V6-EVENT-BODY-COMMENT-*` / `FMT-V6-EVENT-BODY-START-DEFLATE-*` / `FMT-V6-EVENT-BODY-VERSION-*`; crate `nytprof-format-v6`). Default `parse_chunk_frame` stays **non-inflating**. |
| **No performance claims** | **Public P3/P4 certification waived** for R0 / R1-preview (board **R1-P3P4-METHODOLOGY** / PR-A09). Methodology + light engineering harness only: [`docs/BENCH_NOTES.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BENCH_NOTES.md), `tools/bench/light_bench.sh` (P3 proxies dump/verify; P4 proxies report/csv/html). No public SLOs, no CI perf gate, no signed WP-13 certificate. |
| **No full MakeMaker XS CPAN dual-build** | Candidate `Makefile.PL` facade only (**BUILD-MAKEMAKER-OPT**), not BUILD-003 full. |
| **No multi-OS CI matrix** | Single-host `offline_gate.sh` only (**BUILD-006** open). |
| **No product default flip** | Native remains opt-in; Perl `engine=auto` is facade behavior, not charter R3 product default. |

Advertised preview **does** include native aggregates JSON (incl. **SUB_ENTRY**, stream/PID, A2 `time_block_events` **0**/**916**, A9/A8 samples, ATTRIBUTE/OPTION/file samples, and blocks A4/A4b greppable ints), pure-Perl query JSON, **JSON report incomplete fail-closed** (COMPAT-010), **native↔query JSON cross-parity** with **CROSS-EXPAND** / **CROSS-BLOCKS** / **CROSS-META** / **CROSS-TIMEBLOCK** (`sub_entry` on default-calls1 **0** + calls2 **27** + blocks **780**/**810** + `time_block_events` **0**/**916** + stream/PID + A9/A8 + greppable meta when native CLI present), pure-Perl **SUB_ENTRY** event multiplicity, (when cargo present) **FFI open/query/close MVP** (`FFI-CDYLIB-MVP` / `cargo test -p nytprof-ffi`), and **product Data/ReadStream thin binary path MVP** (`PERL-XS-DATA-READSTREAM-MVP` / PR-A06) — without promoting those to full R1 / CPAN / full RUST-010 ABI freeze / full COMPAT-007 bless-array / pure-XS wire decode.

Full residual table: [R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) § Residual for full R1 (includes **Full R1 disposition** column).  
Policy ADR: [0003-r1-full-residual-policy.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md).

---

## 8. Golden fixture checks

Frozen semantic counts (counts exact; tick/time strings only under COMPAT-003):

### `fixtures/v5/default-calls1` (leaf / mid)

| Check | Expected |
|-------|----------|
| `main::leaf` returns | **15** |
| `main::mid` returns | **3** |
| `main::mid` → `main::leaf` edge | **15** |
| `discount_events` (A3) | **818** |
| `sub_entry_events` (`calls=1`; JSON + JsonlData) | **0** |
| `time_block_events` (**JSON-TIME-BLOCK-MVP**) | **0** |
| `attribute_ticks_per_sec` (**JSON-META-FILES-MVP**) | **`10000000`** |
| `option_calls` (**JSON-META-FILES-MVP**) | **`1`** |
| `file_1` (**JSON-META-FILES-MVP**) | path contains **`workload.pl`** (volatile absolute) |
| `file_1_basename` (**JSON-FILE-BASENAME-MVP**) | **`workload.pl`** (stable) |
| stream/PID + A9/A8 (**CROSS-META** when both expose) | complete + leaf **1/3–7** / mid **1/8–12** / source `$x++` |

```sh
# Native report text should show leaf returns=15, mid returns=3
./prefix/bin/nytprof-cli report fixtures/v5/default-calls1/nytprof.out
# Native / query JSON: leaf_returns=15, mid_returns=3, mid_leaf_edge=15,
#   discount_events=818, sub_entry_events=0,
#   attribute_ticks_per_sec=10000000, option_calls=1, file_1=…workload.pl

# Pure-Perl JsonlData from golden dump
perl -Iperl/lib -MDevel::NYTProf::JsonlData -e '
  my $d = Devel::NYTProf::JsonlData->from_jsonl(
    "fixtures/v5/default-calls1/readstream.jsonl");
  die "leaf" unless $d->sub_returns("main::leaf") == 15;
  die "mid"  unless $d->sub_returns("main::mid")  == 3;
  die "edge" unless $d->call_edge_count("main::mid","main::leaf") == 15;
  die "sub_entry" unless $d->sub_entry_count == 0;
  print "OK: default-calls1 leaf=15 mid=3 edge=15 sub_entry=0\n";
'

# Facade query (golden JSONL — no cargo)
perl -Iperl/lib perl/bin/nytprof-engine query \
  --jsonl fixtures/v5/default-calls1/readstream.jsonl
# expect lines: main::leaf returns=15, main::mid returns=3,
#               main::mid -> main::leaf count=15

# Report semantic parity (oracle HTML isolated + native html paths)
bash tools/oracle/report_semantic_parity.sh
```

### `fixtures/v5/blocks-calls1` (line 5 = 780; A4b 1:4 = 810)

| Check | Expected |
|-------|----------|
| `line_total(1,5).calls` / `line_calls(1,5)` (TIME_BLOCK) | **780** |
| JSON `line_calls_1_5` / `block_line_calls_1_4` (**JSON-BLOCKS-MVP**) | **780** / **810** |
| `time_block_events` (**JSON-TIME-BLOCK-MVP** / **CROSS-TIMEBLOCK**) | **916** |

```sh
perl -Iperl/lib -MDevel::NYTProf::JsonlData -e '
  my $d = Devel::NYTProf::JsonlData->from_jsonl(
    "fixtures/v5/blocks-calls1/readstream.jsonl");
  die "line5" unless $d->line_calls(1, 5) == 780;
  print "OK: blocks-calls1 line_calls(1,5)=780\n";
'

# Packaging smoke for A4
./scripts/packaging/perl_line_totals_smoke.sh

# Blocks semantic parity (native path)
bash tools/oracle/blocks_semantic_parity.sh

# JSON convenience (native + Perl query --json)
cargo run -q -p nytprof-cli -- report --json fixtures/v5/blocks-calls1/nytprof.out
perl -Iperl/lib perl/bin/nytprof-engine query --json \
  --jsonl fixtures/v5/blocks-calls1/readstream.jsonl
# expect line_calls_1_5=780, block_line_calls_1_4=810
```

Also on blocks-calls1 when asserted: leaf returns **15**, mid returns **3** (same workload shape). A4b reference: `block_line_calls(1,4)` → **810**.

### `fixtures/v5/calls2-default` (SUB_ENTRY)

| Check | Expected |
|-------|----------|
| `sub_entry_events` / `sub_entry_count` (`calls=2`) | **27** |
| `main::leaf` / `main::mid` returns (same workload) | **15** / **3** |

```sh
./scripts/packaging/perl_sub_entry_smoke.sh
```

---

## Quick operator checklist

1. `./scripts/ci/offline_gate.sh` → all steps green (or honest skips only where documented).  
2. `./scripts/packaging/install_native.sh` + `./prefix/bin/nytprof-cli capability` (+ `--json`).  
3. `nytprof-engine` report/query/auto paths exercise default-calls1 **15 / 3 / 15**.  
4. When native present: `./scripts/packaging/native_query_json_cross_smoke.sh` (native↔query JSON **15/3/15** + discount **818** + `sub_entry` **0**; calls2 **27** expand; blocks **780**/**810**; `time_block_events` **0**/**916** **CROSS-TIMEBLOCK**; stream/PID + A9/A8 + greppable meta **CROSS-META**); `./scripts/packaging/json_report_incomplete_smoke.sh` fail-closed.  
5. JsonlData roll-up smoke green (incl. SUB_ENTRY **0** / **27**); blocks-calls1 line5 **780** + JSON A4/A4b **780**/**810** + `time_block_events` **916**; greppable meta samples **10000000** / **1** / **workload.pl**.  
6. Read residual honesty before claiming “R1 done.”

---

## Board placement

| ID | Status | Evidence |
|----|--------|----------|
| `R1-PREVIEW-RUNBOOK` | done | this file (`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`) |
| `R1-RESIDUAL-MATRIX` | done | [R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) |
| `R1-RESIDUAL-POLICY-ADR` | **done** (PR-A04 policy) | [ADR-0003](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) — full R1 CLOSE/WAIVE map; **OQ-2** FFI/XS → CLOSE PR-A05/A06 (not waive); HTML per-class map. Preview residual honesty unchanged until close PRs land. |
| `FFI-CDYLIB-MVP` | **done** (MVP; PR-A05) | [`docs/schemas/ffi-cdylib-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/ffi-cdylib-mvp-v0.md); `crates/nytprof-ffi` + `include/nytprof_ffi.h`; `cargo test -p nytprof-ffi` (offline_gate step 1). OQ-2 product path; full RUST-010 residual remains. **Before COL-007.** |
| `PERL-XS-DATA-READSTREAM-MVP` | **done** (MVP; PR-A06) | [`docs/schemas/perl-xs-data-readstream-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-xs-data-readstream-mvp-v0.md); `Devel::NYTProf::Data` + `::ReadStream` thin binary path; `./scripts/packaging/perl_xs_data_readstream_smoke.sh`. OQ-2 product path; full PERL-004/005 / COMPAT-007 residual remains. **Before COL-007.** |
| `R1-HONESTY-SYNC` | **done** | matrix + this runbook advertise **NATIVE-QUERY-JSON-CROSS** / **CROSS-EXPAND** / **CROSS-BLOCKS** / **CROSS-META** / **CROSS-TIMEBLOCK** / **CROSS-COUNTS**, **JSON-EVENT-COUNTS-MVP**, **JSON-FILE-BASENAME-MVP** (absolute `file_1` volatile; basename greppable stable sample), **JSON-TIME-BLOCK-MVP**, **JSON-REPORT-INCOMPLETE-FAILCLOSED**, **JSON-SUB-ENTRY-MVP**, **JSON-BLOCKS-MVP**, **JSON-META-FILES-MVP**, + **PERL-SUB-ENTRY-JSONL** + **FFI-CDYLIB-MVP** (open/query/close) + **PERL-XS-DATA-READSTREAM-MVP** (PR-A06 thin binary Data/ReadStream; offline_gate step **5b**) while listing full-R1 residuals (full RUST-010 beyond MVP; full PERL-004/005 beyond thin product MVP — no COMPAT-007 / pure-XS wire decode; no full nytprofhtml DOM, no v6/COL-007, no multi-OS CI, no perf claims). **Before COL-007.** |

| `NATIVE-QUERY-JSON-CROSS` | done | `scripts/packaging/native_query_json_cross_smoke.sh`; offline_gate step 8 when native |
| `NATIVE-QUERY-JSON-CROSS-EXPAND` | done | same smoke: `sub_entry_events` **0** on default-calls1 when both expose; calls2-default **27** (fixture-scoped). **Before COL-007.** |
| `NATIVE-QUERY-JSON-CROSS-BLOCKS` | **done** | same smoke: blocks-calls1 pair ×2 `line_calls_1_5` **780** / `block_line_calls_1_4` **810** equal native↔perl. **Before COL-007.** |
| `NATIVE-QUERY-JSON-CROSS-META` | **done** | same smoke: default-calls1 stream/PID + A9/A8 equal when both expose; greppable meta when both expose. **Before COL-007.** |
| `NATIVE-QUERY-JSON-CROSS-TIMEBLOCK` | **done** | same smoke: `time_block_events` default **0** / blocks **916** when both expose. **Before COL-007.** |
| `NATIVE-QUERY-JSON-CROSS-COUNTS` | **done** | same smoke: default-calls1 event counts **27/3/13/632/31** + `file_1_basename` when both expose. **Before COL-007.** |
| `JSON-EVENT-COUNTS-MVP` | **done** | event counters on both JSON surfaces; smoke `json_event_counts_smoke / json_total_basetime_smoke.sh`. **Before COL-007.** |
| `JSON-NATIVE-STREAM-MVP` | **done** | native `report --json` stream/PID fields; `scripts/packaging/json_native_stream_smoke.sh`. **Before COL-007.** |
| `JSON-TIME-BLOCK-MVP` | **done** | `time_block_events` A2; smoke `scripts/packaging/json_time_block_smoke.sh`; offline_gate step 6f. **Before COL-007.** |
| `JSON-REPORT-INCOMPLETE-FAILCLOSED` | **done** | `report --json` fail-closed on incomplete streams; smoke `scripts/packaging/json_report_incomplete_smoke.sh`; offline_gate when native. **Before COL-007.** |
| `JSON-SUBDEF-SOURCE-MVP` | **done** | `sub_def_leaf` / `sub_def_mid` / `source_line_1_5`; `scripts/packaging/json_subdef_source_smoke.sh`. **Before COL-007.** |
| `JSON-META-FILES-MVP` | **done** | `attribute_ticks_per_sec` / `option_calls` / `file_1` on native + Perl JSON; smoke `scripts/packaging/json_meta_files_smoke.sh`; cargo `native_agg_json.rs`. **Before COL-007.** |
| `JSON-FILE-BASENAME-MVP` | **done** | `file_1_basename` on native + Perl JSON (**`workload.pl`**); smoke `scripts/packaging/json_file_basename_smoke.sh`; offline_gate step 6g; cargo `native_agg_json.rs`. **Before COL-007.** |
| `JSON-TOTAL-EVENTS-MVP` | **done** | `total_events` **2474** on both JSON surfaces; smoke `json_total_basetime_smoke.sh`; offline_gate step 6i. **Before COL-007.** |
| `JSON-ATTR-BASETIME-MVP` | **done** | `attribute_basetime` greppable sample; same smoke. **Before COL-007.** |
| `NATIVE-QUERY-JSON-CROSS-TOTAL` | **done** | cross equal total_events + basetime. **Before COL-007.** |
| `JSON-SUB-ENTRY-MVP` | done | `sub_entry_events` on native `report --json` + Perl `query --json` (default **0** / calls2 **27**) |
| `JSON-BLOCKS-MVP` | done | greppable `line_calls_1_5` / `block_line_calls_1_4` on native + Perl JSON (blocks-calls1 **780** / **810**) |
| `PERL-SUB-ENTRY-JSONL` | done | `JsonlData` `sub_entry_*`; smoke + test above; roll-up in `perl_jsonl_data_all_smoke.sh` |
| `FMT-V6-HEADER-PROVISIONAL` | **done** | provisional v6 header contract (not freeze). **Before full COL-007.** |
| `FMT-V6-HEADER-PARSE-MVP` | **done** | shipped parse + tests `nytprof-format-v6`. **Before full COL-007.** |
| `FMT-V6-CHUNK-PROVISIONAL` | **done** | provisional chunk-frame contract. **Before full COL-007.** |
| `FMT-V6-CHUNK-PARSE-MVP` | **done** | `parse_chunk_frame` + tests. **Before full COL-007.** |
| `FMT-V6-VARINT-PROVISIONAL` | **done** | provisional ULEB128 contract. **Before full COL-007.** |
| `FMT-V6-VARINT-MVP` | **done** | `encode_u64`/`decode_u64` + tests. **Before full COL-007.** |
| `FMT-V6-SVARINT-PROVISIONAL` | **done** | provisional ZigZag+ULEB128 contract. **Before full COL-007.** |
| `FMT-V6-SVARINT-MVP` | **done** | `encode_i64`/`decode_i64` + tests. **Before full COL-007.** |
| `FMT-V6-STRING-PROVISIONAL` | **done** | provisional string/blob contract. **Before full COL-007.** |
| `FMT-V6-STRING-MVP` | **done** | `encode_string_blob`/`decode_string_blob` + tests. **Before full COL-007.** |
| `FMT-V6-TLV-PROVISIONAL` | **done** | provisional header TLV contract. **Before full COL-007.** |
| `FMT-V6-TLV-MVP` | **done** | `encode_tlv`/`decode_tlv` + tests. **Before full COL-007.** |
| `FMT-V6-TLV-REGION-PROVISIONAL` | **done** | multi-TLV region + END terminator. **Before full COL-007.** |
| `FMT-V6-TLV-REGION-MVP` | **done** | `encode_tlv_region`/`decode_tlv_region` + tests. **Before full COL-007.** |
| `FMT-V6-FILE-PREFIX-PROVISIONAL` | **done** | fixed header + multi-TLV file-prefix. **Before full COL-007.** |
| `FMT-V6-FILE-PREFIX-MVP` | **done** | `encode_file_prefix`/`decode_file_prefix` + tests. **Before full COL-007.** |
| `FMT-V6-PREFIX-CHUNK-STREAM-PROVISIONAL` | **done** | prefix + chunk stream layout; codec NONE MVP. **Before full COL-007.** |
| `FMT-V6-PREFIX-CHUNK-STREAM-MVP` | **done** | `encode_prefix_chunk_stream`/`decode_prefix_chunk_stream` + tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-PROVISIONAL` | **done** | event-body opcode codec (codec NONE payload). **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-MVP` | **done** | `encode_event_body`/`decode_event_body` + tests. **Before full COL-007.** |
| `FMT-V6-MINI-PROFILE-PROVISIONAL` | **done** | mini-profile composition (prefix + EVENT + optional FOOTER). **Before full COL-007.** |
| `FMT-V6-MINI-PROFILE-MVP` | **done** | `encode_mini_profile`/`decode_mini_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-EVENT-PROVISIONAL` | **done** | multi-chunk EVENT body framing (records-per-chunk). **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-EVENT-MVP` | **done** | `encode_multi_chunk_event_profile`/`decode_multi_chunk_event_profile` + tests. **Before full COL-007.** |
| `FMT-V6-SOURCE-BODY-PROVISIONAL` | **done** | SOURCE chunk body codec NONE. **Before full COL-007.** |
| `FMT-V6-SOURCE-BODY-MVP` | **done** | `encode_source_body`/`decode_source_body` + EVENT+SOURCE composition + tests. **Before full COL-007.** |
| `FMT-V6-INDEX-BODY-PROVISIONAL` | **done** | INDEX chunk body codec NONE. **Before full COL-007.** |
| `FMT-V6-INDEX-BODY-MVP` | **done** | `encode_index_body`/`decode_index_body` + mixed composition + tests. **Before full COL-007.** |
| `FMT-V6-SUMMARY-BODY-PROVISIONAL` | **done** | SUMMARY chunk body codec NONE. **Before full COL-007.** |
| `FMT-V6-SUMMARY-BODY-MVP` | **done** | `encode_summary_body`/`decode_summary_body` + mixed composition + tests. **Before full COL-007.** |
| `FMT-V6-FOOTER-BODY-PROVISIONAL` | **done** | FOOTER chunk body codec NONE (last chunk). **Before full COL-007.** |
| `FMT-V6-FOOTER-BODY-MVP` | **done** | `encode_footer_body`/`decode_footer_body` + mixed FOOTER-last composition + tests. **Before full COL-007.** |
| `FMT-V6-CRC-PROVISIONAL` | **done** | CRC32 IEEE header/payload contract. **Before full COL-007.** |
| `FMT-V6-CRC-MVP` | **done** | `crc32_ieee` + sealed encode + optional verify + tests. **Before full COL-007.** |
| `FMT-V6-PAYLOAD-ZLIB-PROVISIONAL` | **done** | ZLIB payload codec contract. **Before full COL-007.** |
| `FMT-V6-PAYLOAD-ZLIB-MVP` | **done** | `deflate_zlib`/`inflate_zlib`/`decode_chunk_payload` + tests. **Before full COL-007.** |
| `FMT-V6-PAYLOAD-ZSTD-PROVISIONAL` | **done** | ZSTD payload codec contract (`codec=2`). **Before full COL-007.** |
| `FMT-V6-PAYLOAD-ZSTD-MVP` | **done** | `compress_zstd`/`decompress_zstd`/`encode_chunk_frame_zstd` + tests. **Before full COL-007.** |
| `FMT-V6-PAYLOAD-LZ4-PROVISIONAL` | **done** | LZ4 payload codec contract (`codec=3`, raw block). **Before full COL-007.** |
| `FMT-V6-PAYLOAD-LZ4-MVP` | **done** | `compress_lz4`/`decompress_lz4`/`encode_chunk_frame_lz4` + tests. **Before full COL-007.** |
| `FMT-V6-COMPRESSED-PROFILE-PROVISIONAL` | **done** | Compressed multi-codec mini-profile contract. **Before full COL-007.** |
| `FMT-V6-COMPRESSED-PROFILE-MVP` | **done** | `encode_compressed_mini_profile`/`decode_compressed_mini_profile` + per-codec tests. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-COMPRESSED-PROVISIONAL` | **done** | Multi-chunk EVENT + compressed payloads contract. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-COMPRESSED-MVP` | **done** | `encode_multi_chunk_compressed_profile`/`decode_multi_chunk_compressed_profile` + tests. **Before full COL-007.** |
| `FMT-V6-COMPRESSED-MIXED-PROVISIONAL` | **done** | Compressed multi-kind mixed (EVENT/SOURCE/INDEX/SUMMARY) contract. **Before full COL-007.** |
| `FMT-V6-COMPRESSED-MIXED-MVP` | **done** | `encode_compressed_mixed_profile`/`decode_compressed_mixed_profile` + tests. **Before full COL-007.** |
| `FMT-V6-PER-KIND-CODEC-PROVISIONAL` | **done** | Per-kind payload codecs on mixed profiles contract. **Before full COL-007.** |
| `FMT-V6-PER-KIND-CODEC-MVP` | **done** | `KindCodecs` + `encode_compressed_mixed_profile_per_kind` + tests. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-KIND-PROVISIONAL` | **done** | Multi-chunk EVENT under compressed mixed contract. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-KIND-MVP` | **done** | `encode_multi_chunk_kind_mixed_profile` + multi-EVENT + SOURCE tests. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-SOURCE-PROVISIONAL` | **done** | Multi-chunk SOURCE under compressed mixed contract. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-SOURCE-MVP` | **done** | `partition_source_records` + `encode_multi_chunk_source_mixed_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-INDEX-PROVISIONAL` | **done** | Multi-chunk INDEX under compressed mixed contract. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-INDEX-MVP` | **done** | `partition_index_records` + `encode_multi_chunk_index_mixed_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-SUMMARY-PROVISIONAL` | **done** | Multi-chunk SUMMARY under compressed mixed contract. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-SUMMARY-MVP` | **done** | `partition_summary_records` + `encode_multi_chunk_summary_mixed_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-SPAN-PROVISIONAL` | **done** | Mid-record span across EVENT chunks contract. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-SPAN-MVP` | **done** | `encode_mid_record_span_event_profile`/`decode_mid_record_span_event_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-SOURCE-PROVISIONAL` | **done** | Mid-record span across SOURCE chunks contract. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-SOURCE-MVP` | **done** | `encode_mid_record_span_source_profile`/`decode_mid_record_span_source_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-INDEX-PROVISIONAL` | **done** | Mid-record span across INDEX chunks contract. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-INDEX-MVP` | **done** | `encode_mid_record_span_index_profile`/`decode_mid_record_span_index_profile` + tests. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-SUMMARY-PROVISIONAL` | **done** | Mid-record span across SUMMARY chunks contract. **Before full COL-007.** |
| `FMT-V6-MID-RECORD-SUMMARY-MVP` | **done** | `encode_mid_record_span_summary_profile`/`decode_mid_record_span_summary_profile` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-CHUNK-PROVISIONAL` | **done** | Always-inflate consumer path + optional CRC contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-CHUNK-MVP` | **done** | `decode_chunk`/`decode_chunk_frame_plain` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-STREAM-PROVISIONAL` | **done** | Always-inflate multi-chunk stream contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-STREAM-MVP` | **done** | `decode_prefix_chunk_stream_plain`/`encode_prefix_sealed_chunks` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-EVENT-PROVISIONAL` | **done** | Stream→inflate→event-body contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-EVENT-MVP` | **done** | `encode_decoded_event_profile`/`decode_decoded_event_profile` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-SOURCE-PROVISIONAL` | **done** | Stream→inflate→source-body contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-SOURCE-MVP` | **done** | `encode_decoded_source_profile`/`decode_decoded_source_profile` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-INDEX-PROVISIONAL` | **done** | Stream→inflate→index-body contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-INDEX-MVP` | **done** | `encode_decoded_index_profile`/`decode_decoded_index_profile` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-SUMMARY-PROVISIONAL` | **done** | Stream→inflate→summary-body contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-SUMMARY-MVP` | **done** | `encode_decoded_summary_profile`/`decode_decoded_summary_profile` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-PROVISIONAL` | **done** | Multi-kind always-inflate + optional CRC contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MVP` | **done** | `encode_decoded_mixed_profile`/`decode_decoded_mixed_profile` + tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MULTI-CHUNK-PROVISIONAL` | **done** | Multi-chunk record-aligned always-inflate mixed contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MULTI-CHUNK-MVP` | **done** | `encode_decoded_mixed_multi_chunk_profile` + multi-chunk decode tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-PROVISIONAL` | **done** | Mid-record span on always-inflate multi-kind mixed contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-MVP` | **done** | `encode_decoded_mixed_mid_record_event_profile` + mid-record mixed tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-PROVISIONAL` | **done** | SOURCE mid-record on always-inflate multi-kind mixed contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-MVP` | **done** | `encode_decoded_mixed_mid_record_source_profile` + SOURCE mid-on-mixed tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-INDEX-PROVISIONAL` | **done** | INDEX mid-record on always-inflate multi-kind mixed contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-INDEX-MVP` | **done** | `encode_decoded_mixed_mid_record_index_profile` + INDEX mid-on-mixed tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-SUMMARY-PROVISIONAL` | **done** | SUMMARY mid-record on always-inflate multi-kind mixed contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-SUMMARY-MVP` | **done** | `encode_decoded_mixed_mid_record_summary_profile` + SUMMARY mid-on-mixed tests. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-CONCURRENT-PROVISIONAL` | **done** | Concurrent multi-kind mid-record on always-inflate mixed contract. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-DECODED-MIXED-MID-RECORD-CONCURRENT-MVP` | **done** | `encode_decoded_mixed_mid_record_concurrent_profile` + concurrent mid-on-mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-TIME-BLOCK-SUB-ENTRY-PROVISIONAL` | **done** | TIME_BLOCK + SUB_ENTRY provisional event-body opcodes. Not full catalog freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-TIME-BLOCK-SUB-ENTRY-MVP` | **done** | Event-body TIME_BLOCK/SUB_ENTRY + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SUB-RETURN-SUB-INFO-PROVISIONAL` | **done** | SUB_RETURN + SUB_INFO provisional event-body opcodes. Not full catalog freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SUB-RETURN-SUB-INFO-MVP` | **done** | Event-body SUB_RETURN/SUB_INFO + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SRC-LINE-NEW-FID-PROVISIONAL` | **done** | SRC_LINE + NEW_FID provisional event-body opcodes. Not full catalog freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SRC-LINE-NEW-FID-MVP` | **done** | Event-body SRC_LINE/NEW_FID + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-PID-START-END-PROVISIONAL` | **done** | PID_START + PID_END provisional event-body opcodes. Not full catalog freeze / COL-015. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-PID-START-END-MVP` | **done** | Event-body PID_START/PID_END + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SUB-CALLERS-DISCOUNT-PROVISIONAL` | **done** | SUB_CALLERS + DISCOUNT provisional event-body opcodes. Not full catalog freeze / DISCOUNT accounting freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SUB-CALLERS-DISCOUNT-MVP` | **done** | Event-body SUB_CALLERS/DISCOUNT + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-ATTRIBUTE-OPTION-PROVISIONAL` | **done** | ATTRIBUTE + OPTION provisional event-body opcodes. Not full catalog freeze / key vocabulary freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-ATTRIBUTE-OPTION-MVP` | **done** | Event-body ATTRIBUTE/OPTION + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-COMMENT-PROVISIONAL` | **done** | COMMENT provisional event-body opcode. Not START_DEFLATE-as-event freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-COMMENT-MVP` | **done** | Event-body COMMENT + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-START-DEFLATE-PROVISIONAL` | **done** | START_DEFLATE provisional event-body opcode (marker only). Not VERSION prelude freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-START-DEFLATE-MVP` | **done** | Event-body START_DEFLATE + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-VERSION-PROVISIONAL` | **done** | VERSION provisional event-body opcode (major/minor). Not OI-001-03 sequence-number freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-VERSION-MVP` | **done** | Event-body VERSION + always-inflate EVENT/mixed tests. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-DUAL-OUTPUT-SEQUENCE-PROVISIONAL` | **done** | Dual-output multi-record EVENT order preflight (VERSION→meta→START_DEFLATE?→PID_START…PID_END). Not OI-001-03 sequence-number freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-DUAL-OUTPUT-SEQUENCE-MVP` | **done** | Dual-output sequence body + always-inflate EVENT/mixed tests (order+fields; NONE/ZLIB/ZSTD/LZ4; SOURCE co-kind). **Before full COL-007.** |
| `FMT-V6-START-DEFLATE-MID-STREAM-CODEC-SWITCH-PROVISIONAL` | **done** | START_DEFLATE mid-stream chunk-codec switch preflight (pre NONE + post ZLIB/ZSTD/LZ4). Not v5 mid-payload stream deflate freeze / OI-001-03. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-START-DEFLATE-MID-STREAM-CODEC-SWITCH-MVP` | **done** | Mid-stream codec-switch encode/decode + always-inflate EVENT/mixed tests (order+fields; SOURCE co-kind). **Before full COL-007.** |
| `FMT-V6-AUTO-EMIT-VERSION-PROVISIONAL` | **done** | Auto-emit VERSION from fixed-header major/minor preflight. Not OI-001-03 / full key-vocab freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-AUTO-EMIT-VERSION-MVP` | **done** | Auto-emit VERSION helpers + always-inflate EVENT/mixed tests (header-tied major/minor; NONE/ZLIB/ZSTD/LZ4; mismatch fail-closed). **Before full COL-007.** |
| `FMT-V6-ATTR-OPTION-KNOWN-KEY-PROVISIONAL` | **done** | ATTRIBUTE/OPTION known-key vocabulary preflight (basetime, ticks_per_sec, application, calls, …). Not complete OI-002-03/04 freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-ATTR-OPTION-KNOWN-KEY-MVP` | **done** | Known-key table + always-inflate EVENT/mixed tests (key+value asserts; NONE/ZLIB/ZSTD/LZ4; SOURCE co-kind). **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-UNKNOWN-OPTIONAL-SKIP-PROVISIONAL` | **done** | Unknown optional length-framed skip preflight (`FLAG_BODY_LENGTH`). Not permanent flag freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-UNKNOWN-OPTIONAL-SKIP-MVP` | **done** | Length-framed unknown-optional skip + always-inflate EVENT/mixed tests (order+fields; SOURCE co-kind). **Before full COL-007.** |
| `COL-001-SINK-MVP` | **done (scaffold)** | Overlay `collector/` semantic sink + counting + stub v5; smoke offline_gate step 10; **not** COL-006 wire / COL-007 / live hooks |
| `COL-007` | deferred | C v6 writer — unblocked for *start* after report-side evidence; not implemented by this runbook |

## Revision rule

Expanding advertised preview surfaces, closing residual rows, or changing the offline gate step list requires updating this runbook **and** the residual matrix / surface contract as appropriate. This document is an **operator map**, not release certification.

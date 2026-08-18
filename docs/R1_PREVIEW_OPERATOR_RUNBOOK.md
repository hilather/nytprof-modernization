# R1-preview operator runbook (offline R0 / R1-preview)

**Status:** operator-facing consolidation of the offline R0 / R1-preview stack + **full R1 MVP product cut** honesty (PR-A10)  
**Board ID:** `R1-PREVIEW-RUNBOOK` (honesty sync: `R1-HONESTY-SYNC`; full cut: `R1-FULL-READINESS-CUT`)  
**Date:** 2026-08-11 (E1b attach honesty 2026-08-17)  
**Gate:** done **before COL-007** (C v6 writer)

---

## 1. What this is

This runbook is the single operator entry for **offline developer preview** of the first-slice modernization stack:

| Level | Meaning here |
|-------|----------------|
| **R0** | Developer preview (experimental / opt-in tools). No product default change. See [PROGRAM_CHARTER.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md). |
| **R1-preview** | Opt-in **native v5 read/report** path + pure-Perl dump-JSONL query facade. |
| **Full R1 (MVP product scope)** | Phase A product cut per [ADR-0003](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) + [RELEASE_NOTES_R1.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R1.md) (**PR-A10**): FFI/XS **MVP closed**, HTML A01/A02 closed (A03 flame residual), multi-OS MVP, packaging depth, public perf **waived**. **Not** complete residual-table close. |

**This is not:**

- a full **R1** product release or CPAN readiness statement;
- production **FFI / XS** `Devel::NYTProf::Data` materialization;
- full oracle **`nytprofhtml` DOM** / CSS / tablesorter / flame / Graphviz parity;
- CLI **v6 default** / collection default format flip (collection remains v5 until R4); E3-mixed multi-kind C fixtures; **COL-008** batched Rust writer (COL-007 product E3-EVENT **and** major=6 wire freeze ADR-0006 are **done**);
- performance certification or public perf claims;
- a CPAN upload or full **BUILD-003** XS dual-build certification;
- full **RUST-010** beyond open/query/close MVP, or full **COMPAT-007 / pure-XS** Data materialization (thin product Data/ReadStream MVP is shipped under **PERL-XS-DATA-READSTREAM-MVP**);
- full oracle **`nytprofhtml` DOM** / tablesorter / flame / Graphviz parity (A01/A02 MVP HTML only; Shared JS/tablesorter **WAIVE** for GA-candidate — **PR-M01**; A03 flame open; other WAIVE classes residual);
- **v6** wire freeze, CLI v6 default, or **COL-007** C v6 writer;
- performance certification or public perf SLOs (**A09 default = WAIVE**);
- permission to flip product defaults (`engine=auto` as R3 product default, format defaults — charter R3/R4).

**Ready vs residual freeze:**  
[R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) (§ Residual for full R1 + § Full R1 ready)

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
| 10 | `./scripts/packaging/collector_sink_smoke.sh` | **COL-001..007 + COL-014 dual (test/dev-only OQ-4)** — isolation always; `make -C collector test` when CC; honest skip without C. |
| 11 | `./tools/oracle/e3_c_writer_parity.sh` | **COL-007 product E3-EVENT + E3-mixed MVP** (when cargo): C fixtures `fixtures/v6/from-c/**` incl. `mixed.nytprof` + `e3_c_*` / `e3_c_mixed_*`; honest skip without cargo (fixture presence + NYTPROF6 still checked). **Not** TEST-008 / COL-008 / CLI v6 collection default. |
| 12 | `./scripts/packaging/e4_v5_v6_semantic_smoke.sh --full` | When native CLI available (**E4 product CLI** on dual-sink pairs); honest skip otherwise; dual-sink fixture presence still required; full oracle dual residual (TEST-008) |

Not part of this gate (document only): broader `./scripts/packaging/packaging_gate.sh`, `./scripts/packaging/makemaker_dual_path_smoke.sh`. Multi-OS matrix entry is separate: `./scripts/ci/matrix_gate.sh` + GHA **BUILD-006-MVP** (not full **BUILD-006**).

### Multi-OS matrix MVP (BUILD-006-MVP)

```sh
# Same offline gate on this host after ensuring a host-local oracle pin
./scripts/ci/matrix_gate.sh
```

| Piece | Path |
|-------|------|
| Matrix entry | [scripts/ci/matrix_gate.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/matrix_gate.sh) |
| GHA workflow | [.github/workflows/ci-matrix.yml](https://github.com/hilather/nytprof-modernization/blob/main/.github/workflows/ci-matrix.yml) |
| Policy | [BUILD_SUPPORT_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) § Multi-OS CI matrix MVP |

GHA matrix rows: **ubuntu-latest** (`linux-x86_64`) + **macos-latest** (`macos-arm64`; current GitHub macOS runners are Apple Silicon). Honest skips inside `offline_gate.sh` are unchanged. This is **not** full multi-Perl / multi-rustc / Windows certification.

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

Expect (human): `OK: native capability self-test`, `decode: yes`, `report: yes`, `verify: yes`, plus E5 honesty `v6_decode: yes` / `v6_report: yes` / `convert: yes` / `merge: yes` / `repack: yes` / `salvage: yes` / `collection_default: v5`; `profile_ok` / `v6_profile_ok` non-skip when the default v5 / dual-sink v6 fixtures resolve.  
Expect (JSON): `ok` / `decode` / `report` / `verify` / `v6_decode` / `v6_report` / `convert` / `merge` / `repack` / `salvage` true; `collection_default` `"v5"`; `profile_ok` non-null when the default v5 golden is found; `v6_profile_ok` non-null when `fixtures/e4/dual-sink/default_calls1_v6.nytprof` resolves.

Strict convert (PR-C01; integer-tick dual-sink / representable streams):

```bash
cargo run -q -p nytprof-cli -- convert --to=v6 fixtures/e4/dual-sink/m4_v5.nytprof -o /tmp/m4.v6
cargo run -q -p nytprof-cli -- convert --to=v5 fixtures/e4/dual-sink/m4_v6.nytprof -o /tmp/m4.v5
cargo run -q -p nytprof-cli -- verify /tmp/m4.v5
```

Merge / repack / salvage (PR-C02; recovery semantics unambiguous):

```bash
cargo run -q -p nytprof-cli -- repack fixtures/e4/dual-sink/m4_v6.nytprof -o /tmp/m4.repack.v6
cargo run -q -p nytprof-cli -- merge --to=v6 -o /tmp/m4.merged.v6 \
  fixtures/e4/dual-sink/m4_v5.nytprof fixtures/e4/dual-sink/m4_v6.nytprof
head -c 68 fixtures/e4/dual-sink/m4_v5.nytprof > /tmp/m4.cut.v5
cargo run -q -p nytprof-cli -- salvage /tmp/m4.cut.v5 -o /tmp/m4.salvaged.v5
# Expect: OK: salvage incomplete=yes ...
```

Schema: [convert-strict-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/convert-strict-mvp-v0.md)  
Schema: [merge-repack-salvage-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/merge-repack-salvage-mvp-v0.md)

Schema: [capability-selftest-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/capability-selftest-mvp-v0.md)

Schema: [capability-selftest-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/capability-selftest-mvp-v0.md) (E5 fields); [cli-e5-v6-opt-in-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/cli-e5-v6-opt-in-mvp-v0.md)

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
| `auto` | **Perl facade:** prefer native, fall back to legacy + STDERR note; **not** charter R3 product default (omit still → `native` until gated flip). Policy: [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md). Pure-Rust `nytprof-cli` still maps `auto` → `native`. |
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
| [RELEASE_NOTES_R1.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R1.md) | Full R1 MVP product cut release notes (PR-A10) |
| [BUILD_SUPPORT_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) | Dual-path packaging + offline gate policy |
| [FIRST_SLICE_BOARD.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md) | Ordered board (this runbook = `R1-PREVIEW-RUNBOOK`) |
| [RELEASE_NOTES_R2_PREVIEW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_PREVIEW.md) | R2-preview packaging notes (v6 **opt-in only**; COL-009 / ADR-0007) |
| [DUAL_EQUALITY_READINESS_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md) | Dual-equality E1–E5 readiness checklist |
| [PROGRAM_CHARTER.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md) | R0–R5 levels and non-goals |
| [R3_FIELD_WINDOW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md) | R3 field-window evidence pack (PR-D01; **no** runtime default flip) |
| [0005-r3-engine-auto-default-promotion.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) | **ADR-0005** R3 promotion policy (PR-D02; flip **not** executed) |
| [R3_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) | R3 flip execution + rollback checklist (gated on accepted field report) |
| [0009-r5-legacy-retirement-governance.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0009-r5-legacy-retirement-governance.md) | **ADR-0009** R5 retirement governance (PR-F01; **no** component retired) |
| [R5_RETIREMENT_REVIEW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R5_RETIREMENT_REVIEW.md) | R5 per-component review + retirement checklist (never automatic) |
| [DROP_IN_DOD_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md) | Drop-in D1–D6 contract (**docs-landed**; attach not ready) |
| [product-xs-graft-annex-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-graft-annex-v0.md) | Graft annex A–C (A.6 archive + bootstrap landed; A.4 G03a–G03e emit + G04 attach-MVP landed; A.5 fake-clock mini landed; mid-deflate fork / G05 / G06 residual) |
| [PRODUCT_COMPLETION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PRODUCT_COMPLETION_DROP_IN_v0.md) | Approved rev-4 drop-in design (PR-G01) |

---

## 7. Explicit residual honesty

### 7.0 Drop-in contracts (PR-G01 docs-landed + PR-G02 scaffold + PR-G03a–e emit + PR-G04 attach-MVP)

**Option B identity (PR-A4):** operators switch to **`perl -d:NYTProfM`** / **`perl-NYTProfM`** **6.15**. See [MIGRATION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/MIGRATION_DROP_IN_v0.md). Historical G03a/G04 rows below still describe the attach-MVP landing; the shipped debugger name is `-d:NYTProfM`.

G01 landed the drop-in DoD, graft annex, and three isolation profiles. G02 landed the **v5-only product archive** and a **load-only** bootstrap XS. G03a landed product `perl -d:NYTProf` **load** (in-memory sink; no `nytprof.out` on trivial `-e`). G03b–G03e landed **emit-MVP**. G04 landed **attach-MVP**: live `perl -d:NYTProf` with `NYTPROF file=` on default-calls1-shaped work; shipped dump/report of those bytes shows leaf **15** / mid **3** / mid→leaf **15**. E1b default call attach is grafted C `OP_ENTERSUB` (emit after INIT; `$^P` 0x01 off; `DB::sub` stub). Rollback to wrap: `NYTPROF=wrap=1` (`use_db_sub=1` synonym). G05 landed **options/`format=v6` tests**: unknown + `format=dual` fail-closed; D1-B `format=v6` fail-closed (`v6_collect` rebuild text, no `NYTPROF6`); D1-A `xs-nytprof-v6` writes `NYTPROF6`; default/`format=v5` still 15/3/15. Dual-path remains **oracle-primary** (`legacy_only_smoke.sh` first half). `collection_default` stays **v5**. G06 landed **fork/`addpid` MVP**: live `fork` + `addpid=1` writes parent `NYTProf 5` and `<file>.<childpid>` `NYTProf 5` ([g06_fork_addpid_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g06_fork_addpid_smoke.sh)). Dual-path remains **oracle-primary**. `collection_default` stays **v5**. **Residuals:** mid-deflate continue-in-child, TEST-018, `sigexit`, DI-03 E2–E4, live emit-after-INIT di02 **21** vs oracle start=begin **27**.

| Item | Honesty |
|------|---------|
| `DROP-IN-DOD-V0` | **docs-landed** — [DROP_IN_DOD_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md) |
| Graft annex | A.6 **G02 landed**; A.4 G03a **load** + G03b **stmt emit** + G03c **sub emit** + G03d **meta/finalize** + G03e **compress landed**; A.5 fake-clock mini landed; mid-deflate fork residual — [product-xs-graft-annex-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-xs-graft-annex-v0.md) |
| `G02-V5-PRODUCT-LINK` | **done (scaffold)** — `libnytp_sink_v5.a` + CollectorBootstrap; [g02_v5_product_link_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g02_v5_product_link_smoke.sh) |
| `G03A-LOAD-ONLY` | **done** — product `-d:NYTProf` loads; no `nytprof.out` on trivial `-e` |
| `G03B-STMT-EMIT` | **done** — `nytp_emit_*` statement path + fake-clock mini; [g03b_stmt_emit_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03b_stmt_emit_smoke.sh) |
| `G03C-SUB-EMIT` | **done** — `nytp_emit_sub_*` call path; dump SUB_ENTRY / SUB_RETURN; [g03c_sub_emit_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03c_sub_emit_smoke.sh) |
| `G03D-META-EMIT` | **done** — `nytp_emit_*` meta/finalize; dump ATTRIBUTE / OPTION / NEW_FID / SRC_LINE / SUB_INFO / PID_START / PID_END; [g03d_meta_emit_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03d_meta_emit_smoke.sh) |
| `G03E-COMPRESS-EMIT` | **done** — `nytp_emit_start_deflate`; dump/verify inflate recovers a post-deflate event; mid-deflate fork residual; [g03e_compress_emit_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g03e_compress_emit_smoke.sh) |
| Product `compress` / `durable` | Omitted `compress` ⇒ zlib **level 6** (6.15 `HAS_ZLIB`). `compress=0` disables. `compress=1..9` is that zlib level. `durable=1` (default **0**) keeps live RAM uncompressed and publishes complete-record snapshots (`tmp`+`fsync`+`rename`; `z`+`Z_FINISH` copy when compress≠0). Mid-run `kill -9` after a seal is dumpable; torn live `z` is never verify `OK:`. [di_durable_kill_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/di_durable_kill_smoke.sh). **Not** default-on durable; **not** `aggregate=1` (ADR-0013 proposed). |
| `PRODUCT-XS-ATTACH-MVP` / collection attach | **done (MVP)** — [g04_v5_parity_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g04_v5_parity_smoke.sh) leaf **15** / mid **3** / edge **15**; E1b default `OP_ENTERSUB` (`$^P` 0x01 off); **E2** `OP_GOTO` ([g18_goto_sub_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g18_goto_sub_smoke.sh)); wrap remains `wrap=1`. **Not** leave default 1 / default-on full slowops |
| `DI-03` opcode / `entersub` | **in progress, not done** — E1b default: omit `entersub` installs `OP_ENTERSUB` (emit after INIT; `$^P` 0x01 off; `DB::sub` stub). **E2:** `OP_GOTO`. **E3:** opt-in `leave=1` (default 0). **E4:** opt-in `slowops=full`/`=3` (`=2` stays PRINT/MATCH). Wrap list remains `wrap=1` / `use_db_sub=1` only. Escape / rollback: `NYTPROF=wrap=1`. Residual: live di02 **21** vs oracle start=begin **27**. **Not** DI-05 / GA-candidate / certified perf. Plan: [DI03_OPCODE_ENTERSUB_ATTACH_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/DI03_OPCODE_ENTERSUB_ATTACH_v0.md) |
| Product `use_db_sub=1` | **Intentional fork** of the 6.15 option. On product it is a **synonym for `wrap=1`** (Perl `DB::sub` wrap escape). It is **not** 6.15 stmt `DB::DB` (`init_profiler` `if (opt_use_db_sub)` skips NEXTSTATE/DBSTATE while opcode `ENTERSUB` stays on). Do not copy that 6.15 meaning. `wrap=1` wins over `entersub=1`. Rollback: `NYTPROF=wrap=1`. |
| `PRODUCT-FORK-ADDPID-MVP` | **done (MVP)** — [g06_fork_addpid_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g06_fork_addpid_smoke.sh) parent + `<file>.<pid>` `NYTProf 5`; not TEST-018 / mid-deflate-in-child |
| `product_attach_smoke.sh` | G03a load via real `-d:NYTProf` (no `file=`); not in `offline_gate.sh` |
| `product_legacy_smoke.sh` | **I01 done (MVP)** — cargo-free prefix install + live attach 15/3/15; not in `offline_gate.sh`; S2 dual_path not claimed |
| `I02-MAKEMAKER-NATIVE` | **done (MVP)** — [i02_makemaker_native_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/i02_makemaker_native_smoke.sh): `NYTPROF_NATIVE=1` fail-closed without cargo; `auto`/`=0` cargo-free; cargo-present `nytprof-cli` 15/3/15 |
| `I03-DIST-SCRIPTS` | **done (MVP)** — prefix/dev only ([install_product_scripts.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/install_product_scripts.sh) + [i03_dist_scripts_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/i03_dist_scripts_smoke.sh)). **Not** in the `perl-NYTProfM` module RPM (collection-only + `nytprofm-cli`). **Not** 6.15 nytprofhtml DOM / S2 |
| `J01-CPAN-HYGIENE` | **done (MVP)** — [j01_cpan_hygiene_smoke.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/j01_cpan_hygiene_smoke.sh): `Makefile.PL` MYMETA `NYTProfM` **6.15**; MANIFEST excludes `baseline/` `target/` `prefix/`. **Not** TRIAL/PAUSE |
| `MIG01-MIGRATION-GUIDE` | **done (docs)** — [MIGRATION_DROP_IN_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/MIGRATION_DROP_IN_v0.md) Option B `perl-NYTProfM` / `-d:NYTProfM` |
| `K03-PREBUILT-CLI-ADR` | **done (docs)** — [ADR-0010](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0010-signed-ci-prebuilt-native-cli.md); `EL8-RPM-TOOLS` residual |
| `M01-HTML-JS-WAIVE` | **done (docs)** — Shared JS/tablesorter **WAIVE** for GA-candidate; jquery not shipped |
| `CPAN-TRIAL-READY` | **done (notes-ready / MVP)** — [RELEASE_NOTES_CPAN_TRIAL_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_CPAN_TRIAL_v0.md); **not** PAUSE uploaded |
| `EL8-RPM-MODULE` | **done (MVP)** — [perl-NYTProfM.spec](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/perl-NYTProfM.spec) D1-B **6.15**; **not** mock-certified / tools RPM |
| `EL8-RPM-TOOLS` | **done (MVP)** — [nytprof-cli.spec](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/nytprof-cli.spec) companion; **not** signed-pipeline complete |
| `P01-GA-CANDIDATE` | **done (MVP)** — [RELEASE_NOTES_GA_CANDIDATE_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_GA_CANDIDATE_v0.md); Rocky default **D1-B only**; **not** SEC-012 complete / R3–R4 |
| `P02-SEC-CUT` | **done (MVP / checklist / job)** — [SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md); [sec002_continuous_fuzz_mvp.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/sec002_continuous_fuzz_mvp.sh) wraps `selftest_security_fuzz.sh`; **not** independent sign-off / GA marketing / S2 |
| `BUILD-003-FULL` | **residual** |

### 7.1 Full R1 MVP product cut (PR-A10)

Per [ADR-0003](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) and residual matrix § **Full R1 ready**:

| Area | Full R1 cut status | Do not claim |
|------|--------------------|--------------|
| **FFI (OQ-2 / A05)** | **closed (MVP)** — `crates/nytprof-ffi` open/query/close | full RUST-010 (batch APIs, BUILD-007, production dylib install, ABI freeze) |
| **XS Data/ReadStream (OQ-2 / A06)** | **closed (MVP)** — binary via native dump → Jsonl* | COMPAT-007 bless-array; pure-XS wire decode |
| **HTML A01/A02** | **closed (MVP)** — shared CSS/structure; `index-subs-excl.html` | full oracle DOM |
| **HTML Shared JS** | **WAIVE** — **PR-M01** / Q4 user-final; A01 CSS-only; tablesorter/jquery **not** shipped (not a remaining CLOSE requirement) | Shared JS/tablesorter as native-ready |
| **HTML A03 flame** | **residual open** | native flame SVG / site flame inputs as ready |
| **HTML WAIVE classes** | **waived** (residual-honest) | Graphviz, treemap, block/sub pages, oracle naming as native-ready |
| **Multi-OS (A07)** | **closed (MVP)** — GHA Linux + macOS | full multi-Perl/rustc/Windows certification |
| **Packaging (A08)** | **closed (depth MVP)** | full BUILD-003 XS CPAN dual-build |
| **Public perf (A09)** | **WAIVED** | public SLOs / “% faster” / certified P3–P4 |
| **COL-007 / wire freeze / CLI v6 default** | **OUT-OF-R1** | product v6 writer or default v6 CLI report |
| **R3/R4 defaults** | **OUT-OF-R1** | product default engine/format **runtime** flip |
| **R3 field-window pack (PR-D01)** | **instrumentation only** — collect local evidence packs; **does not** flip defaults | treating packs as flip authorization |
| **R3 default policy (PR-D02 / ADR-0005)** | **policy accepted**; flip **not executed** — [R3_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) | claiming R3 complete or changing omitted-engine default without accepted promote report |
| **R5 retirement governance (PR-F01 / ADR-0009)** | **governance accepted**; **no component retired** — [R5_RETIREMENT_REVIEW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R5_RETIREMENT_REVIEW.md) | claiming legacy removed/deprecated, or treating R3/R4 success as retirement authorization |

Release notes: [RELEASE_NOTES_R1.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R1.md).

### 7.1.1 R3 field window + default policy (post-cut; runtime flip gated)

After the full R1 MVP cut, operators may collect **local** field evidence for `engine=auto` product default promotion **without** changing defaults:

```sh
./scripts/field/r3_field_window_collect.sh --out /tmp/r3-field-pack
./scripts/field/r3_field_window_smoke.sh   # fixture layout + honesty smoke
```

| Doc / tool | Path |
|------------|------|
| Guide | [R3_FIELD_WINDOW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md) |
| Report template | [templates/R3_FIELD_WINDOW_REPORT.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R3_FIELD_WINDOW_REPORT.md) |
| Pack schema | [schemas/r3-field-window-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r3-field-window-mvp-v0.md) |
| Promotion policy ADR | [0005-r3-engine-auto-default-promotion.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) (**accepted policy**; flip not executed) |
| Flip + rollback checklist | [R3_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_DEFAULT_FLIP.md) |

**Not** charter R3 completion. Product default remains omit→`native` until an accepted field report recommends **Promote** and the flip checklist is executed. Operator escape hatch (always): `--engine=legacy` / `NYTPROF_ENGINE=legacy`.

### 7.2 Offline R0 / R1-preview residual honesty (`R1-HONESTY-SYNC`)

Do **not** over-claim preview surfaces as complete product residual-table close. Preview **does** include the Phase A MVPs above when cargo/native is present.

| Residual | Notes |
|----------|--------|
| **No production FFI / XS Data** | No RUST-010 cdylib ABI; no PERL-004 XS ReadStream over binary profiles; no PERL-005 bless-array Data materializer. Preview = CLI subprocess + pure-Perl JsonlData from dump JSONL (incl. SUB_ENTRY multiplicity only). |
| **No full nytprofhtml DOM** | Native HTML is MVP summary / multi-file site only — not oracle DOM, CSS/JS, tablesorter, flame SVG, Graphviz. See HTML residual inventory. |
| **COL-007 E3-EVENT + wire freeze done; E3-mixed residual** | Wire freeze **done** (ADR-0006 major=6 IDs + golden vectors); C v6 writer product E3-EVENT **done** (COL-007 at PR-B09); COL-009 reaffirms C baseline (ADR-0007 / PR-B13); E3-mixed residual; COL-008 deferred non-baseline. Collector default remains 6.15 oracle / v5. **Format stack (IDs frozen ADR-0006):** fixed-header + chunk-frame + ULEB128 + ZigZag signed + length-prefixed string + header TLV + multi-TLV region + file-prefix + prefix+chunk stream + event-body (incl. TIME_BLOCK/SUB_ENTRY/SUB_RETURN/SUB_INFO/SRC_LINE/NEW_FID/PID_START/PID_END/SUB_CALLERS/DISCOUNT/ATTRIBUTE/OPTION/COMMENT/START_DEFLATE/VERSION/dual-output-sequence/mid-stream-codec-switch/auto-emit-VERSION/known-key-attr-option/known-key-expand-from-fixtures/unknown-optional-skip/string-dictionary/site-delta/TIME_LINE_RUN/TIME_BLOCK_RUN/event-seq-number/site-delta-seq-compose/string-dict-site-delta-seq-compose/multi-chunk-packing-continuity/string-dict-multi-chunk-site-delta-seq-packing) + mini-profile + multi-chunk EVENT + SOURCE/INDEX/SUMMARY/FOOTER bodies + CRC32 optional verify + ZLIB/ZSTD/LZ4 payload codecs + compressed multi-codec mini-profile + multi-chunk compressed EVENT + compressed multi-kind mixed + per-kind codecs + multi-chunk EVENT under mixed + multi-chunk SOURCE + multi-chunk INDEX + multi-chunk SUMMARY + mid-record EVENT/SOURCE/INDEX/SUMMARY span + decoded-chunk + decoded-stream + decoded-EVENT/SOURCE/INDEX/SUMMARY + decoded-mixed + multi-chunk decoded-mixed + mid-record decoded-mixed (EVENT+SOURCE+INDEX+SUMMARY + concurrent multi-kind) always-inflate (`FMT-V6-HEADER-*` / `FMT-V6-CHUNK-*` / `FMT-V6-VARINT-*` / `FMT-V6-SVARINT-*` / `FMT-V6-STRING-*` / `FMT-V6-TLV-*` / `FMT-V6-TLV-REGION-*` / `FMT-V6-FILE-PREFIX-*` / `FMT-V6-PREFIX-CHUNK-STREAM-*` / `FMT-V6-EVENT-BODY-*` / `FMT-V6-MINI-PROFILE-*` / `FMT-V6-MULTI-CHUNK-EVENT-*` / `FMT-V6-SOURCE-BODY-*` / `FMT-V6-INDEX-BODY-*` / `FMT-V6-SUMMARY-BODY-*` / `FMT-V6-FOOTER-BODY-*` / `FMT-V6-CRC-*` / `FMT-V6-PAYLOAD-ZLIB-*` / `FMT-V6-PAYLOAD-ZSTD-*` / `FMT-V6-PAYLOAD-LZ4-*` / `FMT-V6-COMPRESSED-PROFILE-*` / `FMT-V6-MULTI-CHUNK-COMPRESSED-*` / `FMT-V6-COMPRESSED-MIXED-*` / `FMT-V6-PER-KIND-CODEC-*` / `FMT-V6-MULTI-CHUNK-KIND-*` / `FMT-V6-MULTI-CHUNK-SOURCE-*` / `FMT-V6-MULTI-CHUNK-INDEX-*` / `FMT-V6-MULTI-CHUNK-SUMMARY-*` / `FMT-V6-MID-RECORD-SPAN-*` / `FMT-V6-MID-RECORD-SOURCE-*` / `FMT-V6-MID-RECORD-INDEX-*` / `FMT-V6-MID-RECORD-SUMMARY-*` / `FMT-V6-DECODED-CHUNK-*` / `FMT-V6-DECODED-STREAM-*` / `FMT-V6-DECODED-EVENT-*` / `FMT-V6-DECODED-SOURCE-*` / `FMT-V6-DECODED-INDEX-*` / `FMT-V6-DECODED-SUMMARY-*` / `FMT-V6-DECODED-MIXED-*` / `FMT-V6-DECODED-MIXED-MULTI-CHUNK-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-SOURCE-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-INDEX-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-SUMMARY-*` / `FMT-V6-DECODED-MIXED-MID-RECORD-CONCURRENT-*` / `FMT-V6-EVENT-BODY-TIME-BLOCK-SUB-ENTRY-*` / `FMT-V6-EVENT-BODY-SUB-RETURN-SUB-INFO-*` / `FMT-V6-EVENT-BODY-SRC-LINE-NEW-FID-*` / `FMT-V6-EVENT-BODY-PID-START-END-*` / `FMT-V6-EVENT-BODY-SUB-CALLERS-DISCOUNT-*` / `FMT-V6-EVENT-BODY-ATTRIBUTE-OPTION-*` / `FMT-V6-EVENT-BODY-COMMENT-*` / `FMT-V6-EVENT-BODY-START-DEFLATE-*` / `FMT-V6-EVENT-BODY-VERSION-*`; crate `nytprof-format-v6`). Default `parse_chunk_frame` stays **non-inflating**. |
| **No performance claims** | Public P1–P4 cert **waived** until R2-stable BENCH gates green. Methodology + light harness only (`docs/BENCH_NOTES.md`, `tools/bench/light_bench.sh` — `size` / `collector_micro` + offline proxies). No public SLOs or “% faster”. |
| **No full MakeMaker XS CPAN dual-build** | Candidate `Makefile.PL` facade only (**BUILD-MAKEMAKER-OPT**), not BUILD-003 full. |
| **No multi-OS CI matrix** | Single-host `offline_gate.sh` only (**BUILD-006** open). |
| **No product default flip** | Native remains opt-in; Perl `engine=auto` is facade behavior, not charter R3 product default. |
| **Convert / merge / salvage (R2-stable tooling)** | Capability `convert`/`merge`/`repack`/`salvage` **true** on integrated R2-stable stack (PR-C01/C02). |
| **FFI / XS Data residual** | **FFI MVP (PR-A05 / `FFI-CDYLIB-MVP`):** `crates/nytprof-ffi` cdylib open/query/close C ABI over `ProfileModel` is shipped ([`docs/schemas/ffi-cdylib-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/ffi-cdylib-mvp-v0.md)); panic-safe; dual-path still works **without** loading the dylib. **XS Data/ReadStream MVP (PR-A06 / `PERL-XS-DATA-READSTREAM-MVP`):** product `Devel::NYTProf::Data` + `::ReadStream` open **binary** profiles via native dump → JsonlData/JsonlReadStream ([`docs/schemas/perl-xs-data-readstream-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-xs-data-readstream-mvp-v0.md)); smoke `./scripts/packaging/perl_xs_data_readstream_smoke.sh`; `claims_compat007_shapes=0`. **Still residual:** full RUST-010; full PERL-004 pure-XS; full PERL-005 COMPAT-007. Preview primary = CLI subprocess + pure-Perl Jsonl* + product thin binary facades. |
| **No full nytprofhtml DOM** | Native HTML is MVP summary / multi-file site (A01 CSS + A02 excl index) — not oracle DOM, Shared JS/tablesorter (**WAIVE** for GA-candidate — **PR-M01** / Q4; not shipped), flame SVG (A03 open), Graphviz. See HTML residual inventory + **ADR-0003** per-class map (M01 amendment). |
| **No v6 / COL-007** | No v6 wire freeze; C v6 writer (**COL-007**) deferred; COL-008 non-baseline. Collector remains 6.15 oracle / v5. Provisional `nytprof-format-v6` preflight only — **not** product writer / CLI v6 default. |
| **No performance claims** | Public SLOs **WAIVED** (`docs/BENCH_NOTES.md`, `tools/bench/light_bench.sh` only). |
| **No full MakeMaker XS CPAN dual-build** | **BUILD-MAKEMAKER-OPT** + **BUILD-003-DEPTH** partial only — **not** BUILD-003 full. |
| **No full multi-OS CI matrix** | **BUILD-006-MVP** lands GHA **ubuntu-latest** (`linux-x86_64`) + **macos-latest** (`macos-arm64`) via `./scripts/ci/matrix_gate.sh`. Full multi-Perl / multi-rustc / Windows / dashboard remains residual. |
| **No product default flip (runtime)** | Native remains product opt-in default (omit→`native`). Perl explicit `engine=auto` is facade prefer-native/fallback, not charter R3 completion. [ADR-0005](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0005-r3-engine-auto-default-promotion.md) freezes promotion criteria; field packs ([R3_FIELD_WINDOW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md)) are evidence-only (`no_default_flip=true`). |

Advertised surfaces **do** include native aggregates JSON (incl. **SUB_ENTRY**, stream/PID, A2 `time_block_events` **0**/**916**, A9/A8 samples, ATTRIBUTE/OPTION/file samples, blocks A4/A4b greppable ints), pure-Perl query JSON, **JSON report incomplete fail-closed**, **native↔query JSON cross-parity**, pure-Perl **SUB_ENTRY** multiplicity, **FFI open/query/close MVP**, **product Data/ReadStream thin binary path MVP**, multi-OS matrix MVP, and packaging depth — **without** promoting those to full residual-table close / CPAN / full RUST-010 / full COMPAT-007 / full oracle DOM / COL-007.

Full residual table: [R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) § Residual for full R1 + § Full R1 ready.  
Policy ADR: [0003-r1-full-residual-policy.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md).

### 7b. R2-preview honesty (`R2-PREVIEW-READINESS-CUT`)

R2-preview is an **opt-in** horizon on top of the R0/R1-preview stack. Authoritative release notes: [RELEASE_NOTES_R2_PREVIEW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_PREVIEW.md). Dual-equality checklist: [DUAL_EQUALITY_READINESS_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md).

| Claim under R2-preview | Status |
|------------------------|--------|
| Offline CLI on **v6** files (report/html/csv/… via magic) | **ready** (PR-B12 E5) |
| Capability `v6_decode` / `v6_report` | **true** |
| Capability `convert` / `merge` | **false** (residual) |
| Collection default | **v5** (`collection_default: v5`) — **not** R4 |
| COL-007 C writer E3-EVENT | **done** (PR-B09 — not packaging) |
| COL-009 production writer backend | **C** reaffirmed (ADR-0007 / PR-B13) |
| Wire freeze ADR-0006 | **done** |
| E4 product smoke (offline_gate step 12) | **ready** on dual-sink scaled pairs |
| Dual-path legacy without Cargo | **unchanged** |
| R2-stable / R3 / R4 / convert-merge product | **not claimed** |
| Public P1/P2 performance certification | **waived** (methodology only; PR-C04) |

```sh
# Capability honesty (E5)
./prefix/bin/nytprof-cli capability
# Expect: v6_decode: yes; v6_report: yes; convert: yes; merge: yes; repack: yes; salvage: yes; collection_default: v5

# E4 product smoke when native present
./scripts/packaging/e4_v5_v6_semantic_smoke.sh --full
```

---

## 8. Golden fixture checks

Frozen semantic counts (counts exact; tick/time strings only under COMPAT-003):


### 7c. R2-stable honesty (`R2-STABLE-READINESS-CUT`)

R2-stable is the Phase C certification cut on the **integrated** stack (PR-C01..C04 + R2-preview base). Authoritative release notes: [RELEASE_NOTES_R2_STABLE.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R2_STABLE.md). Dual-equality checklist: [DUAL_EQUALITY_READINESS_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md).

| Claim under R2-stable | Status |
|-----------------------|--------|
| v6 offline CLI (E5) opt-in read/report | **yes** |
| Strict convert + merge/repack/salvage | **yes** (capability true) |
| COL-015 fork/PID MVP | **yes** (stress suite; oracle TEST-018 residual) |
| SEC-FUZZ offline package | **yes** (P02 SEC-002 **job MVP**; not cargo-fuzz/AFL) |
| P1/P2 public performance SLOs | **waived** (methodology + light harness only) |
| Collection / engine defaults (R3/R4) | **runtime not claimed** (`collection_default: v5`; R4 policy ADR-0008 / flip checklist only) |
| E3-mixed / full oracle E4 | **residual** |
| CPAN upload | **not claimed** |

### 7c.1 R4 field window + default policy (post R2-stable; runtime flip gated)

After the R2-stable cut, operators may collect **local** field evidence for `format=v6` product default promotion **without** changing defaults:

```sh
./scripts/field/r4_field_window_collect.sh --out /tmp/r4-field-pack
./scripts/field/r4_field_window_smoke.sh   # dual-sink layout + honesty smoke
```

| Doc / tool | Path |
|------------|------|
| Guide | [R4_FIELD_WINDOW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_FIELD_WINDOW.md) |
| Report template | [templates/R4_FIELD_WINDOW_REPORT.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R4_FIELD_WINDOW_REPORT.md) |
| Pack schema | [schemas/r4-field-window-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r4-field-window-mvp-v0.md) |
| Promotion policy ADR | [0008-r4-v6-output-default-promotion.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0008-r4-v6-output-default-promotion.md) (**accepted policy**; flip not executed) |
| Flip + rollback checklist | [R4_DEFAULT_FLIP.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_DEFAULT_FLIP.md) |

**Not** charter R4 completion. Product collection default remains **v5** (`collection_default: v5`) until an accepted field report recommends **Promote** and the flip checklist is executed. Packs must keep `collection_default: v5` and `no_default_flip: true`. Operator escape hatch (always retained after any future flip): force `format=v5` / convert `--to=v5`.

### 7c.2 R5 retirement governance (optional; never automatic)

Charter **R5** is **governance only** unless a later **per-component** ADR executes deprecation or removal:

| Doc | Path |
|-----|------|
| Governance ADR | [0009-r5-legacy-retirement-governance.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0009-r5-legacy-retirement-governance.md) (**accepted policy**; **no component retired**) |
| Review + retirement checklist | [R5_RETIREMENT_REVIEW.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/R5_RETIREMENT_REVIEW.md) |

**Binding residual honesty:**

- R3/R4 product default flips (when/if executed) **retain** operator escapes (`--engine=legacy`, `format=v5`) and **do not** authorize path removal.
- **No** legacy report engine, v5 reader/writer, dual-path install, or oracle CLI is removed or deprecation-timed by PR-F01.
- **Absence of retirement is valid success** for the modernization program.
- Any future retirement is **per-component** under REL-012 + a dedicated component ADR — never silent, never bulk-auto.

### 7c.3 Rocky 8 Docker profile demo (testdrive HTML)

End-to-end **unsigned testdrive** on `rockylinux:8`: install `perl-NYTProfM`, download [ack v3](https://beyondgrep.com/) plus a public-domain text corpus, run `perl -d:NYTProfM` on a **core-only** ~1-minute text analyzer, and write `nytprof.out` + native `nytprofm-cli html` into a host directory.

**PR-7 landed:** `use Getopt::Long` / Exporter / Time::HiRes compile under product `-d:NYTProfM` (`INIT` `$DB::single` + `goto &$raw` for those modules; smoke [`g07_getopt_compile_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g07_getopt_compile_smoke.sh)). **PR-10:** do not wrap `CORE::require`; preload BHES / Variable::Magic / namespace::* and `CvNODEBUG` their CVs before `$^P` 0x01 — `DB::sub` during `on_scope_end` broke `DateTime::Duration` (`Can't use string ("#pod\n") as an ARRAY ref` at `B/Hooks/EndOfScope/XS.pm`). Smoke [`g10_datetime_hints_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g10_datetime_hints_smoke.sh). **PR-11:** `DB::nodebug_stash` must not `GvCV` a GP-less stash GV (`Segmentation fault (core dumped)` on the v0.2.12 walker). Smoke [`g11_nodebug_stash_nogp_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g11_nodebug_stash_nogp_smoke.sh). **PR-12:** goto `Memoize::` — `memoize('fn')` uses `caller` as the package (`Cannot operate on nonexistent function` when wrap makes `caller` `DB`). Smoke [`g12_memoize_caller_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g12_memoize_caller_smoke.sh). **PR-13:** do not `eval` around `&$raw` — loggers that use `caller` reported `NYTProfM.pm:308`. Smoke [`g13_logger_caller_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g13_logger_caller_smoke.sh). **PR-14:** exclusive = incl − child inclusive; `stmts=0` skips TIME_LINE (smaller `nytprof.out`). Smoke [`g14_nested_excl_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g14_nested_excl_smoke.sh). **PR-15:** default `stmts=1` TIME_LINE from C `OP_DBSTATE` (`$DB::single=0`; Perl `DB::DB` is not entered). Smoke [`g15_dbstate_timeline_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g15_dbstate_timeline_smoke.sh). **PR-16 / E1b:** default call attach is opcode `OP_ENTERSUB` (g17; `$^P` 0x01 off; `DB::sub` stub). `wrap_push`/`wrap_pop` and `NYTPROF_WRAP_SLOW` run only under `wrap=1` / `use_db_sub=1` / `entersub=0`. Smoke [`g16_wrap_enter_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g16_wrap_enter_smoke.sh) + [`g17_entersub_attach_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/g17_entersub_attach_smoke.sh). **Not** E2 `OP_GOTO` / E3 leave / E4 full `slowops.h` / stock 6.15 XS. The Rocky demo still profiles the **core-only** scanner — **ack is not the profiled process** until a later field retry. Historical fail-closed notes: [failed-attempts.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/agent-notes/failed-attempts.md) (`rocky8-profile-ack-d-nytprofm`).

```sh
./scripts/field/rocky8_docker_profile_demo.sh
# default:
#   ~/Downloads/nytprof-rocky8-demo/html/index.html
./scripts/field/rocky8_docker_profile_demo.sh --out "$HOME/Downloads/nytprof-rocky8-demo"
```

Uses the local [`dist/el8/perl-NYTProfM-6.15-3.el8.x86_64.rpm`](https://github.com/hilather/nytprof-modernization/blob/main/dist/el8/) when present; otherwise downloads the GitHub Release asset. **Not** mock-certified, **not** COPR, **not** a public performance claim. HTML is `nytprofm-cli html` (module RPM is collection-only; no stock `nytprofhtml` overwrite). Not oracle DOM / tablesorter.

Script: [rocky8_docker_profile_demo.sh](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/rocky8_docker_profile_demo.sh)

**Integration smoke** (host scanner checks always; `rockylinux:8` + RPM + `-d:NYTProfM` + `nytprofm-cli html` when docker is usable; honest SKIP without docker or without `nytprofm-cli`). **Not** part of `offline_gate.sh`.

```sh
./scripts/field/rocky8_docker_profile_smoke.sh
# after perl Makefile.PL:
# make rocky8-docker-lab-smoke
```

Schema: [rocky8-docker-profile-lab-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/rocky8-docker-profile-lab-mvp-v0.md). GHA job **Rocky 8 Docker lab** on `ubuntu-latest` (timeout ≥45). Native operator HTML v2 ([ADR-0012](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0012-native-operator-html-v2.md)): oracle-shaped chrome/IA via modern CSS + `nytprof-sort.js`. jquery/tablesorter remain **WAIVE** (M01). Dual lab: `--engine native|oracle|both` (smoke uses `--both`; `$OUT/html` is a symlink to `native/html`). Both engines share `--seconds` and the same corpus (apples-to-apples). Host-side paired reports: [`compare_oracle_native_reports.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/compare_oracle_native_reports.sh). Oracle container builds 6.15 from the committed archive with **no** `crates/` on `PERL5LIB`.

### 7c.4 Rocky 8 Docker Rex / DateTime attach lab

End-to-end **complex CPAN app** on `rockylinux:8`: build **in-tree** `xs-nytprof`, `cpanm` Rex + DateTime + YAML, run `perl -d:NYTProfM run_lab.pl` (`use Rex` + DateTime + YAML; `rex -T` is an unprofiled load canary), write `nytprof.out` + host `nytprof-cli html --no-flame` into `~/Downloads/nytprof-rex-demo/`.

This is the attach net for module graphs the core-only scanner never loads (`DateTime::Duration` / `namespace::autoclean` / `B::Hooks::EndOfScope::XS`, Getopt, Rex). Fail-closed on `%^H` ARRAY-ref / EndOfScope / `$VERSION` abort.

```sh
./scripts/field/complex_app_docker_profile.sh
# default native:
#   ~/Downloads/nytprof-rex-demo/html/index.html
./scripts/field/complex_app_docker_profile.sh --engine both
# same driver + seconds: native NYTProfM and 6.15 -d:NYTProf
# COMPARE.txt is attach survival only — not exclusive-time match
./scripts/field/complex_app_docker_profile_smoke.sh
# make complex-app-docker-lab-smoke
```

Schema: [complex-app-docker-profile-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/complex-app-docker-profile-mvp-v0.md). GHA job **Complex-app Docker lab (Rex)**. **Not** `offline_gate`. **Not** a public perf claim.

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
| `R1-HONESTY-SYNC` | **done** | matrix + this runbook advertise **NATIVE-QUERY-JSON-CROSS** / **CROSS-EXPAND** / **CROSS-BLOCKS** / **CROSS-META** / **CROSS-TIMEBLOCK** / **CROSS-COUNTS**, **JSON-EVENT-COUNTS-MVP**, **JSON-FILE-BASENAME-MVP** (absolute `file_1` volatile; basename greppable stable sample), **JSON-TIME-BLOCK-MVP**, **JSON-REPORT-INCOMPLETE-FAILCLOSED**, **JSON-SUB-ENTRY-MVP**, **JSON-BLOCKS-MVP**, **JSON-META-FILES-MVP**, + **PERL-SUB-ENTRY-JSONL** while listing full-R1 residuals (no production FFI/XS Data, no full nytprofhtml DOM, COL-007 E3-EVENT done with E3-mixed residual; no multi-OS CI, no perf claims). |
| `R1-RESIDUAL-POLICY-ADR` | **done** (PR-A04 policy) | [ADR-0003](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) — full R1 CLOSE/WAIVE map; **OQ-2** FFI/XS → CLOSE PR-A05/A06 (not waive); HTML per-class map. Preview residual honesty unchanged until close PRs land. |
| `FFI-CDYLIB-MVP` | **done** (MVP; PR-A05) | [`docs/schemas/ffi-cdylib-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/ffi-cdylib-mvp-v0.md); `crates/nytprof-ffi` + `include/nytprof_ffi.h`; `cargo test -p nytprof-ffi` (offline_gate step 1). OQ-2 product path; full RUST-010 residual remains. **Before COL-007.** |
| `PERL-XS-DATA-READSTREAM-MVP` | **done** (MVP; PR-A06) | [`docs/schemas/perl-xs-data-readstream-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-xs-data-readstream-mvp-v0.md); `Devel::NYTProf::Data` + `::ReadStream` thin binary path; `./scripts/packaging/perl_xs_data_readstream_smoke.sh`. OQ-2 product path; full PERL-004/005 / COMPAT-007 residual remains. **Before COL-007.** |
| `R1-HONESTY-SYNC` | **done** | matrix + this runbook advertise preview + Phase A product MVPs (FFI/XS Data, BUILD-006-MVP, BUILD-003-DEPTH, HTML A01/A02) while listing beyond-MVP residuals (full RUST-010 / PERL-004/005 / COMPAT-007; no full oracle DOM; A03 flame open; no v6/COL-007 product writer; no full multi-OS / full BUILD-003; public perf **WAIVED**). **Before COL-007.** |
| `R1-FULL-READINESS-CUT` | **done** (PR-A10) | Full R1 **MVP product scope** honesty in matrix § Full R1 ready + this §7 + [RELEASE_NOTES_R1.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/RELEASE_NOTES_R1.md). Does **not** claim COL-007, wire freeze, CLI v6 default, full oracle DOM, full BUILD-003, full RUST-010 beyond MVP. **Before COL-007.** |
| `BUILD-006-MVP` | **done** | GHA `linux-x86_64` + `macos-arm64` + `./scripts/ci/matrix_gate.sh`; portable oracle hash; honest skips preserved; not full BUILD-006. **Before COL-007.** |
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
| **No independent SEC-012 / no full continuous fuzz** | P02 landed a **checklist** ([`SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md)) and a **job MVP** (`scripts/ci/sec002_continuous_fuzz_mvp.sh` + `.github/workflows/sec002-fuzz-mvp.yml`) that wraps shipped `selftest_security_fuzz.sh` / `decode_fuzz`. **Not** independent sign-off, **not** cargo-fuzz/AFL/sanitizer matrix, **not** GA marketing. Offline package: [`SECURITY_FUZZ_HARDENING_PACKAGE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/SECURITY_FUZZ_HARDENING_PACKAGE_v0.md). |
| — | `./scripts/ci/sec002_continuous_fuzz_mvp.sh` | **P02 SEC-002 job MVP** (not offline_gate step 12; that step is E4). Wrapper: honest `SKIP:` without cargo. When cargo present, `exec`s shipped `./tools/oracle/selftest_security_fuzz.sh` (`decode_fuzz`; cargo-required if invoked directly). **Not** full SEC-002 cargo-fuzz; COL-015 residual. |
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
| `COL-001-SINK-MVP` | **done (scaffold)** | Overlay `collector/` semantic sink + counting + v5 wire; smoke offline_gate step 10; **not** live hooks |
| `COL-014-DUAL-SINK-MVP` | **done (test/dev-only)** | Same-run dual fan-out v5+v6 (OQ-4 not product UX); `test_dual_sink`; schema `collector-dual-sink-mvp-v0.md`; full oracle dual residual; feeds E4-v0 model pairs |
| `E4-V0-MODEL-SEMANTIC-MVP` | **done** | E4-v0 model-level v5↔v6 aggregates on dual-sink pairs; `e4_v0_*` tests + `e4_v5_v6_semantic_smoke.sh --model-only`; full oracle residual (TEST-008) |
| `E4-PRODUCT-CLI-SMOKE-MVP` | **done** | E4 product CLI smoke: real CLIs on v5+v6 dual pairs; `e4_v5_v6_semantic_smoke.sh --full`; offline_gate step 12 when native; schema `e4-product-cli-smoke-mvp-v0.md` |
| `CLI-E5-V6-OPT-IN-MVP` | **done** | CLI E5 report/html/csv/folded/callgrind on v6; capability `v6_decode`/`v6_report`; no convert/merge claim; `collection_default: v5`; schema `cli-e5-v6-opt-in-mvp-v0.md` |
| `COL-002-LIFECYCLE-MVP` | **done (scaffold)** | Explicit sink lifecycle + emit gates; **not** COL-015 full fork/signal matrix |
| `COL-003-SEQ-MVP` | **done (scaffold)** | Internal gapless logical seq; not on default v5 wire |
| `COL-004-FAST-PATH-MVP` | **done (scaffold)** | No-alloc TIME_LINE/TIME_BLOCK batch append + `nytp_fast_emit_*`; light microbench engineering only — **not** BENCH cert |
| `COL-005-BATCH-MVP` | **done (scaffold)** | Bounded event batch + arena; order under cap 1..64; SV lifetime; emergency oversized; flush-discount residual |
| `COL-006-V5-WIRE-MVP` | **done (scaffold)** | Real v5 wire via sink + zlib; mini samples Rust-decoder-accepted; **not** full oracle corpus / COL-007 / live hooks |
| `TEST-003-FAKE-CLOCK-MVP` | **done (scaffold)** | Fake-clock + M4 **mini** sample (counting + v5 wire); full corpus residual until complete TEST-003 |
| `ADR-0001-V6-PACKING-ACCEPTED` | **done** (accepted) | Packing intent ADR accepted as-is (OQ-1). Not wire freeze; not COL-007. **Before full COL-007.** |
| `ADR-0002-V6-STRING-POOL-ACCEPTED` | **done** (accepted) | FOOTER string-pool ADR accepted as-is (OQ-1). Not global pool; not COL-007. **Before full COL-007.** |
| `FMT-V6-PROVISIONAL-ID-LOCKFILE` | **done** (frozen status) | ID lockfile + C header; promoted by ADR-0006 after E3/E4-v0. |
| `FMT-V6-WIRE-FREEZE` | **done** | ADR-0006 major=6 numeric ID freeze; catalog `v6-wire-ids-frozen-v1.md`. Residual: E3-mixed / CLI v6 default / COL-008. |
| `FMT-V6-GOLDEN-VECTORS` | **done** | `fixtures/v6/vectors/` + `cargo test -p nytprof-format-v6 golden_vector_`. |
| `DUAL-EQUALITY-READINESS-MVP` | **done** | Dual-equality readiness contract (E1–E5); not product freeze. **Before full COL-007.** |
| `COL-007-ABS-MVP` | **done (scaffold)** | Absolute v6 writer (codec NONE EVENT) + unit vectors; **not** packing/codecs/E3-C; **not** board COL-007 done |
| `COL-007` | deferred | C v6 writer product — staged after ABS-MVP; board flip at PR-B09 E3-C; against accepted ADR-0001/0002 + provisional ID lockfile |
| `FMT-V6-STRING-DICTIONARY-PROVISIONAL` | **done** | String-dictionary intern preflight (`string_id` → blob). Not permanent global pool freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-STRING-DICTIONARY-MVP` | **done** | Dictionary encode/decode + always-inflate EVENT/mixed resolve tests (resolved string bytes; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SITE-DELTA-PROVISIONAL` | **done** | Location/site-delta preflight (`FLAG_SITE_DELTA` for TIME_LINE/TIME_BLOCK/SUB_ENTRY). Not permanent packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SITE-DELTA-MVP` | **done** | Site-delta encode/decode + always-inflate EVENT/mixed tests (absolute reconstruction; NONE/ZLIB/ZSTD/LZ4; SOURCE co-kind). **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-TIME-LINE-RUN-PROVISIONAL` | **done** | TIME_LINE_RUN packed same-site TIME_LINE run preflight. Not permanent packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-TIME-LINE-RUN-MVP` | **done** | TIME_LINE_RUN encode/decode + always-inflate EVENT/mixed (expanded TIME_LINE sequence; every ticks retained; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-TIME-BLOCK-RUN-PROVISIONAL` | **done** | TIME_BLOCK_RUN packed same-site TIME_BLOCK run preflight. Not permanent packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-TIME-BLOCK-RUN-MVP` | **done** | TIME_BLOCK_RUN encode/decode + always-inflate EVENT/mixed (expanded TIME_BLOCK sequence; every ticks retained; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SEQ-NUMBER-PROVISIONAL` | **done** | Logical event sequence-number preflight (OI-001-03 runway). Not full OI-001-03 freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SEQ-NUMBER-MVP` | **done** | Sequence-number encode/decode + always-inflate EVENT/mixed (dual-output order+seq; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-ATTR-OPTION-KNOWN-KEY-EXPAND-PROVISIONAL` | **done** | Expand ATTRIBUTE/OPTION known-key inventory from golden fixture dumps (9+18 keys). Not full OI-002 freeze. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-ATTR-OPTION-KNOWN-KEY-EXPAND-MVP` | **done** | Fixture-driven known-key membership + expanded always-inflate EVENT/mixed (NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SITE-DELTA-SEQ-COMPOSE-PROVISIONAL` | **done** | Composed site-delta + seq packing preflight. Not permanent packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-EVENT-BODY-SITE-DELTA-SEQ-COMPOSE-MVP` | **done** | Compose encode/decode + always-inflate EVENT/mixed (absolute sites + per-event seq; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-STRING-DICT-SITE-DELTA-SEQ-COMPOSE-PROVISIONAL` | **done** | FOOTER string-dictionary + site-delta/seq packing compose. Not permanent pool/packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-STRING-DICT-SITE-DELTA-SEQ-COMPOSE-MVP` | **done** | Dict+packing compose encode/decode + always-inflate EVENT/mixed (resolved strings + absolute sites + seq; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Multi-chunk packing continuity (site/seq bases across chunks). Not permanent packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Multi-chunk packing encode/decode + always-inflate EVENT/mixed (join = single-chunk; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-STRING-DICT-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | FOOTER string-dict + multi-chunk site-delta/seq packing continuity. Not permanent pool/packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-STRING-DICT-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Multi-chunk dict+packing compose encode/decode + always-inflate EVENT/mixed (join = single-chunk; resolved strings + absolute sites + seq; NONE/ZLIB/ZSTD/LZ4). **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-TIME-RUN-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Multi-chunk packing + TIME_*_RUN continuity (post-run site-delta across chunks). Not permanent packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-MULTI-CHUNK-TIME-RUN-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Multi-chunk packing with TIME_LINE_RUN/TIME_BLOCK_RUN + always-inflate EVENT/mixed (NONE/ZLIB/ZSTD/LZ4; SOURCE co-kind). **Before full COL-007.** |
| `FMT-V6-STRING-DICT-MULTI-CHUNK-TIME-RUN-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | FOOTER string-dict + multi-chunk packing + TIME_*_RUN continuity. Not permanent pool/packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-STRING-DICT-MULTI-CHUNK-TIME-RUN-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Dict+multi-chunk+TIME_*_RUN compose encode/decode + always-inflate EVENT/mixed (NONE/ZLIB/ZSTD/LZ4; SOURCE co-kind; resolved strings). **Before full COL-007.** |
| `FMT-V6-AUTO-VERSION-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Auto-VERSION + multi-chunk packing continuity. Not dual-equality / permanent packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-AUTO-VERSION-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Auto-VERSION + multi-chunk packing encode/decode + always-inflate EVENT/mixed (NONE/ZLIB/ZSTD/LZ4; TIME_*_RUN post-run across chunks). **Before full COL-007.** |
| `FMT-V6-AUTO-VERSION-STRING-DICT-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Auto-VERSION + FOOTER dict + multi-chunk packing continuity. Not dual-equality / permanent pool/packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-AUTO-VERSION-STRING-DICT-MULTI-CHUNK-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Auto-VERSION + dict + multi-chunk packing encode/decode + always-inflate EVENT/mixed (NONE/ZLIB/ZSTD/LZ4; TIME_*_RUN post-run; resolved strings). **Before full COL-007.** |
| `FMT-V6-MID-STREAM-CODEC-SWITCH-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Mid-stream codec-switch + packing continuity. Not permanent packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-MID-STREAM-CODEC-SWITCH-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Mid-stream packing encode/decode + always-inflate EVENT/mixed (NONE→ZLIB/ZSTD/LZ4; TIME_*_RUN post-run into post). **Before full COL-007.** |
| `FMT-V6-MID-STREAM-CODEC-SWITCH-STRING-DICT-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Mid-stream packing + FOOTER string-dict continuity. Not permanent pool/packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-MID-STREAM-CODEC-SWITCH-STRING-DICT-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Mid-stream packing + FOOTER dict encode/decode + always-inflate EVENT/mixed (NONE→ZLIB/ZSTD/LZ4; resolved strings; TIME_*_RUN post-run into post). **Before full COL-007.** |
| `FMT-V6-AUTO-VERSION-MID-STREAM-CODEC-SWITCH-SITE-DELTA-SEQ-PACKING-PROVISIONAL` | **done** | Auto-VERSION + mid-stream packing continuity. Not dual-equality / permanent packing ADR. Default `parse_chunk_frame` stays non-inflating. **Before full COL-007.** |
| `FMT-V6-AUTO-VERSION-MID-STREAM-CODEC-SWITCH-SITE-DELTA-SEQ-PACKING-MVP` | **done** | Auto-VERSION mid-stream packing encode/decode + always-inflate EVENT/mixed (NONE→ZLIB/ZSTD/LZ4; TIME_*_RUN post-run into post). **Before full COL-007.** |
| `ADR-0001-V6-PACKING-CANDIDATE` | **done** (proposed) | Permanent packing intent ADR candidate. Not wire freeze; not COL-007. Default parse non-inflating. **Before full COL-007.** |
| `DUAL-EQUALITY-READINESS-PROVISIONAL` | **done** | Dual-equality readiness E1–E5 + open gates. Not dual-equality freeze; not COL-007. **Before full COL-007.** |
| `DUAL-EQUALITY-READINESS-MVP` | **done** | Honesty sync; first-slice complete, full R1 residual, R2 runway only. **Before full COL-007.** |
| `ADR-0002-V6-STRING-POOL-CANDIDATE` | **done** (proposed) | FOOTER string-pool ADR candidate. Not global pool; not COL-007. **Before full COL-007.** |
| `E3-DUAL-EQUALITY-HARNESS-MVP` | **done** | E3 harness writer-bytes→decode equality (stand-in absolute/packing/string-dict/mid-stream). Stand-in **not** product dual-equality evidence. Not COL-007 C writer. **Before full COL-007.** |
| `E4-V5-V6-SEMANTIC-EQUALITY-POLICY-PROVISIONAL` | **done** | E4 v5↔v6 semantic equality policy draft. E4-v0 model enforcement ready (PR-B10). |
| `E4-V5-V6-SEMANTIC-EQUALITY-POLICY-MVP` | **done** | E4 policy honesty sync. E4-v0 model + E4 product CLI ready (PR-B12b / **E4-PRODUCT-CLI-SMOKE-MVP**); full oracle dual residual (TEST-008). |
| `COL-007` | deferred | C v6 writer — unblocked for *start* after report-side evidence; not implemented by this runbook |

## Revision rule

Expanding advertised preview surfaces, closing residual rows, or changing the offline gate step list requires updating this runbook **and** the residual matrix / surface contract as appropriate. This document is an **operator map**, not release certification.

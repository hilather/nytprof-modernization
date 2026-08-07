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
- production **FFI / XS** `Devel::NYTProf::Data` materialization;
- full oracle **`nytprofhtml` DOM** / CSS / tablesorter / flame / Graphviz parity;
- **v6** wire freeze or **COL-007** C v6 writer;
- performance certification or public perf claims;
- permission to flip product defaults (`engine=auto` as R3 product default, format defaults — charter R3/R4).

**Ready vs residual freeze:**  
[R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md)

**Isolation rule (always):** never put `crates/` on oracle `PERL5LIB`. Oracle tools use `baseline/6.15/install` only.

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
| 1 | `cargo test -p nytprof-format-v5 -p nytprof-model -p nytprof-report -p nytprof-cli` | **Honest skip** if `cargo` / `crates/` absent |
| 2 | `./tools/oracle/selftest_harness.sh` | **Required** (dump parity, fail-closed, incomplete-stream, decode-fuzz, normalize, …) |
| 3 | `./scripts/packaging/dual_path_smoke.sh` | **Primary packaging** — legacy always; native install when cargo present |
| 4 | `./scripts/packaging/engine_auto_fallback_smoke.sh` | **Required** (Perl `engine=auto` prefer-native / fall-back-legacy) |
| 5 | `./scripts/packaging/perl_jsonl_data_all_smoke.sh` | **Required** (pure-Perl JsonlData roll-up incl. DISCOUNT A3 + **SUB_ENTRY** multiplicity; golden JSONL; no cargo) |
| 6 | `./scripts/packaging/perl_query_json_smoke.sh` | **Required** (**CI-QUERY-JSON-GATE** / QUERY-JSON-MVP / QUERY-JSON-EXPAND; golden `--jsonl`; no cargo) |
| 7 | `./scripts/packaging/native_agg_json_smoke.sh` | **Optional when native** (**NATIVE-AGG-JSON**; **15/3/15**) |
| 8 | `./scripts/packaging/native_query_json_cross_smoke.sh` | **Optional when native** (**NATIVE-QUERY-JSON-CROSS**: native `report --json` ↔ Perl `query --json` **15/3/15** + discount **818**) |
| 9 | `./scripts/packaging/capability_selftest_smoke.sh` | Run when cargo **or** `prefix`/`target` native CLI (or `$NYTPROF_NATIVE_CLI`); **honest skip** otherwise (**CI-CAPABILITY-GATE**) |

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

Expect JSON fields: `ok`, `profile`, `leaf_returns` **15**, `mid_returns` **3**, `mid_leaf_edge` **15**, `discount_events`, `subs`, `edges` (TAB edge keys). Fail-closed on incomplete/corrupt streams same as text report.

Schema: [native-aggregates-json-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-aggregates-json-mvp-v0.md)

### Cross-check native JSON vs Perl `query --json` (NATIVE-QUERY-JSON-CROSS)

```sh
# Pair: native report --json  vs  Perl query --json --jsonl  (×2 + equal fields)
./scripts/packaging/native_query_json_cross_smoke.sh
```

Shared fields must match: `leaf_returns` **15**, `mid_returns` **3**, `mid_leaf_edge` **15**, `discount_events` **818**. Optional path also runs `query --json` on the live profile (native dump → JsonlData). Fails closed without native CLI; pure-Perl query alone is `perl_query_json_smoke.sh`.

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
# Structured JSON (QUERY-JSON-MVP / QUERY-JSON-EXPAND):
#   leaf_returns=15 / mid_returns=3 / mid_leaf_edge=15
#   discount_events=818 / is_stream_complete=true
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
| [REPORT_HTML_RESIDUAL_INVENTORY_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/REPORT_HTML_RESIDUAL_INVENTORY_v0.md) | Oracle `nytprofhtml` vs native HTML artifact residual honesty |
| [BUILD_SUPPORT_POLICY.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) | Dual-path packaging + offline gate policy |
| [FIRST_SLICE_BOARD.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md) | Ordered board (this runbook = `R1-PREVIEW-RUNBOOK`) |
| [PROGRAM_CHARTER.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/PROGRAM_CHARTER.md) | R0–R5 levels and non-goals |

---

## 7. Explicit residual honesty

Do **not** claim these under offline R0 / R1-preview (full-R1 residuals; `R1-HONESTY-SYNC`):

| Residual | Notes |
|----------|--------|
| **No production FFI / XS Data** | No RUST-010 cdylib ABI; no PERL-004 XS ReadStream over binary profiles; no PERL-005 bless-array Data materializer. Preview = CLI subprocess + pure-Perl JsonlData from dump JSONL (incl. SUB_ENTRY multiplicity only). |
| **No full nytprofhtml DOM** | Native HTML is MVP summary / multi-file site only — not oracle DOM, CSS/JS, tablesorter, flame SVG, Graphviz. See HTML residual inventory. |
| **No v6 / COL-007** | No v6 wire freeze; C v6 writer (**COL-007**) deferred; COL-008 batched Rust writer non-baseline. Collector remains 6.15 oracle / v5. |
| **No performance claims** | Light wall-time notes only (`docs/BENCH_NOTES.md`, `tools/bench/light_bench.sh`). No public SLOs or certification. |
| **No full MakeMaker XS CPAN dual-build** | Candidate `Makefile.PL` facade only (**BUILD-MAKEMAKER-OPT**), not BUILD-003 full. |
| **No multi-OS CI matrix** | Single-host `offline_gate.sh` only (**BUILD-006** open). |
| **No product default flip** | Native remains opt-in; Perl `engine=auto` is facade behavior, not charter R3 product default. |

Advertised preview **does** include native aggregates JSON, pure-Perl query JSON, **native↔query JSON cross-parity** (when native CLI present), and pure-Perl **SUB_ENTRY** event multiplicity — without promoting those to full R1 / CPAN / FFI readiness.

Full residual table: [R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) § Residual for full R1.

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
| `sub_entry_events` (`calls=1`) | **0** |

```sh
# Native report text should show leaf returns=15, mid returns=3
./prefix/bin/nytprof-cli report fixtures/v5/default-calls1/nytprof.out

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

### `fixtures/v5/blocks-calls1` (line 5 = 780)

| Check | Expected |
|-------|----------|
| `line_total(1,5).calls` / `line_calls(1,5)` (TIME_BLOCK) | **780** |

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
4. When native present: `./scripts/packaging/native_query_json_cross_smoke.sh` (native↔query JSON **15/3/15** + discount **818**).  
5. JsonlData roll-up smoke green (incl. SUB_ENTRY **0** / **27**); blocks-calls1 line5 **780**.  
6. Read residual honesty before claiming “R1 done.”

---

## Board placement

| ID | Status | Evidence |
|----|--------|----------|
| `R1-PREVIEW-RUNBOOK` | done | this file (`docs/R1_PREVIEW_OPERATOR_RUNBOOK.md`) |
| `R1-RESIDUAL-MATRIX` | done | [R1_RESIDUAL_READINESS_MATRIX_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md) |
| `R1-HONESTY-SYNC` | **done** (this slice) | matrix + this runbook advertise **NATIVE-QUERY-JSON-CROSS** (`scripts/packaging/native_query_json_cross_smoke.sh`) + **PERL-SUB-ENTRY-JSONL** (`scripts/packaging/perl_sub_entry_smoke.sh`, `perl/t/jsonl_data_sub_entry.t`) while listing full-R1 residuals (no production FFI/XS Data, no full nytprofhtml DOM, no v6/COL-007, no multi-OS CI, no perf claims). **Before COL-007.** |
| `NATIVE-QUERY-JSON-CROSS` | done | `scripts/packaging/native_query_json_cross_smoke.sh`; offline_gate step 8 when native |
| `PERL-SUB-ENTRY-JSONL` | done | `JsonlData` `sub_entry_*`; smoke + test above; roll-up in `perl_jsonl_data_all_smoke.sh` |
| `COL-007` | deferred | C v6 writer — unblocked for *start* after report-side evidence; not implemented by this runbook |

## Revision rule

Expanding advertised preview surfaces, closing residual rows, or changing the offline gate step list requires updating this runbook **and** the residual matrix / surface contract as appropriate. This document is an **operator map**, not release certification.

# Product XS Data / ReadStream path MVP (v0)

**Board ID:** `PERL-XS-DATA-READSTREAM-MVP` (PR-A06 / **OQ-2** / toward **PERL-004** / **PERL-005**)  
**Status:** implemented (product path MVP — **not** full COMPAT-007 / pure-XS wire decode)  
**Modules:**  
- [`perl/lib/Devel/NYTProf/Data.pm`](https://github.com/hilather/nytprof-modernization/blob/main/perl/lib/Devel/NYTProf/Data.pm)  
- [`perl/lib/Devel/NYTProf/ReadStream.pm`](https://github.com/hilather/nytprof-modernization/blob/main/perl/lib/Devel/NYTProf/ReadStream.pm)  
**Policy ADR:** [`docs/adrs/0003-r1-full-residual-policy.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md) (OQ-2: XS Data/ReadStream must **CLOSE**, not waive)

## Goal

Ship a **product** Perl Data + ReadStream path that opens **binary v5 profiles** (not only dump JSONL) and answers the advertised query / callback surface — without requiring oracle `Devel::NYTProf` on `PERL5LIB`.

Implementation for this MVP is a **thin materializer**:

```text
binary nytprof.out
        │
        ▼
native nytprof-cli dump  (subprocess; EngineDispatch::find_native_cli)
        │
        ▼
JsonlReadStream / JsonlData   (pure-Perl JSONL bridge)
        │
        ▼
Devel::NYTProf::Data / ::ReadStream   (product facade)
```

This **closes the OQ-2 product path** for PERL-004/005 at MVP depth. It does **not** claim full bless-array COMPAT-007 fidelity or in-process pure-XS wire decode.

## Non-goals (residual honesty)

| Residual | Notes |
|----------|--------|
| Full COMPAT-007 bless-array / AV-HV shapes | `claims_compat007_shapes` is always **0**; dual-engine shape tests not claimed |
| Pure-XS binary wire decode (no CLI) | Still residual; future depth may bind `nytprof-ffi` (PR-A05) or pure-XS |
| Full oracle `Devel::NYTProf::Data` method set | Only JsonlData-advertised queries + product metadata |
| Full scalar-flag / UTF-8 / taint fidelity package | Residual under PERL-001/004 acceptance |
| Dual-path without native CLI | Binary `from_profile` / `filename` requires discoverable native CLI; golden JSONL bridge works without Cargo |
| v6 / COL-007 | v5 dump path only |
| Perf claims | No public SLOs for this path |

## Product API — Data

| Constructor | Input | Backend tag |
|-------------|-------|-------------|
| `->new({ filename => $path })` / `->from_profile($path)` | binary profile | `native-cli-jsonl` |
| `->from_jsonl($path)` / `->new({ jsonl => $path })` | dump JSONL | `jsonl-file` |
| `->from_cli([@argv])` | CLI stdout JSONL | `jsonl-cli` |

| Method | Semantics |
|--------|-----------|
| `sub_returns` / `call_edge_count` / `line_calls` / `block_line_calls` / … | Delegated to underlying `JsonlData` (full advertised subset) |
| `backend` | `native-cli-jsonl` \| `jsonl-file` \| `jsonl-cli` \| `jsonl-wrap` |
| `materializer` | `thin-native-cli-jsonl` or `jsonl-bridge` |
| `is_product_path` | always true |
| `claims_compat007_shapes` | always **0** |
| `jsonl_data` | underlying `JsonlData` |
| Completeness | default **fail-closed** on incomplete streams (COMPAT-010); `allow_incomplete => 1` to override |

## Product API — ReadStream

| Entry | Input | Path |
|-------|-------|------|
| `for_chunks($cb, filename => $p)` / `profile => $p` | binary | native dump → JsonlReadStream |
| `for_chunks($cb, jsonl => $p)` / `file => $p` | JSONL | JsonlReadStream pass-through |
| `for_chunks($cb, from_cli => [...])` | subprocess | JsonlReadStream |
| `process_profile($path, \%handlers)` | binary | per-tag handlers over dump |

| Meta | Value |
|------|--------|
| `is_product_path()` | true |
| `materializer_kind()` | `thin-native-cli-jsonl` |

Callback shape: `$cb->($tag, $args, $seq)` with dump-schema arg order ([canonical-event-dump-v0](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/canonical-event-dump-v0.md)).

## Semantic golden checks

Real fixtures (no invented counts):

| Fixture | Check | Expected |
|---------|-------|----------|
| `fixtures/v5/default-calls1` | `main::leaf` returns | **15** |
| `fixtures/v5/default-calls1` | `main::mid` returns | **3** |
| `fixtures/v5/default-calls1` | mid→leaf edge | **15** |
| `fixtures/v5/default-calls1` | `discount_events` | **818** |
| `fixtures/v5/blocks-calls1` | `line_calls(1,5)` / `block_line_calls(1,4)` | **780** / **810** |
| incomplete JSONL prefix | default `from_jsonl` | croaks (COMPAT-010) |

Evidence:

```sh
prove -Iperl/lib perl/t/data_product_default_calls1.t
prove -Iperl/lib perl/t/readstream_product_default_calls1.t
prove -Iperl/lib perl/t/data_product_blocks_calls1.t
./scripts/packaging/perl_xs_data_readstream_smoke.sh
```

## Relationship to full R1 / OQ-2

| Claim | Status after PR-A06 |
|-------|---------------------|
| Product Data path over **binary** profiles | **yes** (thin native-cli-jsonl) |
| Product ReadStream path over **binary** profiles | **yes** (thin native-cli-jsonl) |
| JsonlData / JsonlReadStream still available | **yes** (bridge unchanged) |
| PERL-004 / PERL-005 residual rows | **partial close** — product path shipped; full pure-XS / COMPAT-007 residual remains |
| OQ-2 waiver | **forbidden** — this PR implements close path; does not waive |
| Full COMPAT-007 | **not** claimed |

## Board placement

| ID | Status | Evidence |
|----|--------|----------|
| `PERL-XS-DATA-READSTREAM-MVP` | **done** (MVP) | this schema + modules + tests + smoke |
| `PERL-004` (full) | **partial** | remaining: pure-XS wire decode, scalar-flag package, dual-engine callback fidelity |
| `PERL-005` (full) | **partial** | remaining: COMPAT-007 bless-array materializer, full oracle Data method set |

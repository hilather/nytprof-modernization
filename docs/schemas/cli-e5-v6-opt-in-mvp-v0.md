# CLI E5 — v6 opt-in product surfaces (MVP v0)

**Board ID:** `CLI-E5-V6-OPT-IN-MVP`  
**Status:** implemented (PR-B12) — **not** collection default flip (R4); **not** convert/merge (PR-C01/C02); E4 product offline_gate: **E4-PRODUCT-CLI-SMOKE-MVP** (PR-B12b)  
**Depends on:** product v6→ProfileModel ingest ([`product-v6-profilemodel-ingest-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-v6-profilemodel-ingest-mvp-v0.md)); wire freeze ADR-0006; capability self-test MVP  
**Evidence:** `cargo test -p nytprof-cli --test cli_e5_v6`; `cargo test -p nytprof-cli --test capability_selftest`; `./scripts/packaging/capability_selftest_smoke.sh`

## Goal

Ship **full native offline product surfaces** on **v6 EVENT** profiles (magic auto-detect), without flipping the **collection** default from v5:

| Surface | Command | Path |
|---------|---------|------|
| dump | `nytprof-cli dump <file>` | `decode_events_from_path` (v5+v6) |
| verify / inspect | `nytprof-cli verify <file>` | `ProfileModel::from_path` |
| report / summary | `nytprof-cli report [--json] <file>` | same model → text / aggregates JSON |
| aggregates / agg | `nytprof-cli aggregates <file>` | always JSON |
| html | `nytprof-cli html <file> [-o PATH \| --out-dir DIR]` | single-file or multi-file site |
| csv | `nytprof-cli csv [--subs] [--edges] <file>` | dual-section CSV |
| folded | `nytprof-cli folded <file>` | folded stacks |
| callgrind / cg | `nytprof-cli callgrind <file>` | Callgrind-style export |
| capability | `nytprof-cli capability [--json]` | honesty fields (below) |

No extra `--format=v6` flag is required for offline tools: detection is magic-based (`NYTPROF6` vs `NYTProf 5 …`).

## Collection default (no flip)

| Item | Value |
|------|-------|
| Product collection default | **v5** until R4 ADR + field window |
| Opt-in collection | `format=v6` / collector product naming (REL docs) — **not** claimed ready as operator UX in this PR beyond writer harness |
| Capability field | `collection_default: "v5"` (human `collection_default: v5`) — tests assert **no** default flip |

## Capability honesty (E5)

Human markers (stable order after `verify: yes`):

```text
v6_decode: yes
v6_report: yes
convert: no
merge: no
collection_default: v5
profile_ok: <path|skip>
v6_profile_ok: <path|skip>
```

JSON fields (in addition to CAPABILITY-JSON-MVP `ok`/`decode`/`report`/`verify`/`profile_ok`):

| Field | Type | Meaning |
|-------|------|---------|
| `v6_decode` | boolean `true` | Product v6 always-inflate decode is linked |
| `v6_report` | boolean `true` | Product report/html/csv/… path accepts v6 via dual-dispatch model |
| `convert` | boolean `false` | **Must not** claim until PR-C01 |
| `merge` | boolean `false` | **Must not** claim until merge tooling ships |
| `collection_default` | string `"v5"` | Collection format default; R4 residual to flip |
| `v6_profile_ok` | string path **or** `null` | Optional v6 golden verify probe |

Primary v5 probe order unchanged (`--profile` / `NYTPROF_CAPABILITY_FIXTURE` / default-calls1).  
Optional v6 probe: `NYTPROF_CAPABILITY_V6_FIXTURE` → CWD `fixtures/e4/dual-sink/default_calls1_v6.nytprof` → `CARGO_MANIFEST_DIR` repo root. Present probe **must** verify or self-test fails closed.

Schema extension of: [`capability-selftest-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/capability-selftest-mvp-v0.md).

## Semantic smoke fixtures

| Fixture | Surfaces | Exact greps |
|---------|----------|-------------|
| `fixtures/e4/dual-sink/default_calls1_v6.nytprof` | report / `--json` / html / csv / folded / callgrind | leaf **15**, mid **3**, mid→leaf **15** |
| `fixtures/v6/from-c/absolute.nytprof` | verify / dump / cg | `OK:` + EVENT tags / callgrind header |

Fail-closed truncated / CRC-corrupt v6 remains under `fail_closed.rs` (PR-B11a).

## Non-claims / residuals

- **Not** collection `format=v6` as product default (R4)
- **Not** convert / merge / salvage tooling (PR-C01+; capability stays `false`)
- E4 product CLI smoke: [`e4-product-cli-smoke-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/e4-product-cli-smoke-mvp-v0.md)
- **Not** full oracle dual pairs (TEST-003/TEST-008)
- **Not** E3-mixed multi-kind product path
- **Not** full nytprofhtml DOM / FFI / XS Data

## Related

- Dual-equality E5 class: [`docs/contracts/DUAL_EQUALITY_READINESS_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DUAL_EQUALITY_READINESS_v0.md)
- Product ingest: [`product-v6-profilemodel-ingest-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-v6-profilemodel-ingest-mvp-v0.md)
- Board: [`docs/FIRST_SLICE_BOARD.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/FIRST_SLICE_BOARD.md) (`CLI-E5-V6-OPT-IN-MVP`)

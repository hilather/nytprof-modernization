# Product v6 → ProfileModel ingest MVP (v0)

**Board ID:** `PRODUCT-V6-MODEL-INGEST-MVP`  
**Status:** implemented (PR-B11a) — **not** wire freeze; **not** full CLI E5 claim; E4-v0 uses this ingest path (PR-B10)  
**Depends on:** COL-007 E3-EVENT (`fixtures/v6/from-c/**`); A1–A9 aggregation (`aggregate-comparison-v0.md`); dual-equality readiness  
**Evidence:** `cargo test -p nytprof-model` (`v6_*`); `cargo test -p nytprof-format-v6 --lib dual_equality`; CLI `dump`/`verify` on `fixtures/v6/from-c/absolute.nytprof`

## Goal

Ship a **product** load path so `ProfileModel::from_path` (and dump prelim) accept **both**:

| Wire | Magic / header | Decoder |
|------|----------------|---------|
| v5 | first line `NYTProf <major> <minor>` | `nytprof_format_v5::decode_all` |
| v6 | first 8 bytes `NYTPROF6` | always-inflate EVENT (+ FOOTER string-dict when present) |

Aggregation rules (A1–A9) stay format-agnostic once logical events exist.

## Dual dispatch

```text
path / bytes
  → detect_profile_wire_kind
       • NYTPROF6 → product_decode_v6_event_profile (CRC on)
                    → OwnedEventRecord[] → dump-aligned Event[]
       • NYTProf … → v5 decode_all
       • else → ModelError::UnsupportedProfile (fail closed)
  → ProfileModel::from_events (unchanged A1–A9)
```

APIs:

| Surface | Location |
|---------|----------|
| `detect_profile_wire_kind` / `product_decode_v6_event_profile` | `nytprof_format_v6::dual_equality` |
| `decode_events_from_path` / `decode_events_from_bytes` / `from_path` / `from_bytes` | `nytprof_model` |
| CLI `dump` | `decode_events_from_path` (v5 + v6) |
| CLI `verify` / `report` | `ProfileModel::from_path` (inherits dual dispatch) |

## v6 → logical Event mapping (dump-aligned)

Expanded packing (`TIME_*_RUN`, site-delta) and resolved FOOTER dict strings are already absolute `OwnedEventRecord`s before mapping.

| Owned record | Tag | Arg notes |
|--------------|-----|-----------|
| `TimeLine` | `TIME_LINE` | ticks, fid, line |
| `TimeBlock` | `TIME_BLOCK` | ticks, fid, line, block_line, **sub_line=0** (v6 body has no sub_line) |
| `SubReturn` | `SUB_RETURN` | depth, incl, excl, subname (integer ticks as JSON numbers) |
| `NewFid` | `NEW_FID` | fid, **0×5**, filename (pad eval/flags/size/mtime) |
| `Mark` | `COMMENT` | label text (no separate ReadStream MARK tag) |
| others | matching ReadStream tags | ATTRIBUTE/OPTION/SRC_LINE/SUB_* / PID_* / DISCOUNT / START_DEFLATE / VERSION |

Auto-VERSION inject when body omits VERSION (header major/minor).

**Dump `seq`:** stream order `0..n-1` after expansion + auto-VERSION (dumper-monotonic). Packing `FLAG_HAS_SEQ` wire values are **not** reused for dump `Event.seq` (avoids VERSION/`None` colliding with body seq 0).

**FOOTER fail-closed:** when a FOOTER chunk is present, product decode **requires** a well-formed string dictionary and full string_id resolve. Missing ids / corrupt table → `Err` (no empty-string soft fallback into attributes/names).

## Pair aggregate parity tests

| Pair | Assertion |
|------|-----------|
| C `absolute.nytprof` vs `packing.nytprof` / `packing_lz4.nytprof` | equal A1–A9 aggregates |
| Stand-in absolute vs packing of same logical sample | equal aggregates vs `from_events` on dump Events |
| C `dict.nytprof` | `attributes["basetime"]` resolved |
| Stand-in FOOTER missing `string_id` | `from_bytes` / product decode `Err` |
| Trunc / trailing / CRC-corrupt v6 | model + CLI dump/verify/report fail closed |

**E4-v0** model-level enforcement on dual-sink scaled pairs is ready (PR-B10; `fixtures/e4/dual-sink/`). Full oracle same-workload pairs + offline_gate product CLI smoke remain residual (PR-B12b / TEST-008).

## Non-claims

- Not wire freeze / FMT-002..010
- Not full multi-kind SOURCE/INDEX/SUMMARY product path (E3-mixed residual)
- Not CLI collection `format=v6` default; not full E5 capability `v6_report` advertising matrix
- Not COL-008; not convert/merge

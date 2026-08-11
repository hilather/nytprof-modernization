# Format v6 ATTRIBUTE/OPTION known-key vocabulary — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-ATTR-OPTION-KNOWN-KEY-PROVISIONAL` / `FMT-V6-ATTR-OPTION-KNOWN-KEY-MVP` (initial table); `FMT-V6-ATTR-OPTION-KNOWN-KEY-EXPAND-PROVISIONAL` / `FMT-V6-ATTR-OPTION-KNOWN-KEY-EXPAND-MVP` (fixture inventory expand)  
**Depends on:** [`v6-event-body-attribute-option-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-attribute-option-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** complete OI-002-03/04 freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Documents a provisional **known-key set** for ATTRIBUTE and OPTION string keys.

**Inventory source (expand preflight):** union of ATTRIBUTE / OPTION keys observed in golden fixture dump JSONL:

- `fixtures/v5/default-calls1/readstream.jsonl`
- `fixtures/v5/calls2-default/readstream.jsonl`
- `fixtures/v5/blocks-calls1/readstream.jsonl`
- `fixtures/v5/default-calls2/readstream.jsonl`

Evidence test: `known_key_covers_all_keys_in_golden_fixture_jsonl` loads those files via the real filesystem path and asserts every observed key is classified known by shipped `known_key` helpers.

### Known ATTRIBUTE keys (OI-002-03 runway) — 9

| Key | Notes |
|-----|-------|
| `basetime` | JSON-ATTR-BASETIME-MVP dump sample |
| `ticks_per_sec` | JSON-META-FILES-MVP |
| `application` | COMPAT-002 volatile (basename normalize residual) |
| `xs_version` | dump meta sample |
| `perl_version` | fixture dump |
| `nv_size` | fixture dump |
| `PL_perldb` | fixture dump |
| `clock_id` | fixture dump |
| `cumulative_overhead_ticks` | fixture dump (late stream) |

### Known OPTION keys (OI-002-04 runway) — 18

| Key | Notes |
|-----|-------|
| `calls` | default-calls1 multiplicity surface |
| `blocks` | blocks mode option |
| `stmts` | statements mode option |
| `compress` | compress option |
| `usecputime` | fixture dump |
| `subs` | fixture dump |
| `leave` | fixture dump |
| `expand` | fixture dump |
| `trace` | fixture dump |
| `use_db_sub` | fixture dump |
| `clock` | fixture dump |
| `slowops` | fixture dump |
| `findcaller` | fixture dump |
| `forkdepth` | fixture dump |
| `perldb` | fixture dump |
| `nameevals` | fixture dump |
| `nameanonsubs` | fixture dump |
| `evals` | fixture dump |

Values remain **string projections** (ULEB string-blob key + value). Free-form unknown keys may still encode/decode; the table is a preflight inventory, **not** a reject-unknown policy or full writer freeze.

It is **not**:

- complete ATTRIBUTE/OPTION key vocabulary freeze (full OI-002-03 / OI-002-04 from **all** writers / platforms beyond these fixtures);
- reject-unknown key policy;
- full OI-001-03 dual-output sequence-number freeze;
- COMPAT-002 full volatile key-list freeze beyond existing basetime/application rules;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Shipped API

Crate: `crates/nytprof-format-v6`

| Item | Role |
|------|------|
| `known_key::*` constants | Key byte strings |
| `known_key::KNOWN_ATTRIBUTE_KEYS` / `KNOWN_OPTION_KEYS` | Tables (fixture-expanded) |
| `known_key::is_known_attribute_key` / `is_known_option_key` / `is_known_meta_key` | Classification |
| `attribute_kv` / `option_kv` | Spec builders |
| `known_key_attr_option_sample_specs` | Compact 5-record sample |
| `known_key_attr_option_expanded_sample_specs` | Full inventory sample (9+18) |

---

## Always-inflate recovery

Shipped consumers recover known-key ATTRIBUTE/OPTION under:

- `decode_event_body` (key **and** value field asserts)
- `decode_decoded_event_profile` NONE/ZLIB/ZSTD/LZ4 (default stream parse non-inflating)
- `decode_decoded_mixed_profile` with co-present SOURCE under NONE/ZLIB/ZSTD/LZ4 EVENT

Evidence: `cargo test -p nytprof-format-v6` (`known_key_*`, `mixed_known_key_*`, expanded inventory variants).

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid ATTRIBUTE/OPTION | **Err** (unchanged) |
| Unknown opcodes | **Err** (unchanged MVP) |
| Free-form unknown key | **Ok** (still allowed; not in known table) |
| Never panic | Required |

---

## Open residual

1. Complete ATTRIBUTE/OPTION key vocabularies (OI-002-03 / OI-002-04 full inventory from **all** writers / platforms beyond golden fixtures; reject-unknown policy ADR).
2. Full dual-output **sequence-number** freeze (OI-001-03 / COL-003) — seq-number preflight is a sibling.
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default; permanent string pool.

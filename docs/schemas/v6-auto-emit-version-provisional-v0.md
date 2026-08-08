# Format v6 auto-emit VERSION from fixed-header — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-AUTO-EMIT-VERSION-PROVISIONAL` (contract), `FMT-V6-AUTO-EMIT-VERSION-MVP` (shipped helpers + always-inflate tests)  
**Depends on:** [`v6-event-body-version-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-version-provisional-v0.md); fixed-header / file-prefix major/minor; always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** full OI-001-03 freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Surfaces a dump-aligned **VERSION** logical record whose `major`/`minor` match the provisional **fixed-header** (file-prefix) version fields:

| Source | Type | Role |
|--------|------|------|
| Fixed-header `major` / `minor` | `u16` | Authoritative for auto-emit preflight |
| Body `VERSION` opcode | two ULEB `u64` | Optional; must match header when present |

### Policy (preflight)

1. **Auto-emit (decode path):** if recovered EVENT records contain **no** body VERSION, **prepend** `OwnedEventRecord::Version { major: header.major as u64, minor: header.minor as u64 }`.
2. **Consistency:** if any body VERSION is present, every such record must equal the header major/minor (as `u64`); otherwise **fail closed** (`VersionHeaderMismatch`).
3. **Encode path:** `encode_*_auto_version` injects a matching body VERSION when omitted; rejects mismatched body VERSION before seal.

Default always-inflate decode (`decode_decoded_event_profile` / `decode_decoded_mixed_profile`) does **not** auto-emit — use the explicit `*_auto_version` entry points.

It is **not**:

- full **OI-001-03** dual-output sequence-number freeze (COL-003 / ADR + golden vectors);
- complete ATTRIBUTE/OPTION key vocabulary freeze (OI-002-03/04; known-key preflight is a sibling);
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / dual-equality / CLI v6 default / FMT-012 golden corpus;
- permission to mark **COL-007** / **COL-008** done.

---

## Shipped API

Crate: `crates/nytprof-format-v6`

| Function | Role |
|----------|------|
| `version_record_from_header` | Build `OwnedEventRecord::Version` from `FixedHeader` |
| `align_event_records_version_with_header` | Prepend or validate VERSION vs header |
| `encode_decoded_event_profile_auto_version` | Encode with VERSION inject/check |
| `decode_decoded_event_profile_auto_version` | Always-inflate + align |
| `encode_decoded_mixed_profile_auto_version` | Mixed + VERSION inject/check |
| `decode_decoded_mixed_profile_auto_version` | Mixed always-inflate + align |

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Body VERSION ≠ header major/minor | **Err** (`VersionHeaderMismatch`) |
| Truncated / corrupt payload | **Err** (unchanged always-inflate paths) |
| Never panic | Required |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`auto_version_*`, `mixed_auto_version_*`).

- NONE / ZLIB / ZSTD / LZ4 EVENT auto-emit + field asserts tied to header minor
- Decode inject when body omits VERSION
- Mixed path with co-present SOURCE
- Matching body VERSION accepted; mismatch fail-closed
- Default stream parse remains non-inflating for compressed codecs

---

## Open residual

1. Full dual-output **sequence-number** freeze (OI-001-03 / COL-003).
2. Complete ATTRIBUTE/OPTION key vocabularies (OI-002-03/04 full inventory — known-key preflight shipped separately).
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

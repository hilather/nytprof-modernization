# Format v6 event-body unknown optional opcode skip — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-UNKNOWN-OPTIONAL-SKIP-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-UNKNOWN-OPTIONAL-SKIP-MVP` (shipped decode skip + encode helper + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** full opcode-catalog freeze / dual-equality / C v6 writer

---

## Scope and non-claims

Allows `decode_event_body` to **skip** an **unknown** opcode when it is **not** required and the record carries a provisional **length-framed** body, then continue recovering subsequent known records in order.

### Record layout (unknown optional skip)

```text
ULEB128 opcode   (not 0; not a known provisional opcode)
u8 flags         (FLAG_BODY_LENGTH = 0x02; FLAG_OPCODE_REQUIRED must be clear)
ULEB128 body_len
body_len bytes   (opaque; not interpreted)
```

| Flag | Value | Role |
|------|------:|------|
| `FLAG_OPCODE_REQUIRED` | `0x01` | Unknown + required → **Err** (unchanged) |
| `FLAG_BODY_LENGTH` | `0x02` | Optional unknown: length-framed skip preflight |
| `FLAG_SITE_DELTA` | `0x04` | **Not free** — reserved packing (ADR-0001 / lockfile); not used by unknown-optional skip |
| `FLAG_HAS_SEQ` | `0x08` | **Not free** — reserved packing / OI-001-03 runway (ADR-0001 / lockfile); not used by unknown-optional skip |

Known opcodes keep their typed bodies and do **not** require `FLAG_BODY_LENGTH`.

**Reserved opcodes 18/19** (`TIME_LINE_RUN` / `TIME_BLOCK_RUN`) and flag bits **0x04/0x08** are claimed by the provisional ID lockfile ([`V6_PROVISIONAL_ID_LOCKFILE_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md)) and ADR-0001. Unknown-optional-skip preflight must **not** reassign those IDs as free experimental space. Packing encode/decode remains residual.

It is **not**:

- a full opcode catalog freeze or permanent flag-bit freeze (future ADR may reassign bits **other than** lockfile-reserved packing bits without a superseding ADR);
- full OI-001-03 dual-output sequence-number freeze;
- complete OI-002-03/04 key vocabulary inventory;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / dual-equality / CLI v6 default / dictionaries;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Reserved opcode 0 | **Err** (`ReservedOpcode`) |
| Unknown + `FLAG_OPCODE_REQUIRED` | **Err** (`UnknownRequiredOpcode`) |
| Unknown optional **without** `FLAG_BODY_LENGTH` | **Err** (`UnknownOpcode`) |
| Truncated length ULEB / short body | **Err** (`Truncated` / varint) |
| Oversize declared skip body | **Err** (`OversizeSkipBody`) |
| Never panic | Required |

Skipped records are **not** emitted into the recovered `EventRecord` list.

---

## Shipped API

| Item | Role |
|------|------|
| `FLAG_BODY_LENGTH` | Provisional length-frame flag bit |
| `encode_unknown_optional_skip_record` | Test/fixture encoder for skippable unknown optional |
| `decode_event_body` | Skips length-framed unknown optionals; continues |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`unknown_optional_*`, `mixed_unknown_optional_*`).

- Body: known … skip … known preserves order + fields
- `decode_decoded_event_profile` under NONE/ZLIB/ZSTD/LZ4
- `decode_decoded_mixed_profile` with SOURCE co-kind under NONE/ZLIB/ZSTD/LZ4 EVENT
- Default stream parse remains non-inflating for compressed codecs

---

## Open residual

1. ADR freeze of opcode space and permanent flag bits.
2. Full dual-output sequence-number freeze (OI-001-03); complete key vocab inventory (OI-002-03/04).
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

# Format v6 event-body location / site deltas — provisional v0

**Status:** provisional — **not** a v6 wire freeze (not FMT-002..010 ratification; not COL-007 C writer)  
**Board IDs:** `FMT-V6-EVENT-BODY-SITE-DELTA-PROVISIONAL` (contract), `FMT-V6-EVENT-BODY-SITE-DELTA-MVP` (shipped encode/decode + always-inflate tests)  
**Depends on:** [`v6-event-body-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-provisional-v0.md); ZigZag signed varint [`v6-svarint-zigzag-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-svarint-zigzag-provisional-v0.md); always-inflate EVENT/mixed consumers  
**Gate:** COL-007 runway preflight only — **before** permanent packing ADR / dual-equality / C v6 writer

---

## Scope and non-claims

Allows TIME_LINE / TIME_BLOCK / SUB_ENTRY site fields to be carried as **signed deltas** relative to a running base, reconstructed to **absolute** sites on decode.

### Flag

| Flag | Value | Meaning |
|------|------:|---------|
| `FLAG_SITE_DELTA` | `0x04` | Site fields are ZigZag+ULEB deltas (not absolute ULEB) |

Default `encode_event_body` still writes **absolute** ULEB sites with flags `0`.  
`encode_event_body_with_site_deltas` sets `FLAG_SITE_DELTA` on TIME_LINE / TIME_BLOCK / SUB_ENTRY only.

### Delta body layouts (when `FLAG_SITE_DELTA` set)

| Opcode | Typed body |
|--------|------------|
| `TIME_LINE` | `i64 fid_delta`, `i64 line_delta`, `u64 ticks` |
| `TIME_BLOCK` | `i64 fid_delta`, `i64 line_delta`, `i64 block_line_delta`, `u64 ticks` |
| `SUB_ENTRY` | `i64 caller_fid_delta`, `i64 caller_line_delta` |

Running bases start at `0` and update after each reconstructed absolute site (separate bases for statement site vs block_line vs caller site).

It is **not**:

- a permanent packing ADR / alternate opcode freeze;
- permanent run packing freeze (TIME_LINE_RUN is a **sibling** preflight — see [`v6-event-body-time-line-run-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-time-line-run-provisional-v0.md));
- full OI-001-03 sequence-number freeze; complete OI-002 inventory;
- permanent global string-pool freeze;
- mutating default `parse_chunk_frame` to always inflate or verify CRC;
- wire freeze / dual-equality / CLI v6 default;
- permission to mark **COL-007** / **COL-008** done.

---

## Fail-closed

| Condition | Result |
|-----------|--------|
| Truncated mid-delta field | **Err** (varint / truncated) |
| Delta reconstruction outside `u64` | **Err** (`InvalidSiteDelta`) |
| Absolute path (no flag) | Unchanged prior preflight |

---

## Shipped API

| Item | Role |
|------|------|
| `FLAG_SITE_DELTA` | Record flag bit |
| `encode_event_body_with_site_deltas` | Absolute specs → delta wire |
| `decode_event_body` | Reconstructs absolute sites when flag set |

---

## Always-inflate recovery

Evidence: `cargo test -p nytprof-format-v6` (`site_delta_*`, `mixed_site_delta_*`).

- Multi-record TIME_LINE absolute reconstruction
- TIME_BLOCK + SUB_ENTRY deltas
- EVENT NONE/ZLIB/ZSTD/LZ4; mixed + SOURCE
- Absolute-only prior tests remain green
- Default stream parse non-inflating for compressed payloads

---

## Open residual

1. Permanent ADR freeze of delta/run packing / opcode alternate forms (composed site-delta+seq preflight is a sibling — see [`v6-event-body-site-delta-seq-compose-provisional-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/v6-event-body-site-delta-seq-compose-provisional-v0.md)).
2. Full OI-001-03 freeze; full OI-002; permanent string pool (TIME_LINE_RUN / TIME_BLOCK_RUN / seq-number preflights are siblings).
3. Default-parse inflate/CRC mutate; full COL-007 / COL-008; dual-equality; CLI v6 default.

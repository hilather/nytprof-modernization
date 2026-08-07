# COMPAT-002 — Volatile-field normalization (provisional)

**Status:** provisional / in-progress  
**Task:** COMPAT-002  
**Board ID:** COMPAT-002-NORM  
**Date:** 2026-08-07  
**Depends on:** COMPAT-001 (provisional logical events), BASE-005 (fixture/options matrix — partial)  
**Oracle:** Devel::NYTProf 6.15 ReadStream dumps (`fixtures/v5/*/readstream.jsonl`)

---

## Scope and non-claims

This document freezes the **shipped structural normalization rules** used for golden / dual compare of ReadStream JSONL dumps. It does **not**:

- freeze a v6 wire format;
- complete full COMPAT-001 acceptance (every 6.15 record without ambiguity);
- claim that every possible volatile surface (PIDs, HTML whitespace, report timestamps, compression byte streams, version strings) is already normalized.

Full plan acceptance for COMPAT-002 (versioned library + every ignored field traceable) remains **in-progress**. What is landed here is the **executable dump-path contract** matching `tools/oracle/normalize_jsonl.py` mode `structural`.

---

## Why normalization exists

Profile dumps embed host- and run-specific metadata (absolute paths, wall-clock basetime, free-form comments). Structural equality for differential testing must ignore those **without** hiding semantic differences in ticks, counts, event order, locations, source/name bytes, or call edges.

Principle (from COMPAT-001): order and multiplicity are semantic **subject to** the volatile rules below.

---

## Implementation of record (single source of truth)

| Artifact | Path |
|----------|------|
| Normalizer (structural mode) | `tools/oracle/normalize_jsonl.py` |
| Comparator | `tools/oracle/compare_jsonl.pl` |
| Dump schema | `docs/schemas/canonical-event-dump-v0.md` |
| Logical events | `docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md` |
| Machine schema | `docs/contracts/logical-events.schema.json` |
| Harness (volatile + identity cases) | `tools/oracle/selftest_harness.sh` |
| COMPAT-002-named selftest | `tools/oracle/selftest_normalize_compat.sh` |
| Golden dumps | `fixtures/v5/*/readstream.jsonl` |

Usage:

```bash
python3 tools/oracle/normalize_jsonl.py --mode structural [input.jsonl]
# optional: --preserve-seq  --drop-end  -o out.jsonl
```

Default structural mode:

- renumbers `seq` from 0 (unless `--preserve-seq`);
- emits compact JSON (`separators=(",",":")`, keys order `seq`, `tag`, `args`);
- applies the field rules in the table below;
- re-encodes non-integer floats via `normalize_number` / `%.17g` (see COMPAT-003).

---

## Structural mode — volatile rules (shipped)

These rules match **current** `normalize_args` / `normalize_number` behavior. Changing them is a contract change and requires board + test updates.

| # | Location | Field | Rule | Sentinel / result |
|---|----------|-------|------|-------------------|
| V1 | `COMMENT` | all `args` | Replace entire args list | `["<COMMENT>"]` |
| V2 | `ATTRIBUTE` where `key == "basetime"` | value | Replace value | `"<BASETIME>"` |
| V3 | `ATTRIBUTE` where `key == "application"` | value | If empty/None → `<APP>`; if path-like string → **basename**; else keep string (empty basename → `<APP>`) | basename or `"<APP>"` |
| V4 | `NEW_FID` | last arg (`name`) | If string is path-like → **basename**; bare names / package-like strings unchanged | basename only when path-like |
| V5 | any tag | numeric args | Non-string values through `normalize_number` (ints stable; floats re-boxed; see COMPAT-003) | stable dump form |
| V6 | stream | `seq` | Renumber from 0 by default | deterministic sequence |

### Path-like definition (shipped)

A string is path-like when any of:

- starts with `/` or `\`;
- matches Windows absolute `^[A-Za-z]:[/\\]`;
- contains `/` or `\` (relative path with a separator).

Basename: last component after normalizing `\` → `/` and stripping trailing slashes (except root). Pure logic in `basename_path` / `looks_like_path` — no `pathlib` dependency.

### Not volatile under structural mode (must not be rewritten as “noise”)

Normalization **must not** erase or rewrite:

| Class | Examples |
|-------|----------|
| Timing ticks | `TIME_LINE` / `TIME_BLOCK` first arg (integer ticks) |
| Counts / ids | `fid`, `line`, call `count`, `depth`, option values that are profile semantics |
| Event order / multiplicity | tag sequence after normalize |
| Source text | `SRC_LINE` text (exact bytes as dump emits) |
| Sub / eval names that are not path-like | bare filenames, `main::leaf`, eval labels without separators |
| Call edge identities | `SUB_CALLERS` called/caller strings (except float NV re-encoding) |
| Report / HTML surfaces | out of scope of this dump normalizer |
| PIDs | currently **kept** (not rewritten); optional future rule — see open items |

---

## Worked examples (from golden fixtures)

Fixture: `fixtures/v5/default-calls1/readstream.jsonl`.

| Input (conceptual) | After structural normalize |
|--------------------|----------------------------|
| `COMMENT` `"Perl profile database. …\n"` | `["<COMMENT>"]` |
| `ATTRIBUTE` `["basetime","1786111723"]` | `["basetime","<BASETIME>"]` |
| `ATTRIBUTE` `["application","/tmp/…/workload.pl"]` | `["application","workload.pl"]` |
| `NEW_FID` `…,"/tmp/…/workload.pl"` | last arg → `"workload.pl"` |
| `NEW_FID` `…,"/usr/lib/…/warnings.pm"` | last arg → `"warnings.pm"` |

Mutating basetime / application path / COMMENT text / NEW_FID absolute prefix **must** mismatch on raw compare and **must** match after both sides are normalized (see selftests).

---

## Explicitly out of scope / not yet normalized

Tracked so full COMPAT-002 acceptance is not over-claimed:

| Surface | Status |
|---------|--------|
| PID / PPID values | not rewritten (fixture-stable for same-host captures) |
| `ATTRIBUTE` keys other than basetime/application | values kept (numeric re-encode only) |
| Version / xs_version / zlib comment content | COMMENTs collapsed; attribute version strings kept |
| Compression level bytes on wire | binary-level; not JSONL |
| HTML whitespace / report timestamps | report parity (COMPAT-006), not this tool |
| Generated `addtimestamp` filename suffixes | BASE-005 / packaging; not dump args |
| Ordering reordering of independent finalization records | **forbidden** — order is semantic |

---

## Open items (block full COMPAT-002 freeze)

| ID | Item | Notes |
|----|------|-------|
| OI-N-01 | PID / process-id volatility policy | May need rewrite or fixture pinning for cross-host corpus |
| OI-N-02 | Complete volatile attribute key list beyond basetime/application | OI-002-03 vocabulary |
| OI-N-03 | Eval-name / `nameevals` path forms | Option matrix (BASE-005) |
| OI-N-04 | Report/HTML/CSV volatile rules | Separate COMPAT-006 surface |
| OI-N-05 | Version-string attributes in structural mode | Keep vs sentinel |
| OI-N-06 | Whether `mtime` / `size` on NEW_FID are volatile | Currently kept exact |

---

## Forbidden normalizations (semantic)

The following **must always fail** comparison when changed (harness mutates these):

1. Timing tag identity (`TIME_LINE` ↔ `TIME_BLOCK`).  
2. Timing tick values.  
3. Non-volatile string payloads (e.g. `SRC_LINE` text, subnames) when intentionally mutated.  
4. Dropping or reordering mapped events.

---

## Cross-links

| Artifact | Path |
|----------|------|
| COMPAT-001 | `docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md` |
| COMPAT-003 (float / ticks policy) | `docs/contracts/COMPAT-003_PRECISION_NUMERIC_POLICY.md` |
| Task definition | `docs/plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md` § COMPAT-002 |
| Dump schema volatiles section | `docs/schemas/canonical-event-dump-v0.md` |

---

## Provisional acceptance (this landing)

Done for board **COMPAT-002-NORM** when:

1. This document matches shipped `normalize_jsonl.py` structural rules (V1–V6).  
2. Selftests prove: double-normalize identity; pre-normalize volatile mismatch; post-normalize match.  
3. Docs do **not** claim full plan COMPAT-002 or COMPAT-001 freeze.

Not done until open items OI-N-* and plan acceptance (every ignored field class from the full COMPAT-002 work list) close.

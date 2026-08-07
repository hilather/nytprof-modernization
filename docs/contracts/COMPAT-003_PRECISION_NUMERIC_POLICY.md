# COMPAT-003 — Precision and numeric-conversion policy (provisional)

**Status:** provisional / in-progress  
**Task:** COMPAT-003  
**Board ID:** COMPAT-003-PREC  
**Date:** 2026-08-07  
**Depends on:** BASE-003 (timing/lifecycle notes), COMPAT-001 (field domains)  
**Related:** COMPAT-002 (dump float re-encoding for structural compare)

---

## Scope and non-claims

This document records the **provisional numeric policy** for Phase-0 differential work and the first-slice Rust model:

- statement/block **ticks are integer tick counts** (exact);
- floating **seconds** appear only at historical API / display / certain NV wire boundaries;
- dump JSON float stability follows `normalize_number` / `%.17g` in the shipped normalizer.

It does **not**:

- freeze v6 tick signedness or wire encoding (ADR-Q002 / FMT-004);
- freeze exotic native-NV widths or cross-platform NV layouts (ADR-Q013 / RUST-005);
- freeze tick→seconds display formatting for HTML/CSV/report beyond structural dumps;
- close OI-003-01 (elapsed+overflow composition) or OI-003-02 (call-return units).

Full plan COMPAT-003 acceptance remains **in-progress**.

---

## Binding principles

1. **Integer ticks on the storage / model path.** Statement and block samples (`time_line` / `time_block`) carry composed **integer ticks**. Do not reduce precision by storing only floating seconds for those events on the dual-compare path.  
2. **No silent FP accumulation for counts.** Call counts, line call totals, depths, fids, and similar counters are exact integers.  
3. **Float conversion at boundaries only.** Conversion from ticks to seconds (or other display units) happens at legacy API, report, and explicitly NV-typed fields — not as a lossy rewrite of statement timing samples.  
4. **Unsupported layouts are not guessed.** Exotic `nv_size` / endian combinations must fail closed or use an explicit provenance path (RUST-005 later), never a silent reinterpret cast.  
5. **Dump compare uses stable float text, not “close enough” epsilons** for structural golden JSONL (see below). Semantic equality of integers remains bit-exact.

---

## Domains

Aligned with COMPAT-001 type codes and BASE-003:

| Domain | Meaning | Policy (provisional) |
|--------|---------|----------------------|
| `i` ticks | Statement/block elapsed ticks as logical integers | Exact; dump as JSON integer when representable; do not normalize away |
| `u` counts/ids | fid, line, pid, depth, flags, call count, … | Exact integers |
| `n` NV | Native Perl floating image (process times, sub_callers incl/excl/reci, sub_return incl/excl — units open OI-003-02) | Preserve wire/oracle value; dump as JSON number; structural normalize re-encodes via `%.17g` |
| clock scale | `ticks_per_sec` / `clock_id` attributes | Metadata; conversion factor for display, not a rewrite of stored ticks |

### Statement / block ticks

| Topic | Policy |
|-------|--------|
| Wire (v5) | `I32 elapsed` + `U32 overflow` writers (`NYTP_write_time_line` / `time_block`) — composition **open** OI-003-01 |
| Logical event | `time_line.ticks` / `time_block.ticks` as integer domain `i` |
| Aggregation (A4 / A4b) | Sum integer ticks and counts without FP intermediate for the totals path |
| Mutation tests | Tick ±1 must fail structural compare after normalize (`selftest_harness.sh`) |

### Floating seconds and API/display

| Boundary | Policy |
|----------|--------|
| Legacy Perl API projections | May expose seconds or NV as 6.15 does; exact mapping vectors later |
| HTML / CSV / text report | Display formatting **not frozen** beyond first-slice MVP; must not alter underlying model integers |
| `SUB_RETURN` / `SUB_CALLERS` NV fields | Treated as `n` until OI-003-02 freezes units (seconds vs ticks vs mixed) |
| `PID_START` / `PID_END` times | NV images; not statement ticks |

---

## Dump float policy (structural normalize)

Shipped implementation: `tools/oracle/normalize_jsonl.py` → `normalize_number`.

| Input | Output |
|-------|--------|
| `bool` | unchanged |
| `int` | unchanged |
| `float` that is integral and `abs(x) ≤ 2**53` | convert to `int` (stable JSON integer) |
| other finite `float` | `float(f"{x:.17g}")` then JSON-encode (stable enough for equal values after load) |
| NaN | string `"<NAN>"` |
| +Inf / −Inf | `"<INF>"` / `"<-INF>"` |

Notes:

- Structural mode applies `normalize_number` to non-string args on all tags (and to ATTRIBUTE values that are non-string after basetime/application rules).  
- This is a **compare stability** policy for JSONL dumps, not a claim that `%.17g` is the only acceptable report display format.  
- Integer ticks must remain integers through this path (they are not “blurred” into floats).

Cross-link: volatile non-numeric rules live in [COMPAT-002](COMPAT-002_VOLATILE_NORMALIZATION.md).

---

## What is NOT frozen

| Item | Why open |
|------|----------|
| NV width / layout across platforms | ADR-Q013; RUST-005 / TEST-017 |
| Tick → seconds **display** formatting (decimal places, rounding mode for UI) | Report/COMPAT-006; not dump structural policy |
| v6 integer tick width / signedness on wire | ADR-Q002, FMT-004 |
| elapsed + overflow → logical ticks exact formula | OI-003-01 |
| Call-return incl/excl units | OI-003-02 |
| Strict v6→v5 representability failure modes | FMT-013 / TOOL-005 |
| Merge across clock domains | ADR-Q023 |
| Fake-clock certified discount accounting | TEST-003 / BASE-003 |

---

## Open items blocking full freeze

| ID | Item | Unblock |
|----|------|---------|
| OI-003-01 | elapsed+overflow → logical ticks | XS + golden vectors |
| OI-003-02 | call-return numeric units | calls=1/2 fixtures + XS audit |
| OI-P-01 | Document exact display rounding for reports | COMPAT-006 / REPORT tasks |
| OI-P-02 | Native-NV provenance and reject matrix | RUST-005, ADR-Q013 |
| OI-P-03 | Checked accumulation overflow policy for long runs | TEST-019, FMT-004 |
| OI-P-04 | ticks_per_sec edge cases / clock anomaly | OI-003-06 |

---

## Implementation and evidence

| Artifact | Role |
|----------|------|
| `baseline/inventories/timing-lifecycle-notes.md` | BASE-003 numeric/lifecycle notes |
| `docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md` | field domains `i` / `n` |
| `tools/oracle/normalize_jsonl.py` | dump `normalize_number` / `%.17g` |
| `tools/oracle/selftest_harness.sh` | tick mutation must fail compare |
| `tools/oracle/selftest_normalize_compat.sh` | COMPAT-002/003-oriented normalize evidence |
| `docs/schemas/canonical-event-dump-v0.md` | dump numeric JSON conventions |
| `docs/schemas/aggregate-comparison-v0.md` | aggregate integer totals (A4/A4b/…) |

---

## Cross-links

| Artifact | Path |
|----------|------|
| COMPAT-001 | `docs/contracts/COMPAT-001_LOGICAL_EVENT_CONTRACT.md` |
| COMPAT-002 | `docs/contracts/COMPAT-002_VOLATILE_NORMALIZATION.md` |
| Task definition | `docs/plan/01_NON_NEGOTIABLES_AND_COMPATIBILITY_CONTRACT.md` § COMPAT-003 |
| ADR queue (NV / ticks) | `docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md` |

---

## Provisional acceptance (this landing)

Done for board **COMPAT-003-PREC** when:

1. Policy above is written and linked from COMPAT-001.  
2. Shipped normalizer float behavior is documented and matches code.  
3. Statement/block ticks remain exact integers on dump/model compare paths; open items are listed.

Not done until OI-003-01/02, display rounding, and NV portability close for full plan acceptance.

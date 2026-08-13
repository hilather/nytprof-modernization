# SEC-012 release security review checklist (v0)

**Status:** **done (MVP / checklist)** — operator- and reviewer-usable review notes  
**Board IDs:** `P02-SEC-CUT`, `SEC-012-CHECKLIST-MVP`  
**Date:** 2026-08-12  
**Does not supersede:** [SECURITY_FUZZ_HARDENING_PACKAGE_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/SECURITY_FUZZ_HARDENING_PACKAGE_v0.md), [DROP_IN_DOD_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md), ADRs 0001–0010

This file is the shipped **SEC-012** path: a real in-repo **release security review checklist**. Walk it before any **GA marketing** claim. Honesty (plain): covered surfaces are listed below; residual threats stay open; this is not independent sign-off and not GA marketing. It is not SEC-012 complete.

P01 remains a **GA-candidate** honesty cut. Landing this checklist does **not** flip P01 into final GA.

---

## How to use

1. Reviewer reads [SECURITY_FUZZ_HARDENING_PACKAGE_v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/SECURITY_FUZZ_HARDENING_PACKAGE_v0.md) (covered surfaces + residual threats).
2. Run the shipped fuzz entry (do **not** reimplement decode):

   ```bash
   bash scripts/ci/sec002_continuous_fuzz_mvp.sh
   # or, when cargo is present:
   bash tools/oracle/selftest_security_fuzz.sh
   ```

   Honest `SKIP:` when cargo is absent. Never put `crates/` on oracle `PERL5LIB`.
3. Tick the review items below against **evidence already in tree**. Do not invent a parallel decoder or a “green because we said so” row.
4. Leave residual items **open**. Do **not** treat a completed walk as independent release security sign-off.

---

## Covered surfaces (this checklist)

| Surface | Shipped entry | Evidence |
|---------|---------------|----------|
| v5 decode | `nytprof_format_v5::decode_all` / `decode_path` | `crates/nytprof-format-v5/tests/decode_fuzz.rs` |
| v5 verify | `nytprof_report::verify_profile` | `crates/nytprof-report/tests/decode_fuzz.rs` |
| v6 always-inflate EVENT (C sinks) | `e3_decode_writer_bytes` / `decode_decoded_event_profile` | `crates/nytprof-format-v6/tests/decode_fuzz.rs` on `fixtures/v6/from-c/**` |
| Oversize lengths | fail-closed `MAX_*` before large alloc | format-v6 unit tests |
| Collector batch (COL-005) | arena copy; SV clobber after append | `collector/t/test_batch_fast.c` `test_sv_lifetime` |
| Collector fork state (COL-002 MVP) | `begin_fork` / `end_fork_*` | `collector/t/test_lifecycle_seq.c` `test_fork_split_seq_reset` |

Deterministic batteries must return `Ok` or `Err` only — **never panic**. Empty / half / bad-magic inputs must `Err`.

---

## Residual threats (do **not** close from this walk)

| Residual | Honesty |
|----------|---------|
| Full SEC-002 continuous fuzz (cargo-fuzz / AFL / scheduled **deep** corpus) | **deferred** — P02 job MVP only wraps the existing battery |
| Independent SEC-012 release security sign-off | **open** — this file is a checklist, not a signed attestation |
| Full COL-015 fork/PID + compressor inheritance | **residual** |
| SEC-003 salvage / recovery freeze | **open** |
| SEC-004 decompression-bomb corpus depth | **partial** |
| SEC-006 / SEC-007 HTML + filesystem matrices | **open** beyond out-dir + escape MVP |
| SEC-008 FFI unsafe audit | **open** until product FFI ships |
| E3-mixed multi-kind C fuzz corpus | **residual** (EVENT C fixtures only) |
| `BUILD-003-FULL` / S2 / `PRODUCT-V6-COLLECT-EL8` / R3–R4 runtime flips | **residual** — not a security-review close |

---

## Review items (human walk)

Mark only with evidence from shipped tests / smokes. Unchecked items stay residual.

### Decode / report (untrusted profile bytes)

- [ ] v5 `decode_fuzz` battery green (`selftest_security_fuzz.sh` or `cargo test -p nytprof-format-v5 decode_fuzz`)
- [ ] v5 verify `decode_fuzz` battery green (`nytprof-report`)
- [ ] v6 `v6_decode_fuzz` / `fuzz_truncated_mutations_v6` green on C fixtures
- [ ] Oversize-length paths fail closed before large allocation
- [ ] Incomplete / truncated streams fail closed (COMPAT-010)

### Collector / attach

- [ ] Batch SV-lifetime unit evidence still present (`test_sv_lifetime`)
- [ ] Fork-state MVP unit evidence still present (`test_fork_split_seq_reset`)
- [ ] Product D1-B attach remains `collection_default` **v5** (capability JSON)
- [ ] D1-B `format=v6` still fail-closed (G05)

### Isolation / packaging

- [ ] Oracle `PERL5LIB` never contains `crates/`
- [ ] P01 notes still deny **SEC-012 complete**, PAUSE upload, and R3/R4 flips
- [ ] No public P1–P4 performance claim from this review

### Explicitly out of this walk (leave open)

- [ ] Independent third-party / dedicated security sign-off (not this checklist)
- [ ] cargo-fuzz / AFL / sanitizer matrix / deep corpus
- [ ] GA marketing language on P01

---

## What this file is **not**

| Claim | Status |
|-------|--------|
| Independent release security sign-off | **not** claimed |
| SEC-012 complete | **not** claimed |
| GA marketing / final GA | **not** claimed (P01 stays GA-candidate) |
| Full SEC-002 continuous fuzz | **not** claimed (job MVP only) |
| S2 dual_path rewrite | **not** claimed |
| R3 / R4 runtime default flip | **not** claimed |

Related: SEC-002 job MVP [`scripts/ci/sec002_continuous_fuzz_mvp.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/sec002_continuous_fuzz_mvp.sh) + [`.github/workflows/sec002-fuzz-mvp.yml`](https://github.com/hilather/nytprof-modernization/blob/main/.github/workflows/sec002-fuzz-mvp.yml). Schema: [`docs/schemas/p02-sec-cut-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/p02-sec-cut-mvp-v0.md).

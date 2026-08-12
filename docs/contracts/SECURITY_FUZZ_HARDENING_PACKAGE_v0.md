# Security / fuzz hardening package (provisional) — v0

**Status:** provisional R2-stable runway package (**PR-C03**)  
**Board ID:** `SEC-FUZZ-HARDENING-MVP`  
**Plan refs:** SEC-001 (threat model subset), SEC-002 (deterministic offline battery only), COL-005 / COL-015 threat coverage notes  
**Depends on:** COL-007 product E3-EVENT (PR-B09), DECODE-FUZZ-MVP, COMPAT-010 fail-closed  
**Gate:** offline smoke `tools/oracle/selftest_security_fuzz.sh` (+ cargo batteries); offline_gate step 12 when cargo present  
**Schema:** [`docs/schemas/security-fuzz-hardening-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/security-fuzz-hardening-mvp-v0.md)

---

## Goal

Ship an **honest security/fuzz hardening package** for the modernized stack that:

1. Catalogues **in-scope threats** for v5 decode/report, v6 always-inflate EVENT consumers, and collector **v5/v6 sinks** (batch + lifecycle/fork state).
2. Records that **no open critical/high** issues are known for the covered surfaces under current deterministic batteries and unit fail-closed tests.
3. Extends offline **deterministic mutation batteries** to **v6 C-writer sinks** (and keeps v5 DECODE-FUZZ-MVP green).
4. Documents **batching** and **fork** threat mitigations with existing unit evidence — without claiming full SEC-002 continuous fuzz or full COL-015.

This package is **not** a release security certification (SEC-012) and **not** a continuous fuzz program (SEC-002 full).

---

## Findings status (PR-C03 cut)

| Severity | Open count (covered surfaces) | Notes |
|----------|-------------------------------|--------|
| **Critical** | **0** | No known critical findings against deterministic batteries + oversize fail-closed paths |
| **High** | **0** | No known high findings against covered v5/v6 decode sinks + COL-005 SV-lifetime + COL-002 fork state MVP |
| Medium / low / residual | see Residual honesty | Continuous fuzz, full COL-015, salvage, codec bomb depth, SEC-006/007 product audits residual |

**Covered surfaces (this package):**

| Surface | Entry | Evidence |
|---------|-------|----------|
| v5 decode | `nytprof_format_v5::decode_all` / `decode_path` | `crates/nytprof-format-v5/tests/decode_fuzz.rs` |
| v5 verify | `nytprof_report::verify_profile` | `crates/nytprof-report/tests/decode_fuzz.rs` |
| v6 always-inflate EVENT (C sinks) | `e3_decode_writer_bytes` / `decode_decoded_event_profile` | `crates/nytprof-format-v6/tests/decode_fuzz.rs` on `fixtures/v6/from-c/**` |
| Oversize lengths (v6 frame / string / event-body) | `parse_chunk_frame`, `decode_string_blob`, event-body limits | unit tests in `chunk` / `string` / `event_body` (fail-closed before large alloc) |
| Collector batch (COL-005) | arena copy; SV clobber after append | `collector/t/test_batch_fast.c` `test_sv_lifetime` |
| Collector fork state (COL-002 MVP) | `begin_fork` / `end_fork_*`; no emit in `FORK_SPLIT` | `collector/t/test_lifecycle_seq.c` `test_fork_split_seq_reset` |
| Collector v5/v6 wire sinks | real writers under `collector/` | `test_v5_wire` / `test_v6_*` + product E3 |

---

## Threat model (scoped)

Profile bytes and collector event paths are **untrusted** unless the caller establishes trust. Scoped threats and mitigations:

| Threat | Mitigation | Owner evidence | Residual |
|--------|------------|----------------|----------|
| Oversize length → RAM exhaustion | Fail-closed before allocation (`MAX_CHUNK_PAYLOAD`, `MAX_STRING_BYTES`, `MAX_EVENT_BODY_BYTES`, TIME_*_RUN caps) | unit tests + this package battery (no panic / Err path) | Global resource-limit **API** (full SEC-001) residual |
| Corrupt / truncated profile accepted as complete | COMPAT-010; decode/verify Result-only; incomplete-stream contract | DECODE-FUZZ-MVP + v6 decode fuzz + fail-closed smokes | Full salvage/recovery freeze (SEC-003) residual |
| Decompression bomb / extreme expansion | Codec adapters + declared length caps; CRC optional verify | unit codec fail-closed; v6 packing/mid-stream fixtures | Full SEC-004 bomb corpus residual |
| Packing state desync → wrong analytics | E3-EVENT product with C; CRC seal on C writer | `e3_c_*` / `e3_c_writer_parity.sh` | E3-mixed residual |
| Report path traversal / HTML injection | out-dir safety + HTML escape (prior MVP) | html-outdir-safety + report paths | Full SEC-006 browser matrix residual |
| **Batch buffer UAF of Perl SV-like bytes (COL-005)** | Copy into side arena before emit returns; clobber-after-append test | `test_sv_lifetime` in `test_batch_fast` | Continuous ASAN / full Perl SV integration residual |
| **Fork shared FD / compressor / buffer ownership (COL-015)** | Lifecycle `FORK_SPLIT` gates emits; parent keeps seq / child resets; batch `notify_*` forward | `test_fork_split_seq_reset` + batch notify lifecycle | **Full COL-015** fork/PID stress + shared-FD / compressor inheritance residual (PR-C02b) |
| Dictionary/seq domains after fork | Parent continues seq; child resets on `end_fork_child` | lifecycle MVP | COL-015 ADR notes residual |
| Oracle contamination | Never `crates/` or `collector/` install on oracle `PERL5LIB` | packaging isolation smokes | — |
| Privacy / phone-home | No network telemetry in offline stack | capability / dual-path policy | SEC-011 product permissions residual |

---

## Offline fuzz / property batteries

Deterministic only (PR / offline_gate friendly). **Not** cargo-fuzz / AFL / continuous corpus.

| Battery | Package | Input | Pass criterion |
|---------|---------|-------|----------------|
| DECODE-FUZZ-MVP (pre-existing) | format-v5 + report | empty / bad magic / half / prefixes / XOR of default-calls1 | `Ok` or `Err` only; never panic; fail-closed empty/half/magic |
| **V6-DECODE-FUZZ-MVP** (this PR) | format-v6 | empty / bad magic / half / prefixes / XOR of C fixtures (`absolute`, `packing`, `dict`) via `e3_decode_writer_bytes` / `decode_decoded_event_profile` | `Ok` or `Err` only; never panic; empty/half/magic **must** `Err`; full golden **must** `Ok` |
| Oversize fail-closed (pre-existing units) | format-v6 | declared lengths above `MAX_*` | `Err` before large allocation |
| Batch SV lifetime (pre-existing) | collector | clobber caller buffer after append | flushed child still sees original bytes |
| Fork state MVP (pre-existing) | collector | begin/end fork parent+child | no emit in `FORK_SPLIT`; seq policy |

How to run:

```bash
# Full security/fuzz package smoke (v5 + v6 + optional collector)
bash tools/oracle/selftest_security_fuzz.sh

# Focused cargo filters
cargo test -p nytprof-format-v5 decode_fuzz_no_panic -- --nocapture
cargo test -p nytprof-format-v5 fuzz_truncated_mutations -- --nocapture
cargo test -p nytprof-report decode_fuzz_no_panic -- --nocapture
cargo test -p nytprof-format-v6 v6_decode_fuzz -- --nocapture
cargo test -p nytprof-format-v6 fuzz_truncated_mutations_v6 -- --nocapture

# Collector threat evidence (when CC present)
make -C collector test
```

---

## Residual honesty (do **not** claim)

| Topic | Status |
|-------|--------|
| Full SEC-002 continuous fuzz (cargo-fuzz / AFL / scheduled deep corpus) | **deferred** |
| SEC-012 independent security release review | **open** (R2-stable / release) |
| Full COL-015 fork/PID + buffered sink ownership / signal matrix | **residual** (PR-C02b) — state MVP only |
| Full SEC-001 global resource-limit library/CLI API | **partial** (constants + fail-closed; no unified config surface) |
| SEC-003 salvage / recovery freeze | **open** |
| SEC-004 decompression-bomb corpus depth | **partial** |
| SEC-006 full HTML/script injection browser suite | **open** beyond out-dir + escape MVP |
| SEC-007 full filesystem symlink/race matrix | **open** beyond html out-dir MVP |
| SEC-008 FFI unsafe audit | **open** until product FFI ships |
| Product dual-sink hostile merge (SEC-010) | **open** |
| Wire freeze authenticity (checksums ≠ crypto signatures) | **documented** — CRC detects accidental corruption only |
| E3-mixed multi-kind C fuzz corpus | **residual** (EVENT C fixtures only in this package) |

---

## Relationship to plan tasks

| Plan task | This package |
|-----------|--------------|
| SEC-001 | Scoped threat model + limit constants; **not** full limit API |
| SEC-002 | Offline deterministic batteries + smoke harness stub; **not** continuous jobs |
| SEC-003..012 | Residual / not claimed |
| RUST-018 / TEST-016 | Partial offline property/mutation coverage only |
| COL-005 | Batching UAF threat **covered** by existing lifetime tests (catalogued here) |
| COL-015 | Fork threat **documented** + lifecycle MVP evidence; full suite residual |

---

## Board placement

| ID | Status | Evidence |
|----|--------|----------|
| `SEC-FUZZ-HARDENING-MVP` | **done** (package MVP) | this contract + schema + `decode_fuzz` (v5+v6) + `selftest_security_fuzz.sh` |
| `DECODE-FUZZ-MVP` | done (pre-existing) | v5/report batteries |
| `COL-005-BATCH-MVP` | done (scaffold) | SV lifetime evidence reused |
| `COL-002-LIFECYCLE-MVP` | done (scaffold) | fork state MVP evidence reused |
| `COL-015` | residual | full fork/PID stress not claimed |

---

## Revision rule

Closing continuous-fuzz, COL-015, or SEC-012 rows requires a matrix/board revision and new evidence. Do not mark R2-stable security complete from this package alone.

# P02 SEC cut MVP (v0)

**Board IDs:** `P02-SEC-CUT`, `SEC-012-CHECKLIST-MVP`, `SEC-002-CONTINUOUS-FUZZ-MVP`  
**Status:** **done (MVP / checklist / job)**  
**Not:** independent SEC-012 sign-off; full SEC-002 cargo-fuzz / AFL / deep corpus; GA marketing; S2; R3/R4 flip

## Goal

1. Ship a real **SEC-012** release-security **checklist** that names covered surfaces, residual threats, and what is not claimed.
2. Ship a thin **SEC-002** continuous-fuzz **job/script** that invokes the existing shipped fuzz entry — not a reimplemented decoder.
3. Honest `SKIP:` when cargo is absent.

## Shipped paths

| Role | Path |
|------|------|
| SEC-012 checklist | [`docs/contracts/SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/SEC_012_RELEASE_REVIEW_CHECKLIST_v0.md) |
| SEC-002 wrapper | [`scripts/ci/sec002_continuous_fuzz_mvp.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/ci/sec002_continuous_fuzz_mvp.sh) |
| SEC-002 workflow | [`.github/workflows/sec002-fuzz-mvp.yml`](https://github.com/hilather/nytprof-modernization/blob/main/.github/workflows/sec002-fuzz-mvp.yml) |
| Existing fuzz entry | [`tools/oracle/selftest_security_fuzz.sh`](https://github.com/hilather/nytprof-modernization/blob/main/tools/oracle/selftest_security_fuzz.sh) (`decode_fuzz` batteries) |
| P02 smoke | [`scripts/packaging/p02_sec_cut_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/p02_sec_cut_smoke.sh) |

## SEC-002 wrapper behavior

1. Fail closed if `PERL5LIB` contains `/crates/`.
2. If `cargo` is not on `PATH`: print `SKIP:` and exit 0.
3. Else `exec` `tools/oracle/selftest_security_fuzz.sh` (v5 + v6 `decode_fuzz`; optional collector units).

The workflow must call this wrapper (or the same shipped `selftest_security_fuzz.sh` / `cargo test` `decode_fuzz`). It is **not** cargo-fuzz, AFL, or a scheduled deep corpus. A weekly rerun of the same deterministic battery is the continuous **MVP** only.

## Explicit non-goals

| Topic | Status |
|-------|--------|
| Independent security sign-off | residual |
| SEC-012 complete / GA marketing | residual |
| cargo-fuzz / AFL / sanitizer matrix | residual |
| S2 / `BUILD-003-FULL` / `PRODUCT-V6-COLLECT-EL8` | residual |

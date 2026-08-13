# ADR-0010 - Signed CI prebuilt nytprof-cli for EL8 tools RPM (KD-13)

- **Status:** **accepted (policy)** — K02 `nytprof-cli.spec` **landed (MVP)**; signed publish/verify **pipeline residual**
- **Date:** 2026-08-12
- **Accepted:** 2026-08-12 (PR-K03; hard gate for **K02**)
- **Owners/approvers:** packaging / build-release lead; architecture review group
- **Related ADR-Q:** [ADR-Q016](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md) **EL8 tools slice only** (does **not** close CPAN/source native distribution); MSRV number remains [ADR-Q017](https://github.com/hilather/nytprof-modernization/blob/main/docs/plan/18_OPEN_QUESTIONS_AND_ADR_QUEUE.md)
- **Related tasks/risks/gates:** design **KD-13** / **KD-22**; **PR-K03** (this ADR) **hard-gates PR-K02**; residual [`EL8-RPM-TOOLS`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/R1_RESIDUAL_READINESS_MATRIX_v0.md); dual-path [`docs/BUILD_SUPPORT_POLICY.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md); module RPM **K01** / **KD-21** (unchanged); BUILD-004 / BUILD-006 prebuilt matrix residual
- **Decision scope/version:** how the Rocky/EL8 **`nytprof-cli` tools RPM** obtains a native binary. Not a module-RPM change; not a rustc version freeze; not a CPAN tarball layout change; not an implemented CI publish job

## Context

Rocky/EL8 is the advertised RPM companion tier (KD-2). Native tools live in a **separate** `nytprof-cli` package from the Perl/XS module (KD-12). Two historically tempting ways to put a Rust CLI into that RPM both fail the dual-path / EL8 constraints:

| Tempting path | Why it is not primary |
|---------------|------------------------|
| System EL8 `rustc` / distro Cargo | Typically far below workspace needs; not an advertised product toolchain |
| `rustup` (or `cargo`) inside mock `%build` | Pulls a toolchain into the EL8 buildroot; network/trust surface; contradicts cargo-free mock for the module path and operator air-gap expectations |

User-final **Q-prebuilt** / **KD-13** already froze the direction: EL8 `nytprof-cli` comes from **signed CI prebuilt** artifacts. **KD-22** makes this ADR a **hard gate** for K02 — no tools RPM spec without this policy.

**Residual honesty:** signed artifact **pipeline** is still **not implemented**. K02 landed [`packaging/rpm/nytprof-cli.spec`](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/nytprof-cli.spec) on this contract (unpack + fail-closed verify; no rustup-in-mock). Live signed tarball / publish job remains residual.

Numbering coordination (PLAN `8c9b1a63` + product-completion K-track):

| Number | Owner | Topic |
|--------|-------|--------|
| 0001–0002 | PR-B01 | v6 packing / FOOTER string-pool candidates |
| 0004 | PR-B00 | Collector packaging / source-tree |
| 0007 | PR-B13 | Production v6 writer backend (C baseline) |
| **0010** | **PR-K03 (this ADR)** | Signed CI prebuilt `nytprof-cli` for EL8 tools RPM |

Do **not** reuse 0010 as a K02 implementation act. Do **not** treat this ADR as closing all of ADR-Q016.

## Evidence

| Item | Path / note |
|------|-------------|
| KD-13 / Q-prebuilt / KD-22 | [`docs/PRODUCT_COMPLETION_DROP_IN_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/PRODUCT_COMPLETION_DROP_IN_v0.md) (rev 4); [`docs/contracts/DROP_IN_DOD_v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/contracts/DROP_IN_DOD_v0.md) |
| Dual-path (legacy never requires Cargo) | [`docs/BUILD_SUPPORT_POLICY.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) |
| Tools ≠ drop-in | KD-1 / KD-12; Annex C.1 — `nytprof-cli` is a **tools companion** |
| Default module RPM = cargo-free D1-B | KD-21; residual `EL8-RPM-MODULE` (K01) |
| Tools residual | `EL8-RPM-TOOLS` — **K03 then K02**; **not** ready |
| CI today | [`.github/workflows/ci-matrix.yml`](https://github.com/hilather/nytprof-modernization/blob/main/.github/workflows/ci-matrix.yml) — `linux-x86_64` + `macos-arm64` **offline_gate** matrix; **not** a prebuilt publish/sign job; workflow comment: “not multi-OS prebuilt binary distribution” |
| Workspace MSRV pin | **Absent as of this ADR date.** No `rust-toolchain` / `rust-toolchain.toml`. Root [`Cargo.toml`](https://github.com/hilather/nytprof-modernization/blob/main/Cargo.toml) has `edition = "2021"` and **no** `rust-version`. ADR-Q017 remains **open**. CI rust-smoke uses `dtolnay/rust-toolchain@stable` (not an MSRV freeze). |
| This PR | **No** RPM spec; **no** artifact pipeline; **no** certification claim |

## Decision

### 1. Signed CI artifacts are the primary EL8 `nytprof-cli` input

For the official Rocky/EL8 **`nytprof-cli` tools RPM**, the native binary **MUST** come from a **signed CI-published prebuilt** that matches this ADR.

- **Not** system EL8 `rustc`.
- **Not** rustup/cargo inside mock (see §5).
- Source `cargo build` on a developer machine remains valid for **optional-native** / prefix installs under BUILD_SUPPORT_POLICY. It is **not** the primary mock path for the EL8 tools RPM.

K02 **must not** land until it consumes this contract (KD-22).

### 2. Signature verify requirements (operators and packagers)

Every official prebuilt used by the tools RPM **MUST** be published with enough material that a packager or operator can **fail closed** before installing bits.

| Check | Required |
|-------|----------|
| **Identity** | Artifact product version, git commit SHA, and target triple match the RPM `Version`/`Release` (or a documented mapping shipped beside the artifact). |
| **Integrity** | SHA-256 of the payload equals the published `SHA256SUMS` line for that filename. |
| **Authenticity** | A detached cryptographic signature over `SHA256SUMS` (or over the payload + sums) verifies against the project’s **published signing identity**. |
| **Platform** | Triple accepted by this RPM is the advertised EL8 tools triple (`linux-x86_64` at minimum; see §3). Reject other triples. |
| **Fail closed** | Missing sums, missing signature, checksum mismatch, or verify failure **aborts** `%prep` / mock. **No** rustup, cargo, or “unsigned fallback” path. |

**Signing identity (policy, mechanism residual):** when the pipeline lands, official artifacts MUST use one (or both) of:

1. Detached OpenPGP/GPG over `SHA256SUMS` with a **project-published** release public key, and/or
2. Sigstore/cosign **keyless** signature bound to this GitHub repository’s release/CI workflow identity.

This ADR does **not** invent key IDs, a workflow filename, or a claim that either mechanism already runs. K02 + the publish job **must name** the chosen mechanism and publish the verify command. Unsigned Actions logs / “I downloaded `nytprof-cli` from a job” are **not** official EL8 tools inputs.

RPM *package* signatures (`rpmsign` / distro keys) are **additional** and do **not** replace prebuilt verify at ingest.

### 3. Artifact layout (what CI must publish)

**Minimum official triple for EL8 tools:** `linux-x86_64` (glibc, ELF x86_64). That is the only triple K02 is allowed to require.

**Layout contract** (names may gain a `v` prefix or a documented `manifest.json` equivalent; the *roles* are required):

```text
nytprof-cli-<version>-linux-x86_64.tar.gz    # payload: nytprof-cli (+ optional nytprof-dump)
SHA256SUMS                                   # one line per payload
SHA256SUMS.<sig>                             # detached signature (extension per chosen mechanism)
manifest.json                                # version, git SHA, target triple, builder rustc --version
```

Payload is a relocatable binary (or a tiny prefix tree). It is **not** a Perl module, **not** an XS `.so`, and **not** a claim of collection attach.

**Honesty about other platforms:**

| Surface | Status under this ADR |
|---------|------------------------|
| `linux-x86_64` prebuilt | **Required** for official EL8 tools RPM (once the pipeline exists) |
| `macos-arm64` / other CI matrix rows | BUILD-006 MVP **test** hosts only. **Not** required EL8 tools inputs. Publishing them later is allowed if they use the same sign+verify contract — **not** implied by a green matrix row. |
| Windows / multi-Perl / multi-rustc matrix | **Out of scope.** Green [`ci-matrix.yml`](https://github.com/hilather/nytprof-modernization/blob/main/.github/workflows/ci-matrix.yml) ≠ certified multi-OS prebuilt distribution. |
| `ubuntu-latest` glibc vs EL8 glibc | **Residual.** Official EL8 tools artifacts MUST be **runnable on advertised Rocky/EL8**. A generic Ubuntu CI binary is **not** automatically acceptable. Exact builder image / manylinux / EL8 sysroot is chosen when the pipeline lands — do not treat current rust-smoke hosts as that image. |

### 4. Builder-image MSRV (policy; no invented rustc version)

Builder images that compile official prebuilts **MUST** use **`rustc` ≥ workspace MSRV**.

Workspace MSRV is the pin recorded in, when present:

- `rust-toolchain` / `rust-toolchain.toml` at the repo root, or
- `[workspace.package] rust-version` / package `rust-version` in root [`Cargo.toml`](https://github.com/hilather/nytprof-modernization/blob/main/Cargo.toml)

**As of 2026-08-12 those pins do not exist** (no `rust-toolchain*` file; [`Cargo.toml`](https://github.com/hilather/nytprof-modernization/blob/main/Cargo.toml) has `edition = "2021"` and no `rust-version`). This ADR **does not** invent an MSRV number and **does not** close ADR-Q017. Until a pin lands, official builders MUST record the exact `rustc --version` in `manifest.json` and MUST be at least the compiler that successfully built the same commit in the job that produced the artifact. Optional-native source builds continue to use whatever installed `rustc` the operator has ([`docs/BUILD_SUPPORT_POLICY.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md) MSRV row remains open).

### 5. No rustup-in-mock

EL8 mock `%build` / `%install` for **`nytprof-cli`**:

| Allowed | Forbidden |
|---------|-----------|
| Fetch or `%{SOURCE}` the signed prebuilt | `rustup`, `cargo`, `rustc` in the buildroot |
| Verify (§2) then unpack into bindir | Compiling the workspace from `crates/` inside mock |
| `%check`: `nytprof-cli capability` / verify on a bundled tiny fixture | “If verify fails, rustup and rebuild” |

Network in mock, if any, is only to retrieve the **already-signed** artifact + sums + signature from the documented publication surface — not to install a Rust toolchain.

### 6. Tools RPM never claims drop-in collection by itself

`nytprof-cli` is a **tools companion** (KD-1, KD-12, Annex C.1).

| Claim | Allowed? |
|-------|----------|
| “Native NYTProf tools” / dump / report / convert (with capability honesty) | Yes, once K02 ships under this policy |
| “Drop-in replacement for Devel::NYTProf” / collection attach / `perl -d:NYTProfM` | **No** — that is the **module** RPM (K01 `perl-NYTProfM`) + D1–D6 |
| `collection_default: v6` | **No** until an executed ADR-0008 flip |

Recommended relation remains `Recommends:` / `Suggests:` `perl-NYTProfM` — weak dep, not a substitute for the module. A tools-only install MUST NOT set `product_xs_attach` or otherwise stamp collection drop-in.

### 7. Module RPM (K01) remains cargo-free D1-B

This ADR **does not** change K01.

| Package | Policy (unchanged) |
|---------|-------------------|
| `perl-NYTProfM` (default EL8) | Cargo-free mock; **D1-B** v5-only link (`libnytp_sink_v5.a` / `-lz`); `format=v6` **fail-closed** (KD-21); Option B (no `Provides: perl(Devel::NYTProf)`) |
| Optional `--with v6_collect` | D1-A on EL8; still **no** Rust in the module build |
| Tools (`nytprof-cli`) | Signed prebuilt only (§1–§5); **not** part of the module `%build` |

K01 may proceed without this ADR. **K02 may not.**

### 8. Residuals (this ADR is policy only)

| Residual | Honesty |
|----------|---------|
| **K02** `packaging/rpm/nytprof-cli.spec` | **Landed (MVP)** — ingest contract only; no unsigned fallback |
| Signed publish / verify pipeline | **Not implemented** |
| `EL8-RPM-TOOLS` | **done (MVP)** — spec; pipeline residual |
| `EL8-RPM-MODULE` | **done (MVP)** (K01 D1-B spec); independent |
| EL8-runnable builder image / glibc | **Not chosen** |
| Signing key / cosign identity | **Not published** |
| ADR-Q016 (CPAN/source native model) | **Still open** except this EL8 tools slice |
| ADR-Q017 MSRV number | **Still open** |
| Multi-OS prebuilt certification | **Not claimed** |

## Exactness and compatibility consequences

| Surface | Effect |
|---------|--------|
| Wire / collector | None. C writer baseline remains ADR-0007; `collection_default` remains **v5** (ADR-0008 unflipped). |
| Dual-path | Legacy-only / P-PRODUCT-LEGACY stay **cargo-free**. Optional-native source builds unchanged. EL8 tools RPM is a **third ingest** (signed binary), not a new “rustc on EL8” tier. |
| Module RPM | Unchanged D1-B (KD-21). |
| CLI capability | Tools package may expose native dump/report/convert bits already claimed by the CLI; it must **not** grow collection or drop-in stamps. |
| Operators without the tools RPM | Module + legacy scripts remain the collection/report path. |
| CPAN / `NYTPROF_NATIVE` | Unchanged: cargo (or a future documented prebuilt) optional; **not** required by this ADR. |

## Alternatives considered

| Alternative | Correctness/compatibility | Performance/storage | Security/reliability | Build/portability | Reason |
|---|---|---|---|---|---|
| **Signed CI prebuilt (this ADR)** | Matches KD-13; tools ≠ drop-in | N/A (packaging) | Checksum + signature; fail closed | No rustc in mock; EL8 glibc residual called out | **Accepted** |
| rustup inside mock `%build` | Might produce a binary | Larger mock; network | Trusts rustup + crates.io in the EL8 buildroot | Breaks air-gap / cargo-free mock story | **Rejected** — KD-13 |
| System EL8 rustc | Likely cannot build workspace | N/A | Old compiler; unpinned deps | Distro rustc ≠ product MSRV | **Rejected** — KD-13 |
| Defer policy until K02 spec | Leaves packagers free to rustup | N/A | Unsigned binaries become default | K02 would invent policy | **Rejected** — KD-22 hard gate |
| Treat green macos/linux CI as official prebuilts | Wrong glibc/OS; unsigned today | N/A | No sign/verify | Over-claims BUILD-006 MVP | **Rejected** |

## Implementation and testing requirements

K02 (and the future publish job) **must**, when they land:

- Ingest **only** artifacts that pass §2.
- Document the verify command in the spec / runbook (absolute links).
- `%check`: run bundled `nytprof-cli capability` (and a tiny fixture verify if shipped); **no** cargo.
- Keep residual `EL8-RPM-TOOLS` **open** until that spec + pipeline exist.
- Add a regression test that mock/`%prep` **fails closed** on a tampered sums file or missing signature (drive the real verify entry point; no stub).
- **Not** implement rustup-in-mock “just for local scratch”.
- **Not** mark BUILD-006 or ADR-Q016 fully closed.

This ADR itself adds **no** tests and **no** workflow.

## Migration, rollout, and rollback

| Item | Policy |
|------|--------|
| Until K02 | Operators use source optional-native (`cargo` / `make native-install`) or skip native CLI. No official EL8 tools RPM. |
| First K02 cut | Document which git SHA / version the prebuilt maps to; pin absolute links to the tag that published the artifact. |
| Rollback | Remove or downgrade `nytprof-cli`; module RPM and v5 profiles remain. Do **not** roll forward by compiling in mock. |
| Key rotation | Superseding note or K02 docs; old signatures stop verifying → fail closed (no unsigned fallback). |

## Revisit triggers

- Need for a **second** official tools triple (e.g. `linux-aarch64`) as an EL8/EL9 requirement.
- Chosen signing mechanism is rejected by Rocky/EPEL ingest rules.
- Evidence that no EL8-runnable builder can be maintained (would require a superseding ADR — not a silent rustup-in-mock return).
- ADR-Q017 lands an MSRV pin in `rust-toolchain*` / `Cargo.toml` (builders follow that pin; this ADR’s wording does not change).
- Maintainer decision to ship signed prebuilts for CPAN/optional-native (still a **separate** ADR-Q016 slice).

## Non-claims

- **Not** K01 or K02 implemented; **not** an RPM spec.
- **Not** a signed artifact pipeline, key ceremony, or GitHub Release attachment.
- **Not** a rustc version / MSRV freeze (ADR-Q017 remains open).
- **Not** multi-OS prebuilt certification; **not** full BUILD-006; **not** “macos CI binary is supported on EL8.”
- **Not** drop-in collection; **not** R3/R4/R5 flips; **not** COL-008; **not** a public performance claim.
- **Not** a close of ADR-Q016 beyond the EL8 tools RPM ingest path.

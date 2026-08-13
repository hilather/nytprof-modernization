# EL8 tools RPM MVP (v0)

**Board ID:** `EL8-RPM-TOOLS`  
**Status:** **done (MVP)** — `nytprof-cli` companion spec + smoke.  
**Not:** signed publish pipeline; tools-alone drop-in; rustup-in-mock; `BUILD-003-FULL`; S2.

**Spec:** [`packaging/rpm/nytprof-cli.spec`](https://github.com/hilather/nytprof-modernization/blob/main/packaging/rpm/nytprof-cli.spec)  
**Policy:** [ADR-0010](https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0010-signed-ci-prebuilt-native-cli.md)  
**Smoke:** [`scripts/packaging/k02_el8_tools_rpm_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/k02_el8_tools_rpm_smoke.sh)

## Contract

| Field | Value |
|-------|--------|
| Name | `nytprof-cli` |
| Role | Tools companion (report/dump/html) |
| Recommends | `perl-NYTProfM` |
| Ingest | Signed CI prebuilt `linux-x86_64` (ADR-0010) |
| mock `%build` | Unpack + verify; **no** rustup / cargo / rustc |
| Drop-in | **No** — collection is the module RPM |

When a native CLI is discoverable, `report --json` of `fixtures/v5/default-calls1/nytprof.out` is leaf **15** / mid **3** / mid→leaf **15**.

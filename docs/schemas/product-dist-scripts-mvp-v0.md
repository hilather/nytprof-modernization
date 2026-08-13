# Product dist scripts MVP (v0)

**Board ID:** `I03-DIST-SCRIPTS`  
**Status:** **done (MVP)** — cargo-free prefix install of EngineDispatch + familiar report script names.  
**Not:** full 6.15 `nytprofhtml` DOM / Reader / Data graft, `BUILD-003-FULL`, `CPAN-TRIAL-READY`, EL8 RPM, S2 dual_path rewrite, COMPAT-007 bless-array Data drop-in.

**Installer:** [`scripts/packaging/install_product_scripts.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/install_product_scripts.sh)  
**Smoke:** [`scripts/packaging/i03_dist_scripts_smoke.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/i03_dist_scripts_smoke.sh)  
**MakeMaker:** `make install-product-scripts` / `make i03-dist-scripts-smoke` (root [`Makefile.PL`](https://github.com/hilather/nytprof-modernization/blob/main/Makefile.PL))  
**Policy:** [`docs/BUILD_SUPPORT_POLICY.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/BUILD_SUPPORT_POLICY.md)

## Install layout (Annex B / I03)

Same product `lib/perl5` tree as I01 (do **not** overwrite I01 debugger files):

```text
$PREFIX/lib/perl5/Devel/NYTProf/EngineDispatch.pm
$PREFIX/lib/perl5/Devel/NYTProf/JsonlData.pm
$PREFIX/lib/perl5/Devel/NYTProf/JsonlReadStream.pm
$PREFIX/lib/perl5/Devel/NYTProf/LegacyBridge.pm
$PREFIX/lib/perl5/Devel/NYTProf/Data.pm          # thin product MVP, if present
$PREFIX/lib/perl5/Devel/NYTProf/ReadStream.pm    # thin product MVP, if present
$PREFIX/bin/nytprof-engine
$PREFIX/bin/nytprofhtml      # exec sibling nytprof-engine html
$PREFIX/bin/nytprofcsv       # exec sibling nytprof-engine csv
$PREFIX/bin/nytprofcg        # exec sibling nytprof-engine callgrind
$PREFIX/nytprof-product-scripts.install   # packaging_i03=1 full_build003=0 cargo_required=0
```

I01 continues to own `$PREFIX/lib/perl5/Devel/NYTProf.pm`, `Devel/NYTProf/Core.pm`, and `auto/Devel/NYTProf/NYTProf.so`.

Default prefix is `$REPO/prefix` via [`resolve_packaging_prefix.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/packaging/resolve_packaging_prefix.sh). Prefer **`NYTPROF_PREFIX`**.

## Cargo-free query path

Installed `$PREFIX/bin/nytprof-engine` adds `@INC` for `../lib/perl5` (product) and `../lib` (dev). `query --json --jsonl PATH` uses `JsonlData` only — **no Cargo.toml**, no cargo, no native CLI.

Evidence (default-calls1 golden JSONL):

```sh
$PREFIX/bin/nytprof-engine query --json --jsonl fixtures/v5/default-calls1/readstream.jsonl
# leaf_returns=15  mid_returns=3  mid_leaf_edge=15
```

`find_repo_root` failing (no workspace above the prefix) must not block this path.

## Wrappers → EngineDispatch

`nytprofhtml` / `nytprofcsv` / `nytprofcg` are tiny product wrappers that `exec` the sibling `nytprof-engine` with actions `html`, `csv`, and `callgrind`. They are **not** copies of `baseline/6.15` `nytprofhtml` (that script needs the full oracle Reader/Data stack).

Native `html`/`csv` still require a discoverable `nytprof-cli` (sibling `$PREFIX/bin/nytprof-cli`, `NYTPROF_NATIVE_CLI`, or repo `prefix/bin` / `target/`). Honest skip in the smoke when no native CLI is present.

## Hard rules

- Cargo is never invoked. CC is not required.
- Never put `crates/` on `PERL5LIB`.
- Do **not** rewrite `dual_path_smoke.sh` / `legacy_only_smoke.sh` (oracle-primary; S2 not claimed).
- `collection_default` stays **v5**.
- Do **not** flip `BUILD-003-FULL`, `full_build003=1`, `CPAN-TRIAL-READY`, EL8 RPM.
- Do **not** claim COMPAT-007 or replace product `Data.pm` with a 6.15 API drop-in.

## Residuals

| Item | Honesty |
|------|---------|
| Full 6.15 `nytprofhtml` DOM / CSS / tablesorter / flame | **not** I03 |
| 6.15 `Run` / `Util` / `Reader` / `FileInfo` graft | **not** I03 |
| COMPAT-007 bless-array `Data` | **residual** |
| `BUILD-003-FULL` / CPAN trial / EL8 RPM | **residual** |
| S2 dual_path primary → P-PRODUCT-LEGACY | **not claimed** |

## Related

- Engine dispatch: [`docs/schemas/perl-engine-dispatch-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-engine-dispatch-mvp-v0.md)
- I01 product XS: [`docs/schemas/product-attach-smoke-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/product-attach-smoke-mvp-v0.md)
- I02 native CLI: [`docs/schemas/native-install-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-install-mvp-v0.md)

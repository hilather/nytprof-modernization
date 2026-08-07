# Documented oracle test subset

## Full suite

```sh
./scripts/baseline/test_oracle.sh
```

Requires build products under `install/` and `src/`. With `test-deps` installed, expect ~4746–4750 subtests.

**Known interaction:** on this host, full `make test` sometimes fails **exactly one** assertion in `t/12-data.t` (approx. test 56: missing-file constructor exception) even though `prove -b t/12-data.t` and short prefixes of the suite pass cleanly. Treat full-suite green as best-effort; for BASE-001 accept:

1. isolation proof (module loaded only from `baseline/6.15/install`);
2. core suite / documented subset green;
3. `t/12-data.t` green when run alone after install.

Core suite without Capture::Tiny / Test::Differences is sufficient for isolation proof when those deps are missing.

## Core subset (no extra CPAN deps)

If `Capture::Tiny` / `Test::Differences` are unavailable:

```sh
cd baseline/6.15/src
export PERL5LIB="$(cat ../oracle-perl5lib.txt)"
prove -b t/00-load.t t/10-run.t t/11-reader.t t/13-fileinfo.t t/14-subinfo.t \
  t/22-readstream.t t/30-util.t t/31-env.t t/40-savesrc.t t/44-model.t \
  t/50-errno.t t/60-forkdepth.t t/80-version.t t/test*.t t/zzz.t
```

## Extra deps for full suite

```sh
cpanm -l baseline/6.15/test-deps Capture::Tiny Test::Differences
export PERL5LIB="baseline/6.15/test-deps/lib/perl5:$(cat baseline/6.15/oracle-perl5lib.txt)"
./scripts/baseline/test_oracle.sh
```

## Report tools (`nytprofhtml`)

`nytprofhtml` needs `File::Which` (declared in upstream `Makefile.PL`). Install into the same local tree (gitignored):

```sh
cpanm -L baseline/6.15/test-deps File::Which
# optional: JSON::MaybeXS for HTML visualization extras (not required for basic HTML site)
```

Smoke: `bash tools/oracle/report_semantic_parity.sh` (auto-bootstraps `File::Which` when a CPAN client is present).

# Unsigned Rocky 8 `nytprof-cli` (test-drive)

**Not** an ADR-0010 signed tools RPM artifact. Rebuild:

```sh
./scripts/packaging/build_el8_nytprof_cli.sh
```

That runs `cargo build --release -p nytprof-cli` inside `rockylinux:8` and writes `nytprof-cli` here (stripped). `perl-NYTProfM` `%install` copies it to `%{_bindir}/nytprof-cli`. Module `%build` stays cargo-free.

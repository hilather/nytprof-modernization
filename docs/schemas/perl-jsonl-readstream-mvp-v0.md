# Pure-Perl JSONL ReadStream bridge MVP (v0)

**Status:** first-slice pure-Perl dump consumer (not full XS ReadStream)  
**Board ID:** `PERL-READSTREAM-JSONL`  
**Not:** `PERL-004` XS `Devel::NYTProf::ReadStream` over binary profiles

**Related:**

- Record shape / tags: [canonical-event-dump-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/canonical-event-dump-v0.md)
- Native dump parity: [native-dump-parity-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/native-dump-parity-mvp-v0.md)
- Engine dispatch (CLI facade): [perl-engine-dispatch-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-engine-dispatch-mvp-v0.md)

## Goal

A **pure-Perl** module under `perl/` that:

1. Consumes native dump JSONL (committed golden **or** subprocess to shipped `nytprof-cli dump`)
2. Invokes callback-style handlers for tags (at least `TIME_LINE` and `SUB_RETURN`)
3. On `fixtures/v5/default-calls1`, observes **main::leaf** returns **15** and **main::mid** returns **3** by **counting real `SUB_RETURN` events** — not hard-coded theater

No XS, no FFI, no oracle `PERL5LIB`. Core `JSON::PP` only.

## Module

| Path | Role |
|------|------|
| `perl/lib/Devel/NYTProf/JsonlReadStream.pm` | JSONL → tag callbacks |
| `perl/t/jsonl_readstream_default_calls1.t` | Fixture aggregation assertions |
| `scripts/packaging/perl_jsonl_readstream_smoke.sh` | Packaging smoke (golden + optional native dump) |

## API

```perl
use Devel::NYTProf::JsonlReadStream qw(for_chunks process_jsonl count_sub_returns);

# Per-tag handlers
process_jsonl($jsonl_path, {
  SUB_RETURN => sub {
    my ($args, $seq) = @_;
    # args: depth, incl_time, excl_time, subname  (index 3 = name)
  },
  TIME_LINE => sub {
    my ($args, $seq) = @_;
    # args: ticks, fid, line
  },
  '*' => sub { my ($tag, $args, $seq) = @_; },  # optional default
});

# Single callback for every record
for_chunks(
  sub { my ($tag, $args, $seq) = @_; ... },
  file => $jsonl_path,
);
for_chunks(
  sub { my ($tag, $args, $seq) = @_; ... },
  from_cli => [ $nytprof_cli, 'dump', $profile_path ],
);

# Convenience aggregation
my $counts = count_sub_returns($jsonl_path);  # { subname => N }
```

### Input sources

| Option | Behavior |
|--------|----------|
| `file => $path` | Read UTF-8 JSONL file |
| `fh => $fh` | Read open handle |
| `from_cli => \@argv` | `open '-|', @argv`; parse stdout as JSONL; non-zero child exit is error |

Exactly one of `file` / `fh` / `from_cli` for `for_chunks`.

## Tag argument shapes (MVP)

Aligned with [canonical-event-dump-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/canonical-event-dump-v0.md):

| Tag | Args order | Notes |
|-----|------------|-------|
| `TIME_LINE` | `ticks, fid, line` | Statement timing |
| `SUB_RETURN` | `depth, incl_time, excl_time, subname` | **subname at index 3** |
| other tags | pass-through | Handlers optional; unknown tags not fatal |
| `_END` | `[]` | Optional trailing synthetic record |

Return counts: each `SUB_RETURN` with `args[3] eq $subname` increments that sub’s return count by 1.

## Fixture contract (default-calls1)

| Field | Value |
|-------|-------|
| Golden dump | `fixtures/v5/default-calls1/readstream.jsonl` |
| Profile | `fixtures/v5/default-calls1/nytprof.out` |
| `main::leaf` `SUB_RETURN` count | **15** |
| `main::mid` `SUB_RETURN` count | **3** |

These numbers must be **derived by iterating dump events**, not asserted from a constant alone without reading the file.

## Smoke

```sh
# Golden path only (no cargo / no oracle PERL5LIB)
prove -Iperl/lib perl/t/jsonl_readstream_default_calls1.t

# Packaging smoke: golden + native dump when CLI available
./scripts/packaging/perl_jsonl_readstream_smoke.sh
```

Native dump generation (optional second path):

```sh
cargo run -q -p nytprof-cli -- dump fixtures/v5/default-calls1/nytprof.out > /tmp/native.jsonl
# then process_jsonl / count_sub_returns on /tmp/native.jsonl → leaf 15 / mid 3
```

## Non-goals

- Full binary-profile `Devel::NYTProf::ReadStream` XS (`PERL-004`)
- Putting `crates/` or candidate `perl/` on oracle `PERL5LIB`
- Replacing native report / HTML / CSV paths
- Full tag inventory freeze beyond callback pass-through

## Acceptance

Done for board **`PERL-READSTREAM-JSONL`** when:

1. Module exists under `perl/lib/Devel/NYTProf/JsonlReadStream.pm` and uses only core Perl + `JSON::PP`
2. Smoke/test on default-calls1 shows **leaf=15** and **mid=3** from counted `SUB_RETURN` events
3. At least one path uses committed golden JSONL; optional path uses live native `dump`
4. Schema linked from first-slice board evidence

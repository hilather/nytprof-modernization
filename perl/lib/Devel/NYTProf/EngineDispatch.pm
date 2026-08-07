package Devel::NYTProf::EngineDispatch;
# Engine / backend dispatch for the Perl nytprof-engine facade.
#
# Spec: docs/schemas/perl-engine-dispatch-mvp-v0.md
# Complements: docs/schemas/engine-selection-mvp-v0.md

use strict;
use warnings;
use Carp       qw(croak);
use Cwd        qw(abs_path getcwd);
use File::Basename qw(dirname);
use File::Spec;
use JSON::PP;

use Devel::NYTProf::LegacyBridge qw(run_legacy_report);
use Devel::NYTProf::JsonlData;

our $VERSION = '0.005';

use Exporter qw(import);
our @EXPORT_OK = qw(
  ALLOWED_ENGINES
  resolve_engine
  select_runtime_engine
  peel_engine_flag
  find_repo_root
  find_native_cli
  run_native
  run_legacy
  run_query
  print_query_results
  run_cli
  dispatch
);

## Allowed engine names for error messages and docs.
use constant ALLOWED_ENGINES => 'native, legacy, auto';

# ---------------------------------------------------------------------------
# Engine resolution (unit-testable pure function)
# ---------------------------------------------------------------------------

## Resolve requested engine from optional CLI flag value and optional env value.
##
## Precedence: $cli overrides $env; both undef/empty -> 'native'.
## Does B<not> collapse C<auto> to C<native>; callers that need a concrete
## runtime path must use L</select_runtime_engine> (prefer-native fallback).
##
## Returns 'native', 'legacy', or 'auto'. Croaks on invalid names.
sub resolve_engine {
    my ( $cli, $env ) = @_;

    my $raw;
    if ( defined $cli && length $cli ) {
        $raw = $cli;
    }
    elsif ( defined $env && length $env ) {
        $raw = $env;
    }
    else {
        $raw = 'native';
    }

    $raw =~ s/^\s+|\s+$//g;
    my $lc = lc $raw;

    return 'native' if $lc eq 'native';
    return 'legacy' if $lc eq 'legacy';
    return 'auto'   if $lc eq 'auto';

    croak "invalid engine '$raw' (allowed: " . ALLOWED_ENGINES . ")";
}

## Choose the concrete runtime engine (C<native> or C<legacy>) for dispatch.
##
## C<$requested> is typically the result of L</resolve_engine>:
##   legacy → always legacy
##   native → always native (find_native_cli may croak later if missing)
##   auto   → native when discoverable; else legacy (STDERR note)
##
## Does not croak when auto falls back; native-explicit missing CLI still fails
## later in L</find_native_cli> / L</run_native>.
sub select_runtime_engine {
    my ( $repo_root, $requested ) = @_;
    croak "select_runtime_engine: repo_root required"
      if !defined $repo_root || !length $repo_root;

    my $req = defined $requested && length $requested ? lc $requested : 'native';
    $req =~ s/^\s+|\s+$//g;

    return 'legacy' if $req eq 'legacy';
    return 'native' if $req eq 'native';

    if ( $req eq 'auto' ) {
        my $ok;
        eval {
            find_native_cli($repo_root);
            $ok = 1;
            1;
        } or do {
            $ok = 0;
        };
        if ($ok) {
            return 'native';
        }
        print STDERR
"nytprof-engine: auto: native CLI not found; using legacy\n";
        return 'legacy';
    }

    croak "select_runtime_engine: invalid engine '$requested' (allowed: "
      . ALLOWED_ENGINES . ")";
}

## Peel a leading --engine=... / --engine ... from argv-style args.
##
## Returns ($flag_value_or_undef, @remaining_args).
## Croaks on missing value or empty --engine=.
sub peel_engine_flag {
    my (@args) = @_;
    my $engine;
    my $i = 0;
    while ( $i <= $#args ) {
        my $a = $args[$i];
        if ( $a =~ /\A--engine=(.*)\z/s ) {
            my $val = $1;
            croak "duplicate --engine flag" if defined $engine;
            croak "--engine requires a value (allowed: " . ALLOWED_ENGINES . ")"
              if !length $val;
            $engine = $val;
            $i++;
            next;
        }
        if ( $a eq '--engine' ) {
            croak "duplicate --engine flag" if defined $engine;
            $i++;
            my $val = $args[$i];
            croak "--engine requires a value (allowed: " . ALLOWED_ENGINES . ")"
              if !defined $val || $val =~ /\A-/;
            $engine = $val;
            $i++;
            next;
        }
        # Stop peeling at first non-global token.
        return ( $engine, @args[ $i .. $#args ] );
    }
    return ($engine);
}

# ---------------------------------------------------------------------------
# Repo / native CLI discovery
# ---------------------------------------------------------------------------

## Locate workspace root containing Cargo.toml and crates/nytprof-cli.
sub find_repo_root {
    my ($start) = @_;
    $start = getcwd() if !defined $start || !length $start;

    my $dir = abs_path($start);
    if ( defined $dir && -f $dir ) {
        $dir = dirname($dir);
    }
    croak "find_repo_root: cannot resolve start path"
      if !defined $dir || !length $dir;

    my $cursor = $dir;
    for ( 1 .. 40 ) {
        if ( -f File::Spec->catfile( $cursor, 'Cargo.toml' )
            && -d File::Spec->catdir( $cursor, 'crates', 'nytprof-cli' ) )
        {
            return $cursor;
        }
        my $parent = dirname($cursor);
        last if $parent eq $cursor;
        $cursor = $parent;
    }
    croak "find_repo_root: no nytprof-modernization workspace above $dir";
}

## Locate the native nytprof CLI binary or a cargo-run recipe.
##
## Order (per docs/schemas/native-install-mvp-v0.md):
## 0. Test hook: NYTPROF_FORCE_NO_NATIVE truthy → croak immediately
## 1. $ENV{NYTPROF_NATIVE_CLI} if set and executable
## 2. $repo_root/prefix/bin/nytprof-cli or prefix/bin/nytprof-dump
##    (stable install via scripts/packaging/install_native.sh)
## 3. $repo_root/target/release/nytprof-dump then target/debug/nytprof-dump
## 4. cargo run -q -p nytprof-cli -- when cargo is on PATH
##
## Test hook: if C<NYTPROF_FORCE_NO_NATIVE=1> (or any non-empty truthy value
## other than C<0>/C<false>/C<no>/C<off>), discovery fails immediately — used
## only by packaging smokes for C<engine=auto> fallback (ENGINE-AUTO-FALLBACK).
## Do not set in production.
##
## Returns a hashref:
##   { mode => 'path',  path => $abs_path }
##   { mode => 'cargo', argv => [ 'cargo', 'run', ... ] }
## Croaks if nothing usable is found.
sub find_native_cli {
    my ($repo_root) = @_;
    croak "find_native_cli: repo_root required"
      if !defined $repo_root || !length $repo_root;

    # Test-only: force discovery failure for auto-fallback smokes.
    if ( _env_truthy( $ENV{NYTPROF_FORCE_NO_NATIVE} ) ) {
        croak
"native CLI not found: NYTPROF_FORCE_NO_NATIVE is set (test hook; discovery skipped)";
    }

    if ( my $env = $ENV{NYTPROF_NATIVE_CLI} ) {
        if ( length $env && ( -x $env || ( -f $env && -r $env ) ) ) {
            return { mode => 'path', path => abs_path($env) // $env };
        }
    }

    for my $rel (
        qw(
          prefix/bin/nytprof-cli
          prefix/bin/nytprof-dump
          target/release/nytprof-dump
          target/debug/nytprof-dump
        )
      )
    {
        my $p = File::Spec->catfile( $repo_root, split m{/}, $rel );
        if ( -x $p || ( -f $p && -r $p ) ) {
            return { mode => 'path', path => abs_path($p) // $p };
        }
    }

    if ( my $cargo = _which('cargo') ) {
        my $manifest = File::Spec->catfile( $repo_root, 'Cargo.toml' );
        return {
            mode => 'cargo',
            argv => [
                $cargo, 'run', '-q',
                '--manifest-path', $manifest,
                '-p', 'nytprof-cli', '--',
            ],
        };
    }

    croak
"native CLI not found: set NYTPROF_NATIVE_CLI, run scripts/packaging/install_native.sh (prefix/bin), build target/{release,debug}/nytprof-dump, or install cargo";
}

# ---------------------------------------------------------------------------
# Native path
# ---------------------------------------------------------------------------

## Run a native (Rust) action via nytprof-dump / cargo.
##
## C<$engine> should already be a concrete runtime choice (C<native>).
## C<auto> is accepted as native-intent for callers that have not run
## L</select_runtime_engine>; C<legacy> croaks.
## $action is a subcommand such as report, verify, html, csv, dump,
## folded, callgrind, or cg (callgrind alias).
## Remaining @extra are passed after the profile path, except for C<csv>
## where optional C<--subs>/C<--edges> flags go before the profile
## (matching nytprof-cli).
##
## Returns the child exit code (0 = success). Streams stdout/stderr.
sub run_native {
    my ( $repo_root, $engine, $action, $profile, @extra ) = @_;
    croak "run_native: repo_root required"
      if !defined $repo_root || !length $repo_root;
    croak "run_native: action required"
      if !defined $action || !length $action;
    croak "run_native: profile required"
      if !defined $profile || !length $profile;

    my $resolved = resolve_engine( $engine // 'native', undef );
    if ( $resolved eq 'legacy' ) {
        croak "run_native: engine=legacy cannot use the native path";
    }
    # auto / native both mean native path here.
    $resolved = 'native';

    my $cli = find_native_cli($repo_root);
    my @cmd;
    if ( $cli->{mode} eq 'path' ) {
        @cmd = ( $cli->{path} );
    }
    elsif ( $cli->{mode} eq 'cargo' ) {
        @cmd = @{ $cli->{argv} };
    }
    else {
        croak "run_native: unknown native cli mode";
    }

    # nytprof-cli: csv [--subs] [--edges] <profile>
    #              html <profile> [-o path | --out-dir DIR]
    #              report|verify|dump|folded|callgrind|cg <profile>
    my $act = lc $action;
    if ( $act eq 'csv' && @extra ) {
        push @cmd, '--engine=native', $action, @extra, $profile;
    }
    else {
        push @cmd, '--engine=native', $action, $profile, @extra;
    }

    return _system_cmd(@cmd);
}

# ---------------------------------------------------------------------------
# Legacy path (oracle under baseline/6.15 -- no Cargo)
# ---------------------------------------------------------------------------

## Run a legacy (oracle) action without Cargo.
##
## Delegates to Devel::NYTProf::LegacyBridge for report/summary/verify/inspect
## and html/csv/dump/folded/callgrind/cg (smoke via the same stream-dump path).
## Returns 0 on success; croaks or returns non-zero on hard failure.
sub run_legacy {
    my ( $repo_root, $action, $profile ) = @_;
    croak "run_legacy: repo_root required"
      if !defined $repo_root || !length $repo_root;
    croak "run_legacy: action required"
      if !defined $action || !length $action;
    croak "run_legacy: profile required"
      if !defined $profile || !length $profile;

    my $act = lc $action;
    unless ( $act eq 'report'
        || $act eq 'summary'
        || $act eq 'verify'
        || $act eq 'inspect'
        || $act eq 'html'
        || $act eq 'csv'
        || $act eq 'dump'
        || $act eq 'folded'
        || $act eq 'callgrind'
        || $act eq 'cg' )
    {
        croak "run_legacy: unsupported action '$action' "
          . "(allowed: report, summary, verify, inspect, html, csv, dump, "
          . "folded, callgrind, cg)";
    }

    # LegacyBridge croaks on hard failure; returns 0 on success.
    # html/csv/dump/folded/callgrind are not fully wired on oracle; smoke via stream dump.
    my $rc = run_legacy_report( $repo_root, $profile );
    if ( $act eq 'verify' || $act eq 'inspect' ) {
        print "OK: legacy verify (oracle ReadStream dump)\n";
    }
    elsif ( $act eq 'html'
        || $act eq 'csv'
        || $act eq 'dump'
        || $act eq 'folded'
        || $act eq 'callgrind'
        || $act eq 'cg' )
    {
        my $label = $act eq 'cg' ? 'callgrind' : $act;
        print
"NOTE: legacy $label not fully wired; ran oracle stream-dump smoke (use --engine=native for real $label)\n";
    }
    return $rc;
}

# ---------------------------------------------------------------------------
# Query path (JsonlData over native dump / golden JSONL — no XS)
# ---------------------------------------------------------------------------

## Answer dump-derived queries by consuming JSONL via JsonlData.
##
## Options (hash):
##   profile => path to nytprof.out  (native dump via find_native_cli)
##   jsonl   => path to JSONL file   (golden or saved dump; no cargo)
##   json    => truthy for structured JSON stdout (QUERY-JSON-MVP /
##              QUERY-JSON-EXPAND)
##
## Exactly one of profile/jsonl is required (jsonl wins if both set).
## Default (human) prints expanded MVP lines (returns/edges + sub_defs +
## source_line + line_calls + sample block_line_calls + PID lifecycle +
## attribute/option when present). With C<json => 1>, prints a single
## JSON object (JSON::PP). See L</print_query_results>.
##
## Returns 0 on success; croaks on hard failure.
sub run_query {
    my ( $repo_root, %opts ) = @_;
    croak "run_query: repo_root required"
      if !defined $repo_root || !length $repo_root;

    my $jsonl   = $opts{jsonl};
    my $profile = $opts{profile};
    my $as_json = $opts{json} ? 1 : 0;

    my $data;
    if ( defined $jsonl && length $jsonl ) {
        croak "run_query: jsonl path not readable: $jsonl"
          unless -f $jsonl && -r $jsonl;
        $data = Devel::NYTProf::JsonlData->from_jsonl($jsonl);
    }
    elsif ( defined $profile && length $profile ) {
        croak "run_query: profile path not readable: $profile"
          unless -f $profile && -r $profile;
        my $cli = find_native_cli($repo_root);
        my @argv;
        if ( $cli->{mode} eq 'path' ) {
            @argv = ( $cli->{path}, '--engine=native', 'dump', $profile );
        }
        elsif ( $cli->{mode} eq 'cargo' ) {
            @argv = ( @{ $cli->{argv} }, '--engine=native', 'dump', $profile );
        }
        else {
            croak "run_query: unknown native cli mode";
        }
        $data = Devel::NYTProf::JsonlData->from_cli( \@argv );
    }
    else {
        croak "run_query: require profile => PATH or jsonl => PATH";
    }

    print_query_results( $data, json => $as_json );
    return 0;
}

## Print dump-derived query results from a JsonlData object (MVP, always-full).
##
## Options (optional trailing hash):
##   json => 1  — emit a single JSON object (QUERY-JSON-MVP /
##                QUERY-JSON-EXPAND) instead of human greppable lines.
##                Human form remains the default.
##
## Human order (stable / grep-friendly) when C<json> is false:
##   1. sub return totals (sorted names) — e.g. C<main::leaf returns=15>
##   2. call-edge totals (sorted) — e.g. C<main::mid -> main::leaf count=15>
##   3. sub_def ranges: prefer key names C<main::leaf> then C<main::mid> if
##      present, then all remaining names sorted:
##        C<sub_def main::leaf fid=1 first=3 last=7>
##   4. C<source_line 1:5=...> when SRC_LINE is present (trailing newline
##      chomped for one-line display)
##   5. C<line_calls 1:5=N> when A4 call count for (1,5) is non-zero
##   6. Up to a few A4b C<block_line_calls fid:bl=N> lines when non-empty
##      (prefer C<1:4> first if present, then remaining keys sorted)
##   7. PID lifecycle: C<pid_start_count=N>, C<pid_end_count=N>, then each
##      C<pid_start pid=... [ppid=...] [start_time=...]> and
##      C<pid_end pid=... [end_time=...]> from JsonlData (no invented fields)
##   8. ATTRIBUTE / OPTION: prefer key attributes
##      (ticks_per_sec, basetime, application, xs_version) and key options
##      (calls, blocks, stmts, compress), then remaining keys sorted when the
##      map is short enough; otherwise key ones only + total counts
##
## JSON object (stable smoke fields; core JSON::PP only; JsonlData APIs only):
##   {
##     "ok": true,
##     "subs": { "main::leaf": 15, "main::mid": 3, ... },
##     "edges": { "main::mid\tmain::leaf": 15, ... },
##     "leaf_returns": 15,
##     "mid_returns": 3,
##     "mid_leaf_edge": 15,
##     "discount_events": 818,
##     "is_stream_complete": true,
##     "incompleteness_reasons": [],
##     "time_line_events": N,
##     "pid_start_events": N,
##     "pid_end_events": N
##   }
## Edge keys use the same TAB-joined form as JsonlData C<call_edge_totals>.
## Convenience integers always present (0 when the name/edge is missing).
## C<incompleteness_reasons> is C<stream_incompleteness_reasons> as a JSON array.
##
## Uses JsonlData APIs only (no reimplementation of aggregation).
sub print_query_results {
    my ( $data, %opts ) = @_;
    croak "print_query_results: JsonlData required"
      unless defined $data && ref($data);

    my $totals = $data->sub_return_totals;
    my $edges  = $data->call_edge_totals;

    if ( $opts{json} ) {
        my $edge_key = "main::mid\tmain::leaf";
        my $reasons  = $data->stream_incompleteness_reasons;
        $reasons = [] unless defined $reasons && ref($reasons) eq 'ARRAY';
        my $obj = {
            ok                     => JSON::PP::true,
            subs                   => { %$totals },
            edges                  => { %$edges },
            leaf_returns           => 0 + ( $totals->{'main::leaf'} // 0 ),
            mid_returns            => 0 + ( $totals->{'main::mid'}  // 0 ),
            mid_leaf_edge          => 0 + ( $edges->{$edge_key}     // 0 ),
            discount_events        => 0 + ( $data->discount_events // 0 ),
            is_stream_complete     => $data->is_stream_complete
              ? JSON::PP::true
              : JSON::PP::false,
            incompleteness_reasons => [ @$reasons ],
            time_line_events       => 0 + ( $data->time_line_events // 0 ),
            pid_start_events       => 0 + ( $data->pid_start_events // 0 ),
            pid_end_events         => 0 + ( $data->pid_end_events   // 0 ),
        };
        my $json = JSON::PP->new->canonical(1)->ascii(1)->encode($obj);
        print $json, "\n";
        return;
    }

    for my $name ( sort keys %$totals ) {
        printf "%s returns=%d\n", $name, $totals->{$name};
    }

    for my $key ( sort keys %$edges ) {
        my ( $caller, $callee ) = split /\t/, $key, 2;
        next unless defined $caller && defined $callee;
        printf "%s -> %s count=%d\n", $caller, $callee, $edges->{$key};
    }

    # A9 sub_defs: key workload names first, then remaining sorted.
    my $defs = $data->sub_defs;
    my %seen_def;
    for my $prefer (qw(main::leaf main::mid)) {
        my $d = $defs->{$prefer};
        next unless defined $d;
        printf "sub_def %s fid=%d first=%d last=%d\n",
          $prefer, $d->{fid}, $d->{first_line}, $d->{last_line};
        $seen_def{$prefer} = 1;
    }
    for my $name ( sort keys %$defs ) {
        next if $seen_def{$name};
        my $d = $defs->{$name};
        next unless defined $d;
        printf "sub_def %s fid=%d first=%d last=%d\n",
          $name, $d->{fid}, $d->{first_line}, $d->{last_line};
    }

    # A8 hot-loop source line (default-calls1 / blocks-calls1 workload).
    my $src = $data->source_line( 1, 5 );
    if ( defined $src ) {
        my $one = $src;
        $one =~ s/\r?\n\z//;
        printf "source_line 1:5=%s\n", $one;
    }

    # A4 statement line calls (blocks-calls1: 780 on 1:5).
    my $line_calls = $data->line_calls( 1, 5 );
    if ( defined $line_calls && $line_calls != 0 ) {
        printf "line_calls 1:5=%d\n", $line_calls;
    }

    # A4b sample block_line keys (blocks-calls1: 1:4 = 810). Cap for readability.
    my $blocks = $data->block_line_totals;
    if ( $blocks && keys %$blocks ) {
        my $max_block_lines = 8;
        my $n               = 0;
        my %printed;
        if ( exists $blocks->{'1:4'} ) {
            my $c = $blocks->{'1:4'}{calls} // 0;
            printf "block_line_calls 1:4=%d\n", $c;
            $printed{'1:4'} = 1;
            $n++;
        }
        for my $bkey ( sort keys %$blocks ) {
            last if $n >= $max_block_lines;
            next if $printed{$bkey};
            my $c = $blocks->{$bkey}{calls} // 0;
            printf "block_line_calls %s=%d\n", $bkey, $c;
            $n++;
        }
    }

    # PID lifecycle (PERL-QUERY-PID-META): counts + dump-derived events only.
    my $pid_start_n = $data->pid_start_count // 0;
    my $pid_end_n   = $data->pid_end_count   // 0;
    if ( $pid_start_n > 0 || $pid_end_n > 0 ) {
        printf "pid_start_count=%d\n", $pid_start_n;
        printf "pid_end_count=%d\n",   $pid_end_n;
        for my $ev ( @{ $data->pid_starts } ) {
            next unless defined $ev && defined $ev->{pid};
            my $line = sprintf 'pid_start pid=%s', $ev->{pid};
            $line .= sprintf ' ppid=%s', $ev->{ppid}
              if exists $ev->{ppid} && defined $ev->{ppid};
            $line .= sprintf ' start_time=%s', $ev->{start_time}
              if exists $ev->{start_time} && defined $ev->{start_time};
            print "$line\n";
        }
        for my $ev ( @{ $data->pid_ends } ) {
            next unless defined $ev && defined $ev->{pid};
            my $line = sprintf 'pid_end pid=%s', $ev->{pid};
            $line .= sprintf ' end_time=%s', $ev->{end_time}
              if exists $ev->{end_time} && defined $ev->{end_time};
            print "$line\n";
        }
    }

    # ATTRIBUTE / OPTION metadata (dump values as-is; key names first).
    my $attrs = $data->attributes;
    if ( $attrs && keys %$attrs ) {
        my @key_attrs =
          qw(ticks_per_sec basetime application xs_version);
        my %seen_attr;
        my $printed_attr = 0;
        for my $k (@key_attrs) {
            next unless exists $attrs->{$k} && defined $attrs->{$k};
            printf "attribute %s=%s\n", $k, $attrs->{$k};
            $seen_attr{$k} = 1;
            $printed_attr++;
        }
        # Remaining sorted when map is short enough; else counts only.
        my $attr_total = scalar keys %$attrs;
        my $max_extra  = 24;
        if ( $attr_total <= $max_extra + @key_attrs ) {
            for my $k ( sort keys %$attrs ) {
                next if $seen_attr{$k};
                next unless defined $attrs->{$k};
                printf "attribute %s=%s\n", $k, $attrs->{$k};
                $printed_attr++;
            }
        }
        elsif ( $printed_attr < $attr_total ) {
            printf "attribute_count=%d\n", $attr_total;
        }
    }

    my $opts = $data->options;
    if ( $opts && keys %$opts ) {
        my @key_opts = qw(calls blocks stmts compress);
        my %seen_opt;
        my $printed_opt = 0;
        for my $k (@key_opts) {
            next unless exists $opts->{$k} && defined $opts->{$k};
            printf "option %s=%s\n", $k, $opts->{$k};
            $seen_opt{$k} = 1;
            $printed_opt++;
        }
        my $opt_total = scalar keys %$opts;
        my $max_extra = 24;
        if ( $opt_total <= $max_extra + @key_opts ) {
            for my $k ( sort keys %$opts ) {
                next if $seen_opt{$k};
                next unless defined $opts->{$k};
                printf "option %s=%s\n", $k, $opts->{$k};
                $printed_opt++;
            }
        }
        elsif ( $printed_opt < $opt_total ) {
            printf "option_count=%d\n", $opt_total;
        }
    }

    return;
}

# ---------------------------------------------------------------------------
# CLI entry
# ---------------------------------------------------------------------------

## Dispatch a requested engine + action. Returns exit code.
##
## C<$engine> may be C<native>, C<legacy>, or C<auto> (from L</resolve_engine>).
## Runtime selection via L</select_runtime_engine>: C<auto> prefers native and
## falls back to legacy when the native CLI is not discoverable.
##
## Action C<query> / C<data-query> always uses the JsonlData path (native dump
## or --jsonl), not legacy oracle PERL5LIB. Prefer C<run_cli> for full argv
## parsing; C<dispatch> accepts C<@extra> as profile/--jsonl tokens.
sub dispatch {
    my ( $repo_root, $engine, $action, $profile, @extra ) = @_;
    my $act = lc( $action // '' );
    if ( $act eq 'query' || $act eq 'data-query' ) {
        my %qopts = _parse_query_extra(
            ( defined $profile && length $profile ? ($profile) : () ),
            @extra
        );
        # %qopts already includes profile / jsonl / json from _parse_query_extra.
        return run_query( $repo_root, %qopts );
    }
    my $requested = resolve_engine( $engine, undef );
    my $runtime   = select_runtime_engine( $repo_root, $requested );
    if ( $runtime eq 'legacy' ) {
        return run_legacy( $repo_root, $action, $profile );
    }
    return run_native( $repo_root, $runtime, $action, $profile, @extra );
}

## Parse query argv fragments into run_query options.
## Accepts: [profile], [--jsonl PATH], [--jsonl=PATH], [--json],
## [--format=json], [--format json], combinations.
## Note: C<--json> (JSON stdout) is distinct from C<--jsonl> (JSONL input).
sub _parse_query_extra {
    my (@args) = @_;
    my ( $profile, $jsonl, $as_json );
    my $i = 0;
    while ( $i <= $#args ) {
        my $a = $args[$i];
        # --jsonl before --json so prefixes do not collide.
        if ( defined $a && $a =~ /\A--jsonl=(.*)\z/s ) {
            croak "query: duplicate --jsonl" if defined $jsonl;
            my $val = $1;
            croak "query: --jsonl requires a path" if !length $val;
            $jsonl = $val;
            $i++;
            next;
        }
        if ( defined $a && $a eq '--jsonl' ) {
            croak "query: duplicate --jsonl" if defined $jsonl;
            $i++;
            my $val = $args[$i];
            croak "query: --jsonl requires a path"
              if !defined $val || $val =~ /\A-/;
            $jsonl = $val;
            $i++;
            next;
        }
        if ( defined $a && $a eq '--json' ) {
            croak "query: duplicate --json / --format=json" if $as_json;
            $as_json = 1;
            $i++;
            next;
        }
        if ( defined $a && $a =~ /\A--format=(.*)\z/s ) {
            my $fmt = lc $1;
            croak "query: --format requires a value" if !length $fmt;
            croak "query: unknown format '$1' (supported: json)"
              unless $fmt eq 'json';
            croak "query: duplicate --json / --format=json" if $as_json;
            $as_json = 1;
            $i++;
            next;
        }
        if ( defined $a && $a eq '--format' ) {
            $i++;
            my $val = $args[$i];
            croak "query: --format requires a value"
              if !defined $val || $val =~ /\A-/;
            my $fmt = lc $val;
            croak "query: unknown format '$val' (supported: json)"
              unless $fmt eq 'json';
            croak "query: duplicate --json / --format=json" if $as_json;
            $as_json = 1;
            $i++;
            next;
        }
        if ( defined $a && length $a && $a !~ /\A-/ ) {
            croak "query: unexpected extra path '$a'" if defined $profile;
            $profile = $a;
            $i++;
            next;
        }
        croak "query: unexpected argument '$a'" if defined $a;
        $i++;
    }
    return (
        profile => $profile,
        jsonl   => $jsonl,
        json    => $as_json ? 1 : 0,
    );
}

## Parse argv and run. Returns process exit code (0 success).
##
##   exit run_cli($repo_root, \@ARGV);
sub run_cli {
    my ( $repo, $argv ) = @_;
    $repo = abs_path($repo) // $repo;
    $argv = [ @{ $argv // [] } ];

    if ( !@$argv || ( @$argv == 1 && $argv->[0] =~ /\A(-h|--help|help)\z/ ) ) {
        _print_usage();
        return 0;
    }

    my ( $engine_cli, @rest );
    eval {
        ( $engine_cli, @rest ) = peel_engine_flag(@$argv);
        1;
    } or do {
        my $err = $@ // 'argument error';
        $err =~ s/\s+at\s+\S+\s+line\s+\d+\.?\s*\z//;
        print STDERR "nytprof-engine: $err\n";
        return 1;
    };

    if ( !@rest || ( $rest[0] // '' ) =~ /\A(-h|--help|help)\z/ ) {
        _print_usage();
        return 0;
    }

    my $action = shift @rest;
    my $act    = lc( $action // '' );

    my $allowed =
         $act eq 'report'
      || $act eq 'summary'
      || $act eq 'verify'
      || $act eq 'inspect'
      || $act eq 'html'
      || $act eq 'csv'
      || $act eq 'dump'
      || $act eq 'folded'
      || $act eq 'callgrind'
      || $act eq 'cg'
      || $act eq 'query'
      || $act eq 'data-query';
    unless ($allowed) {
        print STDERR
"nytprof-engine: unknown subcommand '$action' (report|summary|verify|inspect|html|csv|dump|folded|callgrind|cg|query)\n";
        return 1;
    }

    # Parse profile + action-specific extras.
    # csv:   optional --subs / --edges before profile
    # html:  optional -o PATH / --out-dir DIR after profile
    # query: profile and/or --jsonl PATH (golden JSONL fallback without cargo)
    # others: profile only
    my ( $profile, @extra );
    if ( $act eq 'csv' ) {
        while ( @rest
            && ( $rest[0] eq '--subs' || $rest[0] eq '--edges' ) )
        {
            push @extra, shift @rest;
        }
        $profile = shift @rest;
        if (@rest) {
            print STDERR "nytprof-engine: unexpected arguments: @rest\n";
            return 1;
        }
    }
    elsif ( $act eq 'html' ) {
        $profile = shift @rest;
        # Pass through remaining flags (-o, --out-dir, values) to native CLI.
        @extra = @rest;
    }
    elsif ( $act eq 'query' || $act eq 'data-query' ) {
        # Validate engine flag even though query uses JsonlData, not legacy.
        eval {
            resolve_engine( $engine_cli, $ENV{NYTPROF_ENGINE} );
            1;
        } or do {
            my $err = $@ // 'invalid engine';
            $err =~ s/\s+at\s+\S+\s+line\s+\d+\.?\s*\z//;
            print STDERR "nytprof-engine: $err\n";
            return 1;
        };

        my %qopts;
        eval {
            %qopts = _parse_query_extra(@rest);
            1;
        } or do {
            my $err = $@ // 'query argument error';
            $err =~ s/\s+at\s+\S+\s+line\s+\d+\.?\s*\z//;
            print STDERR "nytprof-engine: $err\n";
            return 1;
        };
        my $q_profile = $qopts{profile};
        my $jsonl     = $qopts{jsonl};
        if ( !defined $q_profile && !defined $jsonl ) {
            print STDERR
"nytprof-engine: Usage: nytprof-engine query [--json] <profile.out>\n"
              . "                   nytprof-engine query [--json] --jsonl <dump.jsonl>\n";
            return 1;
        }
        # Absolute paths so cargo/chdir does not break relative inputs.
        if ( defined $q_profile && $q_profile !~ m{\A/} && -e $q_profile ) {
            my $abs = abs_path($q_profile);
            $q_profile = $abs if defined $abs && length $abs;
        }
        if ( defined $jsonl && $jsonl !~ m{\A/} && -e $jsonl ) {
            my $jabs = abs_path($jsonl);
            $jsonl = $jabs if defined $jabs && length $jabs;
        }

        my $rc;
        eval {
            $rc = run_query(
                $repo,
                profile => $q_profile,
                jsonl   => $jsonl,
                json    => $qopts{json} ? 1 : 0,
            );
            1;
        } or do {
            my $err = $@ // 'query failed';
            $err =~ s/\s+at\s+\S+\s+line\s+\d+\.?\s*\z//;
            print STDERR "nytprof-engine: $err\n";
            return 1;
        };
        return defined $rc ? $rc : 1;
    }
    else {
        $profile = shift @rest;
        if (@rest) {
            print STDERR "nytprof-engine: unexpected arguments: @rest\n";
            return 1;
        }
    }

    if ( !defined $profile || !length $profile || $profile =~ /\A-/ ) {
        print STDERR
"nytprof-engine: Usage: nytprof-engine [--engine=...] <report|summary|verify|inspect|html|csv|dump|folded|callgrind|cg|query> <profile.out>\n";
        return 1;
    }

    my $native_action =
        $act eq 'summary' ? 'report'
      : $act eq 'inspect' ? 'verify'
      :                     $act;

    my $engine;
    eval {
        $engine = resolve_engine( $engine_cli, $ENV{NYTPROF_ENGINE} );
        1;
    } or do {
        my $err = $@ // 'invalid engine';
        $err =~ s/\s+at\s+\S+\s+line\s+\d+\.?\s*\z//;
        print STDERR "nytprof-engine: $err\n";
        return 1;
    };

    # Make profile absolute so cargo/chdir does not break relative paths.
    if ( $profile !~ m{\A/} && -e $profile ) {
        my $abs = abs_path($profile);
        $profile = $abs if defined $abs && length $abs;
    }

    my $rc;
    eval {
        $rc = dispatch( $repo, $engine, $native_action, $profile, @extra );
        1;
    } or do {
        my $err = $@ // 'dispatch failed';
        $err =~ s/\s+at\s+\S+\s+line\s+\d+\.?\s*\z//;
        print STDERR "nytprof-engine: $err\n";
        return 1;
    };

    return defined $rc ? $rc : 1;
}

# ---------------------------------------------------------------------------
# Internals
# ---------------------------------------------------------------------------

sub _print_usage {
    print <<'USAGE';
Usage:
  nytprof-engine [--engine=native|legacy|auto] <subcommand> <profile.out>

Subcommands:
  report <profile.out>     Text summary (native) or legacy stream-dump smoke
  summary <profile.out>    Alias for report
  verify <profile.out>     Decode/verify (native) or legacy stream-dump smoke
  inspect <profile.out>    Alias for verify
  html <profile.out>       HTML report (native CLI; legacy = stream-dump smoke)
  csv <profile.out>        CSV export (native CLI; legacy = stream-dump smoke)
  dump <profile.out>       Canonical JSONL dump (native; legacy = smoke)
  folded <profile.out>     Folded-stack lines (native CLI; legacy = smoke)
  callgrind <profile.out>  Callgrind-style text (native CLI; legacy = smoke)
  cg <profile.out>         Alias for callgrind
  query <profile.out>      Dump-derived query via JsonlData (returns/edges/
                           sub_defs/source_line/line_calls/block_line sample/
                           PID lifecycle/attributes/options)
  data-query ...           Alias for query

  html options (native, after profile):
    -o path.html           Write single-file HTML
    --out-dir DIR          Multi-file HTML site
  csv options (native, before profile):
    --subs                 Subroutines CSV only
    --edges                Call-edges CSV only
  query options:
    --jsonl PATH           Use golden/saved JSONL instead of native dump
                           (no cargo; pure-Perl JsonlData only)
                           Default output is always-full MVP (readable)
    --json                 Emit structured JSON object (QUERY-JSON-MVP)
    --format=json          Same as --json (also: --format json)
                           Human greppable lines remain the default

Global options:
  --engine=native|legacy|auto   Backend selection (default: native)
  --engine NAME                 Same as --engine=NAME
  -h, --help                    Show this help

Environment:
  NYTPROF_ENGINE                Same values when --engine is omitted
  NYTPROF_NATIVE_CLI            Optional path to nytprof-cli / nytprof-dump
                                (else: prefix/bin, target/{debug,release}, cargo)
  NYTPROF_FORCE_NO_NATIVE=1     Test hook: make find_native_cli fail immediately
                                (ENGINE-AUTO-FALLBACK smokes only)

Engines:
  native   Rust decode/model/report via prefix/bin, nytprof-dump, or cargo
           (fails if native CLI not discoverable)
  auto     Prefer native; if native CLI not discoverable, fall back to legacy
  legacy   Pinned oracle under baseline/6.15 (no Cargo)

Examples:
  nytprof-engine --engine=native report fixtures/v5/default-calls1/nytprof.out
  nytprof-engine --engine=auto report fixtures/v5/default-calls1/nytprof.out
  nytprof-engine --engine=native verify fixtures/v5/default-calls1/nytprof.out
  nytprof-engine --engine=native csv fixtures/v5/default-calls1/nytprof.out
  nytprof-engine --engine=native html fixtures/v5/default-calls1/nytprof.out
  nytprof-engine --engine=native folded fixtures/v5/default-calls1/nytprof.out
  nytprof-engine --engine=native callgrind fixtures/v5/default-calls1/nytprof.out
  nytprof-engine --engine=native query fixtures/v5/default-calls1/nytprof.out
  nytprof-engine query --jsonl fixtures/v5/default-calls1/readstream.jsonl
  nytprof-engine query --json --jsonl fixtures/v5/default-calls1/readstream.jsonl
  nytprof-engine --engine=legacy report fixtures/v5/default-calls1/nytprof.out
USAGE
}

sub _env_truthy {
    my ($v) = @_;
    return 0 unless defined $v && length $v;
    my $lc = lc $v;
    return 0 if $lc eq '0' || $lc eq 'false' || $lc eq 'no' || $lc eq 'off';
    return 1;
}

sub _which {
    my ($name) = @_;
    return undef unless defined $name && length $name;
    return $name if $name =~ m{[/\\]} && -x $name;
    for my $dir ( File::Spec->path ) {
        my $p = File::Spec->catfile( $dir, $name );
        return $p if -x $p;
    }
    return undef;
}

sub _system_cmd {
    my (@cmd) = @_;
    my $rc = system { $cmd[0] } @cmd;
    if ( $rc == -1 ) {
        print STDERR "nytprof-engine: failed to exec $cmd[0]: $!\n";
        return 127;
    }
    if ( $? & 127 ) {
        return 128 + ( $? & 127 );
    }
    return $? >> 8;
}

1;

__END__

=head1 NAME

Devel::NYTProf::EngineDispatch - engine selection + native/legacy dispatch

=head1 SYNOPSIS

  use Devel::NYTProf::EngineDispatch qw(
    resolve_engine select_runtime_engine run_native run_legacy run_query
  );

  my $req = resolve_engine($cli_flag, $ENV{NYTPROF_ENGINE});
  my $eng = select_runtime_engine($repo_root, $req);
  if ($eng eq 'legacy') {
      exit run_legacy($repo_root, 'report', $profile);
  }
  exit run_native($repo_root, $eng, 'report', $profile);

  # Dump-derived query via JsonlData (native dump or golden JSONL):
  exit run_query($repo_root, profile => $profile);
  exit run_query($repo_root, jsonl   => $jsonl_path);
  exit run_query($repo_root, jsonl   => $jsonl_path, json => 1);

=head1 DESCRIPTION

Thin operator facade before the full XS Data/ReadStream path. See
F<docs/schemas/perl-engine-dispatch-mvp-v0.md>.

Engine selection:

=over 4

=item *

C<resolve_engine> returns the B<requested> name: C<native>, C<legacy>, or
C<auto> (does not collapse C<auto>).

=item *

C<select_runtime_engine($repo, $requested)> chooses the concrete path:
C<legacy> stays legacy; C<native> stays native (missing CLI fails later);
C<auto> tries C<find_native_cli> and falls back to legacy with a STDERR note
when native is not discoverable.

=item *

C<NYTPROF_FORCE_NO_NATIVE=1> is a B<test-only> hook that makes
C<find_native_cli> fail immediately (packaging smokes for auto fallback).

=back

Native actions C<report>, C<summary>, C<verify>, C<inspect>, C<html>,
C<csv>, C<dump>, C<folded>, C<callgrind>, and C<cg> subprocess to
C<nytprof-dump> / C<nytprof-cli> (export formats are not reimplemented in
Perl).

Action C<query> (alias C<data-query>) loads L<Devel::NYTProf::JsonlData>
from a native C<dump> subprocess (via C<find_native_cli>) or from
C<--jsonl PATH> golden JSONL, then prints dump-derived MVP results:
subroutine return totals, call-edge counts, A9 C<sub_def> ranges
(prefer C<main::leaf>/C<main::mid>), A8 C<source_line 1:5>, A4
C<line_calls 1:5> when non-zero, a few A4b C<block_line_calls> samples
when present, PID lifecycle (C<pid_start_count> / C<pid_end_count> /
C<pid_start> / C<pid_end>), and ATTRIBUTE/OPTION lines (key names first).
With C<--json> / C<--format=json> (QUERY-JSON-MVP), stdout is a single
JSON object with C<ok>, C<subs>, C<edges>, C<leaf_returns>,
C<mid_returns>, C<mid_leaf_edge> (human form remains the default when
those flags are absent). Uses JsonlData APIs only. No XS; never puts
C<crates/> on oracle C<PERL5LIB>.

Legacy actions (report/verify/etc.) call
L<Devel::NYTProf::LegacyBridge/run_legacy_report>, which isolates oracle
C<PERL5LIB> to C<baseline/6.15/install> only (no C<crates/>, no candidate
C<perl/>) and proves the profile via C<dump_readstream.pl>. For C<html>,
C<csv>, C<dump>, C<folded>, C<callgrind>, and C<cg> the legacy path is a
stream-dump smoke only (real exports require C<--engine=native>).

B<Residual:> the pure-Rust C<nytprof-cli> still maps C<auto> → C<native>
and does not implement legacy fallback (Perl facade is the required
auto-fallback surface for this wave).

=head1 SEE ALSO

L<Devel::NYTProf::LegacyBridge>, L<Devel::NYTProf::JsonlData>

=cut

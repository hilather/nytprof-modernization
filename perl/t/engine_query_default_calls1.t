#!/usr/bin/env perl
# Integration smoke: nytprof-engine query on default-calls1 via JsonlData.
#
# Board: PERL-ENGINE-QUERY / PERL-ENGINE-QUERY-EXPAND / PERL-QUERY-PID-META
# Expects (from real dump events, golden JSONL — no cargo required):
#   main::leaf returns=15
#   main::mid  returns=3
#   main::mid -> main::leaf count=15
#   sub_def main::leaf fid=1 first=3 last=7
#   sub_def main::mid  fid=1 first=8 last=12
#   source_line 1:5=    $x++ for 1 .. 50;
#   pid_start_count>=1, pid_end_count>=1, matching pid (golden 2975381)
#   attribute ticks_per_sec=... and option calls=...
#
# Usage:
#   perl -Iperl/lib perl/t/engine_query_default_calls1.t
#   prove -Iperl/lib perl/t/engine_query_default_calls1.t
use strict;
use warnings;
use FindBin;
use File::Spec;
use Test::More;
use lib File::Spec->catdir( $FindBin::Bin, '..', 'lib' );

use Devel::NYTProf::EngineDispatch qw(run_query find_repo_root);

my $repo = File::Spec->catdir( $FindBin::Bin, '..', '..' );
$repo = File::Spec->rel2abs($repo);
eval { $repo = find_repo_root($repo); 1; };

my $jsonl = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'default-calls1', 'readstream.jsonl'
);
my $profile = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'default-calls1', 'nytprof.out'
);
my $blocks_jsonl = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'blocks-calls1', 'readstream.jsonl'
);

plan skip_all => "missing fixture $jsonl" unless -f $jsonl;

# ---------------------------------------------------------------------------
# Golden JSONL path (always) — returns/edges + expanded surfaces + pid/meta
# ---------------------------------------------------------------------------
{
    my $out = '';
    open my $fh, '>', \$out or die $!;
    my $old = select $fh;
    my $rc  = run_query( $repo, jsonl => $jsonl );
    select $old;
    close $fh;

    is( $rc, 0, 'run_query(jsonl) exit 0' );
    like( $out, qr/main::leaf returns=15/, 'jsonl: leaf returns=15' );
    like( $out, qr/main::mid returns=3/,   'jsonl: mid returns=3' );
    like(
        $out,
        qr/main::mid -> main::leaf count=15/,
        'jsonl: mid→leaf count=15'
    );
    like(
        $out,
        qr/sub_def main::leaf fid=1 first=3 last=7/,
        'jsonl: sub_def leaf 1/3–7'
    );
    like(
        $out,
        qr/sub_def main::mid fid=1 first=8 last=12/,
        'jsonl: sub_def mid 1/8–12'
    );
    like(
        $out,
        qr/^source_line 1:5=\s*\$x\+\+ for 1 \.\. 50;/m,
        'jsonl: source_line 1:5 hot-loop'
    );

    # PERL-QUERY-PID-META: PID lifecycle + ATTRIBUTE/OPTION (dump-derived)
    like(
        $out,
        qr/^pid_start_count=[1-9][0-9]*$/m,
        'jsonl: pid_start_count >= 1'
    );
    like(
        $out,
        qr/^pid_end_count=[1-9][0-9]*$/m,
        'jsonl: pid_end_count >= 1'
    );
    like(
        $out,
        qr/^pid_start pid=2975381(?:\s|$)/m,
        'jsonl: pid_start pid=2975381 (golden dump-derived)'
    );
    like(
        $out,
        qr/^pid_end pid=2975381(?:\s|$)/m,
        'jsonl: pid_end pid=2975381 (matches start)'
    );
    like(
        $out,
        qr/^attribute ticks_per_sec=/m,
        'jsonl: attribute ticks_per_sec=...'
    );
    like(
        $out,
        qr/^option calls=/m,
        'jsonl: option calls=...'
    );
}

# ---------------------------------------------------------------------------
# blocks-calls1: line_calls + A4b block_line sample (golden JSONL)
# ---------------------------------------------------------------------------
SKIP: {
    skip 'missing blocks-calls1 golden', 3 unless -f $blocks_jsonl;

    my $out = '';
    open my $fh, '>', \$out or die $!;
    my $old = select $fh;
    my $rc  = run_query( $repo, jsonl => $blocks_jsonl );
    select $old;
    close $fh;

    is( $rc, 0, 'run_query(blocks jsonl) exit 0' );
    like( $out, qr/line_calls 1:5=780/, 'blocks: line_calls 1:5=780' );
    like(
        $out,
        qr/block_line_calls 1:4=810/,
        'blocks: block_line_calls 1:4=810'
    );
}

# ---------------------------------------------------------------------------
# Optional: native dump path when a CLI binary (not cargo-only) is available
# ---------------------------------------------------------------------------
SKIP: {
    skip 'missing profile', 10 unless -f $profile;

    my $cli;
    if ( my $env = $ENV{NYTPROF_NATIVE_CLI} ) {
        $cli = $env if -x $env || ( -f $env && -r $env );
    }
    if ( !$cli ) {
        for my $rel (
            qw(
              prefix/bin/nytprof-cli
              prefix/bin/nytprof-dump
              target/release/nytprof-dump
              target/debug/nytprof-dump
            )
          )
        {
            my $p = File::Spec->catfile( $repo, split m{/}, $rel );
            if ( -x $p || ( -f $p && -r $p ) ) {
                $cli = $p;
                last;
            }
        }
    }
    skip 'no native CLI binary for live dump (golden path covered)', 10
      unless $cli;

    my $out = '';
    open my $fh, '>', \$out or die $!;
    my $old = select $fh;
    my $rc;
    eval {
        $rc = run_query( $repo, profile => $profile );
        1;
    } or do {
        select $old;
        fail("run_query(profile) threw: $@");
        last SKIP;
    };
    select $old;
    close $fh;

    is( $rc, 0, 'run_query(profile) exit 0' );
    like( $out, qr/main::leaf returns=15/, 'profile: leaf returns=15' );
    like( $out, qr/main::mid returns=3/,   'profile: mid returns=3' );
    like(
        $out,
        qr/main::mid -> main::leaf count=15/,
        'profile: mid→leaf count=15'
    );
    like(
        $out,
        qr/sub_def main::leaf fid=1 first=3 last=7/,
        'profile: sub_def leaf'
    );
    like(
        $out,
        qr/^source_line 1:5=\s*\$x\+\+ for 1 \.\. 50;/m,
        'profile: source_line 1:5'
    );
    like(
        $out,
        qr/^pid_start_count=[1-9][0-9]*$/m,
        'profile: pid_start_count >= 1'
    );
    like(
        $out,
        qr/^pid_end_count=[1-9][0-9]*$/m,
        'profile: pid_end_count >= 1'
    );
    like(
        $out,
        qr/^attribute ticks_per_sec=/m,
        'profile: attribute ticks_per_sec'
    );
    like( $out, qr/^option calls=/m, 'profile: option calls' );
}

# ---------------------------------------------------------------------------
# JSON-SUBDEF-SOURCE-MVP: query --json greppable A9/A8 samples (golden JSONL)
# ---------------------------------------------------------------------------
{
    my $out = '';
    open my $fh, '>', \$out or die $!;
    my $old = select $fh;
    my $rc  = run_query( $repo, jsonl => $jsonl, json => 1 );
    select $old;
    close $fh;

    is( $rc, 0, 'run_query(jsonl, json=>1) exit 0' );
    require JSON::PP;
    my $obj = JSON::PP->new->decode($out);
    ok( $obj->{ok}, 'json: ok true' );
    is_deeply(
        $obj->{sub_def_leaf},
        { fid => 1, first_line => 3, last_line => 7 },
        'json: sub_def_leaf 1/3–7'
    );
    is_deeply(
        $obj->{sub_def_mid},
        { fid => 1, first_line => 8, last_line => 12 },
        'json: sub_def_mid 1/8–12'
    );
    my $src = $obj->{source_line_1_5} // '';
    like( $src, qr/\$x\+\+/, 'json: source_line_1_5 has $x++' );
    like( $src, qr/1 \.\. 50/, 'json: source_line_1_5 has 1 .. 50' );
    is(
        $src,
        "    \$x++ for 1 .. 50;\n",
        'json: source_line_1_5 exact golden text'
    );
}

done_testing();
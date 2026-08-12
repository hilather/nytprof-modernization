#!/usr/bin/env perl
# Product Data materializer (PR-A06 / OQ-2 / PERL-005 MVP).
#
# Golden JSONL path always runs (no native CLI). Binary from_profile path
# runs when a native CLI binary (or cargo) is discoverable.
#
# Expects default-calls1: leaf=15, mid=3, mid→leaf=15, discount=818.
#
# Usage:
#   prove -Iperl/lib perl/t/data_product_default_calls1.t
use strict;
use warnings;
use FindBin;
use File::Spec;
use Test::More;
use lib File::Spec->catdir( $FindBin::Bin, '..', 'lib' );

use Devel::NYTProf::Data;

my $repo = File::Spec->catdir( $FindBin::Bin, '..', '..' );
$repo = File::Spec->rel2abs($repo);
my $jsonl = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'default-calls1', 'readstream.jsonl'
);
my $profile = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'default-calls1', 'nytprof.out'
);

plan skip_all => "missing fixture $jsonl" unless -f $jsonl;

# ---------------------------------------------------------------------------
# from_jsonl bridge (always; no native CLI)
# ---------------------------------------------------------------------------
{
    my $data = Devel::NYTProf::Data->from_jsonl($jsonl);
    ok( defined $data, 'from_jsonl returns object' );
    isa_ok( $data, 'Devel::NYTProf::Data' );
    is( $data->backend, 'jsonl-file', 'backend jsonl-file' );
    ok( $data->is_product_path, 'is_product_path' );
    is( $data->materializer, 'jsonl-bridge', 'materializer jsonl-bridge' );
    is( $data->claims_compat007_shapes, 0, 'no COMPAT-007 claim' );

    is( $data->sub_returns('main::leaf'), 15, 'jsonl: leaf returns 15' );
    is( $data->sub_returns('main::mid'),  3,  'jsonl: mid returns 3' );
    is(
        $data->call_edge_count( 'main::mid', 'main::leaf' ),
        15,
        'jsonl: mid→leaf edge 15'
    );
    is( $data->discount_events, 818, 'jsonl: discount_events 818' );
    ok( $data->is_stream_complete, 'jsonl: stream complete' );
    ok( $data->records_seen > 0,   'jsonl: records_seen > 0' );
}

# new({ jsonl => ... })
{
    my $data = Devel::NYTProf::Data->new( { jsonl => $jsonl } );
    is( $data->sub_returns('main::leaf'), 15, 'new(jsonl): leaf 15' );
}

# Incomplete fail-closed: header-only craft from real golden lines
{
    my $tmp = File::Spec->catfile( $repo, 'target', 'tmp-data-product-incomplete.jsonl' );
    my $dir = File::Spec->catdir( $repo, 'target' );
    mkdir $dir unless -d $dir;
    open my $in,  '<:encoding(UTF-8)', $jsonl or die $!;
    open my $out, '>:encoding(UTF-8)', $tmp  or die $!;
    my $n = 0;
    while (<$in>) {
        print {$out} $_;
        last if ++$n >= 5;    # tiny prefix — no TIME_* → incomplete
    }
    close $in;
    close $out;

    my $ok = eval {
        Devel::NYTProf::Data->from_jsonl($tmp);
        1;
    };
    my $err = $@;
    ok( !$ok, 'incomplete jsonl croaks by default' );
    like( $err, qr/incomplete|no statement timing/i, 'incomplete error mentions reason' );

    my $salvage = Devel::NYTProf::Data->from_jsonl( $tmp, allow_incomplete => 1 );
    ok( defined $salvage, 'allow_incomplete loads partial' );
    ok( !$salvage->is_stream_complete, 'partial is not complete' );
    unlink $tmp;
}

# ---------------------------------------------------------------------------
# Binary product path when native CLI is available
# ---------------------------------------------------------------------------
SKIP: {
    skip 'missing profile nytprof.out', 12 unless -f $profile;

    my $has_cli = 0;
    if ( my $env = $ENV{NYTPROF_NATIVE_CLI} ) {
        $has_cli = 1 if -x $env || ( -f $env && -r $env );
    }
    if ( !$has_cli ) {
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
                $has_cli = 1;
                last;
            }
        }
    }
    if ( !$has_cli ) {
        # cargo-run path is OK for find_native_cli
        require Devel::NYTProf::EngineDispatch;
        my $cli = eval {
            local $ENV{NYTPROF_FORCE_NO_NATIVE};
            delete $ENV{NYTPROF_FORCE_NO_NATIVE};
            Devel::NYTProf::EngineDispatch::find_native_cli($repo);
        };
        $has_cli = 1 if $cli;
    }
    skip 'no native CLI for binary product path (golden path covered)', 12
      unless $has_cli;

    my $data = Devel::NYTProf::Data->from_profile( $profile, repo_root => $repo );
    ok( defined $data, 'from_profile returns object' );
    isa_ok( $data, 'Devel::NYTProf::Data' );
    is( $data->backend, 'native-cli-jsonl', 'backend native-cli-jsonl' );
    is( $data->materializer, 'thin-native-cli-jsonl',
        'materializer thin-native-cli-jsonl' );
    is( $data->sub_returns('main::leaf'), 15, 'profile: leaf 15' );
    is( $data->sub_returns('main::mid'),  3,  'profile: mid 3' );
    is(
        $data->call_edge_count( 'main::mid', 'main::leaf' ),
        15,
        'profile: mid→leaf 15'
    );
    is( $data->discount_events, 818, 'profile: discount 818' );
    ok( $data->is_stream_complete, 'profile: complete' );

    my $via_new =
      Devel::NYTProf::Data->new( { filename => $profile, repo_root => $repo } );
    is( $via_new->sub_returns('main::leaf'), 15, 'new(filename): leaf 15' );
    is( $via_new->backend, 'native-cli-jsonl', 'new(filename): native backend' );
    ok( $via_new->is_product_path, 'new(filename): product path' );
}

done_testing();

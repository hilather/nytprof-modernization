#!/usr/bin/env perl
# Product Data A4/A4b on blocks-calls1 (PR-A06).
#
# Golden JSONL always; binary when native CLI present.
# line_calls(1,5)=780, block_line_calls(1,4)=810.
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
    $repo, 'fixtures', 'v5', 'blocks-calls1', 'readstream.jsonl'
);
my $profile = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'blocks-calls1', 'nytprof.out'
);

plan skip_all => "missing fixture $jsonl" unless -f $jsonl;

{
    my $data = Devel::NYTProf::Data->from_jsonl($jsonl);
    is( $data->line_calls( 1, 5 ),       780, 'jsonl: line_calls 1:5 = 780' );
    is( $data->block_line_calls( 1, 4 ), 810, 'jsonl: block_line_calls 1:4 = 810' );
    is( $data->sub_returns('main::leaf'), 15, 'jsonl: leaf still 15' );
}

SKIP: {
    skip 'missing profile', 4 unless -f $profile;
    my $has_cli = 0;
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
    if ( !$has_cli && $ENV{NYTPROF_NATIVE_CLI} ) {
        my $e = $ENV{NYTPROF_NATIVE_CLI};
        $has_cli = 1 if -x $e || ( -f $e && -r $e );
    }
    if ( !$has_cli ) {
        require Devel::NYTProf::EngineDispatch;
        my $cli = eval {
            Devel::NYTProf::EngineDispatch::find_native_cli($repo);
        };
        $has_cli = 1 if $cli;
    }
    skip 'no native CLI', 4 unless $has_cli;

    my $data =
      Devel::NYTProf::Data->from_profile( $profile, repo_root => $repo );
    is( $data->line_calls( 1, 5 ),       780, 'profile: line_calls 1:5 = 780' );
    is( $data->block_line_calls( 1, 4 ), 810, 'profile: block_line_calls 1:4 = 810' );
    is( $data->backend, 'native-cli-jsonl', 'profile: native-cli-jsonl backend' );
    is( $data->time_block_events, 916, 'profile: time_block_events 916' );
}

done_testing();

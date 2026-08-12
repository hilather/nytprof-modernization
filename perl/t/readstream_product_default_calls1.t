#!/usr/bin/env perl
# Product ReadStream (PR-A06 / OQ-2 / PERL-004 MVP).
#
# Golden JSONL path always runs. Binary filename path when native CLI present.
#
# Expects default-calls1 SUB_RETURN: leaf=15, mid=3.
#
# Usage:
#   prove -Iperl/lib perl/t/readstream_product_default_calls1.t
use strict;
use warnings;
use FindBin;
use File::Spec;
use Test::More;
use lib File::Spec->catdir( $FindBin::Bin, '..', 'lib' );

use Devel::NYTProf::ReadStream qw(
  for_chunks
  process_profile
  count_sub_returns
  is_product_path
  materializer_kind
  SUB_RETURN_SUBNAME_INDEX
);

my $repo = File::Spec->catdir( $FindBin::Bin, '..', '..' );
$repo = File::Spec->rel2abs($repo);
my $jsonl = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'default-calls1', 'readstream.jsonl'
);
my $profile = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'default-calls1', 'nytprof.out'
);

plan skip_all => "missing fixture $jsonl" unless -f $jsonl;

ok( is_product_path(), 'is_product_path' );
is( materializer_kind(), 'thin-native-cli-jsonl', 'materializer_kind' );

# ---------------------------------------------------------------------------
# jsonl / file bridge
# ---------------------------------------------------------------------------
{
    my %ret;
    my $n = for_chunks(
        sub {
            my ( $tag, $args ) = @_;
            if ( $tag eq 'SUB_RETURN' && @$args > SUB_RETURN_SUBNAME_INDEX ) {
                my $name = $args->[SUB_RETURN_SUBNAME_INDEX];
                $ret{$name}++ if defined $name;
            }
        },
        jsonl => $jsonl,
    );
    ok( $n > 0, 'jsonl for_chunks delivered records' );
    is( $ret{'main::leaf'}, 15, 'jsonl: leaf 15' );
    is( $ret{'main::mid'},  3,  'jsonl: mid 3' );

    my $via_file = count_sub_returns($jsonl);
    is( $via_file->{'main::leaf'}, 15, 'count_sub_returns leaf 15' );
    is( $via_file->{'main::mid'},  3,  'count_sub_returns mid 3' );
}

# ---------------------------------------------------------------------------
# Binary product path
# ---------------------------------------------------------------------------
SKIP: {
    skip 'missing profile nytprof.out', 8 unless -f $profile;

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
        require Devel::NYTProf::EngineDispatch;
        my $cli = eval {
            local $ENV{NYTPROF_FORCE_NO_NATIVE};
            delete $ENV{NYTPROF_FORCE_NO_NATIVE};
            Devel::NYTProf::EngineDispatch::find_native_cli($repo);
        };
        $has_cli = 1 if $cli;
    }
    skip 'no native CLI for binary ReadStream (golden path covered)', 8
      unless $has_cli;

    my %ret;
    my $n = for_chunks(
        sub {
            my ( $tag, $args ) = @_;
            if ( $tag eq 'SUB_RETURN' && @$args > SUB_RETURN_SUBNAME_INDEX ) {
                my $name = $args->[SUB_RETURN_SUBNAME_INDEX];
                $ret{$name}++ if defined $name;
            }
        },
        filename  => $profile,
        repo_root => $repo,
    );
    ok( $n > 0, 'binary for_chunks delivered records' );
    is( $ret{'main::leaf'}, 15, 'binary filename: leaf 15' );
    is( $ret{'main::mid'},  3,  'binary filename: mid 3' );

    my %ret2;
    my $n2 = for_chunks(
        sub {
            my ( $tag, $args ) = @_;
            if ( $tag eq 'SUB_RETURN' && @$args > SUB_RETURN_SUBNAME_INDEX ) {
                my $name = $args->[SUB_RETURN_SUBNAME_INDEX];
                $ret2{$name}++ if defined $name;
            }
        },
        profile   => $profile,
        repo_root => $repo,
    );
    is( $ret2{'main::leaf'}, 15, 'binary profile: leaf 15' );
    is( $n2, $n, 'filename and profile deliver same record count' );

    my %ret3;
    process_profile(
        $profile,
        {
            SUB_RETURN => sub {
                my ($args) = @_;
                return unless @$args > SUB_RETURN_SUBNAME_INDEX;
                my $name = $args->[SUB_RETURN_SUBNAME_INDEX];
                $ret3{$name}++ if defined $name;
            },
        },
        repo_root => $repo,
    );
    is( $ret3{'main::leaf'}, 15, 'process_profile: leaf 15' );
    is( $ret3{'main::mid'},  3,  'process_profile: mid 3' );
    ok( 1, 'binary product ReadStream path exercised' );
}

done_testing();

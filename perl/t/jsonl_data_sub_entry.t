#!/usr/bin/env perl
# Smoke / unit test: JsonlData SUB_ENTRY event multiplicity.
#
# Aggregates real SUB_ENTRY events from committed fixture golden JSONL
# (not hard-coded theater). Multiplicity only — not full call-stack /
# arg freeze.
#
# Fixture facts (derived by independent stream re-count of SUB_ENTRY tags):
#   fixtures/v5/default-calls1  (calls=1): sub_entry_count == 0
#   fixtures/v5/calls2-default  (calls=2): sub_entry_count == 27
#
# Usage:
#   perl -Iperl/lib perl/t/jsonl_data_sub_entry.t
#   prove -Iperl/lib perl/t/jsonl_data_sub_entry.t
use strict;
use warnings;
use FindBin;
use File::Spec;
use Test::More;
use lib File::Spec->catdir( $FindBin::Bin, '..', 'lib' );

use Devel::NYTProf::JsonlData;
use Devel::NYTProf::JsonlReadStream qw(for_chunks);

my $repo = File::Spec->catdir( $FindBin::Bin, '..', '..' );
$repo = File::Spec->rel2abs($repo);

my $default_jsonl = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'default-calls1', 'readstream.jsonl'
);
my $calls2_jsonl = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'calls2-default', 'readstream.jsonl'
);

plan skip_all => "missing fixture $default_jsonl" unless -f $default_jsonl;
plan skip_all => "missing fixture $calls2_jsonl"  unless -f $calls2_jsonl;

sub recount_sub_entry {
    my ($path) = @_;
    my $n = 0;
    for_chunks(
        sub {
            my ( $tag, $args ) = @_;
            if ( $tag eq 'SUB_ENTRY' ) {
                # Schema: caller_fid, caller_line; count every SUB_ENTRY tag.
                $n++;
            }
        },
        file => $path,
    );
    return $n;
}

# ---------------------------------------------------------------------------
# default-calls1 (calls=1): SUB_ENTRY count 0
# ---------------------------------------------------------------------------
{
    my $data = Devel::NYTProf::JsonlData->from_jsonl($default_jsonl);
    ok( defined $data, 'default-calls1: from_jsonl returns object' );
    isa_ok( $data, 'Devel::NYTProf::JsonlData' );
    ok( $data->records_seen > 0, 'default-calls1: records_seen > 0' );

    my $se = $data->sub_entry_events;
    my $sc = $data->sub_entry_count;
    ok( defined $se, 'default-calls1: sub_entry_events defined' );
    is( $sc, $se, 'default-calls1: sub_entry_count alias matches sub_entry_events' );

    my $recount = recount_sub_entry($default_jsonl);
    is( $data->sub_entry_events, $recount,
        'default-calls1: JsonlData sub_entry_events matches stream recount' );
    is( $data->sub_entry_count, $recount,
        'default-calls1: JsonlData sub_entry_count matches stream recount' );
    is( $recount, 0,
        'default-calls1 golden SUB_ENTRY re-count is 0 (calls=1; fixture-derived)' );
    is( $se, 0,
        'default-calls1: JsonlData sub_entry_events is 0' );

    diag("default-calls1 observed sub_entry_events=$se recount=$recount");
}

# ---------------------------------------------------------------------------
# calls2-default (calls=2): SUB_ENTRY count 27
# ---------------------------------------------------------------------------
{
    my $data = Devel::NYTProf::JsonlData->from_jsonl($calls2_jsonl);
    ok( defined $data, 'calls2-default: from_jsonl returns object' );
    isa_ok( $data, 'Devel::NYTProf::JsonlData' );
    ok( $data->records_seen > 0, 'calls2-default: records_seen > 0' );

    my $se = $data->sub_entry_events;
    my $sc = $data->sub_entry_count;
    ok( defined $se, 'calls2-default: sub_entry_events defined' );
    ok( $se > 0, "calls2-default: sub_entry_events > 0 (got $se)" );
    is( $sc, $se, 'calls2-default: sub_entry_count alias matches sub_entry_events' );

    my $recount = recount_sub_entry($calls2_jsonl);
    ok( $recount > 0, "calls2-default: stream recount SUB_ENTRY > 0 (got $recount)" );
    is( $data->sub_entry_events, $recount,
        'calls2-default: JsonlData sub_entry_events matches stream recount' );
    is( $data->sub_entry_count, $recount,
        'calls2-default: JsonlData sub_entry_count matches stream recount' );
    is( $recount, 27,
        'calls2-default golden SUB_ENTRY re-count is 27 (calls=2; fixture-derived)' );
    is( $se, 27,
        'calls2-default: JsonlData sub_entry_events is 27' );

    diag("calls2-default observed sub_entry_events=$se recount=$recount");
}

done_testing();

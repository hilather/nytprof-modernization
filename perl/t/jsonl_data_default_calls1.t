#!/usr/bin/env perl
# Smoke / unit test: JsonlData over default-calls1 golden dump.
#
# Aggregates real SUB_RETURN / SUB_CALLERS events from
# fixtures/v5/default-calls1/readstream.jsonl (not hard-coded theater).
# Expects:
#   main::leaf returns == 15
#   main::mid  returns == 3
#   main::mid → main::leaf edge count == 15
#
# Usage:
#   perl -Iperl/lib perl/t/jsonl_data_default_calls1.t
#   prove -Iperl/lib perl/t/jsonl_data_default_calls1.t
use strict;
use warnings;
use FindBin;
use File::Spec;
use Test::More;
use lib File::Spec->catdir( $FindBin::Bin, '..', 'lib' );

use Devel::NYTProf::JsonlData;

my $repo = File::Spec->catdir( $FindBin::Bin, '..', '..' );
$repo = File::Spec->rel2abs($repo);
my $jsonl = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'default-calls1', 'readstream.jsonl'
);

plan skip_all => "missing fixture $jsonl" unless -f $jsonl;

# ---------------------------------------------------------------------------
# from_jsonl: subroutine returns + call edges
# ---------------------------------------------------------------------------
my $data = Devel::NYTProf::JsonlData->from_jsonl($jsonl);
ok( defined $data, 'from_jsonl returns object' );
isa_ok( $data, 'Devel::NYTProf::JsonlData' );
ok( $data->records_seen > 0, 'records_seen > 0' );

is( $data->sub_returns('main::leaf'), 15,
    'main::leaf returns == 15 (from SUB_RETURN events)' );
is( $data->sub_returns('main::mid'), 3,
    'main::mid returns == 3 (from SUB_RETURN events)' );
is( $data->sub_returns('no::such::sub'), 0,
    'missing sub returns 0' );

my $totals = $data->sub_return_totals;
ok( ref($totals) eq 'HASH', 'sub_return_totals is hashref' );
is( $totals->{'main::leaf'}, 15, 'totals hash: leaf 15' );
is( $totals->{'main::mid'},  3,  'totals hash: mid 3' );

is(
    $data->call_edge_count( 'main::mid', 'main::leaf' ),
    15,
    'mid→leaf call_edge_count == 15 (from SUB_CALLERS events)'
);
is(
    $data->call_edge_count( 'main::RUNTIME', 'main::mid' ),
    3,
    'RUNTIME→mid call_edge_count == 3'
);
is( $data->call_edge_count( 'no::caller', 'no::callee' ), 0,
    'missing edge returns 0' );

my $edges = $data->call_edge_totals;
ok( ref($edges) eq 'HASH', 'call_edge_totals is hashref' );
is( $edges->{"main::mid\tmain::leaf"}, 15, 'edge totals key mid→leaf 15' );

# ---------------------------------------------------------------------------
# A4 line_totals from TIME_LINE (default-calls1 has no TIME_BLOCK)
# ---------------------------------------------------------------------------
my $lines = $data->line_totals;
ok( ref($lines) eq 'HASH', 'line_totals is hashref' );
ok( scalar keys %$lines > 0, 'line_totals non-empty on default-calls1' );
# Sanity: each entry has calls/ticks; convenience line_calls matches
my ($sample_key) = keys %$lines;
ok( exists $lines->{$sample_key}{calls}, 'line_totals entry has calls' );
ok( exists $lines->{$sample_key}{ticks}, 'line_totals entry has ticks' );
my ( $sfid, $sline ) = split /:/, $sample_key, 2;
is( $data->line_calls( $sfid, $sline ), $lines->{$sample_key}{calls},
    'line_calls matches line_totals for sample key' );

# ---------------------------------------------------------------------------
# Independent re-count via stream to prove numbers come from events
# ---------------------------------------------------------------------------
use Devel::NYTProf::JsonlReadStream qw(for_chunks);
my %ret;
my %edge;
for_chunks(
    sub {
        my ( $tag, $args ) = @_;
        if ( $tag eq 'SUB_RETURN' && @$args > 3 ) {
            my $n = $args->[3];
            $ret{$n}++ if defined $n;
        }
        elsif ( $tag eq 'SUB_CALLERS' && @$args >= 9 ) {
            my ( $count, $callee, $caller ) =
              ( $args->[2], $args->[-2], $args->[-1] );
            $edge{"$caller\t$callee"} += int( $count // 0 );
        }
    },
    file => $jsonl,
);

is( $ret{'main::leaf'}, 15, 'independent stream recount: leaf 15' );
is( $ret{'main::mid'},  3,  'independent stream recount: mid 3' );
is( $edge{"main::mid\tmain::leaf"}, 15,
    'independent stream recount: mid→leaf 15' );
is( $data->sub_returns('main::leaf'), $ret{'main::leaf'},
    'JsonlData leaf matches stream recount' );
is(
    $data->call_edge_count( 'main::mid', 'main::leaf' ),
    $edge{"main::mid\tmain::leaf"},
    'JsonlData mid→leaf matches stream recount'
);

done_testing();

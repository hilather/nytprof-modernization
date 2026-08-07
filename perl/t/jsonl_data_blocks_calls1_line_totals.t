#!/usr/bin/env perl
# Smoke / unit test: JsonlData line_totals from TIME_BLOCK on blocks-calls1.
#
# blocks=1 emits TIME_BLOCK (not TIME_LINE). A4 line_totals must still fill
# from the statement (fid, line) field. On fixtures/v5/blocks-calls1:
#   line_calls(1, 5) == 780  (hot loop: $x++ for 1 .. 50)
# Leaf/mid SUB_RETURN counts remain 15 / 3.
#
# Numbers come from iterating real dump events — not hard-coded theater.
#
# Usage:
#   perl -Iperl/lib perl/t/jsonl_data_blocks_calls1_line_totals.t
#   prove -Iperl/lib perl/t/jsonl_data_blocks_calls1_line_totals.t
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
    $repo, 'fixtures', 'v5', 'blocks-calls1', 'readstream.jsonl'
);

plan skip_all => "missing fixture $jsonl" unless -f $jsonl;

# ---------------------------------------------------------------------------
# from_jsonl: line_totals / line_calls from TIME_BLOCK
# ---------------------------------------------------------------------------
my $data = Devel::NYTProf::JsonlData->from_jsonl($jsonl);
ok( defined $data, 'from_jsonl returns object' );
isa_ok( $data, 'Devel::NYTProf::JsonlData' );
ok( $data->records_seen > 0, 'records_seen > 0' );

is( $data->line_calls( 1, 5 ), 780,
    'line_calls(1,5) == 780 (from TIME_BLOCK events)' );
is( $data->line_calls( 99, 99 ), 0, 'missing line_calls returns 0' );

my $lines = $data->line_totals;
ok( ref($lines) eq 'HASH', 'line_totals is hashref' );
ok( scalar keys %$lines > 0, 'line_totals non-empty on blocks-calls1' );
ok( exists $lines->{'1:5'}, 'line_totals has key 1:5' );
is( $lines->{'1:5'}{calls}, 780, 'line_totals{"1:5"}.calls == 780' );
ok( exists $lines->{'1:5'}{ticks}, 'line_totals{"1:5"} has ticks' );
ok( ( $lines->{'1:5'}{ticks} // 0 ) > 0, 'line_totals{"1:5"}.ticks > 0' );

# Same fixture still has leaf/mid returns
is( $data->sub_returns('main::leaf'), 15,
    'blocks-calls1 main::leaf returns == 15' );
is( $data->sub_returns('main::mid'), 3,
    'blocks-calls1 main::mid returns == 3' );

# ---------------------------------------------------------------------------
# Independent re-count: TIME_BLOCK (and TIME_LINE if any) for (1,5)
# ---------------------------------------------------------------------------
use Devel::NYTProf::JsonlReadStream qw(for_chunks);
my $time_block_n = 0;
my $time_line_n  = 0;
my $line5_calls  = 0;
for_chunks(
    sub {
        my ( $tag, $args ) = @_;
        if ( $tag eq 'TIME_BLOCK' ) {
            $time_block_n++;
            # Statement (fid, line) at indices 1 and 2 (same as TIME_LINE).
            if (   defined $args
                && @$args > 2
                && 0 + ( $args->[1] // -1 ) == 1
                && 0 + ( $args->[2] // -1 ) == 5 )
            {
                $line5_calls++;
            }
        }
        elsif ( $tag eq 'TIME_LINE' ) {
            $time_line_n++;
        }
    },
    file => $jsonl,
);

ok( $time_block_n > 0, "TIME_BLOCK events present (n=$time_block_n)" );
is( $time_line_n, 0, 'TIME_LINE events == 0 on blocks-calls1 (blocks=1)' );
is( $line5_calls, 780,
    'independent stream recount: TIME_BLOCK fid=1 line=5 count == 780' );
is( $data->line_calls( 1, 5 ), $line5_calls,
    'JsonlData line_calls(1,5) matches stream recount' );

done_testing();

#!/usr/bin/env perl
# Smoke / unit test: JsonlData A4b block_line_totals from TIME_BLOCK on blocks-calls1.
#
# blocks=1 emits TIME_BLOCK (not TIME_LINE). A4 still fills from statement
# (fid, line); A4b fills from (fid, block_line) at args[3].
# On fixtures/v5/blocks-calls1:
#   line_calls(1, 5) == 780                 (A4 statement line)
#   block_line_totals non-empty
#   block_line_calls(1, 4) == 810           (A4b sample key, if derived from recount)
#
# Numbers come from iterating real dump events — not hard-coded theater.
#
# Usage:
#   perl -Iperl/lib perl/t/jsonl_data_a4b_blocks_calls1.t
#   prove -Iperl/lib perl/t/jsonl_data_a4b_blocks_calls1.t
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
# from_jsonl: A4 line_totals + A4b block_line_totals from TIME_BLOCK
# ---------------------------------------------------------------------------
my $data = Devel::NYTProf::JsonlData->from_jsonl($jsonl);
ok( defined $data, 'from_jsonl returns object' );
isa_ok( $data, 'Devel::NYTProf::JsonlData' );
ok( $data->records_seen > 0, 'records_seen > 0' );

# A4 still correct (statement line)
is( $data->line_calls( 1, 5 ), 780,
    'line_calls(1,5) == 780 (A4 from TIME_BLOCK statement line)' );

# A4b map non-empty; at least one key with calls > 0
my $blocks = $data->block_line_totals;
ok( ref($blocks) eq 'HASH', 'block_line_totals is hashref' );
ok( scalar keys %$blocks > 0, 'block_line_totals non-empty on blocks-calls1' );

my $any_positive = 0;
my $sample_key;
my $sample_calls;
while ( my ( $k, $v ) = each %$blocks ) {
    my $c = $v->{calls} // 0;
    if ( $c > 0 ) {
        $any_positive = 1;
        $sample_key   = $k;
        $sample_calls = $c;
        last;
    }
}
ok( $any_positive,
    "some block_line_totals key has calls > 0"
      . ( defined $sample_key ? " (e.g. $sample_key => $sample_calls)" : '' ) );

# Well-known A4b sample from oracle aggregates / Rust model
is( $data->block_line_calls( 1, 4 ), 810,
    'block_line_calls(1,4) == 810 (A4b sample from TIME_BLOCK block_line)' );
ok( exists $blocks->{'1:4'}, 'block_line_totals has key 1:4' );
is( $blocks->{'1:4'}{calls}, 810, 'block_line_totals{"1:4"}.calls == 810' );
ok( exists $blocks->{'1:4'}{ticks}, 'block_line_totals{"1:4"} has ticks' );
ok( ( $blocks->{'1:4'}{ticks} // 0 ) > 0, 'block_line_totals{"1:4"}.ticks > 0' );

is( $data->block_line_calls( 99, 99 ), 0,
    'missing block_line_calls returns 0' );

# default-style fixtures have empty A4b (sanity via missing key only here)

# ---------------------------------------------------------------------------
# Independent re-count: TIME_BLOCK for A4 (1,5) and A4b (1,4)
# ---------------------------------------------------------------------------
use Devel::NYTProf::JsonlReadStream qw(for_chunks);
my $time_block_n = 0;
my $time_line_n  = 0;
my $line5_calls  = 0;
my $b14_calls    = 0;
for_chunks(
    sub {
        my ( $tag, $args ) = @_;
        if ( $tag eq 'TIME_BLOCK' ) {
            $time_block_n++;
            return unless defined $args && @$args > 2;
            my $fid  = 0 + ( $args->[1] // -1 );
            my $line = 0 + ( $args->[2] // -1 );
            # A4 statement (fid, line)
            if ( $fid == 1 && $line == 5 ) {
                $line5_calls++;
            }
            # A4b (fid, block_line) at args[3]
            if ( @$args > 3 ) {
                my $block_line = 0 + ( $args->[3] // -1 );
                if ( $fid == 1 && $block_line == 4 ) {
                    $b14_calls++;
                }
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
is( $b14_calls, 810,
    'independent stream recount: TIME_BLOCK fid=1 block_line=4 count == 810' );
is( $data->block_line_calls( 1, 4 ), $b14_calls,
    'JsonlData block_line_calls(1,4) matches stream recount' );

done_testing();

#!/usr/bin/env perl
# Smoke / unit test: JsonlData A8 source_lines from SRC_LINE on default-calls1.
#
# Aggregates real SRC_LINE events from
# fixtures/v5/default-calls1/readstream.jsonl (not hard-coded theater).
# Expects (derived from dump; also re-counted independently below):
#   source_line(1, 5) contains $x++ and 1 .. 50
#   exact dump text: "    $x++ for 1 .. 50;\n"
#
# Usage:
#   perl -Iperl/lib perl/t/jsonl_data_source_default_calls1.t
#   prove -Iperl/lib perl/t/jsonl_data_source_default_calls1.t
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
# from_jsonl: source_line / source_lines
# ---------------------------------------------------------------------------
my $data = Devel::NYTProf::JsonlData->from_jsonl($jsonl);
ok( defined $data, 'from_jsonl returns object' );
isa_ok( $data, 'Devel::NYTProf::JsonlData' );
ok( $data->records_seen > 0, 'records_seen > 0' );

my $line5 = $data->source_line( 1, 5 );
ok( defined $line5, 'source_line(1,5) defined' );
like( $line5, qr/\$x\+\+/,   'source_line(1,5) contains $x++' );
like( $line5, qr/1 \.\. 50/, 'source_line(1,5) contains 1 .. 50' );
# Exact dump text from golden SRC_LINE args[2]
is( $line5, "    \$x++ for 1 .. 50;\n",
    'source_line(1,5) exact dump text' );

ok( !defined $data->source_line( 1, 99999 ),
    'missing source_line → undef' );

my $all = $data->source_lines;
ok( ref($all) eq 'HASH', 'source_lines is hashref' );
ok( exists $all->{'1:5'}, 'source_lines has 1:5' );
is( $all->{'1:5'}, $line5, 'source_lines{1:5} matches source_line(1,5)' );
# Shallow copy: mutating return must not clobber internal store
$all->{'1:5'} = 'MUTATED';
is( $data->source_line( 1, 5 ), $line5,
    'source_lines() returns a hash copy' );

# Sanity: a few other workload lines present
ok( defined $data->source_line( 1, 1 ), 'source_line(1,1) defined' );
like( $data->source_line( 1, 1 ), qr/use strict/,
    'source_line(1,1) is use strict' );
ok( defined $data->source_line( 1, 3 ), 'source_line(1,3) defined' );
like( $data->source_line( 1, 3 ), qr/sub leaf/,
    'source_line(1,3) is sub leaf' );

# ---------------------------------------------------------------------------
# Independent re-count via stream (prove text comes from dump events)
# ---------------------------------------------------------------------------
use Devel::NYTProf::JsonlReadStream qw(for_chunks);
my %src_rc;    # "fid:line" => text last write wins
my $src_events = 0;
for_chunks(
    sub {
        my ( $tag, $args ) = @_;
        if ( $tag eq 'SRC_LINE' && defined $args && @$args >= 3 ) {
            my ( $fid, $line, $text ) = @{$args}[ 0 .. 2 ];
            return
              unless defined $fid
              && defined $line
              && defined $text
              && !ref($text);
            $src_rc{"$fid:$line"} = $text;
            $src_events++;
        }
    },
    file => $jsonl,
);

ok( $src_events > 0, 'stream recount saw SRC_LINE events' );
ok( exists $src_rc{'1:5'}, 'stream recount has 1:5' );
like( $src_rc{'1:5'}, qr/\$x\+\+/,   'stream 1:5 contains $x++' );
like( $src_rc{'1:5'}, qr/1 \.\. 50/, 'stream 1:5 contains 1 .. 50' );
is( $src_rc{'1:5'}, "    \$x++ for 1 .. 50;\n",
    'stream 1:5 exact dump text' );
is( $data->source_line( 1, 5 ), $src_rc{'1:5'},
    'JsonlData source_line(1,5) matches stream recount' );

# Full map equality for all keys seen in stream
my $data_map = $data->source_lines;
is_deeply(
    $data_map,
    \%src_rc,
    'JsonlData source_lines matches stream recount map'
);

done_testing();

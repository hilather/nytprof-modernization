#!/usr/bin/env perl
# Smoke / unit test: JsonlReadStream over default-calls1 golden dump.
#
# Aggregates real SUB_RETURN events from fixtures/v5/default-calls1/readstream.jsonl
# (not hard-coded theater). Expects main::leaf returns == 15, main::mid == 3.
#
# Usage:
#   perl -Iperl/lib perl/t/jsonl_readstream_default_calls1.t
#   prove -Iperl/lib perl/t/jsonl_readstream_default_calls1.t
use strict;
use warnings;
use FindBin;
use File::Spec;
use Test::More;
use lib File::Spec->catdir( $FindBin::Bin, '..', 'lib' );

use Devel::NYTProf::JsonlReadStream qw(
  for_chunks
  process_jsonl
  count_sub_returns
  SUB_RETURN_SUBNAME_INDEX
);

my $repo = File::Spec->catdir( $FindBin::Bin, '..', '..' );
$repo = File::Spec->rel2abs($repo);
my $jsonl = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'default-calls1', 'readstream.jsonl'
);

plan skip_all => "missing fixture $jsonl" unless -f $jsonl;

# ---------------------------------------------------------------------------
# count_sub_returns helper (file path)
# ---------------------------------------------------------------------------
my $counts = count_sub_returns($jsonl);
ok( ref($counts) eq 'HASH', 'count_sub_returns returns hashref' );

my $leaf = $counts->{'main::leaf'} // 0;
my $mid  = $counts->{'main::mid'}  // 0;

is( $leaf, 15, 'main::leaf SUB_RETURN count == 15 (from dump events)' );
is( $mid,  3,  'main::mid SUB_RETURN count == 3 (from dump events)' );

# ---------------------------------------------------------------------------
# process_jsonl handlers: SUB_RETURN + TIME_LINE
# ---------------------------------------------------------------------------
my %returns;
my $time_line_n = 0;
my $delivered   = process_jsonl(
    $jsonl,
    {
        SUB_RETURN => sub {
            my ( $args, $seq ) = @_;
            my $name = $args->[SUB_RETURN_SUBNAME_INDEX];
            $returns{$name}++ if defined $name;
        },
        TIME_LINE => sub {
            my ( $args, $seq ) = @_;
            $time_line_n++;
        },
    }
);

ok( $delivered > 0, "process_jsonl delivered handlers (n=$delivered)" );
is( $returns{'main::leaf'}, 15, 'handler aggregation: main::leaf == 15' );
is( $returns{'main::mid'},  3,  'handler aggregation: main::mid == 3' );
ok( $time_line_n > 0, "TIME_LINE events observed (n=$time_line_n)" );

# ---------------------------------------------------------------------------
# for_chunks single callback
# ---------------------------------------------------------------------------
my %by_tag;
my %ret2;
my $n = for_chunks(
    sub {
        my ( $tag, $args, $seq ) = @_;
        $by_tag{$tag}++;
        if ( $tag eq 'SUB_RETURN' ) {
            my $name = $args->[3];
            $ret2{$name}++ if defined $name;
        }
    },
    file => $jsonl,
);

ok( $n > 0, "for_chunks delivered $n records" );
ok( ( $by_tag{SUB_RETURN} // 0 ) >= 18,
    'SUB_RETURN multiplicity >= leaf+mid (18)' );
is( $ret2{'main::leaf'}, 15, 'for_chunks: main::leaf == 15' );
is( $ret2{'main::mid'},  3,  'for_chunks: main::mid == 3' );
ok( ( $by_tag{TIME_LINE} // 0 ) > 0, 'for_chunks saw TIME_LINE' );
# Golden dump includes trailing synthetic _END.
is( $by_tag{_END} // 0, 1, 'golden dump ends with _END' );

# Sanity: totals from file equal helper
is_deeply(
    { map { $_ => $ret2{$_} } qw(main::leaf main::mid) },
    { 'main::leaf' => 15, 'main::mid' => 3 },
    'for_chunks leaf/mid match expected workload returns'
);

done_testing();

#!/usr/bin/env perl
# Smoke / unit test: JsonlData DISCOUNT event multiplicity on default-calls1.
#
# Aggregates real DISCOUNT events from
# fixtures/v5/default-calls1/readstream.jsonl (not hard-coded theater).
# A3 discount_events is event multiplicity only — not exclusive-time
# policy freeze (BASE-003 / TEST-003).
#
# Expects (derived by independent stream re-count of DISCOUNT tags):
#   discount_events == discount_count
#   both match independent for_chunks re-count
#   re-count on this golden is 818 (also in aggregates.oracle.json)
#
# Usage:
#   perl -Iperl/lib perl/t/jsonl_data_discount_default_calls1.t
#   prove -Iperl/lib perl/t/jsonl_data_discount_default_calls1.t
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
# from_jsonl: discount_events / discount_count
# ---------------------------------------------------------------------------
my $data = Devel::NYTProf::JsonlData->from_jsonl($jsonl);
ok( defined $data, 'from_jsonl returns object' );
isa_ok( $data, 'Devel::NYTProf::JsonlData' );
ok( $data->records_seen > 0, 'records_seen > 0' );

my $de = $data->discount_events;
my $dc = $data->discount_count;
ok( defined $de, 'discount_events defined' );
ok( $de > 0, "discount_events > 0 (got $de)" );
is( $dc, $de, 'discount_count alias matches discount_events' );

diag("observed discount_events=$de discount_count=$dc");

# ---------------------------------------------------------------------------
# Independent re-count via stream (prove count comes from dump DISCOUNT tags)
# ---------------------------------------------------------------------------
use Devel::NYTProf::JsonlReadStream qw(for_chunks);
my $recount = 0;
for_chunks(
    sub {
        my ( $tag, $args ) = @_;
        if ( $tag eq 'DISCOUNT' ) {
            # Schema: empty args; still count every DISCOUNT tag.
            $recount++;
        }
    },
    file => $jsonl,
);

ok( $recount > 0, "stream recount DISCOUNT > 0 (got $recount)" );
is( $data->discount_events, $recount,
    'JsonlData discount_events matches stream recount of DISCOUNT tags' );
is( $data->discount_count, $recount,
    'JsonlData discount_count matches stream recount of DISCOUNT tags' );

# Golden recount for this committed fixture (also aggregates.oracle.json A3).
# Assert via re-count, not magic alone: stream-derived $recount must be 818.
is( $recount, 818,
    'default-calls1 golden DISCOUNT re-count is 818 (fixture-derived)' );
is( $de, 818,
    'JsonlData discount_events is 818 for default-calls1 golden' );

done_testing();

#!/usr/bin/env perl
# Smoke / unit test: JsonlData ATTRIBUTE + OPTION metadata on default-calls1.
#
# Aggregates real ATTRIBUTE / OPTION events from
# fixtures/v5/default-calls1/readstream.jsonl (not hard-coded theater).
# Expects (derived from dump; also re-counted independently below):
#   attribute('ticks_per_sec') defined (or basetime)
#   option('calls') defined
#   attributes() / options() match stream re-count maps
#
# Usage:
#   perl -Iperl/lib perl/t/jsonl_data_meta_default_calls1.t
#   prove -Iperl/lib perl/t/jsonl_data_meta_default_calls1.t
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
# from_jsonl: attribute / option / attributes / options
# ---------------------------------------------------------------------------
my $data = Devel::NYTProf::JsonlData->from_jsonl($jsonl);
ok( defined $data, 'from_jsonl returns object' );
isa_ok( $data, 'Devel::NYTProf::JsonlData' );
ok( $data->records_seen > 0, 'records_seen > 0' );

# Required presence: ticks_per_sec (or basetime) and option calls
my $tps  = $data->attribute('ticks_per_sec');
my $base = $data->attribute('basetime');
ok( defined $tps || defined $base,
    'has attribute ticks_per_sec or basetime' );
ok( defined $tps,  'attribute(ticks_per_sec) defined' );
ok( defined $base, 'attribute(basetime) defined' );

my $calls = $data->option('calls');
ok( defined $calls, 'option(calls) defined' );
# Print observed dump value for operator evidence (not a hard-coded invent)
diag("observed option(calls)=$calls");
diag("observed attribute(ticks_per_sec)=$tps") if defined $tps;

ok( !defined $data->attribute('__no_such_attribute__'),
    'missing attribute → undef' );
ok( !defined $data->option('__no_such_option__'),
    'missing option → undef' );

my $attrs = $data->attributes;
ok( ref($attrs) eq 'HASH', 'attributes is hashref' );
ok( exists $attrs->{ticks_per_sec},
    'attributes hash has ticks_per_sec' );
is( $attrs->{ticks_per_sec}, $tps,
    'attributes{ticks_per_sec} matches attribute()' );
my $opts = $data->options;
ok( ref($opts) eq 'HASH', 'options is hashref' );
ok( exists $opts->{calls}, 'options hash has calls' );
is( $opts->{calls}, $calls, 'options{calls} matches option()' );

# Shallow copies: mutating return must not clobber internal store
my $tps_before   = $tps;
my $calls_before = $calls;
$attrs->{ticks_per_sec} = 'MUTATED';
$opts->{calls}          = 'MUTATED';
is( $data->attribute('ticks_per_sec'), $tps_before,
    'attributes() returns a hash copy' );
is( $data->option('calls'), $calls_before,
    'options() returns a hash copy' );

# Sample keys commonly present on default-calls1 (presence only; values from dump)
for my $k (qw(xs_version application perl_version)) {
    ok( exists $data->attributes->{$k}, "attributes has $k" );
}
for my $k (qw(blocks stmts compress)) {
    ok( defined $data->option($k), "option($k) defined" );
}

# ---------------------------------------------------------------------------
# Independent re-count via stream (prove meta comes from dump events)
# ---------------------------------------------------------------------------
use Devel::NYTProf::JsonlReadStream qw(for_chunks);
my %attr_rc;    # key => value last write wins
my %opt_rc;
my $attr_events = 0;
my $opt_events  = 0;
for_chunks(
    sub {
        my ( $tag, $args ) = @_;
        if (   ( $tag eq 'ATTRIBUTE' || $tag eq 'OPTION' )
            && defined $args
            && @$args >= 2 )
        {
            my ( $key, $val ) = @{$args}[ 0, 1 ];
            return
              unless defined $key
              && length $key
              && !ref($key)
              && defined $val
              && !ref($val);
            if ( $tag eq 'ATTRIBUTE' ) {
                $attr_rc{$key} = $val;
                $attr_events++;
            }
            else {
                $opt_rc{$key} = $val;
                $opt_events++;
            }
        }
    },
    file => $jsonl,
);

ok( $attr_events > 0, 'stream recount saw ATTRIBUTE events' );
ok( $opt_events > 0,  'stream recount saw OPTION events' );
ok( exists $attr_rc{ticks_per_sec} || exists $attr_rc{basetime},
    'stream recount has ticks_per_sec or basetime' );
ok( exists $opt_rc{calls}, 'stream recount has option calls' );

if ( exists $attr_rc{ticks_per_sec} ) {
    is( $data->attribute('ticks_per_sec'), $attr_rc{ticks_per_sec},
        'JsonlData attribute(ticks_per_sec) matches stream recount' );
}
if ( exists $attr_rc{basetime} ) {
    is( $data->attribute('basetime'), $attr_rc{basetime},
        'JsonlData attribute(basetime) matches stream recount' );
}
is( $data->option('calls'), $opt_rc{calls},
    'JsonlData option(calls) matches stream recount' );

is_deeply(
    $data->attributes,
    \%attr_rc,
    'JsonlData attributes matches stream recount map'
);
is_deeply(
    $data->options,
    \%opt_rc,
    'JsonlData options matches stream recount map'
);

done_testing();

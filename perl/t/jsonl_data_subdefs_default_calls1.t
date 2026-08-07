#!/usr/bin/env perl
# Smoke / unit test: JsonlData A9 sub_defs + NEW_FID files on default-calls1.
#
# Aggregates real SUB_INFO / NEW_FID events from
# fixtures/v5/default-calls1/readstream.jsonl (not hard-coded theater).
# Expects (derived from dump; also re-counted independently below):
#   main::leaf → fid=1 first_line=3 last_line=7
#   main::mid  → fid=1 first_line=8 last_line=12
#   file(1) path contains workload.pl
#
# Usage:
#   perl -Iperl/lib perl/t/jsonl_data_subdefs_default_calls1.t
#   prove -Iperl/lib perl/t/jsonl_data_subdefs_default_calls1.t
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
# from_jsonl: sub_defs + files
# ---------------------------------------------------------------------------
my $data = Devel::NYTProf::JsonlData->from_jsonl($jsonl);
ok( defined $data, 'from_jsonl returns object' );
isa_ok( $data, 'Devel::NYTProf::JsonlData' );
ok( $data->records_seen > 0, 'records_seen > 0' );

my $leaf = $data->sub_def('main::leaf');
ok( defined $leaf, 'sub_def(main::leaf) defined' );
is( $leaf->{fid},        1, 'leaf fid == 1' );
is( $leaf->{first_line}, 3, 'leaf first_line == 3' );
is( $leaf->{last_line},  7, 'leaf last_line == 7' );

my $mid = $data->sub_def('main::mid');
ok( defined $mid, 'sub_def(main::mid) defined' );
is( $mid->{fid},        1,  'mid fid == 1' );
is( $mid->{first_line}, 8,  'mid first_line == 8' );
is( $mid->{last_line},  12, 'mid last_line == 12' );

ok( !defined $data->sub_def('no::such::sub'), 'missing sub_def → undef' );

my $defs = $data->sub_defs;
ok( ref($defs) eq 'HASH', 'sub_defs is hashref' );
ok( exists $defs->{'main::leaf'}, 'sub_defs has main::leaf' );
ok( exists $defs->{'main::mid'},  'sub_defs has main::mid' );
is( $defs->{'main::leaf'}{first_line}, 3,  'sub_defs leaf first 3' );
is( $defs->{'main::mid'}{last_line},   12, 'sub_defs mid last 12' );

# File identity: full path stored; basename helper
my $path = $data->file(1);
ok( defined $path && length $path, 'file(1) defined' );
like( $path, qr/workload\.pl\z/, 'file(1) path ends with workload.pl' );
is( $data->file_basename(1), 'workload.pl',
    'file_basename(1) == workload.pl' );

my $files = $data->files;
ok( ref($files) eq 'HASH', 'files is hashref' );
ok( exists $files->{1},    'files has fid 1' );
like( $files->{1}, qr/workload\.pl/, 'files{1} contains workload.pl' );
ok( !defined $data->file(99999), 'missing file → undef' );
ok( !defined $data->file_basename(99999), 'missing file_basename → undef' );

# ---------------------------------------------------------------------------
# Independent re-count via stream (prove ranges come from dump events)
# ---------------------------------------------------------------------------
use Devel::NYTProf::JsonlReadStream qw(for_chunks);
my %sub_defs;    # name => { fid, first_line, last_line } last write wins
my %files_rc;    # fid => path
for_chunks(
    sub {
        my ( $tag, $args ) = @_;
        if ( $tag eq 'SUB_INFO' && defined $args && @$args >= 4 ) {
            my ( $fid, $first, $last, $name ) = @{$args}[ 0 .. 3 ];
            return unless defined $name && length $name;
            $sub_defs{$name} = {
                fid        => int($fid),
                first_line => int($first),
                last_line  => int($last),
            };
        }
        elsif ( $tag eq 'NEW_FID' && defined $args && @$args >= 2 ) {
            my $fid  = $args->[0];
            my $path = $args->[-1];
            return unless defined $fid && defined $path && length $path;
            $files_rc{ int($fid) } = $path;
        }
    },
    file => $jsonl,
);

ok( exists $sub_defs{'main::leaf'}, 'stream recount has main::leaf' );
ok( exists $sub_defs{'main::mid'},  'stream recount has main::mid' );
is( $sub_defs{'main::leaf'}{fid},        1, 'stream leaf fid 1' );
is( $sub_defs{'main::leaf'}{first_line}, 3, 'stream leaf first 3' );
is( $sub_defs{'main::leaf'}{last_line},  7, 'stream leaf last 7' );
is( $sub_defs{'main::mid'}{fid},         1,  'stream mid fid 1' );
is( $sub_defs{'main::mid'}{first_line},  8,  'stream mid first 8' );
is( $sub_defs{'main::mid'}{last_line},   12, 'stream mid last 12' );

is_deeply(
    $data->sub_def('main::leaf'),
    $sub_defs{'main::leaf'},
    'JsonlData leaf matches stream recount'
);
is_deeply(
    $data->sub_def('main::mid'),
    $sub_defs{'main::mid'},
    'JsonlData mid matches stream recount'
);

ok( exists $files_rc{1}, 'stream recount has fid 1' );
like( $files_rc{1}, qr/workload\.pl/, 'stream fid 1 contains workload.pl' );
is( $data->file(1), $files_rc{1}, 'JsonlData file(1) matches stream recount' );

done_testing();

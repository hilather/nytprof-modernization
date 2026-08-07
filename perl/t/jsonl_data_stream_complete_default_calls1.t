#!/usr/bin/env perl
# Smoke / unit test: JsonlData stream completeness on default-calls1.
#
# Aligned with docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md and Rust
# ProfileModel::is_stream_complete / stream_incompleteness_reasons:
#   1. If pid_start_events > 0, require pid_end_events >= pid_start_events
#   2. Require time_line_events + time_block_events > 0
#
# Evidence:
#   - Complete: fixtures/v5/default-calls1/readstream.jsonl → complete, reasons empty
#   - Incomplete: filter real golden keeping only header tags
#     (VERSION/ATTRIBUTE/OPTION/COMMENT/START_DEFLATE) → incomplete, reasons non-empty
#     (real dump lines only — not invented event semantics)
#
# Usage:
#   perl -Iperl/lib perl/t/jsonl_data_stream_complete_default_calls1.t
#   prove -Iperl/lib perl/t/jsonl_data_stream_complete_default_calls1.t
use strict;
use warnings;
use FindBin;
use File::Spec;
use File::Temp qw(tempfile);
use Test::More;
use lib File::Spec->catdir( $FindBin::Bin, '..', 'lib' );

use Devel::NYTProf::JsonlData;
use Devel::NYTProf::JsonlReadStream qw(for_chunks);

my $repo = File::Spec->catdir( $FindBin::Bin, '..', '..' );
$repo = File::Spec->rel2abs($repo);
my $jsonl = File::Spec->catfile(
    $repo, 'fixtures', 'v5', 'default-calls1', 'readstream.jsonl'
);

plan skip_all => "missing fixture $jsonl" unless -f $jsonl;

# ---------------------------------------------------------------------------
# Helper: write incomplete JSONL from real golden (header-only tags)
# ---------------------------------------------------------------------------
# Prefer real dump lines: keep VERSION / ATTRIBUTE / OPTION / COMMENT /
# START_DEFLATE only — no TIME_LINE / TIME_BLOCK / PID_END (and no
# PID_START in this filter so timing absence is the primary reason;
# a second craft includes PID_START for the missing-end reason).
sub write_filtered_incomplete {
    my ( $src_path, $allowed_tags ) = @_;
    my %ok = map { $_ => 1 } @$allowed_tags;
    my ( $fh, $tmp ) = tempfile(
        'nytprof-jsonl-incomplete-XXXXXX',
        SUFFIX => '.jsonl',
        UNLINK => 1,
    );
    open my $in, '<:encoding(UTF-8)', $src_path
      or die "open $src_path: $!";
    my $kept = 0;
    while ( my $line = <$in> ) {
        chomp $line;
        next unless length $line;
        # Lightweight tag extract without full JSON parse (tag is always a string)
        if ( $line =~ /"tag"\s*:\s*"([^"]+)"/ ) {
            my $tag = $1;
            next unless $ok{$tag};
            print {$fh} $line, "\n";
            $kept++;
        }
    }
    close $in;
    close $fh;
    return ( $tmp, $kept );
}

# ---------------------------------------------------------------------------
# 1. Complete golden stream
# ---------------------------------------------------------------------------
my $data = Devel::NYTProf::JsonlData->from_jsonl($jsonl);
ok( defined $data, 'from_jsonl returns object' );
isa_ok( $data, 'Devel::NYTProf::JsonlData' );
ok( $data->records_seen > 0, 'records_seen > 0' );

my $tl = $data->time_line_events;
my $tb = $data->time_block_events;
my $ps = $data->pid_start_events;
my $pe = $data->pid_end_events;
diag("complete golden: TL=$tl TB=$tb pid_start=$ps pid_end=$pe");

ok( $ps > 0, "pid_start_events > 0 (got $ps)" );
ok( $pe >= $ps, "pid_end_events ($pe) >= pid_start_events ($ps)" );
ok( ( $tl + $tb ) > 0,
    "statement timing present: TL+TB=" . ( $tl + $tb ) );
# default-calls1 uses TIME_LINE (blocks=0)
ok( $tl > 0, "default-calls1 has TIME_LINE events (got $tl)" );

ok( $data->is_stream_complete,
    'is_stream_complete true on default-calls1 golden' );
my $reasons = $data->stream_incompleteness_reasons;
ok( ref($reasons) eq 'ARRAY', 'stream_incompleteness_reasons is arrayref' );
is( scalar @$reasons, 0, 'reasons empty on complete golden' );

# Independent stream re-count of timing + PID counters
my $rc_tl = 0;
my $rc_tb = 0;
my $rc_ps = 0;
my $rc_pe = 0;
for_chunks(
    sub {
        my ( $tag, $args ) = @_;
        if ( $tag eq 'TIME_LINE' && defined $args && @$args > 2 ) {
            $rc_tl++ if defined $args->[1] && defined $args->[2];
        }
        elsif ( $tag eq 'TIME_BLOCK' && defined $args && @$args > 2 ) {
            $rc_tb++ if defined $args->[1] && defined $args->[2];
        }
        elsif ( $tag eq 'PID_START' && defined $args && @$args >= 1 ) {
            $rc_ps++ if defined $args->[0] && !ref( $args->[0] );
        }
        elsif ( $tag eq 'PID_END' && defined $args && @$args >= 1 ) {
            $rc_pe++ if defined $args->[0] && !ref( $args->[0] );
        }
    },
    file => $jsonl,
);
is( $data->time_line_events,  $rc_tl, 'time_line_events matches stream recount' );
is( $data->time_block_events, $rc_tb, 'time_block_events matches stream recount' );
is( $data->pid_start_events,  $rc_ps, 'pid_start_events matches stream recount' );
is( $data->pid_end_events,    $rc_pe, 'pid_end_events matches stream recount' );

# ---------------------------------------------------------------------------
# 2. Incomplete: header-only tags from real golden (no TIME_* / no PID_*)
# ---------------------------------------------------------------------------
my ( $inc_path, $kept ) = write_filtered_incomplete(
    $jsonl,
    [qw( VERSION ATTRIBUTE OPTION COMMENT START_DEFLATE )],
);
ok( $kept > 0, "incomplete craft kept $kept real header lines" );

my $inc = Devel::NYTProf::JsonlData->from_jsonl($inc_path);
ok( defined $inc, 'from_jsonl succeeds on incomplete (load is lenient)' );
ok( !$inc->is_stream_complete,
    'is_stream_complete false on header-only incomplete stream' );
my $inc_reasons = $inc->stream_incompleteness_reasons;
ok( ref($inc_reasons) eq 'ARRAY' && @$inc_reasons > 0,
    'reasons non-empty on incomplete stream' );
ok(
    ( grep { /no statement timing/ } @$inc_reasons ),
    'reasons include no statement timing'
);
is( $inc->time_line_events + $inc->time_block_events, 0,
    'incomplete has zero TIME_LINE+TIME_BLOCK' );
is( $inc->pid_start_events, 0,
    'header-only incomplete has no PID_START (no missing-end reason required)' );

# ---------------------------------------------------------------------------
# 3. Incomplete: header + PID_START from real golden (missing PID_END + no timing)
# ---------------------------------------------------------------------------
my ( $inc2_path, $kept2 ) = write_filtered_incomplete(
    $jsonl,
    [qw( VERSION ATTRIBUTE OPTION COMMENT START_DEFLATE PID_START )],
);
ok( $kept2 > $kept,
    "incomplete+PID_START kept $kept2 lines (> header-only $kept)" );

my $inc2 = Devel::NYTProf::JsonlData->from_jsonl($inc2_path);
ok( !$inc2->is_stream_complete,
    'is_stream_complete false when PID_START without PID_END and no timing' );
my $r2 = $inc2->stream_incompleteness_reasons;
ok( @$r2 >= 2, 'at least two incompleteness reasons (pid + timing)' );
ok( ( grep { /missing PID_END/ } @$r2 ), 'reasons include missing PID_END' );
ok( ( grep { /no statement timing/ } @$r2 ),
    'reasons include no statement timing' );
ok( $inc2->pid_start_events > 0, 'PID_START present on second incomplete' );
is( $inc2->pid_end_events, 0, 'PID_END absent on second incomplete' );
is( $inc2->time_line_events + $inc2->time_block_events, 0,
    'no TIME_* on second incomplete' );

# Reasons arrayref is a copy (mutating return must not clobber internal)
push @$r2, 'caller-mutated';
my $r2b = $inc2->stream_incompleteness_reasons;
ok( !( grep { $_ eq 'caller-mutated' } @$r2b ),
    'stream_incompleteness_reasons returns a fresh arrayref' );

done_testing();

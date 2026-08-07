#!/usr/bin/env perl
# Smoke / unit test: JsonlData PID_START / PID_END on default-calls1.
#
# Aggregates real PID_START / PID_END events from
# fixtures/v5/default-calls1/readstream.jsonl (not hard-coded theater).
# Expects (derived from dump; also re-counted independently below):
#   pid_start_count >= 1
#   pid_end_count   >= 1
#   start pid matches end pid (golden observes 2975381)
#   pids() unique list matches stream re-count
#
# Usage:
#   perl -Iperl/lib perl/t/jsonl_data_pid_default_calls1.t
#   prove -Iperl/lib perl/t/jsonl_data_pid_default_calls1.t
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
# from_jsonl: pid_start_count / pid_end_count / pid_starts / pid_ends / pids
# ---------------------------------------------------------------------------
my $data = Devel::NYTProf::JsonlData->from_jsonl($jsonl);
ok( defined $data, 'from_jsonl returns object' );
isa_ok( $data, 'Devel::NYTProf::JsonlData' );
ok( $data->records_seen > 0, 'records_seen > 0' );

my $start_n = $data->pid_start_count;
my $end_n   = $data->pid_end_count;
ok( $start_n >= 1, "pid_start_count >= 1 (got $start_n)" );
ok( $end_n >= 1,   "pid_end_count >= 1 (got $end_n)" );
is( $data->pid_start_events, $start_n, 'pid_start_events alias matches count' );
is( $data->pid_end_events,   $end_n,   'pid_end_events alias matches count' );

my $starts = $data->pid_starts;
my $ends   = $data->pid_ends;
ok( ref($starts) eq 'ARRAY', 'pid_starts is arrayref' );
ok( ref($ends) eq 'ARRAY',   'pid_ends is arrayref' );
is( scalar @$starts, $start_n, 'pid_starts length == pid_start_count' );
is( scalar @$ends,   $end_n,   'pid_ends length == pid_end_count' );

ok( defined $starts->[0]{pid}, 'first PID_START has pid' );
ok( defined $ends->[0]{pid},   'first PID_END has pid' );
is( $starts->[0]{pid}, $ends->[0]{pid},
    'start pid matches end pid (default-calls1 single process)' );

# Golden dump-derived pid (from committed readstream.jsonl; not invented)
my $obs_pid = $starts->[0]{pid};
diag("observed pid=$obs_pid start_count=$start_n end_count=$end_n");
is( $obs_pid, 2975381,
    'default-calls1 golden PID_START pid is 2975381 (dump-derived)' );

# Optional fields when present on golden
ok( exists $starts->[0]{ppid},       'PID_START has ppid when dump provides it' );
ok( exists $starts->[0]{start_time}, 'PID_START has start_time when dump provides it' );
ok( exists $ends->[0]{end_time},     'PID_END has end_time when dump provides it' );

my $pids = $data->pids;
ok( ref($pids) eq 'ARRAY', 'pids is arrayref' );
ok( @$pids >= 1,           'pids non-empty' );
ok( ( grep { $_ == $obs_pid } @$pids ), 'pids includes observed start pid' );

# Shallow copies: mutating return must not clobber internal store
$starts->[0]{pid} = -1;
$ends->[0]{pid}   = -1;
is( $data->pid_starts->[0]{pid}, $obs_pid, 'pid_starts() returns element copies' );
is( $data->pid_ends->[0]{pid},   $obs_pid, 'pid_ends() returns element copies' );

# ---------------------------------------------------------------------------
# Independent re-count via stream (prove PIDs come from dump events)
# ---------------------------------------------------------------------------
use Devel::NYTProf::JsonlReadStream qw(for_chunks);
my @start_rc;
my @end_rc;
my $start_events = 0;
my $end_events   = 0;
for_chunks(
    sub {
        my ( $tag, $args ) = @_;
        if ( $tag eq 'PID_START' && defined $args && @$args >= 1 ) {
            my $pid = $args->[0];
            return unless defined $pid && !ref($pid);
            my $ev = { pid => int($pid) };
            if (   @$args > 1
                && defined $args->[1]
                && !ref( $args->[1] ) )
            {
                $ev->{ppid} = int( $args->[1] );
            }
            if (   @$args > 2
                && defined $args->[2]
                && !ref( $args->[2] ) )
            {
                $ev->{start_time} = 0 + $args->[2];
            }
            push @start_rc, $ev;
            $start_events++;
        }
        elsif ( $tag eq 'PID_END' && defined $args && @$args >= 1 ) {
            my $pid = $args->[0];
            return unless defined $pid && !ref($pid);
            my $ev = { pid => int($pid) };
            if (   @$args > 1
                && defined $args->[1]
                && !ref( $args->[1] ) )
            {
                $ev->{end_time} = 0 + $args->[1];
            }
            push @end_rc, $ev;
            $end_events++;
        }
    },
    file => $jsonl,
);

ok( $start_events >= 1, "stream recount PID_START >= 1 (got $start_events)" );
ok( $end_events >= 1,   "stream recount PID_END >= 1 (got $end_events)" );
is( $data->pid_start_count, $start_events,
    'JsonlData pid_start_count matches stream recount' );
is( $data->pid_end_count, $end_events,
    'JsonlData pid_end_count matches stream recount' );
is_deeply( $data->pid_starts, \@start_rc,
    'JsonlData pid_starts matches stream recount' );
is_deeply( $data->pid_ends, \@end_rc,
    'JsonlData pid_ends matches stream recount' );

# Unique pids from stream
my %seen;
$seen{ $_->{pid} } = 1 for ( @start_rc, @end_rc );
my @expect_pids = sort { $a <=> $b } keys %seen;
is_deeply( $data->pids, \@expect_pids,
    'JsonlData pids matches stream unique pids' );

# Start/end pid agreement from stream (same as JsonlData)
is( $start_rc[0]{pid}, $end_rc[0]{pid},
    'stream start pid matches stream end pid' );
is( $start_rc[0]{pid}, 2975381,
    'stream re-count pid is dump-derived 2975381' );

done_testing();

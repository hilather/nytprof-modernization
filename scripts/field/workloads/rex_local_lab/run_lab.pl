#!/usr/bin/env perl
# Profiled entry for the Rex + DateTime Docker lab.
# Loads the real Rex stack (use Rex) plus DateTime/YAML, then runs a
# bounded main:: loop so attach wraps the workload (main::) while
# imported Rex DSL / XSLoader stay on the goto list.
use strict;
use warnings;
use DateTime;
use DateTime::Duration;
use YAML;
use Rex -feature => ['1.4'];

sub lab_tick {
    my ($n) = @_;
    my $dt = DateTime->new(
        year      => 2020,
        month     => 1,
        day       => 1,
        time_zone => 'UTC',
    )->add( days => ( $n % 28 ) + 1, hours => ( $n % 24 ) );
    my $dur = DateTime::Duration->new(
        days    => 1,
        hours   => 2,
        minutes => ( $n % 60 ),
    );
    $dt->add_duration($dur);
    my $payload = {
        n     => $n,
        iso   => $dt->iso8601,
        ymd   => $dt->ymd,
        days  => $dur->delta_days,
        epoch => DateTime->now( time_zone => 'UTC' )->epoch,
    };
    my $yaml = YAML::Dump($payload);
    my $back = YAML::Load($yaml);
    die "yaml n" unless $back->{n} == $n;
    die "ymd"    unless $back->{ymd} =~ /^\d{4}-\d{2}-\d{2}$/;
    return length $yaml;
}

sub lab_run {
    my $secs = $ENV{NYTPROF_DEMO_SECONDS} || 5;
    $secs = 5 if $secs !~ /^\d+$/ || $secs < 1;
    $secs = 120 if $secs > 120;
    my $until = time + $secs;
    my $n     = 0;
    my $bytes = 0;
    while ( time < $until ) {
        $n++;
        $bytes += lab_tick($n);
    }
    die "lab produced no passes" if $n < 1;
    print "rex_lab_ok passes=$n bytes=$bytes secs=$secs rex=$Rex::VERSION\n";
}

lab_run();
1;

#!/usr/bin/env perl
# Bounded Cpanel::JSON::XS attach driver.
use strict;
use warnings;
use Cpanel::JSON::XS;

sub lab_run {
    my $secs = $ENV{NYTPROF_DEMO_SECONDS} || 2;
    $secs = 2 if $secs !~ /^\d+$/ || $secs < 1;
    $secs = 30 if $secs > 30;
    my $j     = Cpanel::JSON::XS->new->utf8->canonical;
    my $until = time + $secs;
    my $n     = 0;
    my $bytes = 0;
    while ( time < $until ) {
        $n++;
        my $payload = { n => $n, items => [ 1 .. 16 ], tag => "k$n" };
        my $enc     = $j->encode($payload);
        my $back    = $j->decode($enc);
        die "json roundtrip" unless $back->{n} == $n;
        $bytes += length $enc;
    }
    die "json lab produced no passes" if $n < 1;
    print "json_xs_lab_ok passes=$n bytes=$bytes secs=$secs jsonxs=$Cpanel::JSON::XS::VERSION\n";
}

lab_run();
1;

#!/usr/bin/env perl
# Bounded CryptX attach driver.
use strict;
use warnings;
use Crypt::Digest::SHA256 qw(sha256_hex);
use Crypt::PRNG qw(random_bytes);

sub lab_run {
    my $secs = $ENV{NYTPROF_DEMO_SECONDS} || 2;
    $secs = 2 if $secs !~ /^\d+$/ || $secs < 1;
    $secs = 30 if $secs > 30;
    my $until = time + $secs;
    my $n     = 0;
    my $acc   = 0;
    while ( time < $until ) {
        $n++;
        my $buf = random_bytes(32) . pack( 'N', $n );
        my $h   = sha256_hex($buf);
        $acc += length $h;
    }
    die "cryptx lab produced no passes" if $n < 1;
    print "cryptx_lab_ok passes=$n acc=$acc secs=$secs\n";
}

lab_run();
1;

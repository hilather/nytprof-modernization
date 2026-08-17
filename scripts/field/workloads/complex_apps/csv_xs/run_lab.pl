#!/usr/bin/env perl
# Bounded Text::CSV_XS attach driver.
use strict;
use warnings;
use Text::CSV_XS;

sub lab_run {
    my $secs = $ENV{NYTPROF_DEMO_SECONDS} || 2;
    $secs = 2 if $secs !~ /^\d+$/ || $secs < 1;
    $secs = 30 if $secs > 30;
    my $csv = Text::CSV_XS->new( { binary => 1, auto_diag => 1 } );
    my $until = time + $secs;
    my $n     = 0;
    my $rows  = 0;
    while ( time < $until ) {
        $n++;
        my $line = '';
        open my $fh, '>', \$line or die $!;
        $csv->say( $fh, [ $n, "name-$n", $n * 2 ] );
        close $fh;
        open my $in, '<', \$line or die $!;
        my $row = $csv->getline($in);
        close $in;
        die "csv roundtrip" unless $row && $row->[0] == $n;
        $rows++;
    }
    die "csv lab produced no passes" if $n < 1;
    print "csv_xs_lab_ok passes=$n rows=$rows secs=$secs csvxs=$Text::CSV_XS::VERSION\n";
}

lab_run();
1;

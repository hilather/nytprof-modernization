#!/usr/bin/env perl
# Bounded Template Toolkit attach driver.
use strict;
use warnings;
use Template;

sub lab_run {
    my $secs = $ENV{NYTPROF_DEMO_SECONDS} || 2;
    $secs = 2 if $secs !~ /^\d+$/ || $secs < 1;
    $secs = 30 if $secs > 30;
    my $tt  = Template->new( { ABSOLUTE => 0 } );
    my $src = '[% FOREACH i IN items %]([% i %]=[% name %])[% END %]';
    my $until = time + $secs;
    my $n     = 0;
    my $bytes = 0;
    while ( time < $until ) {
        $n++;
        my $out = '';
        $tt->process( \$src, { name => "t$n", items => [ 1 .. 12 ] }, \$out )
          or die $tt->error;
        $bytes += length $out;
    }
    die "tt lab produced no passes" if $n < 1;
    print "tt_lab_ok passes=$n bytes=$bytes secs=$secs tt=$Template::VERSION\n";
}

lab_run();
1;

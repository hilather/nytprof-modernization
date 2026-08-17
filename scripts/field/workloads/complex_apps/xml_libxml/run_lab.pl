#!/usr/bin/env perl
# Bounded XML::LibXML attach driver.
use strict;
use warnings;
use XML::LibXML;

sub lab_run {
    my $secs = $ENV{NYTPROF_DEMO_SECONDS} || 2;
    $secs = 2 if $secs !~ /^\d+$/ || $secs < 1;
    $secs = 30 if $secs > 30;
    my $parser = XML::LibXML->new();
    my $until  = time + $secs;
    my $n      = 0;
    my $nodes  = 0;
    while ( time < $until ) {
        $n++;
        my $xml =
          '<root n="' . $n . '">' . join( '', map {"<item>$_</item>"} 1 .. 8 ) . '</root>';
        my $dom = $parser->load_xml( string => $xml );
        my @hit = $dom->findnodes('//item');
        $nodes += @hit;
    }
    die "xml lab produced no passes" if $n < 1;
    print "xml_libxml_lab_ok passes=$n nodes=$nodes secs=$secs libxml=$XML::LibXML::VERSION\n";
}

lab_run();
1;

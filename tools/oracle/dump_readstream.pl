#!/usr/bin/env perl
# Canonical-ish ReadStream dump for differential scaffolding (TEST/BASE fixtures).
# Emits JSONL: one object per callback chunk. Uses the pinned oracle only.
use strict;
use warnings;
use JSON::PP ();
use Devel::NYTProf::ReadStream qw(for_chunks);

my $file = shift @ARGV
  or die "Usage: $0 profile.nytprof > dump.jsonl\n";

my $json = JSON::PP->new->canonical(1)->ascii(1);
my $seq  = 0;

for_chunks {
    my ( $tag, @args ) = @_;
    my @norm = map {
        if ( !defined $_ ) { JSON::PP::null }
        elsif ( ref($_) ) { "$_" }
        else              { $_ }
    } @args;
    print $json->encode(
        {
            seq  => $seq++,
            tag  => $tag,
            args => \@norm,
        }
      ),
      "\n";
} filename => $file, quiet => 1;

print $json->encode( { seq => $seq, tag => '_END', args => [] } ), "\n";

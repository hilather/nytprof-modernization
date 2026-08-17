#!/usr/bin/env perl
# Bounded PPI attach driver — parse a small Perl snippet, no daemon.
use strict;
use warnings;
use PPI;

sub lab_tick {
    my ($n) = @_;
    my $src = <<"P";
package App::N$n;
use strict;
sub work {
    my (\$x) = \@_;
    return \$x * $n + 1;
}
1;
P
    my $doc = PPI::Document->new( \$src );
    die "ppi parse" unless $doc;
    my $subs = $doc->find('PPI::Statement::Sub') || [];
    my $words = $doc->find('PPI::Token::Word')   || [];
    return @$subs + @$words;
}

sub lab_run {
    my $secs = $ENV{NYTPROF_DEMO_SECONDS} || 2;
    $secs = 2 if $secs !~ /^\d+$/ || $secs < 1;
    $secs = 30 if $secs > 30;
    my $until = time + $secs;
    my $n     = 0;
    my $hits  = 0;
    while ( time < $until ) {
        $n++;
        $hits += lab_tick($n);
    }
    die "ppi lab produced no passes" if $n < 1;
    print "ppi_lab_ok passes=$n hits=$hits secs=$secs ppi=$PPI::VERSION\n";
}

lab_run();
1;

#!/usr/bin/env perl
# Bounded Mojolicious attach driver — no listen daemon.
use strict;
use warnings;
use Mojolicious;
use Mojo::DOM;
use Mojo::URL;
use Mojo::Util qw(html_unescape trim);

sub lab_tick {
    my ($n) = @_;
    my $html =
      join '', '<div>', ( map {"<span>$_</span>"} 1 .. 20 ), "</div><p>$n</p>";
    my $dom = Mojo::DOM->new($html);
    my $c   = $dom->find('span')->size;
    my $url = Mojo::URL->new("https://example.test/app/$n?q=$c");
    return length( trim( html_unescape( $dom->all_text ) ) ) + length $url;
}

sub lab_run {
    my $secs = $ENV{NYTPROF_DEMO_SECONDS} || 2;
    $secs = 2 if $secs !~ /^\d+$/ || $secs < 1;
    $secs = 30 if $secs > 30;
    my $until = time + $secs;
    my $n     = 0;
    my $acc   = 0;
    while ( time < $until ) {
        $n++;
        $acc += lab_tick($n);
    }
    die "mojo lab produced no passes" if $n < 1;
    print "mojo_lab_ok passes=$n acc=$acc secs=$secs mojo=$Mojolicious::VERSION\n";
}

lab_run();
1;

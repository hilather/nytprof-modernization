#!/usr/bin/env perl
# Bounded HTML::TreeBuilder / HTML::Parser attach driver.
use strict;
use warnings;
use HTML::TreeBuilder;

sub lab_run {
    my $secs = $ENV{NYTPROF_DEMO_SECONDS} || 2;
    $secs = 2 if $secs !~ /^\d+$/ || $secs < 1;
    $secs = 30 if $secs > 30;
    my $until = time + $secs;
    my $n     = 0;
    my $tags  = 0;
    while ( time < $until ) {
        $n++;
        my $html =
            "<html><body><h1>$n</h1>"
          . join( '', map {"<p class='c$_'>item $_</p>"} 1 .. 10 )
          . '</body></html>';
        my $tree = HTML::TreeBuilder->new;
        $tree->parse_content($html);
        my @p = $tree->look_down( _tag => 'p' );
        $tags += @p;
        $tree->delete;
    }
    die "html lab produced no passes" if $n < 1;
    print "html_tree_lab_ok passes=$n tags=$tags secs=$secs\n";
}

lab_run();
1;
